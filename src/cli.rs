use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Parser)]
#[command(name = "cargo-duplicated", version, about = "Find duplicated Rust code blocks")]
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
}
