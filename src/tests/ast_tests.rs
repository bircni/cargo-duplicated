use crate::ast_scanner;
use tempfile::TempDir;

fn write_file(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

#[test]
fn test_ast_scanner_finds_semantic_duplicates() {
    let dir = TempDir::new().unwrap();
    let content1 = r"
fn foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    let content2 = r"
fn bar() {
    let a = 1;
    let b = 2;
    return a + b;
}
";
    write_file(&dir, "a.rs", content1);
    write_file(&dir, "b.rs", content2);

    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 1.0, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    // Both functions have the same AST structure
    assert!(!duplicates.is_empty(), "Should find semantic duplicates");
}

#[test]
fn test_ast_scanner_handles_parse_errors() {
    let dir = TempDir::new().unwrap();
    let invalid = "fn incomplete {";
    write_file(&dir, "invalid.rs", invalid);

    let files = vec![dir.path().join("invalid.rs")];
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 1.0, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    // Should not crash, just skip unparsable files
    assert!(duplicates.is_empty());
}

#[test]
fn test_ast_scanner_respects_include_tests_false() {
    let dir = TempDir::new().unwrap();
    let test_content = r"
#[test]
fn test_foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    let regular_content = r"
fn bar() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    write_file(&dir, "test.rs", test_content);
    write_file(&dir, "regular.rs", regular_content);

    let files = vec![dir.path().join("test.rs"), dir.path().join("regular.rs")];
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 1.0, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    // Should not find duplicates since test file is excluded
    assert!(duplicates.is_empty());
}

#[test]
fn test_ast_scanner_respects_include_tests_true() {
    let dir = TempDir::new().unwrap();
    let test_content = r"
#[test]
fn test_foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    let regular_content = r"
fn bar() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    write_file(&dir, "test.rs", test_content);
    write_file(&dir, "regular.rs", regular_content);

    let files = vec![dir.path().join("test.rs"), dir.path().join("regular.rs")];
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 1.0, true);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    // Should find duplicates since test file is included
    assert!(!duplicates.is_empty());
}

#[test]
fn test_ast_scanner_distinguishes_multiple_functions_in_same_file() {
    let dir = TempDir::new().unwrap();
    let content = r"
fn foo() {
    let x = 1;
    let y = 2;
    return x + y;
}

fn bar() {
    let x = 1;
    let y = 2;
    return x + y;
}

fn baz() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    write_file(&dir, "multi.rs", content);

    let files = vec![dir.path().join("multi.rs")];
    let result = ast_scanner::find_semantic_duplicates(&files, 3, 1.0, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    // Should find all 3 functions as duplicates of each other
    assert!(!duplicates.is_empty());
    assert_eq!(duplicates[0].occurrences.len(), 3);
}

#[test]
fn test_normalize_expression() {
    let expr = syn::parse_str::<syn::Expr>("if x { y }").unwrap();
    assert_eq!(ast_scanner::normalize_expression(&expr), "IF");
}

#[test]
fn test_normalize_statement() {
    let stmt = syn::parse_str::<syn::Stmt>("let x = 5;").unwrap();
    assert_eq!(ast_scanner::normalize_statement(&stmt), "LET");
}

#[test]
fn test_semantic_line_numbers_are_unstable_when_functions_reordered() {
    let dir = TempDir::new().unwrap();

    let original_content = r"
fn first() {
    let x = 1;
    return x + 1;
}

fn second() {
    let y = 2;
    return y + 2;
}

fn third() {
    let z = 3;
    return z + 3;
}
";

    let modified_content = r"
fn new_function() {
    let w = 0;
    return w;
}

fn first() {
    let x = 1;
    return x + 1;
}

fn second() {
    let y = 2;
    return y + 2;
}

fn third() {
    let z = 3;
    return z + 3;
}
";

    write_file(&dir, "original.rs", original_content);
    let files_original = vec![dir.path().join("original.rs")];
    let result_original = ast_scanner::find_semantic_duplicates(&files_original, 1, 1.0, false);

    write_file(&dir, "modified.rs", modified_content);
    let files_modified = vec![dir.path().join("modified.rs")];
    let result_modified = ast_scanner::find_semantic_duplicates(&files_modified, 1, 1.0, false);

    result_original.unwrap();
    result_modified.unwrap();
}

#[test]
fn test_ast_scanner_with_similarity_threshold_below_1() {
    let dir = TempDir::new().unwrap();
    let content1 = r"
fn foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    let content2 = r"
fn bar() {
    let a = 1;
    let b = 2;
    return a + b;
}
";
    write_file(&dir, "a.rs", content1);
    write_file(&dir, "b.rs", content2);

    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];

    // When similarity threshold is 0.9 (< 1.0), exact matches should still be reported
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 0.9, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    assert!(
        !duplicates.is_empty(),
        "Exact matches should be reported even when similarity threshold < 1.0"
    );
}

#[test]
fn test_ast_scanner_with_zero_similarity_threshold() {
    let dir = TempDir::new().unwrap();
    let content1 = r"
fn foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
";
    let content2 = r"
fn bar() {
    let a = 1;
    let b = 2;
    return a + b;
}
";
    write_file(&dir, "a.rs", content1);
    write_file(&dir, "b.rs", content2);

    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];

    // Even with threshold 0.0, exact matches should still be reported
    let result = ast_scanner::find_semantic_duplicates(&files, 2, 0.0, false);

    assert!(result.is_ok());
    let duplicates = result.unwrap();
    assert!(
        !duplicates.is_empty(),
        "Exact matches should be reported regardless of similarity threshold"
    );
}
