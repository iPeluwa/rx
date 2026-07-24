use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::cli::WsCommand;

// ---------------------------------------------------------------------------
// Workspace graph
// ---------------------------------------------------------------------------

/// A resolved workspace member.
#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub path: PathBuf,
}

/// Dependency graph: maps package name -> set of workspace deps it depends on.
pub struct WorkspaceGraph {
    pub root: PathBuf,
    pub members: Vec<Member>,
    pub deps: HashMap<String, HashSet<String>>,
}

/// Build the full workspace graph from `cargo metadata`. Cargo owns
/// workspace semantics (member globs, exclusions, inheritance, name
/// resolution) — rx just reads the answer.
pub fn resolve_workspace() -> Result<WorkspaceGraph> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("failed to run cargo metadata — is cargo installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata output")?;

    let root = PathBuf::from(
        metadata["workspace_root"]
            .as_str()
            .context("cargo metadata output missing workspace_root")?,
    );

    let packages = metadata["packages"]
        .as_array()
        .context("cargo metadata output missing packages")?;

    // With --no-deps, `packages` contains exactly the workspace members.
    let member_names: HashSet<String> = packages
        .iter()
        .filter_map(|p| p["name"].as_str().map(String::from))
        .collect();

    let mut members = Vec::new();
    let mut deps = HashMap::new();

    for package in packages {
        let name = package["name"]
            .as_str()
            .context("package missing name")?
            .to_string();
        let manifest = PathBuf::from(
            package["manifest_path"]
                .as_str()
                .context("package missing manifest_path")?,
        );
        let path = manifest
            .parent()
            .context("manifest_path has no parent directory")?
            .to_path_buf();

        // Workspace-internal deps: path dependencies whose name is a member.
        let ws_deps: HashSet<String> = package["dependencies"]
            .as_array()
            .map(|dep_list| {
                dep_list
                    .iter()
                    .filter(|d| !d["path"].is_null())
                    .filter_map(|d| d["name"].as_str())
                    .filter(|n| member_names.contains(*n))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        deps.insert(name.clone(), ws_deps);
        members.push(Member { name, path });
    }

    if members.is_empty() {
        anyhow::bail!("no packages found in workspace");
    }

    Ok(WorkspaceGraph {
        root,
        members,
        deps,
    })
}

// ---------------------------------------------------------------------------
// Topological sort
// ---------------------------------------------------------------------------

/// Returns members in topological order (dependencies first).
/// Detects cycles.
pub fn topo_sort(graph: &WorkspaceGraph) -> Result<Vec<&Member>> {
    let member_map: HashMap<&str, &Member> =
        graph.members.iter().map(|m| (m.name.as_str(), m)).collect();

    // Compute in-degrees
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for m in &graph.members {
        in_degree.entry(m.name.as_str()).or_insert(0);
    }
    for (name, dep_set) in &graph.deps {
        for dep in dep_set {
            if member_map.contains_key(dep.as_str()) {
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&name, _)| name)
        .collect();
    let mut sorted = Vec::new();

    while let Some(name) = queue.pop_front() {
        sorted.push(*member_map.get(name).unwrap());

        // Find members that depend on `name` and decrement their in-degree
        for (dependent, dep_set) in &graph.deps {
            if dep_set.contains(name) {
                if let Some(deg) = in_degree.get_mut(dependent.as_str()) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dependent.as_str());
                    }
                }
            }
        }
    }

    if sorted.len() != graph.members.len() {
        anyhow::bail!("cycle detected in workspace dependency graph");
    }

    Ok(sorted)
}

/// Group members into parallelizable "waves" based on the dependency graph.
/// Each wave contains members whose dependencies have all been satisfied by
/// previous waves.
pub fn parallel_waves(graph: &WorkspaceGraph) -> Result<Vec<Vec<&Member>>> {
    let member_map: HashMap<&str, &Member> =
        graph.members.iter().map(|m| (m.name.as_str(), m)).collect();

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for m in &graph.members {
        in_degree.entry(m.name.as_str()).or_insert(0);
    }
    for (name, dep_set) in &graph.deps {
        for dep in dep_set {
            if member_map.contains_key(dep.as_str()) {
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
            }
        }
    }

    let mut waves: Vec<Vec<&Member>> = Vec::new();
    let mut remaining: HashSet<&str> = graph.members.iter().map(|m| m.name.as_str()).collect();

    while !remaining.is_empty() {
        let wave: Vec<&str> = remaining
            .iter()
            .filter(|&&name| in_degree.get(name).copied().unwrap_or(0) == 0)
            .copied()
            .collect();

        if wave.is_empty() {
            anyhow::bail!("cycle detected in workspace dependency graph");
        }

        let wave_members: Vec<&Member> =
            wave.iter().map(|&n| *member_map.get(n).unwrap()).collect();
        waves.push(wave_members);

        for &name in &wave {
            remaining.remove(name);
            // Decrement in-degree for dependents
            for (dependent, dep_set) in &graph.deps {
                if dep_set.contains(name) {
                    if let Some(deg) = in_degree.get_mut(dependent.as_str()) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }
    }

    Ok(waves)
}

// ---------------------------------------------------------------------------
// Execution engine
// ---------------------------------------------------------------------------

/// Result of running a command on a package.
struct ExecResult {
    package: String,
    success: bool,
    output: String,
}

/// Run a shell command in a member's directory.
fn run_in_dir(member: &Member, cmd: &str, args: &[String]) -> ExecResult {
    let result = Command::new(cmd)
        .args(args)
        .current_dir(&member.path)
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            ExecResult {
                package: member.name.clone(),
                success: output.status.success(),
                output: format!("{stdout}{stderr}"),
            }
        }
        Err(e) => ExecResult {
            package: member.name.clone(),
            success: false,
            output: format!("failed to execute: {e}"),
        },
    }
}

