//! Path safety validation: rejecting catastrophic mutation targets and,
//! optionally, anything outside a configured workspace.

use crate::errors::{CoreError, CoreResult};
use std::path::{Component, Path, PathBuf};

/// Top-level system directories that must never be targeted directly by a
/// mutation. Matching is exact (case-insensitive, ignoring trailing
/// separators) so legitimate work on subpaths like `/etc/myapp.conf` is fine.
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

/// Collapses `.` and `..` path components lexically (no filesystem access),
/// so a not-yet-existing target (new file, move/copy destination, `mkdir -p`
/// chain) can still be checked against a workspace root.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut stack: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => stack.push(component),
            },
            other => stack.push(other),
        }
    }
    stack.iter().collect()
}

/// Resolves `path` to an absolute, lexically-normalized form relative to the
/// cwd, without requiring it to exist.
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

/// When `AI_WORKSPACE_ROOT` is set, rejects any path resolving outside it.
/// Opt-in and off by default: unset, this is a no-op.
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
            "path '{path}' is outside the configured workspace root '{}' — pass a path inside that directory instead",
            root.display()
        )));
    }

    Ok(())
}

/// True if `trimmed` lexically collapses to "here" under any spelling
/// (`.`, `./`, `././`, `a/..`, ...) — a literal `== "."` comparison misses
/// these equivalent spellings.
fn is_current_dir_equivalent(trimmed: &str) -> bool {
    lexically_normalize(Path::new(trimmed))
        .as_os_str()
        .is_empty()
}

/// Checks shared by both [`validate_path_safety`] and
/// [`validate_read_path_safety`]: NUL bytes, `/`/`*`/`~`, wildcard
/// patterns, critical system directories, and `AI_WORKSPACE_ROOT`
/// confinement. Each rejection states what to pass instead, for the LLM
/// caller deciding how to retry.
fn common_path_checks(trimmed: &str) -> CoreResult<()> {
    if trimmed.contains('\0') {
        return Err(CoreError::Guardrail("path contains a NUL byte".to_string()));
    }

    // Complete matches
    if trimmed == "/" || trimmed == "*" || trimmed == "~" {
        return Err(CoreError::Guardrail(format!(
            "dangerous path sequence '{trimmed}' — this would target an entire filesystem root or every entry in a directory; pass a specific file or subdirectory instead"
        )));
    }

    // Path pattern check: ending in /* or /.* (and also windows backslash style)
    if trimmed.ends_with("/*")
        || trimmed.ends_with("/.*")
        || trimmed.ends_with("\\*")
        || trimmed.ends_with("\\.*")
    {
        return Err(CoreError::Guardrail(format!(
            "wildcard target pattern '{trimmed}' — pass an explicit file or directory path instead of a shell glob"
        )));
    }

    if is_critical_system_dir(trimmed) {
        return Err(CoreError::Guardrail(format!(
            "operation targets a critical system directory '{trimmed}' — operate on a specific path beneath it instead, e.g. '{trimmed}/some-file'"
        )));
    }

    check_workspace_root(trimmed)
}

/// Validates a target path before it is used in a filesystem mutation
/// (delete, move, copy destination, mkdir, or edit).
///
/// Rejects:
/// - empty or whitespace-only paths, and paths containing a NUL byte
/// - the current directory, in any spelling that lexically collapses to it
///   (`.`, `./`, `././`, `a/..`, ...), and exact matches for `/`, `*`, `~`
/// - wildcard-terminated patterns (`/*`, `/.*`, and the Windows-style `\*`, `\.*`)
/// - exact matches (case-insensitive, ignoring trailing separators) against a
///   fixed deny-list of critical system directories (e.g. `/etc`, `/usr`,
///   `C:\Windows`); subpaths beneath these directories are still permitted
/// - anything outside `AI_WORKSPACE_ROOT`, if that environment variable is
///   set (confinement is opt-in; unset means no workspace restriction)
///
/// Read-only operations (list, stat, read, search) should use
/// [`validate_read_path_safety`] instead, which permits the current
/// directory since reading it cannot destroy anything.
///
/// This is a mistake-prevention guard against an AI agent going off-script,
/// not an adversarial security boundary: it does not resolve symlinks in the
/// path being checked, and provides no protection against a command that
/// bypasses this function entirely (e.g. a raw shell command).
///
/// # Errors
/// Returns [`CoreError::Guardrail`] for any of the rejection conditions
/// listed above.
///
/// # Examples
///
/// ```
/// use core_utilities_mcp_lib::guardrails::validate_path_safety;
///
/// assert!(validate_path_safety("src/lib.rs").is_ok());
/// assert!(validate_path_safety(".").is_err());
/// assert!(validate_path_safety("./").is_err());
/// assert!(validate_path_safety("/").is_err());
/// assert!(validate_path_safety("/etc").is_err());
/// assert!(validate_path_safety("/etc/hosts").is_ok());
/// ```
///
/// ```no_run
/// use core_utilities_mcp_lib::guardrails::validate_path_safety;
///
/// std::env::set_var("AI_WORKSPACE_ROOT", "/srv/rad");
/// assert!(validate_path_safety("/srv/rad/notes.md").is_ok());
/// assert!(validate_path_safety("/srv/other-project/id_ed25519").is_err());
/// ```
pub fn validate_path_safety(path: &str) -> CoreResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Guardrail("path is empty".to_string()));
    }

    if is_current_dir_equivalent(trimmed) {
        return Err(CoreError::Guardrail(format!(
            "dangerous path sequence '{trimmed}' — this would target the entire current directory; pass a specific file or subdirectory instead"
        )));
    }

    common_path_checks(trimmed)
}

/// Validates a target path before it is used in a read-only operation
/// (list, stat, read, search). Applies the same checks as
/// [`validate_path_safety`] except that it permits the current directory
/// (`.`, `./`, ...): reading or listing "here" is the normal, safe default
/// for these operations, since — unlike a mutation — it cannot destroy
/// anything.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] under the same conditions as
/// [`validate_path_safety`], except that the current directory is permitted.
///
/// # Examples
///
/// ```
/// use core_utilities_mcp_lib::guardrails::validate_read_path_safety;
///
/// assert!(validate_read_path_safety(".").is_ok());
/// assert!(validate_read_path_safety("./src").is_ok());
/// assert!(validate_read_path_safety("/").is_err());
/// assert!(validate_read_path_safety("/etc/hosts").is_ok());
/// ```
pub fn validate_read_path_safety(path: &str) -> CoreResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Guardrail("path is empty".to_string()));
    }

    common_path_checks(trimmed)
}

/// Validates `path` using [`validate_read_path_safety`] and verifies existence.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] or [`CoreError::File`].
pub fn ensure_existing_read_path(path: &str) -> CoreResult<PathBuf> {
    validate_read_path_safety(path)?;
    let path_buf = Path::new(path);
    if !path_buf.exists() {
        return Err(CoreError::File(format!("Path does not exist: {path}")));
    }
    Ok(path_buf.to_path_buf())
}

#[cfg(test)]
mod tests;
