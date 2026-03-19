use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::cli::CacheCommand;

// ---------------------------------------------------------------------------
// Cache index
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

/// Mtime snapshot for fast-path cache invalidation.
#[derive(Serialize, Deserialize, Default)]
struct MtimeSnapshot {
    /// Maps relative file path -> (mtime_secs, file_size)
    files: HashMap<String, (u64, u64)>,
    /// The fingerprint hash that was computed for this snapshot
    fingerprint: String,
}

pub fn cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".rx").join("cache"))
}

fn index_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("index.toml"))
}

fn lock_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join(".lock"))
}

fn mtime_path(project_root: &Path) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    let key = hex::encode(&hasher.finalize()[..8]);
    Ok(cache_dir()?.join("mtimes").join(format!("{key}.toml")))
}

// ---------------------------------------------------------------------------
// File locking for concurrent access
// ---------------------------------------------------------------------------

struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire() -> Result<Self> {
        let path = lock_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Simple lock: try to create exclusively, retry briefly if locked
        for attempt in 0..50 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    // Write our PID for debugging
                    let _ = write!(f, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(_) if attempt < 49 => {
                    // Check if stale (older than 60s)
                    if let Ok(meta) = fs::metadata(&path) {
                        if let Ok(modified) = meta.modified() {
                            if modified.elapsed().unwrap_or_default().as_secs() > 60 {
                                fs::remove_file(&path).ok();
                                continue;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    anyhow::bail!(
                        "could not acquire cache lock at {}: {e}\n\
                         If no other rx process is running, delete the lock file manually.",
                        path.display()
                    );
                }
            }
        }
        unreachable!()
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        fs::remove_file(&self.path).ok();
    }
}

// ---------------------------------------------------------------------------
// Atomic file writes
// ---------------------------------------------------------------------------

/// Write to a temp file then atomically rename, preventing corruption from
/// interrupted writes.
fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents).with_context(|| format!("failed to write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("failed to rename to {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Index I/O (with lock + atomic writes)
// ---------------------------------------------------------------------------

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
    let contents = toml::to_string_pretty(index)?;
    atomic_write(&path, contents.as_bytes())
}

fn load_mtime_snapshot(project_root: &Path) -> Result<Option<MtimeSnapshot>> {
    let path = mtime_path(project_root)?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&contents)?))
}

fn save_mtime_snapshot(project_root: &Path, snapshot: &MtimeSnapshot) -> Result<()> {
    let path = mtime_path(project_root)?;
    let contents = toml::to_string_pretty(snapshot)?;
    atomic_write(&path, contents.as_bytes())
}

// ---------------------------------------------------------------------------
// Build fingerprinting with mtime fast-path
// ---------------------------------------------------------------------------

