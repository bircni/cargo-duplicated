use crate::config::Config;
use crate::tokenizer;
use anyhow::Context;
use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct NormLine {
    norm: String,
    line_no: usize,
}

#[derive(Debug)]
struct FileLines {
    path: PathBuf,
    lines: Vec<NormLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateBlock {
    pub length: usize,
    pub occurrences: Vec<Location>,
    pub snippet: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub files_scanned: usize,
    pub duplicates: Vec<DuplicateBlock>,
}

#[derive(Debug, Clone)]
struct Occurrence {
    file_idx: usize,
    start_idx: usize,
}

pub fn scan_path(root: &Path, config: &Config) -> anyhow::Result<Report> {
    let matcher = build_exclude_matcher(root, config)?;
    let files = collect_rs_files(root, &matcher);

    // Use parallel processing for file reading
    let results: Vec<anyhow::Result<Option<FileLines>>> = files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            if !config.include_tests && contains_test_markers(&content) {
                return Ok(None);
            }

            // Choose normalization strategy based on detection mode
            let lines = match config.detection_mode {
                crate::config::DetectionMode::Token => read_token_normalized_lines(&content),
                crate::config::DetectionMode::Text
                | crate::config::DetectionMode::Semantic
                | crate::config::DetectionMode::All => read_normalized_lines(&content),
            };

            Ok(Some(FileLines {
                path: path.clone(),
                lines,
            }))
        })
        .collect();

    // Collect file_lines and propagate first error if any
    let mut file_lines = Vec::new();
    for result in results {
        match result {
            Ok(Some(fl)) => file_lines.push(fl),
            Ok(None) => {} // Test file, skip
            Err(e) => return Err(e),
        }
    }

    let duplicates = find_duplicates(&file_lines, config.min_lines, config.min_occurrences);

    Ok(Report {
        files_scanned: file_lines.len(),
        duplicates,
    })
}

pub fn render_human(report: &Report) -> anyhow::Result<String> {
    if report.duplicates.is_empty() {
        return Ok("No duplicates found.\n".to_owned());
    }

    let mut out = String::new();
    writeln!(
        out,
        "Found {} duplicated blocks across {} files.",
        report.duplicates.len(),
        report.files_scanned
    )
    .context("failed to write output")?;
    for (idx, block) in report.duplicates.iter().enumerate() {
        writeln!(
            out,
            "\n{}. {} lines, {} occurrences\n",
            idx + 1,
            block.length,
            block.occurrences.len()
        )
        .context("failed to write output")?;
        for loc in &block.occurrences {
            writeln!(
                out,
                "  - {}:{}-{}\n",
                loc.file.display(),
                loc.start_line,
                loc.end_line
            )
            .context("failed to write output")?;
        }
    }
    Ok(out)
}

fn collect_rs_files(root: &Path, matcher: &GlobSet) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        let name = entry.file_name().to_string_lossy();
        name != "target" && name != ".git"
    });

    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                && should_include(root, path, matcher)
            {
                files.push(path.to_path_buf());
            }
        }
    }

    files
}

/// Public wrapper for collecting Rust files (used by AST scanner)
pub fn collect_rs_files_public(root: &Path, config: &Config) -> anyhow::Result<Vec<PathBuf>> {
    let matcher = build_exclude_matcher(root, config)?;
    Ok(collect_rs_files(root, &matcher))
}

fn build_exclude_matcher(root: &Path, config: &Config) -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &config.exclude {
        let glob =
            Glob::new(pattern).with_context(|| format!("invalid exclude pattern: {pattern}"))?;
        builder.add(glob);
    }
    let matcher = builder.build()?;
    if !matcher.is_empty() && !root.exists() {
        return Err(anyhow::anyhow!("root path does not exist"));
    }
    Ok(matcher)
}

fn should_include(root: &Path, path: &Path, matcher: &GlobSet) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    if matcher.is_empty() {
        return true;
    }
    !matcher.is_match(rel)
}

