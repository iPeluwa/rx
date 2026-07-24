use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "rx",
    version,
    about = "Fast local CI and task runner for Rust workspaces"
)]
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

    /// Build the project (with fast linker)
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

    /// Run a task from rx.toml (or a built-in task); lists tasks if omitted
    Run {
        /// Task name (built-ins: fmt, lint, test, build, check, ci)
        task: Option<String>,
        /// Only run against packages affected by changes since base ref
        #[arg(long)]
        affected: bool,
        /// Base ref for --affected (default: HEAD~1)
        #[arg(long, default_value = "HEAD~1")]
        base: String,
        /// Extra arguments appended to the task's shell command
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

    /// Type-check the project without building
    Check {
        /// Package to check (in a workspace)
        #[arg(long, short)]
        package: Option<String>,
    },

    /// Auto-fix lint warnings, compiler suggestions, and formatting
    Fix,

    /// Run the ci task: your [tasks.ci] pipeline, or fmt+lint+test+build
    Ci {
        /// Only run against packages affected by changes since base ref
        #[arg(long)]
        affected: bool,
        /// Base ref for --affected (default: HEAD~1)
        #[arg(long, default_value = "HEAD~1")]
        base: String,
    },

    /// Show the dependency graph between workspace members
    Graph,

    /// Manage the global artifact cache
    #[command(subcommand)]
    Cache(CacheCommand),

    /// Workspace orchestration commands
    #[command(subcommand)]
    Ws(WsCommand),

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
    /// Execute a shell command in each workspace member directory
    Exec {
        /// The shell command to execute
        #[arg(trailing_var_arg = true, required = true)]
        cmd: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum StatsCommand {
    /// Show build time statistics
    Show,
    /// Clear all recorded stats
    Clear,
}

/// What an --affected resolution means for package selection.
enum Selection {
    /// No filtering — run against the whole project/workspace.
    Everything,
    /// Run against exactly these packages (repeated -p).
    Packages(Vec<String>),
    /// Nothing relevant changed — skip the work entirely.
    Nothing,
}

/// Resolve --affected once; callers translate the selection into a single
/// Cargo invocation.
fn resolve_affected(affected: bool, base: &str) -> Result<Selection> {
    if !affected {
        return Ok(Selection::Everything);
    }
    let packages = crate::affected::affected_packages(base)?;
    if packages.is_empty() {
        return Ok(Selection::Nothing);
    }
    Ok(match crate::affected::to_package_selection(packages) {
        None => Selection::Everything,
        Some(pkgs) => Selection::Packages(pkgs),
    })
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
        Command::Completions { shell, install } => {
            return if *install {
                crate::completions::install_completions(*shell)
            } else {
                crate::completions::generate_completions(*shell)
            };
        }
        Command::Stats(cmd) => {
            return match cmd {
                StatsCommand::Show => crate::stats::show(),
                StatsCommand::Clear => crate::stats::clear(),
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
        } => {
            let packages: Vec<String> = package.into_iter().collect();
            crate::build::build(release, &packages, target.as_deref(), &config)
        }
        Command::Run {
            task,
            affected,
            base,
            args,
        } => match task {
            Some(name) => {
                let packages = match resolve_affected(affected, &base)? {
                    Selection::Everything => vec![],
                    Selection::Packages(pkgs) => pkgs,
                    Selection::Nothing => {
                        crate::output::success("no affected packages — nothing to run");
                        return Ok(());
                    }
                };
                crate::task::runner::run(&name, &args, &packages, &config)
            }
            None => crate::task::runner::list(&config),
        },
        Command::Test {
            filter,
            package,
            release,
            affected,
            base,
        } => {
            // One test invocation with repeated -p, never one per package.
            let packages = if affected {
                match resolve_affected(true, &base)? {
                    Selection::Everything => vec![],
                    Selection::Packages(pkgs) => pkgs,
                    Selection::Nothing => {
                        crate::output::success("no affected packages — skipping tests");
                        return Ok(());
                    }
                }
            } else {
                package.into_iter().collect()
            };
            crate::test::test(filter.as_deref(), &packages, release, &config)
        }
        Command::Fmt { check } => crate::fmt::fmt(check, &[], &config),
        Command::Lint { fix } => crate::lint::lint(fix, &[], &config),
        Command::Check { package } => {
            let packages: Vec<String> = package.into_iter().collect();
            crate::check::check(&packages, &config)
        }
        Command::Fix => crate::fix::fix(&config),
        Command::Ci { affected, base } => {
            let packages = match resolve_affected(affected, &base)? {
                Selection::Everything => vec![],
                Selection::Packages(pkgs) => pkgs,
                Selection::Nothing => {
                    crate::output::success("no affected packages — ci has nothing to check");
                    return Ok(());
                }
            };
            crate::task::runner::run("ci", &[], &packages, &config)
        }
        Command::Graph => crate::workspace::dispatch(WsCommand::Graph),
        Command::Cache(cmd) => crate::cache::dispatch(cmd),
        Command::Ws(cmd) => crate::workspace::dispatch(cmd),
        Command::Clean { gc, all } => crate::cache::clean(gc, all),
        // Already handled above
        Command::Doctor
        | Command::Completions { .. }
        | Command::Stats(_)
        | Command::Init { .. } => unreachable!(),
    }
}
