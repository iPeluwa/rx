use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cache;
use crate::config::RxConfig;
use crate::output::Timer;

/// Detect the fastest available linker on the system.
fn detect_linker() -> Option<&'static str> {
    let candidates = [("mold", "mold"), ("lld", "lld")];
    for (name, bin) in candidates {
        if Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(name);
        }
    }
    None
}

/// Resolve the linker to use based on config.
fn resolve_linker(config: &RxConfig) -> Option<String> {
    match config.build.linker.as_str() {
        "system" => None,
        "mold" => Some("mold".into()),
        "lld" => Some("lld".into()),
        _ => detect_linker().map(|s| s.to_string()), // "auto"
    }
}

/// Build the full RUSTFLAGS string from config + linker.
fn build_rustflags(config: &RxConfig) -> Option<String> {
    let mut flags = Vec::new();

    if let Some(linker) = resolve_linker(config) {
        flags.push(format!("-Clinker={linker}"));
    }

    flags.extend(config.build.rustflags.iter().cloned());

    if flags.is_empty() {
        None
    } else {
        Some(flags.join(" "))
    }
}

fn cargo_cmd(config: &RxConfig) -> Command {
    let mut cmd = Command::new("cargo");
    if let Some(flags) = build_rustflags(config) {
        cmd.env("RUSTFLAGS", flags);
    }
    cmd
}

/// Apply --jobs after the subcommand arg has been added.
fn apply_jobs(cmd: &mut Command, config: &RxConfig) {
    if config.build.jobs > 0 {
        cmd.args(["--jobs", &config.build.jobs.to_string()]);
    }
}

/// Find the project root (directory containing Cargo.toml).
fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!(
                "could not find Cargo.toml in any parent directory\n\
                 hint: run this command from inside a Rust project, or use `rx new <name>` to create one"
            );
        }
    }
}

/// Read the package name from Cargo.toml.
fn package_name(project_root: &Path) -> Result<String> {
    let contents =
        fs::read_to_string(project_root.join("Cargo.toml")).context("failed to read Cargo.toml")?;
    let table: toml::Table = toml::from_str(&contents).context("failed to parse Cargo.toml")?;
    table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .context(
            "could not read package name from Cargo.toml\n\
             hint: ensure [package] section has a `name` field",
        )
}

/// Collect final build artifacts from target/{profile}/.
fn collect_artifacts(target_dir: &Path, profile: &str) -> Result<Vec<(String, PathBuf)>> {
    let out_dir = target_dir.join(profile);
    if !out_dir.exists() {
        return Ok(vec![]);
    }

    let mut artifacts = Vec::new();
    for entry in fs::read_dir(&out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();

        if name.ends_with(".d") || name.ends_with(".fingerprint") || name.starts_with('.') {
            continue;
        }

        let metadata = entry.metadata()?;
        let is_library = name.ends_with(".dylib")
            || name.ends_with(".so")
            || name.ends_with(".a")
            || name.ends_with(".rlib");

        #[cfg(unix)]
        let is_executable = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let is_executable = !name.contains('.');

        if (is_executable || is_library) && metadata.len() > 0 {
            artifacts.push((name, path));
        }
    }

    Ok(artifacts)
}

pub fn build(
    release: bool,
    package: Option<&str>,
    target: Option<&str>,
    config: &RxConfig,
) -> Result<()> {
    let timer = Timer::start("build");
    let project_root = find_project_root()?;
    let profile = if release { "release" } else { "debug" };
    let flags = build_rustflags(config);
    let flags_str = flags.as_deref();

    // Check cache if enabled
    if config.build.cache {
        let fingerprint = cache::compute_build_fingerprint(&project_root, profile, flags_str)?;

        if let Some(cached) = cache::lookup_build(&fingerprint)? {
            let target_dir = project_root.join("target").join(profile);
            let count = cache::restore_build(&cached, &target_dir)?;
            crate::output::success(&format!(
                "cache hit: restored {count} artifact(s) from global cache"
            ));
            return Ok(());
        }
    }

    // Report linker
    if let Some(linker) = resolve_linker(config) {
        crate::output::step("build", &format!("using linker: {linker}"));
    }

    if let Some(t) = target {
        crate::output::step("build", &format!("cross-compiling for {t}"));
    }

    let mut cmd = cargo_cmd(config);
    cmd.arg("build");
    apply_jobs(&mut cmd, config);
    if release {
        cmd.arg("--release");
    }
    if let Some(pkg) = package {
        cmd.args(["--package", pkg]);
    }
    if let Some(t) = target {
        cmd.args(["--target", t]);
    }

    let status = cmd.status().context(
        "failed to run cargo build\n\
         hint: is cargo installed? run `rx doctor` to check",
    )?;
    if !status.success() {
        anyhow::bail!("build failed");
    }

    // Store in cache
    if config.build.cache {
        let fingerprint = cache::compute_build_fingerprint(&project_root, profile, flags_str)?;
        let target_dir = project_root.join("target");
        let artifacts = collect_artifacts(&target_dir, profile)?;
        if !artifacts.is_empty() {
            cache::store_build(&fingerprint, &artifacts)?;
            crate::output::info(&format!(
                "cached {} artifact(s) for future builds",
                artifacts.len()
            ));
        }
    }

    timer.finish();
    Ok(())
}

pub fn run(release: bool, args: &[String], config: &RxConfig) -> Result<()> {
    let project_root = find_project_root()?;
    let profile = if release { "release" } else { "debug" };
    let flags = build_rustflags(config);
    let flags_str = flags.as_deref();

    // Try cache
    let mut needs_build = true;
    if config.build.cache {
        let fingerprint = cache::compute_build_fingerprint(&project_root, profile, flags_str)?;
        if let Some(cached) = cache::lookup_build(&fingerprint)? {
            let target_dir = project_root.join("target").join(profile);
            let count = cache::restore_build(&cached, &target_dir)?;
            crate::output::success(&format!("cache hit: restored {count} artifact(s)"));
            needs_build = false;
        }
    }

    if needs_build {
        if let Some(linker) = resolve_linker(config) {
            crate::output::step("build", &format!("using linker: {linker}"));
        }

        let mut cmd = cargo_cmd(config);
        cmd.arg("build");
        apply_jobs(&mut cmd, config);
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().context(
            "failed to run cargo build\n\
             hint: is cargo installed? run `rx doctor` to check",
        )?;
        if !status.success() {
            anyhow::bail!("build failed");
        }

        if config.build.cache {
            let fingerprint = cache::compute_build_fingerprint(&project_root, profile, flags_str)?;
            let target_dir = project_root.join("target");
            let artifacts = collect_artifacts(&target_dir, profile)?;
            if !artifacts.is_empty() {
                cache::store_build(&fingerprint, &artifacts)?;
                crate::output::info(&format!(
                    "cached {} artifact(s) for future builds",
                    artifacts.len()
                ));
            }
        }
    }

    // Run the binary directly
    let pkg_name = package_name(&project_root)?;
    let binary = project_root.join("target").join(profile).join(&pkg_name);

    if !binary.exists() {
        anyhow::bail!(
            "binary not found at {}\n\
             hint: does this project produce a binary? check [lib] vs [[bin]] in Cargo.toml",
            binary.display()
        );
    }

    let status = Command::new(&binary)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {}", binary.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
