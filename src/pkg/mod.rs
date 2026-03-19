use anyhow::{Context, Result};
use std::process::Command;

use crate::cli::PkgCommand;

fn add(spec: &str, dev: bool, build: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("add");

    if dev {
        cmd.arg("--dev");
    }
    if build {
        cmd.arg("--build");
    }

    // Support `serde@1.0` syntax — translate to cargo's `serde@1.0`
    cmd.arg(spec);

    eprintln!("[rx] adding dependency: {spec}");
    let status = cmd.status().context("failed to run cargo add")?;
    if !status.success() {
        anyhow::bail!("failed to add {spec}");
    }
    Ok(())
}

fn remove(name: &str) -> Result<()> {
    eprintln!("[rx] removing dependency: {name}");
    let status = Command::new("cargo")
        .args(["remove", name])
        .status()
        .context("failed to run cargo remove")?;
    if !status.success() {
        anyhow::bail!("failed to remove {name}");
    }
    Ok(())
}

fn upgrade(name: Option<&str>) -> Result<()> {
    // Use `cargo update` for now — could integrate cargo-edit in the future
    let mut cmd = Command::new("cargo");
    cmd.arg("update");

    if let Some(pkg) = name {
        cmd.args(["--package", pkg]);
        eprintln!("[rx] upgrading: {pkg}");
    } else {
        eprintln!("[rx] upgrading all dependencies...");
    }

    let status = cmd.status().context("failed to run cargo update")?;
    if !status.success() {
        anyhow::bail!("upgrade failed");
    }
    Ok(())
}

fn list() -> Result<()> {
    let status = Command::new("cargo")
        .args(["tree", "--depth", "1"])
        .status()
        .context("failed to run cargo tree")?;
    if !status.success() {
        anyhow::bail!("failed to list dependencies");
    }
    Ok(())
}

pub fn dispatch(cmd: PkgCommand) -> Result<()> {
    match cmd {
        PkgCommand::Add { spec, dev, build } => add(&spec, dev, build),
        PkgCommand::Remove { name } => remove(&name),
        PkgCommand::Upgrade { name } => upgrade(name.as_deref()),
        PkgCommand::List => list(),
    }
}
