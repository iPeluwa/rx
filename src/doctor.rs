use anyhow::Result;
use owo_colors::OwoColorize;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

struct Check {
    name: &'static str,
    found: bool,
    version: String,
    hint: &'static str,
    required: bool,
}

fn check_tool(name: &str, args: &[&str]) -> (bool, String) {
    let result = Command::new(name).args(args).output();
    match result {
        Ok(output) if output.status.success() => {
            let out = String::from_utf8_lossy(&output.stdout);
            let ver = out.lines().next().unwrap_or("").trim().to_string();
            (true, ver)
        }
        _ => (false, String::new()),
    }
}

fn check_cargo_plugin(plugin: &str) -> (bool, String) {
    let result = Command::new("cargo").args([plugin, "--version"]).output();
    match result {
        Ok(output) if output.status.success() => {
            let out = String::from_utf8_lossy(&output.stdout);
            let ver = out.lines().next().unwrap_or("").trim().to_string();
            (true, ver)
        }
        _ => (false, String::new()),
    }
}

/// Calculate directory size in bytes.
fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

/// Extract rust-version (MSRV) from Cargo.toml.
fn extract_msrv(cargo_toml: &Path) -> Option<String> {
    let contents = fs::read_to_string(cargo_toml).ok()?;
    let table: toml::Table = toml::from_str(&contents).ok()?;
    table
        .get("package")?
        .get("rust-version")?
        .as_str()
        .map(|s| s.to_string())
}

/// Count workspace members from Cargo.toml.
fn count_workspace_members(cargo_toml: &Path) -> Option<usize> {
    let contents = fs::read_to_string(cargo_toml).ok()?;
    let table: toml::Table = toml::from_str(&contents).ok()?;
    let workspace = table.get("workspace")?;
    let members = workspace.get("members")?.as_array()?;
    Some(members.len())
}

