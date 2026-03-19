use anyhow::{Context, Result};
use std::process::Command;

pub fn bench(filter: Option<&str>, package: Option<&str>) -> Result<()> {
    crate::output::info("running benchmarks...");

    let mut cmd = Command::new("cargo");
    cmd.arg("bench");

    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }
    if let Some(f) = filter {
        cmd.arg("--").arg(f);
    }

    let status = cmd.status().context("failed to run cargo bench")?;
    if !status.success() {
        anyhow::bail!("benchmarks failed");
    }
    Ok(())
}
