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
        rx::cli::Command::Clean { gc } => assert!(gc),
        _ => panic!("expected Clean"),
    }
}

#[test]
fn parse_new_lib() {
    let cli = parse(&["new", "mylib", "--lib"]);
    match cli.command {
        rx::cli::Command::New { name, lib } => {
            assert_eq!(name, "mylib");
            assert!(lib);
        }
        _ => panic!("expected New"),
    }
}

#[test]
fn parse_init() {
    let cli = parse(&["init"]);
    assert!(matches!(cli.command, rx::cli::Command::Init));
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
        rx::cli::Command::Bench { filter, package } => {
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
        rx::cli::Command::Completions { shell } => {
            assert_eq!(shell, clap_complete::Shell::Bash);
        }
        _ => panic!("expected Completions"),
    }
}
