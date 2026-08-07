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
///
/// Content-addressed rather than line-addressed: `old_string` locates the
/// text to replace by matching it against the file's current content, so
/// an edit can't be invalidated by an earlier edit shifting line numbers,
/// or by the caller's memory of the file being slightly out of date.
#[derive(Debug, Clone, Deserialize)]
pub struct EditChunk {
    pub old_string: String,
    pub new_string: String,
}

/// Trims trailing whitespace from every line, so a match can succeed when
/// the only difference is invisible end-of-line whitespace.
fn trim_line_ends(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Byte range in the file that an [`EditChunk`] resolved to.
struct ResolvedEdit<'a> {
    start: usize,
    end: usize,
    new_string: &'a str,
}

fn find_whitespace_insensitive_matches(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    let trimmed_needle = trim_line_ends(needle);
    let mut fallback: Vec<(usize, usize)> = Vec::new();
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(haystack.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    for &start in &line_starts {
        let remainder = &haystack[start..];
        let needle_line_count = trimmed_needle.lines().count();
        let candidate_end = remainder
            .match_indices('\n')
            .nth(needle_line_count - 1)
            .map_or(remainder.len(), |(i, _)| i);
        let candidate = &remainder[..candidate_end];
        if trim_line_ends(candidate) == trimmed_needle {
            fallback.push((start, start + candidate.len()));
        }
    }
    fallback
}

/// Finds the byte range of the single occurrence of `needle` in
/// `haystack`, first exactly, then (only if there was no exact match at
/// all) ignoring trailing whitespace on each line.
///
/// # Errors
/// Returns [`CoreError::File`] if `needle` appears zero times or more than
/// once — an ambiguous match is refused rather than guessed at, since
/// picking the wrong one would silently corrupt the file.
fn find_unique_match(haystack: &str, needle: &str, path: &str) -> CoreResult<(usize, usize)> {
    if needle.is_empty() {
        return Err(CoreError::File(
            "old_string must not be empty — to create a file use write_file".to_string(),
        ));
    }

    let exact: Vec<usize> = haystack.match_indices(needle).map(|(i, _)| i).collect();
    match exact.len() {
        1 => return Ok((exact[0], exact[0] + needle.len())),
        n if n > 1 => {
            return Err(CoreError::File(format!(
                "old_string is not unique in '{path}': found {n} occurrences. Include more surrounding context to identify exactly one."
            )));
        }
        _ => {}
    }

    let fallback = find_whitespace_insensitive_matches(haystack, needle);
    match fallback.len() {
        1 => Ok(fallback[0]),
        0 => Err(CoreError::File(format!(
            "old_string not found in '{path}'. Re-read the file — its content may differ from what you expected."
        ))),
        n => Err(CoreError::File(format!(
            "old_string is not unique in '{path}': found {n} whitespace-insensitive matches. Include more surrounding context to identify exactly one."
        ))),
    }
}

fn resolve_edit_chunks<'a>(
    content: &str,
    edits: &'a [EditChunk],
    path: &str,
) -> CoreResult<Vec<ResolvedEdit<'a>>> {
    let mut resolved: Vec<ResolvedEdit> = Vec::with_capacity(edits.len());
    for edit in edits {
        let (start, end) = find_unique_match(content, &edit.old_string, path)?;
        resolved.push(ResolvedEdit {
            start,
            end,
            new_string: &edit.new_string,
        });
    }
    resolved.sort_by_key(|r| r.start);
    check_no_overlapping_edits(&resolved)?;
    Ok(resolved)
}

/// Applies one or more edits to a single file in a single atomic
/// transaction: every chunk's `old_string` is located in `path`'s current
/// content and checked for uniqueness before any of them are written, so
/// a failure anywhere leaves the file untouched.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if any `old_string` is missing or ambiguous, if two
/// chunks would edit overlapping regions, or if the write fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::{edit_file, EditChunk};
///
/// edit_file(
///     "src/config.rs",
///     &[EditChunk {
///         old_string: "const LIMIT: usize = 100;".to_string(),
///         new_string: "const LIMIT: usize = 200;".to_string(),
///     }],
/// )?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn edit_file(path: &str, edits: &[EditChunk]) -> CoreResult<Value> {
    validate_path_safety(path)?;
    let file_path = Path::new(path);
    if !file_path.is_file() {
        return Err(CoreError::File(format!("Path is not a file: {path}")));
    }

    if edits.is_empty() {
        return Err(CoreError::File("No edit chunks provided".to_string()));
    }

    let raw_content = std::fs::read_to_string(file_path)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;
    let content = raw_content.replace("\r\n", "\n");
    let original_line_count = content.split('\n').count();

    let resolved = resolve_edit_chunks(&content, edits, path)?;
    let new_content = apply_edits(&content, &resolved);
    atomic_write(file_path, &new_content)?;

    let (new_line_count, line_delta) = compute_line_stats(&new_content, original_line_count);

    Ok(json!({
        "status": "success",
        "chunks_applied": edits.len(),
        "new_line_count": new_line_count,
        "line_delta": line_delta
    }))
}

/// Computes `new_line_count` and `line_delta` from the post-edit text and
/// the pre-edit line count, reported so a caller can see at a glance how
/// much the file grew or shrank.
fn compute_line_stats(new_content: &str, original_line_count: usize) -> (usize, i64) {
    let new_line_count = new_content.split('\n').count();
    // A file's line count can never approach i64::MAX, so this cast cannot
    // actually wrap; `try_from` would only add an unreachable error path.
    #[allow(clippy::cast_possible_wrap)]
    let line_delta = new_line_count as i64 - original_line_count as i64;
    (new_line_count, line_delta)
}

/// Errors if any two (already start-sorted) resolved edits' byte ranges
/// overlap — which would mean two `old_string`s matched intersecting text.
fn check_no_overlapping_edits(edits: &[ResolvedEdit]) -> CoreResult<()> {
    for pair in edits.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(CoreError::File(
                "Two edits matched overlapping regions of the file. Split them into separate calls, or widen their old_strings so they target distinct text.".to_string(),
            ));
        }
    }
    Ok(())
}

/// Splices each resolved chunk's `new_string` over its matched byte range,
/// walking the file left to right. `edits` must already be start-sorted
/// and non-overlapping.
fn apply_edits(content: &str, edits: &[ResolvedEdit]) -> String {
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;

    for edit in edits {
        out.push_str(&content[cursor..edit.start]);
        out.push_str(edit.new_string);
        cursor = edit.end;
    }
    out.push_str(&content[cursor..]);

    out
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
