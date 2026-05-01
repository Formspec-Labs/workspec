// Rust guideline compliant 2026-04-18

//! CI gate: every production schema in `schemas/**/*.schema.json` MUST have
//! zero `SCHEMA-DOC-001` violations.
//!
//! The gate has no schema-level exemptions. New schema leaves must land with
//! descriptions and examples instead of raising a debt ceiling.
//!
//! To diagnose a failure, run:
//!   cargo run -p wos-lint --example count_schema_violations -- <path> --list

use std::path::{Path, PathBuf};

#[test]
fn all_production_schemas_have_zero_schema_doc_violations() {
    let workspace = workspace_root();
    let schemas_dir = workspace.join("schemas");

    let schema_files = collect_schema_files(&schemas_dir);
    assert!(
        !schema_files.is_empty(),
        "no *.schema.json files found under {}",
        schemas_dir.display()
    );

    let mut violations_by_file: Vec<(String, usize)> = Vec::new();

    for abs_path in &schema_files {
        let rel_path = abs_path
            .strip_prefix(&workspace)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let json = std::fs::read_to_string(abs_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", abs_path.display()));

        let diagnostics = wos_lint::lint_schema(&json)
            .unwrap_or_else(|e| panic!("lint_schema failed for {}: {e}", abs_path.display()));

        if !diagnostics.is_empty() {
            violations_by_file.push((rel_path, diagnostics.len()));
        }
    }

    assert!(
        violations_by_file.is_empty(),
        "SCHEMA-DOC-001 regressions detected — {} schema(s) have violations:\n{}\n\
         Run `cargo run -p wos-lint --example count_schema_violations -- <path> --list` \
         to see per-property details.",
        violations_by_file.len(),
        violations_by_file
            .iter()
            .map(|(path, count)| format!("  {path}: {count} violation(s)"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Recursively collect all `*.schema.json` files under `dir`.
fn collect_schema_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_schema_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_schema_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_schema_files_recursive(&path, files);
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".schema.json"))
        {
            files.push(path);
        }
    }
}

/// Returns the workspace root: two directories above `CARGO_MANIFEST_DIR`
/// (i.e., `crates/wos-lint` → `crates` → workspace root).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}
