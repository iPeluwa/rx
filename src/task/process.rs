use anyhow::{Context, Result};
use std::process::Command;

/// Result of a captured shell task run.
pub struct ProcessResult {
    pub success: bool,
    pub output: String,
}

/// Run a shell command, streaming output directly to the terminal.
/// Used when a task runs alone and its output should appear live.
pub fn run_streamed(command: &str, env: &[(String, String)]) -> Result<bool> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    for (key, value) in env {
        cmd.env(key, value);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to run: {command}"))?;
    Ok(status.success())
}

/// Run a shell command with captured output. Used when several independent
/// tasks run concurrently so their output doesn't interleave.
pub fn run_captured(command: &str, env: &[(String, String)]) -> ProcessResult {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    for (key, value) in env {
        cmd.env(key, value);
    }
    match cmd.output() {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            ProcessResult {
                success: output.status.success(),
                output: text,
            }
        }
        Err(e) => ProcessResult {
            success: false,
            output: format!("failed to run `{command}`: {e}"),
        },
    }
}
