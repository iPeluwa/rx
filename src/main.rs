mod affected;
mod audit;
mod bench;
mod bloat;
mod build;
mod cache;
mod cargo_output;
mod check;
mod ci;
mod cli;
mod completions;
mod config;
mod coverage;
mod deps;
mod doc;
mod doctor;
mod env;
mod expand;
mod fix;
mod fmt;
mod lint;
mod migrate;
mod outdated;
mod output;
mod pkg;
mod plugin;
mod publish;
mod release;
mod script;
mod selfupdate;
mod size;
mod stats;
mod templates;
mod test;
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
