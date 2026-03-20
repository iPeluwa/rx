use anyhow::{Context, Result};
use std::process::Command;

pub fn bloat(release: bool, by_crate: bool) -> Result<()> {
    let has_bloat = Command::new("cargo")
        .args(["bloat", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_bloat {
        anyhow::bail!(
            "cargo-bloat is not installed\n\
             hint: install it with: cargo install cargo-bloat"
        );
    }

    crate::output::info("analyzing binary bloat...");

    let mut cmd = Command::new("cargo");
    cmd.arg("bloat");

    if release {
        cmd.arg("--release");
    }

    if by_crate {
        cmd.arg("--crates");
    }

    cmd.args(["-n", "20"]);

    let status = cmd.status().context("failed to run cargo bloat")?;
    if !status.success() {
        anyhow::bail!("bloat analysis failed");
    }
    Ok(())
}
