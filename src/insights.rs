use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

pub fn insights() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cargo_toml = cwd.join("Cargo.toml");

    if !cargo_toml.exists() {
        anyhow::bail!(
            "no Cargo.toml found\n\
             hint: run this from inside a Rust project"
        );
    }

    println!("{}", "rx insights".bold());
    println!();

    // Build health
    println!("  {}", "Build Health".bold());
    println!("  {}", "─".repeat(50));

    analyze_target_dir(&cwd)?;
    analyze_build_stats()?;
    analyze_dependencies(&cargo_toml)?;
    analyze_project_structure(&cwd)?;

    println!();
    println!(
        "  {} run {} for full dependency audit",
        "tip:".dimmed(),
        "rx deps".cyan()
    );

    Ok(())
}

fn analyze_target_dir(cwd: &Path) -> Result<()> {
    let target = cwd.join("target");
    if !target.exists() {
        println!(
            "  {} no target/ directory (project not built yet)",
            "·".dimmed()
        );
        return Ok(());
    }

    let (total_size, file_count) = dir_stats(&target);
    let size_mb = total_size as f64 / 1_048_576.0;
    let size_gb = total_size as f64 / 1_073_741_824.0;

    if size_gb > 1.0 {
        println!(
            "  {} target/ is {:.1}GB ({} files)",
            "⚠".yellow(),
            size_gb,
            file_count
        );
        println!(
            "    {} run {} to clean, or {} to GC cache",
            "→".dimmed(),
            "rx clean".cyan(),
            "rx clean --gc".cyan()
        );
    } else {
        println!(
            "  {} target/ is {:.0}MB ({} files)",
            "✓".green(),
            size_mb,
            file_count
        );
    }

    // Check for debug artifacts in release profile
    let debug_dir = target.join("debug");
    let release_dir = target.join("release");
    if debug_dir.exists() && release_dir.exists() {
        let (debug_size, _) = dir_stats(&debug_dir);
        let (release_size, _) = dir_stats(&release_dir);
        let debug_mb = debug_size as f64 / 1_048_576.0;
        let release_mb = release_size as f64 / 1_048_576.0;
        println!("    debug: {:.0}MB, release: {:.0}MB", debug_mb, release_mb);
    }

    Ok(())
}

fn analyze_build_stats() -> Result<()> {
    let home = dirs::home_dir().context("no home dir")?;
    let stats_path = home.join(".rx").join("stats.json");
    if !stats_path.exists() {
        println!(
            "  {} no build stats yet — they'll appear after a few builds",
            "·".dimmed()
        );
        return Ok(());
    }

    let contents = fs::read_to_string(&stats_path)?;
    let store: serde_json::Value = serde_json::from_str(&contents)?;
    if let Some(builds) = store.get("builds").and_then(|b| b.as_array()) {
        let total = builds.len();
        let failures = builds
            .iter()
            .filter(|b| b.get("success").and_then(|s| s.as_bool()) == Some(false))
            .count();
        let success_rate = if total > 0 {
            (total - failures) as f64 / total as f64 * 100.0
        } else {
            100.0
        };

        let avg_duration: f64 = builds
            .iter()
            .filter_map(|b| b.get("duration_secs").and_then(|d| d.as_f64()))
            .sum::<f64>()
            / total.max(1) as f64;

        if success_rate < 80.0 {
            println!(
                "  {} success rate is {:.0}% ({} of {} builds)",
                "⚠".yellow(),
                success_rate,
                total - failures,
                total
            );
        } else {
            println!(
                "  {} {:.0}% success rate ({} builds, avg {:.1}s)",
                "✓".green(),
                success_rate,
                total,
                avg_duration
            );
        }
    }

    Ok(())
}

