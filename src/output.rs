use owo_colors::OwoColorize;

pub fn info(msg: &str) {
    eprintln!("{} {msg}", "[rx]".cyan().bold());
}

pub fn success(msg: &str) {
    eprintln!("{} {msg}", "[rx]".green().bold());
}

pub fn warn(msg: &str) {
    eprintln!("{} {msg}", "[rx]".yellow().bold());
}

#[allow(dead_code)]
pub fn error(msg: &str) {
    eprintln!("{} {msg}", "[rx]".red().bold());
}

pub fn step(label: &str, detail: &str) {
    eprintln!("{} {detail}", format!("[{label}]").dimmed());
}
