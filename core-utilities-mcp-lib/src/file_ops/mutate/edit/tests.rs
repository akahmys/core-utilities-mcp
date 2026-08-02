use super::*;
use crate::errors::CoreError;
use tempfile::tempdir;

#[test]
fn test_edit_file_single_chunk_success_with_normalization() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_edit.txt");
    std::fs::write(&file_path, b"line 1  \r\nline 2\r\nline 3\nline 4").unwrap();

    let path_str = file_path.to_str().unwrap();

    // Test editing lines 2 to 3 with LF vs CRLF and trailing whitespace normalization
    let res = edit_file(
        path_str,
        vec![EditChunk {
            start_line: 2,
            end_line: 3,
            target_content: "line 2\nline 3".to_string(),
            replacement_content: "new line 2\nnew line 3".to_string(),
        }],
    )
    .unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["line_delta"], 0);

    let updated_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated_content, "line 1  \nnew line 2\nnew line 3\nline 4");

    // Test oob range error
    assert!(edit_file(
        path_str,
        vec![EditChunk {
            start_line: 2,
            end_line: 5,
            target_content: "anything".to_string(),
            replacement_content: "anything".to_string(),
        }],
    )
    .is_err());

    // Test content mismatch error
    assert!(edit_file(
        path_str,
        vec![EditChunk {
            start_line: 2,
            end_line: 3,
            target_content: "wrong target".to_string(),
            replacement_content: "anything".to_string(),
        }],
    )
    .is_err());
}

#[test]
fn test_edit_file_multi_chunk_success_and_rollback() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_multi.txt");
    let original = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6";
    std::fs::write(&file_path, original).unwrap();
    let path_str = file_path.to_str().unwrap();

    let edits = vec![
        EditChunk {
            start_line: 2,
            end_line: 2,
            target_content: "line 2".to_string(),
            replacement_content: "CHANGED 2".to_string(),
        },
        EditChunk {
            start_line: 5,
            end_line: 5,
            target_content: "line 5".to_string(),
            replacement_content: "CHANGED 5".to_string(),
        },
    ];
    let res = edit_file(path_str, edits).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["chunks_applied"], 2);
    assert_eq!(res["line_delta"], 0);

    let updated = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(
        updated,
        "line 1\nCHANGED 2\nline 3\nline 4\nCHANGED 5\nline 6"
    );

    // Rollback when one chunk fails verification
    let failing_edits = vec![
        EditChunk {
            start_line: 1,
            end_line: 1,
            target_content: "line 1".to_string(),
            replacement_content: "SHOULD NOT BE APPLIED".to_string(),
        },
        EditChunk {
            start_line: 3,
            end_line: 3,
            target_content: "WRONG TARGET".to_string(),
            replacement_content: "SHOULD NOT BE APPLIED".to_string(),
        },
    ];
    assert!(edit_file(path_str, failing_edits).is_err());
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "line 1\nCHANGED 2\nline 3\nline 4\nCHANGED 5\nline 6"
    );
}

#[test]
fn test_edit_file_line_delta_reflects_real_line_count_change() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_delta.txt");
    std::fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();
    let path_str = file_path.to_str().unwrap();

    // Replace 1 line with a 3-line block: +2 lines overall, even though the
    // replacement is a single EditChunk (regression test for a bug where
    // line counting missed newlines embedded inside replacement_content).
    let res = edit_file(
        path_str,
        vec![EditChunk {
            start_line: 2,
            end_line: 2,
            target_content: "line 2".to_string(),
            replacement_content: "a\nb\nc".to_string(),
        }],
    )
    .unwrap();
    assert_eq!(res["line_delta"], 2);
    assert_eq!(res["new_line_count"], 5);
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "line 1\na\nb\nc\nline 3"
    );
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
