use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;

pub fn lint(fix: bool, config: &RxConfig) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("clippy");

    if fix {
        cmd.arg("--fix");
        cmd.arg("--allow-dirty");
        crate::output::info("applying lint fixes...");
    } else {
        crate::output::info("linting...");
    }

    cmd.arg("--");

    // Severity from config
    match config.lint.severity.as_str() {
        "deny" => cmd.args(["-D", "warnings"]),
        "warn" => cmd.args(["-W", "warnings"]),
        "allow" => cmd.args(["-A", "warnings"]),
        other => {
            crate::output::warn(&format!(
                "unknown lint severity '{other}', defaulting to deny"
            ));
            cmd.args(["-D", "warnings"])
        }
    };

    // Extra lints from config
    for lint in &config.lint.extra_lints {
        cmd.args(["-W", lint]);
    }

    let status = cmd.status().context("failed to run cargo clippy")?;
    if !status.success() {
        anyhow::bail!("lint failed");
    }
    Ok(())
}
