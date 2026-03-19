use anyhow::{Context, Result};
use std::process::Command;

pub fn publish(package: Option<&str>, dry_run: bool) -> Result<()> {
    if let Some(pkg) = package {
        publish_single(pkg, dry_run)
    } else {
        publish_workspace(dry_run)
    }
}

fn publish_single(package: &str, dry_run: bool) -> Result<()> {
    crate::output::info(&format!("publishing {package}..."));
    let mut cmd = Command::new("cargo");
    cmd.args(["publish", "--package", package]);
    if dry_run {
        cmd.arg("--dry-run");
    }

    let status = cmd.status().context("failed to run cargo publish")?;
    if !status.success() {
        anyhow::bail!("publish failed for {package}");
    }
    if dry_run {
        crate::output::success(&format!("{package} dry-run passed"));
    } else {
        crate::output::success(&format!("{package} published"));
    }
    Ok(())
}

fn publish_workspace(dry_run: bool) -> Result<()> {
    let graph = crate::workspace::resolve_workspace()?;
    let sorted = crate::workspace::topo_sort(&graph)?;

    crate::output::info(&format!(
        "publishing {} workspace members in dependency order...",
        sorted.len()
    ));

    for member in sorted {
        crate::output::info(&format!("publishing {}...", member.name));
        let mut cmd = Command::new("cargo");
        cmd.args(["publish", "--package", &member.name]);
        if dry_run {
            cmd.arg("--dry-run");
        }

        let status = cmd
            .status()
            .with_context(|| format!("failed to publish {}", member.name))?;

        if !status.success() {
            anyhow::bail!("publish failed for {}", member.name);
        }

        if !dry_run {
            // Give crates.io time to index between publishes
            crate::output::info("waiting for crates.io to index...");
            std::thread::sleep(std::time::Duration::from_secs(15));
        }
    }

    crate::output::success("all packages published");
    Ok(())
}
