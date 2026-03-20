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

    crate::output::info(&format!("adding dependency: {spec}"));
    let status = cmd.status().context("failed to run cargo add")?;
    if !status.success() {
        anyhow::bail!("failed to add {spec}");
    }
    Ok(())
}

fn remove(name: &str) -> Result<()> {
    crate::output::info(&format!("removing dependency: {name}"));
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
        crate::output::info(&format!("upgrading: {pkg}"));
    } else {
        crate::output::info("upgrading all dependencies...");
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

fn why(name: &str) -> Result<()> {
    crate::output::info(&format!("tracing dependency: {name}"));
    let status = Command::new("cargo")
        .args(["tree", "--invert", "--package", name])
        .status()
        .context("failed to run cargo tree")?;
    if !status.success() {
        anyhow::bail!(
            "could not find `{name}` in the dependency tree\n\
             hint: check the exact crate name with `rx pkg list`"
        );
    }
    Ok(())
}

fn dedupe() -> Result<()> {
    crate::output::info("checking for duplicate dependency versions...");

    let output = Command::new("cargo")
        .args(["tree", "--duplicates"])
        .output()
        .context("failed to run cargo tree")?;

    if !output.status.success() {
        anyhow::bail!("failed to analyze dependency tree");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        crate::output::success("no duplicate dependencies — your tree is clean");
        return Ok(());
    }

    use owo_colors::OwoColorize;

    println!("{}", "Duplicate Dependencies".bold());
    println!("{}", "─".repeat(60));
    println!();

    // Parse duplicates: group by crate name
    let mut current_crate = String::new();
    let mut versions = Vec::new();
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();

    for line in stdout.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            if !current_crate.is_empty() {
                groups.push((current_crate.clone(), versions.clone()));
                versions.clear();
            }
            current_crate = line.to_string();
            versions.push(line.to_string());
        } else if !line.is_empty() {
            versions.push(line.to_string());
        }
    }
    if !current_crate.is_empty() {
        groups.push((current_crate, versions));
    }

    for (name, lines) in &groups {
        println!("  {}", name.yellow());
        for line in lines.iter().skip(1).take(5) {
            println!("    {}", line.dimmed());
        }
        if lines.len() > 6 {
            println!("    {} ... and {} more", "".dimmed(), lines.len() - 6);
        }
        println!();
    }

    println!(
        "{} duplicate crate(s) found",
        groups.len().to_string().yellow()
    );
    println!(
        "{}",
        "hint: use `rx pkg why <crate>` to trace why each version is needed".dimmed()
    );

    Ok(())
}

pub fn dispatch(cmd: PkgCommand) -> Result<()> {
    match cmd {
        PkgCommand::Add { spec, dev, build } => add(&spec, dev, build),
        PkgCommand::Remove { name } => remove(&name),
        PkgCommand::Upgrade { name } => upgrade(name.as_deref()),
        PkgCommand::List => list(),
        PkgCommand::Why { name } => why(&name),
        PkgCommand::Dedupe => dedupe(),
    }
}
