use super::*;

#[test]
fn test_get_system_context() {
    let ctx = get_system_context().unwrap();
    assert_ne!(ctx["os"], serde_json::Value::Null);
    assert!(ctx["cpu_cores"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_execute_command_success() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().await;
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "100");
    let res = execute_command("echo hello", None, None).await.unwrap();
    assert_eq!(res["exit_code"], 0);
    assert!(res["stdout"].as_str().unwrap().contains("hello"));
}

#[tokio::test]
async fn test_execute_command_captures_stderr_after_stdout_close() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().await;
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "100");
    let res = execute_command("echo hello; echo error_msg >&2", None, None)
        .await
        .unwrap();
    assert_eq!(res["exit_code"], 0);
    assert!(res["stdout"].as_str().unwrap().contains("hello"));
    assert!(res["stderr"].as_str().unwrap().contains("error_msg"));
}

#[tokio::test]
async fn test_execute_command_timeout() {
    // A 1s timeout against a longer sleep should time out quickly rather
    // than waiting out the default.
    let err = execute_command("sleep 5", None, Some(1)).await.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("timed out after 1s"));
    assert!(message.contains("timeout_seconds"));
}

#[tokio::test]
async fn test_execute_command_uses_explicit_working_directory() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().await;
    std::env::remove_var("AI_WORKSPACE_ROOT");
    let dir = tempfile::tempdir().unwrap();
    let canonical = dir.path().canonicalize().unwrap();

    let res = execute_command("pwd", canonical.to_str(), None)
        .await
        .unwrap();
    assert_eq!(
        res["stdout"].as_str().unwrap().trim(),
        canonical.to_str().unwrap()
    );
}

#[tokio::test]
async fn test_execute_command_defaults_to_workspace_root() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let canonical = dir.path().canonicalize().unwrap();
    std::env::set_var("AI_WORKSPACE_ROOT", &canonical);

    let res = execute_command("pwd", None, None).await.unwrap();
    assert_eq!(
        res["stdout"].as_str().unwrap().trim(),
        canonical.to_str().unwrap()
    );

    std::env::remove_var("AI_WORKSPACE_ROOT");
}

#[test]
fn test_resolve_timeout_defaults_and_clamps() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    std::env::remove_var("AI_COMMAND_TIMEOUT_SECONDS");
    assert_eq!(
        resolve_timeout(None),
        Duration::from_secs(DEFAULT_TIMEOUT_SECS)
    );
    assert_eq!(resolve_timeout(Some(5)), Duration::from_secs(5));
    assert_eq!(
        resolve_timeout(Some(9999)),
        Duration::from_secs(MAX_TIMEOUT_SECS)
    );
    assert_eq!(resolve_timeout(Some(0)), Duration::from_secs(1));

    std::env::set_var("AI_COMMAND_TIMEOUT_SECONDS", "60");
    assert_eq!(resolve_timeout(None), Duration::from_mins(1));
    std::env::remove_var("AI_COMMAND_TIMEOUT_SECONDS");
}
