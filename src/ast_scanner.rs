//! AST-based semantic duplicate detection using syn.
//!
//! This module provides semantic code analysis by parsing Rust code into ASTs
//! and comparing their structure, allowing detection of semantically equivalent
//! code even when formatting or variable names differ.

use crate::scanner::{DuplicateBlock, Location};
use anyhow::Context;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, Item};

/// Represents a normalized AST node structure for comparison.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AstSignature {
    kind: String,
    structure: String,
}

/// Information about an AST node occurrence in the codebase.
#[derive(Debug, Clone)]
struct AstOccurrence {
    file: PathBuf,
    line: usize,
    signature: AstSignature,
}

/// Scan files for semantic duplicates using AST analysis.
pub fn find_semantic_duplicates(
    files: &[PathBuf],
    min_occurrences: usize,
    similarity_threshold: f64,
    include_tests: bool,
) -> anyhow::Result<Vec<DuplicateBlock>> {
    let mut all_occurrences: Vec<AstOccurrence> = Vec::new();

    for file_path in files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        // Skip test files if not included
        if !include_tests && crate::scanner::contains_test_markers(&content) {
            continue;
        }

        // Try to parse the file
        let Ok(syntax_tree) = syn::parse_file(&content) else {
            continue; // Skip files that don't parse
        };

        let occurrences = extract_ast_signatures(&syntax_tree, file_path);
        all_occurrences.extend(occurrences);
    }

    let duplicates = group_similar_asts(&all_occurrences, min_occurrences, similarity_threshold);
    Ok(duplicates)
}

/// Extract AST signatures from parsed Rust code.
fn extract_ast_signatures(syntax_tree: &File, file_path: &Path) -> Vec<AstOccurrence> {
    let mut visitor = AstVisitor {
        occurrences: Vec::new(),
        file_path: file_path.to_path_buf(),
    };
    visitor.visit_file(syntax_tree);
    visitor.occurrences
}

/// AST visitor that extracts signatures from function items.
struct AstVisitor {
    occurrences: Vec<AstOccurrence>,
    file_path: PathBuf,
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let signature = create_function_signature(node);

        // Use real line numbers from the span for stable baseline diffing.
        // This ensures that line numbers correspond to actual source code positions
        // and remain stable when functions are reordered in the file.
        let line = node.sig.span().start().line;

        self.occurrences.push(AstOccurrence {
            file: self.file_path.clone(),
            line,
            signature,
        });

        syn::visit::visit_item_fn(self, node);
    }
}

/// Create a normalized signature for a function.
fn create_function_signature(func: &syn::ItemFn) -> AstSignature {
    let mut structure = String::new();

    // Capture control flow structure
    for stmt in &func.block.stmts {
        structure.push_str(&normalize_statement(stmt));
        structure.push(';');
    }

    AstSignature {
        kind: "function".to_owned(),
        structure,
    }
}

/// Normalize a statement to its structural form.
pub fn normalize_statement(stmt: &syn::Stmt) -> String {
    match stmt {
        syn::Stmt::Local(_) => "LET".to_owned(),
        syn::Stmt::Item(item) => match item {
            Item::Fn(_) => "FN".to_owned(),
            Item::Struct(_) => "STRUCT".to_owned(),
            Item::Enum(_) => "ENUM".to_owned(),
            Item::Const(_)
            | Item::ExternCrate(_)
            | Item::ForeignMod(_)
            | Item::Impl(_)
            | Item::Macro(_)
            | Item::Mod(_)
            | Item::Static(_)
            | Item::Trait(_)
            | Item::TraitAlias(_)
            | Item::Type(_)
            | Item::Union(_)
            | Item::Use(_)
            | Item::Verbatim(_)
            | _ => "ITEM".to_owned(),
        },
        syn::Stmt::Expr(expr, _) => normalize_expression(expr),
        syn::Stmt::Macro(_) => "MACRO".to_owned(),
    }
}

