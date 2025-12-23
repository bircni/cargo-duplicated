use crate::cli::{Cli, OutputFormat};
use crate::run_with;
use std::fs;
use tempfile::TempDir;

fn write_file(dir: &TempDir, rel: &str, content: &str) {
    let path = dir.path().join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn run_with_human_output_reports_duplicates() {
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

    let cli = Cli {
        path: dir.path().to_path_buf(),
        config: None,
        format: OutputFormat::Human,
        include_tests: false,
        exclude: Vec::new(),
    };
    let result = run_with(cli).unwrap();

    assert_eq!(result.exit_code, 1);
    assert!(result.output.contains("duplicated blocks"));
}

#[test]
fn run_with_json_output_reports_empty() {
    let dir = TempDir::new().unwrap();
    let content = r"
fn alpha() {
    let x = 1;
}
";
    write_file(&dir, "src/a.rs", content);

    let cli = Cli {
        path: dir.path().to_path_buf(),
        config: None,
        format: OutputFormat::Json,
        include_tests: false,
        exclude: Vec::new(),
    };
    let result = run_with(cli).unwrap();

    assert_eq!(result.exit_code, 0);
    let value: serde_json::Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(value["duplicates"].as_array().unwrap().len(), 0);
}

#[test]
fn run_with_overrides_include_and_exclude() {
    let dir = TempDir::new().unwrap();
    let content = r#"
#[test]
fn alpha() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    let cli = Cli {
        path: dir.path().to_path_buf(),
        config: None,
        format: OutputFormat::Human,
        include_tests: true,
        exclude: vec!["src/b.rs".to_owned()],
    };
    let result = run_with(cli).unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(result.output.contains("No duplicates found"));
}
