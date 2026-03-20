use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::cli::WsCommand;

// ---------------------------------------------------------------------------
// Cargo.toml parsing (minimal, only what we need)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct CargoToml {
    package: Option<PackageSection>,
    workspace: Option<WorkspaceSection>,
    dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(rename = "build-dependencies")]
    build_dependencies: Option<HashMap<String, toml::Value>>,
}

#[derive(Deserialize, Default)]
struct PackageSection {
    name: Option<String>,
}

#[derive(Deserialize, Default)]
struct WorkspaceSection {
    members: Option<Vec<String>>,
}

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

/// Find the workspace root by walking up to find a Cargo.toml with [workspace].
fn find_workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let contents = fs::read_to_string(&cargo_toml)?;
            let parsed: CargoToml = toml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;
            if parsed.workspace.is_some() {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            anyhow::bail!(
                "could not find a workspace root (Cargo.toml with [workspace]) in any parent directory"
            );
        }
    }
}

/// Resolve glob patterns in workspace members list.
fn resolve_member_globs(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for pattern in patterns {
        if pattern.contains('*') {
            // Use simple glob matching via walkdir
            let prefix = pattern.split('*').next().unwrap_or("");
            let search_dir = root.join(prefix);
            if search_dir.exists() {
                for entry in fs::read_dir(&search_dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if p.join("Cargo.toml").exists() {
                        paths.push(p);
                    }
                }
            }
        } else {
            let member_path = root.join(pattern);
            if member_path.join("Cargo.toml").exists() {
                paths.push(member_path);
            }
        }
    }
    Ok(paths)
}

/// Parse a member's Cargo.toml and extract its name and dependency names.
fn parse_member(path: &Path) -> Result<(String, HashSet<String>)> {
    let cargo_toml = path.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("failed to read {}", cargo_toml.display()))?;
    let parsed: CargoToml = toml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;

    let name = parsed.package.and_then(|p| p.name).unwrap_or_else(|| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let mut dep_names = HashSet::new();
    for deps in [
        &parsed.dependencies,
        &parsed.dev_dependencies,
        &parsed.build_dependencies,
    ]
    .into_iter()
    .flatten()
    {
        for (dep_name, value) in deps {
            let is_path_dep = match value {
                toml::Value::Table(t) => {
                    t.contains_key("path")
                        || t.get("workspace")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                }
                _ => false,
            };
            if is_path_dep {
                dep_names.insert(dep_name.clone());
            }
        }
    }

    Ok((name, dep_names))
}

/// Build the full workspace graph.
pub fn resolve_workspace() -> Result<WorkspaceGraph> {
    let root = find_workspace_root()?;
    let cargo_toml = root.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)?;
    let parsed: CargoToml = toml::from_str(&contents)?;

    let ws = parsed
        .workspace
        .context("no [workspace] section in root Cargo.toml")?;
    let member_patterns = ws.members.unwrap_or_default();
    let member_paths = resolve_member_globs(&root, &member_patterns)?;

    let mut members = Vec::new();
    let mut deps = HashMap::new();

    // First pass: collect all member names
    let mut all_names = HashSet::new();
    let mut parsed_members = Vec::new();
    for path in &member_paths {
        let (name, dep_names) = parse_member(path)?;
        all_names.insert(name.clone());
        parsed_members.push((name, path.clone(), dep_names));
    }

    // Second pass: filter deps to only include workspace-internal deps
    for (name, path, dep_names) in parsed_members {
        let ws_deps: HashSet<String> = dep_names
            .into_iter()
            .filter(|d| all_names.contains(d))
            .collect();
        deps.insert(name.clone(), ws_deps);
        members.push(Member {
            name,
            path: path.clone(),
        });
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

/// Check which workspace members actually need to rebuild based on semantic
/// fingerprinting. A downstream crate only needs to rebuild if its upstream
/// dependency's public API changed (not just its implementation).
fn members_needing_rebuild(graph: &WorkspaceGraph) -> HashSet<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return graph.members.iter().map(|m| m.name.clone()).collect(),
    };
    let sem_cache_dir = home.join(".rx").join("cache").join("semantic");
    let _ = std::fs::create_dir_all(&sem_cache_dir);

    let mut needs_rebuild: HashSet<String> = HashSet::new();
    let mut api_changed: HashSet<String> = HashSet::new();

    for member in &graph.members {
        let fp = match crate::cache::compute_semantic_fingerprint(&member.path) {
            Ok(fp) => fp,
            Err(_) => {
                needs_rebuild.insert(member.name.clone());
                api_changed.insert(member.name.clone());
                continue;
            }
        };

        let cache_file = sem_cache_dir.join(format!("{}.txt", member.name));
        let cached = fs::read_to_string(&cache_file).unwrap_or_default();

        if cached.trim() != fp {
            // API changed — this member and its dependents need rebuild
            api_changed.insert(member.name.clone());
            needs_rebuild.insert(member.name.clone());
            let _ = fs::write(&cache_file, &fp);
        }
    }

    // Propagate: if an upstream's API changed, downstream needs rebuild
    for member in &graph.members {
        if let Some(deps) = graph.deps.get(&member.name) {
            for dep in deps {
                if api_changed.contains(dep) {
                    needs_rebuild.insert(member.name.clone());
                }
            }
        }
    }

    needs_rebuild
}

