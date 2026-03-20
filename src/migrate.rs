use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

use crate::config::RxConfig;

/// Detect project characteristics and generate a sensible rx.toml.
pub fn migrate() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let rx_toml = cwd.join("rx.toml");

    if rx_toml.exists() {
        anyhow::bail!(
            "rx.toml already exists\n\
             hint: delete it first if you want to regenerate, or edit it directly"
        );
    }

    let cargo_toml = cwd.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!(
            "no Cargo.toml found in current directory\n\
             hint: run this from inside a Rust project"
        );
    }

    crate::output::info("analyzing project...");

    let mut config = RxConfig::default();
    let contents = fs::read_to_string(&cargo_toml).context("failed to read Cargo.toml")?;

    // Detect workspace
    let is_workspace = contents.contains("[workspace]");
    if is_workspace {
        crate::output::step("detect", "workspace project");
        config.scripts.insert(
            "ci".into(),
            "cargo fmt --check && cargo clippy -- -D warnings && cargo test".into(),
        );
    }

    // Detect available linker
    if has_tool("mold") {
        crate::output::step("detect", "mold linker available");
        config.build.linker = "mold".into();
    } else if has_tool("lld") {
        crate::output::step("detect", "lld linker available");
        config.build.linker = "lld".into();
    }

    // Detect test runner
    if has_cargo_tool("nextest") {
        crate::output::step("detect", "cargo-nextest available");
        config.test.runner = "nextest".into();
    }

    // Detect existing scripts/config files
    if cwd.join("Makefile").exists() || cwd.join("makefile").exists() {
        crate::output::step(
            "detect",
            "Makefile found — consider migrating targets to [scripts]",
        );
    }

    if cwd.join("justfile").exists() {
        crate::output::step(
            "detect",
            "justfile found — consider migrating recipes to [scripts]",
        );
    }

    // Detect common patterns from Cargo.toml
    if contents.contains("criterion") || contents.contains("[bench]") {
        config.scripts.insert("bench".into(), "cargo bench".into());
        crate::output::step("detect", "benchmarks found");
    }

    // Detect if RUST_BACKTRACE is commonly needed
    if contents.contains("anyhow") || contents.contains("eyre") {
        config.env.insert("RUST_BACKTRACE".into(), "1".into());
        crate::output::step(
            "detect",
            "error handling crate found — enabling RUST_BACKTRACE",
        );
    }

    // Detect watch patterns
    if cwd.join("assets").exists() || cwd.join("static").exists() {
        config.watch.ignore.push("assets/**".into());
        crate::output::step("detect", "assets directory found — added to watch ignore");
    }

    // Write config
    let toml_str = toml::to_string_pretty(&config).context("failed to serialize config")?;
    fs::write(&rx_toml, &toml_str)?;

    crate::output::success("created rx.toml from project analysis");
    crate::output::step("hint", "review and customize rx.toml for your workflow");
    Ok(())
}

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_cargo_tool(name: &str) -> bool {
    Command::new("cargo")
        .args([name, "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
