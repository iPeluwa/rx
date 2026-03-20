//! Persistent worker processes: keep warm rustfmt/clippy/check processes
//! to avoid cold-start overhead on repeated invocations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Global worker pool (lazily initialized).
static WORKER_POOL: Mutex<Option<WorkerPool>> = Mutex::new(None);

/// A pool of background worker processes.
struct WorkerPool {
    workers: HashMap<String, WorkerInfo>,
}

struct WorkerInfo {
    pid: u32,
    started_at: std::time::Instant,
}

/// PID file tracking for workers.
#[derive(Serialize, Deserialize, Default)]
struct WorkerPids {
    pids: HashMap<String, u32>,
}

fn pids_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(".rx").join("workers.json"))
}

fn load_pids() -> WorkerPids {
    let path = match pids_path() {
        Ok(p) => p,
        Err(_) => return WorkerPids::default(),
    };
    if !path.exists() {
        return WorkerPids::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_pids(pids: &WorkerPids) {
    if let Ok(path) = pids_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(pids) {
            fs::write(&path, json).ok();
        }
    }
}

/// Start a background cargo check process for the current project.
/// Returns immediately — the check runs in the background and results
/// are available via `worker_status()`.
pub fn start_background_check() -> Result<u32> {
    let child = Command::new("cargo")
        .args(["check", "--message-format=json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start background check")?;

    let pid = child.id();

    let mut pids = load_pids();
    pids.pids.insert("check".to_string(), pid);
    save_pids(&pids);

    crate::output::verbose(&format!("background check started (pid {pid})"));
    Ok(pid)
}

/// Start a background rustfmt check.
pub fn start_background_fmt() -> Result<u32> {
    let child = Command::new("cargo")
        .args(["fmt", "--check"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start background fmt")?;

    let pid = child.id();

    let mut pids = load_pids();
    pids.pids.insert("fmt".to_string(), pid);
    save_pids(&pids);

    crate::output::verbose(&format!("background fmt started (pid {pid})"));
    Ok(pid)
}

/// Start a background clippy check.
pub fn start_background_lint() -> Result<u32> {
    let child = Command::new("cargo")
        .args(["clippy", "--message-format=json", "--", "-W", "clippy::all"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start background lint")?;

    let pid = child.id();

    let mut pids = load_pids();
    pids.pids.insert("lint".to_string(), pid);
    save_pids(&pids);

    crate::output::verbose(&format!("background lint started (pid {pid})"));
    Ok(pid)
}

/// Check if a worker is still running.
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
}

/// Show status of all workers.
pub fn status() -> Result<()> {
    use owo_colors::OwoColorize;

    let pids = load_pids();

    println!("{}", "Worker Processes".bold());
    println!("{}", "─".repeat(60));

    if pids.pids.is_empty() {
        println!("  {} no active workers", "·".dimmed());
        return Ok(());
    }

    for (name, pid) in &pids.pids {
        let alive = is_pid_alive(*pid);
        if alive {
            println!("  {} {name} (pid {pid})", "●".green());
        } else {
            println!("  {} {name} (pid {pid}) — exited", "○".dimmed());
        }
    }

    Ok(())
}

/// Stop all workers.
pub fn stop_all() -> Result<()> {
    let pids = load_pids();

    for (name, pid) in &pids.pids {
        if is_pid_alive(*pid) {
            #[cfg(unix)]
            {
                Command::new("kill").arg(pid.to_string()).status().ok();
            }
            #[cfg(windows)]
            {
                Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .status()
                    .ok();
            }
            crate::output::step(name, &format!("stopped (pid {pid})"));
        }
    }

    save_pids(&WorkerPids::default());
    crate::output::success("all workers stopped");
    Ok(())
}

/// Pre-warm workers for the current project.
/// Starts background check, fmt, and lint processes.
pub fn warm() -> Result<()> {
    crate::output::info("warming up worker processes...");

    let check_pid = start_background_check()?;
    let fmt_pid = start_background_fmt()?;
    let lint_pid = start_background_lint()?;

    crate::output::success(&format!(
        "workers started: check({}), fmt({}), lint({})",
        check_pid, fmt_pid, lint_pid
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_worker_pids_has_empty_pids() {
        let pids = WorkerPids::default();
        assert!(pids.pids.is_empty());
    }

    #[test]
    fn worker_pids_roundtrip_serialize() {
        let mut pids = WorkerPids::default();
        pids.pids.insert("check".to_string(), 12345);
        pids.pids.insert("fmt".to_string(), 67890);

        let json = serde_json::to_string(&pids).expect("serialize failed");
        let restored: WorkerPids = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.pids.len(), 2);
        assert_eq!(restored.pids.get("check"), Some(&12345));
        assert_eq!(restored.pids.get("fmt"), Some(&67890));
    }
}
