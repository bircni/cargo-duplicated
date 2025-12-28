//! Token-based code normalization for structural duplicate detection.
//!
//! This module provides functionality to normalize Rust code at the token level,
//! replacing identifiers and literals with placeholders to detect structurally
//! identical code with different variable names or values.

use regex::Regex;
use std::sync::LazyLock;

/// Replace numeric literals with <NUM>
#[expect(
    clippy::unwrap_used,
    reason = "Regex compilation is infallible with valid patterns"
)]
static NUM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match numeric literals with optional suffixes like u32, f64, usize, i8, etc.
    // Supports: integers, floats, scientific notation, and all Rust numeric type suffixes
    Regex::new(r"\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?(?:_?[iurf](?:8|16|32|64|128|size))?\b").unwrap()
});
/// Replace identifiers with <ID> (but preserve keywords and already-replaced tokens)
static KEYWORDS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    vec![
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ]
});

#[expect(
    clippy::unwrap_used,
    reason = "Regex compilation is infallible with valid patterns"
)]
static ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b").unwrap());

/// Normalize a line of Rust code by replacing identifiers and literals with placeholders.
///
/// This allows detection of structurally similar code that differs only in naming or literal values.
/// For example, `let x = 42;` and `let y = 99;` would both normalize to `let <ID> = <NUM>;`.
///
/// **Note**: This function operates on a single line and handles single-line block comments
/// (e.g., `/* comment */` within a line). Multi-line block comments are handled at the
/// file level by the caller (e.g., `read_token_normalized_lines` in scanner.rs) before
/// lines are passed to this function.
pub fn tokenize_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Skip empty lines and line comments
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }

    // Skip attributes and derives as they're often boilerplate
    if trimmed.starts_with("#[") {
        return None;
    }

    let mut normalized = String::new();
    let mut in_string = false;
    let mut in_block_comment = false;
    let mut chars = trimmed.chars().peekable();

    while let Some(c) = chars.next() {
        // Handle single-line block comments (/* ... */ within this line)
        // Multi-line comments are stripped by the caller before reaching this function
        if !in_string && c == '/' && chars.peek() == Some(&'*') {
            in_block_comment = true;
            chars.next(); // consume '*'
            continue;
        }

        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                in_block_comment = false;
                chars.next(); // consume '/'
            }
            continue;
        }

        match c {
            '"' => {
                in_string = !in_string;
                normalized.push_str("<STR>");
                // Skip until closing quote
                if in_string {
                    let mut prev_was_backslash = false;
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '"' && !prev_was_backslash {
                            in_string = false;
                            break;
                        }
                        // Track backslashes for escape sequences
                        prev_was_backslash = ch == '\\' && !prev_was_backslash;
                    }
                }
            }
            '\'' if chars.peek() != Some(&'\'') => {
                // Distinguish between lifetimes and character literals
                // Lifetimes: 'identifier (no closing quote immediately after)
                // Character literals: 'x' (closing quote follows)

                if chars.peek().is_some() {
                    // Check if this is a character literal by looking ahead for closing quote
                    let mut temp_chars = chars.clone();
                    temp_chars.next(); // consume the character
                    let has_closing_quote = temp_chars.peek() == Some(&'\'');

                    if has_closing_quote {
                        // This is a character literal 'x'
                        normalized.push_str("<CHAR>");
                        chars.next(); // consume character
                        chars.next(); // consume closing quote
                    } else {
                        // This is a lifetime 'a or 'static
                        normalized.push('\'');
                    }
                } else {
                    // End of string, just keep the apostrophe
                    normalized.push('\'');
                }
            }
            _ if !in_string => {
                normalized.push(c);
            }
            _ => {}
        }
    }

    normalized = NUM_REGEX.replace_all(&normalized, "<NUM>").to_string();

    normalized = ID_REGEX
        .replace_all(&normalized, |caps: &regex::Captures<'_>| {
            let word = &caps[0];
            // Don't replace if it's part of a token placeholder (between < and >)
            let (before_match, after_match) = caps.get(0).map_or(("", ""), |m| {
                let match_start = m.start();
                let after_match_start = m.end();
                (&normalized[..match_start], &normalized[after_match_start..])
            });

            // Only preserve actual token placeholders (ID, NUM, STR, CHAR), not generic type parameters
            if before_match.ends_with('<') && after_match.starts_with('>') {
                // Check if this is one of our placeholder tokens
                if matches!(word, "ID" | "NUM" | "STR" | "CHAR") {
                    return word.to_owned();
                }
                // Otherwise, it's a generic type parameter and should be normalized
            }

            if KEYWORDS.contains(&word) {
                word.to_owned()
            } else {
                String::from("<ID>")
            }
        })
        .to_string();

    // Normalize whitespace
    let mut final_norm = String::new();
    for (idx, part) in normalized.split_whitespace().enumerate() {
        if idx > 0 {
            final_norm.push(' ');
        }
        final_norm.push_str(part);
    }

    if final_norm.is_empty() {
        None
    } else {
        Some(final_norm)
    }
}
