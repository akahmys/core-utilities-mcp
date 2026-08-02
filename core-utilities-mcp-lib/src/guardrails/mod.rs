//! Mechanical safety boundaries shared by every operation in this crate:
//! path safety validation (rejecting catastrophic mutation targets and,
//! optionally, anything outside a configured workspace) and output
//! truncation (keeping tool responses within an AI-friendly size).

mod path_safety;
mod truncation;

pub use path_safety::{ensure_existing_read_path, validate_path_safety, validate_read_path_safety};
pub use truncation::{truncate_output, TruncateResult};

/// Serializes tests that mutate process-global env vars. `tokio::sync::Mutex`
/// rather than `std::sync::Mutex`, since `sys_ops`'s async tests hold the
/// guard across an `.await` (flagged by clippy's `await_holding_lock` for
/// std mutexes, not tokio's async-aware one).
#[doc(hidden)]
pub static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
