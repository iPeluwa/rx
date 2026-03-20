use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::BufRead;
use std::process::{Command, Stdio};

/// A parsed cargo JSON diagnostic message.
#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    package_id: Option<String>,
    #[serde(default)]
    target: Option<CargoTarget>,
    #[serde(default)]
    message: Option<CompilerMessage>,
    #[serde(default)]
    fresh: Option<bool>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
}

#[derive(Deserialize)]
struct CompilerMessage {
    #[allow(dead_code)]
    message: String,
    level: String,
    #[serde(default)]
    rendered: Option<String>,
    #[serde(default)]
    code: Option<DiagnosticCode>,
}

#[derive(Deserialize)]
struct DiagnosticCode {
    code: String,
}

/// Summary of a cargo build run, parsed from JSON output.
pub struct BuildSummary {
    #[allow(dead_code)]
    pub compiled: Vec<String>,
    #[allow(dead_code)]
    pub fresh: Vec<String>,
    #[allow(dead_code)]
    pub warnings: usize,
    #[allow(dead_code)]
    pub errors: usize,
    pub success: bool,
}

/// Run a cargo command with `--message-format=json` and parse the output,
/// rendering a cleaner progress view with smart error hints.
pub fn run_cargo_json(args: &[&str], env_vars: &[(&str, &str)]) -> Result<BuildSummary> {
    let mut cmd = Command::new("cargo");
    cmd.args(args)
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn().context("failed to start cargo")?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let reader = std::io::BufReader::new(stdout);

    let mut compiled = Vec::new();
    let mut fresh = Vec::new();
    let mut warnings: usize = 0;
    let mut errors: usize = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let msg: CargoMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match msg.reason.as_str() {
            "compiler-artifact" => {
                let name = msg
                    .target
                    .as_ref()
                    .map(|t| t.name.clone())
                    .or_else(|| {
                        msg.package_id
                            .as_ref()
                            .and_then(|id| id.split_whitespace().next().map(String::from))
                    })
                    .unwrap_or_else(|| "<unknown>".into());

                if msg.fresh == Some(true) {
                    fresh.push(name.clone());
                } else {
                    crate::output::step("compiling", &name);
                    compiled.push(name);
                }
            }
            "compiler-message" => {
                if let Some(ref message) = msg.message {
                    match message.level.as_str() {
                        "warning" => {
                            warnings += 1;
                            if let Some(ref rendered) = message.rendered {
                                eprint!("{rendered}");
                            }
                        }
                        "error" => {
                            errors += 1;
                            if let Some(ref rendered) = message.rendered {
                                eprint!("{rendered}");
                            }
                            // Show smart hint for known error codes
                            if let Some(ref code) = message.code {
                                if let Some(hint) = crate::hints::get_hint(&code.code) {
                                    eprintln!("\n  {} {hint}", "rx hint:".cyan());
                                }
                            }
                        }
                        _ => {
                            if let Some(ref rendered) = message.rendered {
                                crate::output::verbose(&format!("{}", rendered.trim()));
                            }
                        }
                    }
                }
            }
            "build-finished" => {}
            _ => {}
        }
    }

    let status = child.wait().context("cargo process failed")?;

    if !compiled.is_empty() {
        crate::output::verbose(&format!(
            "compiled {} crate(s), {} fresh",
            compiled.len(),
            fresh.len()
        ));
    }

    if warnings > 0 {
        crate::output::warn(&format!("{warnings} warning(s) generated"));
    }

    Ok(BuildSummary {
        compiled,
        fresh,
        warnings,
        errors,
        success: status.success(),
    })
}

use owo_colors::OwoColorize;