fn analyze_dependencies(cargo_toml: &Path) -> Result<()> {
    println!();
    println!("  {}", "Dependency Health".bold());
    println!("  {}", "─".repeat(50));

    let contents = fs::read_to_string(cargo_toml)?;

    // Count dependencies
    let dep_count = contents
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.starts_with('#')
                && !trimmed.starts_with('[')
                && trimmed.contains('=')
                && !trimmed.contains("version")
                && !trimmed.contains("edition")
                && !trimmed.contains("name")
                && !trimmed.contains("license")
                && !trimmed.contains("description")
                && !trimmed.contains("rust-version")
        })
        .count();

    println!("  {} ~{} direct dependencies", "·".dimmed(), dep_count);

    // Check for duplicate deps in workspace
    let output = Command::new("cargo")
        .args(["tree", "--duplicates", "--quiet"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let dup_lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
            if dup_lines.is_empty() {
                println!("  {} no duplicate dependencies", "✓".green());
            } else {
                let dup_count = dup_lines.len();
                println!(
                    "  {} {} duplicate dependency version(s) in tree",
                    "⚠".yellow(),
                    dup_count
                );
                for line in dup_lines.iter().take(5) {
                    println!("    {}", line.dimmed());
                }
                if dup_count > 5 {
                    println!("    {} ... and {} more", "".dimmed(), dup_count - 5);
                }
                println!(
                    "    {} run {} for details",
                    "→".dimmed(),
                    "rx pkg dedupe".cyan()
                );
            }
        }
    }

    // Quick audit check
    let has_audit = Command::new("cargo")
        .args(["audit", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_audit {
        let output = Command::new("cargo").args(["audit", "--quiet"]).output();
        if let Ok(o) = output {
            if o.status.success() {
                println!("  {} no known security vulnerabilities", "✓".green());
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let vuln_count = stderr.lines().filter(|l| l.contains("RUSTSEC")).count();
                println!(
                    "  {} {} known vulnerabilities found",
                    "✗".red(),
                    vuln_count.max(1)
                );
                println!("    {} run {} for details", "→".dimmed(), "rx audit".cyan());
            }
        }
    } else {
        println!(
            "  {} cargo-audit not installed — install with: cargo install cargo-audit",
            "·".dimmed()
        );
    }

    Ok(())
}

fn analyze_project_structure(cwd: &Path) -> Result<()> {
    println!();
    println!("  {}", "Project Structure".bold());
    println!("  {}", "─".repeat(50));

    // Count source files and lines
    let src_dir = cwd.join("src");
    if src_dir.exists() {
        let mut file_count = 0;
        let mut line_count = 0;
        for entry in WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            file_count += 1;
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                line_count += contents.lines().count();
            }
        }
        println!(
            "  {} {} source files, ~{} lines of Rust",
            "·".dimmed(),
            file_count,
            line_count
        );
    }

    // Check for common missing files
    let checks = [
        ("README.md", "readme"),
        (".gitignore", "gitignore"),
        ("LICENSE", "license"),
        ("Cargo.lock", "lockfile"),
    ];

    for (file, label) in checks {
        let path = cwd.join(file);
        if !path.exists() {
            // Also check LICENSE-MIT, LICENSE-APACHE, etc.
            if file == "LICENSE" {
                let has_any_license = ["LICENSE-MIT", "LICENSE-APACHE", "LICENSE.md"]
                    .iter()
                    .any(|f| cwd.join(f).exists());
                if has_any_license {
                    continue;
                }
            }
            println!("  {} missing {}", "⚠".yellow(), label);
        }
    }

    // Check for test coverage
    let tests_dir = cwd.join("tests");
    let has_tests = tests_dir.exists();
    let has_inline_tests = if src_dir.exists() {
        WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .any(|e| {
                fs::read_to_string(e.path())
                    .map(|c| c.contains("#[test]") || c.contains("#[cfg(test)]"))
                    .unwrap_or(false)
            })
    } else {
        false
    };

    if has_tests || has_inline_tests {
        let mut parts = Vec::new();
        if has_tests {
            parts.push("integration tests");
        }
        if has_inline_tests {
            parts.push("unit tests");
        }
        println!("  {} has {}", "✓".green(), parts.join(" + "));
    } else {
        println!("  {} no tests found", "⚠".yellow());
    }

    Ok(())
}

fn dir_stats(path: &Path) -> (u64, usize) {
    let mut total = 0u64;
    let mut count = 0usize;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
                count += 1;
            }
        }
    }
    (total, count)
}
