//! Mechanical safety boundaries shared by every operation in this crate:
//! path safety validation (rejecting catastrophic mutation targets and,
//! optionally, anything outside a configured workspace) and output
//! truncation (keeping tool responses within an AI-friendly size).

use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruncateResult {
    pub content: String,
    pub status: String, // "success" or "truncated"
    pub next_offset: Option<usize>,
}

/// Top-level system directories that must never be targeted directly by a
/// mutation (delete, move, copy destination, mkdir, or edit). Matching is
/// exact (case-insensitive, ignoring trailing separators) so legitimate work
/// on subpaths such as `/etc/myapp.conf` remains unaffected.
const CRITICAL_SYSTEM_DIRS: &[&str] = &[
    "/etc",
    "/bin",
    "/sbin",
    "/usr",
    "/boot",
    "/dev",
    "/proc",
    "/sys",
    "/root",
    "/system",
    "/library",
    "c:\\windows",
    "c:\\program files",
    "c:\\program files (x86)",
];

fn is_critical_system_dir(trimmed: &str) -> bool {
    let normalized = trimmed.trim_end_matches(['/', '\\']).to_lowercase();
    CRITICAL_SYSTEM_DIRS.contains(&normalized.as_str())
}

/// Collapses `.` and `..` components of an absolute path lexically (no
/// filesystem access), so a target that does not exist yet — a new file, a
/// move/copy destination, a not-yet-created `mkdir -p` chain — can still be
/// checked against a workspace root.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut stack: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => stack.push(component),
            },
            other => stack.push(other),
        }
    }
    stack.iter().collect()
}

/// Resolves `path` to an absolute, lexically-normalized form relative to the
/// current working directory, without requiring it to exist.
fn resolve_absolute(path: &str) -> CoreResult<PathBuf> {
    let candidate = Path::new(path);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| CoreError::Guardrail(format!("failed to resolve current directory: {e}")))?
            .join(candidate)
    };
    Ok(lexically_normalize(&absolute))
}

/// When `AI_WORKSPACE_ROOT` is set, rejects any path that resolves outside
/// it. Confinement is opt-in and off by default: with the variable unset,
/// this is a no-op, preserving prior behavior for callers that don't set it.
fn check_workspace_root(path: &str) -> CoreResult<()> {
    let Ok(root_str) = std::env::var("AI_WORKSPACE_ROOT") else {
        return Ok(());
    };

    let root = std::fs::canonicalize(&root_str).map_err(|e| {
        CoreError::Guardrail(format!(
            "AI_WORKSPACE_ROOT '{root_str}' is not a valid directory: {e}"
        ))
    })?;

    let resolved = resolve_absolute(path)?;
    if !resolved.starts_with(&root) {
        return Err(CoreError::Guardrail(format!(
            "path '{path}' is outside the configured workspace root '{}'",
            root.display()
        )));
    }

    Ok(())
}

