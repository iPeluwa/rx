mod affected;
mod build;
mod cache;
mod cargo_output;
mod check;
mod ci_gen;
mod cli;
mod completions;
mod config;
mod doctor;
mod fix;
mod fmt;
mod hints;
mod lint;
mod migrate;
mod output;
mod stats;
mod task;
mod test;
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
