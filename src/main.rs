mod affected;
mod audit;
mod bench;
mod bloat;
mod build;
mod cache;
mod cargo_output;
mod check;
mod ci;
mod ci_gen;
mod cli;
mod compat;
mod completions;
mod config;
mod coverage;
mod daemon;
mod deps;
mod doc;
mod doctor;
mod env;
mod expand;
mod fix;
mod fmt;
mod hints;
mod insights;
mod lint;
mod lockfile;
mod migrate;
mod outdated;
mod output;
mod pipeline;
mod pkg;
mod plugin;
mod publish;
mod registry;
mod release;
#[allow(dead_code)]
mod remote_cache;
#[allow(dead_code)]
mod sandbox;
mod sbom;
mod script;
mod selfupdate;
mod semantic_hash;
mod speculative;
mod size;
mod stats;
#[allow(dead_code)]
mod telemetry;
mod templates;
mod test;
mod test_advanced;
#[allow(dead_code)]
mod test_orchestrator;
mod toolchain;
mod tree;
mod upgrade;
mod watch;
#[allow(dead_code)]
mod worker;
mod workspace;

use clap::Parser;
use cli::Cli;

fn main() {
    // Install Ctrl+C handler for graceful shutdown
    ctrlc::set_handler(|| {
        output::error("interrupted");
        std::process::exit(130);
    })
    .ok();

    let cli = Cli::parse();
    if let Err(err) = cli::dispatch(cli) {
        output::error(&format!("{err:#}"));
        std::process::exit(1);
    }
}
