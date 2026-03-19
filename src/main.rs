mod bench;
mod build;
mod cache;
mod cli;
mod completions;
mod config;
mod doctor;
mod expand;
mod fmt;
mod lint;
mod output;
mod pkg;
mod publish;
mod test;
mod toolchain;
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
