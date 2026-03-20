//! MSRV compatibility checking for dependencies.
//!
//! `rx pkg compat` checks that all dependencies are compatible with the
//! project's declared rust-version (MSRV).

use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

/// Check MSRV compatibility for all dependencies.
pub fn check_compat() -> Result<()> {
    let cargo_toml = fs::read_to_string("Cargo.toml").context("failed to read Cargo.toml")?;

    // Extract rust-version
    let msrv = extract_msrv(&cargo_toml);

    match msrv {
        Some(version) => {
            crate::output::info(&format!("checking compatibility with MSRV {version}..."));
            run_msrv_check(&version)
        }
        None => {
            crate::output::warn(
                "no `rust-version` set in Cargo.toml\n  \
                 hint: add `rust-version = \"1.70.0\"` to [package] to enable MSRV checking",
            );
            Ok(())
        }
    }
}

/// Extract rust-version from Cargo.toml content.
fn extract_msrv(cargo_toml: &str) -> Option<String> {
    let table: toml::Table = toml::from_str(cargo_toml).ok()?;
    table
        .get("package")?
        .get("rust-version")?
        .as_str()
        .map(|s| s.to_string())
}

/// Run cargo check with the MSRV toolchain.
fn run_msrv_check(version: &str) -> Result<()> {
    // Check if the MSRV toolchain is installed
    let toolchain = version.to_string();
    let has_toolchain = Command::new("rustup")
        .args(["toolchain", "list"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.starts_with(&toolchain))
        })
        .unwrap_or(false);

    if !has_toolchain {
        crate::output::info(&format!("installing toolchain {version}..."));
        let status = Command::new("rustup")
            .args(["toolchain", "install", &toolchain, "--profile", "minimal"])
            .status()
            .context("failed to install MSRV toolchain")?;
        if !status.success() {
            anyhow::bail!("failed to install toolchain {version}");
        }
    }

    // Run cargo check with the MSRV toolchain
    crate::output::info(&format!("running cargo check with {version}..."));
    let status = Command::new("cargo")
        .args([&format!("+{toolchain}"), "check", "--all-targets"])
        .status()
        .context("failed to run cargo check with MSRV toolchain")?;

    if status.success() {
        crate::output::success(&format!("all code is compatible with MSRV {version}"));
    } else {
        anyhow::bail!(
            "code is not compatible with MSRV {version}\n  \
             hint: check the errors above and update your code or bump rust-version"
        );
    }

    // Also check that dependency versions respect MSRV
    crate::output::info("verifying dependency MSRV compatibility...");
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .output()
        .context("failed to run cargo metadata")?;

    if output.status.success() {
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata")?;

        if let Some(packages) = metadata.get("packages").and_then(|p| p.as_array()) {
            let mut incompatible = Vec::new();
            for pkg in packages {
                if let Some(rv) = pkg.get("rust_version").and_then(|v| v.as_str()) {
                    let pkg_name = pkg
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    if version_lt(version, rv) {
                        incompatible.push(format!("{pkg_name} requires {rv}"));
                    }
                }
            }

            if incompatible.is_empty() {
                crate::output::success("all dependencies are MSRV-compatible");
            } else {
                use owo_colors::OwoColorize;
                crate::output::warn(&format!(
                    "{} dependency(ies) require a newer Rust version:",
                    incompatible.len()
                ));
                for item in &incompatible {
                    eprintln!("  {} {item}", "⚠".yellow());
                }
            }
        }
    }

    Ok(())
}

/// Simple semver comparison: is `a` < `b`?
fn version_lt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(a) < parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(version_lt("1.70.0", "1.75.0"));
        assert!(!version_lt("1.80.0", "1.75.0"));
        assert!(!version_lt("1.75.0", "1.75.0"));
        assert!(version_lt("1.74.1", "1.75.0"));
    }

    #[test]
    fn extract_msrv_from_toml() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
rust-version = "1.70.0"
"#;
        assert_eq!(extract_msrv(toml), Some("1.70.0".to_string()));
    }

    #[test]
    fn extract_msrv_missing() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        assert_eq!(extract_msrv(toml), None);
    }
}
