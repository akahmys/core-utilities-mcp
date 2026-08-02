use super::*;
use crate::errors::CoreError;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

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