fn read_normalized_lines(content: &str) -> Vec<NormLine> {
    let mut lines = Vec::new();
    let mut in_block_comment = false;

    for (idx, line) in content.lines().enumerate() {
        // Handle block comments
        let processed = if in_block_comment {
            if let Some(end_pos) = line.find("*/") {
                in_block_comment = false;
                line[end_pos + 2..].to_string()
            } else {
                continue;
            }
        } else if let Some(start_pos) = line.find("/*") {
            line[start_pos..].find("*/").map_or_else(
                || {
                    // Start of multi-line block comment
                    in_block_comment = true;
                    line[..start_pos].to_owned()
                },
                |end_pos| format!("{}{}", &line[..start_pos], &line[start_pos + end_pos + 2..]),
            )
        } else {
            line.to_owned()
        };

        if let Some(norm) = normalize_line(&processed) {
            lines.push(NormLine {
                norm,
                line_no: idx + 1,
            });
        }
    }
    lines
}

/// Read and normalize lines using token-based normalization.
///
/// This function handles multi-line block comments at the file level before passing
/// individual lines to `tokenize_line`, ensuring that comment state is preserved across
/// line boundaries.
fn read_token_normalized_lines(content: &str) -> Vec<NormLine> {
    let mut lines = Vec::new();
    let mut in_block_comment = false;

    for (idx, line) in content.lines().enumerate() {
        // Handle block comments (same logic as read_normalized_lines)
        let processed = if in_block_comment {
            if let Some(end_pos) = line.find("*/") {
                in_block_comment = false;
                line[end_pos + 2..].to_string()
            } else {
                continue;
            }
        } else if let Some(start_pos) = line.find("/*") {
            line[start_pos..].find("*/").map_or_else(
                || {
                    // Start of multi-line block comment
                    in_block_comment = true;
                    line[..start_pos].to_owned()
                },
                |end_pos| format!("{}{}", &line[..start_pos], &line[start_pos + end_pos + 2..]),
            )
        } else {
            line.to_owned()
        };

        if let Some(norm) = tokenizer::tokenize_line(&processed) {
            lines.push(NormLine {
                norm,
                line_no: idx + 1,
            });
        }
    }
    lines
}

fn normalize_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    let mut normalized = String::new();
    for (idx, part) in trimmed.split_whitespace().enumerate() {
        if idx > 0 {
            normalized.push(' ');
        }
        normalized.push_str(part);
    }
    Some(normalized)
}

pub fn contains_test_markers(content: &str) -> bool {
    content.contains("#[test]") || content.contains("#[cfg(test)]")
}

fn hash_block(lines: &[NormLine]) -> u64 {
    let mut hasher = FxHasher::default();
    for line in lines {
        line.norm.hash(&mut hasher);
    }
    hasher.finish()
}

fn find_duplicates(
    files: &[FileLines],
    min_lines: usize,
    min_occurrences: usize,
) -> Vec<DuplicateBlock> {
    let mut buckets: HashMap<u64, Vec<Occurrence>> = HashMap::new();

    for (file_idx, file) in files.iter().enumerate() {
        if file.lines.len() < min_lines {
            continue;
        }
        for start in 0..=file.lines.len() - min_lines {
            let hash = hash_block(&file.lines[start..start + min_lines]);
            buckets.entry(hash).or_default().push(Occurrence {
                file_idx,
                start_idx: start,
            });
        }
    }

    let mut results = Vec::new();
    let mut seen_keys = HashSet::new();

    for occurrences in buckets.values() {
        if occurrences.len() < min_occurrences {
            continue;
        }

        let mut groups: HashMap<String, Vec<Occurrence>> = HashMap::new();
        for occ in occurrences {
            let file = &files[occ.file_idx];
            let snippet = join_norm_lines(&file.lines[occ.start_idx..occ.start_idx + min_lines]);
            groups.entry(snippet).or_default().push(occ.clone());
        }

        for (snippet, group) in groups {
            if group.len() < min_occurrences {
                continue;
            }

            if can_extend_back(files, &group) {
                continue;
            }

            let extra = extend_forward(files, &group, min_lines);
            let length = min_lines + extra;

            let mut locations = Vec::new();
            for occ in &group {
                let file = &files[occ.file_idx];
                let start_line = file.lines[occ.start_idx].line_no;
                let end_line = file.lines[occ.start_idx + length - 1].line_no;
                locations.push(Location {
                    file: file.path.clone(),
                    start_line,
                    end_line,
                });
            }

            let mut full_snippet = Vec::new();
            if let Some(first) = group.first() {
                let file = &files[first.file_idx];
                for line in &file.lines[first.start_idx..first.start_idx + length] {
                    full_snippet.push(line.norm.clone());
                }
            }

            let key = dedupe_key(&snippet, length, &locations);
            if seen_keys.insert(key) {
                results.push(DuplicateBlock {
                    length,
                    occurrences: locations,
                    snippet: full_snippet,
                });
            }
        }
    }

    results.sort_by(|a, b| {
        b.length
            .cmp(&a.length)
            .then(a.occurrences.len().cmp(&b.occurrences.len()))
    });
    results
}

