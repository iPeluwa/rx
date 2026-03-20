use anyhow::{Context, Result};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use time::OffsetDateTime;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use crate::cli::CacheCommand;

// ---------------------------------------------------------------------------
// Cache index
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct ArtifactEntry {
    content_hash: String,
    #[serde(with = "time::serde::rfc3339")]
    last_accessed: OffsetDateTime,
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
    let hash = xxh3_128(project_root.to_string_lossy().as_bytes());
    let key = format!("{hash:016x}");
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

    let mut config_input = profile.as_bytes().to_vec();
    config_input.push(0);
    if let Some(flags) = rustflags {
        config_input.extend_from_slice(flags.as_bytes());
    }
    let config_hash = format!("{:016x}", xxh3_128(&config_input));

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

    let mut buf = Vec::new();

    // Config prefix (for mtime fast-path matching)
    let mut config_input = profile.as_bytes().to_vec();
    config_input.push(0);
    if let Some(flags) = rustflags {
        config_input.extend_from_slice(flags.as_bytes());
    }
    let config_prefix = format!("{:016x}", xxh3_128(&config_input));

    buf.extend_from_slice(profile.as_bytes());
    buf.push(0);
    if let Some(flags) = rustflags {
        buf.extend_from_slice(flags.as_bytes());
    }
    buf.push(0);

    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        buf.extend_from_slice(&fs::read(&cargo_toml)?);
    }
    buf.push(0);

    let cargo_lock = project_root.join("Cargo.lock");
    if cargo_lock.exists() {
        buf.extend_from_slice(&fs::read(&cargo_lock)?);
    }
    buf.push(0);

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
            buf.extend_from_slice(rel.to_string_lossy().as_bytes());
            buf.push(0);
            // Use mmap for large files to avoid read syscall overhead
            if let Ok(f) = fs::File::open(file) {
                if let Ok(metadata) = f.metadata() {
                    if metadata.len() > 0 {
                        // SAFETY: file is read-only and we don't hold the mapping across writes
                        if let Ok(mmap) = unsafe { Mmap::map(&f) } {
                            buf.extend_from_slice(&mmap);
                        } else {
                            buf.extend_from_slice(&fs::read(file)?);
                        }
                    }
                }
            }
            buf.push(0);
        }
    }

    let content_hash = format!("{:032x}", xxh3_128(&buf));
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

/// Compute a semantic fingerprint that only changes when the public API changes.
/// This is useful for workspace builds: if an upstream crate's public API hasn't
/// changed, downstream crates don't need to rebuild even if the upstream's
/// implementation changed.
pub fn compute_semantic_fingerprint(project_root: &Path) -> Result<String> {
    let mut buf = Vec::new();

    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        buf.extend_from_slice(&fs::read(&cargo_toml)?);
    }
    buf.push(0);

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
            buf.extend_from_slice(rel.to_string_lossy().as_bytes());
            buf.push(0);
            // Use semantic hash (public API only) instead of full content
            let hash = crate::semantic_hash::semantic_hash_file(file);
            buf.extend_from_slice(&hash.to_le_bytes());
            buf.push(0);
        }
    }

    Ok(format!("{:032x}", xxh3_128(&buf)))
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
            entry.last_accessed = OffsetDateTime::now_utc();
            save_index(&index)?;
        }
        Ok(Some(dir))
    } else {
        Ok(None)
    }
}

/// Copy a single file using the best available strategy:
/// reflink (CoW on APFS/btrfs) -> hard link -> regular copy.
fn copy_file_fast(src: &Path, dest: &Path) -> Result<()> {
    // Try reflink first (copy-on-write, instant on APFS/btrfs)
    if reflink_copy::reflink(src, dest).is_ok() {
        return Ok(());
    }
    // Fall back to hard link (shares inode, zero copy)
    if fs::hard_link(src, dest).is_ok() {
        return Ok(());
    }
    // Fall back to regular copy
    fs::copy(src, dest)
        .with_context(|| format!("failed to copy {} -> {}", src.display(), dest.display()))?;
    Ok(())
}

pub fn store_build(fingerprint: &str, artifacts: &[(String, PathBuf)]) -> Result<PathBuf> {
    use rayon::prelude::*;

    let _lock = FileLock::acquire()?;

    // Write artifacts to a temp dir first, then rename for atomicity
    let final_dir = build_cache_dir(fingerprint)?;
    let staging_dir = final_dir.with_extension("staging");

    // Clean up any leftover staging dir from a previous interrupted store
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).ok();
    }
    fs::create_dir_all(&staging_dir)?;

    // Copy artifacts in parallel using reflink -> hardlink -> copy
    let total_size: u64 = artifacts
        .par_iter()
        .map(|(name, source)| -> Result<u64> {
            let dest = staging_dir.join(name);
            copy_file_fast(source, &dest)?;
            Ok(fs::metadata(&dest).map(|m| m.len()).unwrap_or(0))
        })
        .try_reduce(|| 0u64, |a, b| Ok(a + b))?;

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
            last_accessed: OffsetDateTime::now_utc(),
            size: total_size,
        },
    );
    save_index(&index)?;

    Ok(final_dir)
}

