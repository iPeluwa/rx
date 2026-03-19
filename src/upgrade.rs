use anyhow::{Context, Result};
use std::process::Command;

use crate::output::Timer;

pub fn upgrade() -> Result<()> {
    let timer = Timer::start("upgrade");

    // Step 1: Update Rust toolchains
    crate::output::info("updating Rust toolchains...");
    let status = Command::new("rustup")
        .arg("update")
        .status()
        .context("failed to run rustup update")?;
    if !status.success() {
        crate::output::warn("rustup update failed, continuing...");
    }

    // Step 2: Update dependencies
    crate::output::info("updating dependencies...");
    let status = Command::new("cargo")
        .arg("update")
        .status()
        .context("failed to run cargo update")?;
    if !status.success() {
        anyhow::bail!("cargo update failed");
    }

    // Step 3: Check for outdated deps (informational)
    crate::output::info("checking for outdated dependencies...");
    let output = Command::new("cargo")
        .args(["outdated", "--root-deps-only"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() || stdout.contains("All dependencies are up to date") {
                crate::output::success("all dependencies are up to date");
            } else {
                println!("{stdout}");
            }
        }
        _ => {
            crate::output::verbose(
                "install cargo-outdated for dependency freshness checks: cargo install cargo-outdated",
            );
        }
    }

    timer.finish();
    Ok(())
}
