use clap::Parser;
use cli::{Cli, OutputFormat};
use config::Config;
use scanner::{render_human, scan_path};

mod cli;
mod config;
mod scanner;
#[cfg(test)]
mod tests;

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
    if cli.include_tests {
        config.include_tests = true;
    }
    if !cli.exclude.is_empty() {
        config.exclude.extend(cli.exclude);
    }

    let report = scan_path(&cli.path, &config)?;

    let output = match cli.format {
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
        OutputFormat::Human => render_human(&report)?,
    };

    let exit_code = i32::from(!report.duplicates.is_empty());
    Ok(RunResult { exit_code, output })
}
