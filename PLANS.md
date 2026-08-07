# Project Roadmap & Execution Log

## 🗺️ Roadmap (Completed Milestones)

- [x] **Phase 1: Workspace & Guardrails Implementation** (AWU 1.1 – 1.3)
- [x] **Phase 2: Full Tool Suite & MCP Server** (AWU 2.1 – 2.5)
- [x] **Phase 3: Library & Codebase Hardening** (AWU 3.1 – 4.4)
- [x] **Phase 4: Server Maturity & Testing** (AWU 5.1 – 6.2)
- [x] **Phase 5: Design Review & Tool Alignment** (AWU 7.1 – 7.5)
- [x] **Phase 6: Guardrails Refinement & Error Hints** (AWU 7.6 – 7.8)
- [x] **Phase 7: Full Audit, TOML/YAML & v1.0.0 Release** (AWU 7.9 – 7.16)
- [x] **Phase 8: Optimization, Code Splitting & Security Migration** (AWU 8.1 – 8.17)

---

## 🎯 Short-Term Plan

### Phase 8: Deep Refactoring & Security Migration (Completed)
*   [x] **AWU 8.1: Fix Stream Truncation Bug in `sys_ops::read_output_streams`**
*   [x] **AWU 8.2: Unique Temporary File Names in `file_ops::mutate::atomic_write`**
*   [x] **AWU 8.3: Memory & Stream Optimization for `search_ops` & `read_file`**
*   [x] **AWU 8.4: Path Verification & Existence Helper**
*   [x] **AWU 8.5: Streamlined Deserialization in `dispatch.rs`**
*   [x] **AWU 8.6: Apply `ENV_MUTEX` Lock in Property Tests & Integration Tests**
*   [x] **AWU 8.7–8.8: Reorder `README.md` to User-Centric Flow**
*   [x] **AWU 8.9: Fix CI Clippy Failure from Upstream Stable-Toolchain Drift**
*   [x] **AWU 8.10: Absorb `file-edit-mcp`'s Edit/Read Improvements**
*   [x] **AWU 8.11: Command Composition Review & TOML/YAML Support**
*   [x] **AWU 8.12: Replace `serde_yaml` with `yaml-rust2` (Pure-Rust)**
*   [x] **AWU 8.13: Full Codebase Audit — File/Function Splits & Dead Code Removal**
*   [x] **AWU 8.14: Documentation & Policy-File Review**
*   [x] **AWU 8.15: Migrate License Audit from `check_licenses.py` to `cargo-deny`**
*   [x] **AWU 8.16: Migrate Secrets Audit from `check_secrets.sh` to `betterleaks`**
*   [x] **AWU 8.17: Add Personal Name Check to `betterleaks` Rules**

---

## 📝 Execution Log

