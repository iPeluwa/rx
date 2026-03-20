use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::RxConfig;

/// Parse a .env file into key-value pairs.
fn parse_dotenv(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().to_string();
            let mut value = value.trim().to_string();
            // Strip surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            map.insert(key, value);
        }
    }
    map
}

/// Check if a key name looks sensitive.
fn is_sensitive(key: &str) -> bool {
    let upper = key.to_uppercase();
    upper.contains("SECRET")
        || upper.contains("KEY")
        || upper.contains("TOKEN")
        || upper.contains("PASSWORD")
        || upper.contains("CREDENTIAL")
}

/// Mask a value if the key is sensitive.
fn display_value(key: &str, value: &str) -> String {
    if is_sensitive(key) {
        "***".to_string()
    } else {
        value.to_string()
    }
}

pub fn show_env(config: &RxConfig) -> Result<()> {
    use owo_colors::OwoColorize;

    let dotenv = parse_dotenv(Path::new(".env"));
    let has_rx_env = !config.env.is_empty();
    let has_dotenv = !dotenv.is_empty();

    if !has_rx_env && !has_dotenv {
        crate::output::info("no environment variables defined in rx.toml or .env");
        return Ok(());
    }

    println!("{}", "Resolved Environment".bold());
    println!("{}", "━".repeat(60).dimmed());

    if has_dotenv {
        println!("\n  {} {}", "From .env:".bold(), "(not committed)".dimmed());
        let mut keys: Vec<&String> = dotenv.keys().collect();
        keys.sort();
        for key in keys {
            let value = &dotenv[key];
            println!("    {:<30} = {}", key, display_value(key, value).dimmed());
        }
    }

    if has_rx_env {
        println!("\n  {}", "From rx.toml [env]:".bold());
        let mut keys: Vec<&String> = config.env.keys().collect();
        keys.sort();
        for key in keys {
            let value = &config.env[key];
            println!("    {:<30} = {}", key, display_value(key, value).dimmed());
        }
    }

    println!("\n{}", "━".repeat(60).dimmed());
    Ok(())
}

pub fn shell(config: &RxConfig) -> Result<()> {
    // Load .env first, then rx.toml (rx.toml overrides)
    let dotenv = parse_dotenv(Path::new(".env"));
    let total = dotenv.len() + config.env.len();

    for (key, value) in &dotenv {
        // SAFETY: we're about to exec a shell, no other threads running
        unsafe { std::env::set_var(key, value) };
    }
    for (key, value) in &config.env {
        unsafe { std::env::set_var(key, value) };
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    crate::output::info(&format!(
        "spawning {shell} with {total} env var(s) from .env + rx.toml"
    ));
    crate::output::step("env", "type `exit` to return");

    let status = std::process::Command::new(&shell)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to spawn {shell}: {e}"))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

/// Check .env against .env.example for missing/undocumented variables.
pub fn check_env(_config: &RxConfig) -> Result<()> {
    use owo_colors::OwoColorize;

    let dotenv = parse_dotenv(Path::new(".env"));
    let example = parse_dotenv(Path::new(".env.example"));

    if !Path::new(".env.example").exists() && !Path::new(".env").exists() {
        crate::output::info("no .env or .env.example found — nothing to check");
        return Ok(());
    }

    println!("{}", "Environment Check".bold());
    println!("{}", "━".repeat(60).dimmed());

    let mut issues = 0;

    if Path::new(".env.example").exists() {
        // Check for vars in .env.example missing from .env
        let mut missing: Vec<&String> = example
            .keys()
            .filter(|k| !dotenv.contains_key(*k))
            .collect();
        missing.sort();

        let mut present: Vec<&String> =
            example.keys().filter(|k| dotenv.contains_key(*k)).collect();
        present.sort();

        if !present.is_empty() {
            println!("\n  {}", "Present in .env:".bold());
            for key in &present {
                println!("  {}  {}", "✓".green(), key);
            }
        }

        if !missing.is_empty() {
            println!(
                "\n  {}",
                "Missing from .env (defined in .env.example):".bold()
            );
            for key in &missing {
                println!("  {}  {}", "✗".red(), key);
            }
            issues += missing.len();
        }
    }

    // Check for undocumented vars (in .env but not in .env.example)
    if Path::new(".env.example").exists() {
        let mut undocumented: Vec<&String> = dotenv
            .keys()
            .filter(|k| !example.contains_key(*k))
            .collect();
        undocumented.sort();

        if !undocumented.is_empty() {
            println!(
                "\n  {}",
                "Undocumented (in .env but not .env.example):".bold()
            );
            for key in &undocumented {
                println!("  {}  {}", "⚠".yellow(), key);
            }
        }
    }

    println!("\n{}", "━".repeat(60).dimmed());

    if issues > 0 {
        crate::output::warn(&format!("{issues} required variable(s) missing from .env"));
    } else if Path::new(".env.example").exists() {
        crate::output::success("all variables from .env.example are present in .env");
    }

    Ok(())
}

/// Add a variable name to .env.example.
pub fn add_example(name: &str, description: Option<&str>) -> Result<()> {
    let path = Path::new(".env.example");

    // Check if already present
    if path.exists() {
        let existing = parse_dotenv(path);
        if existing.contains_key(name) {
            crate::output::info(&format!("{name} already exists in .env.example"));
            return Ok(());
        }
    }

    let mut content = String::new();
    if let Some(desc) = description {
        content.push_str(&format!("# {desc}\n"));
    }
    content.push_str(&format!("{name}=\n"));

    // Append to file
    if path.exists() {
        let existing = fs::read_to_string(path)?;
        if !existing.ends_with('\n') && !existing.is_empty() {
            content.insert(0, '\n');
        }
        fs::write(path, format!("{existing}{content}"))?;
    } else {
        fs::write(path, content)?;
    }

    crate::output::success(&format!("added {name} to .env.example"));
    Ok(())
}
