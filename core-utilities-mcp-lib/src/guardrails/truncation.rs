//! Output truncation: keeping tool responses within an AI-friendly size.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruncateResult {
    pub content: String,
    pub status: String, // "success" or "truncated"
    pub next_offset: Option<usize>,
}

/// Truncates `input` to at most `AI_COMMAND_MAX_CHARACTERS` characters
/// (default `8192`), preferring to cut at the last newline so structured
/// output isn't split mid-line. `next_offset` reports where to resume from.
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
#[must_use]
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
mod tests;
