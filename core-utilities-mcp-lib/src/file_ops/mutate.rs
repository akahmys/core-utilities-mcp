//! The mutating half of `file_ops`: copy, move, create, and the line-range
//! editor. Every entry point runs its path(s) through
//! [`crate::guardrails::validate_path_safety`] before touching disk.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::validate_path_safety;
use serde_json::{json, Value};
use std::path::Path;

/// Copies `source` to `destination`, recursing into subdirectories when
/// `source` is a directory. Missing parent directories of `destination` are
/// created automatically.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if either path fails safety validation,
/// or [`CoreError::File`] if `source` does not exist or the copy fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::copy_file_or_directory;
///
/// copy_file_or_directory("config/base.toml", "config/backup.toml")?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn copy_file_or_directory(source: &str, destination: &str) -> CoreResult<()> {
    validate_path_safety(source)?;
    validate_path_safety(destination)?;

    let src = Path::new(source);
    let dest = Path::new(destination);

    if !src.exists() {
        return Err(CoreError::File(format!(
            "Source does not exist: {}",
            source
        )));
    }

    if src.is_dir() {
        copy_dir_all(src, dest)
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::File(format!("Failed to create parent directory: {}", e))
            })?;
        }
        std::fs::copy(src, dest)
            .map(|_| ())
            .map_err(|e| CoreError::File(format!("Failed to copy file: {}", e)))
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> CoreResult<()> {
    std::fs::create_dir_all(&dst)
        .map_err(|e| CoreError::File(format!("Failed to create directory: {}", e)))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| CoreError::File(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry.map_err(|e| CoreError::File(format!("Failed to read entry: {}", e)))?;
        let file_type = entry
            .file_type()
            .map_err(|e| CoreError::File(format!("Failed to get file type: {}", e)))?;
        if file_type.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))
                .map_err(|e| CoreError::File(format!("Failed to copy file: {}", e)))?;
        }
    }
    Ok(())
}

/// Moves (renames) `source` to `destination`, creating any missing parent
/// directories of `destination` first.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if either path fails safety validation,
/// or [`CoreError::File`] if `source` does not exist or the move fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::move_file_or_directory;
///
/// move_file_or_directory("drafts/report.md", "published/report.md")?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn move_file_or_directory(source: &str, destination: &str) -> CoreResult<()> {
    validate_path_safety(source)?;
    validate_path_safety(destination)?;

    let src = Path::new(source);
    let dest = Path::new(destination);

    if !src.exists() {
        return Err(CoreError::File(format!(
            "Source does not exist: {}",
            source
        )));
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CoreError::File(format!("Failed to create parent directory: {}", e)))?;
    }

    std::fs::rename(src, dest).map_err(|e| CoreError::File(format!("Failed to move target: {}", e)))
}

/// Creates `path`, including all intermediate parent directories
/// (equivalent to `mkdir -p`).
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if directory creation fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::create_directory;
///
/// create_directory("build/output/nested")?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn create_directory(path: &str) -> CoreResult<()> {
    validate_path_safety(path)?;
    std::fs::create_dir_all(Path::new(path))
        .map_err(|e| CoreError::File(format!("Failed to create directory: {}", e)))
}

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
            "'{}' already exists — pass overwrite: true to replace it, or use edit_file_content for a targeted change",
            path
        )));
    }

    if let Some(parent) = file_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::File(format!("Failed to create parent directory: {}", e))
            })?;
        }
    }

    atomic_write(file_path, content)?;

    Ok(json!({
        "status": "success",
        "bytes_written": content.len()
    }))
}

/// Replaces the 1-indexed line range `start_line..=end_line` in `path` with
/// `replacement_content`, but only if that range's current content
/// (whitespace-trimmed) exactly matches `target_content`. This
/// verify-then-write pattern prevents editing stale or unexpected content.
/// The write is atomic: content is staged to a sibling `.tmp` file and
/// renamed into place.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if `path` is not a file, the line range is invalid,
/// `target_content` does not match, or the write fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::edit_file_content;
///
/// edit_file_content(
///     "src/config.rs",
///     3,
///     3,
///     "const LIMIT: usize = 100;",
///     "const LIMIT: usize = 200;",
/// )?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn edit_file_content(
    path: &str,
    start_line: usize,
    end_line: usize,
    target_content: &str,
    replacement_content: &str,
) -> CoreResult<Value> {
    validate_path_safety(path)?;
    let file_path = Path::new(path);
    if !file_path.is_file() {
        return Err(CoreError::File(format!("Path is not a file: {}", path)));
    }

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| CoreError::File(format!("Failed to read file: {}", e)))?;

    let lines: Vec<&str> = content.split('\n').collect();
    verify_line_range(&lines, start_line, end_line, target_content)?;

    let mut new_lines = Vec::new();
    new_lines.extend_from_slice(&lines[0..(start_line - 1)]);
    new_lines.push(replacement_content);
    if end_line < lines.len() {
        new_lines.extend_from_slice(&lines[end_line..]);
    }
    let new_content = new_lines.join("\n");

    atomic_write(file_path, &new_content)?;

    Ok(json!({
        "status": "success",
        "lines_modified": (end_line - start_line + 1),
        "new_line_count": new_lines.len()
    }))
}

/// Validates that `start_line..=end_line` is a well-formed 1-indexed range
/// into `lines`, and that its whitespace-trimmed content exactly matches
/// `target_content` — the verify half of [`edit_file_content`]'s
/// verify-then-write pattern.
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
    if actual_target.trim() != target_content.trim() {
        return Err(CoreError::File(format!(
            "Content mismatch. Expected:\n{}\nActual:\n{}",
            target_content.trim(),
            actual_target.trim()
        )));
    }

    Ok(())
}

/// Writes `content` to `path` atomically: staged to a sibling `.tmp` file,
/// then renamed into place, so a crash or concurrent read never observes a
/// partially written file. The staged file is cleaned up if the rename
/// fails.
fn atomic_write(path: &Path, content: &str) -> CoreResult<()> {
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, content.as_bytes())
        .map_err(|e| CoreError::File(format!("Failed to write temp file: {}", e)))?;

    std::fs::rename(&temp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        CoreError::File(format!("Failed to replace file: {}", e))
    })
}

#[cfg(test)]
mod tests;
