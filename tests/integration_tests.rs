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
fn integration_check_succeeds() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "checktest");

    let output = rx(&project, &["check", "--quiet"]);
    assert!(
        output.status.success(),
        "rx check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_lint_succeeds() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "linttest");

    let output = rx(&project, &["lint", "--quiet"]);
    assert!(
        output.status.success(),
        "rx lint failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_ci_runs() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "citest");

    // Create rx.toml
    rx(&project, &["init"]);

    let output = rx(&project, &["ci", "--quiet"]);
    assert!(
        output.status.success(),
        "rx ci failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_run_lists_tasks() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "tasklisttest");

    let rx_toml = project.join("rx.toml");
    fs::write(
        &rx_toml,
        "[tasks]\nmytask = \"echo test\"\n\n[scripts]\nlegacy = \"echo legacy\"\n",
    )
    .unwrap();

    let output = rx(&project, &["run"]);
    assert!(
        output.status.success(),
        "rx run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mytask") && stdout.contains("legacy") && stdout.contains("ci"),
        "rx run should list user tasks, legacy scripts, and built-ins, got: {stdout}"
    );
}

#[test]
fn integration_run_executes_dependency_chain() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "taskchaintest");

    let rx_toml = project.join("rx.toml");
    fs::write(
        &rx_toml,
        "[tasks]\nfirst = \"echo ran-first\"\n\n[tasks.second]\ncommand = \"echo ran-second\"\ndepends-on = [\"first\"]\n",
    )
    .unwrap();

    let output = rx(&project, &["run", "second"]);
    assert!(
        output.status.success(),
        "rx run second failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_pos = stdout.find("ran-first").expect("dependency did not run");
    let second_pos = stdout.find("ran-second").expect("task did not run");
    assert!(first_pos < second_pos, "dependency ran after the task");
}

#[test]
fn integration_run_fails_on_unknown_task() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "taskunknowntest");

    let output = rx(&project, &["run", "no-such-task"]);
    assert!(!output.status.success(), "unknown task should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown task"),
        "expected unknown-task error, got: {stderr}"
    );
}

#[test]
fn integration_run_failing_task_propagates() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "taskfailtest");

    let rx_toml = project.join("rx.toml");
    fs::write(&rx_toml, "[tasks]\nbad = \"exit 3\"\n").unwrap();

    let output = rx(&project, &["run", "bad"]);
    assert!(!output.status.success(), "failing task should fail rx run");
}

#[test]
fn integration_config_with_profile() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "profiletest");

    // Create rx.toml with a profile
    rx(&project, &["init"]);
    let rx_toml = project.join("rx.toml");
    let mut contents = fs::read_to_string(&rx_toml).unwrap();
    contents.push_str("\n[profile.ci]\n[profile.ci.build]\ncache = false\n");
    fs::write(&rx_toml, contents).unwrap();

    let output = rx(&project, &["--profile", "ci", "config"]);
    assert!(
        output.status.success(),
        "rx --profile ci config failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[build]"),
        "config output should show [build]"
    );
}

#[test]
fn integration_clean_succeeds() {
    let tmp = TempDir::new().unwrap();
    let project = create_cargo_project(tmp.path(), "cleantest");

    // Build the project first
    rx(&project, &["build", "--quiet"]);
    assert!(
        project.join("target").exists(),
        "target dir should exist after build"
    );

    let output = rx(&project, &["clean"]);
    assert!(
        output.status.success(),
        "rx clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_stats_show() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["stats", "show"]);
    assert!(
        output.status.success(),
        "rx stats show failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn integration_help_output() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["--help"]);
    assert!(
        output.status.success(),
        "rx --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage") || stdout.contains("USAGE"),
        "help output should contain usage information, got: {stdout}"
    );
}

#[test]
fn integration_version_output() {
    let tmp = TempDir::new().unwrap();

    let output = rx(tmp.path(), &["--version"]);
    assert!(
        output.status.success(),
        "rx --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rx"),
        "version output should contain 'rx', got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// --affected integration tests (two-member workspace with git history)
// ---------------------------------------------------------------------------

/// Create a git workspace with two members where `cli` depends on `core`,
/// plus a `probe` task that echoes RX_AFFECTED_PACKAGES.
fn create_workspace_with_git(dir: &std::path::Path) -> std::path::PathBuf {
    let ws = dir.join("ws");
    fs::create_dir_all(ws.join("core/src")).unwrap();
    fs::create_dir_all(ws.join("cli/src")).unwrap();

    fs::write(
        ws.join("Cargo.toml"),
        "[workspace]\nmembers = [\"core\", \"cli\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        ws.join("core/Cargo.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(ws.join("core/src/lib.rs"), "pub fn core() -> u32 { 1 }\n").unwrap();
    fs::write(
        ws.join("cli/Cargo.toml"),
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
    )
    .unwrap();
    fs::write(ws.join("cli/src/lib.rs"), "pub fn cli() -> u32 { 2 }\n").unwrap();
    fs::write(
        ws.join("rx.toml"),
        "[tasks]\nprobe = \"echo AFFECTED=[$RX_AFFECTED_PACKAGES]\"\n",
    )
    .unwrap();

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&ws)
            .output()
            .expect("git failed to run");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@test",
        "commit",
        "-qm",
        "init",
    ]);

    ws
}

#[test]
fn integration_affected_leaf_change_selects_only_leaf() {
    let tmp = TempDir::new().unwrap();
    let ws = create_workspace_with_git(tmp.path());

    // Change only `cli` (the leaf) — core must not be selected.
    fs::write(ws.join("cli/src/lib.rs"), "pub fn cli() -> u32 { 22 }\n").unwrap();

    let output = rx(&ws, &["run", "probe", "--affected", "--base", "HEAD"]);
    assert!(
        output.status.success(),
        "rx run probe --affected failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AFFECTED=[cli]"),
        "expected only cli affected, got: {stdout}"
    );
}

#[test]
fn integration_affected_base_change_propagates_to_dependents() {
    let tmp = TempDir::new().unwrap();
    let ws = create_workspace_with_git(tmp.path());

    // Change `core` — `cli` depends on it, so both are affected.
    fs::write(ws.join("core/src/lib.rs"), "pub fn core() -> u32 { 11 }\n").unwrap();

    let output = rx(&ws, &["run", "probe", "--affected", "--base", "HEAD"]);
    assert!(
        output.status.success(),
        "rx run probe --affected failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AFFECTED=[cli core]") || stdout.contains("AFFECTED=[core cli]"),
        "expected core change to propagate to cli, got: {stdout}"
    );
}

#[test]
fn integration_affected_no_changes_skips() {
    let tmp = TempDir::new().unwrap();
    let ws = create_workspace_with_git(tmp.path());

    let output = rx(&ws, &["run", "probe", "--affected", "--base", "HEAD"]);
    assert!(
        output.status.success(),
        "rx run probe --affected (clean) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("AFFECTED=["),
        "probe should not run when nothing changed, got: {stdout}"
    );
}

#[test]
fn integration_ci_affected_no_changes_skips() {
    let tmp = TempDir::new().unwrap();
    let ws = create_workspace_with_git(tmp.path());

    let output = rx(&ws, &["ci", "--affected", "--base", "HEAD"]);
    assert!(
        output.status.success(),
        "rx ci --affected (clean) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
