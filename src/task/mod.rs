//! The task executor: one place where every rx pipeline runs.
//!
//! A task is a named unit of work from rx.toml (`[tasks]`) or a built-in
//! default (fmt, lint, test, build, check, ci). Tasks declare dependencies
//! with `depends-on`; the runner resolves the graph, runs independent tasks
//! concurrently, and delegates all compilation to Cargo.

pub mod graph;
pub mod process;
pub mod runner;
