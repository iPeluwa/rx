use anyhow::{Context, Result};
use std::process::Command;

use crate::output::Timer;

pub fn doc(open: bool, no_deps: bool, watch: bool) -> Result<()> {
    if watch {
        return doc_watch(no_deps);
    }

    let timer = Timer::start("doc");
    crate::output::info("building documentation...");

    let mut cmd = Command::new("cargo");
    cmd.arg("doc");

    if no_deps {
        cmd.arg("--no-deps");
    }

    if open {
        cmd.arg("--open");
    }

    let status = cmd.status().context(
        "failed to run cargo doc\n\
         hint: is cargo installed? run `rx doctor` to check",
    )?;
    if !status.success() {
        anyhow::bail!("documentation build failed");
    }

    if !open {
        crate::output::success("documentation built — run `rx doc --open` to view");
    }

    timer.finish();
    Ok(())
}

fn doc_watch(no_deps: bool) -> Result<()> {
    let has_watch = Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_watch {
        anyhow::bail!(
            "cargo-watch is not installed\n\
             hint: install it with: cargo install cargo-watch"
        );
    }

    crate::output::info("watching for changes and rebuilding docs...");

    let mut cmd = Command::new("cargo");
    cmd.args(["watch", "-x"]);

    if no_deps {
        cmd.arg("doc --no-deps");
    } else {
        cmd.arg("doc");
    }

    let status = cmd.status().context("failed to run cargo watch")?;
    if !status.success() {
        anyhow::bail!("doc watch failed");
    }
    Ok(())
}
