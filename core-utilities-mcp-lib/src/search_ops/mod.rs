//! High-efficiency `grep`- and `find`-style search, returning JSON-structured,
//! output-limited results instead of raw unbounded text.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{truncate_output, validate_read_path_safety, TruncateResult};
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// A compiled predicate over a single line of text, produced by
/// [`build_matcher`] from either a regex or a plain substring.
type LineMatcher = Box<dyn Fn(&str) -> bool>;

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
    let matcher = build_matcher(query_string, is_regex.unwrap_or(false))?;

    let path = Path::new(search_root_or_file);
    if !path.exists() {
        return Err(CoreError::File(format!(
            "Search root/file does not exist: {}",
            search_root_or_file
        )));
    }

    let matches = find_matches(&collect_files_to_scan(path), matcher.as_ref());
    let payload = serde_json::to_string_pretty(&matches).unwrap_or_else(|_| "[]".to_string());
    Ok(truncate_output(&payload))
}

/// Builds a line-matching predicate from `query_string`: a compiled regex
/// when `is_regex`, otherwise a plain substring check.
fn build_matcher(query_string: &str, is_regex: bool) -> CoreResult<LineMatcher> {
    if is_regex {
        let re = Regex::new(query_string)
            .map_err(|e| CoreError::Parsing(format!("Invalid regex: {}", e)))?;
        Ok(Box::new(move |s| re.is_match(s)))
    } else {
        let q = query_string.to_string();
        Ok(Box::new(move |s| s.contains(&q)))
    }
}

/// Returns `path` itself if it's a file, or every file beneath it if it's a
/// directory.
fn collect_files_to_scan(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect()
    }
}

/// Reads each of `files` as UTF-8 (silently skipping any that fail — binary
/// or unreadable files are not search errors) and returns every line
/// `matcher` accepts as a `{file, line, content}` JSON object.
fn find_matches(files: &[PathBuf], matcher: &dyn Fn(&str) -> bool) -> Vec<Value> {
    let mut matches = Vec::new();
    for file_path in files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
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
    matches
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

    let results: Vec<String> = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| entry_matches(entry, name_regex.as_ref(), file_type))
        .map(|entry| entry.path().to_string_lossy().into_owned())
        .collect();

    let payload = serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string());
    Ok(truncate_output(&payload))
}

/// True if `entry`'s name matches `name_regex` (when given) and its type
/// matches `file_type` (`"file"`, `"directory"`/`"dir"`, or `"symlink"`;
/// any other value, including `None`, matches everything).
fn entry_matches(entry: &DirEntry, name_regex: Option<&Regex>, file_type: Option<&str>) -> bool {
    if let Some(re) = name_regex {
        if !re.is_match(&entry.file_name().to_string_lossy()) {
            return false;
        }
    }

    match file_type {
        Some("file") => entry.file_type().is_file(),
        Some("directory") | Some("dir") => entry.file_type().is_dir(),
        Some("symlink") => entry.file_type().is_symlink(),
        _ => true,
    }
}

#[cfg(test)]
mod tests;
