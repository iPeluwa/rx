//! rxd — persistent background daemon for rx.
//!
//! The daemon holds workspace state in memory (dependency graph, config,
//! file watcher, fingerprint cache) and communicates with `rx` CLI
//! invocations over a Unix domain socket. This eliminates cold-start
//! overhead for repeated commands.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// The daemon's persistent state held in memory.
pub struct DaemonState {
    pub config: crate::config::RxConfig,
    pub workspace: Option<crate::workspace::WorkspaceGraph>,
    pub pid: u32,
    pub started_at: std::time::Instant,
}

/// A request from the CLI to the daemon.
#[derive(Serialize, Deserialize, Debug)]
pub struct DaemonRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
}

/// A response from the daemon to the CLI.
#[derive(Serialize, Deserialize, Debug)]
pub struct DaemonResponse {
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// Path to the daemon's Unix socket.
pub fn socket_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".rx").join("rxd.sock"))
}

/// Path to the daemon's PID file.
pub fn pid_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".rx").join("rxd.pid"))
}

/// Check if the daemon is already running.
pub fn is_running() -> bool {
    let pid_file = match pid_path() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if !pid_file.exists() {
        return false;
    }

    let pid_str = match fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check if process is still alive
    #[cfg(unix)]
    {
        unsafe { libc_free_kill_check(pid) }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Check if a process is alive without using libc directly.
#[cfg(unix)]
unsafe fn libc_free_kill_check(pid: u32) -> bool {
    // Use /proc or ps to check if process exists
    let output = process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output();
    output.map(|o| o.status.success()).unwrap_or(false)
}

/// Start the daemon. If `foreground` is false, re-exec as a background process.
pub fn start(foreground: bool) -> Result<()> {
    if is_running() {
        crate::output::info("daemon is already running");
        return Ok(());
    }

    if !foreground {
        // Re-exec ourselves with --foreground in the background
        let exe = std::env::current_exe().context("cannot determine own executable path")?;
        let child = process::Command::new(exe)
            .args(["daemon", "start", "--foreground"])
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .spawn()
            .context("failed to spawn background daemon")?;
        crate::output::success(&format!(
            "daemon started in background (pid {})",
            child.id()
        ));
        return Ok(());
    }

    let sock = socket_path()?;
    let pid_file = pid_path()?;

    // Clean up stale socket
    if sock.exists() {
        fs::remove_file(&sock).ok();
    }

    // Ensure parent directory exists
    if let Some(parent) = sock.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write PID
    fs::write(&pid_file, process::id().to_string())?;

    crate::output::success(&format!("daemon started (pid {})", process::id()));
    crate::output::info(&format!("socket: {}", sock.display()));

    // Run the server loop
    run_server(&sock)
}

/// Stop the daemon.
pub fn stop() -> Result<()> {
    let pid_file = pid_path()?;
    let sock = socket_path()?;

    if pid_file.exists() {
        let pid_str = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Send SIGTERM
            #[cfg(unix)]
            {
                let _ = process::Command::new("kill").arg(pid.to_string()).status();
            }
        }
        fs::remove_file(&pid_file).ok();
    }

    if sock.exists() {
        fs::remove_file(&sock).ok();
    }

    crate::output::success("daemon stopped");
    Ok(())
}

/// Get daemon status.
pub fn status() -> Result<()> {
    if is_running() {
        let pid_file = pid_path()?;
        let pid = fs::read_to_string(&pid_file).unwrap_or_default();
        let sock = socket_path()?;
        crate::output::success(&format!("daemon is running (pid {})", pid.trim()));
        crate::output::info(&format!("socket: {}", sock.display()));
    } else {
        crate::output::info("daemon is not running");
    }
    Ok(())
}

/// Send a request to the running daemon.
#[cfg(unix)]
pub fn send_request(request: &DaemonRequest) -> Result<DaemonResponse> {
    let sock = socket_path()?;
    let mut stream =
        UnixStream::connect(&sock).context("could not connect to daemon — is it running?")?;

    let json = serde_json::to_string(request)?;
    writeln!(stream, "{json}")?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    serde_json::from_str(&response_line).context("invalid response from daemon")
}

#[cfg(not(unix))]
pub fn send_request(_request: &DaemonRequest) -> Result<DaemonResponse> {
    anyhow::bail!("daemon is only supported on Unix systems");
}

/// The main server loop — listens on a Unix socket and handles requests.
#[cfg(unix)]
fn run_server(sock_path: &Path) -> Result<()> {
    let listener = UnixListener::bind(sock_path).context("failed to bind daemon socket")?;

    // Load initial state
    let config = crate::config::load().unwrap_or_default();
    let workspace = crate::workspace::resolve_workspace().ok();

    let state = std::sync::Arc::new(std::sync::Mutex::new(DaemonState {
        config,
        workspace,
        pid: process::id(),
        started_at: std::time::Instant::now(),
    }));

    crate::output::info("daemon listening for connections...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = std::sync::Arc::clone(&state);
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, &state) {
                        eprintln!("rxd: client error: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("rxd: accept error: {e}");
            }
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn run_server(_sock_path: &Path) -> Result<()> {
    anyhow::bail!("daemon is only supported on Unix systems");
}

/// Handle a single client connection.
#[cfg(unix)]
fn handle_client(
    stream: UnixStream,
    state: &std::sync::Arc<std::sync::Mutex<DaemonState>>,
) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let request: DaemonRequest = serde_json::from_str(&line)?;
    let start = std::time::Instant::now();

    let response = match request.command.as_str() {
        "ping" => DaemonResponse {
            success: true,
            output: "pong".into(),
            duration_ms: 0,
        },
        "status" => {
            let s = state.lock().unwrap();
            let uptime = s.started_at.elapsed();
            let has_ws = s.workspace.is_some();
            DaemonResponse {
                success: true,
                output: format!(
                    "uptime: {:.0}s, workspace: {}, pid: {}",
                    uptime.as_secs_f64(),
                    if has_ws { "loaded" } else { "none" },
                    s.pid,
                ),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        "reload" => {
            let mut s = state.lock().unwrap();
            s.config = crate::config::load().unwrap_or_default();
            s.workspace = crate::workspace::resolve_workspace().ok();
            DaemonResponse {
                success: true,
                output: "config and workspace reloaded".into(),
                duration_ms: start.elapsed().as_millis() as u64,
            }
        }
        "fingerprint" => {
            let cwd = PathBuf::from(&request.cwd);
            let profile = request.args.first().map(|s| s.as_str()).unwrap_or("debug");
            match crate::cache::compute_build_fingerprint(&cwd, profile, None) {
                Ok(fp) => DaemonResponse {
                    success: true,
                    output: fp,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(e) => DaemonResponse {
                    success: false,
                    output: format!("{e:#}"),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            }
        }
        _ => DaemonResponse {
            success: false,
            output: format!("unknown command: {}", request.command),
            duration_ms: start.elapsed().as_millis() as u64,
        },
    };

    let json = serde_json::to_string(&response)?;
    let mut writer = stream;
    writeln!(writer, "{json}")?;
    writer.flush()?;

    Ok(())
}
