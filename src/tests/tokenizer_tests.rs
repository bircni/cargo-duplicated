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