pub fn restore_build(cache_path: &Path, target_dir: &Path) -> Result<usize> {
    use rayon::prelude::*;

    fs::create_dir_all(target_dir)?;

    // Collect entries first, then restore in parallel
    let entries: Vec<_> = fs::read_dir(cache_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    let count: usize = entries
        .par_iter()
        .map(|entry| -> Result<usize> {
            let src = entry.path();
            let dest = target_dir.join(entry.file_name());

            if dest.exists() {
                fs::remove_file(&dest).ok();
            }

            copy_file_fast(&src, &dest)?;
            Ok(1)
        })
        .try_reduce(|| 0usize, |a, b| Ok(a + b))?;

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
    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(i64::from(older_than_days));
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
            if let Ok(dir) = build_cache_dir(&entry.content_hash) {
                if dir.exists() {
                    fs::remove_dir_all(&dir).ok();
                }
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

fn export(output: Option<&str>) -> Result<()> {
    let dir = cache_dir()?;
    if !dir.exists() {
        anyhow::bail!("cache directory does not exist — nothing to export");
    }

    let output_path = output.unwrap_or("rx-cache.tar.gz");
    crate::output::info(&format!("exporting cache to {output_path}..."));

    let status = std::process::Command::new("tar")
        .args([
            "czf",
            output_path,
            "--exclude",
            ".lock",
            "-C",
            &dir.to_string_lossy(),
            ".",
        ])
        .status()
        .context("failed to run tar — is it installed?")?;

    if !status.success() {
        anyhow::bail!("tar failed to create archive");
    }

    let size = fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);
    crate::output::success(&format!(
        "cache exported to {output_path} ({:.1} MB)",
        size as f64 / 1_048_576.0
    ));
    Ok(())
}

fn import(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("archive not found: {path}");
    }

    let dir = cache_dir()?;
    fs::create_dir_all(&dir)?;

    // Count artifacts before import
    let before = load_index().map(|i| i.artifacts.len()).unwrap_or(0);

    crate::output::info(&format!("importing cache from {path}..."));

    let status = std::process::Command::new("tar")
        .args(["xzf", path, "-C", &dir.to_string_lossy()])
        .status()
        .context("failed to run tar — is it installed?")?;

    if !status.success() {
        anyhow::bail!("tar failed to extract archive");
    }

    let after = load_index().map(|i| i.artifacts.len()).unwrap_or(0);
    let new_count = after.saturating_sub(before);

    crate::output::success(&format!(
        "cache imported: {after} total artifacts ({new_count} new)"
    ));
    Ok(())
}

pub fn dispatch(cmd: CacheCommand) -> Result<()> {
    match cmd {
        CacheCommand::Status => status(),
        CacheCommand::Gc { older_than } => gc(older_than),
        CacheCommand::Purge => purge(),
        CacheCommand::Export { output } => export(output.as_deref()),
        CacheCommand::Import { path } => import(&path),
    }
}

pub fn clean(gc_cache: bool, all_workspace: bool) -> Result<()> {
    if all_workspace {
        // Clean all workspace member target directories
        if let Ok(graph) = crate::workspace::resolve_workspace() {
            crate::output::info(&format!(
                "cleaning {} workspace members...",
                graph.members.len()
            ));
            for member in &graph.members {
                let status = std::process::Command::new("cargo")
                    .arg("clean")
                    .current_dir(&member.path)
                    .status();
                match status {
                    Ok(s) if s.success() => {
                        crate::output::step(&member.name, "cleaned");
                    }
                    _ => {
                        crate::output::warn(&format!("failed to clean {}", member.name));
                    }
                }
            }
            crate::output::success("cleaned all workspace target/ directories");
        } else {
            // Not a workspace, just clean normally
            let status = std::process::Command::new("cargo")
                .arg("clean")
                .status()
                .context("failed to run cargo clean")?;
            if !status.success() {
                anyhow::bail!("cargo clean failed");
            }
            crate::output::success("cleaned local target/ directory");
        }
    } else {
        let status = std::process::Command::new("cargo")
            .arg("clean")
            .status()
            .context("failed to run cargo clean")?;
        if !status.success() {
            anyhow::bail!("cargo clean failed");
        }
        crate::output::success("cleaned local target/ directory");
    }

    if gc_cache {
        crate::output::info("running global cache GC...");
        gc(30)?;
    }
    Ok(())
}
