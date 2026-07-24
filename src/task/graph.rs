use anyhow::Result;
use std::collections::{HashMap, HashSet};

use super::runner::Task;

/// Dependency graph over the tasks reachable from a root task.
pub struct TaskGraph<'a> {
    tasks: &'a HashMap<String, Task>,
}

impl<'a> TaskGraph<'a> {
    pub fn new(tasks: &'a HashMap<String, Task>) -> Self {
        Self { tasks }
    }

    /// Compute execution waves for `root` and its transitive dependencies.
    /// Each wave contains tasks whose dependencies are satisfied by earlier
    /// waves; tasks within a wave are independent of each other.
    /// Fails on unknown task names and dependency cycles.
    pub fn waves(&self, root: &str) -> Result<Vec<Vec<String>>> {
        // Collect the transitive closure, validating names along the way.
        let mut needed: HashSet<String> = HashSet::new();
        let mut stack = vec![root.to_string()];
        while let Some(name) = stack.pop() {
            let task = self.tasks.get(&name).ok_or_else(|| {
                let mut available: Vec<&str> = self.tasks.keys().map(|s| s.as_str()).collect();
                available.sort_unstable();
                anyhow::anyhow!(
                    "unknown task `{name}`\n\
                     available tasks: {}",
                    available.join(", ")
                )
            })?;
            if needed.insert(name) {
                stack.extend(task.depends_on.iter().cloned());
            }
        }

        // Kahn's algorithm over the closure.
        let mut in_degree: HashMap<&str, usize> = needed
            .iter()
            .map(|name| (name.as_str(), self.tasks[name].depends_on.len()))
            .collect();

        let mut waves: Vec<Vec<String>> = Vec::new();
        let mut remaining: HashSet<&str> = needed.iter().map(|s| s.as_str()).collect();

        while !remaining.is_empty() {
            let mut wave: Vec<String> = remaining
                .iter()
                .filter(|name| in_degree[**name] == 0)
                .map(|s| s.to_string())
                .collect();

            if wave.is_empty() {
                let mut cycle: Vec<&str> = remaining.iter().copied().collect();
                cycle.sort_unstable();
                anyhow::bail!(
                    "dependency cycle in tasks: {}\n\
                     hint: check the depends-on entries in rx.toml",
                    cycle.join(", ")
                );
            }
            wave.sort_unstable();

            for name in &wave {
                remaining.remove(name.as_str());
                for (dependent, deg) in in_degree.iter_mut() {
                    if self.tasks[*dependent].depends_on.iter().any(|d| d == name) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
            waves.push(wave);
        }

        Ok(waves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::runner::{Task, TaskKind};

    fn shell(cmd: &str, deps: &[&str]) -> Task {
        Task {
            kind: TaskKind::Shell(cmd.to_string()),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn group(deps: &[&str]) -> Task {
        Task {
            kind: TaskKind::Group,
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn single_task_single_wave() {
        let mut tasks = HashMap::new();
        tasks.insert("fmt".to_string(), shell("cargo fmt", &[]));
        let waves = TaskGraph::new(&tasks).waves("fmt").unwrap();
        assert_eq!(waves, vec![vec!["fmt".to_string()]]);
    }

    #[test]
    fn dependencies_run_before_root() {
        let mut tasks = HashMap::new();
        tasks.insert("fmt".to_string(), shell("a", &[]));
        tasks.insert("lint".to_string(), shell("b", &[]));
        tasks.insert("ci".to_string(), group(&["fmt", "lint"]));
        let waves = TaskGraph::new(&tasks).waves("ci").unwrap();
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0], vec!["fmt".to_string(), "lint".to_string()]);
        assert_eq!(waves[1], vec!["ci".to_string()]);
    }

    #[test]
    fn diamond_resolves_in_three_waves() {
        let mut tasks = HashMap::new();
        tasks.insert("base".to_string(), shell("a", &[]));
        tasks.insert("left".to_string(), shell("b", &["base"]));
        tasks.insert("right".to_string(), shell("c", &["base"]));
        tasks.insert("top".to_string(), group(&["left", "right"]));
        let waves = TaskGraph::new(&tasks).waves("top").unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["base".to_string()]);
        assert_eq!(waves[1], vec!["left".to_string(), "right".to_string()]);
        assert_eq!(waves[2], vec!["top".to_string()]);
    }

    #[test]
    fn only_reachable_tasks_run() {
        let mut tasks = HashMap::new();
        tasks.insert("fmt".to_string(), shell("a", &[]));
        tasks.insert("unrelated".to_string(), shell("b", &[]));
        let waves = TaskGraph::new(&tasks).waves("fmt").unwrap();
        assert_eq!(waves, vec![vec!["fmt".to_string()]]);
    }

    #[test]
    fn unknown_task_errors() {
        let tasks = HashMap::new();
        let err = TaskGraph::new(&tasks).waves("nope").unwrap_err();
        assert!(err.to_string().contains("unknown task"));
    }

    #[test]
    fn unknown_dependency_errors() {
        let mut tasks = HashMap::new();
        tasks.insert("ci".to_string(), group(&["missing"]));
        let err = TaskGraph::new(&tasks).waves("ci").unwrap_err();
        assert!(err.to_string().contains("unknown task `missing`"));
    }

    #[test]
    fn cycle_is_detected() {
        let mut tasks = HashMap::new();
        tasks.insert("a".to_string(), shell("x", &["b"]));
        tasks.insert("b".to_string(), shell("y", &["a"]));
        let err = TaskGraph::new(&tasks).waves("a").unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn self_cycle_is_detected() {
        let mut tasks = HashMap::new();
        tasks.insert("a".to_string(), shell("x", &["a"]));
        let err = TaskGraph::new(&tasks).waves("a").unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }
}
