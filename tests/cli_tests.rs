use clap::Parser;
use rx::cli::Cli;

fn parse(args: &[&str]) -> Cli {
    let mut full = vec!["rx"];
    full.extend(args);
    Cli::parse_from(full)
}

#[test]
fn parse_build_defaults() {
    let cli = parse(&["build"]);
    match cli.command {
        rx::cli::Command::Build {
            release,
            package,
            target,
        } => {
            assert!(!release);
            assert!(package.is_none());
            assert!(target.is_none());
        }
        _ => panic!("expected Build"),
    }
}

#[test]
fn parse_build_release() {
    let cli = parse(&["build", "--release"]);
    match cli.command {
        rx::cli::Command::Build { release, .. } => assert!(release),
        _ => panic!("expected Build"),
    }
}

#[test]
fn parse_build_package() {
    let cli = parse(&["build", "--package", "mylib"]);
    match cli.command {
        rx::cli::Command::Build { package, .. } => assert_eq!(package.unwrap(), "mylib"),
        _ => panic!("expected Build"),
    }
}

#[test]
fn parse_test_with_filter() {
    let cli = parse(&["test", "my_test"]);
    match cli.command {
        rx::cli::Command::Test { filter, .. } => assert_eq!(filter.unwrap(), "my_test"),
        _ => panic!("expected Test"),
    }
}

#[test]
fn parse_fmt_check() {
    let cli = parse(&["fmt", "--check"]);
    match cli.command {
        rx::cli::Command::Fmt { check } => assert!(check),
        _ => panic!("expected Fmt"),
    }
}

#[test]
fn parse_lint_fix() {
    let cli = parse(&["lint", "--fix"]);
    match cli.command {
        rx::cli::Command::Lint { fix } => assert!(fix),
        _ => panic!("expected Lint"),
    }
}

#[test]
fn parse_pkg_add() {
    let cli = parse(&["pkg", "add", "serde@1.0", "--dev"]);
    match cli.command {
        rx::cli::Command::Pkg(rx::cli::PkgCommand::Add { spec, dev, build }) => {
            assert_eq!(spec, "serde@1.0");
            assert!(dev);
            assert!(!build);
        }
        _ => panic!("expected Pkg Add"),
    }
}

#[test]
fn parse_cache_gc() {
    let cli = parse(&["cache", "gc", "--older-than", "60"]);
    match cli.command {
        rx::cli::Command::Cache(rx::cli::CacheCommand::Gc { older_than }) => {
            assert_eq!(older_than, 60)
        }
        _ => panic!("expected Cache Gc"),
    }
}

#[test]
fn parse_clean_with_gc() {
    let cli = parse(&["clean", "--gc"]);
    match cli.command {
        rx::cli::Command::Clean { gc, all } => {
            assert!(gc);
            assert!(!all);
        }
        _ => panic!("expected Clean"),
    }
}

#[test]
fn parse_new_lib() {
    let cli = parse(&["new", "mylib", "--lib"]);
    match cli.command {
        rx::cli::Command::New { name, lib, .. } => {
            assert_eq!(name, "mylib");
            assert!(lib);
        }
        _ => panic!("expected New"),
    }
}

#[test]
fn parse_init() {
    let cli = parse(&["init"]);
    match cli.command {
        rx::cli::Command::Init { ci, migrate } => {
            assert!(ci.is_none());
            assert!(!migrate);
        }
        _ => panic!("expected Init"),
    }
}

#[test]
fn parse_config() {
    let cli = parse(&["config"]);
    assert!(matches!(cli.command, rx::cli::Command::Config));
}

#[test]
fn parse_ws_graph() {
    let cli = parse(&["ws", "graph"]);
    match cli.command {
        rx::cli::Command::Ws(rx::cli::WsCommand::Graph) => {}
        _ => panic!("expected Ws Graph"),
    }
}

