//! File and directory manipulation: copy, move, delete, create, list, stat,
//! and a safe line-range editor. Every mutating entry point runs its path(s)
//! through [`crate::guardrails::validate_path_safety`] before touching disk.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{validate_path_safety, validate_read_path_safety};
use serde_json::{json, Value};
use std::path::Path;
use std::time::SystemTime;

/// Deletes a file or, recursively, a directory, after validating that
/// `path` is not a catastrophic target (see
/// [`validate_path_safety`](crate::guardrails::validate_path_safety)).
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if the path fails safety validation, or
/// [`CoreError::File`] if the target does not exist or removal fails.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::delete_file_or_directory;
///
/// delete_file_or_directory("/tmp/scratch/old_report.txt")?;
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn delete_file_or_directory(path: &str) -> CoreResult<()> {
    validate_path_safety(path)?;

    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("Target does not exist: {}", path)));
    }

    if path_buf.is_dir() {
        std::fs::remove_dir_all(path_buf)
            .map_err(|e| CoreError::File(format!("Failed to delete directory: {}", e)))
    } else {
        std::fs::remove_file(path_buf)
            .map_err(|e| CoreError::File(format!("Failed to delete file: {}", e)))
    }
}

/// Lists the immediate contents of a directory (defaulting to `.`), split
/// into `files`, `directories`, and `links` name arrays.
///
/// # Errors
/// Returns [`CoreError::File`] if `path` is not a directory or cannot be read.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::list_directory_contents;
///
/// let listing = list_directory_contents(Some("src".to_string()))?;
/// println!("{}", listing["files"]);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn list_directory_contents(path: Option<String>) -> CoreResult<Value> {
    let target_path_str = path.unwrap_or_else(|| ".".to_string());
    validate_read_path_safety(&target_path_str)?;

    let target_path = Path::new(&target_path_str);
    if !target_path.is_dir() {
        return Err(CoreError::File(format!(
            "Path is not a directory: {}",
            target_path_str
        )));
    }

    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut links = Vec::new();

    let entries = std::fs::read_dir(target_path)
        .map_err(|e| CoreError::File(format!("Failed to read directory: {}", e)))?;

    for entry in entries {
        let entry = entry.map_err(|e| CoreError::File(format!("Failed to read entry: {}", e)))?;
        let file_type = entry
            .file_type()
            .map_err(|e| CoreError::File(format!("Failed to get file type: {}", e)))?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if file_type.is_symlink() {
            links.push(name);
        } else if file_type.is_dir() {
            directories.push(name);
        } else {
            files.push(name);
        }
    }

    Ok(json!({
        "files": files,
        "directories": directories,
        "links": links
    }))
}

/// Retrieves the canonical absolute path, size, read-only flag, and
/// modified/accessed timestamps (Unix epoch seconds) for `path`.
///
/// # Errors
/// Returns [`CoreError::File`] if `path` does not exist or its metadata
/// cannot be read.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::file_ops::get_file_metadata;
///
/// let meta = get_file_metadata("Cargo.toml")?;
/// println!("size: {}", meta["size_bytes"]);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn get_file_metadata(path: &str) -> CoreResult<Value> {
    validate_read_path_safety(path)?;

    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("Path does not exist: {}", path)));
    }

    let metadata = std::fs::metadata(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to retrieve metadata: {}", e)))?;

    let absolute_path = std::fs::canonicalize(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to resolve absolute path: {}", e)))?
        .to_string_lossy()
        .into_owned();

    let permissions = if metadata.permissions().readonly() {
        "readonly"
    } else {
        "readwrite"
    };

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let accessed = metadata
        .accessed()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(json!({
        "absolute_path": absolute_path,
        "size_bytes": metadata.len(),
        "is_dir": metadata.is_dir(),
        "permissions": permissions,
        "modified_at": modified,
        "accessed_at": accessed
    }))
}

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

    if start_line == 0 || end_line < start_line || end_line > lines.len() {
        return Err(CoreError::File(format!(
            "Invalid line range {}-{} for file with {} lines",
            start_line,
            end_line,
            lines.len()
        )));
    }

    let actual_lines = &lines[(start_line - 1)..end_line];
    let actual_target = actual_lines.join("\n");

    if actual_target.trim() != target_content.trim() {
        return Err(CoreError::File(format!(
            "Content mismatch. Expected:\n{}\nActual:\n{}",
            target_content.trim(),
            actual_target.trim()
        )));
    }

    let mut new_lines = Vec::new();
    new_lines.extend_from_slice(&lines[0..(start_line - 1)]);
    new_lines.push(replacement_content);
    if end_line < lines.len() {
        new_lines.extend_from_slice(&lines[end_line..]);
    }
    let new_content = new_lines.join("\n");

    let temp_path = file_path.with_extension("tmp");
    std::fs::write(&temp_path, new_content.as_bytes())
        .map_err(|e| CoreError::File(format!("Failed to write temp file: {}", e)))?;

    std::fs::rename(&temp_path, file_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        CoreError::File(format!("Failed to replace file: {}", e))
    })?;

    Ok(json!({
        "status": "success",
        "lines_modified": (end_line - start_line + 1),
        "new_line_count": new_lines.len()
    }))
}

#[cfg(test)]
mod tests;
