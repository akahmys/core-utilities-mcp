use super::*;
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_delete_file_success() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_delete.txt");
    File::create(&file_path).unwrap();
    let path_str = file_path.to_str().unwrap();
    assert!(delete_file_or_directory(path_str).is_ok());
    assert!(!file_path.exists());
}

#[test]
fn test_list_directory_contents() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    File::create(dir.path().join("file.txt")).unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();

    let list = list_directory_contents(Some(dir.path().to_str().unwrap().to_string())).unwrap();
    let files = list["files"].as_array().unwrap();
    let dirs = list["directories"].as_array().unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(dirs.len(), 1);
    assert_eq!(files[0], "file.txt");
    assert_eq!(dirs[0], "subdir");
}

#[test]
fn test_get_file_metadata() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("file.txt");
    File::create(&file_path).unwrap();

    let meta = get_file_metadata(file_path.to_str().unwrap()).unwrap();
    assert_eq!(meta["size_bytes"], 0);
    assert_eq!(meta["is_dir"], false);
    assert_eq!(meta["permissions"], "readwrite");
}

#[test]
fn test_operations_on_nonexistent_targets_return_file_errors() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.txt");
    let missing_str = missing.to_str().unwrap();

    assert!(matches!(
        delete_file_or_directory(missing_str),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        get_file_metadata(missing_str),
        Err(CoreError::File(_))
    ));
}
