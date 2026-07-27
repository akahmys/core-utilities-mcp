//! High-efficiency `grep`- and `find`-style search, returning JSON-structured,
//! output-limited results instead of raw unbounded text.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{truncate_output, validate_read_path_safety, TruncateResult};
use regex::Regex;
use serde_json::json;
use std::path::Path;
use walkdir::WalkDir;

/// Searches for `query_string` (plain substring, or a regex when `is_regex`
/// is `true`) across every file under `search_root_or_file`, or within a
/// single file if it points directly at one. Matches are returned as a
/// JSON array of `{file, line, content}` objects and are subject to the
/// same [`truncate_output`](crate::guardrails::truncate_output) limits as
/// other tools.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if the path fails safety validation,
/// [`CoreError::File`] if it does not exist, or [`CoreError::Parsing`] if
/// `is_regex` is `true` and `query_string` is not a valid regex.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::search_ops::search_text_with_limit;
///
/// let result = search_text_with_limit("src", "TODO", Some(false))?;
/// println!("{}", result.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn search_text_with_limit(
    search_root_or_file: &str,
    query_string: &str,
    is_regex: Option<bool>,
) -> CoreResult<TruncateResult> {
    validate_read_path_safety(search_root_or_file)?;

    let is_regex = is_regex.unwrap_or(false);
    let matcher: Box<dyn Fn(&str) -> bool> = if is_regex {
        let re = Regex::new(query_string)
            .map_err(|e| CoreError::Parsing(format!("Invalid regex: {}", e)))?;
        Box::new(move |s| re.is_match(s))
    } else {
        let q = query_string.to_string();
        Box::new(move |s| s.contains(&q))
    };

    let path = Path::new(search_root_or_file);
    if !path.exists() {
        return Err(CoreError::File(format!(
            "Search root/file does not exist: {}",
            search_root_or_file
        )));
    }

    let mut matches = Vec::new();

    let files_to_scan: Vec<std::path::PathBuf> = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect()
    };

    for file_path in files_to_scan {
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            for (idx, line) in content.lines().enumerate() {
                if matcher(line) {
                    matches.push(json!({
                        "file": file_path.to_string_lossy().into_owned(),
                        "line": idx + 1,
                        "content": line.trim()
                    }));
                }
            }
        }
    }

    let payload = serde_json::to_string_pretty(&matches).unwrap_or_else(|_| "[]".to_string());
    Ok(truncate_output(&payload))
}

/// Walks `search_root` (defaulting to `.`) and returns the paths of entries
/// matching an optional `name_pattern` regex and/or `file_type`
/// (`"file"`, `"directory"`/`"dir"`, or `"symlink"`).
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if the root fails safety validation,
/// [`CoreError::File`] if it does not exist, or [`CoreError::Parsing`] if
/// `name_pattern` is not a valid regex.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::search_ops::search_file_by_name_or_type;
///
/// let result = search_file_by_name_or_type(Some("src"), Some(r"\.rs$"), Some("file"))?;
/// println!("{}", result.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn search_file_by_name_or_type(
    search_root: Option<&str>,
    name_pattern: Option<&str>,
    file_type: Option<&str>,
) -> CoreResult<TruncateResult> {
    let root_str = search_root.unwrap_or(".");
    validate_read_path_safety(root_str)?;

    let path = Path::new(root_str);
    if !path.exists() {
        return Err(CoreError::File(format!(
            "Search root directory does not exist: {}",
            root_str
        )));
    }

    let name_regex = if let Some(pattern) = name_pattern {
        Some(
            Regex::new(pattern)
                .map_err(|e| CoreError::Parsing(format!("Invalid name pattern regex: {}", e)))?,
        )
    } else {
        None
    };

    let mut results = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy();

        if let Some(ref re) = name_regex {
            if !re.is_match(&name) {
                continue;
            }
        }

        if let Some(t) = file_type {
            let matches_type = match t {
                "file" => entry.file_type().is_file(),
                "directory" | "dir" => entry.file_type().is_dir(),
                "symlink" => entry.file_type().is_symlink(),
                _ => true,
            };
            if !matches_type {
                continue;
            }
        }

        results.push(entry_path.to_string_lossy().into_owned());
    }

    let payload = serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string());
    Ok(truncate_output(&payload))
}

#[cfg(test)]
mod tests;
