//! Resolves a `tools/call` request to the underlying `core-utilities-mcp-lib`
//! function and converts the outcome into the MCP `tools/call` response
//! shape.

use crate::rpc_types::{
    CopyMoveArgs, EditFileArgs, ExecCmdArgs, FilterSortArgs, JsonRpcError, JsonRpcResponse,
    ListDirArgs, PathArgs, QueryJsonArgs, ReadArgs, SearchFileArgs, SearchTextArgs, WriteFileArgs,
};
use core_utilities_mcp_lib::errors::{CoreError, CoreResult};
use core_utilities_mcp_lib::file_ops::{
    copy_file_or_directory, create_directory, delete_file_or_directory, edit_file_content,
    get_file_metadata, list_directory_contents, move_file_or_directory, write_file,
};
use core_utilities_mcp_lib::search_ops::{search_file_by_name_or_type, search_text_with_limit};
use core_utilities_mcp_lib::sys_ops::{execute_command, get_system_context};
use core_utilities_mcp_lib::text_ops::{
    filter_and_sort_matrix_columns, query_json_by_path, read_file_with_limit,
};
use serde_json::Value;
use tracing::warn;

/// Deserializes `arguments` per `name` and calls the matching
/// `core-utilities-mcp-lib` function, stringifying its JSON result. Unknown
/// tool names yield [`CoreError::General`]; malformed `arguments` fall back
/// to each tool's empty/default value rather than erroring, except
/// `edit_file_content` (whose required numeric fields have no sane default)
/// and `write_file` (an empty-string default for `content` would silently
/// write an empty file rather than surface the malformed request).
///
/// Deliberately kept as one `match` over all tool names rather than split
/// per `CODING.md`'s 40-line function guidance: each arm is the same
/// three-line "deserialize, call, stringify" shape repeated for every tool,
/// so splitting it would scatter one dispatch table across many
/// one-tool-each functions for no readability gain — exactly what that
/// guidance's own anti-fragmentation clause warns against.
pub async fn dispatch_tool_call(name: &str, arguments: Option<Value>) -> CoreResult<String> {
    let args = arguments.unwrap_or(Value::Null);
    match name {
        "list_directory_contents" => {
            let args: ListDirArgs =
                serde_json::from_value(args).unwrap_or(ListDirArgs { path: None });
            list_directory_contents(args.path).map(|v| v.to_string())
        }
        "get_file_metadata" => {
            let args: PathArgs = serde_json::from_value(args).unwrap_or(PathArgs {
                path: String::new(),
            });
            get_file_metadata(&args.path).map(|v| v.to_string())
        }
        "copy_file_or_directory" => {
            let args: CopyMoveArgs = serde_json::from_value(args).unwrap_or(CopyMoveArgs {
                source: String::new(),
                destination: String::new(),
            });
            copy_file_or_directory(&args.source, &args.destination)
                .map(|_| "Successfully copied targets.".to_string())
        }
        "move_file_or_directory" => {
            let args: CopyMoveArgs = serde_json::from_value(args).unwrap_or(CopyMoveArgs {
                source: String::new(),
                destination: String::new(),
            });
            move_file_or_directory(&args.source, &args.destination)
                .map(|_| "Successfully moved targets.".to_string())
        }
        "delete_file_or_directory" => {
            let args: PathArgs = serde_json::from_value(args).unwrap_or(PathArgs {
                path: String::new(),
            });
            delete_file_or_directory(&args.path)
                .map(|_| "Successfully deleted targets.".to_string())
        }
        "create_directory" => {
            let args: PathArgs = serde_json::from_value(args).unwrap_or(PathArgs {
                path: String::new(),
            });
            create_directory(&args.path).map(|_| "Successfully created directory.".to_string())
        }
        "write_file" => match serde_json::from_value::<WriteFileArgs>(args) {
            Ok(args) => {
                write_file(&args.path, &args.content, args.overwrite).map(|v| v.to_string())
            }
            Err(e) => Err(CoreError::Parsing(format!(
                "Invalid arguments for write_file: {}",
                e
            ))),
        },
        "edit_file_content" => match serde_json::from_value::<EditFileArgs>(args) {
            Ok(args) => edit_file_content(
                &args.path,
                args.start_line,
                args.end_line,
                &args.target_content,
                &args.replacement_content,
            )
            .map(|v| v.to_string()),
            Err(e) => Err(CoreError::Parsing(format!(
                "Invalid arguments for edit_file_content: {}",
                e
            ))),
        },
        "read_file_with_limit" => {
            let args: ReadArgs = serde_json::from_value(args).unwrap_or(ReadArgs {
                path: String::new(),
                start_offset: None,
                smart_boundary: None,
            });
            read_file_with_limit(&args.path, args.start_offset, args.smart_boundary)
                .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
        }
        "search_text_with_limit" => {
            let args: SearchTextArgs = serde_json::from_value(args).unwrap_or(SearchTextArgs {
                search_root_or_file: String::new(),
                query_string: String::new(),
                is_regex: None,
            });
            search_text_with_limit(&args.search_root_or_file, &args.query_string, args.is_regex)
                .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
        }
        "search_file_by_name_or_type" => {
            let args: SearchFileArgs = serde_json::from_value(args).unwrap_or(SearchFileArgs {
                search_root: None,
                name_pattern: None,
                file_type: None,
            });
            search_file_by_name_or_type(
                args.search_root.as_deref(),
                args.name_pattern.as_deref(),
                args.file_type.as_deref(),
            )
            .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
        }
        "filter_and_sort_matrix_columns" => {
            let args: FilterSortArgs = serde_json::from_value(args).unwrap_or(FilterSortArgs {
                path: String::new(),
                columns: Vec::new(),
                deduplicate: None,
            });
            filter_and_sort_matrix_columns(&args.path, args.columns, args.deduplicate)
                .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
        }
        "query_json_by_path" => {
            let args: QueryJsonArgs = serde_json::from_value(args).unwrap_or(QueryJsonArgs {
                path: String::new(),
                json_path: String::new(),
            });
            query_json_by_path(&args.path, &args.json_path).map(|v| v.to_string())
        }
        "get_system_context" => get_system_context().map(|v| v.to_string()),
        "execute_command" => {
            let args: ExecCmdArgs = serde_json::from_value(args).unwrap_or(ExecCmdArgs {
                command: String::new(),
                working_directory: None,
                timeout_seconds: None,
            });
            execute_command(
                &args.command,
                args.working_directory.as_deref(),
                args.timeout_seconds,
            )
            .await
            .map(|v| v.to_string())
        }
        _ => Err(CoreError::General(format!("Method not found: {}", name))),
    }
}

