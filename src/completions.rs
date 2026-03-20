use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn generate_completions(shell: Shell) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    generate(shell, &mut cmd, "rx", &mut io::stdout());

    // Also emit dynamic completions for context-aware features
    match shell {
        Shell::Bash => emit_bash_dynamic_completions()?,
        Shell::Zsh => emit_zsh_dynamic_completions()?,
        Shell::Fish => emit_fish_dynamic_completions()?,
        _ => {}
    }

    Ok(())
}

#[allow(dead_code)]
pub fn generate_manpage() -> Result<()> {
    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}

/// Discover workspace members for dynamic completion.
fn workspace_members() -> Vec<String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    json.get("packages")
        .and_then(|p| p.as_array())
        .map(|pkgs| {
            pkgs.iter()
                .filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Discover installed toolchains for dynamic completion.
fn installed_toolchains() -> Vec<String> {
    let output = std::process::Command::new("rustup")
        .args(["toolchain", "list"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.split_whitespace().next().unwrap_or(l).to_string())
            .collect(),
        _ => vec![],
    }
}

/// Discover installed targets for dynamic completion.
fn installed_targets() -> Vec<String> {
    let output = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .collect(),
        _ => vec![],
    }
}

/// Discover scripts from rx.toml for dynamic completion.
fn available_scripts() -> Vec<String> {
    match crate::config::load() {
        Ok(config) => config.scripts.keys().cloned().collect(),
        Err(_) => vec![],
    }
}

fn emit_bash_dynamic_completions() -> Result<()> {
    let members = workspace_members();
    let toolchains = installed_toolchains();
    let targets = installed_targets();
    let scripts = available_scripts();

    println!();
    println!("# rx dynamic completions");
    println!("_rx_dynamic_completions() {{");
    println!("  local cur prev");
    println!("  cur=\"${{COMP_WORDS[COMP_CWORD]}}\"");
    println!("  prev=\"${{COMP_WORDS[COMP_CWORD-1]}}\"");

    if !members.is_empty() {
        println!("  if [[ \"$prev\" == \"--package\" || \"$prev\" == \"-p\" ]]; then");
        println!(
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            members.join(" ")
        );
        println!("    return");
        println!("  fi");
    }

    if !toolchains.is_empty() {
        println!(
            "  if [[ \"${{COMP_WORDS[1]}}\" == \"toolchain\" && \"$prev\" == \"use\" ]]; then"
        );
        println!(
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            toolchains.join(" ")
        );
        println!("    return");
        println!("  fi");
    }

    if !targets.is_empty() {
        println!("  if [[ \"$prev\" == \"--target\" ]]; then");
        println!(
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            targets.join(" ")
        );
        println!("    return");
        println!("  fi");
    }

    if !scripts.is_empty() {
        println!("  if [[ \"${{COMP_WORDS[1]}}\" == \"script\" ]]; then");
        println!(
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            scripts.join(" ")
        );
        println!("    return");
        println!("  fi");
    }

    println!("}}");
    println!("complete -F _rx_dynamic_completions rx");

    Ok(())
}

fn emit_zsh_dynamic_completions() -> Result<()> {
    let members = workspace_members();
    let targets = installed_targets();
    let scripts = available_scripts();

    println!();
    println!("# rx dynamic completions for zsh");

    if !members.is_empty() {
        println!("_rx_packages=({}) ", members.join(" "));
    }
    if !targets.is_empty() {
        println!("_rx_targets=({})", targets.join(" "));
    }
    if !scripts.is_empty() {
        println!("_rx_scripts=({})", scripts.join(" "));
    }

    Ok(())
}

fn emit_fish_dynamic_completions() -> Result<()> {
    let members = workspace_members();
    let targets = installed_targets();
    let scripts = available_scripts();

    println!();
    println!("# rx dynamic completions for fish");

    for member in &members {
        println!(
            "complete -c rx -n '__fish_seen_subcommand_from build test check' -l package -xa '{member}'"
        );
    }

    for target in &targets {
        println!("complete -c rx -n '__fish_seen_subcommand_from build' -l target -xa '{target}'");
    }

    for script in &scripts {
        println!("complete -c rx -n '__fish_seen_subcommand_from script' -xa '{script}'");
    }

    Ok(())
}
