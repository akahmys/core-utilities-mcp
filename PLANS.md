# Project Roadmap & Execution Log

## 🗺️ Roadmap (Completed Milestones)

### Phase 1: Workspace & Guardrails Implementation
*   [x] **AWU 1.1: Workspace Cargo setup**
*   [x] **AWU 1.2: Path safety & Smart Output Truncation Guardrails**
*   [x] **AWU 1.3: Safe Delete and Read File with Limit Prototype**

### Phase 2: Full Tool Suite & MCP Server
*   [x] **AWU 2.1: Implement MCP wrapper and tool definitions**
*   [x] **AWU 2.2: Implement remaining core uutils/coreutils tools**
*   [x] **AWU 2.3: Execution Sandbox** (~~Rollback Manager~~ — see correction below)
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

### Phase 3: Server Maturity (Completed)
*   [x] **AWU 5.1: Integrate Structured Logging** (tracing)
*   [x] **AWU 5.2: Refine JSON-RPC Loop** (graceful shutdown)
*   [x] **AWU 5.3: Add Integration Testing** (end-to-end JSON-RPC)

### Phase 4: Quality Assurance & Automation (Completed)
*   [x] **AWU 6.1: Expand Test Coverage** (unit, integration, property)
*   [x] **AWU 6.2: CI/CD Pipeline Setup** (GitHub Actions)

### Phase 5: Design Review Follow-ups (Completed)
*   [x] **AWU 7.1: Optional Workspace-Root Confinement** (`AI_WORKSPACE_ROOT`)
*   [x] **AWU 7.2: Correct "Sandbox" Terminology to Match Reality**

---

## 📝 Execution Log

