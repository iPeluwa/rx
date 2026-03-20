use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "iPeluwa/rx";

pub fn self_update() -> Result<()> {
    self_update_with_options(false)
}

pub fn self_update_with_options(skip_verify: bool) -> Result<()> {
    crate::output::info("checking for rx updates...");

    // Try to get current version
    let current = env!("CARGO_PKG_VERSION");
    crate::output::step("current", current);

    // Check if we can use the install script
    let has_curl = Command::new("curl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_curl {
        // Detect target platform
        let target = detect_target()?;

        // Download and verify the binary
        let temp_dir = std::env::temp_dir();
        let archive_path = temp_dir.join(format!("rx-{}.tar.gz", target));
        let binary_path = temp_dir.join("rx");

        // Download the archive
        let pb = crate::output::spinner("downloading latest release...");
        download_release(&target, &archive_path)?;
        pb.finish_and_clear();

        // Verify checksum of the archive before extracting
        if !skip_verify {
            let pb = crate::output::spinner("verifying checksum...");
            verify_checksum(&target, &binary_path)?;
            pb.finish_and_clear();
            crate::output::success("checksum verified");
        } else {
            crate::output::info("skipping checksum verification");
        }

        // Extract the binary
        let pb = crate::output::spinner("extracting binary...");
        extract_binary(&archive_path, &binary_path)?;
        pb.finish_and_clear();

        // Install the binary
        let pb = crate::output::spinner("installing binary...");
        install_binary(&binary_path)?;
        pb.finish_and_clear();
    } else {
        // Fall back to cargo install
        crate::output::info("updating via cargo install...");
        let status = Command::new("cargo")
            .args([
                "install",
                "--git",
                &format!("https://github.com/{REPO}.git"),
            ])
            .status()
            .context("failed to run cargo install")?;
        if !status.success() {
            anyhow::bail!("self-update via cargo install failed");
        }
    }

    crate::output::success("rx updated successfully");
    Ok(())
}

#[allow(dead_code)]
pub fn check_latest_version() -> Result<Option<String>> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github.v3+json",
            &url,
        ])
        .output()
        .context("failed to query GitHub API")?;

    if !output.status.success() {
        anyhow::bail!("failed to fetch latest release info");
    }

    let response = String::from_utf8(output.stdout).context("invalid UTF-8 in API response")?;

    // Parse the tag_name from JSON response
    // Simple parsing without a JSON library
    let tag_name = parse_tag_name(&response)?;

    let current = env!("CARGO_PKG_VERSION");
    if compare_versions(&tag_name, current)? {
        Ok(Some(tag_name))
    } else {
        Ok(None)
    }
}

fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => anyhow::bail!("unsupported platform: {}-{}", os, arch),
    };

    Ok(target.to_string())
}

fn download_release(target: &str, dest: &Path) -> Result<()> {
    let url = format!(
        "https://github.com/{}/releases/latest/download/rx-{}.tar.gz",
        REPO, target
    );

    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(dest)
        .arg(&url)
        .status()
        .context("failed to download release")?;

    if !status.success() {
        anyhow::bail!("failed to download release from {}", url);
    }

    Ok(())
}

fn extract_binary(archive_path: &Path, binary_path: &Path) -> Result<()> {
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(archive_path)
        .arg("-C")
        .arg(binary_path.parent().unwrap())
        .status()
        .context("failed to extract archive")?;

    if !status.success() {
        anyhow::bail!("failed to extract binary from archive");
    }

    Ok(())
}

fn install_binary(binary_path: &Path) -> Result<()> {
    // Determine install location
    let install_dir = if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local/bin")
    } else {
        PathBuf::from("/usr/local/bin")
    };

    std::fs::create_dir_all(&install_dir).context("failed to create install directory")?;

    let dest = install_dir.join("rx");

    // Copy and make executable
    std::fs::copy(binary_path, &dest).context("failed to copy binary to install location")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    Ok(())
}

