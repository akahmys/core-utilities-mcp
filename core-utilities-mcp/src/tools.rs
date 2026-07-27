//! The static `tools/list` schema: one JSON Schema object per tool exposed
//! by this server. Kept as a single data table (not split further) since
//! it's declarative content, not logic — the same rationale `CODING.md`
//! grants central enum/AST definitions.

use serde_json::{json, Value};

/// Returns the JSON array of tool definitions advertised by `tools/list`.
pub fn tool_definitions() -> Value {
    json!([
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
            "name": "execute_command",
            "description": "Runs a shell command with a wall-clock timeout and an output-size guard. Provides no filesystem, network, CPU, or memory isolation from the host, and neither the command nor working_directory are subject to path-safety validation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "working_directory": { "type": "string", "description": "Directory to run the command in. Defaults to AI_WORKSPACE_ROOT if set, otherwise the server's own working directory." },
                    "timeout_seconds": { "type": "integer", "description": "Wall-clock timeout in seconds (default 30, capped at 300)." }
                },
                "required": ["command"]
            }
        }
    ])
}
