use anyhow::{Context, Result};
use std::process::Command;

use crate::output::Timer;

pub fn coverage(open: bool, lcov: bool) -> Result<()> {
    let timer = Timer::start("coverage");

    // Prefer cargo-llvm-cov, fall back to cargo-tarpaulin
    let has_llvm_cov = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let has_tarpaulin = Command::new("cargo")
        .args(["tarpaulin", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_llvm_cov {
        crate::output::info("generating coverage report with cargo-llvm-cov...");
        let mut cmd = Command::new("cargo");

        if lcov {
            cmd.args(["llvm-cov", "--lcov", "--output-path", "lcov.info"]);
            let status = cmd.status().context("failed to run cargo llvm-cov")?;
            if !status.success() {
                anyhow::bail!("coverage generation failed");
            }
            crate::output::success("LCOV report written to lcov.info");
        } else {
            cmd.args(["llvm-cov", "--html"]);
            if open {
                cmd.arg("--open");
            }
            let status = cmd.status().context("failed to run cargo llvm-cov")?;
            if !status.success() {
                anyhow::bail!("coverage generation failed");
            }
        }
    } else if has_tarpaulin {
        crate::output::info("generating coverage report with cargo-tarpaulin...");
        let mut cmd = Command::new("cargo");

        if lcov {
            cmd.args(["tarpaulin", "--out", "lcov"]);
        } else {
            cmd.args(["tarpaulin", "--out", "html"]);
        }

        let status = cmd.status().context("failed to run cargo tarpaulin")?;
        if !status.success() {
            anyhow::bail!("coverage generation failed");
        }

        if lcov {
            crate::output::success("LCOV report generated");
        } else if open {
            let _ = Command::new("open").arg("tarpaulin-report.html").status();
        }
    } else {
        anyhow::bail!(
            "no coverage tool found\n\
             hint: install one with:\n  \
             cargo install cargo-llvm-cov   (recommended)\n  \
             cargo install cargo-tarpaulin"
        );
    }

    timer.finish();
    Ok(())
}
