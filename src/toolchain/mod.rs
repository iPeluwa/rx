use anyhow::{Context, Result};
use std::process::Command;

use crate::cli::ToolchainCommand;

fn ensure_rustup() -> Result<()> {
    let output = Command::new("rustup").arg("--version").output();
    match output {
        Ok(o) if o.status.success() => Ok(()),
        _ => anyhow::bail!(
            "rustup is not installed. Install it from https://rustup.rs and try again."
        ),
    }
}

fn install(version: &str) -> Result<()> {
    ensure_rustup()?;
    eprintln!("[rx] installing toolchain: {version}");
    let status = Command::new("rustup")
        .args(["toolchain", "install", version])
        .status()
        .context("failed to run rustup")?;
    if !status.success() {
        anyhow::bail!("failed to install toolchain {version}");
    }
    Ok(())
}

fn use_toolchain(version: &str) -> Result<()> {
    ensure_rustup()?;
    eprintln!("[rx] setting default toolchain: {version}");
    let status = Command::new("rustup")
        .args(["default", version])
        .status()
        .context("failed to run rustup")?;
    if !status.success() {
        anyhow::bail!("failed to set default toolchain {version}");
    }
    Ok(())
}

fn list() -> Result<()> {
    ensure_rustup()?;
    let status = Command::new("rustup")
        .args(["toolchain", "list"])
        .status()
        .context("failed to run rustup")?;
    if !status.success() {
        anyhow::bail!("failed to list toolchains");
    }
    Ok(())
}

fn update() -> Result<()> {
    ensure_rustup()?;
    eprintln!("[rx] updating all toolchains...");
    let status = Command::new("rustup")
        .arg("update")
        .status()
        .context("failed to run rustup")?;
    if !status.success() {
        anyhow::bail!("failed to update toolchains");
    }
    Ok(())
}

pub fn dispatch(cmd: ToolchainCommand) -> Result<()> {
    match cmd {
        ToolchainCommand::Install { version } => install(&version),
        ToolchainCommand::Use { version } => use_toolchain(&version),
        ToolchainCommand::List => list(),
        ToolchainCommand::Update => update(),
    }
}
