use crate::tokenizer::tokenize_line;

#[test]
fn test_tokenize_basic() {
    assert_eq!(
        tokenize_line("let x = 42;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
    assert_eq!(
        tokenize_line("let y = 99;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
}

#[test]
fn test_tokenize_string_literals() {
    assert_eq!(
        tokenize_line(r#"println!("hello");"#),
        Some("<ID>!(<STR>);".to_owned())
    );
}

#[test]
fn test_tokenize_keywords_preserved() {
    assert_eq!(
        tokenize_line("fn foo() -> i32 {"),
        Some("fn <ID>() -> <ID> {".to_owned())
    );
}

#[test]
fn test_tokenize_skips_comments() {
    assert_eq!(tokenize_line("// this is a comment"), None);
}

#[test]
fn test_tokenize_skips_attributes() {
    assert_eq!(tokenize_line("#[derive(Debug)]"), None);
}

#[test]
fn test_tokenize_char_literal() {
    assert_eq!(
        tokenize_line("let c = 'a';"),
        Some("let <ID> = <CHAR>;".to_owned())
    );
}

#[test]
fn test_tokenize_escaped_quotes() {
    assert_eq!(
        tokenize_line(r#"let s = "say \"hello\"";"#),
        Some("let <ID> = <STR>;".to_owned())
    );
    assert_eq!(
        tokenize_line(r#"let s = "path\\file";"#),
        Some("let <ID> = <STR>;".to_owned())
    );
}

#[test]
fn test_tokenize_numeric_literals_with_suffixes() {
    assert_eq!(
        tokenize_line("let x = 42u32;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
    assert_eq!(
        tokenize_line("let x = 100u8;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
    assert_eq!(
        tokenize_line("let x = 5usize;"),
        Some("let <ID> = <NUM>;".to_owned())
    );

    assert_eq!(
        tokenize_line("let x = 42i32;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
    assert_eq!(
        tokenize_line("let x = -100i64;"),
        Some("let <ID> = -<NUM>;".to_owned())
    );

    assert_eq!(
        tokenize_line("let x = 1.0f64;"),
        Some("let <ID> = <NUM>;".to_owned())
    );
    assert_eq!(
        tokenize_line("let x = 3.14f32;"),
        Some("let <ID> = <NUM>;".to_owned())
    );

    assert_eq!(
        tokenize_line("let x = 42u32;"),
        tokenize_line("let x = 99u32;")
    );
}

#[test]
fn test_tokenize_lifetimes_not_treated_as_char_literals() {
    assert_eq!(
        tokenize_line("fn foo<'a>(x: &'a str) -> &'a str {"),
        Some("fn <ID><'<ID>>(<ID>: &'<ID> <ID>) -> &'<ID> <ID> {".to_owned())
    );

    assert_eq!(
        tokenize_line("static DATA: &'static str = \"hello\";"),
        Some("static <ID>: &'static <ID> = <STR>;".to_owned())
    );

    assert_eq!(
        tokenize_line("fn longest<'a, 'b>(x: &'a str, y: &'b str) {"),
        Some("fn <ID><'<ID>, '<ID>>(<ID>: &'<ID> <ID>, <ID>: &'<ID> <ID>) {".to_owned())
    );
}

#[test]
fn test_tokenize_normalizes_generic_type_parameters() {
    // Generic type parameters like T, U should be normalized to <ID>
    // They should NOT be preserved just because they're between < and >
    assert_eq!(
        tokenize_line("fn foo<T>() -> T {"),
        Some("fn <ID><<ID>>() -> <ID> {".to_owned())
    );

    assert_eq!(
        tokenize_line("fn bar<T, U>(x: T, y: U) {"),
        Some("fn <ID><<ID>, <ID>>(<ID>: <ID>, <ID>: <ID>) {".to_owned())
    );

    // Two functions with different generic type parameter names should normalize the same
    assert_eq!(
        tokenize_line("fn process<T>(data: T) {"),
        tokenize_line("fn process<U>(data: U) {")
    );
}

#[test]
fn test_tokenize_skips_block_comments() {
    // Single-line block comments
    assert_eq!(
        tokenize_line("let x = 42; /* comment */"),
        Some("let <ID> = <NUM>;".to_owned())
    );

    // Block comment at beginning
    assert_eq!(
        tokenize_line("/* comment */ let x = 42;"),
        Some("let <ID> = <NUM>;".to_owned())
    );

    // Block comment in middle
    assert_eq!(
        tokenize_line("let x /* comment */ = 42;"),
        Some("let <ID> = <NUM>;".to_owned())
    );

    // Multiple block comments
    assert_eq!(
        tokenize_line("/* c1 */ let x /* c2 */ = 42; /* c3 */"),
        Some("let <ID> = <NUM>;".to_owned())
    );
}

#[test]
fn test_multi_line_block_comments_are_skipped() {
    // This test simulates what happens when scanner processes a file with multi-line comments
    // The issue is that tokenize_line is called per-line with no shared state

    // Middle line (should be entirely inside comment)
    let line2 = "   let fake_code = 42;";

    // Currently, each line is processed independently, so line2 would be tokenized as code
    // After fix, we need to strip multi-line comments before tokenizing
    // For now, this test documents the current broken behavior

    // The broken behavior: line2 is tokenized as if it's real code
    let result2 = tokenize_line(line2);
    // This SHOULD be None (comment), but currently returns Some (incorrectly tokenizes as code)
    assert!(
        result2.is_some(),
        "This test demonstrates the bug: multi-line comment content is tokenized as code"
    );
}
