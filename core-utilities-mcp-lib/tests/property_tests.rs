//! Property-based tests exercising the invariants of `guardrails`, across
//! randomly generated inputs rather than fixed examples.

use core_utilities_mcp_lib::guardrails::{truncate_output, validate_path_safety};
use proptest::prelude::*;

/// `truncate_output` falls back to this limit whenever
/// `AI_COMMAND_MAX_CHARACTERS` is unset; removing the var up front keeps
/// these properties independent of the ambient environment.
fn default_limit() -> usize {
    std::env::remove_var("AI_COMMAND_MAX_CHARACTERS");
    8192
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn content_never_exceeds_the_default_limit(input in "[a-zA-Z0-9 \n]{0,20000}") {
        let limit = default_limit();
        let result = truncate_output(&input);
        prop_assert!(result.content.chars().count() <= limit);
    }

    #[test]
    fn content_is_always_a_prefix_of_the_input(input in "[a-zA-Z0-9 \n]{0,20000}") {
        default_limit();
        let result = truncate_output(&input);
        prop_assert!(input.starts_with(&result.content));
    }

    #[test]
    fn short_input_is_returned_unchanged(input in "[a-zA-Z0-9 \n]{0,8192}") {
        default_limit();
        let result = truncate_output(&input);
        prop_assert_eq!(result.status, "success");
        prop_assert_eq!(result.content, input);
        prop_assert_eq!(result.next_offset, None);
    }

    #[test]
    fn truncated_next_offset_matches_content_length(input in "[a-zA-Z0-9 \n]{8193,20000}") {
        default_limit();
        let result = truncate_output(&input);
        prop_assert_eq!(result.status, "truncated");
        prop_assert_eq!(result.next_offset, Some(result.content.chars().count()));
    }

    #[test]
    fn nul_byte_paths_are_always_rejected(
        prefix in "[a-zA-Z0-9/_.]{0,50}",
        suffix in "[a-zA-Z0-9/_.]{0,50}",
    ) {
        let path = format!("{}\0{}", prefix, suffix);
        prop_assert!(validate_path_safety(&path).is_err());
    }

    #[test]
    fn whitespace_only_paths_are_always_rejected(input in "[ \t\n]{0,20}") {
        prop_assert!(validate_path_safety(&input).is_err());
    }
}
