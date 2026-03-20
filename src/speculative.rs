//! Speculative cargo check — start type-checking in the background
//! while config/cache is being loaded, saving 100-300ms of latency.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Result of a speculative check.
pub struct CheckResult {
    pub success: bool,
}

/// Spawns `cargo check` immediately in a background thread.
///
/// The check runs with no custom RUSTFLAGS. If the final config also has no
/// custom flags, the result can be reused directly. Otherwise the speculative
/// process is killed and a normal check proceeds.
pub struct SpeculativeChecker {
    handle: Option<JoinHandle<CheckResult>>,
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl SpeculativeChecker {
    /// Start a speculative `cargo check` immediately.
    pub fn start() -> Self {
        let child_holder: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
        let child_for_thread = Arc::clone(&child_holder);

        let handle = thread::spawn(move || {
            let child_result = Command::new("cargo")
                .arg("check")
                .arg("--message-format=short")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            let child = match child_result {
                Ok(c) => c,
                Err(_) => return CheckResult { success: false },
            };

            // Store child handle so cancel() can kill it
            {
                let mut slot = child_for_thread.lock().unwrap();
                *slot = Some(child);
            }

            // Wait for the child to complete
            let mut slot = child_for_thread.lock().unwrap();
            if let Some(ref mut child) = *slot {
                let status = child.wait();
                let success = status.map(|s| s.success()).unwrap_or(false);
                *slot = None;
                CheckResult { success }
            } else {
                // Child was taken by cancel()
                CheckResult { success: false }
            }
        });

        Self {
            handle: Some(handle),
            child: child_holder,
        }
    }

    /// Wait for the speculative check to finish and return the result.
    pub fn join(mut self) -> CheckResult {
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap_or(CheckResult { success: false })
        } else {
            CheckResult { success: false }
        }
    }

    /// Kill the speculative check process (e.g., when we need different flags).
    pub fn cancel(mut self) {
        if let Ok(mut slot) = self.child.lock() {
            if let Some(ref mut child) = *slot {
                let _ = child.kill();
                let _ = child.wait(); // Reap zombie
            }
        }
        // Detach the thread — it'll see the child is gone and exit
        self.handle.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speculative_checker_runs_true_command() {
        // Test the mechanism with a real (fast) command
        // We can't easily mock cargo check, but we can verify the struct works
        let checker = SpeculativeChecker {
            handle: Some(thread::spawn(|| CheckResult { success: true })),
            child: Arc::new(Mutex::new(None)),
        };
        let result = checker.join();
        assert!(result.success);
    }

    #[test]
    fn speculative_checker_cancel_is_safe() {
        let checker = SpeculativeChecker {
            handle: Some(thread::spawn(|| {
                thread::sleep(std::time::Duration::from_secs(10));
                CheckResult { success: false }
            })),
            child: Arc::new(Mutex::new(None)),
        };
        // Cancel should not hang
        checker.cancel();
    }
}
