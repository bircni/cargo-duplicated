use crate::{
    baseline::{self, Baseline},
    scanner::{DuplicateBlock, Report},
};
use tempfile::NamedTempFile;

#[test]
fn test_baseline_save_load() {
    let report = Report {
        files_scanned: 5,
        duplicates: vec![],
    };

    let baseline = Baseline::from_report(report);
    let temp_file = NamedTempFile::new().unwrap();

    baseline.save(temp_file.path()).unwrap();
    let loaded = Baseline::load(temp_file.path()).unwrap();

    assert_eq!(baseline.report.files_scanned, loaded.report.files_scanned);
}

#[test]
fn test_diff_reports() {
    use crate::scanner::Location;
    use std::path::PathBuf;

    let baseline = Report {
        files_scanned: 3,
        duplicates: vec![DuplicateBlock {
            length: 5,
            occurrences: vec![Location {
                file: PathBuf::from("a.rs"),
                start_line: 1,
                end_line: 5,
            }],
            snippet: vec![String::from("line1")],
        }],
    };

    let current = Report {
        files_scanned: 3,
        duplicates: vec![
            DuplicateBlock {
                length: 5,
                occurrences: vec![
                    Location {
                        file: PathBuf::from("a.rs"),
                        start_line: 1,
                        end_line: 5,
                    },
                    Location {
                        file: PathBuf::from("b.rs"),
                        start_line: 10,
                        end_line: 14,
                    },
                ],
                snippet: vec![String::from("line1")],
            },
            DuplicateBlock {
                length: 7,
                occurrences: vec![Location {
                    file: PathBuf::from("c.rs"),
                    start_line: 20,
                    end_line: 26,
                }],
                snippet: vec![String::from("line2")],
            },
        ],
    };

    let diff = baseline::diff_reports(&current, &baseline);
    // Should have 2 duplicate blocks:
    // 1. line1 with the new occurrence in b.rs
    // 2. line2 which is entirely new
    assert_eq!(diff.duplicates.len(), 2);

    // Find the line1 duplicate (should have 1 new occurrence in b.rs)
    let line1_dup = diff
        .duplicates
        .iter()
        .find(|d| d.snippet[0] == "line1")
        .unwrap();
    assert_eq!(line1_dup.occurrences.len(), 1);
    assert_eq!(line1_dup.occurrences[0].file, PathBuf::from("b.rs"));

    // Find the line2 duplicate (entirely new, should have 1 occurrence)
    let line2_dup = diff
        .duplicates
        .iter()
        .find(|d| d.snippet[0] == "line2")
        .unwrap();
    assert_eq!(line2_dup.length, 7);
    assert_eq!(line2_dup.occurrences.len(), 1);
}

#[test]
fn test_diff_reports_filters_all_old_occurrences() {
    use crate::scanner::Location;
    use std::path::PathBuf;

    // Baseline has occurrences in a.rs and b.rs
    let baseline = Report {
        files_scanned: 3,
        duplicates: vec![DuplicateBlock {
            length: 5,
            occurrences: vec![
                Location {
                    file: PathBuf::from("a.rs"),
                    start_line: 1,
                    end_line: 5,
                },
                Location {
                    file: PathBuf::from("b.rs"),
                    start_line: 1,
                    end_line: 5,
                },
            ],
            snippet: vec![String::from("duplicate")],
        }],
    };

    // Current has the same occurrences (no new ones)
    let current = Report {
        files_scanned: 3,
        duplicates: vec![DuplicateBlock {
            length: 5,
            occurrences: vec![
                Location {
                    file: PathBuf::from("a.rs"),
                    start_line: 1,
                    end_line: 5,
                },
                Location {
                    file: PathBuf::from("b.rs"),
                    start_line: 1,
                    end_line: 5,
                },
            ],
            snippet: vec![String::from("duplicate")],
        }],
    };

    let diff = baseline::diff_reports(&current, &baseline);
    // Should have no new duplicates
    assert_eq!(diff.duplicates.len(), 0);
}

#[test]
fn test_diff_reports_includes_only_new_occurrences_of_known_pattern() {
    use crate::scanner::Location;
    use std::path::PathBuf;

    let baseline = Report {
        files_scanned: 2,
        duplicates: vec![DuplicateBlock {
            length: 5,
            occurrences: vec![
                Location {
                    file: PathBuf::from("a.rs"),
                    start_line: 1,
                    end_line: 5,
                },
                Location {
                    file: PathBuf::from("b.rs"),
                    start_line: 1,
                    end_line: 5,
                },
            ],
            snippet: vec![String::from("pattern")],
        }],
    };

    // Now the pattern appears in 3 new files: c.rs, d.rs, e.rs
    let current = Report {
        files_scanned: 5,
        duplicates: vec![DuplicateBlock {
            length: 5,
            occurrences: vec![
                Location {
                    file: PathBuf::from("a.rs"),
                    start_line: 1,
                    end_line: 5,
                },
                Location {
                    file: PathBuf::from("b.rs"),
                    start_line: 1,
                    end_line: 5,
                },
                Location {
                    file: PathBuf::from("c.rs"),
                    start_line: 10,
                    end_line: 14,
                },
                Location {
                    file: PathBuf::from("d.rs"),
                    start_line: 20,
                    end_line: 24,
                },
                Location {
                    file: PathBuf::from("e.rs"),
                    start_line: 30,
                    end_line: 34,
                },
            ],
            snippet: vec![String::from("pattern")],
        }],
    };

    let diff = baseline::diff_reports(&current, &baseline);
    // Should have 1 duplicate block with 3 new occurrences
    assert_eq!(diff.duplicates.len(), 1);
    assert_eq!(diff.duplicates[0].occurrences.len(), 3);

    let new_files: Vec<_> = diff.duplicates[0]
        .occurrences
        .iter()
        .map(|o| o.file.to_str().unwrap())
        .collect();
    assert!(new_files.contains(&"c.rs"));
    assert!(new_files.contains(&"d.rs"));
    assert!(new_files.contains(&"e.rs"));
}
