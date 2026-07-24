use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// Sentinel returned for single-package (non-workspace) projects: the whole
/// project is affected, and no `-p` selection should be passed to Cargo.
pub const ROOT: &str = "(root)";

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

/// Expand a set of directly-changed members to everything that depends on
/// them, transitively. A change in `core` affects every member whose
/// (transitive) dependencies include `core`.
fn propagate_to_dependents(
    direct: &HashSet<String>,
    deps: &HashMap<String, HashSet<String>>,
) -> HashSet<String> {
    let mut affected = direct.clone();
    // Fixed point: keep adding members that depend on an affected member.
    loop {
        let before = affected.len();
        for (member, member_deps) in deps {
            if !affected.contains(member) && member_deps.iter().any(|d| affected.contains(d)) {
                affected.insert(member.clone());
            }
        }
        if affected.len() == before {
            return affected;
        }
    }
}

/// Determine which workspace packages are affected by changed files.
///
/// Resolution happens once: changed files are mapped to workspace members,
/// then the set is expanded to transitive dependents. Callers pass the
/// result to a single Cargo invocation with repeated `-p` selections.
///
/// Returns `[ROOT]` for single-package projects (run without `-p`), or an
/// empty vec when nothing relevant changed.
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
            // A single-package project has no meaningful member selection.
            if graph.members.len() == 1 {
                return Ok(vec![ROOT.to_string()]);
            }

            // Member directories relative to the workspace root. A member
            // with an empty rel is the root package itself.
            let member_rels: Vec<(String, String)> = graph
                .members
                .iter()
                .map(|m| {
                    let rel = m
                        .path
                        .strip_prefix(&graph.root)
                        .unwrap_or(&m.path)
                        .to_string_lossy()
                        .into_owned();
                    (m.name.clone(), rel)
                })
                .collect();

            let mut direct: HashSet<String> = HashSet::new();

            for file in &files {
                // Longest matching member directory wins, so nested members
                // don't also attribute changes to the root package (rel "").
                let owner = member_rels
                    .iter()
                    .filter(|(_, rel)| rel.is_empty() || file.starts_with(rel.as_str()))
                    .max_by_key(|(_, rel)| rel.len());

                match owner {
                    Some((name, rel)) if !rel.is_empty() => {
                        direct.insert(name.clone());
                    }
                    Some((name, _)) if file.contains('/') => {
                        // Root package owns nested files no member matched
                        // (e.g. its own src/).
                        direct.insert(name.clone());
                    }
                    _ => {}
                }

                // Root-level files (Cargo.toml, Cargo.lock, etc.) affect everything
                if !file.contains('/') {
                    for member in &graph.members {
                        direct.insert(member.name.clone());
                    }
                }
            }

            if direct.is_empty() {
                crate::output::info("no workspace members affected");
                return Ok(vec![]);
            }

            // A changed member affects everything that depends on it.
            let affected = propagate_to_dependents(&direct, &graph.deps);

            let mut result: Vec<String> = affected.into_iter().collect();
            result.sort();

            crate::output::info(&format!("affected packages: {}", result.join(", ")));
            Ok(result)
        }
        Err(_) => {
            // Not a cargo project we can resolve — if any Rust files changed,
            // treat the whole project as affected
            let has_rust_changes = files
                .iter()
                .any(|f| f.ends_with(".rs") || f == "Cargo.toml" || f == "Cargo.lock");

            if has_rust_changes {
                Ok(vec![ROOT.to_string()])
            } else {
                crate::output::info("no Rust source files changed");
                Ok(vec![])
            }
        }
    }
}

/// Convert an affected-packages result into the `-p` selection to hand to
/// Cargo: `None` means "run without selection" (nothing to filter by),
/// otherwise the list of package names.
pub fn to_package_selection(affected: Vec<String>) -> Option<Vec<String>> {
    if affected.len() == 1 && affected[0] == ROOT {
        None
    } else {
        Some(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deps(pairs: &[(&str, &[&str])]) -> HashMap<String, HashSet<String>> {
        pairs
            .iter()
            .map(|(name, ds)| (name.to_string(), ds.iter().map(|d| d.to_string()).collect()))
            .collect()
    }

    fn direct(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn leaf_change_does_not_propagate_upstream() {
        // cli depends on core; changing cli affects only cli
        let graph = deps(&[("core", &[]), ("cli", &["core"])]);
        let affected = propagate_to_dependents(&direct(&["cli"]), &graph);
        assert_eq!(affected, direct(&["cli"]));
    }

    #[test]
    fn base_change_propagates_to_dependents() {
        // cli depends on core; changing core affects both
        let graph = deps(&[("core", &[]), ("cli", &["core"])]);
        let affected = propagate_to_dependents(&direct(&["core"]), &graph);
        assert_eq!(affected, direct(&["core", "cli"]));
    }

    #[test]
    fn propagation_is_transitive() {
        // api -> core, cli -> api; changing core affects all three
        let graph = deps(&[("core", &[]), ("api", &["core"]), ("cli", &["api"])]);
        let affected = propagate_to_dependents(&direct(&["core"]), &graph);
        assert_eq!(affected, direct(&["core", "api", "cli"]));
    }

    #[test]
    fn siblings_are_not_affected() {
        let graph = deps(&[("core", &[]), ("a", &["core"]), ("b", &["core"])]);
        let affected = propagate_to_dependents(&direct(&["a"]), &graph);
        assert_eq!(affected, direct(&["a"]));
    }

    #[test]
    fn root_selection_maps_to_none() {
        assert_eq!(to_package_selection(vec![ROOT.to_string()]), None);
        assert_eq!(
            to_package_selection(vec!["a".to_string()]),
            Some(vec!["a".to_string()])
        );
    }
}
