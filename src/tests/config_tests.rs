use crate::config::Config;
use std::fs;
use tempfile::TempDir;

#[test]
fn load_defaults_when_missing() {
    let dir = TempDir::new().unwrap();
    let config = Config::load(dir.path(), None).unwrap();

    assert_eq!(config.min_lines, 5);
    assert_eq!(config.min_occurrences, 2);
    assert!(config.exclude.is_empty());
    assert!(!config.include_tests);
}

#[test]
fn load_from_custom_path() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("custom.toml");
    let content = r#"
min_lines = 3
min_occurrences = 4
exclude = ["src/generated/**"]
include_tests = true
"#;
    fs::write(&config_path, content).unwrap();

    let config = Config::load(dir.path(), Some(&config_path)).unwrap();

    assert_eq!(config.min_lines, 3);
    assert_eq!(config.min_occurrences, 4);
    assert_eq!(config.exclude, vec!["src/generated/**".to_owned()]);
    assert!(config.include_tests);
}

#[test]
fn load_invalid_toml_fails() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("dups.toml");
    fs::write(&config_path, "min_lines =").unwrap();

    let result = Config::load(dir.path(), Some(&config_path));
    result.unwrap_err();
}
