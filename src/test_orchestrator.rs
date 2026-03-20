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
    /// Maps test name -> number of observed flip-flops (pass->fail or fail->pass)
    #[serde(default)]
    flaky_counts: HashMap<String, u32>,
    /// Maps test name -> last known state (true = passed, false = failed)
    #[serde(default)]
    last_state: HashMap<String, bool>,
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
    let output = cmd.output().context("failed to run tests")?;
    let status = output.status;

    // Parse test results for flakiness detection
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut test_results = parse_test_results(&stdout);
    test_results.extend(parse_test_results(&stderr));

    // Update history based on results
    let mut new_history = history;

    // Detect flip-flops: tests that changed state since last run
    for (test_name, passed) in &test_results {
        if let Some(&last_passed) = new_history.last_state.get(test_name) {
            // Check if state changed (flip-flop)
            if last_passed != *passed {
                let flip_count = new_history
                    .flaky_counts
                    .entry(test_name.clone())
                    .or_insert(0);
                *flip_count += 1;

                // Auto-mark as flaky after 3+ flip-flops
                if *flip_count >= 3 && !new_history.flaky.contains(test_name) {
                    use owo_colors::OwoColorize;
                    println!(
                        "  {} test '{}' marked as flaky after {} flip-flops",
                        "⚠".yellow(),
                        test_name.yellow(),
                        flip_count
                    );
                    new_history.flaky.push(test_name.clone());
                }
            }
        }

        // Update last known state
        new_history.last_state.insert(test_name.clone(), *passed);

        // Update failure counts
        if !passed {
            *new_history.failures.entry(test_name.clone()).or_insert(0) += 1;
        }
    }

    if !status.success() {
        // Also track general failure for compatibility
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

/// Parse cargo test output to extract test results.
/// Returns a map of test name -> pass/fail status (true = passed, false = failed).
fn parse_test_results(output: &str) -> HashMap<String, bool> {
    let mut results = HashMap::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Match lines like: "test module::test_name ... ok"
        if let Some(rest) = trimmed.strip_prefix("test ") {
            if let Some(dots_pos) = rest.find(" ... ") {
                let test_name = rest[..dots_pos].trim();
                let status = rest[dots_pos + 5..].trim();

                if status == "ok" {
                    results.insert(test_name.to_string(), true);
                } else if status == "FAILED" {
                    results.insert(test_name.to_string(), false);
                }
            }
        }
    }

    results
}

