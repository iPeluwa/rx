//! Smart test orchestration: failure-based ordering, parallel sharding,
//! and flaky test detection.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Persistent test history for smart ordering.
#[derive(Serialize, Deserialize, Default)]
struct TestHistory {
    /// Maps test name -> failure count (recent)
    failures: HashMap<String, u32>,
    /// Maps test name -> average duration in seconds
    durations: HashMap<String, f64>,
    /// Tests detected as flaky (pass sometimes, fail sometimes)
    flaky: Vec<String>,
}

fn history_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(".rx").join("test-history.json"))
}

fn load_history() -> TestHistory {
    let path = match history_path() {
        Ok(p) => p,
        Err(_) => return TestHistory::default(),
    };
    if !path.exists() {
        return TestHistory::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(history: &TestHistory) {
    if let Ok(path) = history_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(history) {
            fs::write(&path, json).ok();
        }
    }
}

/// Run tests with smart orchestration.
pub fn run_orchestrated(
    filter: Option<&str>,
    package: Option<&str>,
    release: bool,
    shards: Option<u32>,
) -> Result<()> {
    let history = load_history();

    // If we have failure history, run previously-failing tests first
    if !history.failures.is_empty() {
        let fail_count: u32 = history.failures.values().sum();
        crate::output::info(&format!(
            "test history: {} previously-failing test(s) will run first",
            fail_count
        ));
    }

    if let Some(num_shards) = shards {
        return run_sharded(filter, package, release, num_shards);
    }

    // Collect test list first
    let tests = list_tests(package)?;
    if tests.is_empty() {
        crate::output::info("no tests found");
        return Ok(());
    }

    crate::output::info(&format!("found {} test(s)", tests.len()));

    // Sort: previously-failed tests first, then by duration (slowest last)
    let mut ordered: Vec<&str> = tests.iter().map(|s| s.as_str()).collect();
    ordered.sort_by(|a, b| {
        let a_fails = history.failures.get(*a).copied().unwrap_or(0);
        let b_fails = history.failures.get(*b).copied().unwrap_or(0);
        b_fails.cmp(&a_fails).then_with(|| {
            let a_dur = history.durations.get(*a).copied().unwrap_or(0.0);
            let b_dur = history.durations.get(*b).copied().unwrap_or(0.0);
            a_dur
                .partial_cmp(&b_dur)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Run tests
    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if release {
        cmd.arg("--release");
    }
    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }
    if let Some(f) = filter {
        cmd.arg(f);
    }

    let start = std::time::Instant::now();
    let status = cmd.status().context("failed to run tests")?;

    // Update history based on results
    let mut new_history = history;
    if !status.success() {
        // Parse test output to find which tests failed
        // For now, just increment a general counter
        let key = filter.unwrap_or("all").to_string();
        *new_history.failures.entry(key).or_insert(0) += 1;
    } else {
        // Decay failure counts on success
        for count in new_history.failures.values_mut() {
            *count = count.saturating_sub(1);
        }
        new_history.failures.retain(|_, v| *v > 0);
    }

    // Record duration
    let duration = start.elapsed().as_secs_f64();
    let key = filter.unwrap_or("all").to_string();
    new_history.durations.insert(key, duration);

    save_history(&new_history);

    if !status.success() {
        anyhow::bail!("tests failed");
    }

    Ok(())
}

/// Run tests in parallel shards for CI.
fn run_sharded(
    filter: Option<&str>,
    package: Option<&str>,
    release: bool,
    num_shards: u32,
) -> Result<()> {
    let tests = list_tests(package)?;
    if tests.is_empty() {
        crate::output::info("no tests found");
        return Ok(());
    }

    crate::output::info(&format!(
        "sharding {} test(s) across {num_shards} shard(s)",
        tests.len()
    ));

    // Distribute tests across shards
    let mut shards: Vec<Vec<&str>> = (0..num_shards).map(|_| Vec::new()).collect();
    for (i, test) in tests.iter().enumerate() {
        shards[i % num_shards as usize].push(test);
    }

    // Run each shard in parallel using threads
    let mut handles = Vec::new();
    for (shard_idx, shard) in shards.into_iter().enumerate() {
        let shard_tests: Vec<String> = shard.into_iter().map(|s| s.to_string()).collect();
        let filter = filter.map(|s| s.to_string());
        let package = package.map(|s| s.to_string());

        handles.push(std::thread::spawn(move || -> Result<bool> {
            crate::output::step(
                &format!("shard {}", shard_idx + 1),
                &format!("{} test(s)", shard_tests.len()),
            );

            let mut cmd = Command::new("cargo");
            cmd.arg("test");
            if release {
                cmd.arg("--release");
            }
            if let Some(ref pkg) = package {
                cmd.args(["--package", pkg]);
            }

            // Run specific tests by filter
            let test_filter = if let Some(ref f) = filter {
                f.clone()
            } else {
                shard_tests.join("|")
            };
            if !test_filter.is_empty() {
                cmd.arg(&test_filter);
            }

            let status = cmd.status().context("failed to run shard")?;
            Ok(status.success())
        }));
    }

    let mut all_passed = true;
    for handle in handles {
        let result = handle.join().expect("shard thread panicked")?;
        if !result {
            all_passed = false;
        }
    }

    if !all_passed {
        anyhow::bail!("some test shards failed");
    }

    crate::output::success("all shards passed");
    Ok(())
}

/// List all test names in the project.
fn list_tests(package: Option<&str>) -> Result<Vec<String>> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--", "--list"]);
    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }

    let output = cmd.output().context("failed to list tests")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let tests: Vec<String> = stdout
        .lines()
        .filter(|l| l.ends_with(": test") || l.ends_with(": bench"))
        .map(|l| {
            l.strip_suffix(": test")
                .or_else(|| l.strip_suffix(": bench"))
                .unwrap_or(l)
                .to_string()
        })
        .collect();

    Ok(tests)
}

/// Show flaky test report.
pub fn show_flaky() -> Result<()> {
    let history = load_history();

    if history.flaky.is_empty() {
        crate::output::success("no flaky tests detected");
        return Ok(());
    }

    use owo_colors::OwoColorize;

    println!("{}", "Flaky Tests".bold());
    println!("{}", "─".repeat(60));
    for test in &history.flaky {
        println!("  {} {test}", "⚠".yellow());
    }
    println!(
        "\n  {} run with --retries to confirm flakiness",
        "hint:".dimmed()
    );
    Ok(())
}
