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

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::dispatch(cli)
}
