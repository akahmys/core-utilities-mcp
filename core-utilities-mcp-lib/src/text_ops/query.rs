//! Querying JSON, TOML, or YAML files by dot-path, normalizing all three
//! into [`serde_json::Value`] so one resolution path serves all formats.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::ensure_existing_read_path;
use serde_json::Value;
use std::path::Path;

/// Reads the JSON, TOML, or YAML file at `path` (format auto-detected from
/// its extension — `.toml`, `.yml`/`.yaml`, or JSON for anything else,
/// including `.json`) and resolves `data_path`, a dot-separated path with
/// optional bracket indices (e.g. `data.users[0].id`), returning the
/// matched value.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation,
/// [`CoreError::File`] if it does not exist, [`CoreError::Parsing`] if it is
/// not valid for its detected format, or [`CoreError::General`] if
/// `data_path` does not resolve to a value.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::query_data_by_path;
///
/// let id = query_data_by_path("data/users.json", "data.users[0].id")?;
/// println!("{}", id);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn query_data_by_path(path: &str, data_path: &str) -> CoreResult<Value> {
    let path_buf = ensure_existing_read_path(path)?;

    let content = std::fs::read_to_string(&path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;

    let data = parse_structured_data(&content, &path_buf)?;

    let pointer = dot_path_to_json_pointer(data_path);
    match data.pointer(&pointer) {
        Some(value) => Ok(value.clone()),
        None => Err(CoreError::General(describe_pointer_resolution_failure(
            &data, &pointer, data_path,
        ))),
    }
}

/// Parses `content` as JSON, TOML, or YAML based on `path`'s extension
/// (`.toml`, `.yml`/`.yaml`, else JSON), normalizing all three into a single
/// [`Value`] representation so [`query_data_by_path`] can query any of them
/// with the same dot-path syntax.
fn parse_structured_data(content: &str, path: &Path) -> CoreResult<Value> {
    let ext = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "toml" => {
            let value: toml::Value = toml::from_str(content)
                .map_err(|e| CoreError::Parsing(format!("Failed to parse TOML: {e}")))?;
            serde_json::to_value(value)
                .map_err(|e| CoreError::Parsing(format!("Failed to convert TOML to JSON: {e}")))
        }
        "yml" | "yaml" => {
            let docs = yaml_rust2::YamlLoader::load_from_str(content)
                .map_err(|e| CoreError::Parsing(format!("Failed to parse YAML: {e}")))?;
            let doc = docs
                .first()
                .ok_or_else(|| CoreError::Parsing("YAML file contains no documents".to_string()))?;
            Ok(yaml_to_json_value(doc))
        }
        _ => serde_json::from_str(content)
            .map_err(|e| CoreError::Parsing(format!("Failed to parse JSON: {e}"))),
    }
}

/// Converts a parsed [`yaml_rust2::Yaml`] tree into an equivalent
/// [`Value`], so [`query_data_by_path`] can query YAML with the same
/// dot-path/pointer logic used for JSON and TOML. `Alias`, `Null`, and the
/// parse-failure sentinel `BadValue` all map to [`Value::Null`] — real
/// documents don't contain unresolved aliases or `BadValue` (that variant
/// only appears from invalid indexing, which this conversion never does).
fn yaml_to_json_value(yaml: &yaml_rust2::Yaml) -> Value {
    use yaml_rust2::Yaml;
    match yaml {
        Yaml::Real(s) => s
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| Value::String(s.clone()), Value::Number),
        Yaml::Integer(i) => Value::Number((*i).into()),
        Yaml::String(s) => Value::String(s.clone()),
        Yaml::Boolean(b) => Value::Bool(*b),
        Yaml::Array(arr) => Value::Array(arr.iter().map(yaml_to_json_value).collect()),
        Yaml::Hash(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (yaml_key_to_string(k), yaml_to_json_value(v)))
                .collect(),
        ),
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Value::Null,
    }
}

/// Stringifies a YAML mapping key for use as a JSON object key (which must
/// be a string). Non-scalar or null keys — vanishingly rare in real config
/// files — collapse to an empty string rather than being dropped, so a
/// lookup miss is at least visible in the resulting object's shape.
fn yaml_key_to_string(key: &yaml_rust2::Yaml) -> String {
    use yaml_rust2::Yaml;
    match key {
        Yaml::String(s) | Yaml::Real(s) => s.clone(),
        Yaml::Integer(i) => i.to_string(),
        Yaml::Boolean(b) => b.to_string(),
        _ => String::new(),
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
