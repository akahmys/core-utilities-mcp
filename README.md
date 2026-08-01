# core-utilities-mcp

`core-utilities-mcp` is an AI-optimized Rust library and Model Context Protocol (MCP) server, powered by a modular, pure Rust design with optional integration of the standard UNIX utilities engine (`uutils/coreutils`).

It translates heavy, unpredictable, and raw shell commands into deterministic, JSON-structured, and line-windowed API tools. It is structured as a **Cargo Workspace** to eliminate process invocation overhead by compiling as a library, allowing direct integration into custom AI agents or standalone execution as an MCP server.

---

## 🛠️ The 15 Core Command Specification

### File Operations & Directory Management
1. `list_directory_contents` (ls): Lists contents divided into files, directories, and links.
2. `get_file_metadata` (stat, realpath): Retrieves absolute path, size, permissions, and timestamps.
3. `copy_file_or_directory` (cp)
4. `move_file_or_directory` (mv): Validates source and destination safety.
5. `delete_file_or_directory` (rm): Rigidly rejects dangerous paths.
6. `create_directory` (mkdir): Automatically creates parent directories (`mkdir -p`).
7. `write_file` (touch, `>`): Writes content to a new file, creating missing parent directories. Refuses to overwrite an existing file unless `overwrite: true` is passed.
8. `edit_file_content` (edit): Safe, hybrid search-and-replace style editor targeting a specific line range and verifying its content.

### Search & Text Control
9. `read_file_with_limit` (cat, head, tail): Paginates file reads using `start_offset` and smart truncation.
10. `search_text_with_limit` (grep): JSON-structured regex/plain text finder with context support.
11. `search_file_by_name_or_type` (find): Locates files based on name/type constraints.

### Structural Data Formatting
12. `filter_and_sort_matrix_columns` (cut, sort, uniq): Filters CSV/TSV/logs and removes duplicates natively.
13. `query_json_by_path`: Queries JSON structures using standard path queries (e.g., `data.users[0].id`).

### System & Shell Execution
14. `get_system_context` (uname, df, id): Aggregates system details (OS, CPU, Hostname, Disk free space, PID/UID info) into JSON.
15. `execute_command`: Runs a shell command with a configurable timeout (`timeout_seconds`, default `30`, capped at `300`) and an output-size guard. Accepts an optional `working_directory`, defaulting to `AI_WORKSPACE_ROOT` if set. **Not path-validated and not isolated** — no filesystem, network, CPU, or memory restriction from the host; use an OS-level sandbox (container, VM) if you need that.

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
All destructive commands (`delete_file_or_directory`, `move_file_or_directory`, `copy_file_or_directory`, `create_directory`, `write_file`, `edit_file_content`) apply path safety validation before execution. Operations on dangerous targets — the current directory under any spelling that lexically collapses to it (`.`, `./`, `a/..`, ...), `/`, `*`, `~`, `""` (empty string), paths containing a NUL byte, or paths ending with wildcards (`/*`, `/.*`) — are immediately rejected. Exact (case-insensitive) matches against a fixed deny-list of critical system directories (e.g. `/etc`, `/usr`, `/bin`, `C:\Windows`) are also rejected, while subpaths beneath them (e.g. `/etc/hosts`) remain permitted.

Read-only commands (`list_directory_contents`, `get_file_metadata`, `search_text_with_limit`, `search_file_by_name_or_type`, `read_file_with_limit`, `filter_and_sort_matrix_columns`, `query_json_by_path`) apply the same validation *except* they permit the current directory — reading or listing "here" is the normal, safe default for these tools (several default to `.` when no path is given), and unlike a mutation it cannot destroy anything. Every rejection states what to pass instead, since the caller is typically an LLM deciding how to retry.

This is a mistake-prevention guard for an AI agent going off-script, not an adversarial security boundary — it does not resolve symlinks, and provides no protection for calls that bypass it entirely (e.g. `execute_command`, which is not path-validated).

**Optional workspace confinement (`AI_WORKSPACE_ROOT`)**: if set, every path-validated tool call is restricted to that directory (and its subdirectories) — anything outside it, including via `../` traversal, is rejected. Unset (the default), there is no such restriction. It also becomes the default `working_directory` for `execute_command` when the caller doesn't specify one.
```json
"core-utilities-mcp": {
  "command": "/Users/akahmys/.cargo/bin/core-utilities-mcp",
  "env": {
    "AI_WORKSPACE_ROOT": "/Users/akahmys/projects/rad"
  }
}
```

### 4. Configurable Command Timeout (`AI_COMMAND_TIMEOUT_SECONDS`)
`execute_command` applies a wall-clock timeout to every command: an explicit per-call `timeout_seconds` argument wins, falling back to `AI_COMMAND_TIMEOUT_SECONDS` (default `30`), always clamped to at most `300` seconds.
```bash
AI_COMMAND_TIMEOUT_SECONDS=120 core-utilities-mcp
```

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
│       ├── file_ops/           # Read/delete/list/stat; mutate.rs holds copy/move/create/write/edit
│       ├── search_ops/         # High-efficiency finder and grep operations
│       ├── text_ops/           # Pagination, parsing, structured-data querying
│       └── sys_ops/            # System introspection and shell command execution
└── core-utilities-mcp/         # [Wrapper] MCP Server implementation (depends on core-utilities-mcp-lib)
    ├── Cargo.toml
    └── src/
        ├── main.rs             # Entry point, stdin/stdout JSON-RPC loop, per-method routing
        ├── rpc_types.rs        # Our own JSON-RPC 2.0 envelope + per-tool argument structs
        ├── tools.rs            # tools/list schema definitions
        └── dispatch.rs         # tools/call: resolves a tool name to its lib function
```

MCP protocol *result* types (`InitializeResult`, `ListToolsResult`, `CallToolResult`, `Tool`, `ProtocolVersion`, ...) come from the [`rust-mcp-schema`](https://crates.io/crates/rust-mcp-schema) crate rather than hand-rolled `serde_json::json!()`; the JSON-RPC 2.0 envelope itself (`rpc_types.rs`) is still our own, since that transport layer isn't `rust-mcp-schema`'s concern. `protocolVersion` in the `initialize` response comes from `ProtocolVersion::latest().to_string()`, so it tracks the crate's supported MCP version automatically on upgrade rather than being hardcoded. One resulting wire-format detail for client authors: a failed `tools/call`'s error category (e.g. `"File"`, `"Guardrail"`) lives at `content[0]._meta.error_type`, the standard MCP extension-field location, rather than as a bespoke top-level key on the content block.

---

## ⚡ Setup & Test

### Run Tests
Every command has unit tests for edge cases and limits, plus property tests for the guardrails and end-to-end JSON-RPC integration tests for the server.
```bash
cargo test --all
```

### Continuous Integration
Every push and pull request to `main` runs `scripts/check_secrets.sh --all`, `scripts/check_licenses.py`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, `cargo build`, and `cargo test` via [GitHub Actions](.github/workflows/ci.yml).

### Git Hooks
`scripts/check_secrets.sh` also runs locally as a pre-commit hook (staged changes only), catching secrets and absolute paths before they're ever committed — CI can only catch them after the fact, once they're already in history. The hook lives in `.githooks/` (tracked in git) rather than `.git/hooks/` (which isn't), so it needs a one-time opt-in per clone:
```bash
git config core.hooksPath .githooks
```
