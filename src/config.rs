use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

const DEFAULT_MIN_LINES: usize = 5;
const DEFAULT_MIN_OCCURRENCES: usize = 2;

#[derive(Debug, Clone, Deserialize)]
struct PartialConfig {
    min_lines: Option<usize>,
    min_occurrences: Option<usize>,
    exclude: Option<Vec<String>>,
    include_tests: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub min_lines: usize,
    pub min_occurrences: usize,
    pub exclude: Vec<String>,
    pub include_tests: bool,
}

impl Config {
    pub fn load(root: &Path, config_path: Option<&Path>) -> Result<Self> {
        let config_path = match config_path {
            Some(path) => path.to_path_buf(),
            None => root.join("dups.toml"),
        };
        if !config_path.exists() {
            return Ok(Self::defaults());
        }

        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let partial: PartialConfig = toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;

        Ok(Self {
            min_lines: partial.min_lines.unwrap_or(DEFAULT_MIN_LINES),
            min_occurrences: partial.min_occurrences.unwrap_or(DEFAULT_MIN_OCCURRENCES),
            exclude: partial.exclude.unwrap_or_default(),
            include_tests: partial.include_tests.unwrap_or(false),
        })
    }

    pub fn defaults() -> Self {
        Self {
            min_lines: DEFAULT_MIN_LINES,
            min_occurrences: DEFAULT_MIN_OCCURRENCES,
            exclude: Vec::new(),
            include_tests: false,
        }
    }
}
