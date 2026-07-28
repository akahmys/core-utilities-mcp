use super::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_read_file_within_limit() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "50");
    let res = read_file_with_limit(file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(res.content, "hello\nworld\n");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_offset, None);
}

#[test]
fn test_read_file_with_offset_and_truncation() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "4");
    let res = read_file_with_limit(file_path.to_str().unwrap(), Some(6)).unwrap();
    assert_eq!(res.content, "worl");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(10));
}

#[test]
fn test_filter_and_sort_matrix_columns() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("matrix.csv");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "id,name,age").unwrap();
    writeln!(file, "1,Alice,30").unwrap();
    writeln!(file, "2,Bob,25").unwrap();
    writeln!(file, "1,Alice,30").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
    let res = filter_and_sort_matrix_columns(
        file_path.to_str().unwrap(),
        &["name".to_string(), "id".to_string()],
        Some(true),
    )
    .unwrap();
    assert!(res.content.contains("name,id"));
    assert!(res.content.contains("Alice,1"));
    assert!(res.content.contains("Bob,2"));
}

#[test]
fn test_query_json_by_path() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        r#"{{"data": {{"users": [{{"id": 42, "name": "Alice"}}]}}}}"#
    )
    .unwrap();

    let val = query_json_by_path(file_path.to_str().unwrap(), "data.users[0].id").unwrap();
    assert_eq!(val, serde_json::Value::from(42));
}

#[test]
fn test_filter_and_sort_matrix_columns_no_matching_columns() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("matrix.csv");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "id,name,age").unwrap();
    writeln!(file, "1,Alice,30").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
    let err = filter_and_sort_matrix_columns(
        file_path.to_str().unwrap(),
        &["nonexistent".to_string()],
        None,
    )
    .unwrap_err();
    assert!(matches!(err, CoreError::General(_)));
    let message = err.to_string();
    assert!(message.contains("nonexistent"));
    assert!(message.contains("\"id\""));
    assert!(message.contains("\"name\""));
    assert!(message.contains("\"age\""));
}

#[test]
fn test_query_json_by_path_missing_path_returns_general_error() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, r#"{{"data": {{}}}}"#).unwrap();

    let err = query_json_by_path(file_path.to_str().unwrap(), "data.missing").unwrap_err();
    assert!(matches!(err, CoreError::General(_)));
    let message = err.to_string();
    assert!(message.contains("stopped at '/data'"));
    assert!(message.contains("an object with keys"));
}

#[test]
fn test_query_json_by_path_invalid_json_returns_parsing_error() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bad.json");
    std::fs::write(&file_path, b"not json").unwrap();

    assert!(matches!(
        query_json_by_path(file_path.to_str().unwrap(), "data"),
        Err(CoreError::Parsing(_))
    ));
}
