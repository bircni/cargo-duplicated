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
        mode: None,
        max_memory: None,
        diff: None,
        save_baseline: None,
        similarity: None,
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
        mode: None,
        max_memory: None,
        diff: None,
        save_baseline: None,
        similarity: None,
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
        mode: None,
        max_memory: None,
        diff: None,
        save_baseline: None,
        similarity: None,
    };
    let result = run_with(cli).unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(result.output.contains("No duplicates found"));
}

#[test]
fn save_baseline_with_diff_saves_full_report() {
    let dir = TempDir::new().unwrap();
    let content = r#"
fn shared() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}
"#;
    write_file(&dir, "src/a.rs", content);
    write_file(&dir, "src/b.rs", content);

    // Create a baseline with the initial duplicates
    let baseline_path = dir.path().join("baseline.json");
    let save_baseline_path = dir.path().join("new_baseline.json");

    // First run: create baseline
    let cli_first = Cli {
        path: dir.path().to_path_buf(),
        config: None,
        format: OutputFormat::Json,
        include_tests: false,
        exclude: Vec::new(),
        mode: None,
        max_memory: None,
        diff: None,
        save_baseline: Some(baseline_path.clone()),
        similarity: None,
    };
    run_with(cli_first).unwrap();

    // Add a new duplicate file
    write_file(&dir, "src/c.rs", content);

    // Second run: diff against baseline and save new baseline
    let cli_second = Cli {
        path: dir.path().to_path_buf(),
        config: None,
        format: OutputFormat::Json,
        include_tests: false,
        exclude: Vec::new(),
        mode: None,
        max_memory: None,
        diff: Some(baseline_path),
        save_baseline: Some(save_baseline_path.clone()),
        similarity: None,
    };
    run_with(cli_second).unwrap();

    // Load the saved baseline and verify it contains ALL occurrences, not just the diff
    let baseline_content = fs::read_to_string(&save_baseline_path).unwrap();
    let baseline: crate::baseline::Baseline = serde_json::from_str(&baseline_content).unwrap();

    // The saved baseline should have 3 occurrences (a.rs, b.rs, c.rs), not just the new one (c.rs)
    assert_eq!(baseline.report.duplicates.len(), 1);
    assert_eq!(
        baseline.report.duplicates[0].occurrences.len(),
        3,
        "Saved baseline should contain all occurrences (a.rs, b.rs, c.rs), not just the diff"
    );
}
