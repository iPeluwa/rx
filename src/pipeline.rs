//! Pipelined build execution for workspaces.
//!
//! Instead of waiting for an upstream crate's full compilation (codegen) to finish,
//! we can start a downstream crate's `check` (type analysis) as soon as upstream's
//! metadata/rmeta is ready. This overlaps type-checking of downstream with codegen
//! of upstream, reducing wall-clock time for workspace builds.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
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

/// Shared state for the event-driven scheduler.
struct SchedulerState {
    states: Mutex<HashMap<String, CrateState>>,
    condvar: Condvar,
}

impl SchedulerState {
    fn new(members: &[Member]) -> Self {
        let states = members
            .iter()
            .map(|m| (m.name.clone(), CrateState::Pending))
            .collect();

        Self {
            states: Mutex::new(states),
            condvar: Condvar::new(),
        }
    }

    /// Update the state of a crate and notify all waiting threads.
    fn update_state(&self, name: &str, state: CrateState) {
        let mut states = self.states.lock().unwrap();
        states.insert(name.to_string(), state);
        drop(states);
        self.condvar.notify_all();
    }

    #[cfg(test)]
    fn get_state(&self, name: &str) -> Option<CrateState> {
        let states = self.states.lock().unwrap();
        states.get(name).cloned()
    }

    /// Wait until all dependencies have reached MetadataReady or Done state.
    fn wait_for_deps(&self, deps: &HashSet<String>) {
        let mut states = self.states.lock().unwrap();

        loop {
            let all_ready = deps.iter().all(|dep_name| {
                if let Some(state) = states.get(dep_name) {
                    matches!(state, CrateState::MetadataReady | CrateState::Done)
                } else {
                    false
                }
            });

            if all_ready {
                break;
            }

            states = self.condvar.wait(states).unwrap();
        }
    }
}

