use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MIN_LINES: usize = 5;
const DEFAULT_MIN_OCCURRENCES: usize = 2;
const DEFAULT_MAX_MEMORY_MB: usize = 2048;
const DEFAULT_SIMILARITY_THRESHOLD: f64 = 1.0;

/// Detection mode for duplicate scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMode {
    /// Text-based exact matching (fastest)
    #[default]
    Text,
    /// Token-based structural matching
    Token,
    /// AST-based semantic matching (slowest, most accurate)
    Semantic,
    /// Run all modes and merge results
    All,
}

#[derive(Debug, Clone, Deserialize)]
struct PartialConfig {
    min_lines: Option<usize>,
    min_occurrences: Option<usize>,
    exclude: Option<Vec<String>>,
    include_tests: Option<bool>,
    detection_mode: Option<DetectionMode>,
    max_memory_mb: Option<usize>,
    ignore_patterns: Option<Vec<String>>,
    similarity_threshold: Option<f64>,
    baseline_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub min_lines: usize,
    pub min_occurrences: usize,
    pub exclude: Vec<String>,
    pub include_tests: bool,
    pub detection_mode: DetectionMode,
    pub max_memory_mb: usize,
    #[expect(
        dead_code,
        reason = "Will be used for pattern filtering in future enhancement"
    )]
    pub ignore_patterns: Vec<String>,
    pub similarity_threshold: f64,
    #[expect(
        dead_code,
        reason = "Will be used for baseline comparison in future enhancement"
    )]
    pub baseline_path: Option<PathBuf>,
}

impl Config {
    pub fn load(root: &Path, config_path: Option<&Path>) -> Result<Self> {
        let config_path = config_path.map_or_else(|| root.join("dups.toml"), Path::to_path_buf);
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
            detection_mode: partial.detection_mode.unwrap_or_default(),
            max_memory_mb: partial.max_memory_mb.unwrap_or(DEFAULT_MAX_MEMORY_MB),
            ignore_patterns: partial.ignore_patterns.unwrap_or_default(),
            similarity_threshold: partial
                .similarity_threshold
                .unwrap_or(DEFAULT_SIMILARITY_THRESHOLD)
                .clamp(0.0, 1.0),
            baseline_path: partial.baseline_path,
        })
    }

    pub const fn defaults() -> Self {
        Self {
            min_lines: DEFAULT_MIN_LINES,
            min_occurrences: DEFAULT_MIN_OCCURRENCES,
            exclude: Vec::new(),
            include_tests: false,
            detection_mode: DetectionMode::Text,
            max_memory_mb: DEFAULT_MAX_MEMORY_MB,
            ignore_patterns: Vec::new(),
            similarity_threshold: DEFAULT_SIMILARITY_THRESHOLD,
            baseline_path: None,
        }
    }
}
