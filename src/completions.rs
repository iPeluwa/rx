use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn generate_completions(shell: Shell) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    generate(shell, &mut cmd, "rx", &mut io::stdout());
    Ok(())
}

#[allow(dead_code)]
pub fn generate_manpage() -> Result<()> {
    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}