/// Run an event-driven pipelined build across workspace members.
///
/// This function spawns all crates at once, and each crate's thread waits for its
/// dependencies to reach MetadataReady state before starting its check phase.
/// This replaces the wave-based approach with a more efficient event-driven model.
pub fn run_event_driven(
    graph: &WorkspaceGraph,
    release: bool,
    jobs: u32,
) -> Result<Vec<PipelineResult>> {
    let scheduler_state = Arc::new(SchedulerState::new(&graph.members));
    let results: Arc<Mutex<Vec<PipelineResult>>> = Arc::new(Mutex::new(Vec::new()));

    // Create a semaphore for job limiting if jobs > 0
    let semaphore = if jobs > 0 {
        Some(Arc::new(std::sync::Semaphore::new(jobs as usize)))
    } else {
        None
    };

    let mut handles = Vec::new();

    for member in &graph.members {
        let member = member.clone();
        let scheduler_state = Arc::clone(&scheduler_state);
        let deps = graph.deps.get(&member.name).cloned().unwrap_or_default();
        let semaphore = semaphore.clone();

        let handle = thread::spawn(move || {
            // Acquire semaphore permit if job limiting is enabled
            let _permit = semaphore.as_ref().map(|s| s.acquire());

            // Wait for all dependencies to have their metadata ready
            scheduler_state.wait_for_deps(&deps);

            // Phase 1: Check
            scheduler_state.update_state(&member.name, CrateState::Checking);

            let check_start = Instant::now();
            let mut check_cmd = Command::new("cargo");
            check_cmd.arg("check").current_dir(&member.path);
            if release {
                check_cmd.arg("--release");
            }

            let check_success = check_cmd
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            let check_duration = check_start.elapsed();

            if !check_success {
                scheduler_state
                    .update_state(&member.name, CrateState::Failed("check failed".into()));
                return (member.name.clone(), check_duration, Duration::ZERO, false);
            }

            // Metadata is now ready - update state and notify
            scheduler_state.update_state(&member.name, CrateState::MetadataReady);

            // Phase 2: Build
            scheduler_state.update_state(&member.name, CrateState::Building);

            let build_start = Instant::now();
            let mut build_cmd = Command::new("cargo");
            build_cmd.arg("build").current_dir(&member.path);
            if release {
                build_cmd.arg("--release");
            }

            let build_success = build_cmd
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            let build_duration = build_start.elapsed();

            if build_success {
                scheduler_state.update_state(&member.name, CrateState::Done);
            } else {
                scheduler_state
                    .update_state(&member.name, CrateState::Failed("build failed".into()));
            }

            (
                member.name.clone(),
                check_duration,
                build_duration,
                check_success && build_success,
            )
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (name, check_duration, build_duration, success) =
            handle.join().expect("worker thread panicked");

        results.lock().unwrap().push(PipelineResult {
            name,
            check_duration,
            build_duration,
            success,
        });
    }

    let final_results = Arc::try_unwrap(results)
        .expect("results still shared")
        .into_inner()
        .unwrap();

    // Check for failures
    let failures: Vec<String> = final_results
        .iter()
        .filter(|r| !r.success)
        .map(|r| r.name.clone())
        .collect();

    if !failures.is_empty() {
        anyhow::bail!("pipelined build failed for: {}", failures.join(", "));
    }

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

/// Run a pipelined build across workspace members.
///
/// This function now delegates to the event-driven scheduler for improved performance.
#[allow(dead_code)]
pub fn pipelined_build(
    graph: &WorkspaceGraph,
    release: bool,
    jobs: u32,
) -> Result<Vec<PipelineResult>> {
    run_event_driven(graph, release, jobs)
}

#[allow(dead_code)]
fn wait_for_deps_metadata(_name: &str, _states: &Arc<Mutex<HashMap<String, CrateState>>>) {
    // Deprecated: kept for backwards compatibility, but not used in event-driven approach.
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

// Semaphore implementation for job limiting
mod std {
    pub use ::std::*;

    pub mod sync {
        pub use ::std::sync::*;

        /// A simple counting semaphore for limiting concurrency.
        pub struct Semaphore {
            count: Mutex<usize>,
            condvar: Condvar,
        }

        impl Semaphore {
            pub fn new(count: usize) -> Self {
                Self {
                    count: Mutex::new(count),
                    condvar: Condvar::new(),
                }
            }

            pub fn acquire(&self) -> SemaphoreGuard<'_> {
                let mut count = self.count.lock().unwrap();
                while *count == 0 {
                    count = self.condvar.wait(count).unwrap();
                }
                *count -= 1;
                SemaphoreGuard { semaphore: self }
            }

            fn release(&self) {
                let mut count = self.count.lock().unwrap();
                *count += 1;
                self.condvar.notify_one();
            }
        }

        pub struct SemaphoreGuard<'a> {
            semaphore: &'a Semaphore,
        }

        impl<'a> Drop for SemaphoreGuard<'a> {
            fn drop(&mut self) {
                self.semaphore.release();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_state_transitions() {
        let members = vec![Member {
            name: "test-crate".to_string(),
            path: std::path::PathBuf::from("/tmp/test"),
        }];

        let state = SchedulerState::new(&members);

        // Initial state should be Pending
        assert_eq!(state.get_state("test-crate"), Some(CrateState::Pending));

        // Transition to Checking
        state.update_state("test-crate", CrateState::Checking);
        assert_eq!(state.get_state("test-crate"), Some(CrateState::Checking));

        // Transition to MetadataReady
        state.update_state("test-crate", CrateState::MetadataReady);
        assert_eq!(
            state.get_state("test-crate"),
            Some(CrateState::MetadataReady)
        );

        // Transition to Building
        state.update_state("test-crate", CrateState::Building);
        assert_eq!(state.get_state("test-crate"), Some(CrateState::Building));

        // Transition to Done
        state.update_state("test-crate", CrateState::Done);
        assert_eq!(state.get_state("test-crate"), Some(CrateState::Done));
    }

    #[test]
    fn test_crate_state_failure() {
        let members = vec![Member {
            name: "failing-crate".to_string(),
            path: std::path::PathBuf::from("/tmp/fail"),
        }];

        let state = SchedulerState::new(&members);

        // Transition to Failed state
        state.update_state(
            "failing-crate",
            CrateState::Failed("compilation error".to_string()),
        );

        match state.get_state("failing-crate") {
            Some(CrateState::Failed(msg)) => {
                assert_eq!(msg, "compilation error");
            }
            _ => panic!("Expected Failed state"),
        }
    }

    #[test]
    fn test_dependency_ordering_mock() {
        // Create a mock scenario with 3 crates: A <- B <- C
        // (C depends on B, B depends on A)
        let members = vec![
            Member {
                name: "crate-a".to_string(),
                path: std::path::PathBuf::from("/tmp/a"),
            },
            Member {
                name: "crate-b".to_string(),
                path: std::path::PathBuf::from("/tmp/b"),
            },
            Member {
                name: "crate-c".to_string(),
                path: std::path::PathBuf::from("/tmp/c"),
            },
        ];

        let state = Arc::new(SchedulerState::new(&members));

        // Simulate crate-b waiting for crate-a
        let state_clone = Arc::clone(&state);
        let handle_b = thread::spawn(move || {
            let mut deps = HashSet::new();
            deps.insert("crate-a".to_string());
            state_clone.wait_for_deps(&deps);
            // If we get here, crate-a must be ready
            true
        });

        // Give the thread time to start waiting
        thread::sleep(Duration::from_millis(50));

        // Verify crate-b is still waiting (handle hasn't completed)
        assert!(!handle_b.is_finished());

        // Now mark crate-a as MetadataReady
        state.update_state("crate-a", CrateState::MetadataReady);

        // crate-b should now proceed
        let result = handle_b.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_multiple_dependencies() {
        let members = vec![
            Member {
                name: "dep1".to_string(),
                path: std::path::PathBuf::from("/tmp/dep1"),
            },
            Member {
                name: "dep2".to_string(),
                path: std::path::PathBuf::from("/tmp/dep2"),
            },
            Member {
                name: "consumer".to_string(),
                path: std::path::PathBuf::from("/tmp/consumer"),
            },
        ];

        let state = Arc::new(SchedulerState::new(&members));

        // consumer depends on both dep1 and dep2
        let state_clone = Arc::clone(&state);
        let handle = thread::spawn(move || {
            let mut deps = HashSet::new();
            deps.insert("dep1".to_string());
            deps.insert("dep2".to_string());
            state_clone.wait_for_deps(&deps);
            true
        });

        thread::sleep(Duration::from_millis(50));

        // Mark only dep1 as ready - should still wait
        state.update_state("dep1", CrateState::MetadataReady);
        thread::sleep(Duration::from_millis(50));
        assert!(!handle.is_finished());

        // Mark dep2 as Done - now should proceed
        state.update_state("dep2", CrateState::Done);
        let result = handle.join().unwrap();
        assert!(result);
    }

    #[test]
    fn test_semaphore_limits_concurrency() {
        let sem = Arc::new(std::sync::Semaphore::new(2));
        let counter = Arc::new(Mutex::new(0));

        let mut handles = vec![];

        for _ in 0..5 {
            let sem = Arc::clone(&sem);
            let counter = Arc::clone(&counter);

            let handle = thread::spawn(move || {
                let _guard = sem.acquire();
                let mut count = counter.lock().unwrap();
                *count += 1;
                let current = *count;
                drop(count);

                // With semaphore limit of 2, we should never see more than 2 concurrent
                assert!(current <= 2);

                // Simulate work
                thread::sleep(Duration::from_millis(10));

                let mut count = counter.lock().unwrap();
                *count -= 1;
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
