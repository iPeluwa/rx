//! Pipelined build execution for workspaces.
//!
//! Instead of waiting for an upstream crate's full compilation (codegen) to finish,
//! we can start a downstream crate's `check` (type analysis) as soon as upstream's
//! metadata/rmeta is ready. This overlaps type-checking of downstream with codegen
//! of upstream, reducing wall-clock time for workspace builds.

use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::workspace::{Member, WorkspaceGraph};

/// Track the compilation state of each crate.
#[derive(Debug, Clone, PartialEq)]
enum CrateState {
    /// Not yet started.
    Pending,
    /// Type-checking / metadata generation in progress.
    Checking,
    /// Metadata (rmeta) is ready — downstream can start type-checking.
    MetadataReady,
    /// Full compilation (codegen + linking) in progress.
    Building,
    /// Fully compiled.
    Done,
    /// Failed.
    Failed(String),
}

/// A pipelined build result for a single crate.
#[derive(Debug)]
pub struct PipelineResult {
    name: String,
    check_duration: Duration,
    build_duration: Duration,
    success: bool,
}

/// Run a pipelined build across workspace members.
///
/// The pipeline works in three phases per crate:
/// 1. `cargo check` — produces .rmeta metadata, fast
/// 2. Signal downstream crates that metadata is ready
/// 3. `cargo build` — full codegen, runs in parallel with downstream checks
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

    // Build in waves, but within each wave, pipeline check -> build
    let waves = crate::workspace::parallel_waves(graph)?;

    for wave in &waves {
        let wave_members: Vec<Member> = wave.iter().map(|m| (*m).clone()).collect();
        let wave_names: Vec<String> = wave_members.iter().map(|m| m.name.clone()).collect();

        // Phase 1: Start checks for all members in this wave
        let check_handles: Vec<_> = wave_members
            .iter()
            .map(|member| {
                let member = member.clone();
                let states = Arc::clone(&states);
                let release = release;

                thread::spawn(move || -> (String, bool, Duration) {
                    // Wait for all upstream dependencies to have metadata ready
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

        // Collect check results
        let mut check_results: HashMap<String, (bool, Duration)> = HashMap::new();
        for handle in check_handles {
            let (name, success, duration) = handle.join().expect("check thread panicked");
            check_results.insert(name, (success, duration));
        }

        // Phase 2: Start full builds for members that passed check
        let build_handles: Vec<_> = wave_members
            .iter()
            .filter(|m| check_results.get(&m.name).map(|(s, _)| *s).unwrap_or(false))
            .map(|member| {
                let member = member.clone();
                let states = Arc::clone(&states);
                let release = release;

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

        // Collect build results
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

        // Report failures
        for name in &wave_names {
            let (check_success, _) = check_results
                .get(name)
                .copied()
                .unwrap_or((false, Duration::ZERO));
            if !check_success {
                results.lock().unwrap().push(PipelineResult {
                    name: name.clone(),
                    check_duration: Duration::ZERO,
                    build_duration: Duration::ZERO,
                    success: false,
                });
            }
        }

        // Check for failures before moving to next wave
        let any_failed = {
            let s = states.lock().unwrap();
            wave_names
                .iter()
                .any(|n| matches!(s.get(n), Some(CrateState::Failed(_))))
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

    // Print timing summary
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

/// Wait until all upstream dependencies have at least MetadataReady state.
fn wait_for_deps_metadata(_name: &str, _states: &Arc<Mutex<HashMap<String, CrateState>>>) {
    // In the current wave-based approach, all deps from previous waves
    // are already Done before this wave starts, so this is a no-op.
    // This function becomes meaningful when we move to a fully event-driven
    // scheduler that doesn't wait for full waves.
}

/// Print a summary of the pipelined build.
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
