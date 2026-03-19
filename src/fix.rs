use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;
use crate::output::Timer;

pub fn fix(config: &RxConfig) -> Result<()> {
    let timer = Timer::start("fix");

    // Step 1: cargo fix
    crate::output::info("applying compiler suggestions...");
    let status = Command::new("cargo")
        .args(["fix", "--allow-dirty", "--allow-staged"])
        .status()
        .context("failed to run cargo fix")?;
    if !status.success() {
        crate::output::warn("cargo fix encountered issues, continuing...");
    }

    // Step 2: clippy --fix
    crate::output::info("applying clippy fixes...");
    let mut clippy = Command::new("cargo");
    clippy.args(["clippy", "--fix", "--allow-dirty", "--allow-staged", "--"]);

    match config.lint.severity.as_str() {
        "deny" => clippy.args(["-D", "warnings"]),
        "warn" => clippy.args(["-W", "warnings"]),
        "allow" => clippy.args(["-A", "warnings"]),
        _ => clippy.args(["-D", "warnings"]),
    };

    for lint in &config.lint.extra_lints {
        clippy.args(["-W", lint]);
    }

    let status = clippy.status().context(
        "failed to run cargo clippy --fix\n\
         hint: install clippy with `rustup component add clippy`",
    )?;
    if !status.success() {
        crate::output::warn("clippy fix encountered issues, continuing...");
    }

    // Step 3: cargo fmt
    crate::output::info("formatting code...");
    let mut fmt = Command::new("cargo");
    fmt.arg("fmt");

    for arg in &config.fmt.extra_args {
        fmt.arg(arg);
    }

    let status = fmt.status().context(
        "failed to run cargo fmt\n\
         hint: install rustfmt with `rustup component add rustfmt`",
    )?;
    if !status.success() {
        anyhow::bail!("formatting failed");
    }

    crate::output::success("all auto-fixes applied");
    timer.finish();
    Ok(())
}