pub fn doctor() -> Result<()> {
    // Welcome message for first-time users
    let rx_dir = dirs::home_dir().map(|h| h.join(".rx"));
    if let Some(ref dir) = rx_dir {
        if !dir.exists() {
            println!(
                "\n  {}",
                "Welcome to rx! Scanning your environment...".cyan().bold()
            );
            println!();
        }
    }

    println!("{}", "rx doctor".bold());
    println!("{}", "━".repeat(60).dimmed());

    // ── Environment ──
    println!("\n  {}", "Environment".bold().underline());

    let checks = vec![
        {
            let (found, version) = check_tool("rustc", &["--version"]);
            Check {
                name: "rustc",
                found,
                version,
                hint: "install via https://rustup.rs",
                required: true,
            }
        },
        {
            let (found, version) = check_tool("cargo", &["--version"]);
            Check {
                name: "cargo",
                found,
                version,
                hint: "install via https://rustup.rs",
                required: true,
            }
        },
        {
            let (found, version) = check_tool("rustup", &["--version"]);
            Check {
                name: "rustup",
                found,
                version,
                hint: "install via https://rustup.rs",
                required: false,
            }
        },
        {
            let (found, version) = check_cargo_plugin("fmt");
            Check {
                name: "rustfmt",
                found,
                version,
                hint: "rustup component add rustfmt",
                required: false,
            }
        },
        {
            let (found, version) = check_cargo_plugin("clippy");
            Check {
                name: "clippy",
                found,
                version,
                hint: "rustup component add clippy",
                required: false,
            }
        },
        {
            let (found, version) = check_tool("mold", &["--version"]);
            Check {
                name: "mold",
                found,
                version,
                hint: "fast linker — https://github.com/rui314/mold",
                required: false,
            }
        },
        {
            let (found, version) = check_tool("lld", &["--version"]);
            Check {
                name: "lld",
                found,
                version,
                hint: "fast linker — install via your package manager",
                required: false,
            }
        },
        {
            let (found, version) = check_cargo_plugin("nextest");
            Check {
                name: "nextest",
                found,
                version,
                hint: "cargo install cargo-nextest",
                required: false,
            }
        },
    ];

    let has_fast_linker = checks
        .iter()
        .any(|c| (c.name == "mold" || c.name == "lld") && c.found);
    let mut suggestions: Vec<String> = Vec::new();

    for check in &checks {
        let icon = if check.found {
            "✓".green().to_string()
        } else if check.required {
            "✗".red().to_string()
        } else {
            "⚠".yellow().to_string()
        };

        if check.found {
            println!("  {icon}  {:<12} {}", check.name, check.version.dimmed());
        } else {
            println!("  {icon}  {:<12} {}", check.name, check.hint.dimmed());
        }
    }

    if !has_fast_linker {
        suggestions.push("No fast linker found — install mold for 3-5x faster linking".to_string());
    }

    // ── Project Analysis ──
    let cargo_toml = Path::new("Cargo.toml");
    if cargo_toml.exists() {
        println!("\n  {}", "Project".bold().underline());

        // Workspace detection
        if let Some(member_count) = count_workspace_members(cargo_toml) {
            println!("  {}  Workspace with {} members", "✓".green(), member_count);
        } else {
            println!("  {}  Single crate", "✓".green());
        }

        // MSRV
        if let Some(msrv) = extract_msrv(cargo_toml) {
            println!("  {}  MSRV: {}", "✓".green(), msrv);
        }

        // target/ size
        let target_dir = Path::new("target");
        if target_dir.exists() {
            let size = dir_size(target_dir);
            let size_str = format_size(size);
            if size > 2_147_483_648 {
                // > 2 GB
                println!("  {}  target/ is {}", "⚠".yellow(), size_str);
                suggestions.push(format!(
                    "target/ is {} — run `rx clean` or `rx cache gc` to reclaim space",
                    size_str
                ));
            } else {
                println!("  {}  target/ is {}", "✓".green(), size_str);
            }
        }

        // Cargo.lock
        if Path::new("Cargo.lock").exists() {
            // Check if committed
            let git_status = Command::new("git")
                .args(["status", "--porcelain", "Cargo.lock"])
                .output();
            if let Ok(output) = git_status {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("??") {
                    println!(
                        "  {}  Cargo.lock exists but is not tracked by git",
                        "⚠".yellow()
                    );
                    suggestions
                        .push("Cargo.lock is not tracked — run `git add Cargo.lock`".to_string());
                } else {
                    println!("  {}  Cargo.lock committed", "✓".green());
                }
            } else {
                println!("  {}  Cargo.lock exists", "✓".green());
            }
        } else {
            println!("  {}  Cargo.lock missing", "⚠".yellow());
            suggestions.push(
                "No Cargo.lock — run `cargo generate-lockfile` for reproducible builds".to_string(),
            );
        }

        // rx.toml
        if Path::new("rx.toml").exists() {
            println!("  {}  rx.toml configured", "✓".green());
        } else {
            println!(
                "  {}  No rx.toml — run `rx init` to configure",
                "⚠".yellow()
            );
            suggestions
                .push("No rx.toml found — run `rx init` to configure your project".to_string());
        }

        // Quick security check (non-blocking)
        let (has_audit, _) = check_cargo_plugin("audit");
        if has_audit {
            let audit_out = Command::new("cargo").args(["audit", "--quiet"]).output();
            if let Ok(output) = audit_out {
                if output.status.success() {
                    println!("  {}  No known vulnerabilities", "✓".green());
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // Skip CVSS parse errors
                    if stderr.contains("unsupported CVSS version") {
                        println!(
                            "  {}  cargo-audit needs upgrade (CVSS v4.0 parse issue)",
                            "⚠".yellow()
                        );
                    } else {
                        let vuln_count = String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .filter(|l| l.contains("RUSTSEC"))
                            .count();
                        if vuln_count > 0 {
                            println!(
                                "  {}  {} security advisory(ies) — run `rx audit`",
                                "⚠".yellow(),
                                vuln_count
                            );
                            suggestions.push(format!(
                                "{vuln_count} security advisory(ies) found — run `rx audit` for details"
                            ));
                        } else {
                            println!("  {}  Security check completed with warnings", "⚠".yellow());
                        }
                    }
                }
            }
        }
    }

    // ── Cache ──
    println!("\n  {}", "Cache".bold().underline());
    let cache_dir = crate::cache::cache_dir()?;
    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        println!(
            "  {}  {} ({})",
            "✓".green(),
            cache_dir.display(),
            format_size(size)
        );
    } else {
        println!(
            "  {}  Not initialized (will be created on first build)",
            "⚠".yellow()
        );
    }

    // Invalidate env detection cache
    crate::build::invalidate_env_cache();

    // ── Suggestions ──
    if !suggestions.is_empty() {
        println!("\n  {}", "Suggestions".bold().underline());
        for (i, suggestion) in suggestions.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().yellow(), suggestion);
        }
    }

    println!("\n{}", "━".repeat(60).dimmed());

    let missing_required = checks.iter().filter(|c| c.required && !c.found).count();
    if missing_required > 0 {
        println!(
            "{}",
            "Required tools missing! Install them before using rx.".red()
        );
    } else if suggestions.is_empty() {
        println!("{}", "All checks passed. Happy building!".green());
    } else {
        println!(
            "{}",
            "Environment is ready. See suggestions above for improvements.".green()
        );
    }

    Ok(())
}
