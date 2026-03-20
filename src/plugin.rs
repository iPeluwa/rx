use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn plugins_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".rx").join("plugins"))
}

/// Find a plugin binary by name.
fn find_plugin(name: &str) -> Option<PathBuf> {
    let dir = plugins_dir()?;
    if !dir.exists() {
        return None;
    }

    // Look for rx-<name> binary
    let plugin_name = format!("rx-{name}");
    let path = dir.join(&plugin_name);
    if path.exists() {
        return Some(path);
    }

    // Also check PATH for rx-<name>
    if let Ok(output) = Command::new("which").arg(&plugin_name).output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Some(PathBuf::from(path_str));
        }
    }

    None
}

/// Run a plugin with the given arguments.
pub fn run_plugin(name: &str, args: &[String]) -> Result<()> {
    let plugin_path = find_plugin(name).ok_or_else(|| {
        anyhow::anyhow!(
            "no plugin `{name}` found\n\
             hint: install plugins to ~/.rx/plugins/ as `rx-{name}` executables, \
             or put `rx-{name}` on your PATH"
        )
    })?;

    crate::output::verbose(&format!("running plugin: {}", plugin_path.display()));

    let status = Command::new(&plugin_path)
        .args(args)
        .status()
        .with_context(|| format!("failed to run plugin {}", plugin_path.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// List all available plugins.
pub fn list_plugins() -> Result<()> {
    let mut found = Vec::new();

    // Check ~/.rx/plugins/
    if let Some(dir) = plugins_dir() {
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(short) = name.strip_prefix("rx-") {
                        found.push((short.to_string(), entry.path().display().to_string()));
                    }
                }
            }
        }
    }

    // Check PATH for rx-* binaries
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(stripped) = name.strip_prefix("rx-") {
                        let short = stripped.to_string();
                        if !found.iter().any(|(n, _)| n == &short) {
                            found.push((short, entry.path().display().to_string()));
                        }
                    }
                }
            }
        }
    }

    if found.is_empty() {
        crate::output::info("no plugins found");
        crate::output::step(
            "hint",
            "install plugins to ~/.rx/plugins/ as `rx-<name>` executables",
        );
        return Ok(());
    }

    found.sort();
    crate::output::info(&format!("{} plugin(s) found:", found.len()));
    for (name, path) in &found {
        println!("  {name:<20} {path}");
    }
    Ok(())
}