#[test]
fn parse_ws_run() {
    let cli = parse(&["ws", "run", "test", "--", "--release"]);
    match cli.command {
        rx::cli::Command::Ws(rx::cli::WsCommand::Run { cmd, args }) => {
            assert_eq!(cmd, "test");
            assert_eq!(args, vec!["--release"]);
        }
        _ => panic!("expected Ws Run"),
    }
}

#[test]
fn parse_ws_script_with_packages() {
    let cli = parse(&["ws", "script", "ci", "--packages", "core,utils"]);
    match cli.command {
        rx::cli::Command::Ws(rx::cli::WsCommand::Script { name, packages }) => {
            assert_eq!(name, "ci");
            assert_eq!(packages, vec!["core", "utils"]);
        }
        _ => panic!("expected Ws Script"),
    }
}

#[test]
fn parse_build_target() {
    let cli = parse(&["build", "--target", "x86_64-unknown-linux-gnu"]);
    match cli.command {
        rx::cli::Command::Build { target, .. } => {
            assert_eq!(target.unwrap(), "x86_64-unknown-linux-gnu");
        }
        _ => panic!("expected Build"),
    }
}

#[test]
fn parse_doctor() {
    let cli = parse(&["doctor"]);
    assert!(matches!(cli.command, rx::cli::Command::Doctor));
}

#[test]
fn parse_upgrade() {
    let cli = parse(&["upgrade"]);
    assert!(matches!(cli.command, rx::cli::Command::Upgrade));
}

#[test]
fn parse_bench() {
    let cli = parse(&["bench", "my_bench", "--package", "core"]);
    match cli.command {
        rx::cli::Command::Bench {
            filter, package, ..
        } => {
            assert_eq!(filter.unwrap(), "my_bench");
            assert_eq!(package.unwrap(), "core");
        }
        _ => panic!("expected Bench"),
    }
}

#[test]
fn parse_expand() {
    let cli = parse(&["expand", "my_module"]);
    match cli.command {
        rx::cli::Command::Expand { item } => {
            assert_eq!(item.unwrap(), "my_module");
        }
        _ => panic!("expected Expand"),
    }
}

#[test]
fn parse_publish_dry_run() {
    let cli = parse(&["publish", "--package", "core", "--dry-run"]);
    match cli.command {
        rx::cli::Command::Publish { package, dry_run } => {
            assert_eq!(package.unwrap(), "core");
            assert!(dry_run);
        }
        _ => panic!("expected Publish"),
    }
}

#[test]
fn parse_completions() {
    let cli = parse(&["completions", "bash"]);
    match cli.command {
        rx::cli::Command::Completions { shell, .. } => {
            assert_eq!(shell, clap_complete::Shell::Bash);
        }
        _ => panic!("expected Completions"),
    }
}

#[test]
fn parse_quiet_flag() {
    let cli = parse(&["--quiet", "build"]);
    assert!(cli.quiet);
    assert!(!cli.verbose);
}

#[test]
fn parse_verbose_flag() {
    let cli = parse(&["--verbose", "build"]);
    assert!(cli.verbose);
    assert!(!cli.quiet);
}

