use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the compiled rx binary.
fn rx_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("rx");
    path
}

/// Run rx with the given args in a directory.
fn rx(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(rx_bin())
        .args(args)
        .current_dir(dir)
        .env("HOME", dir.parent().unwrap_or(dir))
        .output()
        .expect("failed to run rx")
}

fn create_cargo_project(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let project_dir = dir.join(name);
    Command::new("cargo")
        .args(["new", name])
        .current_dir(dir)
        .output()
        .expect("failed to create cargo project");
    project_dir
}

#[test]
fn integration_init_creates_rx_toml() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "myapp");

    let output = rx(&project, &["init"]);
    assert!(output.status.success(), "rx init failed");
    assert!(project.join("rx.toml").exists());

    // Verify it's valid TOML
    let contents = fs::read_to_string(project.join("rx.toml")).unwrap();
    let _: toml::Table = toml::from_str(&contents).expect("rx.toml is not valid TOML");
}

#[test]
fn integration_init_refuses_duplicate() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "myapp2");

    rx(&project, &["init"]);
    let output = rx(&project, &["init"]);
    assert!(!output.status.success(), "second rx init should fail");
}

#[test]
fn integration_config_shows_defaults() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "myapp3");

    let output = rx(&project, &["config"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[build]"));
    assert!(stdout.contains("[test]"));
}

#[test]
fn integration_build_succeeds() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "buildtest");

    let output = rx(&project, &["build", "--quiet"]);
    assert!(
        output.status.success(),
        "rx build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_build_release() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "reltest");

    let output = rx(&project, &["build", "--release", "--quiet"]);
    assert!(
        output.status.success(),
        "rx build --release failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.join("target/release").exists());
}

#[test]
fn integration_test_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "testproj");

    let output = rx(&project, &["test", "--quiet"]);
    assert!(
        output.status.success(),
        "rx test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_fmt_check() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "fmtproj");

    let output = rx(&project, &["fmt", "--check", "--quiet"]);
    assert!(
        output.status.success(),
        "rx fmt --check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_doctor_runs() {
    let tmp = TempDir::new().unwrap();
    let output = rx(tmp.path(), &["doctor"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rustc"));
    assert!(stdout.contains("cargo"));
}

#[test]
fn integration_verbose_flag() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "verbosetest");

    let output = rx(&project, &["--verbose", "config"]);
    assert!(output.status.success());
}

#[test]
fn integration_quiet_flag() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "quiettest");

    let output = rx(&project, &["--quiet", "build"]);
    assert!(output.status.success());
    // Quiet mode should suppress info messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("[rx]"),
        "quiet mode should suppress [rx] messages, got: {stderr}"
    );
}

#[test]
fn integration_compat_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "int_compat");

    let output = rx(&project, &["compat", "--quiet"]);
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "rx compat exited with unexpected code: {code}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "rx compat panicked: {stderr}"
    );
}

#[test]
fn integration_lockfile_check_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "int_lockcheck");

    // Generate Cargo.lock by building
    Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(&project)
        .output()
        .expect("cargo build failed");

    let output = rx(&project, &["lockfile", "check"]);
    assert!(
        output.status.success(),
        "rx lockfile check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_lockfile_enforce_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "int_lockenforce");

    // Generate Cargo.lock by building
    Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(&project)
        .output()
        .expect("cargo build failed");

    let output = rx(&project, &["lockfile", "enforce"]);
    assert!(
        output.status.success(),
        "rx lockfile enforce failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_telemetry_status_runs() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["telemetry", "status"]);
    assert!(
        output.status.success(),
        "rx telemetry status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_telemetry_on_off() {
    let tmp = TempDir::new().unwrap();

    let output_on = rx(tmp.path(), &["telemetry", "on"]);
    assert!(
        output_on.status.success(),
        "rx telemetry on failed: {}",
        String::from_utf8_lossy(&output_on.stderr)
    );

    let output_off = rx(tmp.path(), &["telemetry", "off"]);
    assert!(
        output_off.status.success(),
        "rx telemetry off failed: {}",
        String::from_utf8_lossy(&output_off.stderr)
    );
}

#[test]
#[ignore] // requires sandbox-exec (macOS only, not available in all environments)
fn integration_sandbox_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "int_sandbox");

    let output = rx(&project, &["sandbox", "--quiet"]);
    assert!(
        output.status.success(),
        "rx sandbox failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_explain_known_code() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["explain", "E0502"]);
    assert!(
        output.status.success(),
        "rx explain E0502 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("borrow"),
        "rx explain E0502 should mention 'borrow', got: {stdout}"
    );
}

#[test]
fn integration_explain_unknown_code() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["explain", "E9999"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "rx explain E9999 panicked: {stderr}"
    );
}

#[test]
fn integration_worker_status() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["worker", "status"]);
    assert!(
        output.status.success(),
        "rx worker status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_registry_list() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["registry", "list"]);
    assert!(
        output.status.success(),
        "rx registry list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("crates.io"),
        "rx registry list should mention 'crates.io', got: {stdout}"
    );
}
