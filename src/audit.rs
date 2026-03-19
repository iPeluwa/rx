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
    let status = Command::new("cargo")
        .arg("audit")
        .status()
        .context("failed to run cargo audit")?;
    if !status.success() {
        anyhow::bail!(
            "security vulnerabilities found\n\
             hint: run `cargo audit fix` to attempt automatic fixes, or update affected dependencies"
        );
    }
    crate::output::success("no known vulnerabilities found");
    Ok(())
}
