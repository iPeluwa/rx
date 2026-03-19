use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Config schema
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct RxConfig {
    pub build: BuildConfig,
    pub test: TestConfig,
    pub lint: LintConfig,
    pub fmt: FmtConfig,
    pub watch: WatchConfig,
    pub scripts: HashMap<String, String>,
    pub env: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BuildConfig {
    /// Linker to use: "auto", "mold", "lld", or "system"
    pub linker: String,
    /// Extra RUSTFLAGS to append
    pub rustflags: Vec<String>,
    /// Enable the global artifact cache
    pub cache: bool,
    /// Default number of parallel jobs (0 = auto)
    pub jobs: u32,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            linker: "auto".into(),
            rustflags: vec![],
            cache: true,
            jobs: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct TestConfig {
    /// Test runner: "auto", "nextest", or "cargo"
    pub runner: String,
    /// Extra arguments always passed to the test runner
    pub extra_args: Vec<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            runner: "auto".into(),
            extra_args: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LintConfig {
    /// Clippy severity: "deny", "warn", or "allow"
    pub severity: String,
    /// Extra clippy lints to enable (e.g. "clippy::pedantic")
    pub extra_lints: Vec<String>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            severity: "deny".into(),
            extra_lints: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct FmtConfig {
    /// Extra rustfmt arguments
    pub extra_args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WatchConfig {
    /// Default command to run on file changes
    pub cmd: String,
    /// File patterns to ignore
    pub ignore: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            cmd: "build".into(),
            ignore: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Config resolution
// ---------------------------------------------------------------------------

/// Find rx.toml by walking up from cwd.
fn find_project_config() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("rx.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Global config at ~/.rx/config.toml.
fn global_config_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".rx").join("config.toml");
    if path.exists() { Some(path) } else { None }
}

pub fn load_from_path(path: &Path) -> Result<RxConfig> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

/// Merge two configs. Project-level values override global values.
pub fn merge(global: RxConfig, project: RxConfig) -> RxConfig {
    RxConfig {
        build: BuildConfig {
            linker: if project.build.linker != "auto" {
                project.build.linker
            } else {
                global.build.linker
            },
            rustflags: if project.build.rustflags.is_empty() {
                global.build.rustflags
            } else {
                project.build.rustflags
            },
            cache: project.build.cache && global.build.cache,
            jobs: if project.build.jobs > 0 {
                project.build.jobs
            } else {
                global.build.jobs
            },
        },
        test: TestConfig {
            runner: if project.test.runner != "auto" {
                project.test.runner
            } else {
                global.test.runner
            },
            extra_args: if project.test.extra_args.is_empty() {
                global.test.extra_args
            } else {
                project.test.extra_args
            },
        },
        lint: LintConfig {
            severity: if project.lint.severity != "deny" {
                project.lint.severity
            } else {
                global.lint.severity
            },
            extra_lints: {
                let mut lints = global.lint.extra_lints;
                lints.extend(project.lint.extra_lints);
                lints
            },
        },
        fmt: FmtConfig {
            extra_args: if project.fmt.extra_args.is_empty() {
                global.fmt.extra_args
            } else {
                project.fmt.extra_args
            },
        },
        watch: WatchConfig {
            cmd: if project.watch.cmd != "build" {
                project.watch.cmd
            } else {
                global.watch.cmd
            },
            ignore: {
                let mut patterns = global.watch.ignore;
                patterns.extend(project.watch.ignore);
                patterns
            },
        },
        scripts: {
            let mut scripts = global.scripts;
            scripts.extend(project.scripts);
            scripts
        },
        env: {
            let mut env = global.env;
            env.extend(project.env);
            env
        },
    }
}

/// Load the resolved config (global merged with project-level).
pub fn load() -> Result<RxConfig> {
    let global = match global_config_path() {
        Some(path) => load_from_path(&path).unwrap_or_default(),
        None => RxConfig::default(),
    };

    let project = match find_project_config() {
        Some(path) => load_from_path(&path)?,
        None => RxConfig::default(),
    };

    Ok(merge(global, project))
}

/// Load config for a specific directory (used by workspace per-member configs).
pub fn load_for_dir(dir: &Path) -> Result<RxConfig> {
    let config_path = dir.join("rx.toml");
    if config_path.exists() {
        load_from_path(&config_path)
    } else {
        Ok(RxConfig::default())
    }
}

/// Generate a starter rx.toml.
pub fn init_config(path: &Path) -> Result<()> {
    let config = RxConfig::default();
    let contents = toml::to_string_pretty(&config).context("failed to serialize config")?;
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Pretty-print the resolved config.
pub fn show(config: &RxConfig) -> Result<()> {
    let contents = toml::to_string_pretty(config)?;
    println!("{contents}");
    Ok(())
}
