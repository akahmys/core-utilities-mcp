# Project Roadmap & Execution Log

## 🗺️ Roadmap (Completed Milestones)

### Phase 1: Workspace & Guardrails Implementation
*   [x] **AWU 1.1: Workspace Cargo setup**
*   [x] **AWU 1.2: Path safety & Smart Output Truncation Guardrails**
*   [x] **AWU 1.3: Safe Delete and Read File with Limit Prototype**

### Phase 2: Full Tool Suite & MCP Server
*   [x] **AWU 2.1: Implement MCP wrapper and tool definitions**
*   [x] **AWU 2.2: Implement remaining core uutils/coreutils tools**
*   [x] **AWU 2.3: Integrate Rollback Manager and Execution Sandbox**
*   [x] **AWU 2.4: Reinstall renamed mcp-uutils binary**
*   [x] **AWU 2.5: Rename project to core-utilities-mcp**

---

## 🎯 Short-Term Plan

*   [x] **AWU 4.4: Refactor `tools/call` Error Handling**
*   [x] **AWU 3.1: Fix Clippy Warnings** (needless_borrows_for_generic_args)
*   [x] **AWU 3.2: Apply Standard Formatting** (cargo fmt)
*   [x] **AWU 3.3: Dependency Audit** (cargo audit - *Skipped: tool not in environment*)

### Phase 2: Library Robustness (Completed)
*   [x] **AWU 4.1: Implement Structured Error Handling** (thiserror)
*   [x] **AWU 4.2: Enhance Guardrails & Safety**
*   [x] **AWU 4.3: Documentation & API Design** (doc comments & examples)

### Phase 3: Server Maturity (Planned)
*   [ ] **AWU 5.1: Integrate Structured Logging** (tracing)
*   [ ] **AWU 5.2: Refine JSON-RPC Loop** (graceful shutdown)
*   [ ] **AWU 5.3: Add Integration Testing** (end-to-end JSON-RPC)

### Phase 4: Quality Assurance & Automation (Planned)
*   [ ] **AWU 6.1: Expand Test Coverage** (unit, integration, property)
*   [ ] **AWU 6.2: CI/CD Pipeline Setup** (GitHub Actions)

---

## 📝 Execution Log

| AWU ID | Task Name | Status | Result / Notes |
| :--- | :--- | :--- | :--- |
| **1.1** | **Workspace Cargo setup** | `[✅] Completed` | Converted crate to a Cargo Workspace layout. |
| 1.2 | Path safety & Smart Output Truncation | `[✅] Completed` | Implemented validate_path_safety and truncate_output. |
| 1.3 | Safe Delete and Read File with Limit | `[✅] Completed` | Implemented delete_file_or_directory and read_file_with_limit. |
| 2.1 | MCP Server integration | `[✅] Completed` | Integrated all tools into JSON-RPC schema. |
| 2.2 | Full Tool Suite | `[✅] Completed` | Implemented all 14 core tools. |
| 2.3 | Rollback / Sandbox Integration | `[✅] Completed` | Sandbox runner with stdout guard integrated. |
| 2.4 | Reinstall renamed mcp-uutils binary | `[✅] Completed` | Successfully reinstalled mcp-uutils to cargo bin. |
| 2.5 | Rename project to core-utilities-mcp | `[✅] Completed` | Successfully renamed directories, workspace crates (`core-utilities-mcp` & `core-utilities-mcp-lib`), Rust imports, binary output, license script, and all documentation. Installed binary to `~/.cargo/bin/core-utilities-mcp`. |
| 3.0 | Synthesize Technical Audit & Refactoring Plan | `[✅] Completed` | Plan generated and added to roadmap. |
| **3.1** | **Fix Clippy Warnings** | `[✅] Completed` | Fixed `needless_borrows_for_generic_args` in `sys_ops/mod.rs`. |
| **3.2** | **Apply Standard Formatting** | `[✅] Completed` | Ran `cargo fmt`. |
| **3.3** | **Dependency Audit** | `[✅] Completed` | *Skipped: tool not in environment.* |
| **4.1** | **Implement Structured Error Handling** | `[✅] Completed` | Added `errors.rs` with `thiserror`-based `CoreError`/`CoreResult`. Migrated `file_ops`, `guardrails`, `search_ops`, `text_ops`, and `sys_ops` off `Result<_, String>`. |
| **4.4** | **Refactor `tools/call` Error Handling** | `[✅] Completed` | All `tools/call` arms now return `Result<String, CoreError>`; error responses include `error_type` from `CoreError::category()`. Verified via manual JSON-RPC smoke test and `cargo test --workspace` (19 passed). |
| **4.2** | **Enhance Guardrails & Safety** | `[✅] Completed` | `validate_path_safety` now also rejects NUL-byte paths and exact (case-insensitive) matches against a deny-list of critical system directories (`/etc`, `/bin`, `/sbin`, `/usr`, `/boot`, `/dev`, `/proc`, `/sys`, `/root`, `/System`, `/Library`, `C:\Windows`, `C:\Program Files`); subpaths remain permitted. Corrected `execute_command_in_sandbox` docs to no longer claim an enforced memory limit (only timeout + output-size kill switch exist). README updated. |
| **4.3** | **Documentation & API Design** | `[✅] Completed` | Added crate-level (`//!`) and per-module doc comments across `guardrails`, `file_ops`, `search_ops`, `text_ops`, `sys_ops`, each with `# Errors` and `# Examples` sections. `cargo test --doc` grew from 0 to 17 passing doctests. |

---

## 🚀 Next Steps
1. **AWU 5.1: Integrate Structured Logging** (tracing).
2. **AWU 5.2: Refine JSON-RPC Loop** (graceful shutdown).
3. **AWU 5.3: Add Integration Testing** (end-to-end JSON-RPC).
