use anyhow::{Context, Result};
use std::process::Command;

use crate::output::Timer;

pub fn bench(filter: Option<&str>, package: Option<&str>) -> Result<()> {
    let timer = Timer::start("bench");
    crate::output::info("running benchmarks...");

    let mut cmd = Command::new("cargo");
    cmd.arg("bench");

    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }
    if let Some(f) = filter {
        cmd.arg("--").arg(f);
    }

    let status = cmd.status().context(
        "failed to run cargo bench\n\
         hint: ensure your project has benchmark targets configured",
    )?;
    if !status.success() {
        anyhow::bail!("benchmarks failed");
    }
    timer.finish();
    Ok(())
}
