use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

use crate::output::Timer;

pub fn bench(filter: Option<&str>, package: Option<&str>) -> Result<()> {
    let timer = Timer::start("bench");
    let start = std::time::Instant::now();
    crate::output::info("running benchmarks...");

    let mut cmd = Command::new("cargo");
    cmd.arg("bench");

    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }
    if let Some(f) = filter {
        cmd.arg("--").arg(f);
    }

    let status = cmd.status().context(
        "failed to run cargo bench\n\
         hint: ensure your project has benchmark targets configured",
    )?;
    if !status.success() {
        crate::stats::record("bench", start, false);
        anyhow::bail!("benchmarks failed");
    }
    crate::stats::record("bench", start, true);
    timer.finish();
    Ok(())
}

/// Save current benchmark results to a named baseline.
pub fn bench_save(name: &str) -> Result<()> {
    let home = dirs::home_dir().context("no home dir")?;
    let baselines_dir = home.join(".rx").join("bench-baselines");
    fs::create_dir_all(&baselines_dir)?;

    crate::output::info(&format!("running benchmarks and saving as '{name}'..."));

    let output = Command::new("cargo")
        .args(["bench", "--", "--format", "json"])
        .output()
        .context("failed to run cargo bench")?;

    let data = String::from_utf8_lossy(&output.stdout).to_string();
    let path = baselines_dir.join(format!("{name}.txt"));
    fs::write(&path, &data)?;

    // Also save the human-readable output
    let readable = String::from_utf8_lossy(&output.stderr).to_string();
    let readable_path = baselines_dir.join(format!("{name}.log"));
    fs::write(&readable_path, &readable)?;

    crate::output::success(&format!("baseline '{name}' saved to {}", path.display()));
    Ok(())
}

/// Compare current benchmark results against a saved baseline.
pub fn bench_compare(baseline: &str) -> Result<()> {
    let home = dirs::home_dir().context("no home dir")?;
    let baselines_dir = home.join(".rx").join("bench-baselines");
    let baseline_path = baselines_dir.join(format!("{baseline}.log"));

    if !baseline_path.exists() {
        anyhow::bail!(
            "baseline '{baseline}' not found\n\
             hint: save a baseline first with `rx bench --save {baseline}`"
        );
    }

    use owo_colors::OwoColorize;

    let old_data = fs::read_to_string(&baseline_path)?;

    crate::output::info("running benchmarks for comparison...");

    let output = Command::new("cargo")
        .args(["bench"])
        .output()
        .context("failed to run cargo bench")?;

    let new_data = String::from_utf8_lossy(&output.stderr).to_string();

    // Parse benchmark results: "test name ... bench: N ns/iter (+/- M)"
    let old_results = parse_bench_results(&old_data);
    let new_results = parse_bench_results(&new_data);

    println!("{}", "Benchmark Comparison".bold());
    println!("  baseline: {} vs current", baseline.cyan());
    println!("{}", "─".repeat(70));

    let mut improvements = 0;
    let mut regressions = 0;

    for (name, new_ns) in &new_results {
        if let Some(old_ns) = old_results.get(name.as_str()) {
            let change = (*new_ns as f64 - *old_ns as f64) / *old_ns as f64 * 100.0;
            let indicator = if change < -5.0 {
                improvements += 1;
                format!("{:.1}%", change).green().to_string()
            } else if change > 5.0 {
                regressions += 1;
                format!("+{:.1}%", change).red().to_string()
            } else {
                format!("{:.1}%", change).dimmed().to_string()
            };

            println!(
                "  {:<40} {:>10} ns → {:>10} ns  {}",
                name, old_ns, new_ns, indicator
            );
        } else {
            println!(
                "  {:<40} {:>10}    {:>10} ns  {}",
                name,
                "new".dimmed(),
                new_ns,
                "new".cyan()
            );
        }
    }

    println!("{}", "─".repeat(70));
    println!(
        "  {} improvements, {} regressions",
        improvements.to_string().green(),
        regressions.to_string().red()
    );

    Ok(())
}

/// List saved baselines.
pub fn bench_list() -> Result<()> {
    let home = dirs::home_dir().context("no home dir")?;
    let baselines_dir = home.join(".rx").join("bench-baselines");

    if !baselines_dir.exists() {
        crate::output::info("no saved baselines");
        return Ok(());
    }

    use owo_colors::OwoColorize;

    println!("{}", "Saved Baselines".bold());
    let mut found = false;
    for entry in fs::read_dir(&baselines_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".log") {
            let baseline = name.trim_end_matches(".log");
            let meta = entry.metadata()?;
            let size = meta.len();
            println!("  {} ({} bytes)", baseline.cyan(), size);
            found = true;
        }
    }

    if !found {
        crate::output::info("no saved baselines");
    }

    Ok(())
}

/// Parse "test <name> ... bench: <ns> ns/iter" lines.
fn parse_bench_results(output: &str) -> std::collections::HashMap<String, u64> {
    let mut results = std::collections::HashMap::new();

    for line in output.lines() {
        let line = line.trim();
        if line.contains("bench:") && line.contains("ns/iter") {
            // Format: "test name ... bench: 1,234 ns/iter (+/- 56)"
            if let Some(name_end) = line.find(" ... bench:") {
                let name = line[..name_end]
                    .trim_start_matches("test ")
                    .trim()
                    .to_string();
                let after_bench = &line[name_end + 11..];
                let ns_str: String = after_bench
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == ',')
                    .filter(|c| *c != ',')
                    .collect();
                if let Ok(ns) = ns_str.trim().parse::<u64>() {
                    results.insert(name, ns);
                }
            }
        }
    }

    results
}
