use anyhow::{Context, Result};
use std::process::Command;

use crate::cli::TestAdvancedCommand;

pub fn dispatch(cmd: &TestAdvancedCommand) -> Result<()> {
    match cmd {
        TestAdvancedCommand::Snapshot { review } => snapshot(*review),
        TestAdvancedCommand::Fuzz { target, time } => fuzz(target, time),
        TestAdvancedCommand::Mutate { package } => mutate(package.as_deref()),
    }
}

/// Run snapshot tests using cargo-insta.
fn snapshot(review: bool) -> Result<()> {
    let has_insta = Command::new("cargo")
        .args(["insta", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_insta {
        anyhow::bail!(
            "cargo-insta is not installed\n\
             hint: install with `cargo install cargo-insta`"
        );
    }

    if review {
        crate::output::info("reviewing snapshots interactively...");
        let status = Command::new("cargo")
            .args(["insta", "review"])
            .status()
            .context("failed to run cargo insta review")?;
        if !status.success() {
            anyhow::bail!("snapshot review failed");
        }
    } else {
        crate::output::info("updating snapshots...");
        let status = Command::new("cargo")
            .args(["insta", "test", "--accept"])
            .status()
            .context("failed to run cargo insta test")?;
        if !status.success() {
            anyhow::bail!("snapshot update failed");
        }
        crate::output::success("snapshots updated");
    }

    Ok(())
}

/// Run fuzz tests using cargo-fuzz.
fn fuzz(target: &str, time: &str) -> Result<()> {
    let has_fuzz = Command::new("cargo")
        .args(["fuzz", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_fuzz {
        anyhow::bail!(
            "cargo-fuzz is not installed\n\
             hint: install with `cargo install cargo-fuzz`\n\
             note: requires nightly toolchain"
        );
    }

    // Parse time duration (e.g., "60s", "5m", "1h")
    let max_time = parse_duration(time)?;

    crate::output::info(&format!("fuzzing target '{target}' for {max_time}s..."));

    let status = Command::new("cargo")
        .args([
            "+nightly",
            "fuzz",
            "run",
            target,
            "--",
            &format!("-max_total_time={max_time}"),
        ])
        .status()
        .context("failed to run cargo fuzz")?;

    if !status.success() {
        anyhow::bail!("fuzzing found a failure — check the output above");
    }

    crate::output::success(&format!("fuzzing completed for {target}"));
    Ok(())
}

/// Run mutation tests using cargo-mutants.
fn mutate(package: Option<&str>) -> Result<()> {
    let has_mutants = Command::new("cargo")
        .args(["mutants", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_mutants {
        anyhow::bail!(
            "cargo-mutants is not installed\n\
             hint: install with `cargo install cargo-mutants`"
        );
    }

    crate::output::info("running mutation tests (this may take a while)...");

    let mut cmd = Command::new("cargo");
    cmd.arg("mutants");

    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }

    let status = cmd.status().context("failed to run cargo mutants")?;
    if !status.success() {
        anyhow::bail!(
            "mutation testing completed with surviving mutants\n\
             hint: review the output above — surviving mutants indicate gaps in test coverage"
        );
    }

    crate::output::success("mutation testing passed — all mutants caught");
    Ok(())
}

/// Parse duration strings like "60s", "5m", "1h" into seconds.
fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('s') {
        return num.parse().context("invalid duration number");
    }
    if let Some(num) = s.strip_suffix('m') {
        let mins: u64 = num.parse().context("invalid duration number")?;
        return Ok(mins * 60);
    }
    if let Some(num) = s.strip_suffix('h') {
        let hours: u64 = num.parse().context("invalid duration number")?;
        return Ok(hours * 3600);
    }
    // Default: assume seconds
    s.parse()
        .context("invalid duration — use 60s, 5m, or 1h format")
}
