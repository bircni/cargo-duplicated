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
    let config_path = cli.config.as_deref();
    let mut config = Config::load(&cli.path, config_path)?;
    if cli.include_tests {
        config.include_tests = true;
    }
    if !cli.exclude.is_empty() {
        config.exclude.extend(cli.exclude);
    }

    let report = scan_path(&cli.path, &config)?;

    match cli.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
        }
        OutputFormat::Human => {
            let text = render_human(&report)?;
            print!("{text}");
        }
    }

    if report.duplicates.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}
