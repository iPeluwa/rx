use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

const TEMPLATE_AXUM: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
"#;

const TEMPLATE_AXUM_MAIN: &str = r#"use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::init();

    let app = Router::new().route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
"#;

const TEMPLATE_CLI_DEPS: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
anyhow = "1"
"#;

const TEMPLATE_CLI_MAIN: &str = r#"use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Name to greet
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    for _ in 0..cli.count {
        println!("Hello, {}!", cli.name);
    }

    Ok(())
}
"#;

const TEMPLATE_WASM_DEPS: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
"#;

const TEMPLATE_WASM_LIB: &str = r#"use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_greet() {
        assert_eq!(greet("world"), "Hello, world!");
    }
}
"#;

const TEMPLATE_LIB_DEPS: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
description = "A Rust library"

[dependencies]
"#;

const TEMPLATE_LIB_SRC: &str = r#"/// Add two numbers together.
///
/// # Examples
///
/// ```
/// assert_eq!({{crate_name}}::add(2, 3), 5);
/// ```
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;

pub const AVAILABLE_TEMPLATES: &[&str] = &["axum", "cli", "wasm", "lib"];

pub fn new_from_template(name: &str, template: &str) -> Result<()> {
    if !AVAILABLE_TEMPLATES.contains(&template) {
        anyhow::bail!(
            "unknown template `{template}`\n\
             available templates: {}",
            AVAILABLE_TEMPLATES.join(", ")
        );
    }

    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("directory `{name}` already exists");
    }

    crate::output::info(&format!(
        "creating project `{name}` from template `{template}`..."
    ));

    // Create project structure
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).context("failed to create project directory")?;

    let crate_name = name.replace('-', "_");

    match template {
        "axum" => {
            let cargo = TEMPLATE_AXUM.replace("{{name}}", name);
            fs::write(project_dir.join("Cargo.toml"), cargo)?;
            fs::write(src_dir.join("main.rs"), TEMPLATE_AXUM_MAIN)?;
        }
        "cli" => {
            let cargo = TEMPLATE_CLI_DEPS.replace("{{name}}", name);
            fs::write(project_dir.join("Cargo.toml"), cargo)?;
            fs::write(src_dir.join("main.rs"), TEMPLATE_CLI_MAIN)?;
        }
        "wasm" => {
            let cargo = TEMPLATE_WASM_DEPS.replace("{{name}}", name);
            fs::write(project_dir.join("Cargo.toml"), cargo)?;
            fs::write(src_dir.join("lib.rs"), TEMPLATE_WASM_LIB)?;
        }
        "lib" => {
            let cargo = TEMPLATE_LIB_DEPS.replace("{{name}}", name);
            fs::write(project_dir.join("Cargo.toml"), cargo)?;
            let lib_src = TEMPLATE_LIB_SRC.replace("{{crate_name}}", &crate_name);
            fs::write(src_dir.join("lib.rs"), lib_src)?;
        }
        _ => unreachable!(),
    }

    // Initialize git
    let _ = Command::new("git").args(["init", name]).output();

    // Create .gitignore
    fs::write(project_dir.join(".gitignore"), "/target\n")?;

    crate::output::success(&format!(
        "project created at ./{name} (template: {template})"
    ));
    eprintln!("  cd {name}");
    if template == "wasm" {
        eprintln!("  wasm-pack build");
    } else if template == "lib" {
        eprintln!("  cargo test");
    } else {
        eprintln!("  rx run");
    }
    Ok(())
}