fn verify_checksum(target: &str, _binary_path: &Path) -> Result<()> {
    // The .sha256 file is for the archive, not the extracted binary.
    // Verify the archive checksum before extraction.
    let temp_dir = std::env::temp_dir();
    let archive_path = temp_dir.join(format!("rx-{}.tar.gz", target));
    let checksum_path = temp_dir.join(format!("rx-{}.tar.gz.sha256", target));

    let url = format!(
        "https://github.com/{}/releases/latest/download/rx-{}.tar.gz.sha256",
        REPO, target
    );

    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&checksum_path)
        .arg(&url)
        .status()
        .context("failed to download checksum file")?;

    if !status.success() {
        anyhow::bail!("failed to download checksum file from {}", url);
    }

    let expected =
        std::fs::read_to_string(&checksum_path).context("failed to read checksum file")?;
    let expected = parse_checksum(&expected)?;

    // Compute actual checksum of the archive (not the extracted binary)
    let actual = compute_sha256(&archive_path)?;

    if actual != expected {
        anyhow::bail!(
            "checksum verification failed!\n\
             expected: {}\n\
             actual:   {}",
            expected,
            actual
        );
    }

    Ok(())
}

fn compute_sha256(file_path: &Path) -> Result<String> {
    // Try sha256sum (Linux) first, then shasum (macOS)
    let commands = [
        ("sha256sum", vec![file_path.to_str().unwrap()]),
        ("shasum", vec!["-a", "256", file_path.to_str().unwrap()]),
    ];

    for (cmd, args) in &commands {
        if let Ok(output) = Command::new(cmd).args(args).output() {
            if output.status.success() {
                let stdout =
                    String::from_utf8(output.stdout).context("invalid UTF-8 in hash output")?;
                return parse_hash_output(&stdout);
            }
        }
    }

    anyhow::bail!("neither sha256sum nor shasum command available")
}

fn parse_hash_output(output: &str) -> Result<String> {
    // Hash output format: "<hash> <filename>" or "<hash>  <filename>"
    let hash = output
        .split_whitespace()
        .next()
        .context("empty hash output")?;

    if hash.len() != 64 {
        anyhow::bail!("invalid hash length: expected 64, got {}", hash.len());
    }

    Ok(hash.to_lowercase())
}

fn parse_checksum(content: &str) -> Result<String> {
    // Checksum file format: "<hash> <filename>" or just "<hash>"
    let hash = content
        .split_whitespace()
        .next()
        .context("empty checksum file")?;

    if hash.len() != 64 {
        anyhow::bail!("invalid checksum length: expected 64, got {}", hash.len());
    }

    Ok(hash.to_lowercase())
}

fn parse_tag_name(json: &str) -> Result<String> {
    // Simple JSON parsing for "tag_name": "vX.Y.Z"
    for line in json.lines() {
        let line = line.trim();
        if line.starts_with("\"tag_name\"") {
            if let Some(start) = line.find(": \"") {
                let value_start = start + 3;
                if let Some(end) = line[value_start..].find('"') {
                    let tag = &line[value_start..value_start + end];
                    // Remove 'v' prefix if present
                    return Ok(tag.trim_start_matches('v').to_string());
                }
            }
        }
    }
    anyhow::bail!("could not find tag_name in API response")
}

fn compare_versions(latest: &str, current: &str) -> Result<bool> {
    let latest_parts = parse_version(latest)?;
    let current_parts = parse_version(current)?;

    Ok(latest_parts > current_parts)
}

fn parse_version(version: &str) -> Result<Vec<u32>> {
    version
        .trim_start_matches('v')
        .split('.')
        .map(|s| {
            s.parse::<u32>()
                .with_context(|| format!("invalid version component: {}", s))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hash_output() {
        let output = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  rx";
        let hash = parse_hash_output(output).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_parse_hash_output_single_space() {
        let output = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 rx";
        let hash = parse_hash_output(output).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_parse_checksum() {
        let content = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  rx-x86_64-apple-darwin.tar.gz\n";
        let hash = parse_checksum(content).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_parse_checksum_hash_only() {
        let content = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n";
        let hash = parse_checksum(content).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_version("v1.2.3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_version("0.1.0").unwrap(), vec![0, 1, 0]);
    }

    #[test]
    fn test_compare_versions() {
        // Latest is newer
        assert!(compare_versions("1.2.3", "1.2.2").unwrap());
        assert!(compare_versions("1.3.0", "1.2.9").unwrap());
        assert!(compare_versions("2.0.0", "1.9.9").unwrap());

        // Current is newer or equal
        assert!(!compare_versions("1.2.2", "1.2.3").unwrap());
        assert!(!compare_versions("1.2.3", "1.2.3").unwrap());
    }

    #[test]
    fn test_parse_tag_name() {
        let json = r#"{
  "tag_name": "v1.2.3",
  "name": "Release 1.2.3"
}"#;
        assert_eq!(parse_tag_name(json).unwrap(), "1.2.3");
    }
}