/// Detect flaky tests by running the test suite multiple times.
pub fn detect_flaky(filter: Option<&str>, package: Option<&str>, retries: u32) -> Result<()> {
    use owo_colors::OwoColorize;

    let retries = if retries == 0 { 3 } else { retries };

    crate::output::info(&format!(
        "running test suite {} times to detect flaky tests...",
        retries
    ));

    // Track test results across runs
    let mut all_results: Vec<HashMap<String, bool>> = Vec::new();

    for run in 1..=retries {
        crate::output::step(&format!("run {}/{}", run, retries), "running tests");

        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.arg("--");
        cmd.arg("--nocapture");

        if let Some(pkg) = package {
            cmd.args(["--package", pkg]);
        }
        if let Some(f) = filter {
            cmd.arg(f);
        }

        let output = cmd.output().context("failed to run tests")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse both stdout and stderr as cargo test can output to either
        let mut results = parse_test_results(&stdout);
        results.extend(parse_test_results(&stderr));

        if results.is_empty() {
            crate::output::info("warning: no test results parsed");
        } else {
            crate::output::info(&format!("parsed {} test results", results.len()));
        }

        all_results.push(results);
    }

    if all_results.is_empty() {
        crate::output::info("no test runs completed");
        return Ok(());
    }

    // Detect flaky tests: tests that have both passes and failures
    let mut flaky_tests: HashMap<String, (u32, u32)> = HashMap::new(); // test -> (passes, failures)

    // Collect all test names
    let mut all_test_names = std::collections::HashSet::new();
    for results in &all_results {
        for test_name in results.keys() {
            all_test_names.insert(test_name.clone());
        }
    }

    // Check each test across all runs
    for test_name in all_test_names {
        let mut passes = 0;
        let mut failures = 0;

        for results in &all_results {
            if let Some(&passed) = results.get(&test_name) {
                if passed {
                    passes += 1;
                } else {
                    failures += 1;
                }
            }
        }

        // Flaky if both passed AND failed across runs
        if passes > 0 && failures > 0 {
            flaky_tests.insert(test_name, (passes, failures));
        }
    }

    // Update history
    let mut history = load_history();

    if flaky_tests.is_empty() {
        crate::output::success("no flaky tests detected!");
    } else {
        println!("\n{}", "Flaky Tests Detected".bold().red());
        println!("{}", "─".repeat(70));

        for (test_name, (passes, failures)) in &flaky_tests {
            println!(
                "  {} {} (passed: {}, failed: {})",
                "⚠".yellow(),
                test_name.bold(),
                passes.to_string().green(),
                failures.to_string().red()
            );

            // Add to flaky list if not already there
            if !history.flaky.contains(test_name) {
                history.flaky.push(test_name.clone());
            }

            // Update flaky counts
            let flip_count = passes + failures - 1; // Number of state changes
            *history.flaky_counts.entry(test_name.clone()).or_insert(0) += flip_count;
        }

        println!(
            "\n  {} {} flaky test(s) detected",
            "total:".dimmed(),
            flaky_tests.len()
        );
    }

    save_history(&history);

    Ok(())
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
    println!("{}", "─".repeat(70));
    for test in &history.flaky {
        let flip_count = history.flaky_counts.get(test).copied().unwrap_or(0);
        if flip_count > 0 {
            println!(
                "  {} {} (flip-flops: {})",
                "⚠".yellow(),
                test,
                flip_count.to_string().yellow()
            );
        } else {
            println!("  {} {}", "⚠".yellow(), test);
        }
    }
    println!(
        "\n  {} {} flaky test(s) detected",
        "total:".dimmed(),
        history.flaky.len()
    );
    println!(
        "  {} run 'rx test detect-flaky' to re-scan for flaky tests",
        "hint:".dimmed()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_test_history_is_empty() {
        let history = TestHistory::default();
        assert!(history.failures.is_empty());
        assert!(history.durations.is_empty());
        assert!(history.flaky.is_empty());
    }

    #[test]
    fn test_history_roundtrip_serialize() {
        let mut history = TestHistory::default();
        history.failures.insert("test_foo".to_string(), 3);
        history.durations.insert("test_foo".to_string(), 1.5);
        history.flaky.push("test_bar".to_string());

        let json = serde_json::to_string(&history).expect("serialize failed");
        let restored: TestHistory = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.failures.get("test_foo"), Some(&3));
        assert_eq!(restored.durations.get("test_foo"), Some(&1.5));
        assert_eq!(restored.flaky, vec!["test_bar".to_string()]);
    }

    #[test]
    fn parse_test_results_basic() {
        let output = r#"
running 3 tests
test module::test_one ... ok
test module::test_two ... FAILED
test another::test_three ... ok
        "#;

        let results = parse_test_results(output);

        assert_eq!(results.len(), 3);
        assert_eq!(results.get("module::test_one"), Some(&true));
        assert_eq!(results.get("module::test_two"), Some(&false));
        assert_eq!(results.get("another::test_three"), Some(&true));
    }

    #[test]
    fn parse_test_results_empty() {
        let output = "some random output\nwith no test results";
        let results = parse_test_results(output);
        assert!(results.is_empty());
    }

    #[test]
    fn parse_test_results_with_whitespace() {
        let output = r#"
    test   utils::helper_test   ...   ok
test core::main_test ... FAILED
        "#;

        let results = parse_test_results(output);

        assert_eq!(results.len(), 2);
        assert_eq!(results.get("utils::helper_test"), Some(&true));
        assert_eq!(results.get("core::main_test"), Some(&false));
    }

    #[test]
    fn parse_test_results_mixed_content() {
        let output = r#"
Compiling test-project v0.1.0
Finished test [unoptimized + debuginfo] target(s) in 1.23s
Running unittests (target/debug/deps/test_project-abc123)

running 2 tests
test fast_test ... ok
test slow_test ... FAILED

failures:

---- slow_test stdout ----
thread 'slow_test' panicked at 'assertion failed'

failures:
    slow_test

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
        "#;

        let results = parse_test_results(output);

        assert_eq!(results.len(), 2);
        assert_eq!(results.get("fast_test"), Some(&true));
        assert_eq!(results.get("slow_test"), Some(&false));
    }

    #[test]
    fn test_history_with_flaky_counts() {
        let mut history = TestHistory::default();
        history.flaky.push("test_flaky".to_string());
        history.flaky_counts.insert("test_flaky".to_string(), 5);

        let json = serde_json::to_string(&history).expect("serialize failed");
        let restored: TestHistory = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.flaky, vec!["test_flaky".to_string()]);
        assert_eq!(restored.flaky_counts.get("test_flaky"), Some(&5));
    }

    #[test]
    fn test_history_backward_compat() {
        // Old format without flaky_counts and last_state
        let old_json = r#"{
            "failures": {"test_a": 2},
            "durations": {"test_a": 1.5},
            "flaky": ["test_b"]
        }"#;

        let restored: TestHistory = serde_json::from_str(old_json).expect("deserialize failed");

        assert_eq!(restored.failures.get("test_a"), Some(&2));
        assert_eq!(restored.flaky, vec!["test_b".to_string()]);
        assert!(restored.flaky_counts.is_empty());
        assert!(restored.last_state.is_empty());
    }
}
