use anyhow::Result;

use crate::output::Timer;

pub fn deps() -> Result<()> {
    let timer = Timer::start("deps");

    // 1. Dependency tree (depth-limited)
    crate::output::info("dependency tree (depth 1):");
    let _ = std::process::Command::new("cargo")
        .args(["tree", "--depth", "1"])
        .status();

    println!();

    // 2. Duplicates
    let dup_output = std::process::Command::new("cargo")
        .args(["tree", "--duplicates"])
        .output();

    if let Ok(output) = dup_output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            crate::output::success("no duplicate dependencies");
        } else {
            crate::output::warn("duplicate dependencies found:");
            print!("{stdout}");
        }
    }

    println!();

    // 3. Outdated check
    let has_outdated = std::process::Command::new("cargo")
        .args(["outdated", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_outdated {
        crate::output::info("outdated dependencies:");
        let _ = std::process::Command::new("cargo")
            .args(["outdated", "--root-deps-only"])
            .status();
    } else {
        crate::output::info("checking for updates (cargo update --dry-run):");
        let _ = std::process::Command::new("cargo")
            .args(["update", "--dry-run"])
            .status();
    }

    println!();

    // 4. Security audit
    let has_audit = std::process::Command::new("cargo")
        .args(["audit", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_audit {
        crate::output::info("security audit:");
        let status = std::process::Command::new("cargo").arg("audit").status();

        if let Ok(s) = status {
            if s.success() {
                crate::output::success("no known vulnerabilities");
            }
        }
    } else {
        crate::output::verbose(
            "install cargo-audit for security scanning: cargo install cargo-audit",
        );
    }

    timer.finish();
    Ok(())
}
