use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::fs;
use std::io;
use std::path::PathBuf;

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

pub fn generate_manpage() -> Result<()> {
    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}

pub fn install_completions(shell: Shell) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();
    let mut buffer = Vec::new();
    generate(shell, &mut cmd, "rx", &mut buffer);

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

    let (path, instructions) = match shell {
        Shell::Bash => {
            let dir = PathBuf::from(&home).join(".bash_completion.d");
            fs::create_dir_all(&dir)?;
            let path = dir.join("rx.bash");
            let instructions = format!(
                "Bash completions installed to {}\n\nTo enable, add this to your ~/.bashrc:\n  source {}",
                path.display(),
                path.display()
            );
            (path, instructions)
        }
        Shell::Zsh => {
            let dir = PathBuf::from(&home).join(".zfunc");
            fs::create_dir_all(&dir)?;
            let path = dir.join("_rx");
            let instructions = format!(
                "Zsh completions installed to {}\n\nTo enable, add this to your ~/.zshrc:\n  fpath=(~/.zfunc $fpath)\n  autoload -Uz compinit && compinit",
                path.display()
            );
            (path, instructions)
        }
        Shell::Fish => {
            let dir = PathBuf::from(&home).join(".config/fish/completions");
            fs::create_dir_all(&dir)?;
            let path = dir.join("rx.fish");
            let instructions = format!(
                "Fish completions installed to {}\n\nCompletions will be automatically loaded.",
                path.display()
            );
            (path, instructions)
        }
        Shell::PowerShell => {
            let path = PathBuf::from(&home).join("rx_completions.ps1");
            let instructions = format!(
                "PowerShell completions saved to {}\n\nTo enable, add this line to your PowerShell $PROFILE:\n  . {}",
                path.display(),
                path.display()
            );
            (path, instructions)
        }
        _ => {
            anyhow::bail!(
                "Installation not supported for {:?}. Use `rx completions <shell>` to print completions.",
                shell
            );
        }
    };

    fs::write(&path, &buffer)?;

    // Also write dynamic completions if applicable
    match shell {
        Shell::Bash | Shell::Zsh | Shell::Fish => {
            let mut dynamic_buffer = Vec::new();
            match shell {
                Shell::Bash => {
                    emit_bash_dynamic_completions_to_buffer(&mut dynamic_buffer)?;
                }
                Shell::Zsh => {
                    emit_zsh_dynamic_completions_to_buffer(&mut dynamic_buffer)?;
                }
                Shell::Fish => {
                    emit_fish_dynamic_completions_to_buffer(&mut dynamic_buffer)?;
                }
                _ => {}
            }
            if !dynamic_buffer.is_empty() {
                let mut content = buffer;
                content.push(b'\n');
                content.extend_from_slice(&dynamic_buffer);
                fs::write(&path, &content)?;
            }
        }
        _ => {}
    }

    println!("{}", instructions);
    Ok(())
}

pub fn install_manpage() -> Result<()> {
    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);

    let mut buffer = Vec::new();
    man.render(&mut buffer)?;

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

    let dir = PathBuf::from(&home).join(".local/share/man/man1");
    fs::create_dir_all(&dir)?;

    let path = dir.join("rx.1");
    fs::write(&path, &buffer)?;

    println!("Man page installed to {}", path.display());
    println!("\nTo use: man rx");
    println!(
        "Note: Ensure {} is in your MANPATH",
        dir.parent().unwrap().display()
    );

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

fn emit_bash_dynamic_completions_to_buffer(buffer: &mut Vec<u8>) -> Result<()> {
    use std::io::Write;
    let members = workspace_members();
    let toolchains = installed_toolchains();
    let targets = installed_targets();
    let scripts = available_scripts();

    writeln!(buffer)?;
    writeln!(buffer, "# rx dynamic completions")?;
    writeln!(buffer, "_rx_dynamic_completions() {{")?;
    writeln!(buffer, "  local cur prev")?;
    writeln!(buffer, "  cur=\"${{COMP_WORDS[COMP_CWORD]}}\"")?;
    writeln!(buffer, "  prev=\"${{COMP_WORDS[COMP_CWORD-1]}}\"")?;

    if !members.is_empty() {
        writeln!(
            buffer,
            "  if [[ \"$prev\" == \"--package\" || \"$prev\" == \"-p\" ]]; then"
        )?;
        writeln!(
            buffer,
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            members.join(" ")
        )?;
        writeln!(buffer, "    return")?;
        writeln!(buffer, "  fi")?;
    }

    if !toolchains.is_empty() {
        writeln!(
            buffer,
            "  if [[ \"${{COMP_WORDS[1]}}\" == \"toolchain\" && \"$prev\" == \"use\" ]]; then"
        )?;
        writeln!(
            buffer,
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            toolchains.join(" ")
        )?;
        writeln!(buffer, "    return")?;
        writeln!(buffer, "  fi")?;
    }

    if !targets.is_empty() {
        writeln!(buffer, "  if [[ \"$prev\" == \"--target\" ]]; then")?;
        writeln!(
            buffer,
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            targets.join(" ")
        )?;
        writeln!(buffer, "    return")?;
        writeln!(buffer, "  fi")?;
    }

    if !scripts.is_empty() {
        writeln!(
            buffer,
            "  if [[ \"${{COMP_WORDS[1]}}\" == \"script\" ]]; then"
        )?;
        writeln!(
            buffer,
            "    COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            scripts.join(" ")
        )?;
        writeln!(buffer, "    return")?;
        writeln!(buffer, "  fi")?;
    }

    writeln!(buffer, "}}")?;
    writeln!(buffer, "complete -F _rx_dynamic_completions rx")?;

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

fn emit_zsh_dynamic_completions_to_buffer(buffer: &mut Vec<u8>) -> Result<()> {
    use std::io::Write;
    let members = workspace_members();
    let targets = installed_targets();
    let scripts = available_scripts();

    writeln!(buffer)?;
    writeln!(buffer, "# rx dynamic completions for zsh")?;

    if !members.is_empty() {
        writeln!(buffer, "_rx_packages=({}) ", members.join(" "))?;
    }
    if !targets.is_empty() {
        writeln!(buffer, "_rx_targets=({})", targets.join(" "))?;
    }
    if !scripts.is_empty() {
        writeln!(buffer, "_rx_scripts=({})", scripts.join(" "))?;
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

fn emit_fish_dynamic_completions_to_buffer(buffer: &mut Vec<u8>) -> Result<()> {
    use std::io::Write;
    let members = workspace_members();
    let targets = installed_targets();
    let scripts = available_scripts();

    writeln!(buffer)?;
    writeln!(buffer, "# rx dynamic completions for fish")?;

    for member in &members {
        writeln!(
            buffer,
            "complete -c rx -n '__fish_seen_subcommand_from build test check' -l package -xa '{member}'"
        )?;
    }

    for target in &targets {
        writeln!(
            buffer,
            "complete -c rx -n '__fish_seen_subcommand_from build' -l target -xa '{target}'"
        )?;
    }

    for script in &scripts {
        writeln!(
            buffer,
            "complete -c rx -n '__fish_seen_subcommand_from script' -xa '{script}'"
        )?;
    }

    Ok(())
}
