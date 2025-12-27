//! Baseline caching for incremental duplicate detection.
//!
//! This module provides functionality to cache scan results and compare them
//! against previous scans, enabling incremental analysis and diff mode.

use crate::scanner::Report;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A cached baseline of previous scan results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub timestamp: String,
    pub report: Report,
}

impl Baseline {
    /// Load a baseline from a file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read baseline from {}", path.display()))?;
        let baseline: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse baseline from {}", path.display()))?;
        Ok(baseline)
    }

    /// Save a baseline to a file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self).context("failed to serialize baseline")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write baseline to {}", path.display()))?;
        Ok(())
    }

    /// Create a new baseline from a report.
    pub fn from_report(report: Report) -> Self {
        use std::time::SystemTime;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or_else(|_| String::from("unknown"), |d| d.as_secs().to_string());

        Self { timestamp, report }
    }
}

/// Compare two reports and return only new duplicates and new occurrences of known duplicates.
pub fn diff_reports(current: &Report, baseline: &Report) -> Report {
    use std::collections::{HashMap, HashSet};

    // Group baseline occurrences by their snippet signature
    let mut baseline_occurrences: HashMap<String, HashSet<String>> = HashMap::new();
    for dup in &baseline.duplicates {
        let snippet_sig = create_snippet_signature(dup);
        let occurrence_sigs: HashSet<_> = dup
            .occurrences
            .iter()
            .map(|occ| format!("{}:{}:{}", occ.file.display(), occ.start_line, occ.end_line))
            .collect();
        baseline_occurrences.insert(snippet_sig, occurrence_sigs);
    }

    // Filter current duplicates to only include new occurrences
    let mut new_duplicates = Vec::new();
    for dup in &current.duplicates {
        let snippet_sig = create_snippet_signature(dup);

        // Get baseline occurrences for this snippet (if any)
        let baseline_occs = baseline_occurrences.get(&snippet_sig);

        // Filter to only new occurrences
        let new_occurrences: Vec<_> = dup
            .occurrences
            .iter()
            .filter(|occ| {
                let occ_sig = format!("{}:{}:{}", occ.file.display(), occ.start_line, occ.end_line);
                !baseline_occs.is_some_and(|set| set.contains(&occ_sig))
            })
            .cloned()
            .collect();

        // Only include this duplicate if there are new occurrences
        if !new_occurrences.is_empty() {
            new_duplicates.push(crate::scanner::DuplicateBlock {
                length: dup.length,
                occurrences: new_occurrences,
                snippet: dup.snippet.clone(),
            });
        }
    }

    Report {
        files_scanned: current.files_scanned,
        duplicates: new_duplicates,
    }
}

/// Create a signature based only on the snippet content.
fn create_snippet_signature(dup: &crate::scanner::DuplicateBlock) -> String {
    let mut sig = format!("len:{}", dup.length);
    for line in &dup.snippet {
        sig.push('|');
        sig.push_str(line);
    }
    sig
}
