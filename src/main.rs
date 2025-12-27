use clap::Parser;
use cli::{Cli, OutputFormat};
use config::Config;
use scanner::{Report, render_human, scan_path};

mod ast_scanner;
mod baseline;
mod cli;
mod config;
mod scanner;
#[cfg(test)]
mod tests;
mod tokenizer;

fn main() {
    let exit_code = match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err:?}");
            2
        }
    };
    std::process::exit(exit_code);
}

fn run() -> anyhow::Result<i32> {
    let cli = Cli::parse();
    let result = run_with(cli)?;
    print!("{}", result.output);
    Ok(result.exit_code)
}

struct RunResult {
    exit_code: i32,
    output: String,
}

fn run_with(cli: Cli) -> anyhow::Result<RunResult> {
    let config_path = cli.config.as_deref();
    let mut config = Config::load(&cli.path, config_path)?;

    // Apply CLI overrides
    if cli.include_tests {
        config.include_tests = true;
    }
    if !cli.exclude.is_empty() {
        config.exclude.extend(cli.exclude);
    }
    if let Some(mode) = cli.mode {
        config.detection_mode = mode.into();
    }
    if let Some(max_mem) = cli.max_memory {
        config.max_memory_mb = max_mem;
    }
    if let Some(sim) = cli.similarity {
        config.similarity_threshold = sim.clamp(0.0, 1.0);
    }

    // Run detection based on mode
    let mut report = match config.detection_mode {
        config::DetectionMode::Text | config::DetectionMode::Token => {
            scan_path(&cli.path, &config)?
        }
        config::DetectionMode::Semantic => {
            let files = scanner::collect_rs_files_public(&cli.path, &config)?;
            let semantic_dups = ast_scanner::find_semantic_duplicates(
                &files,
                config.min_occurrences,
                config.similarity_threshold,
                config.include_tests,
            )?;
            Report {
                files_scanned: files.len(),
                duplicates: semantic_dups,
            }
        }
        config::DetectionMode::All => {
            // Run all modes and merge results
            let text_report = {
                let mut cfg = config.clone();
                cfg.detection_mode = config::DetectionMode::Text;
                scan_path(&cli.path, &cfg)?
            };

            let token_report = {
                let mut cfg = config.clone();
                cfg.detection_mode = config::DetectionMode::Token;
                scan_path(&cli.path, &cfg)?
            };

            let files = scanner::collect_rs_files_public(&cli.path, &config)?;
            let semantic_dups = ast_scanner::find_semantic_duplicates(
                &files,
                config.min_occurrences,
                config.similarity_threshold,
                config.include_tests,
            )?;

            // Merge all duplicates
            let mut all_dups = text_report.duplicates;
            all_dups.extend(token_report.duplicates);
            all_dups.extend(semantic_dups);

            // Deduplicate by creating unique keys
            let mut seen_keys = std::collections::HashSet::new();
            all_dups.retain(|dup| {
                let key = format!("{}:{:?}", dup.length, dup.snippet);
                seen_keys.insert(key)
            });

            Report {
                files_scanned: text_report.files_scanned,
                duplicates: all_dups,
            }
        }
    };

    // Handle diff mode
    if let Some(baseline_path) = cli.diff {
        let baseline = baseline::Baseline::load(&baseline_path)?;
        report = baseline::diff_reports(&report, &baseline.report);
    }

    // Save baseline if requested
    if let Some(save_path) = cli.save_baseline {
        let baseline = baseline::Baseline::from_report(report.clone());
        baseline.save(&save_path)?;
    }

    let output = match cli.format {
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
        OutputFormat::Human => render_human(&report)?,
    };

    let exit_code = i32::from(!report.duplicates.is_empty());
    Ok(RunResult { exit_code, output })
}
