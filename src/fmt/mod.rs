use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;

pub fn fmt(check: bool, config: &RxConfig) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt");

    if check {
        cmd.arg("--check");
        eprintln!("[rx] checking formatting...");
    } else {
        eprintln!("[rx] formatting code...");
    }

    for arg in &config.fmt.extra_args {
        cmd.arg(arg);
    }

    let status = cmd.status().context("failed to run cargo fmt")?;
    if !status.success() {
        if check {
            anyhow::bail!("formatting check failed — run `rx fmt` to fix");
        }
        anyhow::bail!("formatting failed");
    }
    Ok(())
}
