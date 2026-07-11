# Project Work Plan (PLANS.md)

**Last Updated**: 2026-07-11

## 🗺️ Long-Term Plan (Roadmap)

* [ ] Phase 1: Core Foundation & Minimal MCP SDK Integration
* [ ] Phase 2: Complete Coreutils Mapping (`structured_grep` & `verify_file`)
* [ ] Phase 3: Path Canonicalization & Security Guardrails Implementation

---

## 🛠️ Short-Term Plan: Phase 1 (Core Foundation & Minimal MCP SDK Integration)

### 💡 Current AWU Status

* [ ] AWU-1: [In Progress] Initialize Cargo project and import `uucore`, `uu_ls`, and `uu_cat` from git repository.
* [ ] AWU-2: [Todo] Implement `inspect_dir` tool by wrapping the `uu_ls` library logic.
* [ ] AWU-3: [Todo] Implement `peek_file` tool by wrapping the `uu_cat` library logic with line-windowing filters.

### 📝 AWU Details

#### AWU-1: Initialize Cargo project and import `uucore`, `uu_ls`, and `uu_cat` from git repository.

* **Objective**: Create a valid Rust binary project that links successfully with the remote `uutils` Git crates and an empty MCP standard I/O loop.
* **Scope**: `Cargo.toml`, `src/main.rs`
* **DoD**: `cargo check` and `cargo clippy --all-targets` pass with zero warnings under strict `#![deny(clippy::pedantic)]`.
* **Result**:

#### AWU-2: Implement `inspect_dir` tool by wrapping the `uu_ls` library logic.

* **Objective**: Call `uu_ls` core structures internally to fetch directory entries and output a filtered JSON array (name, type, size) that excludes `.git` and `target`.
* **Scope**: `src/tools/inspect_dir.rs`, `src/main.rs`
* **DoD**: Unit tests in `src/tools/inspect_dir/tests.rs` pass successfully. Output matches the specific JSON Schema: `Array<{name: string, type: string, size_bytes: number}>`.
* **Result**:

#### AWU-3: Implement `peek_file` tool by wrapping the `uu_cat` library logic with line-windowing filters.

* **Objective**: Expose an MCP tool that leverages `uu_cat` or its lower-level stream buffers to read only the requested line range, preserving token efficiency.
* **Scope**: `src/tools/peek_file.rs`, `src/main.rs`
* **DoD**: `cargo test` passes. Negative test cases (e.g., file not found, permission denied) are cleanly converted from `uucore` errors into `thiserror` domain errors.
* **Result**:
