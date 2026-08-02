use super::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_read_file_annotates_line_numbers() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    // `writeln!` adds a trailing newline, so `split('\n')` (the same model
    // `edit_file` uses) sees a phantom trailing empty line 3 — kept
    // consistent with `edit_file` rather than hidden, so line numbers shown
    // here match what `edit_file` expects.
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "50");
    let res = read_file(file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(res.content, "1\thello\n2\tworld\n3\t\n");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_start_line, None);
}

#[test]
fn test_read_file_from_start_line() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "50");
    let res = read_file(file_path.to_str().unwrap(), Some(2)).unwrap();
    assert_eq!(res.content, "2\tworld\n3\t\n");
    assert_eq!(res.status, "success");
    assert_eq!(res.next_start_line, None);
}

#[test]
fn test_read_file_truncates_and_reports_next_start_line() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello").unwrap();
    writeln!(file, "world").unwrap();
    writeln!(file, "third").unwrap();

    // "1\thello\n" is 8 chars; a limit of 10 fits line 1 but not line 2.
    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    let res = read_file(file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(res.content, "1\thello\n");
    assert_eq!(res.status, "truncated");
    assert_eq!(res.next_start_line, Some(2));

    let page2 = read_file(file_path.to_str().unwrap(), res.next_start_line).unwrap();
    assert_eq!(page2.content, "2\tworld\n");
    assert_eq!(page2.status, "truncated");
    assert_eq!(page2.next_start_line, Some(3));
}

#[test]
fn test_read_file_oversized_single_line_still_makes_progress() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_read.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "{}", "x".repeat(50)).unwrap();
    writeln!(file, "short").unwrap();

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
    let res = read_file(file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(res.status, "truncated");
    // Even though line 1 alone exceeds the limit, next_start_line must
    // still advance so a caller retrying doesn't loop forever.
    assert_eq!(res.next_start_line, Some(2));
}

#[test]
fn test_read_file_bounds_accumulation_for_huge_files() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("huge.txt");
    let mut file = File::create(&file_path).unwrap();
    // Far more lines than the limit could ever include in one window.
    for i in 0..200_000 {
        writeln!(file, "line {i}").unwrap();
    }

    std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "100");
    let res = read_file(file_path.to_str().unwrap(), None).unwrap();
    assert_eq!(res.status, "truncated");
    assert!(res.content.len() <= 100 + READ_WINDOW_MARGIN + 32);
    assert!(res.next_start_line.unwrap() < 200_000);
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
fn test_query_data_by_path() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        r#"{{"data": {{"users": [{{"id": 42, "name": "Alice"}}]}}}}"#
    )
    .unwrap();

    let val = query_data_by_path(file_path.to_str().unwrap(), "data.users[0].id").unwrap();
    assert_eq!(val, serde_json::Value::from(42));
}

#[test]
fn test_query_data_by_path_toml() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.toml");
    std::fs::write(
        &file_path,
        "[package]\nname = \"demo\"\n\n[[package.authors]]\nname = \"Alice\"\n",
    )
    .unwrap();

    let val = query_data_by_path(file_path.to_str().unwrap(), "package.name").unwrap();
    assert_eq!(val, serde_json::Value::from("demo"));

    let author =
        query_data_by_path(file_path.to_str().unwrap(), "package.authors[0].name").unwrap();
    assert_eq!(author, serde_json::Value::from("Alice"));
}

#[test]
fn test_query_data_by_path_yaml() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("compose.yml");
    std::fs::write(
        &file_path,
        "services:\n  web:\n    image: nginx\n    ports:\n      - \"80:80\"\n",
    )
    .unwrap();

    let image = query_data_by_path(file_path.to_str().unwrap(), "services.web.image").unwrap();
    assert_eq!(image, serde_json::Value::from("nginx"));

    let port = query_data_by_path(file_path.to_str().unwrap(), "services.web.ports[0]").unwrap();
    assert_eq!(port, serde_json::Value::from("80:80"));
}

#[test]
fn test_query_data_by_path_yaml_extension_variant() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.yaml");
    std::fs::write(&file_path, "name: Bob\n").unwrap();

    let val = query_data_by_path(file_path.to_str().unwrap(), "name").unwrap();
    assert_eq!(val, serde_json::Value::from("Bob"));
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
fn test_query_data_by_path_missing_path_returns_general_error() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, r#"{{"data": {{}}}}"#).unwrap();

    let err = query_data_by_path(file_path.to_str().unwrap(), "data.missing").unwrap_err();
    assert!(matches!(err, CoreError::General(_)));
    let message = err.to_string();
    assert!(message.contains("stopped at '/data'"));
    assert!(message.contains("an object with keys"));
}

#[test]
fn test_query_data_by_path_invalid_json_returns_parsing_error() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bad.json");
    std::fs::write(&file_path, b"not json").unwrap();

    assert!(matches!(
        query_data_by_path(file_path.to_str().unwrap(), "data"),
        Err(CoreError::Parsing(_))
    ));
}
