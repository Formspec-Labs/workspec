// Rust guideline compliant 2026-05-01

//! CI gate: open `type: string` leaves (no `enum`/`const`/`pattern` at the
//! leaf) ratchet downward. Every production schema has a fixed expected count;
//! any drift — up or down — fails. This forces:
//!
//! - regressions to surface: a leaf that loses its constraint can't slip in.
//! - cleanups to be recorded: tightening a leaf's constraint requires lowering
//!   the baseline below, which is the audit trail of the closed-vocabulary
//!   hardening pass (companion to `schema_doc_zero_regression`).
//!
//! `SCHEMA-DOC-001` already enforces "prose says closed, no constraint" — this
//! ratchet enforces "the open count we accept today is the open count we
//! accept tomorrow, until the table is lowered."
//!
//! To diagnose:
//!   cargo run -p wos-lint --example schema_string_leaf_report -- <path> [--csv]
//!
//! When you tighten a leaf to `enum`/`const`/`pattern`, lower the matching row
//! in `EXPECTED_OPEN_STRING_LEAVES` by the number of leaves you closed. When
//! you add or remove a schema file, update the table to match.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Per-schema expected open-string-leaf count. Keys are workspace-relative
/// paths with forward slashes. **Lower these when leaves are tightened; never
/// raise.**
const EXPECTED_OPEN_STRING_LEAVES: &[(&str, usize)] = &[
    ("schemas/conformance/conformance-trace.schema.json", 22),
    ("schemas/lint/wos-lint-diagnostic.schema.json", 6),
    ("schemas/mcp/wos-mcp-tools.schema.json", 1),
    ("schemas/sidecars/wos-delivery.schema.json", 29),
    ("schemas/sidecars/wos-ontology-alignment.schema.json", 20),
    ("schemas/synth/wos-synth-trace.schema.json", 0),
    ("schemas/wos-case-instance.schema.json", 49),
    ("schemas/wos-provenance-log.schema.json", 20),
    ("schemas/wos-tooling.schema.json", 54),
    ("schemas/wos-workflow.schema.json", 189),
];

#[test]
fn open_string_leaf_count_matches_baseline_per_schema() {
    let workspace = workspace_root();
    let schemas_dir = workspace.join("schemas");
    let schema_files = collect_schema_files(&schemas_dir);
    assert!(
        !schema_files.is_empty(),
        "no *.schema.json files found under {}",
        schemas_dir.display()
    );

    let baseline: BTreeMap<&str, usize> = EXPECTED_OPEN_STRING_LEAVES.iter().copied().collect();

    let mut actual: BTreeMap<String, usize> = BTreeMap::new();
    for abs_path in &schema_files {
        let rel_path = abs_path
            .strip_prefix(&workspace)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .replace('\\', "/");
        let json = std::fs::read_to_string(abs_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", abs_path.display()));
        let root: serde_json::Value = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", abs_path.display()));
        let inv = wos_lint::rules::schema_doc::inventory_string_leaves(&root);
        actual.insert(rel_path, inv.open_string_leaves());
    }

    let mut drift: Vec<String> = Vec::new();

    for (path, expected) in &baseline {
        match actual.get(*path) {
            None => drift.push(format!(
                "  {path}: expected {expected} open leaves but file is missing"
            )),
            Some(found) if found != expected => {
                let direction = if found > expected { "ROSE" } else { "FELL" };
                drift.push(format!(
                    "  {path}: open leaves {direction} {expected} -> {found} \
                     (delta {:+})",
                    *found as isize - *expected as isize
                ));
            }
            Some(_) => {}
        }
    }
    for path in actual.keys() {
        if !baseline.contains_key(path.as_str()) {
            drift.push(format!(
                "  {path}: not registered in EXPECTED_OPEN_STRING_LEAVES \
                 (add it with the current open-leaf count)"
            ));
        }
    }

    assert!(
        drift.is_empty(),
        "open-string-leaf ratchet drift detected — update \
         EXPECTED_OPEN_STRING_LEAVES (lower when you tighten leaves; raise \
         is forbidden):\n{}\n\n\
         Diagnose with: cargo run -p wos-lint --example \
         schema_string_leaf_report -- <path> --csv",
        drift.join("\n"),
    );
}

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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}
