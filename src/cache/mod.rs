use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::CacheCommand;

// ---------------------------------------------------------------------------
// Cache index (tracks all stored artifacts for GC and status)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct ArtifactEntry {
    content_hash: String,
    last_accessed: DateTime<Utc>,
    size: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct CacheIndex {
    artifacts: HashMap<String, ArtifactEntry>,
}

pub fn cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".rx").join("cache"))
}

fn index_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("index.toml"))
}

fn load_index() -> Result<CacheIndex> {
    let path = index_path()?;
    if !path.exists() {
        return Ok(CacheIndex::default());
    }
    let contents = fs::read_to_string(&path).context("failed to read cache index")?;
    toml::from_str(&contents).context("failed to parse cache index")
}

fn save_index(index: &CacheIndex) -> Result<()> {
    let path = index_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(index)?;
    fs::write(&path, contents)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Build fingerprinting
// ---------------------------------------------------------------------------

/// Compute a fingerprint for the current project state.
/// Inputs: Cargo.toml, Cargo.lock, all .rs source files, profile, rustflags.
pub fn compute_build_fingerprint(
    project_root: &Path,
    profile: &str,
    rustflags: Option<&str>,
) -> Result<String> {
    let mut hasher = Sha256::new();

    // Hash profile and flags
    hasher.update(profile.as_bytes());
    hasher.update(b"\0");
    if let Some(flags) = rustflags {
        hasher.update(flags.as_bytes());
    }
    hasher.update(b"\0");

    // Hash Cargo.toml
    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        hasher.update(fs::read(&cargo_toml)?);
    }
    hasher.update(b"\0");

    // Hash Cargo.lock
    let cargo_lock = project_root.join("Cargo.lock");
    if cargo_lock.exists() {
        hasher.update(fs::read(&cargo_lock)?);
    }
    hasher.update(b"\0");

    // Hash all .rs files sorted for determinism
    let src_dir = project_root.join("src");
    if src_dir.exists() {
        let mut rs_files: Vec<PathBuf> = WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "rs")
            })
            .map(|e| e.into_path())
            .collect();
        rs_files.sort();

        for file in &rs_files {
            // Include the relative path so renames invalidate
            let rel = file.strip_prefix(project_root).unwrap_or(file);
            hasher.update(rel.to_string_lossy().as_bytes());
            hasher.update(b"\0");
            hasher.update(fs::read(file)?);
            hasher.update(b"\0");
        }
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Directory where cached build outputs for a given fingerprint are stored.
fn build_cache_dir(fingerprint: &str) -> Result<PathBuf> {
    Ok(cache_dir()?.join("builds").join(&fingerprint[..2]).join(fingerprint))
}

/// Check if we have cached build outputs for this fingerprint.
/// Returns the cache directory if it exists and contains files.
pub fn lookup_build(fingerprint: &str) -> Result<Option<PathBuf>> {
    let dir = build_cache_dir(fingerprint)?;
    if dir.exists() && fs::read_dir(&dir)?.next().is_some() {
        // Update last-accessed in the index
        let mut index = load_index()?;
        if let Some(entry) = index.artifacts.get_mut(fingerprint) {
            entry.last_accessed = Utc::now();
            save_index(&index)?;
        }
        Ok(Some(dir))
    } else {
        Ok(None)
    }
}

/// Store build outputs into the cache.
/// `artifacts` is a list of (filename, source_path) pairs — typically final
/// binaries, dylibs, and rlibs from target/{profile}/.
pub fn store_build(fingerprint: &str, artifacts: &[(String, PathBuf)]) -> Result<PathBuf> {
    let dir = build_cache_dir(fingerprint)?;
    fs::create_dir_all(&dir)?;

    let mut total_size: u64 = 0;
    for (name, source) in artifacts {
        let dest = dir.join(name);
        // Try hardlink first (same filesystem = free), fall back to copy
        if fs::hard_link(source, &dest).is_err() {
            fs::copy(source, &dest).with_context(|| {
                format!("failed to cache artifact {name}")
            })?;
        }
        total_size += fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    }

    let mut index = load_index()?;
    index.artifacts.insert(
        fingerprint.to_string(),
        ArtifactEntry {
            content_hash: fingerprint.to_string(),
            last_accessed: Utc::now(),
            size: total_size,
        },
    );
    save_index(&index)?;

    Ok(dir)
}

/// Restore cached artifacts into the target directory.
/// Uses hardlinks when possible.
pub fn restore_build(cache_path: &Path, target_dir: &Path) -> Result<usize> {
    fs::create_dir_all(target_dir)?;
    let mut count = 0;

    for entry in fs::read_dir(cache_path)? {
        let entry = entry?;
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let name = entry.file_name();
        let dest = target_dir.join(&name);

        // Remove existing file so we can link
        if dest.exists() {
            fs::remove_file(&dest).ok();
        }

        if fs::hard_link(&src, &dest).is_err() {
            fs::copy(&src, &dest)?;
        }
        count += 1;
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// CLI subcommands
// ---------------------------------------------------------------------------

fn status() -> Result<()> {
    let dir = cache_dir()?;
    if !dir.exists() {
        println!("Cache directory does not exist yet: {}", dir.display());
        return Ok(());
    }

    let index = load_index()?;
    let total_size: u64 = index.artifacts.values().map(|e| e.size).sum();
    let count = index.artifacts.len();

    let disk_size: u64 = WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum();

    println!("Cache: {}", dir.display());
    println!("Indexed artifacts: {count}");
    println!(
        "Indexed size:      {:.1} MB",
        total_size as f64 / 1_048_576.0
    );
    println!(
        "Disk usage:        {:.1} MB",
        disk_size as f64 / 1_048_576.0
    );
    Ok(())
}

fn gc(older_than_days: u32) -> Result<()> {
    let mut index = load_index()?;
    let cutoff = Utc::now() - chrono::Duration::days(i64::from(older_than_days));
    let stale_keys: Vec<String> = index
        .artifacts
        .iter()
        .filter(|(_, entry)| entry.last_accessed < cutoff)
        .map(|(key, _)| key.clone())
        .collect();

    let mut freed: u64 = 0;
    for key in &stale_keys {
        if let Some(entry) = index.artifacts.remove(key) {
            freed += entry.size;
            // Remove build cache directory
            if let Ok(dir) = build_cache_dir(&entry.content_hash) {
                if dir.exists() {
                    fs::remove_dir_all(&dir).ok();
                }
            }
        }
    }

    save_index(&index)?;
    println!(
        "Removed {} stale artifacts, freed {:.1} MB",
        stale_keys.len(),
        freed as f64 / 1_048_576.0
    );
    Ok(())
}

fn purge() -> Result<()> {
    let dir = cache_dir()?;
    if dir.exists() {
        fs::remove_dir_all(&dir).context("failed to purge cache")?;
        println!("Cache purged: {}", dir.display());
    } else {
        println!("Nothing to purge.");
    }
    Ok(())
}

pub fn dispatch(cmd: CacheCommand) -> Result<()> {
    match cmd {
        CacheCommand::Status => status(),
        CacheCommand::Gc { older_than } => gc(older_than),
        CacheCommand::Purge => purge(),
    }
}

pub fn clean(gc_cache: bool) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .arg("clean")
        .status()
        .context("failed to run cargo clean")?;
    if !status.success() {
        anyhow::bail!("cargo clean failed");
    }
    println!("Cleaned local target/ directory.");

    if gc_cache {
        println!("Running global cache GC...");
        gc(30)?;
    }
    Ok(())
}
