use crate::config::DetectionMode;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum CliDetectionMode {
    Text,
    Token,
    Semantic,
    All,
}

impl From<CliDetectionMode> for DetectionMode {
    fn from(mode: CliDetectionMode) -> Self {
        match mode {
            CliDetectionMode::Text => Self::Text,
            CliDetectionMode::Token => Self::Token,
            CliDetectionMode::Semantic => Self::Semantic,
            CliDetectionMode::All => Self::All,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "cargo-duplicated",
    version,
    about = "Find duplicated Rust code blocks"
)]
pub struct Cli {
    /// Path to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Path to config file (defaults to <path>/dups.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Include files under tests/
    #[arg(long)]
    pub include_tests: bool,

    /// Exclude path glob (relative to root). Can be repeated.
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Detection mode: text (exact), token (structural), semantic (AST), or all
    #[arg(long, value_enum)]
    pub mode: Option<CliDetectionMode>,

    /// Maximum memory usage in MB (default: 2048)
    #[arg(long)]
    pub max_memory: Option<usize>,

    /// Compare against baseline file and report only new duplicates
    #[arg(long)]
    pub diff: Option<PathBuf>,

    /// Save current results as baseline for future diff
    #[arg(long)]
    pub save_baseline: Option<PathBuf>,

    /// Similarity threshold for semantic mode (0.0-1.0, default: 1.0)
    #[arg(long)]
    pub similarity: Option<f64>,
}
