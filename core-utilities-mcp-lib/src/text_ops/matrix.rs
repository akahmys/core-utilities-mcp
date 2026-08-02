//! Filtering, sorting, and deduplicating columns from CSV/TSV data.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{ensure_existing_read_path, truncate_output, TruncateResult};

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

#[cfg(test)]
mod tests;
