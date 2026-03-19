mod build;
mod cache;
mod cli;
mod config;
mod fmt;
mod lint;
mod pkg;
mod test;
mod toolchain;
mod watch;
mod workspace;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::dispatch(cli)
}
