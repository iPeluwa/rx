//! Private registry support: configure authentication for private crate registries.
//!
//! Supports Cargo's native registry config plus rx.toml convenience wrappers.

use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

/// Configure authentication for a private registry.
pub fn login(registry: &str, token: Option<&str>) -> Result<()> {
    crate::output::info(&format!("configuring registry: {registry}"));

    match token {
        Some(tok) => {
            let status = Command::new("cargo")
                .args(["login", "--registry", registry, tok])
                .status()
                .context("failed to run cargo login")?;
            if !status.success() {
                anyhow::bail!("failed to authenticate with registry {registry}");
            }
            crate::output::success(&format!("authenticated with {registry}"));
        }
        None => {
            // Interactive login
            let status = Command::new("cargo")
                .args(["login", "--registry", registry])
                .status()
                .context("failed to run cargo login")?;
            if !status.success() {
                anyhow::bail!("failed to authenticate with registry {registry}");
            }
        }
    }

    Ok(())
}

/// List configured registries from .cargo/config.toml.
pub fn list_registries() -> Result<()> {
    use owo_colors::OwoColorize;

    println!("{}", "Configured Registries".bold());
    println!("{}", "─".repeat(60));

    // Default registry
    println!("  {} {}", "crates.io".cyan(), "(default)".dimmed());

    // Check .cargo/config.toml for additional registries
    let config_paths = [
        std::env::current_dir()
            .unwrap_or_default()
            .join(".cargo")
            .join("config.toml"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cargo")
            .join("config.toml"),
    ];

    for config_path in &config_paths {
        if config_path.exists() {
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(table) = contents.parse::<toml::Table>() {
                    if let Some(registries) = table.get("registries") {
                        if let Some(reg_table) = registries.as_table() {
                            for (name, value) in reg_table {
                                let index = value
                                    .get("index")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("(no index)");
                                println!("  {} {}", name.cyan(), index.dimmed());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for credential files
    let cred_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".cargo")
        .join("credentials.toml");
    if cred_path.exists() {
        println!(
            "\n  {} credentials found at {}",
            "✓".green(),
            cred_path.display()
        );
    } else {
        println!("\n  {} no credentials file found", "·".dimmed());
    }

    Ok(())
}

/// Add a registry to the project's .cargo/config.toml.
pub fn add_registry(name: &str, index: &str) -> Result<()> {
    let cargo_dir = std::env::current_dir()?.join(".cargo");
    fs::create_dir_all(&cargo_dir)?;

    let config_path = cargo_dir.join("config.toml");
    let mut contents = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    // Append registry config
    contents.push_str(&format!("\n[registries.{name}]\nindex = \"{index}\"\n"));

    fs::write(&config_path, contents)?;
    crate::output::success(&format!("added registry '{name}' with index {index}"));
    crate::output::step(
        "hint",
        &format!("run `rx registry login {name}` to authenticate"),
    );
    Ok(())
}
