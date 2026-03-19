use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Global quiet mode flag — suppresses info/step output.
static QUIET: AtomicBool = AtomicBool::new(false);
/// Global verbose mode flag — shows extra detail.
static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(q: bool) {
    QUIET.store(q, Ordering::Relaxed);
}

pub fn set_verbose(v: bool) {
    VERBOSE.store(v, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

pub fn info(msg: &str) {
    if !is_quiet() {
        eprintln!("{} {msg}", "[rx]".cyan().bold());
    }
}

pub fn success(msg: &str) {
    if !is_quiet() {
        eprintln!("{} {msg}", "[rx]".green().bold());
    }
}

pub fn warn(msg: &str) {
    eprintln!("{} {msg}", "[rx]".yellow().bold());
}

pub fn error(msg: &str) {
    eprintln!("{} {msg}", "[rx]".red().bold());
}

pub fn step(label: &str, detail: &str) {
    if !is_quiet() {
        eprintln!("{} {detail}", format!("[{label}]").dimmed());
    }
}

pub fn verbose(msg: &str) {
    if is_verbose() {
        eprintln!("{} {msg}", "[rx]".dimmed());
    }
}

/// Create a spinner for long-running operations.
pub fn spinner(msg: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// Helper to time an operation and report duration.
pub struct Timer {
    label: String,
    start: Instant,
}

impl Timer {
    pub fn start(label: &str) -> Self {
        Self {
            label: label.to_string(),
            start: Instant::now(),
        }
    }

    pub fn finish(&self) {
        if !is_quiet() {
            let elapsed = self.start.elapsed();
            let secs = elapsed.as_secs_f64();
            if secs >= 0.1 {
                eprintln!(
                    "{} {} completed in {:.1}s",
                    "[rx]".green().bold(),
                    self.label,
                    secs
                );
            }
        }
    }
}
