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
fn test_edit_file_content_success() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_edit.txt");
    std::fs::write(&file_path, b"line 1\nline 2\nline 3\nline 4").unwrap();

    let path_str = file_path.to_str().unwrap();

    // Test editing lines 2 to 3
    let res =
        edit_file_content(path_str, 2, 3, "line 2\nline 3", "new line 2\nnew line 3").unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["lines_modified"], 2);

    let updated_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated_content, "line 1\nnew line 2\nnew line 3\nline 4");

    // Test oob range error
    assert!(edit_file_content(path_str, 2, 5, "anything", "anything").is_err());

    // Test content mismatch error
    assert!(edit_file_content(path_str, 2, 3, "wrong target", "anything").is_err());
}

#[test]
fn test_write_file_creates_new_file_and_parents() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("nested").join("new.txt");
    let path_str = file_path.to_str().unwrap();

    let res = write_file(path_str, "hello world", None).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["bytes_written"], 11);
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello world");
}

#[test]
fn test_write_file_refuses_overwrite_without_flag() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("existing.txt");
    std::fs::write(&file_path, "original").unwrap();
    let path_str = file_path.to_str().unwrap();

    assert!(matches!(
        write_file(path_str, "clobbered", None),
        Err(CoreError::File(_))
    ));
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "original");

    assert!(write_file(path_str, "clobbered", Some(true)).is_ok());
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "clobbered");
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