fn join_norm_lines(lines: &[NormLine]) -> String {
    let mut joined = String::new();
    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            joined.push('\n');
        }
        joined.push_str(&line.norm);
    }
    joined
}

fn can_extend_back(files: &[FileLines], group: &[Occurrence]) -> bool {
    if group.is_empty() {
        return false;
    }
    if group.iter().any(|occ| occ.start_idx == 0) {
        return false;
    }
    let first = &group[0];
    let prev_line = &files[first.file_idx].lines[first.start_idx - 1].norm;
    group
        .iter()
        .all(|occ| &files[occ.file_idx].lines[occ.start_idx - 1].norm == prev_line)
}

fn extend_forward(files: &[FileLines], group: &[Occurrence], min_lines: usize) -> usize {
    if group.is_empty() {
        return 0;
    }
    let first = &group[0];
    let base_lines = &files[first.file_idx].lines;
    let mut extra = 0;
    loop {
        let base_idx = first.start_idx + min_lines + extra;
        if base_idx >= base_lines.len() {
            break;
        }
        let base_norm = &base_lines[base_idx].norm;
        let all_match = group.iter().all(|occ| {
            let file_lines = &files[occ.file_idx].lines;
            let idx = occ.start_idx + min_lines + extra;
            idx < file_lines.len() && &file_lines[idx].norm == base_norm
        });
        if all_match {
            extra += 1;
        } else {
            break;
        }
    }
    extra
}

fn dedupe_key(snippet: &str, length: usize, locations: &[Location]) -> String {
    let mut key = String::new();
    key.push_str(snippet);
    key.push('|');
    key.push_str(&length.to_string());
    for loc in locations {
        key.push('|');
        key.push_str(&loc.file.to_string_lossy());
        key.push(':');
        key.push_str(&loc.start_line.to_string());
    }
    key
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use globset::GlobSetBuilder;
    use std::path::Path;

    fn make_norm_lines(lines: &[&str]) -> Vec<NormLine> {
        lines
            .iter()
            .enumerate()
            .map(|(idx, line)| NormLine {
                norm: line.to_string(),
                line_no: idx + 1,
            })
            .collect()
    }

    #[test]
    fn should_include_rejects_outside_root() {
        let matcher = GlobSetBuilder::new().build().unwrap();
        let root = Path::new("/tmp/root");
        let path = Path::new("/tmp/other/file.rs");
        assert!(!should_include(root, path, &matcher));
    }

    #[test]
    fn can_extend_back_empty_group() {
        let files: Vec<FileLines> = Vec::new();
        let group: Vec<Occurrence> = Vec::new();
        assert!(!can_extend_back(&files, &group));
    }

    #[test]
    fn extend_forward_empty_group() {
        let files: Vec<FileLines> = Vec::new();
        let group: Vec<Occurrence> = Vec::new();
        assert_eq!(extend_forward(&files, &group, 2), 0);
    }

    #[test]
    fn extend_forward_stops_on_mismatch() {
        let files = vec![
            FileLines {
                path: PathBuf::from("a.rs"),
                lines: make_norm_lines(&["a", "b", "c"]),
            },
            FileLines {
                path: PathBuf::from("b.rs"),
                lines: make_norm_lines(&["a", "b", "d"]),
            },
        ];
        let group = vec![
            Occurrence {
                file_idx: 0,
                start_idx: 0,
            },
            Occurrence {
                file_idx: 1,
                start_idx: 0,
            },
        ];

        let extra = extend_forward(&files, &group, 2);
        assert_eq!(extra, 0);
    }
}
