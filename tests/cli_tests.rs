use clap::Parser;
use rx::cli::Cli;

fn parse(args: &[&str]) -> Cli {
    let mut full = vec!["rx"];
    full.extend(args);
    Cli::parse_from(full)
}

fn try_parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut full = vec!["rx"];
    full.extend(args);
    Cli::try_parse_from(full)
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
fn parse_test_with_filter() {
    let cli = parse(&["test", "my_test"]);
    match cli.command {
        rx::cli::Command::Test { filter, .. } => assert_eq!(filter.unwrap(), "my_test"),
        _ => panic!("expected Test"),
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
fn parse_graph() {
    let cli = parse(&["graph"]);
    assert!(matches!(cli.command, rx::cli::Command::Graph));
}

#[test]
fn parse_run_task() {
    let cli = parse(&["run", "ci"]);
    match cli.command {
        rx::cli::Command::Run { task, args } => {
            assert_eq!(task.unwrap(), "ci");
            assert!(args.is_empty());
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_run_lists_when_no_task() {
    let cli = parse(&["run"]);
    match cli.command {
        rx::cli::Command::Run { task, .. } => assert!(task.is_none()),
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_run_with_passthrough_args() {
    let cli = parse(&["run", "hello", "--", "--my-flag", "value"]);
    match cli.command {
        rx::cli::Command::Run { task, args } => {
            assert_eq!(task.unwrap(), "hello");
            assert_eq!(args, vec!["--my-flag", "value"]);
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_cache_status() {
    let cli = parse(&["cache", "status"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Cache(rx::cli::CacheCommand::Status)
    ));
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
fn parse_cache_purge() {
    let cli = parse(&["cache", "purge"]);
    assert!(matches!(
        cli.command,
        rx::cli::Command::Cache(rx::cli::CacheCommand::Purge)
    ));
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
fn parse_init_migrate() {
    let cli = parse(&["init", "--migrate"]);
    match cli.command {
        rx::cli::Command::Init { migrate, .. } => assert!(migrate),
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
fn parse_ws_exec() {
    let cli = parse(&["ws", "exec", "echo", "hi"]);
    match cli.command {
        rx::cli::Command::Ws(rx::cli::WsCommand::Exec { cmd }) => {
            assert_eq!(cmd, vec!["echo", "hi"]);
        }
        _ => panic!("expected Ws Exec"),
    }
}

#[test]
fn parse_doctor() {
    let cli = parse(&["doctor"]);
    assert!(matches!(cli.command, rx::cli::Command::Doctor));
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
fn parse_profile_flag() {
    let cli = parse(&["--profile", "ci", "build"]);
    assert_eq!(cli.profile.unwrap(), "ci");
}

#[test]
fn removed_commands_are_rejected() {
    // Out-of-scope commands deleted in the 0.2 scope reset must not parse.
    for cmd in [
        "new",
        "bench",
        "expand",
        "publish",
        "pkg",
        "toolchain",
        "watch",
        "upgrade",
        "size",
        "tree",
        "outdated",
        "audit",
        "self-update",
        "coverage",
        "deps",
        "bloat",
        "doc",
        "release",
        "env",
        "plugin",
        "registry",
        "lockfile",
        "sbom",
        "telemetry",
        "insights",
        "explain",
        "manpage",
        "test-advanced",
        "test-smart",
        "daemon",
        "worker",
        "compat",
        "sandbox",
        "script",
    ] {
        assert!(
            try_parse(&[cmd]).is_err(),
            "removed command `{cmd}` still parses"
        );
    }
}

#[test]
fn removed_cache_subcommands_are_rejected() {
    assert!(try_parse(&["cache", "export"]).is_err());
    assert!(try_parse(&["cache", "import", "x.tar.gz"]).is_err());
}

#[test]
fn removed_ws_subcommands_are_rejected() {
    assert!(try_parse(&["ws", "cache-push"]).is_err());
    assert!(try_parse(&["ws", "cache-pull"]).is_err());
    assert!(try_parse(&["ws", "script", "ci"]).is_err());
}
