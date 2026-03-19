use std::fs;
use tempfile::TempDir;

// We test the config module through the public binary interface and by
// writing rx.toml files and parsing them.

#[test]
fn default_config_round_trips() {
    let config = rx::config::RxConfig::default();
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: rx::config::RxConfig = toml::from_str(&serialized).unwrap();

    assert_eq!(deserialized.build.linker, "auto");
    assert!(deserialized.build.cache);
    assert_eq!(deserialized.build.jobs, 0);
    assert!(deserialized.build.rustflags.is_empty());
    assert_eq!(deserialized.test.runner, "auto");
    assert_eq!(deserialized.lint.severity, "deny");
    assert_eq!(deserialized.watch.cmd, "build");
    assert!(deserialized.scripts.is_empty());
    assert!(deserialized.env.is_empty());
}

#[test]
fn parse_partial_config() {
    let toml_str = r#"
[build]
linker = "mold"
jobs = 8

[scripts]
ci = "cargo test"
"#;
    let config: rx::config::RxConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.build.linker, "mold");
    assert_eq!(config.build.jobs, 8);
    assert!(config.build.cache); // default
    assert_eq!(config.scripts.get("ci").unwrap(), "cargo test");
    assert_eq!(config.test.runner, "auto"); // default
}

#[test]
fn parse_full_config() {
    let toml_str = r#"
[build]
linker = "lld"
rustflags = ["-Cdebuginfo=1"]
cache = false
jobs = 4

[test]
runner = "nextest"
extra_args = ["--no-fail-fast"]

[lint]
severity = "warn"
extra_lints = ["clippy::pedantic"]

[fmt]
extra_args = ["--edition", "2021"]

[watch]
cmd = "check"
ignore = ["*.md"]

[scripts]
ci = "cargo fmt --check && cargo test"
bench = "cargo bench"

[env]
RUST_BACKTRACE = "1"
RUST_LOG = "debug"
"#;
    let config: rx::config::RxConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.build.linker, "lld");
    assert_eq!(config.build.rustflags, vec!["-Cdebuginfo=1"]);
    assert!(!config.build.cache);
    assert_eq!(config.build.jobs, 4);
    assert_eq!(config.test.runner, "nextest");
    assert_eq!(config.test.extra_args, vec!["--no-fail-fast"]);
    assert_eq!(config.lint.severity, "warn");
    assert_eq!(config.lint.extra_lints, vec!["clippy::pedantic"]);
    assert_eq!(config.fmt.extra_args, vec!["--edition", "2021"]);
    assert_eq!(config.watch.cmd, "check");
    assert_eq!(config.watch.ignore, vec!["*.md"]);
    assert_eq!(config.scripts.len(), 2);
    assert_eq!(config.env.get("RUST_BACKTRACE").unwrap(), "1");
}

#[test]
fn merge_project_overrides_global() {
    let global = rx::config::RxConfig {
        build: rx::config::BuildConfig {
            linker: "mold".into(),
            rustflags: vec!["-Copt-level=2".into()],
            cache: true,
            jobs: 8,
        },
        test: rx::config::TestConfig {
            runner: "nextest".into(),
            extra_args: vec!["--retries=2".into()],
        },
        lint: rx::config::LintConfig {
            severity: "warn".into(),
            extra_lints: vec!["clippy::pedantic".into()],
        },
        ..Default::default()
    };

    let project = rx::config::RxConfig {
        build: rx::config::BuildConfig {
            linker: "lld".into(),
            jobs: 4,
            ..Default::default()
        },
        lint: rx::config::LintConfig {
            extra_lints: vec!["clippy::nursery".into()],
            ..Default::default()
        },
        ..Default::default()
    };

    let merged = rx::config::merge(global, project);

    // Project overrides linker (non-default)
    assert_eq!(merged.build.linker, "lld");
    // Project rustflags empty -> falls back to global
    assert_eq!(merged.build.rustflags, vec!["-Copt-level=2"]);
    // Both true -> true
    assert!(merged.build.cache);
    // Project jobs non-zero -> overrides
    assert_eq!(merged.build.jobs, 4);
    // Project runner is "auto" (default) -> falls back to global
    assert_eq!(merged.test.runner, "nextest");
    // Project test extra_args empty -> falls back to global
    assert_eq!(merged.test.extra_args, vec!["--retries=2"]);
    // Project lint severity is "deny" (default) -> falls back to global
    assert_eq!(merged.lint.severity, "warn");
    // Extra lints are concatenated
    assert_eq!(
        merged.lint.extra_lints,
        vec!["clippy::pedantic", "clippy::nursery"]
    );
}

#[test]
fn merge_scripts_and_env_are_combined() {
    let mut global_scripts = std::collections::HashMap::new();
    global_scripts.insert("lint".into(), "cargo clippy".into());
    global_scripts.insert("test".into(), "cargo test".into());

    let mut project_scripts = std::collections::HashMap::new();
    project_scripts.insert("test".into(), "cargo nextest run".into()); // override
    project_scripts.insert("deploy".into(), "./deploy.sh".into()); // new

    let mut global_env = std::collections::HashMap::new();
    global_env.insert("RUST_BACKTRACE".into(), "1".into());

    let mut project_env = std::collections::HashMap::new();
    project_env.insert("RUST_LOG".into(), "debug".into());

    let global = rx::config::RxConfig {
        scripts: global_scripts,
        env: global_env,
        ..Default::default()
    };
    let project = rx::config::RxConfig {
        scripts: project_scripts,
        env: project_env,
        ..Default::default()
    };

    let merged = rx::config::merge(global, project);
    assert_eq!(merged.scripts.len(), 3);
    assert_eq!(merged.scripts.get("test").unwrap(), "cargo nextest run");
    assert_eq!(merged.scripts.get("lint").unwrap(), "cargo clippy");
    assert_eq!(merged.scripts.get("deploy").unwrap(), "./deploy.sh");
    assert_eq!(merged.env.len(), 2);
}

#[test]
fn load_from_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("rx.toml");
    fs::write(
        &path,
        r#"
[build]
linker = "mold"
cache = false

[scripts]
hello = "echo hi"
"#,
    )
    .unwrap();

    let config = rx::config::load_from_path(&path).unwrap();
    assert_eq!(config.build.linker, "mold");
    assert!(!config.build.cache);
    assert_eq!(config.scripts.get("hello").unwrap(), "echo hi");
}

#[test]
fn load_for_dir_missing_file_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let config = rx::config::load_for_dir(dir.path()).unwrap();
    assert_eq!(config.build.linker, "auto");
    assert!(config.build.cache);
}

#[test]
fn init_config_creates_valid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("rx.toml");
    rx::config::init_config(&path).unwrap();

    assert!(path.exists());
    let config = rx::config::load_from_path(&path).unwrap();
    assert_eq!(config.build.linker, "auto");
    assert!(config.build.cache);
    assert_eq!(config.test.runner, "auto");
}
