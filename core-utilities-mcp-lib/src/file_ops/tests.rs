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
fn test_operations_on_nonexistent_targets_return_file_errors() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.txt");
    let missing_str = missing.to_str().unwrap();
    let dest = dir.path().join("dest.txt");
    let dest_str = dest.to_str().unwrap();

    assert!(matches!(
        delete_file_or_directory(missing_str),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        get_file_metadata(missing_str),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        copy_file_or_directory(missing_str, dest_str),
        Err(CoreError::File(_))
    ));
    assert!(matches!(
        move_file_or_directory(missing_str, dest_str),
        Err(CoreError::File(_))
    ));
}