fn file_mtime_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Collect all input files with their mtime and size.
fn collect_input_files(project_root: &Path) -> Vec<(String, u64, u64)> {
    let mut files = Vec::new();

    for name in ["Cargo.toml", "Cargo.lock"] {
        let p = project_root.join(name);
        if p.exists() {
            files.push((name.to_string(), file_mtime_secs(&p), file_size(&p)));
        }
    }

    let src_dir = project_root.join("src");
    if src_dir.exists() {
        let mut rs_files: Vec<PathBuf> = WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .map(|e| e.into_path())
            .collect();
        rs_files.sort();

        for file in rs_files {
            let rel = file
                .strip_prefix(project_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .to_string();
            files.push((rel, file_mtime_secs(&file), file_size(&file)));
        }
    }

    files
}

/// Try to use mtime-based fast path. Returns the cached fingerprint if nothing changed.
fn try_mtime_fast_path(
    project_root: &Path,
    profile: &str,
    rustflags: Option<&str>,
) -> Option<String> {
    let snapshot = load_mtime_snapshot(project_root).ok()??;

    let current_files = collect_input_files(project_root);

    let current_map: HashMap<&str, (u64, u64)> = current_files
        .iter()
        .map(|(path, mtime, size)| (path.as_str(), (*mtime, *size)))
        .collect();

    if current_map.len() != snapshot.files.len() {
        return None;
    }

    for (path, (mtime, size)) in &snapshot.files {
        match current_map.get(path.as_str()) {
            Some(&(cur_mtime, cur_size)) if cur_mtime == *mtime && cur_size == *size => {}
            _ => return None,
        }
    }

    let mut config_hasher = Sha256::new();
    config_hasher.update(profile.as_bytes());
    config_hasher.update(b"\0");
    if let Some(flags) = rustflags {
        config_hasher.update(flags.as_bytes());
    }
    let config_hash = hex::encode(&config_hasher.finalize()[..8]);

    if snapshot.fingerprint.starts_with(&config_hash) {
        Some(snapshot.fingerprint.clone())
    } else {
        None
    }
}

/// Compute a fingerprint for the current project state.
/// Uses mtime fast-path when possible, falls back to full content hashing.
pub fn compute_build_fingerprint(
    project_root: &Path,
    profile: &str,
    rustflags: Option<&str>,
) -> Result<String> {
    // Try fast path first
    if let Some(fp) = try_mtime_fast_path(project_root, profile, rustflags) {
        crate::output::verbose("fingerprint: mtime fast-path hit");
        return Ok(fp);
    }

    crate::output::verbose("fingerprint: computing full content hash...");

    let mut hasher = Sha256::new();

    let mut config_hasher = Sha256::new();
    config_hasher.update(profile.as_bytes());
    config_hasher.update(b"\0");
    if let Some(flags) = rustflags {
        config_hasher.update(flags.as_bytes());
    }
    let config_prefix = hex::encode(&config_hasher.finalize()[..8]);

    hasher.update(profile.as_bytes());
    hasher.update(b"\0");
    if let Some(flags) = rustflags {
        hasher.update(flags.as_bytes());
    }
    hasher.update(b"\0");

    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        hasher.update(fs::read(&cargo_toml)?);
    }
    hasher.update(b"\0");

    let cargo_lock = project_root.join("Cargo.lock");
    if cargo_lock.exists() {
        hasher.update(fs::read(&cargo_lock)?);
    }
    hasher.update(b"\0");

    let src_dir = project_root.join("src");
    if src_dir.exists() {
        let mut rs_files: Vec<PathBuf> = WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            .map(|e| e.into_path())
            .collect();
        rs_files.sort();

        for file in &rs_files {
            let rel = file.strip_prefix(project_root).unwrap_or(file);
            hasher.update(rel.to_string_lossy().as_bytes());
            hasher.update(b"\0");
            hasher.update(fs::read(file)?);
            hasher.update(b"\0");
        }
    }

    let content_hash = hex::encode(hasher.finalize());
    let fingerprint = format!("{config_prefix}{content_hash}");

    // Save mtime snapshot for future fast-path
    let input_files = collect_input_files(project_root);
    let files: HashMap<String, (u64, u64)> = input_files
        .into_iter()
        .map(|(path, mtime, size)| (path, (mtime, size)))
        .collect();
    let snapshot = MtimeSnapshot {
        files,
        fingerprint: fingerprint.clone(),
    };
    save_mtime_snapshot(project_root, &snapshot).ok();

    Ok(fingerprint)
}

/// Directory where cached build outputs for a given fingerprint are stored.
fn build_cache_dir(fingerprint: &str) -> Result<PathBuf> {
    Ok(cache_dir()?
        .join("builds")
        .join(&fingerprint[..2])
        .join(fingerprint))
}

pub fn lookup_build(fingerprint: &str) -> Result<Option<PathBuf>> {
    let _lock = FileLock::acquire()?;
    let dir = build_cache_dir(fingerprint)?;
    if dir.exists() && fs::read_dir(&dir)?.next().is_some() {
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

pub fn store_build(fingerprint: &str, artifacts: &[(String, PathBuf)]) -> Result<PathBuf> {
    let _lock = FileLock::acquire()?;

    // Write artifacts to a temp dir first, then rename for atomicity
    let final_dir = build_cache_dir(fingerprint)?;
    let staging_dir = final_dir.with_extension("staging");

    // Clean up any leftover staging dir from a previous interrupted store
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).ok();
    }
    fs::create_dir_all(&staging_dir)?;

    let mut total_size: u64 = 0;
    for (name, source) in artifacts {
        let dest = staging_dir.join(name);
        if fs::hard_link(source, &dest).is_err() {
            fs::copy(source, &dest).with_context(|| format!("failed to cache artifact {name}"))?;
        }
        total_size += fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    }

    // Atomically move staging -> final
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir).ok();
    }
    if let Some(parent) = final_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&staging_dir, &final_dir)
        .with_context(|| format!("failed to finalize cache at {}", final_dir.display()))?;

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

    Ok(final_dir)
}

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
    let _lock = FileLock::acquire()?;
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
            if let Ok(dir) = build_cache_dir(&entry.content_hash)
                && dir.exists()
            {
                fs::remove_dir_all(&dir).ok();
            }
        }
    }

    save_index(&index)?;
    crate::output::success(&format!(
        "removed {} stale artifacts, freed {:.1} MB",
        stale_keys.len(),
        freed as f64 / 1_048_576.0
    ));
    Ok(())
}

fn purge() -> Result<()> {
    let dir = cache_dir()?;
    if dir.exists() {
        fs::remove_dir_all(&dir).context("failed to purge cache")?;
        crate::output::success(&format!("cache purged: {}", dir.display()));
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
    crate::output::success("cleaned local target/ directory");

    if gc_cache {
        crate::output::info("running global cache GC...");
        gc(30)?;
    }
    Ok(())
}
