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
mod migrate;
mod outdated;
mod output;
mod pipeline;
mod pkg;
mod plugin;
mod publish;
mod release;
mod sbom;
mod script;
mod selfupdate;
mod semantic_hash;
mod size;
mod stats;
mod templates;
mod test;
mod test_advanced;
mod toolchain;
mod tree;
mod upgrade;
mod watch;
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
