//! JSON-RPC 2.0 envelope types and the per-tool argument structs used to
//! deserialize `tools/call` arguments.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// `jsonrpc` is never read after deserialization, but keeping it a required
// `String` (not `Option`/`#[serde(default)]`) means requests missing the
// mandatory JSON-RPC 2.0 "jsonrpc" key fail to parse, which is the only
// validation we do on it — a real, if minimal, use.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Option<Value>,
}

// Arguments structs
#[derive(Debug, Deserialize)]
pub struct PathArgs {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ListDirArgs {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CopyMoveArgs {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub target_content: String,
    pub replacement_content: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    pub path: String,
    pub start_offset: Option<usize>,
    pub smart_boundary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTextArgs {
    pub search_root_or_file: String,
    pub query_string: String,
    pub is_regex: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchFileArgs {
    pub search_root: Option<String>,
    pub name_pattern: Option<String>,
    pub file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FilterSortArgs {
    pub path: String,
    pub columns: Vec<String>,
    pub deduplicate: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct QueryJsonArgs {
    pub path: String,
    pub json_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecCmdArgs {
    pub command: String,
    pub working_directory: Option<String>,
    pub timeout_seconds: Option<u64>,
}
