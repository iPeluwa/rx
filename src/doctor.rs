use anyhow::Result;
use owo_colors::OwoColorize;
use std::process::Command;

struct Check {
    name: &'static str,
    found: bool,
    version: String,
    hint: &'static str,
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

pub fn doctor() -> Result<()> {
    println!("{}", "rx doctor".bold());
    println!("{}", "─".repeat(50).dimmed());

    let checks = vec![
        {
            let (found, version) = check_tool("rustc", &["--version"]);
            Check {
                name: "rustc",
                found,
                version,
                hint: "install via https://rustup.rs",
            }
        },
        {
            let (found, version) = check_tool("cargo", &["--version"]);
            Check {
                name: "cargo",
                found,
                version,
                hint: "install via https://rustup.rs",
            }
        },
        {
            let (found, version) = check_tool("rustup", &["--version"]);
            Check {
                name: "rustup",
                found,
                version,
                hint: "install via https://rustup.rs",
            }
        },
        {
            let (found, version) = check_cargo_plugin("fmt");
            Check {
                name: "rustfmt",
                found,
                version,
                hint: "rustup component add rustfmt",
            }
        },
        {
            let (found, version) = check_cargo_plugin("clippy");
            Check {
                name: "clippy",
                found,
                version,
                hint: "rustup component add clippy",
            }
        },
        {
            let (found, version) = check_tool("mold", &["--version"]);
            Check {
                name: "mold",
                found,
                version,
                hint: "https://github.com/rui314/mold (optional, speeds up linking)",
            }
        },
        {
            let (found, version) = check_tool("lld", &["--version"]);
            Check {
                name: "lld",
                found,
                version,
                hint: "install via your package manager (optional, speeds up linking)",
            }
        },
        {
            let (found, version) = check_cargo_plugin("nextest");
            Check {
                name: "nextest",
                found,
                version,
                hint: "cargo install cargo-nextest (optional, faster test runner)",
            }
        },
    ];

    let mut missing_required = 0;
    for check in &checks {
        let status = if check.found {
            "OK".green().bold().to_string()
        } else {
            "MISSING".red().bold().to_string()
        };

        let version_str = if check.found {
            format!(" ({})", check.version.dimmed())
        } else {
            format!(" -> {}", check.hint.yellow())
        };

        println!("  {status:<18} {:<14}{version_str}", check.name);

        if !check.found && ["rustc", "cargo"].contains(&check.name) {
            missing_required += 1;
        }
    }

    println!("{}", "─".repeat(50).dimmed());

    let cache_dir = crate::cache::cache_dir()?;
    if cache_dir.exists() {
        println!(
            "  {} {}",
            "Cache:".dimmed(),
            cache_dir.display().to_string().dimmed()
        );
    } else {
        println!("  {} (not initialized)", "Cache:".dimmed());
    }

    // Invalidate env detection cache so next build re-detects tools
    crate::build::invalidate_env_cache();
    println!("  {} refreshed", "Env cache:".dimmed());

    if missing_required > 0 {
        println!(
            "\n{}",
            "Required tools missing! Install them before using rx.".red()
        );
    } else {
        println!("\n{}", "All required tools present.".green());
    }

    Ok(())
}
