use super::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_search_text() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("search.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "magic keyword found").unwrap();
    writeln!(file, "ordinary line").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "2000");
    let res = search_text_with_limit(dir.path().to_str().unwrap(), "magic", Some(false)).unwrap();
    assert!(res.content.contains("magic keyword found"));
    assert!(!res.content.contains("ordinary line"));
}

#[test]
fn test_search_file_by_name() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    File::create(dir.path().join("target_file.rs")).unwrap();
    File::create(dir.path().join("ignore.txt")).unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "2000");
    let res = search_file_by_name_or_type(
        Some(dir.path().to_str().unwrap()),
        Some(r"\.rs$"),
        Some("file"),
    )
    .unwrap();
    assert!(res.content.contains("target_file.rs"));
    assert!(!res.content.contains("ignore.txt"));
}

#[test]
fn test_invalid_regex_returns_parsing_error() {
    let dir = tempdir().unwrap();

    assert!(matches!(
        search_text_with_limit(dir.path().to_str().unwrap(), "(unclosed", Some(true)),
        Err(CoreError::Parsing(_))
    ));
    assert!(matches!(
        search_file_by_name_or_type(Some(dir.path().to_str().unwrap()), Some("(unclosed"), None),
        Err(CoreError::Parsing(_))
    ));
}

#[test]
fn test_nonexistent_search_root_returns_file_error() {
    assert!(matches!(
        search_text_with_limit("/does/not/exist", "query", None),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        search_file_by_name_or_type(Some("/does/not/exist"), None, None),
        Err(CoreError::File(_))
    ));
}
