use super::*;

#[test]
fn test_path_safety() {
    let _lock = ENV_MUTEX.blocking_lock();
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
fn test_path_safety_rejects_current_dir_spellings() {
    let _lock = ENV_MUTEX.blocking_lock();
    // Lexical spellings that all collapse to "here", not just the literal ".".
    assert!(validate_path_safety("./").is_err());
    assert!(validate_path_safety("././").is_err());
    assert!(validate_path_safety("a/..").is_err());
    // ".." on its own is a legitimate parent-directory reference, not "here".
    assert!(validate_path_safety("..").is_ok());
}

#[test]
fn test_read_path_safety_permits_current_dir() {
    let _lock = ENV_MUTEX.blocking_lock();
    assert!(validate_read_path_safety(".").is_ok());
    assert!(validate_read_path_safety("./").is_ok());
    assert!(validate_read_path_safety("./src").is_ok());
    assert!(validate_read_path_safety("src/lib.rs").is_ok());
    // Other dangerous patterns are still rejected for read-only operations.
    assert!(validate_read_path_safety("").is_err());
    assert!(validate_read_path_safety("/").is_err());
    assert!(validate_read_path_safety("*").is_err());
    assert!(validate_read_path_safety("~").is_err());
    assert!(validate_read_path_safety("/etc").is_err());
    assert!(validate_read_path_safety("/etc/hosts").is_ok());
    assert!(validate_read_path_safety("path/with\0null").is_err());
}

#[test]
fn test_critical_system_dirs_rejected() {
    let _lock = ENV_MUTEX.blocking_lock();
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
    let _lock = ENV_MUTEX.blocking_lock();
    assert!(validate_path_safety("path/with\0null").is_err());
}

#[test]
fn test_workspace_root_confinement() {
    let _lock = ENV_MUTEX.blocking_lock();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::env::set_var("AI_WORKSPACE_ROOT", &root);

    let inside = root.join("inside.txt");
    assert!(validate_path_safety(inside.to_str().unwrap()).is_ok());
    assert!(validate_path_safety("/etc/hosts").is_err());
    assert!(validate_path_safety("relative/inside/workspace.txt").is_err());

    std::env::remove_var("AI_WORKSPACE_ROOT");
}

#[test]
fn test_workspace_root_blocks_parent_traversal() {
    let _lock = ENV_MUTEX.blocking_lock();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("subdir")).unwrap();
    std::env::set_var("AI_WORKSPACE_ROOT", &root);

    let escaping = root
        .join("subdir")
        .join("..")
        .join("..")
        .join("outside.txt");
    assert!(validate_path_safety(escaping.to_str().unwrap()).is_err());

    std::env::remove_var("AI_WORKSPACE_ROOT");
}

#[test]
fn test_workspace_root_unset_means_unrestricted() {
    let _lock = ENV_MUTEX.blocking_lock();
    std::env::remove_var("AI_WORKSPACE_ROOT");
    assert!(validate_path_safety("/any/path/at/all").is_ok());
}

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
