mod dispatch;
mod rpc_types;
mod tools;

use rpc_types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolCallParams};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

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
            () = shutdown_signal() => {
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
        () = ctrl_c => {},
        () = terminate => {},
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
                    message: format!("Parse error: {e}"),
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

/// Routes a parsed request to its per-method handler by JSON-RPC `method`.
async fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" | "initialized" => handle_initialize(id),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tools_call(id, req.params).await,
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

/// Handles both `initialize` (advertises protocol version, capabilities,
/// and server info) and `initialized` (acknowledged with an empty result)
/// — the two handshake notifications an MCP client sends before its first
/// real request.
fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
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

/// Handles `tools/list` by wrapping [`tools::tool_definitions`] in the
/// expected `{"tools": [...]}` envelope.
fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({ "tools": tools::tool_definitions() })),
        error: None,
    }
}

/// Handles `tools/call`: validates `params` is present and parses as
/// [`ToolCallParams`], then delegates to [`dispatch::dispatch_tool_call`]
/// and wraps the outcome via [`dispatch::build_tool_call_response`].
async fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let Some(params) = params else {
        return dispatch::missing_params_response(id);
    };

    let call_params: ToolCallParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => return dispatch::invalid_params_response(id, &e),
    };

    let tool_result = dispatch::dispatch_tool_call(&call_params.name, call_params.arguments).await;
    dispatch::build_tool_call_response(id, &call_params.name, tool_result)
}
