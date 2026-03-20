//! Build sandbox: isolated builds to detect undeclared dependencies.
//!
//! Runs `cargo build` in a clean, minimal environment to ensure the project
//! doesn't accidentally depend on globally-installed tools or env vars.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Run a sandboxed build that strips the environment to detect undeclared dependencies.
pub fn sandboxed_build(release: bool) -> Result<()> {
    crate::output::info("running sandboxed build (clean environment)...");

    // Create a temporary target directory
    let tmp = std::env::temp_dir().join("rx-sandbox-build");
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    fs::create_dir_all(&tmp)?;

    // Build with a minimal environment
    let minimal_env = build_minimal_env();

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target-dir").arg(&tmp).env_clear();

    // Only pass essential environment variables
    for (key, value) in &minimal_env {
        cmd.env(key, value);
    }

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("failed to run sandboxed build")?;

    // Clean up
    fs::remove_dir_all(&tmp).ok();

    if status.success() {
        crate::output::success("sandboxed build passed — no undeclared dependencies detected");
    } else {
        anyhow::bail!(
            "sandboxed build failed — your project may depend on:\n  \
             • globally-installed tools not in Cargo.toml\n  \
             • environment variables not declared in rx.toml\n  \
             • implicit system dependencies\n  \
             hint: check the error output above"
        );
    }

    Ok(())
}

/// Verify that Cargo.lock is consistent with Cargo.toml.
pub fn check_lockfile() -> Result<()> {
    crate::output::info("checking Cargo.lock consistency...");

    let status = Command::new("cargo")
        .args(["generate-lockfile", "--check"])
        .output();

    // --check is available in recent cargo; fall back to update --dry-run
    match status {
        Ok(output) if output.status.success() => {
            crate::output::success("Cargo.lock is up to date");
            Ok(())
        }
        _ => {
            // Fallback: compare lockfile before/after update
            let _before = fs::read_to_string("Cargo.lock").unwrap_or_default();

            let output = Command::new("cargo")
                .args(["update", "--dry-run"])
                .output()
                .context("failed to run cargo update --dry-run")?;

            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Updating") || stderr.contains("Adding") {
                crate::output::warn(
                    "Cargo.lock may be out of date\n  \
                     hint: run `cargo update` to refresh it",
                );
            } else {
                crate::output::success("Cargo.lock is up to date");
            }

            Ok(())
        }
    }
}

/// Build a minimal environment for sandboxed builds.
fn build_minimal_env() -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Essential paths
    if let Ok(home) = std::env::var("HOME") {
        env.insert("HOME".into(), home.clone());
        // Cargo and rustup need these
        env.insert("CARGO_HOME".into(), format!("{home}/.cargo"));
        env.insert("RUSTUP_HOME".into(), format!("{home}/.rustup"));
    }

    // PATH: only include cargo/rustup bin dirs
    let mut paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{home}/.cargo/bin"));
        paths.push(format!("{home}/.rustup/shims"));
    }
    // Add minimal system paths
    paths.extend([
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ]);
    let separator = if cfg!(windows) { ";" } else { ":" };
    env.insert("PATH".into(), paths.join(separator));

    // Cargo needs these
    env.insert("CARGO_TERM_COLOR".into(), "always".into());

    // Temp directory
    if let Ok(tmp) = std::env::var("TMPDIR") {
        env.insert("TMPDIR".into(), tmp);
    }

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_env_contains_path() {
        let env = build_minimal_env();
        assert!(env.contains_key("PATH"), "expected PATH in minimal env");
    }

    #[test]
    fn minimal_env_path_contains_system_dirs() {
        let env = build_minimal_env();
        let path = env.get("PATH").expect("PATH missing");
        assert!(path.contains("/usr/bin"), "PATH should contain /usr/bin");
        assert!(
            path.contains("/usr/local/bin"),
            "PATH should contain /usr/local/bin"
        );
    }

    #[test]
    fn minimal_env_contains_cargo_term_color() {
        let env = build_minimal_env();
        assert_eq!(
            env.get("CARGO_TERM_COLOR").map(|s| s.as_str()),
            Some("always")
        );
    }
}
