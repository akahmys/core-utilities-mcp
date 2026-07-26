//! Pagination, structural extraction, and structured-data querying: reading
//! large files in windows, filtering/sorting CSV-like matrices, stripping
//! implementation bodies down to a code skeleton, and querying JSON by path.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{truncate_output, validate_path_safety, TruncateResult};
use serde_json::Value;
use std::path::Path;

/// Reads `path` as UTF-8 text starting at character offset `start_offset`
/// (default `0`) and applies the standard
/// [`truncate_output`](crate::guardrails::truncate_output) size limit. When
/// the result is truncated, `next_offset` gives the offset to resume from on
/// a subsequent call, enabling windowed reads of arbitrarily large files.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if it does not exist or is not valid UTF-8.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::read_file_with_limit;
///
/// let page = read_file_with_limit("README.md", None, None)?;
/// println!("{}", page.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn read_file_with_limit(
    path: &str,
    start_offset: Option<usize>,
    _smart_boundary: Option<bool>,
) -> CoreResult<TruncateResult> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("File not found: {}", path)));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {}", e)))?;

    let offset = start_offset.unwrap_or(0);
    let skipped_content: String = content.chars().skip(offset).collect();

    let mut res = truncate_output(&skipped_content);
    if let Some(next) = res.next_offset {
        res.next_offset = Some(offset + next);
    }
    Ok(res)
}

/// Reads the CSV (or TSV, inferred from a `.tsv` extension) file at `path`,
/// keeps only the named `columns` (in the order given), sorts the resulting
/// rows, optionally deduplicating them, and returns the output as
/// delimiter-joined text subject to the standard output limit.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation,
/// [`CoreError::File`] if it does not exist or cannot be opened,
/// [`CoreError::Parsing`] if the CSV/TSV cannot be parsed, or
/// [`CoreError::General`] if none of `columns` match the header row.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::filter_and_sort_matrix_columns;
///
/// let result = filter_and_sort_matrix_columns(
///     "data/users.csv",
///     vec!["id".to_string(), "name".to_string()],
///     Some(true),
/// )?;
/// println!("{}", result.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn filter_and_sort_matrix_columns(
    path: &str,
    columns: Vec<String>,
    deduplicate: Option<bool>,
) -> CoreResult<TruncateResult> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("File not found: {}", path)));
    }

    let file = std::fs::File::open(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to open file: {}", e)))?;
    // Support tab separation dynamically based on path
    let is_tsv = path.ends_with(".tsv");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(if is_tsv { b'\t' } else { b',' })
        .from_reader(file);

    let headers = reader
        .headers()
        .map_err(|e| CoreError::Parsing(format!("Failed to read CSV headers: {}", e)))?
        .clone();
    let col_indices: Vec<usize> = columns
        .iter()
        .filter_map(|col_name| headers.iter().position(|h| h == col_name))
        .collect();

    if col_indices.is_empty() {
        return Err(CoreError::General(
            "None of the specified columns were found in the CSV/TSV headers.".to_string(),
        ));
    }

    let mut rows = Vec::new();
    for result in reader.records() {
        let record =
            result.map_err(|e| CoreError::Parsing(format!("Failed to read CSV record: {}", e)))?;
        let filtered_row: Vec<String> = col_indices
            .iter()
            .map(|&idx| record.get(idx).unwrap_or("").to_string())
            .collect();
        rows.push(filtered_row);
    }

    if deduplicate.unwrap_or(false) {
        rows.sort();
        rows.dedup();
    } else {
        rows.sort(); // Always sort as per spec
    }

    let mut output = String::new();
    // Headers
    let header_line = columns.join(if is_tsv { "\t" } else { "," });
    output.push_str(&header_line);
    output.push('\n');

    for row in rows {
        output.push_str(&row.join(if is_tsv { "\t" } else { "," }));
        output.push('\n');
    }

    Ok(truncate_output(&output))
}

/// Produces a compact "skeleton" of the source file at `path` by keeping
/// only definition lines (`class`, `def`, `fn`, `struct`, `impl`, etc.) and
/// top-level statements, stripping block bodies and comments. This is a
/// language-agnostic, regex-based heuristic (not a full parser), intended to
/// let an AI agent survey a file's structure using a fraction of the tokens
/// a full read would cost.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if it does not exist or is not valid UTF-8.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::extract_code_skeleton;
///
/// let skeleton = extract_code_skeleton("src/lib.rs")?;
/// println!("{}", skeleton.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn extract_code_skeleton(path: &str) -> CoreResult<TruncateResult> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("File not found: {}", path)));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {}", e)))?;

    let mut skeleton = String::new();
    let re_keywords = regex::Regex::new(r"^(?i)\s*(class|def|fn|struct|enum|trait|impl|func|interface|type|public\s+class|private\s+class|pub\s+fn)\b")
        .unwrap();

    let mut inside_comment_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("/*") {
            inside_comment_block = true;
        }
        if inside_comment_block {
            if trimmed.ends_with("*/") {
                inside_comment_block = false;
            }
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("#") {
            continue;
        }

        // Keep definitions or very shallow indent markers
        if re_keywords.is_match(line)
            || (line.starts_with(|c: char| !c.is_whitespace()) && !trimmed.is_empty())
        {
            // Strip brackets/braces from definition lines to keep skeleton compact
            let clean_line = line.split('{').next().unwrap_or(line).trim_end();
            skeleton.push_str(clean_line);
            skeleton.push('\n');
        }
    }

    Ok(truncate_output(&skeleton))
}

/// Reads the JSON file at `path` and resolves `json_path`, a dot-separated
/// path with optional bracket indices (e.g. `data.users[0].id`), returning
/// the matched value.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation,
/// [`CoreError::File`] if it does not exist, [`CoreError::Parsing`] if it is
/// not valid JSON, or [`CoreError::General`] if `json_path` does not resolve
/// to a value.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::query_json_by_path;
///
/// let id = query_json_by_path("data/users.json", "data.users[0].id")?;
/// println!("{}", id);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn query_json_by_path(path: &str, json_path: &str) -> CoreResult<Value> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("File not found: {}", path)));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {}", e)))?;

    let json_data: Value = serde_json::from_str(&content)
        .map_err(|e| CoreError::Parsing(format!("Failed to parse JSON: {}", e)))?;

    // Convert path like "data.users[0].id" to json pointer "/data/users/0/id"
    let mut pointer = String::new();
    let parts = json_path.split('.');
    for part in parts {
        if part.contains('[') && part.contains(']') {
            let base = part.split('[').next().unwrap_or("");
            let index = part
                .split('[')
                .nth(1)
                .and_then(|idx| idx.split(']').next())
                .unwrap_or("");
            if !base.is_empty() {
                pointer.push('/');
                pointer.push_str(base);
            }
            pointer.push('/');
            pointer.push_str(index);
        } else if !part.is_empty() {
            pointer.push('/');
            pointer.push_str(part);
        }
    }

    json_data
        .pointer(&pointer)
        .cloned()
        .ok_or_else(|| CoreError::General(format!("Path '{}' not found in JSON object", json_path)))
}

#[cfg(test)]
mod tests;