/// Normalize an expression to its structural form.
pub fn normalize_expression(expr: &syn::Expr) -> String {
    let expr = match expr {
        syn::Expr::If(_) => "IF",
        syn::Expr::Match(_) => "MATCH",
        syn::Expr::While(_) => "WHILE",
        syn::Expr::ForLoop(_) => "FOR",
        syn::Expr::Loop(_) => "LOOP",
        syn::Expr::Call(_) => "CALL",
        syn::Expr::MethodCall(_) => "MCALL",
        syn::Expr::Binary(bin) => &format!("BIN{:?}", bin.op),
        syn::Expr::Unary(un) => &format!("UN{:?}", un.op),
        syn::Expr::Return(_) => "RET",
        syn::Expr::Break(_) => "BREAK",
        syn::Expr::Continue(_) => "CONT",
        syn::Expr::Block(_) => "BLOCK",
        syn::Expr::Assign(_) => "ASSIGN",
        syn::Expr::Array(_)
        | syn::Expr::Async(_)
        | syn::Expr::Await(_)
        | syn::Expr::Cast(_)
        | syn::Expr::Closure(_)
        | syn::Expr::Const(_)
        | syn::Expr::Field(_)
        | syn::Expr::Group(_)
        | syn::Expr::Index(_)
        | syn::Expr::Infer(_)
        | syn::Expr::Let(_)
        | syn::Expr::Lit(_)
        | syn::Expr::Macro(_)
        | syn::Expr::Paren(_)
        | syn::Expr::Path(_)
        | syn::Expr::Range(_)
        | syn::Expr::RawAddr(_)
        | syn::Expr::Reference(_)
        | syn::Expr::Repeat(_)
        | syn::Expr::Struct(_)
        | syn::Expr::Try(_)
        | syn::Expr::TryBlock(_)
        | syn::Expr::Tuple(_)
        | syn::Expr::Unsafe(_)
        | syn::Expr::Verbatim(_)
        | syn::Expr::Yield(_)
        | _ => "EXPR",
    };
    expr.to_owned()
}

/// Group similar AST signatures and create duplicate reports.
///
/// Note: Currently only exact matches (similarity == 1.0) are detected via hash-based grouping.
/// The `similarity_threshold` parameter is accepted for future extensibility but does not yet
/// affect results. Fuzzy matching with lower thresholds would require implementing approximate
/// matching algorithms (e.g., tree edit distance on AST structures).
///
/// However, exact matches are always reported regardless of the configured threshold, so users
/// can still get useful results even when `similarity_threshold` < 1.0 is specified.
fn group_similar_asts(
    occurrences: &[AstOccurrence],
    min_occurrences: usize,
    _similarity_threshold: f64,
) -> Vec<DuplicateBlock> {
    let mut groups: FxHashMap<u64, Vec<&AstOccurrence>> = FxHashMap::default();

    // Hash signatures for grouping
    for occ in occurrences {
        let hash = hash_signature(&occ.signature);
        groups.entry(hash).or_default().push(occ);
    }

    let mut duplicates = Vec::new();
    let mut seen = HashSet::new();

    for group in groups.values() {
        if group.len() < min_occurrences {
            continue;
        }

        // Report exact matches. These represent semantically identical code structures.
        let mut locations = Vec::new();
        for occ in group {
            let key = format!("{}:{}", occ.file.display(), occ.line);
            if seen.insert(key) {
                locations.push(Location {
                    file: occ.file.clone(),
                    start_line: occ.line,
                    end_line: occ.line + 10, // Approximate
                });
            }
        }

        if locations.len() >= min_occurrences {
            duplicates.push(DuplicateBlock {
                length: 10, // Approximate for AST-based detection
                occurrences: locations,
                snippet: vec![format!("AST: {}", group[0].signature.structure)],
            });
        }
    }

    duplicates.sort_by_key(|a| std::cmp::Reverse(a.occurrences.len()));
    duplicates
}

/// Hash an AST signature for grouping.
fn hash_signature(sig: &AstSignature) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    sig.hash(&mut hasher);
    hasher.finish()
}
