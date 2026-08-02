use super::*;
use crate::errors::CoreError;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

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
