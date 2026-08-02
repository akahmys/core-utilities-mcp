use super::*;
use crate::guardrails::ENV_MUTEX;

#[test]
fn test_truncate_output_success() {
    let _lock = ENV_MUTEX.blocking_lock();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    let res = truncate_output("hello");
    assert_eq!(res.content, "hello");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_offset, None);
}

#[test]
fn test_truncate_output_smart_boundary() {
    let _lock = ENV_MUTEX.blocking_lock();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    // Limit is 10, newline at index 5. Should cut at 6 (after \n)
    let res = truncate_output("hello\nworld!");
    assert_eq!(res.content, "hello\n");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(6));
}

#[test]
fn test_truncate_output_hard_cutoff() {
    let _lock = ENV_MUTEX.blocking_lock();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    // Limit 10, no newline
    let res = truncate_output("helloworldtest");
    assert_eq!(res.content, "helloworld");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(10));
}
