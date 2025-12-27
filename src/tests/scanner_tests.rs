use crate::{
    config::{Config, DetectionMode},
    scanner::{self, Report},
};
use std::fs;
use tempfile::TempDir;

fn write_file(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn report_from(dir: &TempDir, config: &Config) -> Report {
    scanner::scan_path(dir.path(), config).unwrap()
}

fn test_config() -> Config {
    Config {
        min_lines: 3,
        min_occurrences: 2,
        exclude: Vec::new(),
        include_tests: false,
        detection_mode: DetectionMode::Text,
        max_memory_mb: 2048,
        ignore_patterns: Vec::new(),
        similarity_threshold: 1.0,
        baseline_path: None,
    }
}

#[test]
fn finds_duplicates_with_defaults() {
    let dir = TempDir::new().unwrap();
    let content = r#"
fn calc() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let report = report_from(&dir, &Config::defaults());

    assert!(!report.duplicates.is_empty());
    let block = &report.duplicates[0];
    assert!(block.length >= 5);
    assert_eq!(block.occurrences.len(), 2);
}

#[test]
fn respects_config_thresholds() {
    let dir = TempDir::new().unwrap();
    let content = r"
fn alpha() {
    let x = 1;
    let y = 2;
}
";
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);
    write_file(&dir, "src/c.rs", content);

    let mut config = test_config();
    config.min_lines = 3;
    config.min_occurrences = 3;
    let report = report_from(&dir, &config);

    assert!(!report.duplicates.is_empty());
    assert_eq!(report.duplicates[0].occurrences.len(), 3);
}

#[test]
fn ignores_comment_only_blocks() {
    let dir = TempDir::new().unwrap();
    let content = r"
// same
// same
// same
";
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let report = report_from(&dir, &Config::defaults());

    assert!(report.duplicates.is_empty());
}

#[test]
fn excludes_paths_by_glob() {
    let dir = TempDir::new().unwrap();
    let content = r#"
fn alpha() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let mut config = test_config();
    config.min_lines = 5;
    config.exclude = vec!["src/b.rs".to_owned()];
    let report = report_from(&dir, &config);

    assert!(report.duplicates.is_empty());
}

#[test]
fn includes_tests_when_enabled() {
    let dir = TempDir::new().unwrap();
    let content = r#"
#[test]
fn beta() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "tests/b.rs", content);

    let mut config = test_config();
    config.include_tests = true;
    let report = report_from(&dir, &config);

    assert!(!report.duplicates.is_empty());
}

#[test]
fn excludes_tests_when_disabled() {
    let dir = TempDir::new().unwrap();
    let content = r#"
#[cfg(test)]
fn helper() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let report = report_from(&dir, &Config::defaults());

    assert!(report.duplicates.is_empty());
}

#[test]
fn render_human_empty_report() {
    let report = Report {
        files_scanned: 0,
        duplicates: Vec::new(),
    };
    let output = scanner::render_human(&report).unwrap();
    assert_eq!(output, "No duplicates found.\n");
}

#[test]
fn invalid_exclude_pattern_fails() {
    let dir = TempDir::new().unwrap();
    let content = r#"
fn alpha() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let mut config = test_config();
    config.min_lines = 5;
    config.exclude = vec!["[".to_owned()];

    let result = scanner::scan_path(dir.path(), &config);
    result.unwrap_err();
}

#[test]
fn finds_multiple_duplicate_blocks() {
    let dir = TempDir::new().unwrap();
    let content_a = r#"
fn alpha() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    let content_b = r#"
fn beta() {
    let a = 10;
    let b = 20;
    let c = a + b;
    println!("{}", c);
}
"#;
    write_file(
        &dir,
        "src/a.rs",
        &format!("{content_a}\nlet sep = 1;\n{content_b}"),
    );
    write_file(
        &dir,
        "src/b.rs",
        &format!("{content_a}\nlet sep = 2;\n{content_b}"),
    );

    let report = report_from(&dir, &Config::defaults());

    assert!(report.duplicates.len() >= 2);
}

#[test]
fn missing_root_with_excludes_fails() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing");
    let mut config = test_config();
    config.exclude = vec!["src/ignored/**".to_owned()];

    let result = scanner::scan_path(&missing, &config);
    result.unwrap_err();
}

#[test]
fn file_read_error_propagates() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let content = "fn test() {}";
    write_file(&dir, "src/test.rs", content);

    // Make the file unreadable (Unix only)
    let file_path = dir.path().join("src/test.rs");
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&file_path, perms).unwrap();

    let config = test_config();
    let result = scanner::scan_path(dir.path(), &config);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o644);
    let _ = fs::set_permissions(&file_path, perms);

    // Should propagate the error, not silently skip
    result.unwrap_err();
}

#[test]
fn token_mode_normalizes_identifiers() {
    let dir = TempDir::new().unwrap();
    let content1 = r"
fn calc() {
    let foo = 42;
    let bar = 99;
    return foo + bar;
}
";
    let content2 = r"
fn compute() {
    let alpha = 42;
    let beta = 99;
    return alpha + beta;
}
";
    write_file(&dir, "src/a.rs", content1);
    write_file(&dir, "src/b.rs", content2);

    let mut config = test_config();
    config.detection_mode = DetectionMode::Token;
    let report = report_from(&dir, &config);

    // Token mode should find these as duplicates despite different names
    assert!(!report.duplicates.is_empty());
}

#[test]
fn token_mode_different_from_text_mode() {
    let dir = TempDir::new().unwrap();
    let content1 = r"
fn calc() {
    let x = 42;
    let y = 99;
}
";
    let content2 = r"
fn calc() {
    let a = 42;
    let b = 99;
}
";
    write_file(&dir, "src/a.rs", content1);
    write_file(&dir, "src/b.rs", content2);

    // Text mode should NOT find duplicates
    let mut text_config = test_config();
    text_config.detection_mode = DetectionMode::Text;
    let text_report = report_from(&dir, &text_config);
    assert!(text_report.duplicates.is_empty());

    // Token mode SHOULD find duplicates
    let mut token_config = test_config();
    token_config.detection_mode = DetectionMode::Token;
    let token_report = report_from(&dir, &token_config);
    assert!(!token_report.duplicates.is_empty());
}
