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

    /// Run a script defined in rx.toml
    Script {
        /// Script name (omit to list all scripts)
        name: Option<String>,
    },

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
        Command::Graph => crate::workspace::dispatch(WsCommand::Graph),
        Command::Cache(cmd) => crate::cache::dispatch(cmd),
        Command::Ws(cmd) => crate::workspace::dispatch(cmd),
        Command::Clean { gc, all } => crate::cache::clean(gc, all),
        Command::Script { name } => match name {
            Some(n) => crate::script::run_script(&n, &config),
            None => crate::script::list_scripts(&config),
        },
        // Already handled above
        Command::Doctor
        | Command::Completions { .. }
        | Command::Stats(_)
        | Command::Init { .. } => unreachable!(),
    }
}