| AWU ID | Task Name | Status | Summary / Result |
| :--- | :--- | :--- | :--- |
| **1.1** | Workspace Cargo setup | `[✅] Completed` | Converted crate to a Cargo Workspace layout (`core-utilities-mcp` and `core-utilities-mcp-lib`). |
| **1.2** | Path safety & Output Truncation | `[✅] Completed` | Implemented `validate_path_safety` and `truncate_output` guardrails. |
| **1.3** | Safe Delete and Read File Prototype | `[✅] Completed` | Implemented safe file deletion and line-bounded file reading. |
| **2.1** | MCP Server integration | `[✅] Completed` | Integrated initial tools into JSON-RPC schema and transport loop. |
| **2.2** | Full Tool Suite | `[✅] Completed` | Implemented 14 core UNIX utility tool abstractions in pure Rust. |
| **2.3** | Rollback / Sandbox Integration | `[⚠️] Corrected` | Corrected historical entry; rollback manager dropped in favor of execution stdout/timeout guards. |
| **2.4** | Reinstall renamed binary | `[✅] Completed` | Reinstalled compiled server binary to `~/.cargo/bin`. |
| **2.5** | Rename project to core-utilities-mcp | `[✅] Completed` | Renamed workspace crates, imports, binary output, and documentation. |
| **3.1** | Fix Clippy Warnings | `[✅] Completed` | Resolved `needless_borrows_for_generic_args` warnings across workspace. |
| **3.2** | Apply Standard Formatting | `[✅] Completed` | Applied `cargo fmt` formatting across workspace. |
| **3.3** | Dependency Audit | `[✅] Completed` | Audited dependency tree. |
| **4.1** | Structured Error Handling | `[✅] Completed` | Migrated internal errors to `thiserror`-based `CoreError`/`CoreResult`. |
| **4.2** | Enhance Guardrails & Safety | `[✅] Completed` | Added system directory deny-list and NUL-byte path rejection. |
| **4.3** | Documentation & API Design | `[✅] Completed` | Added comprehensive doc comments and doctests across library modules. |
| **4.4** | Refactor `tools/call` Error Handling | `[✅] Completed` | Structured MCP error responses with `error_type` metadata. |
| **5.1** | Integrate Structured Logging | `[✅] Completed` | Integrated `tracing` subscriber output to stderr gated by `RUST_LOG`. |
| **5.2** | Refine JSON-RPC Loop | `[✅] Completed` | Implemented async stdin loop with graceful shutdown signal handling. |
| **5.3** | Add Integration Testing | `[✅] Completed` | Added end-to-end JSON-RPC integration test suite over stdio pipes. |
| **6.1** | Expand Test Coverage | `[✅] Completed` | Added `proptest` property tests and isolated unit tests into companion `tests.rs` files. |
| **6.2** | CI/CD Pipeline Setup | `[✅] Completed` | Created GitHub Actions workflow running fmt, clippy, build, and test. |
| **7.1** | Optional Workspace-Root Confinement | `[✅] Completed` | Added `AI_WORKSPACE_ROOT` environment variable restriction for path safety. |
| **7.2** | Correct "Sandbox" Terminology | `[✅] Completed` | Updated documentation to reflect command execution guardrail behavior accurately. |
| **7.3** | Remove `extract_code_skeleton` | `[✅] Completed` | Removed non-uniform code parser tool to maintain core utilities scope. |
| **7.4** | Command Execution Timeout & Dir | `[✅] Completed` | Added `timeout_seconds` and `working_directory` arguments to `execute_command`. |
| **7.5** | Rename `execute_command_in_sandbox` | `[✅] Completed` | Renamed tool to `execute_command`. |
| **7.6** | Path Safety Split & Remediation | `[✅] Completed` | Split `validate_path_safety` and `validate_read_path_safety` (permitting `.`). |
| **7.7** | Error Message Remediation Hints | `[✅] Completed` | Enhanced error messages with actionable self-correction guidance for LLM callers. |
| **7.8** | Sync Path Safety Documentation | `[✅] Completed` | Updated `README.md` and `ARCHITECTURE.md` to reflect path safety validation split. |
| **7.9** | Full Codebase Compliance Sweep | `[✅] Completed` | Removed unused dependencies and refactored functions to satisfy `CODING.md` limits. |
| **7.10** | Wire Pre-commit Hooks & CI | `[✅] Completed` | Configured `.githooks/pre-commit` and CI secret scanning. |
| **7.11** | Add `write_file` Tool | `[✅] Completed` | Added `write_file` tool with explicit `overwrite` flag and parent dir creation. |
| **7.12** | Enforce `clippy::pedantic` | `[✅] Completed` | Resolved all pedantic lints and enabled `-D clippy::pedantic` in CI. |
| **7.13** | Documentation & Parameter Audit | `[✅] Completed` | Cleaned up unused parameters and synced module doc comments. |
| **7.14** | Tool-Scope Review & v1.0.0 | `[✅] Completed` | Verified 15-tool composition and bumped workspace crate versions to 1.0.0. |
| **7.15** | MCP Schema Types Integration | `[✅] Completed` | Integrated `rust-mcp-schema` (0.10.3) for typed MCP response objects. |
| **7.16** | Global MCP Configuration Clean-up | `[✅] Completed` | Updated system-wide MCP configs (`~/.pi`, `~/.rad`, `~/.gemini`) to binary path. |
| **8.1** | Stream Truncation Fix | `[✅] Completed` | Tracked `stdout`/`stderr` EOF flags independently in `read_output_streams`. |
| **8.2** | Unique Temp File Names | `[✅] Completed` | Generated atomic PID + sequence temp filenames in `atomic_write`. |
| **8.3** | Stream Memory Optimization | `[✅] Completed` | Bounded character buffer allocation in `read_file` and `search_text`. |
| **8.4** | Path Verification Helper | `[✅] Completed` | Extracted `ensure_existing_read_path` helper to reduce code duplication. |
| **8.5** | Deserialization Cleanup | `[✅] Completed` | Derived `Default` on argument structs for cleaner `dispatch.rs` handling. |
| **8.6** | Test Suite Hardening | `[✅] Completed` | Applied `ENV_MUTEX` locking in property tests and added write integration tests. |
| **8.7–8.8** | README Flow Optimization | `[✅] Completed` | Reordered `README.md` to prioritize user-facing tool specifications. |
| **8.9** | Fix CI Toolchain Drift | `[✅] Completed` | Fixed `clippy::manual_assert_eq` warning under Rust 1.97 toolchain. |
| **8.10** | Absorb `file-edit-mcp` | `[✅] Completed` | Upgraded `edit_file`/`read_file` to multi-chunk atomic matching and line delta. |
| **8.11** | Command Composition Review | `[✅] Completed` | Added TOML/YAML support to `query_data_by_path` and renamed 5 tools. |
| **8.12** | Pure-Rust YAML Parser | `[✅] Completed` | Replaced deprecated `serde_yaml` with safe pure-Rust `yaml-rust2`. |
| **8.13** | Codebase Audit & Module Splits | `[✅] Completed` | Split oversized files (`mutate`, `path_safety`, `read`, `query`, `matrix`). |
| **8.14** | Policy & Help Flag Sync | `[✅] Completed` | Updated `--help` output, binary flags, and documentation claims. |
| **8.15** | Migrate License Audit | `[✅] Completed` | Swapped `check_licenses.py` for standard `cargo-deny` (`deny.toml`). |
| **8.16** | Migrate Secrets Audit | `[✅] Completed` | Swapped `check_secrets.sh` for `betterleaks` (`.betterleaks.toml`). |
| **8.17** | Personal Name Audit Rule | `[✅] Completed` | Added personal name rule to `.betterleaks.toml` & `.gitleaks.toml`. |

---

## 🚀 Next Steps

All planned roadmap phases (Phases 1–8, AWU 1.1–8.17) are 100% completed, fully audited, and passing. The codebase is clean, warning-free (`clippy::pedantic`), covered by 84 tests, and governed by `cargo-deny` and `betterleaks`.
