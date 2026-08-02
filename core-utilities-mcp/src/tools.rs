//! The static `tools/list` schema: one JSON Schema object per tool exposed
//! by this server. Kept as a single data table (not split further) since
//! it's declarative content, not logic — the same rationale `CODING.md`
//! grants central enum/AST definitions.

use serde_json::{json, Value};

/// Returns the JSON array of tool definitions advertised by `tools/list`.
// Same "single data table" rationale as the module doc for
// `#[allow(clippy::too_many_lines)]`: it's one JSON literal, not logic.
#[allow(clippy::too_many_lines)]
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
            "name": "write_file",
            "description": "Writes content to a new file, creating any missing parent directories. Refuses to overwrite an existing file unless overwrite is true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                    "overwrite": { "type": "boolean", "description": "Set true to replace an existing file. Defaults to false." }
                },
                "required": ["path", "content"]
            }
        },
        {
            "name": "edit_file",
            "description": "Applies one or more non-contiguous line-range edits to a file as a single atomic transaction. Each edit's target_content (the exact current content of start_line..end_line, without any line-number prefix) is verified before anything is written; if any edit fails verification, none are applied. A single edit is just a one-element edits array. The response's line_delta reports how many lines the file grew or shrank by, so a follow-up edit can adjust line numbers without re-reading the file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to edit." },
                    "edits": {
                        "type": "array",
                        "description": "One or more edit chunks. A single edit is just a one-element array.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "start_line": { "type": "integer", "description": "1-indexed start line of the range to edit." },
                                "end_line": { "type": "integer", "description": "1-indexed end line of the range to edit (inclusive)." },
                                "target_content": { "type": "string", "description": "The exact content expected within the line range to prevent stale edits." },
                                "replacement_content": { "type": "string", "description": "The replacement content." }
                            },
                            "required": ["start_line", "end_line", "target_content", "replacement_content"]
                        }
                    }
                },
                "required": ["path", "edits"]
            }
        },
        {
            "name": "read_file",
            "description": "Reads a file starting at a 1-indexed line number, annotating each returned line as '{line_number}\\t{content}'. Use the line numbers directly as start_line/end_line for edit_file, and the raw content after the tab (without the number) as target_content. If the response's status is 'truncated', pass its next_start_line back in to continue reading.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read." },
                    "start_line": { "type": "integer", "description": "1-indexed line to start reading from. Defaults to 1." }
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
