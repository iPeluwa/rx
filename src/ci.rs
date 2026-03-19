use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;
use crate::output::Timer;

pub fn ci(config: &RxConfig) -> Result<()> {
    let timer = Timer::start("ci");

    // If a "ci" script is defined in rx.toml, run that instead
    if let Some(script) = config.scripts.get("ci") {
        crate::output::info(&format!("running ci script: {script}"));
        let status = Command::new("sh")
            .arg("-c")
            .arg(script)
            .status()
            .context("failed to run ci script")?;
        if !status.success() {
            anyhow::bail!("ci script failed");
        }
        timer.finish();
        return Ok(());
    }

    // Default CI pipeline: fmt check → clippy → test → build
    let steps: &[(&str, &[&str])] = &[
        ("fmt --check", &["fmt", "--check"]),
        ("clippy", &["clippy", "--", "-D", "warnings"]),
        ("test", &["test"]),
        ("build", &["build"]),
    ];

    for (label, args) in steps {
        crate::output::info(&format!("ci: {label}..."));
        let status = Command::new("cargo")
            .args(*args)
            .status()
            .with_context(|| format!("failed to run cargo {label}"))?;
        if !status.success() {
            anyhow::bail!(
                "ci failed at: {label}\n\
                 hint: fix the issue above and re-run `rx ci`"
            );
        }
    }

    crate::output::success("ci passed — all checks green");
    timer.finish();
    Ok(())
}
