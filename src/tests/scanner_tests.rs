use crate::{
    config::Config,
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

fn report_from(dir: &TempDir, config: Config) -> Report {
    scanner::scan_path(dir.path(), &config).unwrap()
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

    let report = report_from(&dir, Config::defaults());

    assert!(!report.duplicates.is_empty());
    let block = &report.duplicates[0];
    assert!(block.length >= 5);
    assert_eq!(block.occurrences.len(), 2);
}

#[test]
fn respects_config_thresholds() {
    let dir = TempDir::new().unwrap();
    let content = r#"
fn alpha() {
    let x = 1;
    let y = 2;
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);
    write_file(&dir, "src/c.rs", content);

    let config = Config {
        min_lines: 3,
        min_occurrences: 3,
        exclude: Vec::new(),
        include_tests: false,
    };
    let report = report_from(&dir, config);

    assert!(!report.duplicates.is_empty());
    assert_eq!(report.duplicates[0].occurrences.len(), 3);
}

#[test]
fn ignores_comment_only_blocks() {
    let dir = TempDir::new().unwrap();
    let content = r#"
// same
// same
// same
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let report = report_from(&dir, Config::defaults());

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

    let config = Config {
        min_lines: 5,
        min_occurrences: 2,
        exclude: vec!["src/b.rs".to_string()],
        include_tests: false,
    };
    let report = report_from(&dir, config);

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

    let config = Config {
        min_lines: 5,
        min_occurrences: 2,
        exclude: Vec::new(),
        include_tests: true,
    };
    let report = report_from(&dir, config);

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

    let report = report_from(&dir, Config::defaults());

    assert!(report.duplicates.is_empty());
}
