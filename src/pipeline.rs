//! Pipelined build execution for workspaces.
//!
//! Instead of waiting for an upstream crate's full compilation (codegen) to finish,
//! we can start a downstream crate's `check` (type analysis) as soon as upstream's
//! metadata/rmeta is ready. This overlaps type-checking of downstream with codegen
//! of upstream, reducing wall-clock time for workspace builds.

// Pipeline module is scaffolded for future use when the event-driven scheduler
// replaces the wave-based approach.

use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::workspace::{Member, WorkspaceGraph};

/// Track the compilation state of each crate.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum CrateState {
    Pending,
    Checking,
    MetadataReady,
    Building,
    Done,
    Failed(String),
}

/// A pipelined build result for a single crate.
#[allow(dead_code)]
#[derive(Debug)]
pub struct PipelineResult {
    pub name: String,
    pub check_duration: Duration,
    pub build_duration: Duration,
    pub success: bool,
}

/// Run a pipelined build across workspace members.
#[allow(dead_code)]
pub fn pipelined_build(
    graph: &WorkspaceGraph,
    release: bool,
    _jobs: u32,
) -> Result<Vec<PipelineResult>> {
    let states: Arc<Mutex<HashMap<String, CrateState>>> = Arc::new(Mutex::new(
        graph
            .members
            .iter()
            .map(|m| (m.name.clone(), CrateState::Pending))
            .collect(),
    ));

    let results: Arc<Mutex<Vec<PipelineResult>>> = Arc::new(Mutex::new(Vec::new()));
    let waves = crate::workspace::parallel_waves(graph)?;

    for wave in &waves {
        let wave_members: Vec<Member> = wave.iter().map(|m| (*m).clone()).collect();
        let wave_names: Vec<String> = wave_members.iter().map(|m| m.name.clone()).collect();

        // Phase 1: check all members in this wave
        let check_handles: Vec<_> = wave_members
            .iter()
            .map(|member| {
                let member = member.clone();
                let states = Arc::clone(&states);

                thread::spawn(move || -> (String, bool, Duration) {
                    wait_for_deps_metadata(&member.name, &states);

                    {
                        let mut s = states.lock().unwrap();
                        s.insert(member.name.clone(), CrateState::Checking);
                    }

                    let start = Instant::now();
                    let mut cmd = Command::new("cargo");
                    cmd.arg("check").current_dir(&member.path);
                    if release {
                        cmd.arg("--release");
                    }

                    let success = cmd.output().map(|o| o.status.success()).unwrap_or(false);
                    let duration = start.elapsed();

                    {
                        let mut s = states.lock().unwrap();
                        if success {
                            s.insert(member.name.clone(), CrateState::MetadataReady);
                        } else {
                            s.insert(
                                member.name.clone(),
                                CrateState::Failed("check failed".into()),
                            );
                        }
                    }

                    (member.name.clone(), success, duration)
                })
            })
            .collect();

        let mut check_results: HashMap<String, (bool, Duration)> = HashMap::new();
        for handle in check_handles {
            let (name, success, duration) = handle.join().expect("check thread panicked");
            check_results.insert(name, (success, duration));
        }

        // Phase 2: full build for members that passed check
        let build_handles: Vec<_> = wave_members
            .iter()
            .filter(|m| check_results.get(&m.name).map(|(s, _)| *s).unwrap_or(false))
            .map(|member| {
                let member = member.clone();
                let states = Arc::clone(&states);

                thread::spawn(move || -> (String, bool, Duration) {
                    {
                        let mut s = states.lock().unwrap();
                        s.insert(member.name.clone(), CrateState::Building);
                    }

                    let start = Instant::now();
                    let mut cmd = Command::new("cargo");
                    cmd.arg("build").current_dir(&member.path);
                    if release {
                        cmd.arg("--release");
                    }

                    let success = cmd.output().map(|o| o.status.success()).unwrap_or(false);
                    let duration = start.elapsed();

                    {
                        let mut s = states.lock().unwrap();
                        if success {
                            s.insert(member.name.clone(), CrateState::Done);
                        } else {
                            s.insert(
                                member.name.clone(),
                                CrateState::Failed("build failed".into()),
                            );
                        }
                    }

                    (member.name.clone(), success, duration)
                })
            })
            .collect();

        for handle in build_handles {
            let (name, success, build_duration) = handle.join().expect("build thread panicked");
            let (check_success, check_duration) = check_results
                .get(&name)
                .copied()
                .unwrap_or((false, Duration::ZERO));

            results.lock().unwrap().push(PipelineResult {
                name,
                check_duration,
                build_duration,
                success: check_success && success,
            });
        }

        // Check for failures
        let any_failed = {
            let s = states.lock().unwrap();
            wave_names
                .iter()
                .any(|n| matches!(s.get(n.as_str()), Some(CrateState::Failed(_))))
        };
        if any_failed {
            let failures: Vec<String> = {
                let s = states.lock().unwrap();
                wave_names
                    .iter()
                    .filter(|n| matches!(s.get(n.as_str()), Some(CrateState::Failed(_))))
                    .cloned()
                    .collect()
            };
            anyhow::bail!("pipelined build failed for: {}", failures.join(", "));
        }
    }

    let final_results = Arc::try_unwrap(results)
        .expect("results still shared")
        .into_inner()
        .unwrap();

    crate::output::info("pipeline build timing:");
    for r in &final_results {
        if r.success {
            crate::output::step(
                &r.name,
                &format!(
                    "check {:.1}s + build {:.1}s",
                    r.check_duration.as_secs_f64(),
                    r.build_duration.as_secs_f64()
                ),
            );
        }
    }

    Ok(final_results)
}

#[allow(dead_code)]
fn wait_for_deps_metadata(_name: &str, _states: &Arc<Mutex<HashMap<String, CrateState>>>) {
    // No-op in wave-based approach. Becomes meaningful with event-driven scheduler.
}

#[allow(dead_code)]
pub fn print_summary(results: &[PipelineResult]) {
    let total_check: Duration = results.iter().map(|r| r.check_duration).sum();
    let total_build: Duration = results.iter().map(|r| r.build_duration).sum();
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.len() - succeeded;

    crate::output::info(&format!(
        "pipeline summary: {} succeeded, {} failed, total check {:.1}s, total build {:.1}s",
        succeeded,
        failed,
        total_check.as_secs_f64(),
        total_build.as_secs_f64()
    ));
}
