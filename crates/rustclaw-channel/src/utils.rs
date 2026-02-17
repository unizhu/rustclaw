//! Text formatting utilities
//!
//! This module provides utilities for formatting text content from various sources
//! (MCP tools, LLM responses, etc.) for proper display on messaging platforms like Telegram.

/// Format text for Telegram display
///
/// This function handles:
/// - Escaped newline characters (`\\n` -> actual newlines)
/// - Escaped tab characters (`\\t` -> actual tabs)
/// - Other common escape sequences
/// - Proper spacing for markdown-like headers
/// - Trimming excessive whitespace while preserving intentional spacing
pub fn format_for_telegram(text: &str) -> String {
    let mut result = text.to_string();

    // Handle escaped sequences from JSON/MCP responses
    result = result.replace("\\n", "\n");
    result = result.replace("\\t", "\t");
    result = result.replace("\\r", "\r");
    result = result.replace("\\\"", "\"");

    // Ensure markdown headers have proper spacing
    // Add newline before headers if they're not at the start of a line
    let mut formatted = String::with_capacity(result.len());
    let mut prev_char = '\n'; // Assume we start at a newline

    for line in result.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') && !prev_char.is_whitespace() && !formatted.is_empty() {
            // Add newline before header if needed
            formatted.push('\n');
        }
        if !formatted.is_empty() && !formatted.ends_with('\n') {
            formatted.push('\n');
        }
        formatted.push_str(line);
        prev_char = line.chars().last().unwrap_or('\n');
    }

    // Trim trailing whitespace
    let mut result = formatted.trim_end().to_string();

    // Limit excessive consecutive newlines to 2 max
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result
}

/// Format text for display, truncating if too long
///
/// This is useful for platforms with message length limits like Telegram (4096 chars)
pub fn format_for_telegram_truncated(text: &str, max_length: usize) -> String {
    let formatted = format_for_telegram(text);
    if formatted.len() <= max_length {
        formatted
    } else {
        // Try to truncate at a word boundary
        let truncated = &formatted[..max_length.saturating_sub(50)];
        if let Some(last_period) = truncated.rfind('.') {
            format!(
                "{}...\n\n[Message truncated - {} more characters]",
                &formatted[..last_period + 1],
                formatted.len() - last_period - 1
            )
        } else if let Some(last_space) = truncated.rfind(' ') {
            format!(
                "{}...\n\n[Message truncated - {} more characters]",
                &formatted[..last_space],
                formatted.len() - last_space
            )
        } else {
            format!(
                "{}...\n\n[Message truncated]",
                &formatted[..max_length.saturating_sub(50)]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_escaped_newlines() {
        let input = "Line 1\\nLine 2\\nLine 3";
        let expected = "Line 1\nLine 2\nLine 3";
        assert_eq!(format_for_telegram(input), expected);
    }

    #[test]
    fn test_format_escaped_tabs() {
        let input = "Col1\\tCol2\\tCol3";
        let expected = "Col1\tCol2\tCol3";
        assert_eq!(format_for_telegram(input), expected);
    }

    #[test]
    fn test_format_mixed_escapes() {
        let input = "Hello\\nWorld\\t!";
        let expected = "Hello\nWorld\t!";
        assert_eq!(format_for_telegram(input), expected);
    }

    #[test]
    fn test_format_preserves_actual_newlines() {
        let input = "Line 1\nLine 2";
        let expected = "Line 1\nLine 2";
        assert_eq!(format_for_telegram(input), expected);
    }

    #[test]
    fn test_format_mixed_actual_and_escaped() {
        let input = "Line 1\nLine 2\\nLine 3";
        let expected = "Line 1\nLine 2\nLine 3";
        assert_eq!(format_for_telegram(input), expected);
    }

    #[test]
    fn test_truncation() {
        let input = "A".repeat(5000);
        let result = format_for_telegram_truncated(&input, 4000);
        assert!(result.len() <= 4050); // Allow some buffer for truncation message
        assert!(result.contains("[Message truncated"));
    }
}
