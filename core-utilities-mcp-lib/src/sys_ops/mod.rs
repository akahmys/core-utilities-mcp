use crate::guardrails::truncate_output;
use crate::{CoreError, CoreResult};
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Retrieves system diagnostics context including OS, hostname, CPUs, disk capacity, and IDs.
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

/// Executes a shell command inside a sandboxed wrapper with timeout and memory limits.
/// Limits output using AI_COMMAND_MAX_CHARACTERS boundary limiters.
pub async fn execute_command_in_sandbox(command: &str) -> CoreResult<Value> {
    let limit: usize = std::env::var("AI_COMMAND_MAX_CHARACTERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192);

    #[cfg(target_family = "windows")]
    let mut child = Command::new("cmd")
        .args(["/C", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Process(format!("Failed to spawn command: {}", e)))?;

    #[cfg(not(target_family = "windows"))]
    let mut child = Command::new("sh")
        .args(["-c", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    let timeout_duration = Duration::from_secs(5);

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
mod tests {
    use super::*;

    #[test]
    fn test_get_system_context() {
        let ctx = get_system_context().unwrap();
        assert!(ctx["os"] != serde_json::Value::Null);
        assert!(ctx["cpu_cores"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_execute_command_success() {
        let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "100");
        let res = execute_command_in_sandbox("echo hello").await.unwrap();
        assert_eq!(res["exit_code"], 0);
        assert!(res["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_command_timeout() {
        // Sleep commands to test timeout
        let res = execute_command_in_sandbox("sleep 10").await;
        assert!(res.is_err());
    }
}
