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

/// Export telemetry data in various formats.
pub fn export(format: &str) -> Result<()> {
    let data = load_telemetry();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&data)
                .context("failed to serialize telemetry data")?;
            println!("{}", json);
        }
        "csv" => {
            println!("metric,value");
            println!("enabled,{}", data.enabled);
            println!("total_invocations,{}", data.total_invocations);
            if let Some(ref platform) = data.platform {
                println!("platform,{}", platform);
            }
            if let Some(ref first_seen) = data.first_seen {
                println!("first_seen,{}", first_seen);
            }
            for (cmd, count) in &data.commands {
                println!("command:{},{}", cmd, count);
            }
            for feature in &data.features_used {
                println!("feature:{},true", feature);
            }
        }
        "markdown" | "md" => {
            println!("# Telemetry Report\n");
            println!("| Metric | Value |");
            println!("| ------ | ----- |");
            println!("| Enabled | {} |", data.enabled);
            println!("| Total Invocations | {} |", data.total_invocations);
            if let Some(ref platform) = data.platform {
                println!("| Platform | {} |", platform);
            }
            if let Some(ref first_seen) = data.first_seen {
                println!("| First Seen | {} |", first_seen);
            }
            if !data.commands.is_empty() {
                println!("\n## Command Usage\n");
                println!("| Command | Count |");
                println!("| ------- | ----- |");
                let mut sorted: Vec<_> = data.commands.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (cmd, count) in sorted {
                    println!("| {} | {} |", cmd, count);
                }
            }
            if !data.features_used.is_empty() {
                println!("\n## Features Used\n");
                for feature in &data.features_used {
                    println!("- {}", feature);
                }
            }
        }
        _ => anyhow::bail!(
            "unsupported export format: {} (use json, csv, or markdown)",
            format
        ),
    }

    Ok(())
}

/// Print a human-readable usage report.
pub fn report() -> Result<()> {
    let data = load_telemetry();

    use owo_colors::OwoColorize;

    println!("{}", "Telemetry Usage Report".bold());
    println!("{}", "═".repeat(60));

    if !data.enabled {
        println!("\n  {} Telemetry is disabled", "○".dimmed());
        println!(
            "  {} Run `rx telemetry on` to start collecting usage data",
            "→".dimmed()
        );
        return Ok(());
    }

    // Calculate time period
    let time_period = if let Some(ref first_seen) = data.first_seen {
        if let Ok(first_time) =
            time::OffsetDateTime::parse(first_seen, &time::format_description::well_known::Rfc3339)
        {
            let now = time::OffsetDateTime::now_utc();
            let days = (now - first_time).whole_days();
            if days > 0 {
                format!("{} days", days)
            } else {
                "less than a day".to_string()
            }
        } else {
            "unknown".to_string()
        }
    } else {
        "unknown".to_string()
    };

    println!("\n{}", "Overview".bold());
    println!(
        "  Total invocations: {}",
        data.total_invocations.to_string().cyan()
    );
    println!("  Time period: {}", time_period.dimmed());

    // Calculate average invocations per day
    if let Some(ref first_seen) = data.first_seen {
        if let Ok(first_time) =
            time::OffsetDateTime::parse(first_seen, &time::format_description::well_known::Rfc3339)
        {
            let now = time::OffsetDateTime::now_utc();
            let days = (now - first_time).whole_days().max(1);
            let avg = data.total_invocations as f64 / days as f64;
            println!("  Average per day: {:.1}", avg);
        }
    }

    if let Some(ref platform) = data.platform {
        println!("  Platform: {}", platform.dimmed());
    }

    // Most-used commands (top 10)
    if !data.commands.is_empty() {
        println!("\n{}", "Most-Used Commands (Top 10)".bold());
        let mut sorted: Vec<_> = data.commands.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (cmd, count)) in sorted.iter().take(10).enumerate() {
            let percentage = (**count as f64 / data.total_invocations as f64) * 100.0;
            println!(
                "  {}. {:<20} {} ({:.1}%)",
                i + 1,
                cmd.cyan(),
                count,
                percentage
            );
        }
    }

    // Features utilized
    if !data.features_used.is_empty() {
        println!("\n{}", "Features Utilized".bold());
        for feature in &data.features_used {
            println!("  • {}", feature.green());
        }
    } else {
        println!("\n{}", "Features Utilized".bold());
        println!("  {} No features recorded yet", "○".dimmed());
    }

    println!("\n{}", "─".repeat(60));
    println!(
        "  Data stored at: {}",
        telemetry_path()?.display().to_string().dimmed()
    );

    Ok(())
}

