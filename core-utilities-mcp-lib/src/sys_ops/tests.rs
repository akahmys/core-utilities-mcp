use super::*;

#[test]
fn test_get_system_context() {
    let ctx = get_system_context().unwrap();
    assert!(ctx["os"] != serde_json::Value::Null);
    assert!(ctx["cpu_cores"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_execute_command_success() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().await;
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
