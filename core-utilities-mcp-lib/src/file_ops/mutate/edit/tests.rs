use super::*;
use crate::errors::CoreError;
use tempfile::tempdir;

fn chunk(old: &str, new: &str) -> EditChunk {
    EditChunk {
        old_string: old.to_string(),
        new_string: new.to_string(),
    }
}

#[test]
fn test_edit_file_single_chunk_success_across_crlf() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_edit.txt");
    std::fs::write(&file_path, b"line 1\r\nline 2\r\nline 3\nline 4").unwrap();
    let path_str = file_path.to_str().unwrap();

    // old_string is written with LF; the file's CRLF is normalized first,
    // so the match still succeeds.
    let res = edit_file(
        path_str,
        &[chunk("line 2\nline 3", "new line 2\nnew line 3")],
    )
    .unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["line_delta"], 0);
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "line 1\nnew line 2\nnew line 3\nline 4"
    );
}

#[test]
fn test_edit_file_errors_when_old_string_not_found() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_missing.txt");
    std::fs::write(&file_path, "alpha\nbeta\ngamma").unwrap();
    let path_str = file_path.to_str().unwrap();

    let err = edit_file(path_str, &[chunk("not in the file", "x")]).unwrap_err();
    assert!(
        format!("{err}").contains("not found"),
        "error should say the old_string wasn't found, got: {err}"
    );
    // File must be untouched.
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "alpha\nbeta\ngamma"
    );
}

#[test]
fn test_edit_file_errors_when_old_string_is_ambiguous() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_ambiguous.txt");
    std::fs::write(&file_path, "dup\nunique\ndup").unwrap();
    let path_str = file_path.to_str().unwrap();

    let err = edit_file(path_str, &[chunk("dup", "x")]).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("not unique") && msg.contains('2'),
        "error should report the ambiguity and its count, got: {msg}"
    );
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "dup\nunique\ndup"
    );
}

#[test]
fn test_edit_file_disambiguates_with_surrounding_context() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_context.txt");
    std::fs::write(&file_path, "dup\nunique\ndup").unwrap();
    let path_str = file_path.to_str().unwrap();

    // The bare "dup" was ambiguous; including its neighbour makes it unique.
    let res = edit_file(path_str, &[chunk("unique\ndup", "unique\nCHANGED")]).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "dup\nunique\nCHANGED"
    );
}

#[test]
fn test_edit_file_falls_back_to_whitespace_insensitive_match() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_ws.txt");
    // The file has trailing spaces the caller doesn't know about.
    std::fs::write(&file_path, "keep\nfoo   \nbar\t\nkeep2").unwrap();
    let path_str = file_path.to_str().unwrap();

    let res = edit_file(path_str, &[chunk("foo\nbar", "REPLACED")]).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(
        std::fs::read_to_string(&file_path).unwrap(),
        "keep\nREPLACED\nkeep2"
    );
}

#[test]
fn test_edit_file_multi_chunk_success_and_all_or_nothing_rollback() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_multi.txt");
    let original = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6";
    std::fs::write(&file_path, original).unwrap();
    let path_str = file_path.to_str().unwrap();

    let res = edit_file(
        path_str,
        &[chunk("line 2", "CHANGED 2"), chunk("line 5", "CHANGED 5")],
    )
    .unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["chunks_applied"], 2);
    assert_eq!(res["line_delta"], 0);
    let after_success = "line 1\nCHANGED 2\nline 3\nline 4\nCHANGED 5\nline 6";
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), after_success);

    // One bad chunk must prevent the good one from being applied too.
    assert!(edit_file(
        path_str,
        &[
            chunk("line 1", "SHOULD NOT BE APPLIED"),
            chunk("WRONG TARGET", "SHOULD NOT BE APPLIED"),
        ],
    )
    .is_err());
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), after_success);
}

#[test]
fn test_edit_file_applies_edits_out_of_document_order() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_order.txt");
    std::fs::write(&file_path, "a\nb\nc").unwrap();
    let path_str = file_path.to_str().unwrap();

    // Later-in-file edit listed first — resolution sorts by position.
    let res = edit_file(path_str, &[chunk("c", "C"), chunk("a", "A")]).unwrap();
    assert_eq!(res["chunks_applied"], 2);
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "A\nb\nC");
}

#[test]
fn test_edit_file_rejects_overlapping_matches() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_overlap.txt");
    std::fs::write(&file_path, "abcdef").unwrap();
    let path_str = file_path.to_str().unwrap();

    let err = edit_file(path_str, &[chunk("abcd", "X"), chunk("cdef", "Y")]).unwrap_err();
    assert!(
        format!("{err}").contains("overlapping"),
        "error should call out the overlap, got: {err}"
    );
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "abcdef");
}

#[test]
fn test_edit_file_empty_new_string_deletes_matched_text() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_delete.txt");
    std::fs::write(&file_path, "keep\nremove me\nkeep2").unwrap();
    let path_str = file_path.to_str().unwrap();

    let res = edit_file(path_str, &[chunk("remove me\n", "")]).unwrap();
    assert_eq!(res["status"], "success");
    assert_eq!(res["line_delta"], -1);
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "keep\nkeep2");
}

#[test]
fn test_edit_file_rejects_empty_old_string() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_empty.txt");
    std::fs::write(&file_path, "content").unwrap();
    let path_str = file_path.to_str().unwrap();

    assert!(edit_file(path_str, &[chunk("", "x")]).is_err());
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "content");
}

#[test]
fn test_edit_file_line_delta_reflects_real_line_count_change() {
    let _lock = crate::guardrails::ENV_MUTEX.blocking_lock();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_delta.txt");
    std::fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();
    let path_str = file_path.to_str().unwrap();

    // Replace 1 line with a 3-line block: +2 lines overall.
    let res = edit_file(path_str, &[chunk("line 2", "a\nb\nc")]).unwrap();
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