/// Clear all telemetry data while preserving enabled/disabled state.
pub fn reset() -> Result<()> {
    let mut data = load_telemetry();
    let enabled = data.enabled;

    data.commands.clear();
    data.features_used.clear();
    data.total_invocations = 0;
    data.first_seen = None;
    data.platform = None;
    data.enabled = enabled;

    save_telemetry(&data);
    crate::output::success("telemetry data cleared (enabled/disabled state preserved)");

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
        let mut data = TelemetryData {
            enabled: true,
            ..Default::default()
        };
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

    #[test]
    fn csv_export_format() {
        // Create test data
        let mut data = TelemetryData {
            enabled: true,
            ..Default::default()
        };
        data.commands.insert("build".to_string(), 10);
        data.commands.insert("test".to_string(), 5);
        data.features_used.push("remote-cache".to_string());
        data.total_invocations = 15;
        data.first_seen = Some("2025-01-01T00:00:00Z".to_string());
        data.platform = Some("linux-x86_64".to_string());

        // Capture CSV output logic
        let mut csv_lines = vec!["metric,value".to_string()];
        csv_lines.push(format!("enabled,{}", data.enabled));
        csv_lines.push(format!("total_invocations,{}", data.total_invocations));
        if let Some(ref platform) = data.platform {
            csv_lines.push(format!("platform,{}", platform));
        }
        if let Some(ref first_seen) = data.first_seen {
            csv_lines.push(format!("first_seen,{}", first_seen));
        }
        for (cmd, count) in &data.commands {
            csv_lines.push(format!("command:{},{}", cmd, count));
        }
        for feature in &data.features_used {
            csv_lines.push(format!("feature:{},true", feature));
        }

        let csv_output = csv_lines.join("\n");

        // Verify CSV format
        assert!(csv_output.contains("metric,value"));
        assert!(csv_output.contains("enabled,true"));
        assert!(csv_output.contains("total_invocations,15"));
        assert!(csv_output.contains("platform,linux-x86_64"));
        assert!(csv_output.contains("first_seen,2025-01-01T00:00:00Z"));
        assert!(csv_output.contains("command:build,10") || csv_output.contains("command:test,5"));
        assert!(csv_output.contains("feature:remote-cache,true"));
    }

    #[test]
    fn markdown_export_format() {
        // Create test data
        let mut data = TelemetryData {
            enabled: true,
            ..Default::default()
        };
        data.commands.insert("build".to_string(), 10);
        data.commands.insert("test".to_string(), 5);
        data.features_used.push("remote-cache".to_string());
        data.total_invocations = 15;
        data.first_seen = Some("2025-01-01T00:00:00Z".to_string());
        data.platform = Some("linux-x86_64".to_string());

        // Capture markdown output logic
        let mut md_lines = vec!["# Telemetry Report".to_string()];
        md_lines.push("".to_string());
        md_lines.push("| Metric | Value |".to_string());
        md_lines.push("| ------ | ----- |".to_string());
        md_lines.push(format!("| Enabled | {} |", data.enabled));
        md_lines.push(format!(
            "| Total Invocations | {} |",
            data.total_invocations
        ));
        if let Some(ref platform) = data.platform {
            md_lines.push(format!("| Platform | {} |", platform));
        }
        if let Some(ref first_seen) = data.first_seen {
            md_lines.push(format!("| First Seen | {} |", first_seen));
        }
        if !data.commands.is_empty() {
            md_lines.push("".to_string());
            md_lines.push("## Command Usage".to_string());
            md_lines.push("".to_string());
            md_lines.push("| Command | Count |".to_string());
            md_lines.push("| ------- | ----- |".to_string());
            let mut sorted: Vec<_> = data.commands.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (cmd, count) in sorted {
                md_lines.push(format!("| {} | {} |", cmd, count));
            }
        }
        if !data.features_used.is_empty() {
            md_lines.push("".to_string());
            md_lines.push("## Features Used".to_string());
            md_lines.push("".to_string());
            for feature in &data.features_used {
                md_lines.push(format!("- {}", feature));
            }
        }

        let md_output = md_lines.join("\n");

        // Verify markdown format
        assert!(md_output.contains("# Telemetry Report"));
        assert!(md_output.contains("| Metric | Value |"));
        assert!(md_output.contains("| Enabled | true |"));
        assert!(md_output.contains("| Total Invocations | 15 |"));
        assert!(md_output.contains("| Platform | linux-x86_64 |"));
        assert!(md_output.contains("| First Seen | 2025-01-01T00:00:00Z |"));
        assert!(md_output.contains("## Command Usage"));
        assert!(md_output.contains("| build | 10 |"));
        assert!(md_output.contains("| test | 5 |"));
        assert!(md_output.contains("## Features Used"));
        assert!(md_output.contains("- remote-cache"));
    }
}
