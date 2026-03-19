use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::process::Command;

pub fn upgrade() -> Result<()> {
    // Step 1: Update Rust toolchains
    eprintln!("{} updating Rust toolchains...", "[rx]".cyan().bold());
    let status = Command::new("rustup")
        .arg("update")
        .status()
        .context("failed to run rustup update")?;
    if !status.success() {
        eprintln!(
            "{} rustup update failed, continuing...",
            "[rx]".yellow().bold()
        );
    }

    // Step 2: Update dependencies
    eprintln!("{} updating dependencies...", "[rx]".cyan().bold());
    let status = Command::new("cargo")
        .arg("update")
        .status()
        .context("failed to run cargo update")?;
    if !status.success() {
        anyhow::bail!("cargo update failed");
    }

    // Step 3: Check for outdated deps (informational)
    eprintln!(
        "{} checking for outdated dependencies...",
        "[rx]".cyan().bold()
    );
    let output = Command::new("cargo")
        .args(["outdated", "--root-deps-only"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() || stdout.contains("All dependencies are up to date") {
                eprintln!("{} all dependencies are up to date", "[rx]".green().bold());
            } else {
                println!("{stdout}");
            }
        }
        _ => {
            eprintln!(
                "{} install cargo-outdated for dependency freshness checks",
                "[rx]".dimmed()
            );
        }
    }

    eprintln!("{} upgrade complete", "[rx]".green().bold());
    Ok(())
}
