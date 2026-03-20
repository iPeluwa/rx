use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;
use crate::output::Timer;

pub fn lint(fix: bool, config: &RxConfig) -> Result<()> {
    let timer = Timer::start("lint");
    let start = std::time::Instant::now();
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
                "unknown lint severity '{other}', defaulting to deny\n\
                 hint: valid values are \"deny\", \"warn\", \"allow\" in rx.toml [lint] section"
            ));
            cmd.args(["-D", "warnings"])
        }
    };

    for lint in &config.lint.extra_lints {
        cmd.args(["-W", lint]);
    }

    let status = cmd.status().context(
        "failed to run cargo clippy\n\
         hint: install clippy with `rustup component add clippy`",
    )?;
    if !status.success() {
        crate::stats::record("lint", start, false);
        if fix {
            anyhow::bail!("lint fix failed — some issues may require manual attention");
        }
        anyhow::bail!("lint failed — run `rx lint --fix` to auto-fix what's possible");
    }
    crate::stats::record("lint", start, true);
    timer.finish();
    Ok(())
}