/// Compute parallel waves from owned member data (avoids lifetime issues with threads).
fn compute_waves(graph: &WorkspaceGraph) -> Result<Vec<Vec<Member>>> {
    let waves = parallel_waves(graph)?;
    Ok(waves
        .into_iter()
        .map(|wave| wave.into_iter().cloned().collect())
        .collect())
}

/// Run a cargo subcommand across workspace members in parallel waves.
fn run_across_workspace(cargo_cmd: &str, extra_args: &[String]) -> Result<()> {
    let graph = resolve_workspace()?;
    let waves = compute_waves(&graph)?;
    let total = graph.members.len();

    crate::output::info(&format!(
        "workspace: {total} members, {} wave(s)",
        waves.len()
    ));

    let failed = Arc::new(Mutex::new(Vec::<String>::new()));

    for (wave_idx, wave) in waves.iter().enumerate() {
        let active: Vec<Member> = wave.to_vec();

        if active.is_empty() {
            continue;
        }

        crate::output::step(
            &format!("wave {}/{}", wave_idx + 1, waves.len()),
            &active
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );

        if active.len() == 1 {
            let member = &active[0];
            let mut args = vec![cargo_cmd.to_string()];
            args.extend(extra_args.iter().cloned());
            let result = run_in_dir(member, "cargo", &args);
            print!("{}", result.output);
            if !result.success {
                failed.lock().unwrap().push(result.package);
            }
        } else {
            let handles: Vec<_> = active
                .iter()
                .map(|member| {
                    let member = member.clone();
                    let cmd = cargo_cmd.to_string();
                    let args: Vec<String> = extra_args.to_vec();
                    let failed = Arc::clone(&failed);

                    thread::spawn(move || {
                        let mut full_args = vec![cmd];
                        full_args.extend(args);
                        let result = run_in_dir(&member, "cargo", &full_args);
                        if !result.success {
                            failed.lock().unwrap().push(result.package.clone());
                        }
                        result
                    })
                })
                .collect();

            for handle in handles {
                let result = handle.join().expect("thread panicked");
                print!("{}", result.output);
            }
        }

        if !failed.lock().unwrap().is_empty() {
            let failures = failed.lock().unwrap();
            anyhow::bail!("failed in wave {}: {}", wave_idx + 1, failures.join(", "));
        }
    }

    crate::output::success(&format!(
        "workspace: all {total} members completed successfully"
    ));
    Ok(())
}

/// Execute a raw shell command in each member's directory.
fn exec_across_workspace(cmd_parts: &[String]) -> Result<()> {
    let graph = resolve_workspace()?;
    let sorted = topo_sort(&graph)?;

    for member in sorted {
        crate::output::step(&member.name, &cmd_parts.join(" "));
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd_parts.join(" "))
            .current_dir(&member.path)
            .status()
            .with_context(|| format!("failed to exec in {}", member.name))?;

        if !status.success() {
            anyhow::bail!("command failed in {}", member.name);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn print_graph(graph: &WorkspaceGraph) {
    println!("Workspace: {}", graph.root.display());
    println!("Members ({}):", graph.members.len());
    for member in &graph.members {
        let dep_set = graph.deps.get(&member.name);
        let deps_str = match dep_set {
            Some(deps) if !deps.is_empty() => {
                let mut sorted: Vec<&str> = deps.iter().map(|s| s.as_str()).collect();
                sorted.sort();
                format!(" -> {}", sorted.join(", "))
            }
            _ => String::new(),
        };
        println!("  {}{deps_str}", member.name);
    }
}

fn list_members(graph: &WorkspaceGraph) {
    for member in &graph.members {
        let rel = member
            .path
            .strip_prefix(&graph.root)
            .unwrap_or(&member.path);
        println!("{:<24} {}", member.name, rel.display());
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn dispatch(cmd: WsCommand) -> Result<()> {
    match cmd {
        WsCommand::List => {
            let graph = resolve_workspace()?;
            list_members(&graph);
            Ok(())
        }
        WsCommand::Graph => {
            let graph = resolve_workspace()?;
            print_graph(&graph);
            Ok(())
        }
        WsCommand::Run { cmd, args } => run_across_workspace(&cmd, &args),
        WsCommand::Exec { cmd } => exec_across_workspace(&cmd),
    }
}
