use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!(
                "could not find Cargo.toml in any parent directory\n\
                 hint: run this command from inside a Rust project"
            );
        }
    }
}

fn package_name(project_root: &std::path::Path) -> Result<String> {
    let contents =
        fs::read_to_string(project_root.join("Cargo.toml")).context("failed to read Cargo.toml")?;
    let table: toml::Table = toml::from_str(&contents)?;
    table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .context("could not read package name from Cargo.toml")
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn size(release: bool) -> Result<()> {
    let project_root = find_project_root()?;
    let profile = if release { "release" } else { "debug" };

    // Build first to ensure binary exists
    crate::output::info(&format!("building ({profile}) to measure size..."));
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().context("failed to run cargo build")?;
    if !status.success() {
        anyhow::bail!("build failed");
    }

    let pkg_name = package_name(&project_root)?;
    let binary = project_root.join("target").join(profile).join(&pkg_name);

    if !binary.exists() {
        anyhow::bail!(
            "binary not found at {}\n\
             hint: does this project produce a binary?",
            binary.display()
        );
    }

    let meta = fs::metadata(&binary)?;
    let bytes = meta.len();

    println!("{}", "Binary size".bold());
    println!("  {} {}", "Binary:".dimmed(), binary.display());
    println!("  {} {}", "Profile:".dimmed(), profile);
    println!(
        "  {} {}",
        "Size:".dimmed(),
        format_size(bytes).green().bold()
    );

    // Try cargo-bloat for detailed breakdown
    let has_bloat = Command::new("cargo")
        .args(["bloat", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_bloat {
        println!("\n{}", "Top functions by size (cargo-bloat):".bold());
        let mut bloat = Command::new("cargo");
        bloat.args(["bloat", "--crates", "-n", "10"]);
        if release {
            bloat.arg("--release");
        }
        let _ = bloat.status();
    } else {
        println!(
            "\n  {}",
            "install cargo-bloat for detailed size analysis: cargo install cargo-bloat".dimmed()
        );
    }

    Ok(())
}
