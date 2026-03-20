use anyhow::Result;

use crate::config::RxConfig;

pub fn show_env(config: &RxConfig) -> Result<()> {
    if config.env.is_empty() {
        crate::output::info("no environment variables defined in rx.toml");
        return Ok(());
    }

    let mut keys: Vec<&String> = config.env.keys().collect();
    keys.sort();

    for key in keys {
        let value = &config.env[key];
        println!("{key}={value}");
    }
    Ok(())
}

pub fn shell(config: &RxConfig) -> Result<()> {
    // Set all env vars from config
    for (key, value) in &config.env {
        // SAFETY: we're about to exec a shell, no other threads running
        unsafe { std::env::set_var(key, value) };
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    crate::output::info(&format!(
        "spawning {shell} with {} env var(s) from rx.toml",
        config.env.len()
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