/// Converts a [`dispatch_tool_call`] outcome into the MCP `tools/call`
/// response shape: `Ok` becomes a single text content block, `Err` becomes
/// the same shape with `isError: true` and an `error_type` field (also
/// logged via `tracing`) so a caller can distinguish failure categories
/// without parsing the message text.
pub fn build_tool_call_response(
    id: Option<Value>,
    tool_name: &str,
    tool_result: CoreResult<String>,
) -> JsonRpcResponse {
    let content = match tool_result {
        Ok(msg) => serde_json::json!({
            "content": [{ "type": "text", "text": msg }]
        }),
        Err(e) => {
            warn!(
                tool = %tool_name,
                error_type = e.category(),
                error = %e,
                "tool call failed"
            );
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("Error: {}", e),
                    "error_type": e.category()
                }],
                "isError": true
            })
        }
    };

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(content),
        error: None,
    }
}

/// Builds the `-32602` "missing params" error response used when a
/// `tools/call` request has no `params` object.
pub fn missing_params_response(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: "Missing params".to_string(),
            data: None,
        }),
    }
}

/// Builds the `-32602` "invalid params" error response used when
/// `tools/call`'s `params` doesn't deserialize into [`crate::rpc_types::ToolCallParams`].
pub fn invalid_params_response(id: Option<Value>, error: serde_json::Error) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: format!("Invalid params: {}", error),
            data: None,
        }),
    }
}
