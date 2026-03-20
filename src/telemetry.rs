//! Opt-in anonymous telemetry for understanding rx usage patterns.
//!
//! Telemetry is OFF by default. Users must explicitly opt in via `rx telemetry on`.
//! Data is stored locally at ~/.rx/telemetry.json and never sent without consent.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct TelemetryData {
    /// Whether telemetry collection is enabled
    pub enabled: bool,
    /// Command usage counts
    pub commands: HashMap<String, u64>,
    /// Feature usage flags
    pub features_used: Vec<String>,
    /// Total rx invocations
    pub total_invocations: u64,
    /// First seen timestamp
    pub first_seen: Option<String>,
    /// OS/arch
    pub platform: Option<String>,
}

fn telemetry_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(".rx").join("telemetry.json"))
}

fn load_telemetry() -> TelemetryData {
    let path = match telemetry_path() {
        Ok(p) => p,
        Err(_) => return TelemetryData::default(),
    };
    if !path.exists() {
        return TelemetryData::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_telemetry(data: &TelemetryData) {
    if let Ok(path) = telemetry_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(data) {
            fs::write(&path, json).ok();
        }
    }
}

/// Record a command invocation (only if telemetry is enabled).
pub fn record_command(command: &str) {
    let mut data = load_telemetry();
    if !data.enabled {
        return;
    }

    *data.commands.entry(command.to_string()).or_insert(0) += 1;
    data.total_invocations += 1;

    if data.first_seen.is_none() {
        data.first_seen = Some(
            time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        );
    }

    if data.platform.is_none() {
        data.platform = Some(format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }

    save_telemetry(&data);
}

/// Record a feature being used.
pub fn record_feature(feature: &str) {
    let mut data = load_telemetry();
    if !data.enabled {
        return;
    }

    if !data.features_used.contains(&feature.to_string()) {
        data.features_used.push(feature.to_string());
        save_telemetry(&data);
    }
}

/// Enable telemetry.
pub fn enable() -> Result<()> {
    let mut data = load_telemetry();
    data.enabled = true;
    data.platform = Some(format!(
        "{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    data.first_seen = Some(
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    );
    save_telemetry(&data);
    crate::output::success("telemetry enabled — anonymous usage data will be collected locally");
    crate::output::step(
        "note",
        "data is stored at ~/.rx/telemetry.json and never sent automatically",
    );
    Ok(())
}

/// Disable telemetry.
pub fn disable() -> Result<()> {
    let mut data = load_telemetry();
    data.enabled = false;
    save_telemetry(&data);
    crate::output::success("telemetry disabled");
    Ok(())
}

/// Show telemetry status and collected data.
pub fn status() -> Result<()> {
    let data = load_telemetry();

    use owo_colors::OwoColorize;

    println!("{}", "Telemetry Status".bold());
    println!("{}", "─".repeat(60));

    if data.enabled {
        println!("  {} telemetry is {}", "●".green(), "enabled".green());
    } else {
        println!("  {} telemetry is {}", "○".dimmed(), "disabled".dimmed());
        println!("  {} run `rx telemetry on` to opt in", "→".dimmed());
        return Ok(());
    }

    if let Some(ref platform) = data.platform {
        println!("  platform: {}", platform.dimmed());
    }
    if let Some(ref first) = data.first_seen {
        let display: &str = if first.len() >= 10 {
            &first[..10]
        } else {
            first
        };
        println!("  tracking since: {}", display.dimmed());
    }
    println!("  total invocations: {}", data.total_invocations);

    if !data.commands.is_empty() {
        println!("\n  {}", "Command usage:".bold());
        let mut sorted: Vec<_> = data.commands.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (cmd, count) in sorted.iter().take(15) {
            println!("    {:<20} {}", cmd.cyan(), count);
        }
    }

    if !data.features_used.is_empty() {
        println!("\n  {}", "Features used:".bold());
        for feature in &data.features_used {
            println!("    {}", feature.dimmed());
        }
    }

    println!(
        "\n  {} data stored at {}",
        "📁".dimmed(),
        telemetry_path()?.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_telemetry_data_has_enabled_false() {
        let data = TelemetryData::default();
        assert!(!data.enabled);
        assert!(data.commands.is_empty());
        assert!(data.features_used.is_empty());
        assert_eq!(data.total_invocations, 0);
        assert!(data.first_seen.is_none());
        assert!(data.platform.is_none());
    }

    #[test]
    fn telemetry_data_roundtrip_serialize() {
        let mut data = TelemetryData::default();
        data.enabled = true;
        data.commands.insert("build".to_string(), 5);
        data.features_used.push("remote-cache".to_string());
        data.total_invocations = 42;
        data.first_seen = Some("2025-01-01T00:00:00Z".to_string());
        data.platform = Some("macos-aarch64".to_string());

        let json = serde_json::to_string(&data).expect("serialize failed");
        let restored: TelemetryData = serde_json::from_str(&json).expect("deserialize failed");

        assert!(restored.enabled);
        assert_eq!(restored.commands.get("build"), Some(&5));
        assert_eq!(restored.features_used, vec!["remote-cache".to_string()]);
        assert_eq!(restored.total_invocations, 42);
        assert_eq!(restored.first_seen.as_deref(), Some("2025-01-01T00:00:00Z"));
        assert_eq!(restored.platform.as_deref(), Some("macos-aarch64"));
    }
}
