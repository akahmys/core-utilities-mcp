use super::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_read_file_within_limit() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "50");
    let res = read_file_with_limit(file_path.to_str().unwrap(), None, None).unwrap();
    assert_eq!(res.content, "hello\nworld\n");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_offset, None);
}

#[test]
fn test_read_file_with_offset_and_truncation() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "4");
    let res = read_file_with_limit(file_path.to_str().unwrap(), Some(6), None).unwrap();
    assert_eq!(res.content, "worl");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_offset, Some(10));
}

#[test]
fn test_filter_and_sort_matrix_columns() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
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
        vec!["name".to_string(), "id".to_string()],
        Some(true),
    )
    .unwrap();
    assert!(res.content.contains("name,id"));
    assert!(res.content.contains("Alice,1"));
    assert!(res.content.contains("Bob,2"));
}

#[test]
fn test_extract_code_skeleton() {
    let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("code.rs");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "pub struct User {{").unwrap();
    writeln!(file, "    pub name: String,").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file, "impl User {{").unwrap();
    writeln!(file, "    pub fn new() -> Self {{").unwrap();
    writeln!(file, "        User {{ name: String::new() }}").unwrap();
    writeln!(file, "    }}").unwrap();
    writeln!(file, "}}").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
    let res = extract_code_skeleton(file_path.to_str().unwrap()).unwrap();
    assert!(res.content.contains("pub struct User"));
    assert!(res.content.contains("impl User"));
    assert!(!res.content.contains("pub name: String"));
}

#[test]
fn test_query_json_by_path() {
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
    let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("matrix.csv");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "id,name,age").unwrap();
    writeln!(file, "1,Alice,30").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
    assert!(matches!(
        filter_and_sort_matrix_columns(
            file_path.to_str().unwrap(),
            vec!["nonexistent".to_string()],
            None,
        ),
        Err(CoreError::General(_))
    ));
}

#[test]
fn test_query_json_by_path_missing_path_returns_general_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, r#"{{"data": {{}}}}"#).unwrap();

    assert!(matches!(
        query_json_by_path(file_path.to_str().unwrap(), "data.missing"),
        Err(CoreError::General(_))
    ));
}

#[test]
fn test_query_json_by_path_invalid_json_returns_parsing_error() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bad.json");
    std::fs::write(&file_path, b"not json").unwrap();

    assert!(matches!(
        query_json_by_path(file_path.to_str().unwrap(), "data"),
        Err(CoreError::Parsing(_))
    ));
}
