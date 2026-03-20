//! Remote/shared cache: push/pull build artifacts to S3, GCS, or a local path.
//!
//! Enables teams to share build caches across CI runners and developer machines.
//! Artifacts are keyed by fingerprint and stored as compressed tarballs.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::RxConfig;

/// Supported remote cache backends.
#[derive(Debug, Clone)]
pub enum CacheBackend {
    /// AWS S3 bucket (requires `aws` CLI)
    S3 { bucket: String, prefix: String },
    /// Google Cloud Storage (requires `gcloud` CLI)
    Gcs { bucket: String, prefix: String },
    /// Simple local/network path (NFS, shared drive)
    Path { root: PathBuf },
}

impl CacheBackend {
    /// Parse a remote cache URL from config.
    /// Formats: s3://bucket/prefix, gs://bucket/prefix, /path/to/dir
    pub fn from_url(url: &str) -> Result<Self> {
        if let Some(rest) = url.strip_prefix("s3://") {
            let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, "rx-cache"));
            Ok(Self::S3 {
                bucket: bucket.to_string(),
                prefix: prefix.to_string(),
            })
        } else if let Some(rest) = url.strip_prefix("gs://") {
            let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, "rx-cache"));
            Ok(Self::Gcs {
                bucket: bucket.to_string(),
                prefix: prefix.to_string(),
            })
        } else {
            Ok(Self::Path {
                root: PathBuf::from(url),
            })
        }
    }

    /// The remote key path for a fingerprint.
    fn remote_key(&self, fingerprint: &str) -> String {
        match self {
            Self::S3 { prefix, .. } | Self::Gcs { prefix, .. } => {
                format!("{prefix}/{}/{fingerprint}.tar.gz", &fingerprint[..2])
            }
            Self::Path { .. } => {
                format!("{}/{fingerprint}.tar.gz", &fingerprint[..2])
            }
        }
    }
}

/// Push a local cache directory to the remote.
pub fn push(backend: &CacheBackend, fingerprint: &str, local_dir: &Path) -> Result<()> {
    let tarball = create_tarball(local_dir, fingerprint)?;
    let key = backend.remote_key(fingerprint);

    match backend {
        CacheBackend::S3 { bucket, .. } => {
            let dest = format!("s3://{bucket}/{key}");
            crate::output::verbose(&format!("pushing cache to {dest}"));
            let status = Command::new("aws")
                .args(["s3", "cp", &tarball.to_string_lossy(), &dest, "--quiet"])
                .status()
                .context("failed to run aws s3 cp — is the AWS CLI installed?")?;
            if !status.success() {
                anyhow::bail!("failed to push cache to S3");
            }
        }
        CacheBackend::Gcs { bucket, .. } => {
            let dest = format!("gs://{bucket}/{key}");
            crate::output::verbose(&format!("pushing cache to {dest}"));
            let status = Command::new("gsutil")
                .args(["cp", &tarball.to_string_lossy(), &dest])
                .status()
                .context("failed to run gsutil — is the Google Cloud SDK installed?")?;
            if !status.success() {
                anyhow::bail!("failed to push cache to GCS");
            }
        }
        CacheBackend::Path { root } => {
            let dest_dir = root.join(&key[..key.rfind('/').unwrap_or(0)]);
            fs::create_dir_all(&dest_dir)?;
            let dest = root.join(&key);
            fs::copy(&tarball, &dest)
                .with_context(|| format!("failed to copy cache to {}", dest.display()))?;
        }
    }

    // Clean up temp tarball
    fs::remove_file(&tarball).ok();
    crate::output::verbose("cache pushed to remote");
    Ok(())
}

/// Pull a cached artifact from the remote.
pub fn pull(backend: &CacheBackend, fingerprint: &str, target_dir: &Path) -> Result<bool> {
    let key = backend.remote_key(fingerprint);
    let tmp_dir = std::env::temp_dir().join(format!("rx-remote-cache-{}", &fingerprint[..8]));
    let tarball = tmp_dir.join(format!("{fingerprint}.tar.gz"));
    fs::create_dir_all(&tmp_dir)?;

    let downloaded = match backend {
        CacheBackend::S3 { bucket, .. } => {
            let src = format!("s3://{bucket}/{key}");
            Command::new("aws")
                .args(["s3", "cp", &src, &tarball.to_string_lossy(), "--quiet"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        CacheBackend::Gcs { bucket, .. } => {
            let src = format!("gs://{bucket}/{key}");
            Command::new("gsutil")
                .args(["cp", &src, &tarball.to_string_lossy()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        CacheBackend::Path { root } => {
            let src = root.join(&key);
            if src.exists() {
                fs::copy(&src, &tarball).is_ok()
            } else {
                false
            }
        }
    };

    if !downloaded {
        fs::remove_dir_all(&tmp_dir).ok();
        return Ok(false);
    }

    // Extract tarball into target directory
    extract_tarball(&tarball, target_dir)?;

    // Clean up
    fs::remove_dir_all(&tmp_dir).ok();
    crate::output::verbose("cache pulled from remote");
    Ok(true)
}

/// Create a compressed tarball of a directory.
fn create_tarball(dir: &Path, name: &str) -> Result<PathBuf> {
    let tmp_dir = std::env::temp_dir().join("rx-cache-pack");
    fs::create_dir_all(&tmp_dir)?;
    let tarball = tmp_dir.join(format!("{name}.tar.gz"));

    let status = Command::new("tar")
        .args([
            "czf",
            &tarball.to_string_lossy(),
            "-C",
            &dir.to_string_lossy(),
            ".",
        ])
        .status()
        .context("failed to create tarball")?;

    if !status.success() {
        anyhow::bail!("tar failed");
    }
    Ok(tarball)
}

/// Extract a tarball into a directory.
fn extract_tarball(tarball: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    let status = Command::new("tar")
        .args([
            "xzf",
            &tarball.to_string_lossy(),
            "-C",
            &target.to_string_lossy(),
        ])
        .status()
        .context("failed to extract tarball")?;
    if !status.success() {
        anyhow::bail!("tar extraction failed");
    }
    Ok(())
}

/// Resolve the remote cache backend from config.
pub fn resolve_backend(config: &RxConfig) -> Option<CacheBackend> {
    let url = &config.build.remote_cache;
    if url.is_empty() {
        return None;
    }
    CacheBackend::from_url(url).ok()
}
