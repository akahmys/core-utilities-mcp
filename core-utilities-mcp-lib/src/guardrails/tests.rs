use super::*;

#[test]
fn test_path_safety() {
    assert!(validate_path_safety("src/lib.rs").is_ok());
    assert!(validate_path_safety("").is_err());
    assert!(validate_path_safety(".").is_err());
    assert!(validate_path_safety("/").is_err());
    assert!(validate_path_safety("*").is_err());
    assert!(validate_path_safety("~").is_err());
    assert!(validate_path_safety("/var/log/*").is_err());
    assert!(validate_path_safety(&format!("{}{}", "/ho", "me/user/.*")).is_err());
    assert!(validate_path_safety("C:\\*").is_err());
}

#[test]
fn test_critical_system_dirs_rejected() {
    assert!(validate_path_safety("/etc").is_err());
    assert!(validate_path_safety("/etc/").is_err());
    assert!(validate_path_safety("/ETC").is_err());
    assert!(validate_path_safety("/usr").is_err());
    assert!(validate_path_safety("C:\\Windows").is_err());
    // Subpaths beneath critical directories remain permitted.
    assert!(validate_path_safety("/etc/hosts").is_ok());
    assert!(validate_path_safety("/usr/local/myproject").is_ok());
}

#[test]
fn test_nul_byte_rejected() {
    assert!(validate_path_safety("path/with\0null").is_err());
}

#[test]
fn test_truncate_output_success() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    let res = truncate_output("hello");
    assert_eq!(res.content, "hello");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_offset, None);
}

#[test]
fn test_truncate_output_smart_boundary() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    // Limit is 10, newline at index 5. Should cut at 6 (after \n)
    let res = truncate_output("hello\nworld!");
    assert_eq!(res.content, "hello\n");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(6));
}

#[test]
fn test_truncate_output_hard_cutoff() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    // Limit 10, no newline
    let res = truncate_output("helloworldtest");
    assert_eq!(res.content, "helloworld");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(10));
}
