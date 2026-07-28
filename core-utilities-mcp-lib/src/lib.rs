//! Core library backing `core-utilities-mcp`: an AI-optimized, pure-Rust
//! toolkit that wraps common file, search, text, and system operations
//! behind deterministic, JSON-structured, output-limited functions.
//!
//! Every fallible function returns [`CoreResult<T>`], an alias for
//! `Result<T, CoreError>`. [`CoreError::category()`] gives a stable string
//! (e.g. `"File"`, `"Guardrail"`, `"Parsing"`) that callers such as the MCP
//! server surface as `error_type` alongside the human-readable message.
//!
//! Mutating operations (delete, move, copy destination, mkdir, write, edit)
//! are routed through [`guardrails::validate_path_safety`] before touching
//! disk; see that module for the specific paths and patterns it rejects.

pub mod errors;
pub use errors::{CoreError, CoreResult};

pub mod file_ops;
pub mod guardrails;
pub mod search_ops;
pub mod sys_ops;
pub mod text_ops;
