use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn find_cargo_toml() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let path = dir.join("Cargo.toml");
        if path.exists() {
            return Ok(path);
        }
        if !dir.pop() {
            anyhow::bail!(
                "could not find Cargo.toml\n\
                 hint: run this command from inside a Rust project"
            );
        }
    }
}

fn current_version(cargo_toml: &str) -> Option<String> {
    let table: toml::Table = toml::from_str(cargo_toml).ok()?;
    table
        .get("package")?
        .get("version")?
        .as_str()
        .map(|s| s.to_string())
}

/// Resolve a version specifier: "patch", "minor", "major", or an explicit version.
fn resolve_version(specifier: &str, current: &str) -> Result<String> {
    let parts: Vec<u64> = current
        .split('.')
        .map(|p| p.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| anyhow::anyhow!("current version `{current}` is not valid semver"))?;

    if parts.len() != 3 {
        anyhow::bail!("current version `{current}` is not valid semver (expected X.Y.Z)");
    }

    match specifier {
        "patch" => Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1)),
        "minor" => Ok(format!("{}.{}.0", parts[0], parts[1] + 1)),
        "major" => Ok(format!("{}.0.0", parts[0] + 1)),
        _ => {
            // Validate explicit version
            let new_parts: Vec<&str> = specifier.split('.').collect();
            if new_parts.len() != 3 || !new_parts.iter().all(|p| p.parse::<u64>().is_ok()) {
                anyhow::bail!(
                    "invalid version `{specifier}`\n\
                     hint: use semver (1.2.3) or a bump keyword (patch, minor, major)"
                );
            }
            Ok(specifier.to_string())
        }
    }
}

pub fn release(version_spec: &str, dry_run: bool, no_push: bool) -> Result<()> {
    let cargo_toml_path = find_cargo_toml()?;
    let contents = fs::read_to_string(&cargo_toml_path).context("failed to read Cargo.toml")?;

    let old_version = current_version(&contents)
        .context("could not read version from Cargo.toml [package] section")?;

    let version = resolve_version(version_spec, &old_version)?;

    crate::output::info(&format!("bumping version: {old_version} → {version}"));

    if dry_run {
        crate::output::info("[dry-run] would update Cargo.toml, commit, tag, and push");
        return Ok(());
    }

    // Update version in Cargo.toml
    let new_contents = contents.replace(
        &format!("version = \"{old_version}\""),
        &format!("version = \"{version}\""),
    );

    if new_contents == contents {
        anyhow::bail!(
            "could not find `version = \"{old_version}\"` in Cargo.toml to replace\n\
             hint: ensure the version field is in the standard format"
        );
    }

    fs::write(&cargo_toml_path, &new_contents)
        .with_context(|| format!("failed to write {}", cargo_toml_path.display()))?;

    // Update Cargo.lock
    crate::output::step("release", "updating Cargo.lock...");
    let _ = Command::new("cargo")
        .args(["update", "--workspace"])
        .status();

    // Git commit
    crate::output::step("release", "committing version bump...");
    let status = Command::new("git")
        .args(["add", "Cargo.toml", "Cargo.lock"])
        .status()
        .context("failed to stage files")?;
    if !status.success() {
        anyhow::bail!("git add failed");
    }

    let commit_msg = format!("release v{version}");
    let status = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .status()
        .context("failed to create commit")?;
    if !status.success() {
        anyhow::bail!("git commit failed");
    }

    // Git tag
    let tag = format!("v{version}");
    crate::output::step("release", &format!("creating tag {tag}..."));
    let status = Command::new("git")
        .args(["tag", "-a", &tag, "-m", &format!("Release {tag}")])
        .status()
        .context("failed to create tag")?;
    if !status.success() {
        anyhow::bail!("git tag failed");
    }

    if no_push {
        crate::output::success(&format!(
            "release v{version} prepared locally — push with: git push && git push origin {tag}"
        ));
        return Ok(());
    }

    // Push commit and tag
    crate::output::step("release", "pushing...");
    let status = Command::new("git")
        .args(["push"])
        .status()
        .context("failed to push")?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    let status = Command::new("git")
        .args(["push", "origin", &tag])
        .status()
        .context("failed to push tag")?;
    if !status.success() {
        anyhow::bail!("git push tag failed");
    }

    crate::output::success(&format!("released v{version}"));
    Ok(())
}
