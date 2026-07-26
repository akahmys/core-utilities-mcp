# core-utilities-mcp

`core-utilities-mcp` is an AI-optimized Rust library and Model Context Protocol (MCP) server, powered by a modular, pure Rust design with optional integration of the standard UNIX utilities engine (`uutils/coreutils`).

It translates heavy, unpredictable, and raw shell commands into deterministic, JSON-structured, and line-windowed API tools. It is structured as a **Cargo Workspace** to eliminate process invocation overhead by compiling as a library, allowing direct integration into custom AI agents or standalone execution as an MCP server.

---

## 📁 Repository Structure (Cargo Workspace)

The repository is structured as follows:

```
core-utilities-mcp/
├── Cargo.toml                  # Workspace configuration
├── core-utilities-mcp-lib/     # [Core] Independent pure Rust library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Public entry point and shared interfaces
│       ├── guardrails/         # Character limits, safety boundaries, path validation
│       ├── file_ops/           # File and directory manipulation (cp, mv, rm, mkdir, list, stat)
│       ├── search_ops/         # High-efficiency finder and grep operations
│       └── text_ops/           # Pagination, parsing, structural extraction
└── core-utilities-mcp/         # [Wrapper] MCP Server implementation (depends on core-utilities-mcp-lib)
    ├── Cargo.toml
    └── src/main.rs             # JSON-RPC MCP handlers wrapping the core library
```

---

## 🛡️ Common Guardrails

`core-utilities-mcp-lib` enforces mechanical guardrails to prevent AI failures:

### 1. Global Character Limits (`AI_COMMAND_MAX_CHARACTERS`)
All commands outputting text respect the `AI_COMMAND_MAX_CHARACTERS` environment variable (default: `8192`).
- **Smart Boundary**: Output is automatically truncated at the last newline (`\n`) before the limit. If no newline is present within the range, it is truncated exactly at the limit.
- **Metadata**: Truncated responses are returned with `status: "truncated"` and a `next_offset` value for pagination.
- **Configuration Examples**:
  - Running manually in CLI:
    ```bash
    AI_COMMAND_MAX_CHARACTERS=4096 core-utilities-mcp
    ```
  - Specifying in an MCP configuration file (e.g., `mcp.json`):
    ```json
    "core-utilities-mcp": {
      "command": "/Users/akahmys/.cargo/bin/core-utilities-mcp",
      "env": {
        "AI_COMMAND_MAX_CHARACTERS": "4096"
      }
    }
    ```

### 2. Structured Logging (`RUST_LOG`)
The server emits structured `tracing` logs to **stderr** (never stdout, which is reserved for JSON-RPC), controlled by the standard `RUST_LOG` environment variable (default: `info`). Each request logs its method; each failed tool call logs the tool name and `error_type`.
```bash
RUST_LOG=debug core-utilities-mcp
```

### 3. Path Safety Validation (`rm -rf` Water-Edge Defense)
All destructive commands apply path safety validation before execution. Operations on dangerous targets such as `.`, `/`, `*`, `~`, `""` (empty string), paths containing a NUL byte, or paths ending with wildcards (`/*`, `/.*`) are immediately rejected. Exact (case-insensitive) matches against a fixed deny-list of critical system directories (e.g. `/etc`, `/usr`, `/bin`, `C:\Windows`) are also rejected, while subpaths beneath them (e.g. `/etc/hosts`) remain permitted.

---

## 🛠️ The 15 Core Command Specification

### File Operations & Directory Management
1. `list_directory_contents` (ls): Lists contents divided into files, directories, and links.
2. `get_file_metadata` (stat, realpath): Retrieves absolute path, size, permissions, and timestamps.
3. `copy_file_or_directory` (cp)
4. `move_file_or_directory` (mv): Validates source and destination safety.
5. `delete_file_or_directory` (rm): Rigidly rejects dangerous paths.
6. `create_directory` (mkdir): Automatically creates parent directories (`mkdir -p`).
7. `edit_file_content` (edit): Safe, hybrid search-and-replace style editor targeting a specific line range and verifying its content.

### Search & Text Control
8. `read_file_with_limit` (cat, head, tail): Paginates file reads using `start_offset` and smart truncation.
9. `search_text_with_limit` (grep): JSON-structured regex/plain text finder with context support.
10. `search_file_by_name_or_type` (find): Locates files based on name/type constraints.

### Structural Data Formatting
11. `filter_and_sort_matrix_columns` (cut, sort, uniq): Filters CSV/TSV/logs and removes duplicates natively.
12. `extract_code_skeleton`: Uses tree-sitter or regex parses to extract class/function structures, saving ~90% of tokens.
13. `query_json_by_path`: Queries JSON structures using standard path queries (e.g., `data.users[0].id`).

### System & Sandbox
14. `get_system_context` (uname, df, id): Aggregates system details (OS, CPU, Hostname, Disk free space, PID/UID info) into JSON.
15. `execute_command_in_sandbox`: Runs commands in a constrained sandbox environment.



---

## ⚡ Setup & Test

### Run Unit Tests
Every command contains unit tests for edge cases and limits.
```bash
cargo test --all
```
