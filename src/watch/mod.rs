use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use crate::config::RxConfig;

/// Check if a path should be ignored based on config patterns and common defaults.
fn should_ignore(path: &Path, ignore_patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    // Always ignore target/, .git/, and hidden files
    let default_ignores = ["target/", ".git/", ".DS_Store"];
    for pattern in &default_ignores {
        if path_str.contains(pattern) {
            return true;
        }
    }

    // User-configured ignore patterns (simple glob matching)
    for pattern in ignore_patterns {
        if pattern.starts_with("*.") {
            // Extension match: "*.log" matches any .log file
            let ext = &pattern[2..];
            if path_str.ends_with(ext) {
                return true;
            }
        } else if path_str.contains(pattern.trim_end_matches("**")) {
            return true;
        }
    }

    false
}

/// Only trigger rebuilds for Rust source files and key config files.
fn is_relevant_change(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    matches!(ext, Some("rs" | "toml")) || name == "Cargo.lock"
}

pub fn watch(cmd: Option<&str>, config: &RxConfig) -> Result<()> {
    let watch_cmd = cmd.unwrap_or(config.watch.cmd.as_str());
    let ignore_patterns = config.watch.ignore.clone();

    crate::output::info(&format!(
        "watching for changes (running: cargo {watch_cmd})..."
    ));

    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(300), tx)
        .context("failed to initialize file watcher")?;

    // Watch the src/ directory and Cargo.toml
    let cwd = std::env::current_dir()?;
    let src_dir = cwd.join("src");
    if src_dir.exists() {
        debouncer
            .watcher()
            .watch(&src_dir, RecursiveMode::Recursive)
            .context("failed to watch src/")?;
    }

    // Watch Cargo.toml and Cargo.lock at project root
    for name in ["Cargo.toml", "Cargo.lock"] {
        let p = cwd.join(name);
        if p.exists() {
            debouncer
                .watcher()
                .watch(&p, RecursiveMode::NonRecursive)
                .context(format!("failed to watch {name}"))?;
        }
    }

    // Watch additional directories if they exist (build.rs, benches, examples)
    for dir_name in ["benches", "examples", "tests"] {
        let dir = cwd.join(dir_name);
        if dir.exists() {
            debouncer
                .watcher()
                .watch(&dir, RecursiveMode::Recursive)
                .ok();
        }
    }

    // Watch build.rs if it exists
    let build_rs = cwd.join("build.rs");
    if build_rs.exists() {
        debouncer
            .watcher()
            .watch(&build_rs, RecursiveMode::NonRecursive)
            .ok();
    }

    // Run the initial build
    run_cargo_cmd(watch_cmd);

    crate::output::info("waiting for changes...");

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Filter to relevant, non-ignored changes
                let relevant = events.iter().any(|e| {
                    !should_ignore(&e.path, &ignore_patterns) && is_relevant_change(&e.path)
                });

                if relevant {
                    // Drain any queued events to avoid double-triggering
                    while rx.try_recv().is_ok() {}

                    crate::output::info(&format!("change detected, running cargo {watch_cmd}..."));
                    run_cargo_cmd(watch_cmd);
                    crate::output::info("waiting for changes...");
                }
            }
            Ok(Err(err)) => {
                crate::output::warn(&format!("watch error: {err}"));
            }
            Err(_) => {
                // Channel closed, watcher dropped
                break;
            }
        }
    }

    Ok(())
}

fn run_cargo_cmd(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let status = Command::new("cargo").args(&parts).status();
    match status {
        Ok(s) if s.success() => {
            crate::output::success(&format!("cargo {cmd} completed"));
        }
        Ok(s) => {
            crate::output::error(&format!(
                "cargo {cmd} failed (exit code: {})",
                s.code().unwrap_or(-1)
            ));
        }
        Err(e) => {
            crate::output::error(&format!("failed to run cargo {cmd}: {e}"));
        }
    }
}
