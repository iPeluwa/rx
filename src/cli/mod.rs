use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "rx", version, about = "A fast, unified Rust toolchain manager")]
pub struct Cli {
    /// Suppress non-error output
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Show verbose output
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Config profile to use (e.g. --profile ci)
    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize rx.toml in the current project
    Init {
        /// Generate CI config (github, gitlab, circle — default: github)
        #[arg(long, default_missing_value = "github", num_args = 0..=1)]
        ci: Option<String>,
        /// Auto-detect project settings from existing Cargo.toml and tools
        #[arg(long)]
        migrate: bool,
    },

    /// Show the resolved rx configuration
    Config,

    /// Create a new Rust project
    New {
        /// Project name
        name: String,
        /// Use library template instead of binary
        #[arg(long)]
        lib: bool,
        /// Project template: axum, cli, wasm, lib
        #[arg(long, short)]
        template: Option<String>,
    },

    /// Build the project (with fast linker + caching)
    Build {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
        /// Package to build (in a workspace)
        #[arg(long, short)]
        package: Option<String>,
        /// Cross-compile for a target triple (e.g. x86_64-unknown-linux-gnu)
        #[arg(long)]
        target: Option<String>,
    },

    /// Build and run the project
    Run {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run tests (using nextest when available)
    Test {
        /// Test name filter
        filter: Option<String>,
        /// Package to test (in a workspace)
        #[arg(long, short)]
        package: Option<String>,
        /// Run tests in release mode
        #[arg(long, short)]
        release: bool,
        /// Only test packages affected by changes since base ref
        #[arg(long)]
        affected: bool,
        /// Base ref for --affected (default: HEAD~1)
        #[arg(long, default_value = "HEAD~1")]
        base: String,
    },

    /// Format code (rustfmt)
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Lint code (clippy)
    Lint {
        /// Automatically apply suggestions
        #[arg(long)]
        fix: bool,
    },

    /// Run benchmarks
    Bench {
        /// Benchmark filter
        filter: Option<String>,
        /// Package to benchmark (in a workspace)
        #[arg(long, short)]
        package: Option<String>,
        /// Save results as a named baseline
        #[arg(long)]
        save: Option<String>,
        /// Compare results against a saved baseline
        #[arg(long)]
        compare: Option<String>,
        /// List saved baselines
        #[arg(long)]
        list: bool,
    },

    /// Expand macros (requires cargo-expand)
    Expand {
        /// Item path to expand (e.g. module::function)
        item: Option<String>,
    },

    /// Publish crate(s) to crates.io
    Publish {
        /// Package to publish (publishes entire workspace in order if omitted)
        #[arg(long, short)]
        package: Option<String>,
        /// Perform a dry run without actually publishing
        #[arg(long)]
        dry_run: bool,
    },

    /// Manage dependencies
    #[command(subcommand)]
    Pkg(PkgCommand),

    /// Manage Rust toolchains
    #[command(subcommand)]
    Toolchain(ToolchainCommand),

    /// Manage the global artifact cache
    #[command(subcommand)]
    Cache(CacheCommand),

    /// Workspace orchestration commands
    #[command(subcommand)]
    Ws(WsCommand),

    /// Watch for changes and rebuild
    Watch {
        /// Command to run on changes (default: build)
        #[arg(long, short)]
        cmd: Option<String>,
    },

    /// Clean build artifacts
    Clean {
        /// Also garbage-collect the global cache
        #[arg(long)]
        gc: bool,
        /// Clean all workspace member target directories
        #[arg(long)]
        all: bool,
    },

    /// Check your development environment
    Doctor,

    /// Update toolchains and dependencies
    Upgrade,

    /// Type-check the project without building
    Check {
        /// Package to check (in a workspace)
        #[arg(long, short)]
        package: Option<String>,
    },

    /// Auto-fix lint warnings, compiler suggestions, and formatting
    Fix,

    /// Run the full CI pipeline locally (fmt, clippy, test, build)
    Ci,

    /// Show binary size (and cargo-bloat breakdown if available)
    Size {
        /// Measure release binary
        #[arg(long, short)]
        release: bool,
    },

    /// Show the dependency tree
    Tree {
        /// Show only duplicate dependencies
        #[arg(long, short)]
        duplicates: bool,
        /// Limit tree depth
        #[arg(long)]
        depth: Option<u32>,
    },

    /// Check for outdated dependencies
    Outdated,

    /// Audit dependencies for known security vulnerabilities
    Audit,

    /// Update rx to the latest version
    #[command(name = "self-update")]
    SelfUpdate,

    /// Run a script defined in rx.toml
    Script {
        /// Script name (omit to list all scripts)
        name: Option<String>,
    },

    /// Generate code coverage report
    Coverage {
        /// Open the report in a browser
        #[arg(long)]
        open: bool,
        /// Output LCOV format (for CI integrations)
        #[arg(long)]
        lcov: bool,
    },

    /// Dependency health dashboard (tree + outdated + audit)
    Deps,

    /// Analyze binary bloat by function or crate
    Bloat {
        /// Analyze release binary
        #[arg(long, short)]
        release: bool,
        /// Group by crate instead of function
        #[arg(long)]
        crates: bool,
    },

    /// Build documentation
    Doc {
        /// Open docs in browser after building
        #[arg(long)]
        open: bool,
        /// Skip building docs for dependencies
        #[arg(long)]
        no_deps: bool,
        /// Watch for changes and rebuild
        #[arg(long, short)]
        watch: bool,
    },

    /// Bump version, commit, tag, and push a release
    Release {
        /// Version or bump keyword: patch, minor, major, or explicit (e.g. 1.2.3)
        version: String,
        /// Show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
        /// Create commit and tag but don't push
        #[arg(long)]
        no_push: bool,
    },

    /// Manage environment variables from rx.toml
    #[command(subcommand)]
    Env(EnvCommand),

    /// Manage or run plugins (~/.rx/plugins/)
    #[command(subcommand)]
    Plugin(PluginCommand),

    /// Show build time statistics and trends
    #[command(subcommand)]
    Stats(StatsCommand),

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
        /// Install completions to the appropriate system location
        #[arg(long)]
        install: bool,
    },

