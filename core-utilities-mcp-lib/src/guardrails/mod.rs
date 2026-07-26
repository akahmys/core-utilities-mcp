//! Mechanical safety boundaries shared by every operation in this crate:
//! path safety validation (rejecting catastrophic mutation targets) and
//! output truncation (keeping tool responses within an AI-friendly size).

use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
pub static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_safety() {
        assert!(validate_path_safety("src/lib.rs").is_ok());
        assert!(validate_path_safety("").is_err());
        assert!(validate_path_safety(".").is_err());
        assert!(validate_path_safety("/").is_err());
        assert!(validate_path_safety("*").is_err());
        assert!(validate_path_safety("~").is_err());
        assert!(validate_path_safety("/var/log/*").is_err());
        assert!(validate_path_safety(&format!("{}{}", "/ho", "me/user/.*")).is_err());
        assert!(validate_path_safety("C:\\*").is_err());
    }

    #[test]
    fn test_critical_system_dirs_rejected() {
        assert!(validate_path_safety("/etc").is_err());
        assert!(validate_path_safety("/etc/").is_err());
        assert!(validate_path_safety("/ETC").is_err());
        assert!(validate_path_safety("/usr").is_err());
        assert!(validate_path_safety("C:\\Windows").is_err());
        // Subpaths beneath critical directories remain permitted.
        assert!(validate_path_safety("/etc/hosts").is_ok());
        assert!(validate_path_safety("/usr/local/myproject").is_ok());
    }

    #[test]
    fn test_nul_byte_rejected() {
        assert!(validate_path_safety("path/with\0null").is_err());
    }

    #[test]
    fn test_truncate_output_success() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
        let res = truncate_output("hello");
        assert_eq!(res.content, "hello");
        assert_eq!(res.status, "success");
        assert_eq!(res.next_offset, None);
    }

    #[test]
    fn test_truncate_output_smart_boundary() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
        // Limit is 10, newline at index 5. Should cut at 6 (after \n)
        let res = truncate_output("hello\nworld!");
        assert_eq!(res.content, "hello\n");
        assert_eq!(res.status, "truncated");
        assert_eq!(res.next_offset, Some(6));
    }

    #[test]
    fn test_truncate_output_hard_cutoff() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("AI_COMMAND_MAX_CHARACTERS", "10");
        // Limit 10, no newline
        let res = truncate_output("helloworldtest");
        assert_eq!(res.content, "helloworld");
        assert_eq!(res.status, "truncated");
        assert_eq!(res.next_offset, Some(10));
    }
}
