use anyhow::{Context, Result};
use std::process::Command;

pub fn outdated() -> Result<()> {
    let has_outdated = Command::new("cargo")
        .args(["outdated", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_outdated {
        // Fall back to cargo update --dry-run which shows what would change
        crate::output::info("checking for dependency updates (cargo update --dry-run)...");
        crate::output::verbose(
            "install cargo-outdated for better output: cargo install cargo-outdated",
        );
        let status = Command::new("cargo")
            .args(["update", "--dry-run"])
            .status()
            .context("failed to run cargo update --dry-run")?;
        if !status.success() {
            anyhow::bail!("dependency check failed");
        }
        return Ok(());
    }

    crate::output::info("checking for outdated dependencies...");
    let status = Command::new("cargo")
        .args(["outdated", "--root-deps-only"])
        .status()
        .context("failed to run cargo outdated")?;
    if !status.success() {
        anyhow::bail!("outdated check failed");
    }
    Ok(())
}
