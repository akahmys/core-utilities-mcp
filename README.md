# mcp-uutils

`mcp-uutils` is an AI-optimized Rust library and Model Context Protocol (MCP) server, powered by a modular, pure Rust design with optional integration of the standard UNIX utilities engine (`uutils/coreutils`).

It translates heavy, unpredictable, and raw shell commands into deterministic, JSON-structured, and line-windowed API tools. It is structured as a **Cargo Workspace** to eliminate process invocation overhead by compiling as a library, allowing direct integration into custom AI agents or standalone execution as an MCP server.

---

## 📁 Repository Structure (Cargo Workspace)

The repository is structured as follows:

```
mcp-uutils/
├── Cargo.toml                  # Workspace configuration
├── mcp-uutils-lib/             # [Core] Independent pure Rust library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Public entry point and shared interfaces
│       ├── guardrails/         # Character limits, safety boundaries, path validation
│       ├── file_ops/           # File and directory manipulation (cp, mv, rm, mkdir, list, stat)
│       ├── search_ops/         # High-efficiency finder and grep operations
│       └── text_ops/           # Pagination, parsing, structural extraction
└── mcp-uutils-mcp/             # [Wrapper] MCP Server implementation (depends on mcp-uutils-lib)
    ├── Cargo.toml
    └── src/main.rs             # JSON-RPC MCP handlers wrapping the core library
```

---

## 🛡️ Common Guardrails

`mcp-uutils-lib` enforces mechanical guardrails to prevent AI failures:

### 1. Global Character Limits (`AI_COMMAND_MAX_CHARACTERS`)
All commands outputting text respect the `AI_COMMAND_MAX_CHARACTERS` environment variable (default: `8192`).
- **Smart Boundary**: Output is automatically truncated at the last newline (`\n`) before the limit. If no newline is present within the range, it is truncated exactly at the limit.
- **Metadata**: Truncated responses are returned with `status: "truncated"` and a `next_offset` value for pagination.

### 2. Path Safety Validation (`rm -rf` Water-Edge Defense)
All destructive commands apply path safety validation before execution. Operations on dangerous targets such as `.`, `/`, `*`, `~`, `""` (empty string), or paths ending with wildcards (`/*`, `/.*`) are immediately rejected.

---

## 🛠️ The 13 Core Command Specification

### File Operations & Directory Management
1. `list_directory_contents` (ls): Lists contents divided into files, directories, and links.
2. `get_file_metadata` (stat, realpath): Retrieves absolute path, size, permissions, and timestamps.
3. `copy_file_or_directory` (cp)
4. `move_file_or_directory` (mv): Validates source and destination safety.
5. `delete_file_or_directory` (rm): Rigidly rejects dangerous paths.
6. `create_directory` (mkdir): Automatically creates parent directories (`mkdir -p`).

### Search & Text Control
7. `read_file_with_limit` (cat, head, tail): Paginates file reads using `start_offset` and smart truncation.
8. `search_text_with_limit` (grep): JSON-structured regex/plain text finder with context support.
9. `search_file_by_name_or_type` (find): Locates files based on name/type constraints.

### Structural Data Formatting
10. `filter_and_sort_matrix_columns` (cut, sort, uniq): Filters CSV/TSV/logs and removes duplicates natively.
11. `extract_code_skeleton`: Uses tree-sitter or regex parses to extract class/function structures, saving ~90% of tokens.
12. `query_json_by_path`: Queries JSON structures using standard path queries (e.g., `data.users[0].id`).

### System & Sandbox
13. `get_system_context` (uname, df, id): Aggregates system details (OS, CPU, Hostname, Disk free space, PID/UID info) into JSON.
14. `execute_command_in_sandbox`: Runs commands in a constrained sandbox environment.

---

## ⚡ Setup & Test

### Run Unit Tests
Every command contains unit tests for edge cases and limits.
```bash
cargo test --all
```
