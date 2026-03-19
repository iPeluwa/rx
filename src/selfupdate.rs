use anyhow::{Context, Result};
use std::process::Command;

const REPO: &str = "iPeluwa/rx";

pub fn self_update() -> Result<()> {
    crate::output::info("checking for rx updates...");

    // Try to get current version
    let current = env!("CARGO_PKG_VERSION");
    crate::output::step("current", current);

    // Check if we can use the install script
    let has_curl = Command::new("curl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_curl {
        // Use the install script which handles architecture detection
        let pb = crate::output::spinner("downloading latest release...");
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "curl -fsSL https://raw.githubusercontent.com/{REPO}/master/install.sh | sh"
            ))
            .status()
            .context("failed to run install script")?;
        pb.finish_and_clear();

        if !status.success() {
            anyhow::bail!(
                "self-update via install script failed\n\
                 hint: try manually with: cargo install --git https://github.com/{REPO}.git"
            );
        }
    } else {
        // Fall back to cargo install
        crate::output::info("updating via cargo install...");
        let status = Command::new("cargo")
            .args([
                "install",
                "--git",
                &format!("https://github.com/{REPO}.git"),
            ])
            .status()
            .context("failed to run cargo install")?;
        if !status.success() {
            anyhow::bail!("self-update via cargo install failed");
        }
    }

    crate::output::success("rx updated successfully");
    Ok(())
}
