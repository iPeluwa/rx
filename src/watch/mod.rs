use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;

pub fn watch(cmd: Option<&str>, config: &RxConfig) -> Result<()> {
    let has_watch = Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_watch {
        anyhow::bail!(
            "cargo-watch is not installed. Install it with:\n  cargo install cargo-watch"
        );
    }

    // CLI flag overrides config, config overrides default
    let watch_cmd = cmd.unwrap_or(config.watch.cmd.as_str());
    crate::output::info(&format!(
        "watching for changes (running: cargo {watch_cmd})..."
    ));

    let mut watch = Command::new("cargo");
    watch.args(["watch", "-x", watch_cmd]);

    // Add ignore patterns from config
    for pattern in &config.watch.ignore {
        watch.args(["-i", pattern]);
    }

    let status = watch.status().context("failed to run cargo watch")?;
    if !status.success() {
        anyhow::bail!("watch exited with error");
    }
    Ok(())
}