/// Validates a target path before it is used in a filesystem mutation
/// (delete, move, copy destination, mkdir, or edit).
///
/// Rejects:
/// - empty or whitespace-only paths, and paths containing a NUL byte
/// - exact matches for `.`, `/`, `*`, `~`
/// - wildcard-terminated patterns (`/*`, `/.*`, and the Windows-style `\*`, `\.*`)
/// - exact matches (case-insensitive, ignoring trailing separators) against a
///   fixed deny-list of critical system directories (e.g. `/etc`, `/usr`,
///   `C:\Windows`); subpaths beneath these directories are still permitted
/// - anything outside `AI_WORKSPACE_ROOT`, if that environment variable is
///   set (confinement is opt-in; unset means no workspace restriction)
///
/// This is a mistake-prevention guard against an AI agent going off-script,
/// not an adversarial security boundary: it does not resolve symlinks in the
/// path being checked, and provides no protection against a command that
/// bypasses this function entirely (e.g. a raw shell command).
///
/// # Examples
///
/// ```
/// use core_utilities_mcp_lib::guardrails::validate_path_safety;
///
/// assert!(validate_path_safety("src/lib.rs").is_ok());
/// assert!(validate_path_safety("/").is_err());
/// assert!(validate_path_safety("/etc").is_err());
/// assert!(validate_path_safety("/etc/hosts").is_ok());
/// ```
///
/// ```no_run
/// use core_utilities_mcp_lib::guardrails::validate_path_safety;
///
/// std::env::set_var("AI_WORKSPACE_ROOT", "/Users/me/projects/rad");
/// assert!(validate_path_safety("/Users/me/projects/rad/notes.md").is_ok());
/// assert!(validate_path_safety("/Users/me/.ssh/id_ed25519").is_err());
/// ```
pub fn validate_path_safety(path: &str) -> CoreResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Guardrail("path is empty".to_string()));
    }

    if trimmed.contains('\0') {
        return Err(CoreError::Guardrail("path contains a NUL byte".to_string()));
    }

    // Complete matches
    if trimmed == "." || trimmed == "/" || trimmed == "*" || trimmed == "~" {
        return Err(CoreError::Guardrail(format!(
            "dangerous path sequence '{}'",
            trimmed
        )));
    }

    // Path pattern check: ending in /* or /.* (and also windows backslash style)
    if trimmed.ends_with("/*")
        || trimmed.ends_with("/.*")
        || trimmed.ends_with("\\*")
        || trimmed.ends_with("\\.*")
    {
        return Err(CoreError::Guardrail(format!(
            "wildcard target pattern '{}'",
            trimmed
        )));
    }

    if is_critical_system_dir(trimmed) {
        return Err(CoreError::Guardrail(format!(
            "operation targets a critical system directory '{}'",
            trimmed
        )));
    }

    check_workspace_root(trimmed)?;

    Ok(())
}

/// Truncates `input` to at most `AI_COMMAND_MAX_CHARACTERS` characters
/// (default `8192`), preferring to cut at the last newline within the limit
/// so structured output (JSON lines, log lines) is not split mid-line. When
/// truncation occurs, `next_offset` reports the character offset callers
/// should resume reading from.
///
/// # Examples
///
/// ```
/// use core_utilities_mcp_lib::guardrails::truncate_output;
///
/// std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "5");
/// let result = truncate_output("hello world");
/// assert_eq!(result.status, "truncated");
/// assert_eq!(result.content, "hello");
/// assert_eq!(result.next_offset, Some(5));
/// ```
pub fn truncate_output(input: &str) -> TruncateResult {
    let limit: usize = std::env::var("AI_COMMAND_MAX_CHARACTERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192);

    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= limit {
        return TruncateResult {
            content: input.to_string(),
            status: "success".to_string(),
            next_offset: None,
        };
    }

    // Attempt to locate standard smart boundary (newline \n) before limit
    let limit_chunk = &chars[..limit];
    if let Some(last_newline_idx) = limit_chunk.iter().rposition(|&c| c == '\n') {
        // Cut right after the newline (include it in the output, or cut exactly before? Let's cut after it)
        let cut_idx = last_newline_idx + 1;
        let content: String = chars[..cut_idx].iter().collect();
        return TruncateResult {
            content,
            status: "truncated".to_string(),
            next_offset: Some(cut_idx),
        };
    }

    // Fallback: hard truncate
    let content: String = chars[..limit].iter().collect();
    TruncateResult {
        content,
        status: "truncated".to_string(),
        next_offset: Some(limit),
    }
}

/// Guards mutations of the process-global `AI_COMMAND_MAX_CHARACTERS`
/// environment variable across tests that run concurrently within the same
/// test binary. A `tokio::sync::Mutex` (rather than `std::sync::Mutex`) is
/// used deliberately: `sys_ops`'s async tests must hold the guard across an
/// `.await`, which clippy's `await_holding_lock` lint (rightly) flags for
/// std mutexes but not for tokio's async-aware one.
#[cfg(test)]
pub static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests;
