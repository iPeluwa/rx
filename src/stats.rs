use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Serialize, Deserialize, Default)]
struct StatsStore {
    builds: VecDeque<BuildRecord>,
}

#[derive(Serialize, Deserialize)]
struct BuildRecord {
    timestamp: String,
    command: String,
    duration_secs: f64,
    success: bool,
}

fn stats_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".rx").join("stats.json"))
}

fn load_stats() -> Result<StatsStore> {
    let path = stats_path()?;
    if !path.exists() {
        return Ok(StatsStore::default());
    }
    let contents = fs::read_to_string(&path).context("failed to read stats")?;
    serde_json::from_str(&contents).context("failed to parse stats")
}

fn save_stats(store: &StatsStore) -> Result<()> {
    let path = stats_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(store)?;
    fs::write(&path, contents)?;
    Ok(())
}

/// Record a build timing. Called by other modules to track performance.
#[allow(dead_code)]
pub fn record(command: &str, start: Instant, success: bool) {
    let duration = start.elapsed().as_secs_f64();
    if duration < 0.5 {
        return; // Don't record trivial operations
    }

    let record = BuildRecord {
        timestamp: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        command: command.to_string(),
        duration_secs: duration,
        success,
    };

    if let Ok(mut store) = load_stats() {
        store.builds.push_back(record);
        while store.builds.len() > 100 {
            store.builds.pop_front();
        }
        save_stats(&store).ok();
    }
}

/// Show build time statistics.
pub fn show() -> Result<()> {
    let store = load_stats()?;

    if store.builds.is_empty() {
        crate::output::info("no build history recorded yet");
        crate::output::step(
            "hint",
            "build stats are recorded automatically as you use rx",
        );
        return Ok(());
    }

    use owo_colors::OwoColorize;

    println!("{}", "Build Time Statistics".bold());
    println!("{}", "─".repeat(60));

    // Group by command
    let mut by_command: std::collections::HashMap<&str, Vec<f64>> =
        std::collections::HashMap::new();
    for record in &store.builds {
        by_command
            .entry(&record.command)
            .or_default()
            .push(record.duration_secs);
    }

    let mut commands: Vec<&&str> = by_command.keys().collect();
    commands.sort();

    for cmd in commands {
        let times = &by_command[cmd];
        let count = times.len();
        let avg = times.iter().sum::<f64>() / count as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        println!(
            "  {:<16} {} runs  avg {:.1}s  min {:.1}s  max {:.1}s",
            cmd.cyan(),
            count,
            avg,
            min,
            max
        );
    }

    println!("{}", "─".repeat(60));

    // Recent history
    let recent: Vec<&BuildRecord> = store.builds.iter().rev().take(10).collect();
    println!("\n{}", "Recent builds:".bold());
    for record in recent {
        let status = if record.success {
            "ok".green().to_string()
        } else {
            "fail".red().to_string()
        };
        // Show just the date/time portion
        let time = &record.timestamp[..19];
        println!(
            "  {} {:<16} {:.1}s  {}",
            time.dimmed(),
            record.command,
            record.duration_secs,
            status
        );
    }

    println!(
        "\n  {} {} total records",
        "history:".dimmed(),
        store.builds.len()
    );

    Ok(())
}

/// Clear all recorded stats.
pub fn clear() -> Result<()> {
    let path = stats_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    crate::output::success("build stats cleared");
    Ok(())
}
