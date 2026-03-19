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

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize rx.toml in the current project
    Init,

    /// Show the resolved rx configuration
    Config,

    /// Create a new Rust project
    New {
        /// Project name
        name: String,
        /// Use library template instead of binary
        #[arg(long)]
        lib: bool,
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
    },

    /// Check your development environment
    Doctor,

    /// Update toolchains and dependencies
    Upgrade,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
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
}

pub fn dispatch(cli: Cli) -> Result<()> {
    // Set output verbosity
    crate::output::set_quiet(cli.quiet);
    crate::output::set_verbose(cli.verbose);

    let config = crate::config::load()?;

    // Apply env vars from config
    // SAFETY: rx is single-threaded at this point (before any thread spawning)
    for (key, value) in &config.env {
        unsafe { std::env::set_var(key, value) };
    }

    match cli.command {
        Command::Init => {
            let path = std::env::current_dir()?.join("rx.toml");
            if path.exists() {
                anyhow::bail!("rx.toml already exists");
            }
            crate::config::init_config(&path)?;
            crate::output::success("created rx.toml");
            Ok(())
        }
        Command::Config => crate::config::show(&config),
        Command::New { name, lib } => crate::workspace::new_project(&name, lib),
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
        } => crate::test::test(filter.as_deref(), package.as_deref(), release, &config),
        Command::Fmt { check } => crate::fmt::fmt(check, &config),
        Command::Lint { fix } => crate::lint::lint(fix, &config),
        Command::Bench { filter, package } => {
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
        Command::Clean { gc } => crate::cache::clean(gc),
        Command::Doctor => crate::doctor::doctor(),
        Command::Upgrade => crate::upgrade::upgrade(),
        Command::Completions { shell } => crate::completions::generate_completions(shell),
    }
}
