//! Pagination, structural extraction, and structured-data querying: reading
//! large files in windows, filtering/sorting CSV-like matrices, stripping
//! implementation bodies down to a code skeleton, and querying JSON by path.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{ensure_existing_read_path, truncate_output, TruncateResult};
use serde_json::Value;
use std::io::Read;

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
/// let page = read_file_with_limit("README.md", None)?;
/// println!("{}", page.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn read_file_with_limit(path: &str, start_offset: Option<usize>) -> CoreResult<TruncateResult> {
    let path_buf = ensure_existing_read_path(path)?;

    let file = std::fs::File::open(&path_buf)
        .map_err(|e| CoreError::File(format!("Failed to open file: {e}")))?;
    let mut reader = std::io::BufReader::new(file);

    let offset = start_offset.unwrap_or(0);
    let limit: usize = std::env::var("AI_COMMAND_MAX_CHARACTERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192);

    let mut full_content = String::new();
    reader
        .read_to_string(&mut full_content)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;

    let skipped_content: String = full_content
        .chars()
        .skip(offset)
        .take(limit + 1024)
        .collect();

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
///     &["id".to_string(), "name".to_string()],
///     Some(true),
/// )?;
/// println!("{}", result.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn filter_and_sort_matrix_columns(
    path: &str,
    columns: &[String],
    deduplicate: Option<bool>,
) -> CoreResult<TruncateResult> {
    let path_buf = ensure_existing_read_path(path)?;

    let file = std::fs::File::open(&path_buf)
        .map_err(|e| CoreError::File(format!("Failed to open file: {e}")))?;
    // Support tab separation dynamically based on path
    let is_tsv = path.to_lowercase().ends_with(".tsv");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(if is_tsv { b'\t' } else { b',' })
        .from_reader(file);

    let col_indices = resolve_column_indices(&mut reader, columns, path)?;
    let mut rows = read_filtered_rows(&mut reader, &col_indices)?;
    if deduplicate.unwrap_or(false) {
        rows.sort();
        rows.dedup();
    } else {
        rows.sort(); // Always sort as per spec
    }

    Ok(truncate_output(&format_matrix_output(
        columns, &rows, is_tsv,
    )))
}

/// Resolves `columns` (requested header names) to their indices in
/// `reader`'s header row, in the order requested. Errors with the actual
/// headers present if none of `columns` match.
fn resolve_column_indices<R: std::io::Read>(
    reader: &mut csv::Reader<R>,
    columns: &[String],
    path: &str,
) -> CoreResult<Vec<usize>> {
    let headers = reader
        .headers()
        .map_err(|e| CoreError::Parsing(format!("Failed to read CSV headers: {e}")))?
        .clone();
    let col_indices: Vec<usize> = columns
        .iter()
        .filter_map(|col_name| headers.iter().position(|h| h == col_name))
        .collect();

    if col_indices.is_empty() {
        return Err(CoreError::General(format!(
            "None of the requested columns {:?} were found. Actual headers in '{}': {:?}",
            columns,
            path,
            headers.iter().collect::<Vec<_>>()
        )));
    }

    Ok(col_indices)
}

/// Reads every record from `reader`, keeping only the fields at `col_indices`
/// (in that order) from each row.
fn read_filtered_rows<R: std::io::Read>(
    reader: &mut csv::Reader<R>,
    col_indices: &[usize],
) -> CoreResult<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for result in reader.records() {
        let record =
            result.map_err(|e| CoreError::Parsing(format!("Failed to read CSV record: {e}")))?;
        let filtered_row: Vec<String> = col_indices
            .iter()
            .map(|&idx| record.get(idx).unwrap_or("").to_string())
            .collect();
        rows.push(filtered_row);
    }
    Ok(rows)
}

/// Joins `columns` as a header line followed by one delimiter-joined line per
/// row, using tabs when `is_tsv` else commas.
fn format_matrix_output(columns: &[String], rows: &[Vec<String>], is_tsv: bool) -> String {
    let delimiter = if is_tsv { "\t" } else { "," };
    let mut output = columns.join(delimiter);
    output.push('\n');
    for row in rows {
        output.push_str(&row.join(delimiter));
        output.push('\n');
    }
    output
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
    let path_buf = ensure_existing_read_path(path)?;

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;

    let json_data: Value = serde_json::from_str(&content)
        .map_err(|e| CoreError::Parsing(format!("Failed to parse JSON: {e}")))?;

    let pointer = dot_path_to_json_pointer(json_path);
    match json_data.pointer(&pointer) {
        Some(value) => Ok(value.clone()),
        None => Err(CoreError::General(describe_pointer_resolution_failure(
            &json_data, &pointer, json_path,
        ))),
    }
}

/// Converts a dot-separated path with optional bracket indices (e.g.
/// `"data.users[0].id"`) into a JSON pointer (`"/data/users/0/id"`).
fn dot_path_to_json_pointer(json_path: &str) -> String {
    let mut pointer = String::new();
    for part in json_path.split('.') {
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
    pointer
}

/// Walks `pointer` segment by segment against `json_data` to report exactly
/// where resolution broke and what's available there — a bare "not found"
/// gives an LLM nothing to correct `json_path` (the original dot-path
/// syntax, used only for the message) with.
fn describe_pointer_resolution_failure(
    json_data: &Value,
    pointer: &str,
    json_path: &str,
) -> String {
    let mut resolved_pointer = String::new();
    let mut current = json_data;
    for segment in pointer.split('/').filter(|s| !s.is_empty()) {
        let next_pointer = format!("{resolved_pointer}/{segment}");
        match json_data.pointer(&next_pointer) {
            Some(value) => {
                current = value;
                resolved_pointer = next_pointer;
            }
            None => break,
        }
    }

    let available = match current {
        Value::Object(map) => format!("an object with keys {:?}", map.keys().collect::<Vec<_>>()),
        Value::Array(arr) => format!(
            "an array with {} element(s) (valid indices 0..{})",
            arr.len(),
            arr.len()
        ),
        other => format!("a scalar value ({other})"),
    };
    let location = if resolved_pointer.is_empty() {
        "the root"
    } else {
        &resolved_pointer
    };

    format!(
        "Path '{json_path}' not found in JSON object — resolution stopped at '{location}', which is {available}"
    )
}

#[cfg(test)]
mod tests;
