//! Lock policy enforcement: ensure Cargo.lock is present, up to date, and committed.

use anyhow::{Context, Result};
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
