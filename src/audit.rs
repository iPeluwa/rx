use anyhow::{Context, Result};
use std::process::Command;

pub fn audit() -> Result<()> {
    let has_audit = Command::new("cargo")
        .args(["audit", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_audit {
        anyhow::bail!(
            "cargo-audit is not installed\n\
             hint: install it with: cargo install cargo-audit"
        );
    }

    crate::output::info("auditing dependencies for security vulnerabilities...");

    let output = Command::new("cargo")
        .arg("audit")
        .output()
        .context("failed to run cargo audit")?;

    if output.status.success() {
        crate::output::success("no known vulnerabilities found");
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    // cargo-audit < 0.21 can't parse CVSS v4.0 entries in the advisory DB.
    // Detect this and retry with --ignore-source to skip the DB parse step,
    // or advise the user to upgrade.
    if stderr.contains("unsupported CVSS version") || stderr.contains("error loading advisory") {
        crate::output::warn(
            "cargo-audit failed to parse advisory database (CVSS v4.0 entries unsupported)",
        );
        crate::output::info("retrying with --ignore-source...");

        let retry = Command::new("cargo")
            .args(["audit", "--ignore-source"])
            .output()
            .context("failed to run cargo audit --ignore-source")?;

        if retry.status.success() {
            crate::output::success("no known vulnerabilities found");
            crate::output::step(
                "hint",
                "upgrade cargo-audit to fix the parse warning: cargo install cargo-audit",
            );
            return Ok(());
        }

        let retry_stdout = String::from_utf8_lossy(&retry.stdout);
        let retry_stderr = String::from_utf8_lossy(&retry.stderr);

        // If it still fails with ignore-source, check if it's the same parse error
        if retry_stderr.contains("unsupported CVSS version") {
            crate::output::warn("cargo-audit cannot parse the current advisory database");
            crate::output::step(
                "fix",
                "upgrade cargo-audit: cargo install cargo-audit --force",
            );
            return Ok(());
        }

        // Real vulnerabilities found
        if !retry_stdout.is_empty() {
            print!("{}", retry_stdout);
        }
        if !retry_stderr.is_empty() {
            eprint!("{}", retry_stderr);
        }
        anyhow::bail!(
            "security vulnerabilities found\n\
             hint: run `cargo audit fix` to attempt automatic fixes, or update affected dependencies"
        );
    }

    // Normal failure — real vulnerabilities found
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }
    anyhow::bail!(
        "security vulnerabilities found\n\
         hint: run `cargo audit fix` to attempt automatic fixes, or update affected dependencies"
    );
}
