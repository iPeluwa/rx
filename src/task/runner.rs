use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;

use crate::config::RxConfig;

use super::graph::TaskGraph;
use super::process;

/// What executing a task actually does.
#[derive(Clone)]
pub enum TaskKind {
    /// A shell command from rx.toml.
    Shell(String),
    /// A built-in implementation (fmt, lint, test, build, check).
    Builtin(Builtin),
    /// No command of its own — exists only to group dependencies (e.g. ci).
    Group,
}

#[derive(Clone, Copy)]
pub enum Builtin {
    /// `cargo fmt --check` semantics: tasks verify, they don't rewrite.
    FmtCheck,
    Lint,
    Test,
    Build,
    Check,
}

/// A fully resolved task: how to run it and what must run first.
#[derive(Clone)]
pub struct Task {
    pub kind: TaskKind,
    pub depends_on: Vec<String>,
}

/// Built-in default tasks, used when rx.toml doesn't define the name.
/// The `ci` default is itself expressed as a task with dependencies —
/// there is no separate hard-coded pipeline.
fn builtin_tasks() -> HashMap<String, Task> {
    let mut tasks = HashMap::new();
    for (name, builtin) in [
        ("fmt", Builtin::FmtCheck),
        ("lint", Builtin::Lint),
        ("test", Builtin::Test),
        ("build", Builtin::Build),
        ("check", Builtin::Check),
    ] {
        tasks.insert(
            name.to_string(),
            Task {
                kind: TaskKind::Builtin(builtin),
                depends_on: vec![],
            },
        );
    }
    tasks.insert(
        "ci".to_string(),
        Task {
            kind: TaskKind::Group,
            depends_on: vec![
                "fmt".to_string(),
                "lint".to_string(),
                "test".to_string(),
                "build".to_string(),
            ],
        },
    );
    tasks
}

/// Resolve the full task table: built-in defaults overlaid with rx.toml
/// definitions ([tasks], plus legacy [scripts]). User definitions win.
pub fn resolve_tasks(config: &RxConfig) -> HashMap<String, Task> {
    let mut tasks = builtin_tasks();
    for (name, def) in config.resolved_tasks() {
        let kind = match def.command() {
            Some(cmd) => TaskKind::Shell(cmd.to_string()),
            None => TaskKind::Group,
        };
        tasks.insert(
            name,
            Task {
                kind,
                depends_on: def.depends_on().to_vec(),
            },
        );
    }
    tasks
}

/// Run a task and its dependencies. Independent shell tasks within a wave
/// run concurrently (with captured output); built-ins stream directly.
pub fn run(name: &str, extra_args: &[String], config: &RxConfig) -> Result<()> {
    let mut tasks = resolve_tasks(config);

    // `rx run <task> -- args` appends the args to the task's shell command.
    if !extra_args.is_empty() {
        match tasks.get_mut(name).map(|t| &mut t.kind) {
            Some(TaskKind::Shell(cmd)) => {
                cmd.push(' ');
                cmd.push_str(&extra_args.join(" "));
            }
            Some(_) => anyhow::bail!(
                "task `{name}` is not a shell command — extra arguments after `--` only apply to [tasks] entries with a `command`"
            ),
            None => {} // unknown name — let waves() produce the error
        }
    }

    let waves = TaskGraph::new(&tasks).waves(name)?;
    let total: usize = waves.iter().map(|w| w.len()).sum();
    if total > 1 {
        crate::output::info(&format!("task `{name}`: {total} task(s) to run"));
    }

    let started = Instant::now();
    let mut timings: Vec<(String, std::time::Duration)> = Vec::new();

    for wave in &waves {
        let mut shell: Vec<(String, String)> = Vec::new();
        let mut sequential: Vec<String> = Vec::new();

        for task_name in wave {
            match &tasks[task_name].kind {
                TaskKind::Shell(cmd) => shell.push((task_name.clone(), cmd.clone())),
                TaskKind::Builtin(_) => sequential.push(task_name.clone()),
                TaskKind::Group => {}
            }
        }

        // A lone shell task streams; several run concurrently with
        // captured output so they don't interleave.
        if shell.len() == 1 && sequential.is_empty() {
            let (task_name, cmd) = &shell[0];
            crate::output::step(task_name, cmd);
            let start = Instant::now();
            if !process::run_streamed(cmd)? {
                anyhow::bail!("task `{task_name}` failed");
            }
            timings.push((task_name.clone(), start.elapsed()));
            continue;
        }

        let handles: Vec<_> = shell
            .into_iter()
            .map(|(task_name, cmd)| {
                crate::output::step(&task_name, &cmd);
                let start = Instant::now();
                std::thread::spawn(move || {
                    let result = process::run_captured(&cmd);
                    (task_name, result, start.elapsed())
                })
            })
            .collect();

        let mut failed: Vec<String> = Vec::new();

        for task_name in sequential {
            let start = Instant::now();
            crate::output::step(&task_name, "(built-in)");
            if let Err(e) = run_builtin_task(&tasks[&task_name], config) {
                crate::output::error(&format!("task `{task_name}` failed: {e:#}"));
                failed.push(task_name.clone());
            }
            timings.push((task_name, start.elapsed()));
        }

        for handle in handles {
            let (task_name, result, elapsed) = handle.join().expect("task thread panicked");
            print!("{}", result.output);
            if !result.success {
                failed.push(task_name.clone());
            }
            timings.push((task_name, elapsed));
        }

        if !failed.is_empty() {
            failed.sort_unstable();
            anyhow::bail!("task(s) failed: {}", failed.join(", "));
        }
    }

    if total > 1 {
        for (task_name, elapsed) in &timings {
            crate::output::verbose(&format!("{task_name}: {:.1}s", elapsed.as_secs_f64()));
        }
        crate::output::success(&format!(
            "task `{name}` passed ({} task(s) in {:.1}s)",
            total,
            started.elapsed().as_secs_f64()
        ));
    }
    Ok(())
}

fn run_builtin_task(task: &Task, config: &RxConfig) -> Result<()> {
    let TaskKind::Builtin(builtin) = &task.kind else {
        unreachable!("run_builtin_task called on non-builtin");
    };
    match builtin {
        Builtin::FmtCheck => crate::fmt::fmt(true, config),
        Builtin::Lint => crate::lint::lint(false, config),
        Builtin::Test => crate::test::test(None, None, false, config),
        Builtin::Build => crate::build::build(false, None, None, config),
        Builtin::Check => crate::check::check(None, config),
    }
}

/// List all available tasks: rx.toml definitions plus built-in defaults.
pub fn list(config: &RxConfig) -> Result<()> {
    let tasks = resolve_tasks(config);
    let user_defined = config.resolved_tasks();

    let mut names: Vec<&String> = tasks.keys().collect();
    names.sort_unstable();

    for name in names {
        let task = &tasks[name];
        let what = match &task.kind {
            TaskKind::Shell(cmd) => cmd.clone(),
            TaskKind::Builtin(_) => "(built-in)".to_string(),
            TaskKind::Group => format!("depends-on: {}", task.depends_on.join(", ")),
        };
        let origin = if user_defined.contains_key(name.as_str()) {
            "rx.toml "
        } else {
            "default "
        };
        println!("  {origin}{name:<16} {what}");
    }
    Ok(())
}
