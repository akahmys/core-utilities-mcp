//! File and directory manipulation: delete, list, and stat live here;
//! copy/move/create/edit live in [`mutate`] and are re-exported so callers
//! keep using `file_ops::{copy_file_or_directory, ...}` regardless of the
//! split. The mutating entry points (here and in `mutate`) run their path(s)
//! through [`crate::guardrails::validate_path_safety`] before touching disk;
//! the read-only ones here use
//! [`validate_read_path_safety`](crate::guardrails::validate_read_path_safety).

mod mutate;
pub use mutate::{
    copy_file_or_directory, create_directory, edit_file_content, move_file_or_directory, write_file,
};

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
        return Err(CoreError::File(format!("Target does not exist: {path}")));
    }

    if path_buf.is_dir() {
        std::fs::remove_dir_all(path_buf)
            .map_err(|e| CoreError::File(format!("Failed to delete directory: {e}")))
    } else {
        std::fs::remove_file(path_buf)
            .map_err(|e| CoreError::File(format!("Failed to delete file: {e}")))
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
            "Path is not a directory: {target_path_str}"
        )));
    }

    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut links = Vec::new();

    let entries = std::fs::read_dir(target_path)
        .map_err(|e| CoreError::File(format!("Failed to read directory: {e}")))?;

    for entry in entries {
        let entry = entry.map_err(|e| CoreError::File(format!("Failed to read entry: {e}")))?;
        let file_type = entry
            .file_type()
            .map_err(|e| CoreError::File(format!("Failed to get file type: {e}")))?;
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
        return Err(CoreError::File(format!("Path does not exist: {path}")));
    }

    let metadata = std::fs::metadata(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to retrieve metadata: {e}")))?;

    let absolute_path = std::fs::canonicalize(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to resolve absolute path: {e}")))?
        .to_string_lossy()
        .into_owned();

    let permissions = if metadata.permissions().readonly() {
        "readonly"
    } else {
        "readwrite"
    };

    Ok(json!({
        "absolute_path": absolute_path,
        "size_bytes": metadata.len(),
        "is_dir": metadata.is_dir(),
        "permissions": permissions,
        "modified_at": epoch_secs(metadata.modified()),
        "accessed_at": epoch_secs(metadata.accessed())
    }))
}

/// Converts a fallible [`SystemTime`] (as returned by
/// [`std::fs::Metadata::modified`]/`accessed`, which can fail on platforms
/// without that timestamp) to Unix epoch seconds, defaulting to `0`.
fn epoch_secs(time: std::io::Result<SystemTime>) -> u64 {
    time.ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests;
