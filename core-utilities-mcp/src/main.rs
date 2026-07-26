use core_utilities_mcp_lib::errors::CoreError;
use core_utilities_mcp_lib::file_ops::{
    copy_file_or_directory, create_directory, delete_file_or_directory, edit_file_content,
    get_file_metadata, list_directory_contents, move_file_or_directory,
};
use core_utilities_mcp_lib::search_ops::{search_file_by_name_or_type, search_text_with_limit};
use core_utilities_mcp_lib::sys_ops::{execute_command_in_sandbox, get_system_context};
use core_utilities_mcp_lib::text_ops::{
    extract_code_skeleton, filter_and_sort_matrix_columns, query_json_by_path, read_file_with_limit,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    arguments: Option<Value>,
}

// Arguments structs
#[derive(Debug, Deserialize)]
struct PathArgs {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ListDirArgs {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopyMoveArgs {
    source: String,
    destination: String,
}

#[derive(Debug, Deserialize)]
struct EditFileArgs {
    path: String,
    start_line: usize,
    end_line: usize,
    target_content: String,
    replacement_content: String,
}

#[derive(Debug, Deserialize)]
struct ReadArgs {
    path: String,
    start_offset: Option<usize>,
    smart_boundary: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SearchTextArgs {
    search_root_or_file: String,
    query_string: String,
    is_regex: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SearchFileArgs {
    search_root: Option<String>,
    name_pattern: Option<String>,
    file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FilterSortArgs {
    path: String,
    columns: Vec<String>,
    deduplicate: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct QueryJsonArgs {
    path: String,
    json_path: String,
}

#[derive(Debug, Deserialize)]
struct ExecCmdArgs {
    command: String,
}

/// Initializes a `tracing` subscriber that writes structured logs to
/// stderr, controlled by `RUST_LOG` (defaulting to `info`). Logs must never
/// go to stdout, which is reserved exclusively for JSON-RPC responses.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let arg = &args[1];
        if arg == "-h" || arg == "--help" {
            println!("core-utilities-mcp - AI-optimized MCP server powered by uutils/coreutils");
            println!("\nUsage:");
            println!("  core-utilities-mcp [options]");
            println!("\nOptions:");
            println!("  -h, --help     Print help information");
            println!("  -v, --version  Print version information");
            println!("\nEnvironment Variables:");
            println!("  AI_COMMAND_MAX_CHARACTERS  Maximum characters returned in output (default: 8192)");
            return Ok(());
        } else if arg == "-v" || arg == "--version" {
            println!("core-utilities-mcp 0.1.0");
            return Ok(());
        }
    }

    run_server().await
}

/// Reads JSON-RPC requests from stdin one line at a time and dispatches
/// them, until stdin closes, a read error occurs, or a shutdown signal
/// (`Ctrl+C`, or `SIGTERM` on Unix) is received. Async stdin reading lets
/// the shutdown signal interrupt a blocked read, instead of waiting for the
/// next line to arrive before the process can exit.
async fn run_server() -> anyhow::Result<()> {
    info!("core-utilities-mcp starting; awaiting JSON-RPC requests on stdin");

    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    loop {
        tokio::select! {
            biased;
            _ = shutdown_signal() => {
                info!("shutdown signal received; core-utilities-mcp shutting down");
                // `tokio::io::stdin()` reads via a dedicated blocking OS
                // thread; if it is still parked in a blocking read (the
                // normal state for a long-lived MCP client connection), the
                // Tokio runtime's Drop would hang forever waiting for it.
                // Exit immediately rather than unwinding back through main.
                std::process::exit(0);
            }
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => process_line(&line).await?,
                    Ok(None) => {
                        info!("stdin closed; core-utilities-mcp shutting down");
                        break;
                    }
                    Err(e) => {
                        warn!(error = %e, "error reading stdin; core-utilities-mcp shutting down");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Resolves once a `Ctrl+C` (or, on Unix, a `SIGTERM`) is received, letting
/// callers race it against other work via `tokio::select!` for graceful
/// shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Parses and dispatches a single JSON-RPC request line, printing the
/// response to stdout unless the request was a notification (no `id`).
/// Malformed input yields a JSON-RPC parse-error response rather than
/// aborting the server.
async fn process_line(line: &str) -> anyhow::Result<()> {
    if line.trim().is_empty() {
        return Ok(());
    }

    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "failed to parse JSON-RPC request");
            let err_resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                    data: None,
                }),
            };
            println!("{}", serde_json::to_string(&err_resp)?);
            return Ok(());
        }
    };

