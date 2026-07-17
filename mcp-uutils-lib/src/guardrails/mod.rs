use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruncateResult {
    pub content: String,
    pub status: String, // "success" or "truncated"
    pub next_offset: Option<usize>,
}

/// Validates target paths for filesystem mutations.
/// Reject targets like `.`, `/`, `*`, `~`, `""`, or path patterns ending with `/*` or `/.*`.
pub fn validate_path_safety(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path safety violation: path is empty".to_string());
    }
    
    // Complete matches
    if trimmed == "." || trimmed == "/" || trimmed == "*" || trimmed == "~" {
        return Err(format!("Path safety violation: dangerous path sequence '{}'", trimmed));
    }

    // Path pattern check: ending in /* or /.* (and also windows backslash style)
    if trimmed.ends_with("/*") || trimmed.ends_with("/.*") || trimmed.ends_with("\\*") || trimmed.ends_with("\\.*") {
        return Err(format!("Path safety violation: wildcard target pattern '{}'", trimmed));
    }

    Ok(())
}

/// Dynamic character output limiter utilizing environment variables.
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
        assert!(validate_path_safety("/home/user/.*").is_err());
        assert!(validate_path_safety("C:\\*").is_err());
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
