//! The line-range editor: single/multi-chunk verified edits and controlled
//! file creation. Every entry point runs its path through
//! [`crate::guardrails::validate_path_safety`] before touching disk.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::validate_path_safety;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Writes `content` to `path`, creating any missing parent directories
/// first. Refuses to overwrite an existing file unless `overwrite` is
/// `true`, to prevent accidentally clobbering content the caller didn't
/// intend to replace. The write is atomic (see [`atomic_write`]).
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if `path` already exists and `overwrite` is not
/// `true`, if parent directories can't be created, or if the write fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::write_file;
///
/// write_file("notes/todo.md", "- [ ] write more tests\n", None)?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn write_file(path: &str, content: &str, overwrite: Option<bool>) -> CoreResult<Value> {
    validate_path_safety(path)?;
    let file_path = Path::new(path);

    if file_path.exists() && !overwrite.unwrap_or(false) {
        return Err(CoreError::File(format!(
            "'{path}' already exists — pass overwrite: true to replace it, or use edit_file for a targeted change"
        )));
    }

    if let Some(parent) = file_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CoreError::File(format!("Failed to create parent directory: {e}")))?;
        }
    }

    atomic_write(file_path, content)?;

    Ok(json!({
        "status": "success",
        "bytes_written": content.len()
    }))
}

/// Represents a single edit chunk for [`edit_file`].
#[derive(Debug, Clone, Deserialize)]
pub struct EditChunk {
    pub start_line: usize,
    pub end_line: usize,
    pub target_content: String,
    pub replacement_content: String,
}

/// Normalizes text by unifying newline formats and trimming trailing whitespace from lines.
fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Applies one or more non-contiguous edits to a single file in a single
/// atomic transaction, verifying every chunk's `target_content` against
/// `path`'s current content before writing any of them.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if any edit chunk fails verification, or if ranges overlap.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::{edit_file, EditChunk};
///
/// edit_file(
///     "src/config.rs",
///     vec![EditChunk {
///         start_line: 3,
///         end_line: 3,
///         target_content: "const LIMIT: usize = 100;".to_string(),
///         replacement_content: "const LIMIT: usize = 200;".to_string(),
///     }],
/// )?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn edit_file(path: &str, mut edits: Vec<EditChunk>) -> CoreResult<Value> {
    validate_path_safety(path)?;
    let file_path = Path::new(path);
    if !file_path.is_file() {
        return Err(CoreError::File(format!("Path is not a file: {path}")));
    }

    if edits.is_empty() {
        return Err(CoreError::File("No edit chunks provided".to_string()));
    }

    // Sort edits by start_line ascending to process predictably
    edits.sort_by_key(|e| e.start_line);
    check_no_overlapping_edits(&edits)?;

    let raw_content = std::fs::read_to_string(file_path)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;

    let normalized_raw = raw_content.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized_raw.split('\n').collect();
    let original_line_count = lines.len();

    // Verify all edit chunks first (All-or-Nothing transaction)
    for edit in &edits {
        verify_line_range(&lines, edit.start_line, edit.end_line, &edit.target_content)?;
    }

    let new_lines = apply_edits(&lines, &edits);
    let new_content = new_lines.join("\n");
    atomic_write(file_path, &new_content)?;

    let (new_line_count, line_delta) = compute_line_stats(&new_content, original_line_count);

    Ok(json!({
        "status": "success",
        "chunks_applied": edits.len(),
        "new_line_count": new_line_count,
        "line_delta": line_delta
    }))
}

/// Computes `new_line_count` and `line_delta` from the joined post-edit
/// text and the pre-edit line count. Recounts from the joined text rather
/// than trusting `new_lines.len()`: a chunk whose `replacement_content`
/// itself spans multiple lines contributes only one entry to that vector,
/// which would otherwise undercount.
fn compute_line_stats(new_content: &str, original_line_count: usize) -> (usize, i64) {
    let new_line_count = new_content.split('\n').count();
    // A file's line count can never approach i64::MAX, so this cast cannot
    // actually wrap; `try_from` would only add an unreachable error path.
    #[allow(clippy::cast_possible_wrap)]
    let line_delta = new_line_count as i64 - original_line_count as i64;
    (new_line_count, line_delta)
}

/// Errors if any two (already start_line-sorted) edits' ranges overlap.
fn check_no_overlapping_edits(edits: &[EditChunk]) -> CoreResult<()> {
    for i in 0..edits.len().saturating_sub(1) {
        if edits[i].end_line >= edits[i + 1].start_line {
            return Err(CoreError::File(format!(
                "Overlapping edit ranges: {}-{} and {}-{}",
                edits[i].start_line,
                edits[i].end_line,
                edits[i + 1].start_line,
                edits[i + 1].end_line
            )));
        }
    }
    Ok(())
}

/// Builds the post-edit line vector: `edits` (already start_line-sorted and
/// pre-verified by the caller) are applied top to bottom, splicing each
/// chunk's replacement in place of its `start_line..=end_line` range.
fn apply_edits<'a>(lines: &[&'a str], edits: &'a [EditChunk]) -> Vec<&'a str> {
    let mut new_lines = Vec::new();
    let mut current_idx = 0; // 0-indexed line pointer

    for edit in edits {
        let chunk_start_idx = edit.start_line - 1;
        let chunk_end_idx = edit.end_line; // exclusive boundary

        // Push untouched lines preceding this chunk
        new_lines.extend_from_slice(&lines[current_idx..chunk_start_idx]);

        // Push replacement lines
        if !edit.replacement_content.is_empty() {
            new_lines.push(edit.replacement_content.as_str());
        }

        current_idx = chunk_end_idx;
    }

    // Push remaining lines after last chunk
    if current_idx < lines.len() {
        new_lines.extend_from_slice(&lines[current_idx..]);
    }

    new_lines
}

/// Validates that `start_line..=end_line` is a well-formed 1-indexed range
/// into `lines`, and that its content matches `target_content` (with whitespace normalization).
fn verify_line_range(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    target_content: &str,
) -> CoreResult<()> {
    if start_line == 0 || end_line < start_line || end_line > lines.len() {
        return Err(CoreError::File(format!(
            "Invalid line range {}-{} for file with {} lines",
            start_line,
            end_line,
            lines.len()
        )));
    }

    let actual_target = lines[(start_line - 1)..end_line].join("\n");
    if normalize_text(&actual_target) != normalize_text(target_content) {
        return Err(CoreError::File(format!(
            "Content mismatch for range {}-{}.\nExpected:\n{}\nActual:\n{}",
            start_line,
            end_line,
            target_content.trim(),
            actual_target.trim()
        )));
    }

    Ok(())
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `content` to `path` atomically: staged to a unique sibling `.tmp` file,
/// then renamed into place, so a crash or concurrent read never observes a
/// partially written file. The staged file is cleaned up if the rename
/// fails.
fn atomic_write(path: &Path, content: &str) -> CoreResult<()> {
    let pid = std::process::id();
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .map_or_else(|| "file".to_string(), |n| n.to_string_lossy().into_owned());
    let temp_name = format!(".{file_name}.tmp.{pid}_{seq}");
    let temp_path = path.with_file_name(temp_name);

    std::fs::write(&temp_path, content.as_bytes()).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        CoreError::File(format!("Failed to write temp file: {e}"))
    })?;

    std::fs::rename(&temp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        CoreError::File(format!("Failed to replace file: {e}"))
    })
}

#[cfg(test)]
mod tests;