    debug!(method = %req.method, "received JSON-RPC request");
    let is_notification = req.id.is_none();
    let response = handle_request(req).await;

    if !is_notification {
        println!("{}", serde_json::to_string(&response)?);
    }

    Ok(())
}

async fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "core-utilities-mcp",
                    "version": "0.1.0"
                }
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }
        "initialized" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({})),
            error: None,
        },
        "tools/list" => {
            let tools = serde_json::json!({
                "tools": [
                    {
                        "name": "list_directory_contents",
                        "description": "Lists contents of a directory divided into files, directories, and links.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string", "description": "Target directory path." }
                            }
                        }
                    },
                    {
                        "name": "get_file_metadata",
                        "description": "Retrieves realpath, size, permissions, and modified timestamps for a path.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "copy_file_or_directory",
                        "description": "Copy a file or folder recursively to a destination path.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": { "type": "string" },
                                "destination": { "type": "string" }
                            },
                            "required": ["source", "destination"]
                        }
                    },
                    {
                        "name": "move_file_or_directory",
                        "description": "Move a file or folder safely to a destination path.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": { "type": "string" },
                                "destination": { "type": "string" }
                            },
                            "required": ["source", "destination"]
                        }
                    },
                    {
                        "name": "delete_file_or_directory",
                        "description": "Safely delete a file or directory with path validations.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "create_directory",
                        "description": "Creates a directory including intermediate parents (mkdir -p).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "edit_file_content",
                        "description": "Safe, hybrid search-and-replace style editor targeting a specific line range and verifying its content before replacement.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string", "description": "The file path to edit." },
                                "start_line": { "type": "integer", "description": "1-indexed start line of the range to edit." },
                                "end_line": { "type": "integer", "description": "1-indexed end line of the range to edit (inclusive)." },
                                "target_content": { "type": "string", "description": "The exact content expected within the line range to prevent stale edits." },
                                "replacement_content": { "type": "string", "description": "The replacement content." }
                            },
                            "required": ["path", "start_line", "end_line", "target_content", "replacement_content"]
                        }
                    },
                    {
                        "name": "read_file_with_limit",
                        "description": "Read file contents starting from an optional offset, respecting character boundaries and limits.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "start_offset": { "type": "integer" },
                                "smart_boundary": { "type": "boolean" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "search_text_with_limit",
                        "description": "Performs high-efficiency regex or substring search on text files under a directory.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "search_root_or_file": { "type": "string" },
                                "query_string": { "type": "string" },
                                "is_regex": { "type": "boolean" }
                            },
                            "required": ["search_root_or_file", "query_string"]
                        }
                    },
                    {
                        "name": "search_file_by_name_or_type",
                        "description": "Finds files or directories by name patterns or types under a root directory.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "search_root": { "type": "string" },
                                "name_pattern": { "type": "string" },
                                "file_type": { "type": "string", "enum": ["file", "directory", "dir", "symlink"] }
                            }
                        }
                    },
                    {
                        "name": "filter_and_sort_matrix_columns",
                        "description": "Filters, sorts, and optionally deduplicates columns from CSV/TSV data.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "columns": { "type": "array", "items": { "type": "string" } },
                                "deduplicate": { "type": "boolean" }
                            },
                            "required": ["path", "columns"]
                        }
                    },
                    {
                        "name": "extract_code_skeleton",
                        "description": "Strips local block execution logic and functions to yield a skeleton representation of classes, functions, and definitions.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "query_json_by_path",
                        "description": "Queries a JSON file using a path query syntax (e.g. data.users[0].id).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "json_path": { "type": "string" }
                            },
                            "required": ["path", "json_path"]
                        }
                    },
                    {
                        "name": "get_system_context",
                        "description": "Aggregates system metadata, disk free space, user IDs, and environment parameters.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "execute_command_in_sandbox",
                        "description": "Runs a shell command with a wall-clock timeout and an output-size guard. Provides no filesystem, network, CPU, or memory isolation from the host, and is not subject to path-safety validation.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" }
                            },
                            "required": ["command"]
                        }
                    }
                ]
            });
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(tools),
                error: None,
            }
        }
        "tools/call" => {
            let params = match req.params {
                Some(p) => p,
                None => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing params".to_string(),
                            data: None,
                        }),
                    };
                }
            };

            let call_params: ToolCallParams = match serde_json::from_value(params) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: format!("Invalid params: {}", e),
                            data: None,
                        }),
                    };
                }
            };

            let tool_result = match call_params.name.as_str() {
                "list_directory_contents" => {
                    let args: ListDirArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(ListDirArgs { path: None });
                    list_directory_contents(args.path).map(|v| v.to_string())
                }
                "get_file_metadata" => {
                    let args: PathArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(PathArgs {
                                path: String::new(),
                            });
                    get_file_metadata(&args.path).map(|v| v.to_string())
                }
                "copy_file_or_directory" => {
                    let args: CopyMoveArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(CopyMoveArgs {
                                source: String::new(),
                                destination: String::new(),
                            });
                    copy_file_or_directory(&args.source, &args.destination)
                        .map(|_| "Successfully copied targets.".to_string())
                }
                "move_file_or_directory" => {
                    let args: CopyMoveArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(CopyMoveArgs {
                                source: String::new(),
                                destination: String::new(),
                            });
                    move_file_or_directory(&args.source, &args.destination)
                        .map(|_| "Successfully moved targets.".to_string())
                }
                "delete_file_or_directory" => {
                    let args: PathArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(PathArgs {
                                path: String::new(),
                            });
                    delete_file_or_directory(&args.path)
                        .map(|_| "Successfully deleted targets.".to_string())
                }
                "create_directory" => {
                    let args: PathArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(PathArgs {
                                path: String::new(),
                            });
                    create_directory(&args.path)
                        .map(|_| "Successfully created directory.".to_string())
                }
                "edit_file_content" => {
                    let args_res: Result<EditFileArgs, _> =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null));
                    match args_res {
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
                    }
                }
                "read_file_with_limit" => {
                    let args: ReadArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(ReadArgs {
                                path: String::new(),
                                start_offset: None,
                                smart_boundary: None,
                            });
                    read_file_with_limit(&args.path, args.start_offset, args.smart_boundary)
                        .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
                }
                "search_text_with_limit" => {
                    let args: SearchTextArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(SearchTextArgs {
                                search_root_or_file: String::new(),
                                query_string: String::new(),
                                is_regex: None,
                            });
                    search_text_with_limit(
                        &args.search_root_or_file,
                        &args.query_string,
                        args.is_regex,
                    )
                    .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
                }
                "search_file_by_name_or_type" => {
                    let args: SearchFileArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(SearchFileArgs {
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
                    let args: FilterSortArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(FilterSortArgs {
                                path: String::new(),
                                columns: Vec::new(),
                                deduplicate: None,
                            });
                    filter_and_sort_matrix_columns(&args.path, args.columns, args.deduplicate)
                        .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
                }
                "extract_code_skeleton" => {
                    let args: PathArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(PathArgs {
                                path: String::new(),
                            });
                    extract_code_skeleton(&args.path)
                        .map(|res| serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()))
                }
                "query_json_by_path" => {
                    let args: QueryJsonArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(QueryJsonArgs {
                                path: String::new(),
                                json_path: String::new(),
                            });
                    query_json_by_path(&args.path, &args.json_path).map(|v| v.to_string())
                }
                "get_system_context" => get_system_context().map(|v| v.to_string()),
                "execute_command_in_sandbox" => {
                    let args: ExecCmdArgs =
                        serde_json::from_value(call_params.arguments.unwrap_or(Value::Null))
                            .unwrap_or(ExecCmdArgs {
                                command: String::new(),
                            });
                    match execute_command_in_sandbox(&args.command).await {
                        Ok(v) => Ok(v.to_string()),
                        Err(e) => Err(e),
                    }
                }
                _ => Err(CoreError::General(format!(
                    "Method not found: {}",
                    call_params.name
                ))),
            };

            match tool_result {
                Ok(msg) => {
                    let content = serde_json::json!({
                        "content": [
                            {
                                "type": "text",
                                "text": msg
                            }
                        ]
                    });
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(content),
                        error: None,
                    }
                }
                Err(e) => {
                    warn!(
                        tool = %call_params.name,
                        error_type = e.category(),
                        error = %e,
                        "tool call failed"
                    );
                    let content = serde_json::json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Error: {}", e),
                                "error_type": e.category()
                            }
                        ],
                        "isError": true
                    });
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(content),
                        error: None,
                    }
                }
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        },
    }
}