    /// Analyze project health, build times, and dependency status
    Insights,

    /// Explain a Rust error code with practical fixes
    Explain {
        /// Error code (e.g. E0502 or 0502)
        code: String,
    },

    /// Generate a man page for rx
    Manpage {
        /// Install man page to the appropriate system location
        #[arg(long)]
        install: bool,
    },

    /// Generate a Software Bill of Materials (SBOM)
    Sbom {
        /// Output format: spdx or cyclonedx
        #[arg(long, default_value = "spdx")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Run tests with advanced strategies (snapshot, fuzz, mutate)
    #[command(name = "test-advanced", subcommand)]
    TestAdvanced(TestAdvancedCommand),

    /// Manage the rxd background daemon
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Check MSRV compatibility of dependencies
    Compat,

    /// Run tests with smart ordering and optional sharding
    #[command(name = "test-smart")]
    TestSmart {
        /// Test name filter
        filter: Option<String>,
        /// Package to test
        #[arg(long, short)]
        package: Option<String>,
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
        /// Number of parallel shards
        #[arg(long)]
        shards: Option<u32>,
    },

    /// Run a sandboxed build to detect undeclared dependencies
    Sandbox {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,
    },

    /// Manage private crate registries
    #[command(subcommand)]
    Registry(RegistryCommand),

    /// Check lockfile health and enforce policies
    #[command(subcommand)]
    Lockfile(LockfileCommand),

    /// Manage anonymous usage telemetry
    #[command(subcommand)]
    Telemetry(TelemetryCommand),

    /// Manage persistent background worker processes
    #[command(subcommand)]
    Worker(WorkerCommand),
}

#[derive(Subcommand)]
pub enum RegistryCommand {
    /// Authenticate with a private registry
    Login {
        /// Registry name
        registry: String,
        /// Token (omit for interactive login)
        token: Option<String>,
    },
    /// List configured registries
    List,
    /// Add a new registry
    Add {
        /// Registry name
        name: String,
        /// Registry index URL
        index: String,
    },
}

#[derive(Subcommand)]
pub enum LockfileCommand {
    /// Check lockfile health (existence, freshness, git status)
    Check,
    /// Enforce lockfile is in sync (for CI: fails if out of date)
    Enforce,
}

#[derive(Subcommand)]
pub enum TelemetryCommand {
    /// Enable anonymous telemetry
    On,
    /// Disable telemetry
    Off,
    /// Show telemetry status and collected data
    Status,
    /// Export telemetry data in various formats (json, csv, markdown)
    Export {
        /// Export format: json, csv, markdown/md
        #[arg(default_value = "json")]
        format: String,
    },
    /// Print a human-readable usage report
    Report,
    /// Clear all telemetry data (preserves enabled/disabled state)
    Reset,
}

#[derive(Subcommand)]
pub enum WorkerCommand {
    /// Pre-warm background workers (check, fmt, lint)
    Warm,
    /// Show status of active workers
    Status,
    /// Stop all workers
    Stop,
}

#[derive(Subcommand)]
pub enum DaemonCommand {
    /// Start the daemon
    Start {
        /// Run in the foreground (default: daemonize to background)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop a running daemon
    Stop,
    /// Show daemon status
    Status,
    /// Ping the daemon
    Ping,
}

#[derive(Subcommand)]
pub enum PkgCommand {
    /// Add a dependency
    Add {
        /// Crate name (optionally with version: `serde@1.0`)
        spec: String,
        /// Add as dev dependency
        #[arg(long, short = 'D')]
        dev: bool,
        /// Add as build dependency
        #[arg(long, short = 'B')]
        build: bool,
    },
    /// Remove a dependency
    Remove {
        /// Crate name
        name: String,
    },
    /// Upgrade dependencies
    Upgrade {
        /// Specific crate to upgrade (upgrades all if omitted)
        name: Option<String>,
    },
    /// List dependencies and their versions
    List,
    /// Show why a dependency is in your tree (reverse dependency trace)
    Why {
        /// Crate name to trace
        name: String,
    },
    /// Find and show duplicate dependency versions
    Dedupe,
    /// Check MSRV compatibility of dependencies
    Compat,
}

#[derive(Subcommand)]
pub enum TestAdvancedCommand {
    /// Update snapshot tests (requires cargo-insta)
    Snapshot {
        /// Review snapshots interactively
        #[arg(long)]
        review: bool,
    },
    /// Run fuzz tests (requires cargo-fuzz)
    Fuzz {
        /// Fuzz target name
        target: String,
        /// Duration to run (e.g. 60s, 5m)
        #[arg(long, default_value = "60s")]
        time: String,
    },
    /// Run mutation tests (requires cargo-mutants)
    Mutate {
        /// Package to test
        #[arg(long, short)]
        package: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ToolchainCommand {
    /// Install a Rust toolchain
    Install {
        /// Toolchain specifier (e.g. stable, nightly, 1.75.0)
        version: String,
    },
    /// Set the default toolchain
    Use {
        /// Toolchain specifier
        version: String,
    },
    /// List installed toolchains
    List,
    /// Update installed toolchains
    Update,
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache status and disk usage
    Status,
    /// Garbage-collect stale artifacts
    Gc {
        /// Remove artifacts older than this many days (default: 30)
        #[arg(long, default_value = "30")]
        older_than: u32,
    },
    /// Purge the entire cache
    Purge,
}

#[derive(Subcommand)]
pub enum WsCommand {
    /// List all workspace members
    List,
    /// Show the dependency graph between workspace members
    Graph,
    /// Run a cargo command across all workspace members in dependency order
    Run {
        /// The cargo command to run (e.g. build, test, clippy)
        cmd: String,
        /// Extra arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run a custom script defined in rx.toml across workspace members
    Script {
        /// Script name
        name: String,
        /// Only run on specific packages (comma-separated)
        #[arg(long, short, value_delimiter = ',')]
        packages: Vec<String>,
    },
    /// Execute a shell command in each workspace member directory
    Exec {
        /// The shell command to execute
        #[arg(trailing_var_arg = true, required = true)]
        cmd: Vec<String>,
    },
    /// Push workspace member caches to remote
    CachePush,
    /// Pull workspace member caches from remote
    CachePull,
}

#[derive(Subcommand)]
pub enum EnvCommand {
    /// Show resolved environment variables from rx.toml and .env
    Show,
    /// Spawn a shell with rx.toml and .env vars loaded
    Shell,
    /// Check .env against .env.example for missing variables
    Check,
    /// Add a variable to .env.example
    AddExample {
        /// Variable name
        name: String,
        /// Optional description comment
        #[arg(long)]
        description: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// List available plugins
    List,
    /// Run a plugin by name
    Run {
        /// Plugin name
        name: String,
        /// Arguments to pass to the plugin
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum StatsCommand {
    /// Show build time statistics
    Show,
    /// Clear all recorded stats
    Clear,
}

/// Load config, apply profile overrides, and set env vars.
fn load_config(profile: Option<&str>) -> Result<crate::config::RxConfig> {
    let mut config = crate::config::load()?;

    // Apply profile overrides if specified
    if let Some(profile_name) = profile {
        crate::config::apply_profile(&mut config, profile_name)?;
    }

    // SAFETY: rx is single-threaded at this point (before any thread spawning)
    for (key, value) in &config.env {
        unsafe { std::env::set_var(key, value) };
    }
    Ok(config)
}

pub fn dispatch(cli: Cli) -> Result<()> {
    // Set output verbosity
    crate::output::set_quiet(cli.quiet);
    crate::output::set_verbose(cli.verbose);

    // Commands that don't need config
    match &cli.command {
        Command::Doctor => return crate::doctor::doctor(),
        Command::SelfUpdate => return crate::selfupdate::self_update(),
        Command::Completions { shell, install } => {
            return if *install {
                crate::completions::install_completions(*shell)
            } else {
                crate::completions::generate_completions(*shell)
            };
        }
        Command::Outdated => return crate::outdated::outdated(),
        Command::Audit => return crate::audit::audit(),
        Command::Tree { duplicates, depth } => return crate::tree::tree(*duplicates, *depth),
        Command::Size { release } => return crate::size::size(*release),
        Command::Deps => return crate::deps::deps(),
        Command::Bloat { release, crates } => return crate::bloat::bloat(*release, *crates),
        Command::Coverage { open, lcov } => return crate::coverage::coverage(*open, *lcov),
        Command::Doc {
            open,
            no_deps,
            watch,
        } => {
            return crate::doc::doc(*open, *no_deps, *watch);
        }
        Command::Stats(cmd) => {
            return match cmd {
                StatsCommand::Show => crate::stats::show(),
                StatsCommand::Clear => crate::stats::clear(),
            };
        }
        Command::Plugin(cmd) => {
            return match cmd {
                PluginCommand::List => crate::plugin::list_plugins(),
                PluginCommand::Run { name, args } => crate::plugin::run_plugin(name, args),
            };
        }
        Command::New {
            name,
            lib,
            template,
        } => {
            return if let Some(tmpl) = template {
                crate::templates::new_from_template(name, tmpl)
            } else {
                crate::workspace::new_project(name, *lib)
            };
        }
        Command::Init { ci, migrate } => {
            if *migrate {
                return crate::migrate::migrate();
            }
            let path = std::env::current_dir()?.join("rx.toml");
            if path.exists() {
                anyhow::bail!("rx.toml already exists");
            }
            crate::config::init_config(&path)?;
            crate::output::success("created rx.toml");
            if let Some(provider) = ci {
                crate::ci_gen::generate_ci(provider)?;
            }
            return Ok(());
        }
        Command::Release {
            version,
            dry_run,
            no_push,
        } => return crate::release::release(version, *dry_run, *no_push),
        Command::Insights => return crate::insights::insights(),
        Command::Explain { code } => return crate::hints::explain(code),
        Command::Manpage { install } => {
            return if *install {
                crate::completions::install_manpage()
            } else {
                crate::completions::generate_manpage()
            };
        }
        Command::Sbom { format, output } => {
            return crate::sbom::generate_sbom(format, output.as_deref());
        }
        Command::TestAdvanced(cmd) => {
            return crate::test_advanced::dispatch(cmd);
        }
        Command::Daemon(cmd) => {
            return match cmd {
                DaemonCommand::Start { foreground } => crate::daemon::start(*foreground),
                DaemonCommand::Stop => crate::daemon::stop(),
                DaemonCommand::Status => crate::daemon::status(),
                DaemonCommand::Ping => {
                    let request = crate::daemon::DaemonRequest {
                        command: "ping".into(),
                        args: vec![],
                        cwd: std::env::current_dir()?.to_string_lossy().to_string(),
                    };
                    let response = crate::daemon::send_request(&request)?;
                    if response.success {
                        crate::output::success(&response.output);
                    } else {
                        anyhow::bail!("{}", response.output);
                    }
                    Ok(())
                }
            };
        }
        Command::Compat => return crate::compat::check_compat(),
        Command::TestSmart {
            filter,
            package,
            release,
            shards,
        } => {
            return crate::test_orchestrator::run_orchestrated(
                filter.as_deref(),
                package.as_deref(),
                *release,
                *shards,
            );
        }
        Command::Sandbox { release } => {
            return crate::sandbox::sandboxed_build(*release);
        }
        Command::Registry(cmd) => {
            return match cmd {
                RegistryCommand::Login { registry, token } => {
                    crate::registry::login(registry, token.as_deref())
                }
                RegistryCommand::List => crate::registry::list_registries(),
                RegistryCommand::Add { name, index } => crate::registry::add_registry(name, index),
            };
        }
        Command::Lockfile(cmd) => {
            return match cmd {
                LockfileCommand::Check => crate::lockfile::check(),
                LockfileCommand::Enforce => crate::lockfile::enforce(),
            };
        }
        Command::Telemetry(cmd) => {
            return match cmd {
                TelemetryCommand::On => crate::telemetry::enable(),
                TelemetryCommand::Off => crate::telemetry::disable(),
                TelemetryCommand::Status => crate::telemetry::status(),
                TelemetryCommand::Export { format } => crate::telemetry::export(format),
                TelemetryCommand::Report => crate::telemetry::report(),
                TelemetryCommand::Reset => crate::telemetry::reset(),
            };
        }
        Command::Worker(cmd) => {
            return match cmd {
                WorkerCommand::Warm => crate::worker::warm(),
                WorkerCommand::Status => crate::worker::status(),
                WorkerCommand::Stop => crate::worker::stop_all(),
            };
        }
        _ => {}
    }

    // Commands that need config
    let config = load_config(cli.profile.as_deref())?;

    match cli.command {
        Command::Config => crate::config::show(&config),
        Command::Build {
            release,
            package,
            target,
        } => crate::build::build(release, package.as_deref(), target.as_deref(), &config),
        Command::Run { release, args } => crate::build::run(release, &args, &config),
        Command::Test {
            filter,
            package,
            release,
            affected,
            base,
        } => {
            if affected {
                let packages = crate::affected::affected_packages(&base)?;
                if packages.is_empty() {
                    crate::output::success("no affected packages — skipping tests");
                    return Ok(());
                }
                // If it's a single root package, just run tests normally
                if packages.len() == 1 && packages[0] == "(root)" {
                    return crate::test::test(filter.as_deref(), None, release, &config);
                }
                // Run tests for each affected package
                for pkg in &packages {
                    crate::test::test(filter.as_deref(), Some(pkg), release, &config)?;
                }
                Ok(())
            } else {
                crate::test::test(filter.as_deref(), package.as_deref(), release, &config)
            }
        }
        Command::Fmt { check } => crate::fmt::fmt(check, &config),
        Command::Lint { fix } => crate::lint::lint(fix, &config),
        Command::Check { package } => crate::check::check(package.as_deref(), &config),
        Command::Fix => crate::fix::fix(&config),
        Command::Ci => crate::ci::ci(&config),
        Command::Bench {
            filter,
            package,
            save,
            compare,
            list,
        } => {
            if list {
                return crate::bench::bench_list();
            }
            if let Some(baseline) = compare {
                return crate::bench::bench_compare(&baseline);
            }
            if let Some(name) = save {
                return crate::bench::bench_save(&name);
            }
            crate::bench::bench(filter.as_deref(), package.as_deref())
        }
        Command::Expand { item } => crate::expand::expand(item.as_deref()),
        Command::Publish { package, dry_run } => {
            crate::publish::publish(package.as_deref(), dry_run)
        }
        Command::Pkg(cmd) => crate::pkg::dispatch(cmd),
        Command::Toolchain(cmd) => crate::toolchain::dispatch(cmd),
        Command::Cache(cmd) => crate::cache::dispatch(cmd),
        Command::Ws(cmd) => crate::workspace::dispatch(cmd),
        Command::Watch { cmd } => crate::watch::watch(cmd.as_deref(), &config),
        Command::Clean { gc, all } => crate::cache::clean(gc, all),
        Command::Upgrade => crate::upgrade::upgrade(),
        Command::Script { name } => match name {
            Some(n) => crate::script::run_script(&n, &config),
            None => crate::script::list_scripts(&config),
        },
        Command::Env(cmd) => match cmd {
            EnvCommand::Show => crate::env::show_env(&config),
            EnvCommand::Shell => crate::env::shell(&config),
            EnvCommand::Check => crate::env::check_env(&config),
            EnvCommand::AddExample { name, description } => {
                crate::env::add_example(&name, description.as_deref())
            }
        },
        // Already handled above
        Command::Doctor
        | Command::SelfUpdate
        | Command::Completions { .. }
        | Command::Outdated
        | Command::Audit
        | Command::Tree { .. }
        | Command::Size { .. }
        | Command::New { .. }
        | Command::Deps
        | Command::Bloat { .. }
        | Command::Coverage { .. }
        | Command::Doc { .. }
        | Command::Stats(_)
        | Command::Plugin(_)
        | Command::Init { .. }
        | Command::Release { .. }
        | Command::Insights
        | Command::Explain { .. }
        | Command::Manpage { .. }
        | Command::Sbom { .. }
        | Command::TestAdvanced(_)
        | Command::Daemon(_)
        | Command::Compat
        | Command::TestSmart { .. }
        | Command::Sandbox { .. }
        | Command::Registry(_)
        | Command::Lockfile(_)
        | Command::Telemetry(_)
        | Command::Worker(_) => unreachable!(),
    }
}
