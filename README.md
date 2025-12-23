# cargo-duplicated

[![Crates.io](https://img.shields.io/crates/v/cargo-duplicated.svg)](https://crates.io/crates/cargo-duplicated)
[![Github All Releases](https://img.shields.io/github/downloads/bircni/cargo-duplicated/total.svg)](https://github.com/bircni/cargo-duplicated/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/cargo-duplicated/blob/main/LICENSE)
[![CI](https://github.com/bircni/cargo-duplicated/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/cargo-duplicated/actions/workflows/ci.yml)

A command-line tool to detect duplicated Rust code blocks with configurable thresholds.

## Features

- Scans Rust source files and reports duplicated code blocks
- Normalizes whitespace and ignores comment-only lines
- Configurable thresholds via `dups.toml`
- Optional JSON output for automation
- Exclude patterns and test detection via `#[test]` / `#[cfg(test)]`

## Installation

You need [Rust](https://www.rust-lang.org/tools/install) installed.

```sh
cargo install cargo-duplicated
```

Or build locally:

```sh
cargo build --release
```

## Usage

```text
Find duplicated Rust code blocks

Usage: cargo-duplicated [OPTIONS] [PATH]

Arguments:
  [PATH]  Path to scan [default: .]

Options:
      --config <CONFIG>   Path to config file (defaults to <path>/dups.toml)
      --format <FORMAT>   Output format [default: human] [possible values: human, json]
      --include-tests     Include files that contain #[test] or #[cfg(test)]
      --exclude <EXCLUDE> Exclude path glob (relative to root). Can be repeated.
  -h, --help              Print help
  -V, --version           Print version
```

Examples:

```sh
cargo-duplicated .
cargo-duplicated --format json .
cargo-duplicated --exclude "src/generated/**" --exclude "**/fixtures/**" .
cargo-duplicated --config ./configs/dups.toml .
```

## Configuration

Create a `dups.toml` file in the repo you want to scan. You can start from

`dups-example.toml`.

```toml
min_lines = 5
min_occurrences = 2
exclude = ["target/**", "src/generated/**"]
include_tests = false
```

Notes:

- `include_tests = false` skips files containing `#[test]` or `#[cfg(test)]`.
- The `--config` flag overrides the default `<path>/dups.toml`.
- Exit codes: `0` no duplicates, `1` duplicates found, `2` errors.

## Development

- Requires Rust 1.88+ (edition 2024)
- Linting: `cargo clippy`
- Tests: `cargo test`
