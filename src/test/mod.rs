use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;

fn has_nextest() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn test(
    filter: Option<&str>,
    package: Option<&str>,
    release: bool,
    config: &RxConfig,
) -> Result<()> {
    let use_nextest = match config.test.runner.as_str() {
        "nextest" => true,
        "cargo" => false,
        _ => has_nextest(), // "auto"
    };

    let mut cmd = Command::new("cargo");
    if use_nextest {
        cmd.arg("nextest").arg("run");
        crate::output::info("running tests with nextest...");
    } else {
        cmd.arg("test");
        crate::output::info("running tests...");
    }

    if release {
        cmd.arg("--release");
    }
    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }

    // Extra args from config
    for arg in &config.test.extra_args {
        cmd.arg(arg);
    }

    if let Some(f) = filter {
        if use_nextest {
            cmd.args(["--filter", f]);
        } else {
            cmd.arg(f);
        }
    }

    let status = cmd.status().context("failed to run tests")?;
    if !status.success() {
        anyhow::bail!("tests failed");
    }
    Ok(())
}
