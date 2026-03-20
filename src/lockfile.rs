//! Lock policy enforcement: ensure Cargo.lock is present, up to date, and committed.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Check lockfile health and enforce policies.
pub fn check() -> Result<()> {
    use owo_colors::OwoColorize;

    println!("{}", "Lockfile Policy Check".bold());
    println!("{}", "─".repeat(60));

    let mut issues = 0;

    // 1. Check Cargo.lock exists
    if !std::path::Path::new("Cargo.lock").exists() {
        println!("  {} Cargo.lock is missing", "✗".red());
        println!(
            "    {} run `cargo generate-lockfile` to create it",
            "→".dimmed()
        );
        issues += 1;
    } else {
        println!("  {} Cargo.lock exists", "✓".green());

        // 2. Check if Cargo.lock is up to date
        let output = Command::new("cargo")
            .args(["update", "--dry-run"])
            .output()
            .context("failed to run cargo update")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let pending_updates: Vec<&str> = stderr
            .lines()
            .filter(|l| l.contains("Updating") || l.contains("Adding") || l.contains("Removing"))
            .collect();

        if pending_updates.is_empty() {
            println!("  {} Cargo.lock is up to date", "✓".green());
        } else {
            println!(
                "  {} Cargo.lock has {} pending update(s)",
                "⚠".yellow(),
                pending_updates.len()
            );
            for update in pending_updates.iter().take(5) {
                println!("    {}", update.dimmed());
            }
            issues += 1;
        }

        // 3. Check if Cargo.lock is committed (git)
        let git_status = Command::new("git")
            .args(["status", "--porcelain", "Cargo.lock"])
            .output();

        if let Ok(output) = git_status {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                println!("  {} Cargo.lock is committed in git", "✓".green());
            } else if stdout.contains("??") {
                println!("  {} Cargo.lock is not tracked by git", "✗".red());
                println!("    {} run `git add Cargo.lock` to track it", "→".dimmed());
                issues += 1;
            } else if stdout.starts_with(" M") || stdout.starts_with("M ") {
                println!("  {} Cargo.lock has uncommitted changes", "⚠".yellow());
                issues += 1;
            } else {
                println!("  {} Cargo.lock is committed in git", "✓".green());
            }
        }

        // 4. Check for yanked crates
        let audit_output = Command::new("cargo")
            .args(["audit", "--quiet", "--json"])
            .output();

        if let Ok(output) = audit_output {
            if output.status.success() {
                println!("  {} no yanked or vulnerable dependencies", "✓".green());
            }
        }
    }

    // 5. Check that Cargo.lock format is v3+
    if let Ok(lockfile) = fs::read_to_string("Cargo.lock") {
        if lockfile.starts_with("# This file is automatically") {
            // v3+ format
            println!("  {} Cargo.lock uses modern format", "✓".green());
        } else if lockfile.contains("version = 3") || lockfile.contains("version = 4") {
            println!("  {} Cargo.lock uses modern format", "✓".green());
        } else {
            println!(
                "  {} Cargo.lock uses legacy format — consider regenerating",
                "⚠".yellow()
            );
            issues += 1;
        }
    }

    println!("{}", "─".repeat(60));
    if issues == 0 {
        crate::output::success("lockfile policy: all checks passed");
    } else {
        crate::output::warn(&format!("lockfile policy: {issues} issue(s) found"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Deep lockfile audit with reproducibility score
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CargoLock {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Deserialize)]
struct LockPackage {
    name: String,
    version: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    checksum: Option<String>,
}

/// Deep audit of Cargo.lock: duplicates, git deps, checksums, reproducibility score.
pub fn audit() -> Result<()> {
    use owo_colors::OwoColorize;

    if !std::path::Path::new("Cargo.lock").exists() {
        anyhow::bail!(
            "Cargo.lock not found\n\
             hint: run `cargo generate-lockfile` to create it"
        );
    }

    let contents = fs::read_to_string("Cargo.lock").context("failed to read Cargo.lock")?;
    let lockfile: CargoLock = toml::from_str(&contents).context("failed to parse Cargo.lock")?;

    let packages = &lockfile.package;
    let total = packages.len();

    // Classify sources
    let mut registry_count = 0u32;
    let mut git_deps: Vec<(&str, &str)> = Vec::new();
    let mut missing_checksums = 0u32;
    let mut path_deps = 0u32;

    for pkg in packages {
        if let Some(ref source) = pkg.source {
            if source.starts_with("registry+") {
                registry_count += 1;
            } else if source.starts_with("git+") {
                git_deps.push((&pkg.name, source));
            }
        } else {
            // No source = path dependency (local crate)
            path_deps += 1;
        }

        if pkg.source.is_some() && pkg.checksum.is_none() {
            missing_checksums += 1;
        }
    }

    // Find duplicate major versions
    let mut by_name: HashMap<&str, Vec<&str>> = HashMap::new();
    for pkg in packages {
        by_name.entry(&pkg.name).or_default().push(&pkg.version);
    }

    let mut duplicates: Vec<(&str, Vec<&str>)> = Vec::new();
    for (name, versions) in &by_name {
        if versions.len() > 1 {
            // Check if they have different major versions
            let majors: std::collections::HashSet<&str> = versions
                .iter()
                .map(|v| v.split('.').next().unwrap_or("0"))
                .collect();
            if majors.len() > 1 {
                let mut sorted = versions.clone();
                sorted.sort();
                duplicates.push((name, sorted));
            }
        }
    }
    duplicates.sort_by_key(|(name, _)| *name);

    // Reproducibility score
    let mut score: i32 = 100;
    score -= git_deps.len() as i32 * 5;
    score -= duplicates.len() as i32 * 2;
    score -= missing_checksums as i32;
    if total == 0 {
        score -= 10;
    }
    let score = score.max(0) as u32;

    // Output
    println!("{}", "Cargo.lock Audit".bold());
    println!("{}", "━".repeat(60).dimmed());

    println!("  Total dependencies:    {:>4}", total);
    println!("  Registry (crates.io):  {:>4}", registry_count);
    if path_deps > 0 {
        println!("  Path (local):          {:>4}", path_deps);
    }

    if git_deps.is_empty() {
        println!("  Git dependencies:      {:>4}  {}", 0, "✓".green());
    } else {
        println!(
            "  Git dependencies:      {:>4}  {}",
            git_deps.len(),
            "⚠".yellow()
        );
    }

    if duplicates.is_empty() {
        println!("  Duplicate versions:    {:>4}  {}", 0, "✓".green());
    } else {
        println!(
            "  Duplicate versions:    {:>4}  {}",
            duplicates.len(),
            "⚠".yellow()
        );
    }

    if missing_checksums == 0 {
        println!("  Missing checksums:     {:>4}  {}", 0, "✓".green());
    } else {
        println!(
            "  Missing checksums:     {:>4}  {}",
            missing_checksums,
            "⚠".yellow()
        );
    }

    // Details
    if !duplicates.is_empty() {
        println!("\n  {}", "Duplicate major versions:".bold());
        for (name, versions) in &duplicates {
            println!("    {} → {}", name, versions.join(", ").dimmed());
        }
    }

    if !git_deps.is_empty() {
        println!("\n  {}", "Git dependencies:".bold());
        for (name, source) in &git_deps {
            println!("    {} → {}", name, source.dimmed());
        }
    }

    // Score
    let score_color = if score >= 90 {
        format!("{}/100", score).green().to_string()
    } else if score >= 70 {
        format!("{}/100", score).yellow().to_string()
    } else {
        format!("{}/100", score).red().to_string()
    };

    println!("\n  {} {}", "Reproducibility score:".bold(), score_color);

    println!("{}", "━".repeat(60).dimmed());

    // Suggestions
    if !git_deps.is_empty() {
        crate::output::warn(
            "git dependencies reduce reproducibility — consider publishing to a registry",
        );
    }
    if !duplicates.is_empty() {
        crate::output::info(
            "duplicate major versions increase binary size — run `rx pkg dedupe` to investigate",
        );
    }

    Ok(())
}

/// Enforce that Cargo.lock is unchanged (for CI).
pub fn enforce() -> Result<()> {
    crate::output::info("enforcing lockfile policy...");

    // Run cargo check to trigger any lockfile changes
    let status = Command::new("cargo")
        .args(["check", "--locked"])
        .status()
        .context("failed to run cargo check --locked")?;

    if status.success() {
        crate::output::success("lockfile is consistent — no changes needed");
        Ok(())
    } else {
        anyhow::bail!(
            "Cargo.lock is out of sync with Cargo.toml\n  \
             hint: run `cargo update` locally and commit the updated Cargo.lock"
        );
    }
}
