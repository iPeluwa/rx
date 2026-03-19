use anyhow::{Context, Result};
use std::process::Command;

pub fn tree(duplicates: bool, depth: Option<u32>) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("tree");

    if duplicates {
        cmd.arg("--duplicates");
        crate::output::info("showing duplicate dependencies...");
    }

    if let Some(d) = depth {
        cmd.args(["--depth", &d.to_string()]);
    }

    let status = cmd.status().context("failed to run cargo tree")?;
    if !status.success() {
        anyhow::bail!("dependency tree failed");
    }
    Ok(())
}
