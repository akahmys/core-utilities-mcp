//! Copy, move, and directory creation. Every entry point runs its path(s)
//! through [`crate::guardrails::validate_path_safety`] before touching disk.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::validate_path_safety;
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
        return Err(CoreError::File(format!("Source does not exist: {source}")));
    }

    if src.is_dir() {
        copy_dir_all(src, dest)
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CoreError::File(format!("Failed to create parent directory: {e}")))?;
        }
        std::fs::copy(src, dest)
            .map(|_| ())
            .map_err(|e| CoreError::File(format!("Failed to copy file: {e}")))
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> CoreResult<()> {
    std::fs::create_dir_all(&dst)
        .map_err(|e| CoreError::File(format!("Failed to create directory: {e}")))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| CoreError::File(format!("Failed to read directory: {e}")))?
    {
        let entry = entry.map_err(|e| CoreError::File(format!("Failed to read entry: {e}")))?;
        let file_type = entry
            .file_type()
            .map_err(|e| CoreError::File(format!("Failed to get file type: {e}")))?;
        if file_type.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))
                .map_err(|e| CoreError::File(format!("Failed to copy file: {e}")))?;
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
        return Err(CoreError::File(format!("Source does not exist: {source}")));
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CoreError::File(format!("Failed to create parent directory: {e}")))?;
    }

    std::fs::rename(src, dest).map_err(|e| CoreError::File(format!("Failed to move target: {e}")))
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
        .map_err(|e| CoreError::File(format!("Failed to create directory: {e}")))
}

#[cfg(test)]
mod tests;
