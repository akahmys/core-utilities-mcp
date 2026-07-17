use crate::guardrails::{truncate_output, TruncateResult, validate_path_safety};
use std::path::Path;
use serde_json::Value;

/// Reads file contents starting from `start_offset` and applies output limits.
pub fn read_file_with_limit(
    path: &str,
    start_offset: Option<usize>,
    _smart_boundary: Option<bool>,
) -> Result<TruncateResult, String> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let offset = start_offset.unwrap_or(0);
    let skipped_content: String = content.chars().skip(offset).collect();

    let mut res = truncate_output(&skipped_content);
    if let Some(next) = res.next_offset {
        res.next_offset = Some(offset + next);
    }
    Ok(res)
}

/// Filters specified columns from CSV/TSV matrices and applies optional deduplication.
pub fn filter_and_sort_matrix_columns(
    path: &str,
    columns: Vec<String>,
    deduplicate: Option<bool>,
) -> Result<TruncateResult, String> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let file = std::fs::File::open(path_buf).map_err(|e| format!("Failed to open file: {}", e))?;
    // Support tab separation dynamically based on path
    let is_tsv = path.ends_with(".tsv");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(if is_tsv { b'\t' } else { b',' })
        .from_reader(file);

    let headers = reader.headers().map_err(|e| format!("Failed to read CSV headers: {}", e))?.clone();
    let col_indices: Vec<usize> = columns.iter()
        .filter_map(|col_name| {
            headers.iter().position(|h| h == col_name)
        })
        .collect();

    if col_indices.is_empty() {
        return Err("None of the specified columns were found in the CSV/TSV headers.".to_string());
    }

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        let filtered_row: Vec<String> = col_indices.iter()
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

/// Agnostic regex skeleton parser that strips local implementation bodies to save token counts.
pub fn extract_code_skeleton(path: &str) -> Result<TruncateResult, String> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| format!("Failed to read file: {}", e))?;

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
        if re_keywords.is_match(line) || (line.starts_with(|c: char| !c.is_whitespace()) && !trimmed.is_empty()) {
            // Strip brackets/braces from definition lines to keep skeleton compact
            let clean_line = line.split('{').next().unwrap_or(line).trim_end();
            skeleton.push_str(clean_line);
            skeleton.push('\n');
        }
    }

    Ok(truncate_output(&skeleton))
}

/// Helper to parse dot-separated and indexed paths (e.g. data.users[0].id)
pub fn query_json_by_path(path: &str, json_path: &str) -> Result<Value, String> {
    validate_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let content = std::fs::read_to_string(path_buf)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let json_data: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Convert path like "data.users[0].id" to json pointer "/data/users/0/id"
    let mut pointer = String::new();
    let parts = json_path.split('.');
    for part in parts {
        if part.contains('[') && part.contains(']') {
            let base = part.split('[').next().unwrap_or("");
            let index = part.split('[').nth(1).and_then(|idx| idx.split(']').next()).unwrap_or("");
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

    json_data.pointer(&pointer)
        .cloned()
        .ok_or_else(|| format!("Path '{}' not found in JSON object", json_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_read_file_within_limit() {
        let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_read.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "hello").unwrap();
        writeln!(file, "world").unwrap();

        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "50");
        let res = read_file_with_limit(file_path.to_str().unwrap(), None, None).unwrap();
        assert_eq!(res.content, "hello\nworld\n");
        assert_eq!(res.status, "success");
        assert_eq!(res.next_offset, None);
    }

    #[test]
    fn test_read_file_with_offset_and_truncation() {
        let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_read.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "hello").unwrap();
        writeln!(file, "world").unwrap();

        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "4");
        let res = read_file_with_limit(file_path.to_str().unwrap(), Some(6), None).unwrap();
        assert_eq!(res.content, "worl");
        assert_eq!(res.status, "truncated");
        assert_eq!(res.next_offset, Some(10));
    }

    #[test]
    fn test_filter_and_sort_matrix_columns() {
        let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("matrix.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "id,name,age").unwrap();
        writeln!(file, "1,Alice,30").unwrap();
        writeln!(file, "2,Bob,25").unwrap();
        writeln!(file, "1,Alice,30").unwrap();

        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
        let res = filter_and_sort_matrix_columns(
            file_path.to_str().unwrap(),
            vec!["name".to_string(), "id".to_string()],
            Some(true)
        ).unwrap();
        assert!(res.content.contains("name,id"));
        assert!(res.content.contains("Alice,1"));
        assert!(res.content.contains("Bob,2"));
    }

    #[test]
    fn test_extract_code_skeleton() {
        let _lock = crate::guardrails::ENV_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("code.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "pub struct User {{").unwrap();
        writeln!(file, "    pub name: String,").unwrap();
        writeln!(file, "}}").unwrap();
        writeln!(file, "impl User {{").unwrap();
        writeln!(file, "    pub fn new() -> Self {{").unwrap();
        writeln!(file, "        User {{ name: String::new() }}").unwrap();
        writeln!(file, "    }}").unwrap();
        writeln!(file, "}}").unwrap();

        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "1000");
        let res = extract_code_skeleton(file_path.to_str().unwrap()).unwrap();
        assert!(res.content.contains("pub struct User"));
        assert!(res.content.contains("impl User"));
        assert!(!res.content.contains("pub name: String"));
    }

    #[test]
    fn test_query_json_by_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("data.json");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, r#"{{"data": {{"users": [{{"id": 42, "name": "Alice"}}]}}}}"#).unwrap();

        let val = query_json_by_path(file_path.to_str().unwrap(), "data.users[0].id").unwrap();
        assert_eq!(val, serde_json::Value::from(42));
    }
}
