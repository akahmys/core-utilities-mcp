//! Line-windowed, line-numbered file reading.

use crate::errors::{CoreError, CoreResult};
use crate::guardrails::{ensure_existing_read_path, truncate_output};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Safety margin (chars) beyond `AI_COMMAND_MAX_CHARACTERS` that [`read_file`]
/// accumulates before giving up on the current window and stopping — bounds
/// its working buffer to roughly the output size regardless of how much of
/// the file remains, rather than annotating every remaining line up front.
const READ_WINDOW_MARGIN: usize = 1024;

/// The result of [`read_file`]: line-numbered content, whether it was
/// truncated, and where to resume from if so.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadFileResult {
    /// Each included line as `"{line_number}\t{line_content}\n"`.
    pub content: String,
    /// `"success"` or `"truncated"`.
    pub status: String,
    /// Present when truncated: the 1-indexed line to pass as `start_line` on
    /// the next call to continue reading where this one left off.
    pub next_start_line: Option<usize>,
}

/// Reads `path` as UTF-8 text starting at 1-indexed line `start_line`
/// (default `1`), annotating each line with its number so the result can be
/// used directly as `start_line`/`end_line`/`target_content` input to
/// [`crate::file_ops::edit_file`]. Applies the standard
/// [`truncate_output`](crate::guardrails::truncate_output) size limit,
/// stopping early (within [`READ_WINDOW_MARGIN`] of it) rather than
/// annotating the file's entire remainder up front.
///
/// # Errors
/// Returns [`CoreError::Guardrail`] if `path` fails safety validation, or
/// [`CoreError::File`] if it does not exist or is not valid UTF-8.
///
/// # Examples
///
/// ```no_run
/// use core_utilities_mcp_lib::text_ops::read_file;
///
/// let page = read_file("README.md", None)?;
/// println!("{}", page.content);
/// # Ok::<(), core_utilities_mcp_lib::CoreError>(())
/// ```
pub fn read_file(path: &str, start_line: Option<usize>) -> CoreResult<ReadFileResult> {
    let path_buf = ensure_existing_read_path(path)?;

    let raw = std::fs::read_to_string(&path_buf)
        .map_err(|e| CoreError::File(format!("Failed to read file: {e}")))?;
    let normalized = raw.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    let start = start_line.unwrap_or(1).max(1);
    let limit: usize = std::env::var("AI_COMMAND_MAX_CHARACTERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192);

    let mut annotated = String::new();
    for (i, line) in lines.iter().enumerate().skip(start.saturating_sub(1)) {
        if annotated.chars().count() > limit + READ_WINDOW_MARGIN {
            break;
        }
        // `fmt::Write` for `String` never actually fails; ignore the `Result`
        // rather than `.unwrap()`/`.expect()`, which `CODING.md` bans outright.
        let _ = writeln!(annotated, "{}\t{line}", i + 1);
    }

    let truncated = truncate_output(&annotated);
    let next_start_line = truncated.next_offset.map(|_| {
        let lines_included = truncated.content.matches('\n').count();
        start + lines_included.max(1)
    });

    Ok(ReadFileResult {
        content: truncated.content,
        status: truncated.status,
        next_start_line,
    })
}

#[cfg(test)]
mod tests;
