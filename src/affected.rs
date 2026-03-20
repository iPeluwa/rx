use anyhow::{Context, Result};
use std::collections::HashSet;
use std::process::Command;

/// Get the list of files changed since a base ref (default: HEAD~1).
fn changed_files(base: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", base])
        .output()
        .context("failed to run git diff — is this a git repository?")?;

    if !output.status.success() {
        // Try against the base as a branch
        let output = Command::new("git")
            .args(["diff", "--name-only", &format!("{base}...HEAD")])
            .output()
            .context("failed to run git diff")?;

        if !output.status.success() {
            anyhow::bail!(
                "could not determine changed files against `{base}`\n\
                 hint: make sure `{base}` is a valid git ref"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        return Ok(stdout.lines().map(|s| s.to_string()).collect());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

/// Determine which workspace packages are affected by changed files.
pub fn affected_packages(base: &str) -> Result<Vec<String>> {
    let files = changed_files(base)?;

    if files.is_empty() {
        crate::output::info("no files changed");
        return Ok(vec![]);
    }

    crate::output::verbose(&format!("{} files changed since {base}", files.len()));

    // Try to resolve workspace
    match crate::workspace::resolve_workspace() {
        Ok(graph) => {
            let mut affected = HashSet::new();

            for file in &files {
                for member in &graph.members {
                    let rel = member
                        .path
                        .strip_prefix(&graph.root)
                        .unwrap_or(&member.path)
                        .to_string_lossy();

                    if file.starts_with(rel.as_ref()) {
                        affected.insert(member.name.clone());
                    }
                }

                // Root-level files (Cargo.toml, Cargo.lock, etc.) affect everything
                if !file.contains('/') {
                    for member in &graph.members {
                        affected.insert(member.name.clone());
                    }
                }
            }

            let mut result: Vec<String> = affected.into_iter().collect();
            result.sort();

            if result.is_empty() {
                // Not in any member path — might be a single-package project
                // In that case, any change means the package is affected
                crate::output::verbose(
                    "not a workspace or changes outside members — treating as affected",
                );
                return Ok(vec!["(root)".to_string()]);
            }

            crate::output::info(&format!("affected packages: {}", result.join(", ")));
            Ok(result)
        }
        Err(_) => {
            // Not a workspace — if any Rust files changed, the project is affected
            let has_rust_changes = files
                .iter()
                .any(|f| f.ends_with(".rs") || f == "Cargo.toml" || f == "Cargo.lock");

            if has_rust_changes {
                Ok(vec!["(root)".to_string()])
            } else {
                crate::output::info("no Rust source files changed");
                Ok(vec![])
            }
        }
    }
}
