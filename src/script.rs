use anyhow::{Context, Result};
use std::process::Command;

use crate::config::RxConfig;

pub fn run_script(name: &str, config: &RxConfig) -> Result<()> {
    let script = config.scripts.get(name).ok_or_else(|| {
        let available: Vec<&String> = config.scripts.keys().collect();
        if available.is_empty() {
            anyhow::anyhow!(
                "no script named `{name}` found\n\
                 hint: define scripts in rx.toml under [scripts]"
            )
        } else {
            let mut sorted: Vec<&str> = available.iter().map(|s| s.as_str()).collect();
            sorted.sort();
            anyhow::anyhow!(
                "no script named `{name}` found\n\
                 available scripts: {}",
                sorted.join(", ")
            )
        }
    })?;

    crate::output::info(&format!("running script `{name}`: {script}"));
    let status = Command::new("sh")
        .arg("-c")
        .arg(script)
        .status()
        .with_context(|| format!("failed to run script `{name}`"))?;

    if !status.success() {
        anyhow::bail!(
            "script `{name}` failed with exit code {}",
            status.code().unwrap_or(1)
        );
    }

    crate::output::success(&format!("script `{name}` completed"));
    Ok(())
}

pub fn list_scripts(config: &RxConfig) -> Result<()> {
    if config.scripts.is_empty() {
        crate::output::info("no scripts defined in rx.toml");
        return Ok(());
    }

    let mut names: Vec<&String> = config.scripts.keys().collect();
    names.sort();

    for name in names {
        let script = &config.scripts[name];
        println!("  {name:<16} {script}");
    }
    Ok(())
}