#[test]
fn parse_run_with_passthrough_args() {
    let cli = parse(&["run", "--", "--my-flag", "value"]);
    match cli.command {
        rx::cli::Command::Run { args, .. } => {
            assert_eq!(args, vec!["--my-flag", "value"]);
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_check() {
    let cli = parse(&["check", "--package", "core"]);
    match cli.command {
        rx::cli::Command::Check { package } => {
            assert_eq!(package.unwrap(), "core");
        }
        _ => panic!("expected Check"),
    }
}

#[test]
fn parse_fix() {
    let cli = parse(&["fix"]);
    assert!(matches!(cli.command, rx::cli::Command::Fix));
}

#[test]
fn parse_ci() {
    let cli = parse(&["ci"]);
    assert!(matches!(cli.command, rx::cli::Command::Ci));
}

#[test]
fn parse_size_release() {
    let cli = parse(&["size", "--release"]);
    match cli.command {
        rx::cli::Command::Size { release } => assert!(release),
        _ => panic!("expected Size"),
    }
}

#[test]
fn parse_tree_duplicates() {
    let cli = parse(&["tree", "--duplicates", "--depth", "3"]);
    match cli.command {
        rx::cli::Command::Tree { duplicates, depth } => {
            assert!(duplicates);
            assert_eq!(depth, Some(3));
        }
        _ => panic!("expected Tree"),
    }
}

#[test]
fn parse_outdated() {
    let cli = parse(&["outdated"]);
    assert!(matches!(cli.command, rx::cli::Command::Outdated));
}

#[test]
fn parse_audit() {
    let cli = parse(&["audit"]);
    assert!(matches!(cli.command, rx::cli::Command::Audit));
}

#[test]
fn parse_self_update() {
    let cli = parse(&["self-update"]);
    assert!(matches!(cli.command, rx::cli::Command::SelfUpdate));
}

#[test]
fn parse_script() {
    let cli = parse(&["script", "ci"]);
    match cli.command {
        rx::cli::Command::Script { name } => assert_eq!(name.unwrap(), "ci"),
        _ => panic!("expected Script"),
    }
}

#[test]
fn parse_script_list() {
    let cli = parse(&["script"]);
    match cli.command {
        rx::cli::Command::Script { name } => assert!(name.is_none()),
        _ => panic!("expected Script"),
    }
}

#[test]
fn parse_coverage() {
    let cli = parse(&["coverage", "--open"]);
    match cli.command {
        rx::cli::Command::Coverage { open, lcov } => {
            assert!(open);
            assert!(!lcov);
        }
        _ => panic!("expected Coverage"),
    }
}

#[test]
fn parse_deps() {
    let cli = parse(&["deps"]);
    assert!(matches!(cli.command, rx::cli::Command::Deps));
}

#[test]
fn parse_bloat() {
    let cli = parse(&["bloat", "--release", "--crates"]);
    match cli.command {
        rx::cli::Command::Bloat { release, crates } => {
            assert!(release);
            assert!(crates);
        }
        _ => panic!("expected Bloat"),
    }
}

#[test]
fn parse_doc_open() {
    let cli = parse(&["doc", "--open", "--no-deps"]);
    match cli.command {
        rx::cli::Command::Doc {
            open,
            no_deps,
            watch,
        } => {
            assert!(open);
            assert!(no_deps);
            assert!(!watch);
        }
        _ => panic!("expected Doc"),
    }
}

#[test]
fn parse_release() {
    let cli = parse(&["release", "1.2.3", "--dry-run"]);
    match cli.command {
        rx::cli::Command::Release {
            version,
            dry_run,
            no_push,
        } => {
            assert_eq!(version, "1.2.3");
            assert!(dry_run);
            assert!(!no_push);
        }
        _ => panic!("expected Release"),
    }
}

#[test]
fn parse_init_with_ci() {
    let cli = parse(&["init", "--ci"]);
    match cli.command {
        rx::cli::Command::Init { ci, .. } => assert_eq!(ci.unwrap(), "github"),
        _ => panic!("expected Init"),
    }
}

#[test]
fn parse_init_with_ci_gitlab() {
    let cli = parse(&["init", "--ci", "gitlab"]);
    match cli.command {
        rx::cli::Command::Init { ci, .. } => assert_eq!(ci.unwrap(), "gitlab"),
        _ => panic!("expected Init"),
    }
}

#[test]
fn parse_clean_all() {
    let cli = parse(&["clean", "--all"]);
    match cli.command {
        rx::cli::Command::Clean { gc, all } => {
            assert!(!gc);
            assert!(all);
        }
        _ => panic!("expected Clean"),
    }
}

#[test]
fn parse_profile_flag() {
    let cli = parse(&["--profile", "ci", "build"]);
    assert_eq!(cli.profile.unwrap(), "ci");
}

#[test]
fn parse_new_with_template() {
    let cli = parse(&["new", "myapp", "--template", "axum"]);
    match cli.command {
        rx::cli::Command::New { name, template, .. } => {
            assert_eq!(name, "myapp");
            assert_eq!(template.unwrap(), "axum");
        }
        _ => panic!("expected New"),
    }
}

#[test]
fn parse_release_patch() {
    let cli = parse(&["release", "patch"]);
    match cli.command {
        rx::cli::Command::Release { version, .. } => {
            assert_eq!(version, "patch");
        }
        _ => panic!("expected Release"),
    }
}

#[test]
fn parse_coverage_lcov() {
    let cli = parse(&["coverage", "--lcov"]);
    match cli.command {
        rx::cli::Command::Coverage { open, lcov } => {
            assert!(!open);
            assert!(lcov);
        }
        _ => panic!("expected Coverage"),
    }
}

#[test]
fn parse_doc_watch() {
    let cli = parse(&["doc", "--watch"]);
    match cli.command {
        rx::cli::Command::Doc { watch, .. } => assert!(watch),
        _ => panic!("expected Doc"),
    }
}

#[test]
fn parse_test_affected() {
    let cli = parse(&["test", "--affected", "--base", "main"]);
    match cli.command {
        rx::cli::Command::Test { affected, base, .. } => {
            assert!(affected);
            assert_eq!(base, "main");
        }
        _ => panic!("expected Test"),
    }
}

#[test]
fn parse_env_show() {
    let cli = parse(&["env", "show"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Env(rx::cli::EnvCommand::Show)
    ));
}

#[test]
fn parse_env_shell() {
    let cli = parse(&["env", "shell"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Env(rx::cli::EnvCommand::Shell)
    ));
}

#[test]
fn parse_plugin_list() {
    let cli = parse(&["plugin", "list"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Plugin(rx::cli::PluginCommand::List)
    ));
}

#[test]
fn parse_plugin_run() {
    let cli = parse(&["plugin", "run", "myplug", "--", "--flag"]);
    match cli.command {
        rx::cli::Command::Plugin(rx::cli::PluginCommand::Run { name, args }) => {
            assert_eq!(name, "myplug");
            assert_eq!(args, vec!["--flag"]);
        }
        _ => panic!("expected Plugin Run"),
    }
}

#[test]
fn parse_stats_show() {
    let cli = parse(&["stats", "show"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Stats(rx::cli::StatsCommand::Show)
    ));
}

#[test]
fn parse_stats_clear() {
    let cli = parse(&["stats", "clear"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Stats(rx::cli::StatsCommand::Clear)
    ));
}

#[test]
fn parse_init_migrate() {
    let cli = parse(&["init", "--migrate"]);
    match cli.command {
        rx::cli::Command::Init { migrate, .. } => assert!(migrate),
        _ => panic!("expected Init"),
    }
}

#[test]
fn parse_insights() {
    let cli = parse(&["insights"]);
    assert!(matches!(cli.command, rx::cli::Command::Insights));
}

#[test]
fn parse_explain() {
    let cli = parse(&["explain", "E0502"]);
    match cli.command {
        rx::cli::Command::Explain { code } => assert_eq!(code, "E0502"),
        _ => panic!("expected Explain"),
    }
}

#[test]
fn parse_manpage() {
    let cli = parse(&["manpage"]);
    assert!(matches!(cli.command, rx::cli::Command::Manpage { .. }));
}

#[test]
fn parse_sbom_spdx() {
    let cli = parse(&["sbom"]);
    match cli.command {
        rx::cli::Command::Sbom { format, output } => {
            assert_eq!(format, "spdx");
            assert!(output.is_none());
        }
        _ => panic!("expected Sbom"),
    }
}

#[test]
fn parse_sbom_cyclonedx() {
    let cli = parse(&["sbom", "--format", "cyclonedx", "--output", "bom.json"]);
    match cli.command {
        rx::cli::Command::Sbom { format, output } => {
            assert_eq!(format, "cyclonedx");
            assert_eq!(output.unwrap(), "bom.json");
        }
        _ => panic!("expected Sbom"),
    }
}

#[test]
fn parse_pkg_why() {
    let cli = parse(&["pkg", "why", "serde"]);
    match cli.command {
        rx::cli::Command::Pkg(rx::cli::PkgCommand::Why { name }) => {
            assert_eq!(name, "serde");
        }
        _ => panic!("expected Pkg Why"),
    }
}

#[test]
fn parse_pkg_dedupe() {
    let cli = parse(&["pkg", "dedupe"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Pkg(rx::cli::PkgCommand::Dedupe)
    ));
}

#[test]
fn parse_bench_save() {
    let cli = parse(&["bench", "--save", "baseline1"]);
    match cli.command {
        rx::cli::Command::Bench { save, .. } => assert_eq!(save.unwrap(), "baseline1"),
        _ => panic!("expected Bench"),
    }
}

#[test]
fn parse_bench_compare() {
    let cli = parse(&["bench", "--compare", "baseline1"]);
    match cli.command {
        rx::cli::Command::Bench { compare, .. } => assert_eq!(compare.unwrap(), "baseline1"),
        _ => panic!("expected Bench"),
    }
}

#[test]
fn parse_bench_list() {
    let cli = parse(&["bench", "--list"]);
    match cli.command {
        rx::cli::Command::Bench { list, .. } => assert!(list),
        _ => panic!("expected Bench"),
    }
}

#[test]
fn parse_test_advanced_snapshot() {
    let cli = parse(&["test-advanced", "snapshot"]);
    match cli.command {
        rx::cli::Command::TestAdvanced(rx::cli::TestAdvancedCommand::Snapshot { review }) => {
            assert!(!review);
        }
        _ => panic!("expected TestAdvanced Snapshot"),
    }
}

#[test]
fn parse_test_advanced_fuzz() {
    let cli = parse(&["test-advanced", "fuzz", "my_target"]);
    match cli.command {
        rx::cli::Command::TestAdvanced(rx::cli::TestAdvancedCommand::Fuzz { target, time }) => {
            assert_eq!(target, "my_target");
            assert_eq!(time, "60s");
        }
        _ => panic!("expected TestAdvanced Fuzz"),
    }
}

#[test]
fn parse_test_advanced_mutate() {
    let cli = parse(&["test-advanced", "mutate"]);
    match cli.command {
        rx::cli::Command::TestAdvanced(rx::cli::TestAdvancedCommand::Mutate { package }) => {
            assert!(package.is_none());
        }
        _ => panic!("expected TestAdvanced Mutate"),
    }
}

#[test]
fn parse_daemon_start() {
    let cli = parse(&["daemon", "start"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Daemon(rx::cli::DaemonCommand::Start { .. })
    ));
}

#[test]
fn parse_daemon_stop() {
    let cli = parse(&["daemon", "stop"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Daemon(rx::cli::DaemonCommand::Stop)
    ));
}

#[test]
fn parse_daemon_status() {
    let cli = parse(&["daemon", "status"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Daemon(rx::cli::DaemonCommand::Status)
    ));
}

#[test]
fn parse_daemon_ping() {
    let cli = parse(&["daemon", "ping"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Daemon(rx::cli::DaemonCommand::Ping)
    ));
}

#[test]
fn parse_compat() {
    let cli = parse(&["compat"]);
    assert!(matches!(cli.command, rx::cli::Command::Compat));
}

#[test]
fn parse_test_smart() {
    let cli = parse(&["test-smart"]);
    match cli.command {
        rx::cli::Command::TestSmart {
            filter,
            package,
            shards,
            release,
        } => {
            assert!(filter.is_none());
            assert!(package.is_none());
            assert!(shards.is_none());
            assert!(!release);
        }
        _ => panic!("expected TestSmart"),
    }
}

#[test]
fn parse_test_smart_with_shards() {
    let cli = parse(&["test-smart", "--shards", "4"]);
    match cli.command {
        rx::cli::Command::TestSmart { shards, .. } => {
            assert_eq!(shards, Some(4));
        }
        _ => panic!("expected TestSmart"),
    }
}

#[test]
fn parse_test_smart_with_filter() {
    let cli = parse(&["test-smart", "my_test", "--release"]);
    match cli.command {
        rx::cli::Command::TestSmart {
            filter, release, ..
        } => {
            assert_eq!(filter.unwrap(), "my_test");
            assert!(release);
        }
        _ => panic!("expected TestSmart"),
    }
}

#[test]
fn parse_sandbox() {
    let cli = parse(&["sandbox"]);
    match cli.command {
        rx::cli::Command::Sandbox { release } => assert!(!release),
        _ => panic!("expected Sandbox"),
    }
}

#[test]
fn parse_sandbox_release() {
    let cli = parse(&["sandbox", "--release"]);
    match cli.command {
        rx::cli::Command::Sandbox { release } => assert!(release),
        _ => panic!("expected Sandbox"),
    }
}

#[test]
fn parse_registry_login() {
    let cli = parse(&["registry", "login", "my-reg"]);
    match cli.command {
        rx::cli::Command::Registry(rx::cli::RegistryCommand::Login { registry, token }) => {
            assert_eq!(registry, "my-reg");
            assert!(token.is_none());
        }
        _ => panic!("expected Registry Login"),
    }
}

#[test]
fn parse_registry_login_with_token() {
    let cli = parse(&["registry", "login", "my-reg", "tok123"]);
    match cli.command {
        rx::cli::Command::Registry(rx::cli::RegistryCommand::Login { registry, token }) => {
            assert_eq!(registry, "my-reg");
            assert_eq!(token.unwrap(), "tok123");
        }
        _ => panic!("expected Registry Login"),
    }
}

#[test]
fn parse_registry_list() {
    let cli = parse(&["registry", "list"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Registry(rx::cli::RegistryCommand::List)
    ));
}

#[test]
fn parse_registry_add() {
    let cli = parse(&["registry", "add", "my-reg", "https://index.example.com"]);
    match cli.command {
        rx::cli::Command::Registry(rx::cli::RegistryCommand::Add { name, index }) => {
            assert_eq!(name, "my-reg");
            assert_eq!(index, "https://index.example.com");
        }
        _ => panic!("expected Registry Add"),
    }
}

#[test]
fn parse_lockfile_check() {
    let cli = parse(&["lockfile", "check"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Lockfile(rx::cli::LockfileCommand::Check)
    ));
}

#[test]
fn parse_lockfile_enforce() {
    let cli = parse(&["lockfile", "enforce"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Lockfile(rx::cli::LockfileCommand::Enforce)
    ));
}

#[test]
fn parse_telemetry_on() {
    let cli = parse(&["telemetry", "on"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Telemetry(rx::cli::TelemetryCommand::On)
    ));
}

#[test]
fn parse_telemetry_off() {
    let cli = parse(&["telemetry", "off"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Telemetry(rx::cli::TelemetryCommand::Off)
    ));
}

#[test]
fn parse_telemetry_status() {
    let cli = parse(&["telemetry", "status"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Telemetry(rx::cli::TelemetryCommand::Status)
    ));
}

#[test]
fn parse_worker_warm() {
    let cli = parse(&["worker", "warm"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Worker(rx::cli::WorkerCommand::Warm)
    ));
}

#[test]
fn parse_worker_status() {
    let cli = parse(&["worker", "status"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Worker(rx::cli::WorkerCommand::Status)
    ));
}

#[test]
fn parse_worker_stop() {
    let cli = parse(&["worker", "stop"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Worker(rx::cli::WorkerCommand::Stop)
    ));
}

#[test]
fn parse_pkg_compat() {
    let cli = parse(&["pkg", "compat"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Pkg(rx::cli::PkgCommand::Compat)
    ));
}
