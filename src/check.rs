use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;
use crate::output::Timer;

pub fn check(package: Option<&str>, config: &RxConfig) -> Result<()> {
    let timer = Timer::start("check");
    crate::output::info("type-checking...");

    let mut cmd = Command::new("cargo");
    cmd.arg("check");

    if let Some(flags) = crate::build::build_rustflags_pub(config) {
        cmd.env("RUSTFLAGS", flags);
    }

    if config.build.jobs > 0 {
        cmd.args(["--jobs", &config.build.jobs.to_string()]);
    }

    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }

    let status = cmd.status().context(
        "failed to run cargo check\n\
         hint: is cargo installed? run `rx doctor` to check",
    )?;
    if !status.success() {
        anyhow::bail!("check failed");
    }
    timer.finish();
    Ok(())
}
