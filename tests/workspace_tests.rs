use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use rx::workspace::{Member, WorkspaceGraph};

fn make_graph(
    members: &[&str],
    edges: &[(&str, &str)], // (dependent, dependency)
) -> WorkspaceGraph {
    let members: Vec<Member> = members
        .iter()
        .map(|name| Member {
            name: name.to_string(),
            path: PathBuf::from(format!("/fake/{name}")),
        })
        .collect();

    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for m in &members {
        deps.insert(m.name.clone(), HashSet::new());
    }
    for (dependent, dependency) in edges {
        deps.get_mut(*dependent)
            .unwrap()
            .insert(dependency.to_string());
    }

    WorkspaceGraph {
        root: PathBuf::from("/fake"),
        members,
        deps,
    }
}

// ---------------------------------------------------------------------------
// Topological sort
// ---------------------------------------------------------------------------

#[test]
fn topo_sort_single_member() {
    let graph = make_graph(&["core"], &[]);
    let sorted = rx::workspace::topo_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].name, "core");
}

#[test]
fn topo_sort_linear_chain() {
    // core <- utils <- app
    let graph = make_graph(
        &["core", "utils", "app"],
        &[("utils", "core"), ("app", "utils")],
    );
    let sorted = rx::workspace::topo_sort(&graph).unwrap();
    let names: Vec<&str> = sorted.iter().map(|m| m.name.as_str()).collect();

    // core must come before utils, utils before app
    let core_pos = names.iter().position(|&n| n == "core").unwrap();
    let utils_pos = names.iter().position(|&n| n == "utils").unwrap();
    let app_pos = names.iter().position(|&n| n == "app").unwrap();
    assert!(core_pos < utils_pos);
    assert!(utils_pos < app_pos);
}

#[test]
fn topo_sort_diamond() {
    // core <- utils, core <- api, utils <- app, api <- app
    let graph = make_graph(
        &["core", "utils", "api", "app"],
        &[
            ("utils", "core"),
            ("api", "core"),
            ("app", "utils"),
            ("app", "api"),
        ],
    );
    let sorted = rx::workspace::topo_sort(&graph).unwrap();
    let names: Vec<&str> = sorted.iter().map(|m| m.name.as_str()).collect();

    let core_pos = names.iter().position(|&n| n == "core").unwrap();
    let utils_pos = names.iter().position(|&n| n == "utils").unwrap();
    let api_pos = names.iter().position(|&n| n == "api").unwrap();
    let app_pos = names.iter().position(|&n| n == "app").unwrap();

    assert!(core_pos < utils_pos);
    assert!(core_pos < api_pos);
    assert!(utils_pos < app_pos);
    assert!(api_pos < app_pos);
}

#[test]
fn topo_sort_detects_cycle() {
    let graph = make_graph(&["a", "b"], &[("a", "b"), ("b", "a")]);
    let result = rx::workspace::topo_sort(&graph);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cycle"));
}

#[test]
fn topo_sort_independent_members() {
    let graph = make_graph(&["a", "b", "c"], &[]);
    let sorted = rx::workspace::topo_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 3);
}

// ---------------------------------------------------------------------------
// Parallel waves
// ---------------------------------------------------------------------------

#[test]
fn waves_single_member() {
    let graph = make_graph(&["core"], &[]);
    let waves = rx::workspace::parallel_waves(&graph).unwrap();
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].len(), 1);
}

#[test]
fn waves_all_independent() {
    let graph = make_graph(&["a", "b", "c", "d"], &[]);
    let waves = rx::workspace::parallel_waves(&graph).unwrap();
    // All in one wave since no dependencies
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].len(), 4);
}

#[test]
fn waves_linear_chain() {
    let graph = make_graph(
        &["core", "utils", "app"],
        &[("utils", "core"), ("app", "utils")],
    );
    let waves = rx::workspace::parallel_waves(&graph).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0].len(), 1);
    assert_eq!(waves[0][0].name, "core");
    assert_eq!(waves[1].len(), 1);
    assert_eq!(waves[1][0].name, "utils");
    assert_eq!(waves[2].len(), 1);
    assert_eq!(waves[2][0].name, "app");
}

#[test]
fn waves_diamond_has_parallel_middle() {
    // core <- utils, core <- api, utils <- app, api <- app
    let graph = make_graph(
        &["core", "utils", "api", "app"],
        &[
            ("utils", "core"),
            ("api", "core"),
            ("app", "utils"),
            ("app", "api"),
        ],
    );
    let waves = rx::workspace::parallel_waves(&graph).unwrap();

    // Wave 1: core
    // Wave 2: utils + api (parallel)
    // Wave 3: app
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0].len(), 1);
    assert_eq!(waves[0][0].name, "core");
    assert_eq!(waves[1].len(), 2);
    let middle_names: HashSet<&str> = waves[1].iter().map(|m| m.name.as_str()).collect();
    assert!(middle_names.contains("utils"));
    assert!(middle_names.contains("api"));
    assert_eq!(waves[2].len(), 1);
    assert_eq!(waves[2][0].name, "app");
}

#[test]
fn waves_detects_cycle() {
    let graph = make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c"), ("c", "a")]);
    let result = rx::workspace::parallel_waves(&graph);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cycle"));
}

#[test]
fn waves_complex_graph() {
    // a (no deps), b -> a, c -> a, d -> b + c, e -> d
    let graph = make_graph(
        &["a", "b", "c", "d", "e"],
        &[("b", "a"), ("c", "a"), ("d", "b"), ("d", "c"), ("e", "d")],
    );
    let waves = rx::workspace::parallel_waves(&graph).unwrap();

    // Wave 1: a
    // Wave 2: b, c (parallel)
    // Wave 3: d
    // Wave 4: e
    assert_eq!(waves.len(), 4);
    assert_eq!(waves[0][0].name, "a");
    let wave2: HashSet<&str> = waves[1].iter().map(|m| m.name.as_str()).collect();
    assert!(wave2.contains("b"));
    assert!(wave2.contains("c"));
    assert_eq!(waves[2][0].name, "d");
    assert_eq!(waves[3][0].name, "e");
}
