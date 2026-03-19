use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;
use crate::output::Timer;

pub fn fmt(check: bool, config: &RxConfig) -> Result<()> {
    let timer = Timer::start("fmt");
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt");

    if check {
        cmd.arg("--check");
        crate::output::info("checking formatting...");
    } else {
        crate::output::info("formatting code...");
    }

    for arg in &config.fmt.extra_args {
        cmd.arg(arg);
    }

    let status = cmd.status().context(
        "failed to run cargo fmt\n\
         hint: install rustfmt with `rustup component add rustfmt`",
    )?;
    if !status.success() {
        if check {
            anyhow::bail!("formatting check failed — run `rx fmt` to fix");
        }
        anyhow::bail!("formatting failed");
    }
    timer.finish();
    Ok(())
}