/// Run a cargo subcommand across workspace members in parallel waves.
fn run_across_workspace(cargo_cmd: &str, extra_args: &[String]) -> Result<()> {
    let graph = resolve_workspace()?;
    let waves = compute_waves(&graph)?;
    let total = graph.members.len();

    // Use semantic fingerprinting to skip unchanged members for build/check
    let skip_set = if cargo_cmd == "build" || cargo_cmd == "check" {
        let needs = members_needing_rebuild(&graph);
        let skipped = total - needs.len();
        if skipped > 0 {
            crate::output::info(&format!(
                "semantic fingerprint: skipping {skipped} unchanged member(s)"
            ));
        }
        graph
            .members
            .iter()
            .filter(|m| !needs.contains(&m.name))
            .map(|m| m.name.clone())
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };

    crate::output::info(&format!(
        "workspace: {total} members, {} wave(s)",
        waves.len()
    ));

    let failed = Arc::new(Mutex::new(Vec::<String>::new()));

    for (wave_idx, wave) in waves.iter().enumerate() {
        // Filter out members that don't need rebuild
        let active: Vec<Member> = wave
            .iter()
            .filter(|m| !skip_set.contains(&m.name))
            .cloned()
            .collect();

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

/// Run a named script from rx.toml across workspace members.
fn run_script(name: &str, packages: &[String]) -> Result<()> {
    let graph = resolve_workspace()?;
    let sorted = topo_sort(&graph)?;

    let filter: HashSet<&str> = packages.iter().map(|s| s.as_str()).collect();

    let mut found_any = false;
    for member in sorted {
        if !filter.is_empty() && !filter.contains(member.name.as_str()) {
            continue;
        }

        let config = crate::config::load_for_dir(&member.path)?;
        if let Some(script) = config.scripts.get(name) {
            found_any = true;
            crate::output::step(&member.name, script);
            let status = Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&member.path)
                .status()
                .with_context(|| format!("failed to run script '{name}' in {}", member.name))?;

            if !status.success() {
                anyhow::bail!("script '{name}' failed in {}", member.name);
            }
        }
    }

    if !found_any {
        crate::output::warn(&format!(
            "no workspace members define script '{name}' in rx.toml"
        ));
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
// Remote cache integration
// ---------------------------------------------------------------------------

/// Push workspace member caches to remote backend.
pub fn ws_cache_push(config: &crate::config::RxConfig) -> Result<()> {
    let backend = match crate::remote_cache::resolve_backend(config) {
        Some(b) => b,
        None => {
            crate::output::warn("no remote cache configured in rx.toml (build.remote_cache)");
            return Ok(());
        }
    };

    let graph = resolve_workspace()?;
    crate::output::info(&format!(
        "pushing cache for {} workspace members...",
        graph.members.len()
    ));

    let mut pushed = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    for member in &graph.members {
        let profile = "debug";
        let rustflags = std::env::var("RUSTFLAGS").ok();

        let fingerprint = match crate::cache::compute_build_fingerprint(
            &member.path,
            profile,
            rustflags.as_deref(),
        ) {
            Ok(fp) => fp,
            Err(e) => {
                crate::output::warn(&format!(
                    "{}: failed to compute fingerprint: {e}",
                    member.name
                ));
                failed.push(member.name.clone());
                continue;
            }
        };

        let target_dir = member.path.join("target").join(profile);
        if !target_dir.exists() {
            crate::output::verbose(&format!("{}: no target dir, skipping", member.name));
            skipped += 1;
            continue;
        }

        crate::output::step(&member.name, &format!("pushing {}", &fingerprint[..8]));
        match crate::remote_cache::push(&backend, &fingerprint, &target_dir) {
            Ok(()) => {
                pushed += 1;
            }
            Err(e) => {
                crate::output::warn(&format!("{}: push failed: {e}", member.name));
                failed.push(member.name.clone());
            }
        }
    }

    if !failed.is_empty() {
        crate::output::warn(&format!("failed to push: {}", failed.join(", ")));
    }

    if pushed > 0 {
        crate::output::success(&format!("pushed {pushed} member(s), skipped {skipped}"));
    } else {
        crate::output::info("no caches pushed");
    }

    Ok(())
}

/// Pull workspace member caches from remote backend.
pub fn ws_cache_pull(config: &crate::config::RxConfig) -> Result<()> {
    let backend = match crate::remote_cache::resolve_backend(config) {
        Some(b) => b,
        None => {
            crate::output::warn("no remote cache configured in rx.toml (build.remote_cache)");
            return Ok(());
        }
    };

    let graph = resolve_workspace()?;
    crate::output::info(&format!(
        "pulling cache for {} workspace members...",
        graph.members.len()
    ));

    let mut hits = 0;
    let mut misses = 0;
    let mut failed = Vec::new();

    for member in &graph.members {
        let profile = "debug";
        let rustflags = std::env::var("RUSTFLAGS").ok();

        let fingerprint = match crate::cache::compute_build_fingerprint(
            &member.path,
            profile,
            rustflags.as_deref(),
        ) {
            Ok(fp) => fp,
            Err(e) => {
                crate::output::warn(&format!(
                    "{}: failed to compute fingerprint: {e}",
                    member.name
                ));
                failed.push(member.name.clone());
                continue;
            }
        };

        let target_dir = member.path.join("target").join(profile);

        crate::output::step(&member.name, &format!("checking {}", &fingerprint[..8]));
        match crate::remote_cache::pull(&backend, &fingerprint, &target_dir) {
            Ok(true) => {
                hits += 1;
                crate::output::verbose(&format!("{}: cache hit", member.name));
            }
            Ok(false) => {
                misses += 1;
                crate::output::verbose(&format!("{}: cache miss", member.name));
            }
            Err(e) => {
                crate::output::warn(&format!("{}: pull failed: {e}", member.name));
                failed.push(member.name.clone());
            }
        }
    }

    if !failed.is_empty() {
        crate::output::warn(&format!("failed to pull: {}", failed.join(", ")));
    }

    if hits > 0 {
        crate::output::success(&format!("cache hits: {hits}, misses: {misses}"));
    } else {
        crate::output::info(&format!("no cache hits ({misses} misses)"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn new_project(name: &str, lib: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("new").arg(name);
    if lib {
        cmd.arg("--lib");
    }

    crate::output::info(&format!("creating project: {name}"));
    let status = cmd.status().context("failed to run cargo new")?;
    if !status.success() {
        anyhow::bail!("failed to create project {name}");
    }

    crate::output::success(&format!("project created at ./{name}"));
    eprintln!("  cd {name}");
    eprintln!("  rx run");
    Ok(())
}

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
        WsCommand::Script { name, packages } => run_script(&name, &packages),
        WsCommand::Exec { cmd } => exec_across_workspace(&cmd),
        WsCommand::CachePush => {
            let config = crate::config::load()?;
            ws_cache_push(&config)
        }
        WsCommand::CachePull => {
            let config = crate::config::load()?;
            ws_cache_pull(&config)
        }
    }
}
