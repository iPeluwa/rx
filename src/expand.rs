use anyhow::{Context, Result};
use std::process::Command;

pub fn expand(item: Option<&str>) -> Result<()> {
    // Check if cargo-expand is installed
    let has_expand = Command::new("cargo")
        .args(["expand", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_expand {
        anyhow::bail!(
            "cargo-expand is not installed. Install it with:\n  cargo install cargo-expand"
        );
    }

    crate::output::info("expanding macros...");

    let mut cmd = Command::new("cargo");
    cmd.arg("expand");

    if let Some(item_path) = item {
        cmd.arg(item_path);
    }

    let status = cmd.status().context("failed to run cargo expand")?;
    if !status.success() {
        anyhow::bail!("macro expansion failed");
    }
    Ok(())
}
