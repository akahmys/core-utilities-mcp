//! System introspection and sandboxed shell command execution.

use crate::guardrails::truncate_output;
use crate::{CoreError, CoreResult};
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Timeout applied when a call doesn't specify `timeout_seconds` and
/// `AI_COMMAND_TIMEOUT_SECONDS` is unset.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Hard ceiling a requested timeout is clamped to, regardless of source.
const MAX_TIMEOUT_SECS: u64 = 300;

/// Resolves the wall-clock timeout for a command: an explicit per-call
/// value wins, falling back to `AI_COMMAND_TIMEOUT_SECONDS`, then
/// [`DEFAULT_TIMEOUT_SECS`]. Always clamped to `1..=`[`MAX_TIMEOUT_SECS`].
fn resolve_timeout(requested: Option<u64>) -> Duration {
    let env_default = std::env::var("AI_COMMAND_TIMEOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    let secs = requested.unwrap_or(env_default).clamp(1, MAX_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Builds the (unspawned) shell invocation for `command`, using `cmd /C` on
/// Windows and `sh -c` elsewhere, applying `working_directory` if given.
fn build_command(command: &str, working_directory: Option<&str>) -> Command {
    #[cfg(target_family = "windows")]
    let mut builder = {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    };

    #[cfg(not(target_family = "windows"))]
    let mut builder = {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };

    if let Some(dir) = working_directory {
        builder.current_dir(dir);
    }
    builder.stdout(Stdio::piped()).stderr(Stdio::piped());
    builder
}

/// Aggregates OS name, hostname, CPU core count, total free disk space
/// across all mounted disks, and the current process's user/group ID
/// (Unix only; `"N/A"` elsewhere) into a single JSON object.
///
/// # Errors
/// Returns [`CoreError::System`] if the current process ID cannot be
/// determined.
///
/// # Examples
///
/// ```
/// use core_utilities_mcp_lib::sys_ops::get_system_context;
///
/// let ctx = get_system_context()?;
/// println!("running on {}", ctx["os"]);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn get_system_context() -> CoreResult<Value> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = sys.cpus().len();
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());

    let disks = Disks::new_with_refreshed_list();
    let mut disk_free_bytes: u64 = 0;
    for disk in &disks {
        disk_free_bytes += disk.available_space();
    }

    let current_pid = sysinfo::get_current_pid().map_err(|e| CoreError::System(e.to_string()))?;

    let mut uid = None;
    let mut gid = None;

    if let Some(proc) = sys.process(current_pid) {
        #[cfg(target_family = "unix")]
        {
            if let Some(u) = proc.user_id() {
                uid = Some(u.to_string());
            }
            if let Some(g) = proc.group_id() {
                gid = Some(g.to_string());
            }
        }
    }

    Ok(json!({
        "os": os_name,
        "cpu_cores": cpu_cores,
        "hostname": hostname,
        "disk_free_bytes": disk_free_bytes,
        "uid": uid.unwrap_or_else(|| "N/A".to_string()),
        "gid": gid.unwrap_or_else(|| "N/A".to_string())
    }))
}

/// Runs `command` in a shell (`sh -c` on Unix, `cmd /C` on Windows) with two
/// safety constraints: a wall-clock timeout (`timeout_seconds`, falling back
/// to `AI_COMMAND_TIMEOUT_SECONDS`, then 30s, always clamped to at most 5
/// minutes), and a hard kill if buffered stdout exceeds 4x the
/// `AI_COMMAND_MAX_CHARACTERS` byte budget (default `8192`, so a 32KB
/// safeguard). Returned `stdout` is further truncated to the configured
/// character limit via [`truncate_output`](crate::guardrails::truncate_output).
///
/// `working_directory`, if given, is used as-is (not path-validated); if
/// omitted, `AI_WORKSPACE_ROOT` is used as the default when set, otherwise
/// the server process's own current directory. Neither the command string
/// nor the resolved working directory go through
/// [`validate_path_safety`](crate::guardrails::validate_path_safety) — this
/// function does not impose CPU or memory limits either, and does not
/// restrict the command's filesystem or network access. Callers requiring
/// stronger isolation should run this behind an OS-level sandbox (container,
/// VM, seccomp).
///
/// # Errors
/// Returns [`CoreError::Process`] if the command cannot be spawned or its
/// exit status cannot be read, or [`CoreError::Command`] if it exceeds the
/// timeout.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::sys_ops::execute_command_in_sandbox;
///
/// # async fn run() -> Result<(), core_utilities_mcp_lib::CoreError> {
/// let result = execute_command_in_sandbox("cargo test", Some("/path/to/project"), Some(120)).await?;
/// println!("{}", result["stdout"]);
/// # Ok(())
/// # }
/// ```
pub async fn execute_command_in_sandbox(
    command: &str,
    working_directory: Option<&str>,
    timeout_seconds: Option<u64>,
) -> CoreResult<Value> {
    let limit: usize = std::env::var("AI_COMMAND_MAX_CHARACTERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192);

    let workspace_root = std::env::var("AI_WORKSPACE_ROOT").ok();
    let cwd = working_directory.or(workspace_root.as_deref());

    let mut child = build_command(command, cwd)
        .spawn()
        .map_err(|e| CoreError::Process(format!("Failed to spawn command: {}", e)))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| CoreError::Process("Failed to open stdout pipe".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| CoreError::Process("Failed to open stderr pipe".to_string()))?;

    let timeout_duration = resolve_timeout(timeout_seconds);

    // Futures to read output
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    let child_ref = &mut child;

    let read_fut = async {
        let mut stdout_chunk = [0; 1024];
        let mut stderr_chunk = [0; 1024];
        loop {
            tokio::select! {
                res = stdout.read(&mut stdout_chunk) => {
                    match res {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            stdout_buf.extend_from_slice(&stdout_chunk[..n]);
                            if stdout_buf.len() > limit * 4 { // Hard byte limit safeguard
                                let _ = child_ref.kill().await;
                                break;
                            }
                        }
                    }
                }
                res = stderr.read(&mut stderr_chunk) => {
                    match res {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            stderr_buf.extend_from_slice(&stderr_chunk[..n]);
                        }
                    }
                }
            }
        }
    };

    tokio::select! {
        _ = tokio::time::sleep(timeout_duration) => {
            let _ = child.kill().await;
            Err(CoreError::Command("Command execution timed out".to_string()))
        }
        _ = read_fut => {
            let status = child.wait().await.map_err(|e| CoreError::Process(format!("Failed to wait for child: {}", e)))?;

            let stdout_str = String::from_utf8_lossy(&stdout_buf).into_owned();
            let stderr_str = String::from_utf8_lossy(&stderr_buf).into_owned();

            let truncated_stdout = truncate_output(&stdout_str);

            Ok(json!({
                "exit_code": status.code().unwrap_or(-1),
                "stdout": truncated_stdout.content,
                "status": truncated_stdout.status,
                "next_offset": truncated_stdout.next_offset,
                "stderr": stderr_str
            }))
        }
    }
}

#[cfg(test)]
mod tests;
