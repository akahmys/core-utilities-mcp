use super::*;
use crate::errors::CoreError;
use tempfile::tempdir;

#[test]
fn test_copy_move_create_dir() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src_dir");
    let dest_dir = dir.path().join("dest_dir");

    assert!(create_directory(src_dir.to_str().unwrap()).is_ok());
    let test_file = src_dir.join("test.txt");
    std::fs::write(&test_file, b"data").unwrap();

    assert!(copy_file_or_directory(src_dir.to_str().unwrap(), dest_dir.to_str().unwrap()).is_ok());
    assert!(dest_dir.join("test.txt").exists());

    let moved_dir = dir.path().join("moved_dir");
    assert!(
        move_file_or_directory(dest_dir.to_str().unwrap(), moved_dir.to_str().unwrap()).is_ok()
    );
    assert!(!dest_dir.exists());
    assert!(moved_dir.join("test.txt").exists());
}

#[test]
fn test_mutations_on_nonexistent_targets_return_file_errors() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.txt");
    let missing_str = missing.to_str().unwrap();
    let dest = dir.path().join("dest.txt");
    let dest_str = dest.to_str().unwrap();

    assert!(matches!(
        copy_file_or_directory(missing_str, dest_str),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        move_file_or_directory(missing_str, dest_str),
        Err(CoreError::File(_))
    ));
}