| AWU ID | Task Name | Status | Result / Notes |
| :--- | :--- | :--- | :--- |
| **1.1** | **Workspace Cargo setup** | `[✅] Completed` | Converted crate to a Cargo Workspace layout. |
| 1.2 | Path safety & Smart Output Truncation | `[✅] Completed` | Implemented validate_path_safety and truncate_output. |
| 1.3 | Safe Delete and Read File with Limit | `[✅] Completed` | Implemented delete_file_or_directory and read_file_with_limit. |
| 2.1 | MCP Server integration | `[✅] Completed` | Integrated all tools into JSON-RPC schema. |
| 2.2 | Full Tool Suite | `[✅] Completed` | Implemented all 14 core tools. |
| 2.3 | Rollback / Sandbox Integration | `[⚠️] Corrected` | Original entry claimed a "Rollback Manager" was completed; no such code (backup-before-delete, undo log) has ever existed in the repo — only the sandbox runner with stdout guard (`execute_command_in_sandbox`) was actually built. Surfaced during the 2026-07-27 design review; the user decided a rollback manager is not needed, so this will not be implemented. Entry corrected rather than deleted, per this project's own history-integrity rule. |
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
| **5.1** | **Integrate Structured Logging** | `[✅] Completed` | Added `tracing`/`tracing-subscriber` to `core-utilities-mcp`. Logs (request method, tool name, `error_type` on failure, startup/shutdown) go to stderr only, gated by `RUST_LOG` (default `info`); stdout remains pure JSON-RPC. Verified via manual stdout/stderr separation test. README documents `RUST_LOG`. |
| **5.2** | **Refine JSON-RPC Loop** | `[✅] Completed` | Switched stdin reading to async (`tokio::io::stdin` + `AsyncBufReadExt`) so a `Ctrl+C`/`SIGTERM` (`shutdown_signal()`) can interrupt a blocked read via `tokio::select!`. Discovered and fixed a real hang: since `tokio::io::stdin()` reads via a blocking OS thread, returning normally from `main` after a signal made the implicit `Runtime` Drop wait forever for that thread (reproduced with a FIFO held open, simulating a live MCP client connection) — fixed via `std::process::exit(0)` on the signal path. Verified: SIGTERM now exits in ~100ms even mid-read; stdin-EOF and normal request/response flows unaffected (21 unit + 17 doctests still pass). |
| **5.3** | **Add Integration Testing** | `[✅] Completed` | Added `core-utilities-mcp/tests/integration_test.rs` (157 lines): spawns the real compiled binary and drives it over actual stdin/stdout pipes. 7 tests cover `initialize`/`tools/list` (asserts all 15 tools), a successful `tools/call`, a failing `tools/call` asserting `error_type`, malformed-JSON parse errors, unknown-method errors, notification-produces-no-output-line, and graceful exit on stdin close. All pass; `cargo fmt`/`clippy` clean. |
| **6.1** | **Expand Test Coverage** | `[✅] Completed` | Added `proptest` dev-dependency and `core-utilities-mcp-lib/tests/property_tests.rs` (6 property tests: `truncate_output` bounds/prefix/next_offset invariants, `validate_path_safety` NUL-byte/whitespace rejection). Added 6 new unit tests for previously-untested error paths (`file_ops` nonexistent-target errors, `search_ops` invalid-regex/nonexistent-root errors, `text_ops` missing-column/missing-JSON-path/invalid-JSON errors). While touching every test module, also fixed a pre-existing `CODING.md` violation ("Strict Test Separation"): extracted all inline `#[cfg(test)] mod tests { ... }` blocks in `guardrails`, `file_ops`, `search_ops`, `text_ops`, `sys_ops` into companion `tests.rs` files (mechanical move, no logic changes), which also brought `file_ops/mod.rs` and `text_ops/mod.rs` back toward the 300-line production-file limit (still 374/270 respectively — `file_ops/mod.rs` remains over the limit due to AWU 4.3's doc comments; splitting production logic further was judged out of scope for this AWU). Total suite: 27 unit + 6 property + 7 integration + 17 doctests, all passing; `cargo fmt`/`clippy` clean. |

| **6.2** | **CI/CD Pipeline Setup** | `[✅] Completed` | Added `.github/workflows/ci.yml` (checkout → stable toolchain w/ rustfmt+clippy → cargo cache → `fmt --check` → `clippy --workspace --all-targets -- -D warnings` → `build` → `test`), triggered on push/PR to `main`. This exposed the one pre-existing clippy warning (`await_holding_lock` in a `sys_ops` test holding `ENV_MUTEX` across an `.await`) as a real CI blocker: fixed by switching `ENV_MUTEX` from `std::sync::Mutex` to `tokio::sync::Mutex` (sync tests now use `.blocking_lock()`, the async test uses `.lock().await`), which is the correct fix rather than suppressing the lint. `cargo clippy --workspace --all-targets -- -D warnings` now passes cleanly. README documents the CI pipeline. |
| **7.1** | **Optional Workspace-Root Confinement** | `[✅] Completed` | Design review raised that this MCP is meant to run scoped to `~/projects/rad`, and that a real process-isolation "sandbox" is disproportionate for single-user local use — the higher-value, cheaper investment is confining file operations to an intended directory. Added `AI_WORKSPACE_ROOT` (opt-in, off by default) to `guardrails::validate_path_safety`: when set, every path-validated call is lexically resolved (handles `..`/`.` and not-yet-existing targets without touching disk) and rejected if it falls outside the canonicalized root. Wired in centrally, so `file_ops`, `search_ops`, and `text_ops` all get it for free. Added 3 new unit tests, plus `ENV_MUTEX` locking to ~14 previously-unlocked tests across `guardrails`/`file_ops`/`search_ops`/`text_ops` to prevent the new global env var from racing unrelated concurrent tests (verified stable across 5 repeated `cargo test` runs). Explicitly does **not** cover `execute_command_in_sandbox`, which remains entirely unconfined — a conscious tradeoff, not an oversight (see 7.2). Manually verified end-to-end via the compiled binary. |
| **7.2** | **Correct "Sandbox" Terminology to Match Reality** | `[✅] Completed` | The design review found the "sandbox" framing overstated what `execute_command_in_sandbox` actually provides (timeout + stdout-size guard only — no process isolation), and that `extract_code_skeleton`'s README description falsely claimed tree-sitter parsing (it's regex-only). Reworded the MCP `tools/list` description and README entries for both to state plainly what they do and don't do, without renaming the tool (avoids breaking existing client configs). README's path-safety section now also documents `AI_WORKSPACE_ROOT` and states explicitly that guardrails are a mistake-prevention guard, not an adversarial security boundary, and that `execute_command_in_sandbox` bypasses them entirely. |

---

## 🚀 Next Steps
All planned roadmap phases (1–5) are now complete. Remaining open item (not yet an AWU):
1. `core-utilities-mcp-lib/src/file_ops/mod.rs` (374 lines) still exceeds the 300-line CODING.md limit purely from doc comments + logic; consider splitting into smaller per-operation files if this becomes a recurring audit finding.
