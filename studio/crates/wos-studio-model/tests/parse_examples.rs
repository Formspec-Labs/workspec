// Rust guideline compliant 2026-05-02

//! Parse every JSON document under `studio/examples/` that carries a
//! `$wosStudio*` marker through the typed [`StudioDocument`] enum.
//!
//! Mirrors the Python `test_studio_examples.py` coverage but in Rust so
//! the typed model stays honest against the canonical vertical-slice
//! examples. Failures here mean either:
//! - The example artifact drifted from the schema, OR
//! - The typed model's envelope is missing a field the example exercises.
//!
//! ## Strictness (R9.3)
//!
//! - `parsed_count + skipped_count == total_files` — no silent loss.
//! - Each marker in `WrapperAllowlist::WRAPPER_FILENAMES` is the only
//!   permitted wrapper-without-marker. New wrapper-shaped examples
//!   MUST be added to the allowlist explicitly.
//! - Coverage assertion: every `$wosStudio*` marker the workspace
//!   carries appears at least once across the example corpus, OR is
//!   tracked in `MARKERS_NOT_YET_EXEMPLIFIED`.

use std::path::{Path, PathBuf};

use wos_studio_model::{StudioDocument, StudioMarker, classify};

/// Filenames (relative to `studio/examples/`) that are allowed to
/// carry no `$wosStudio*` marker. These are aggregator wrappers (e.g.
/// `policyObjects: [...]` under a `workspaceId` field) the schemas
/// admit alongside the marker form. Adding a new file here requires
/// a deliberate decision.
const WRAPPER_FILENAMES: &[&str] = &[
    "snap-redetermination-from-sources/wos-workflow.json", // compiled WOS output, not a Studio doc
    "snap-redetermination-from-sources/identity/subjects.json",
    "snap-redetermination-from-sources/bindings/bindings.json",
    "snap-redetermination-from-sources/policy-objects/wos-projecting-kinds.json",
    "snap-redetermination-from-sources/policy-objects/studio-only-kinds.json",
];

/// Markers known to lack an example artifact in the current corpus.
/// Tracked here rather than failing silently. Add a new example
/// covering one of these → remove from this list.
const MARKERS_NOT_YET_EXEMPLIFIED: &[StudioMarker] =
    &[StudioMarker::Binding, StudioMarker::IdentitySubject, StudioMarker::PolicyObject];

#[test]
fn every_marked_studio_example_parses_through_studio_document() {
    let workspace = workspace_root();
    let examples_root = workspace.join("examples");

    let all_files = collect_json_files(&examples_root);
    let total_files = all_files.len();
    assert!(total_files > 0, "no example files found under {examples_root:?}");

    let mut parsed_count = 0usize;
    let mut wrapper_count = 0usize;
    let mut malformed: Vec<String> = Vec::new();
    let mut markers_seen: std::collections::HashSet<StudioMarker> =
        std::collections::HashSet::new();

    for path in &all_files {
        let rel = path
            .strip_prefix(&examples_root)
            .unwrap_or(path)
            .display()
            .to_string()
            .replace('\\', "/");
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                malformed.push(format!("{rel}: {e}"));
                continue;
            }
        };

        match classify(&value) {
            Some(marker) => {
                markers_seen.insert(marker);
                if let Err(e) = serde_json::from_value::<StudioDocument>(value.clone()) {
                    panic!("failed to parse {rel} as StudioDocument: {e}");
                }
                parsed_count += 1;
            }
            None => {
                // Wrapper-without-marker. Permitted only when the file
                // is on the explicit allowlist; otherwise fail.
                if !WRAPPER_FILENAMES.contains(&rel.as_str()) {
                    panic!(
                        "example file {rel} carries no $wosStudio* marker AND \
                         is not on the wrapper allowlist. Either add a marker \
                         to the file or add the path to WRAPPER_FILENAMES \
                         with a justification."
                    );
                }
                wrapper_count += 1;
            }
        }
    }

    // Strictness: every file accounted for; no silent loss.
    assert_eq!(
        parsed_count + wrapper_count + malformed.len(),
        total_files,
        "file accounting drift: parsed={parsed_count} wrapper={wrapper_count} \
         malformed={} total={total_files}",
        malformed.len(),
    );
    assert!(malformed.is_empty(), "malformed JSON: {malformed:?}");
    assert!(parsed_count > 0, "no marked Studio examples parsed");

    // Coverage: every marker appears at least once unless tracked as
    // not-yet-exemplified.
    let exempted: std::collections::HashSet<StudioMarker> =
        MARKERS_NOT_YET_EXEMPLIFIED.iter().copied().collect();
    let all_markers = [
        StudioMarker::Approval,
        StudioMarker::Binding,
        StudioMarker::Effectiveness,
        StudioMarker::IdentitySubject,
        StudioMarker::Mapping,
        StudioMarker::MigrationPath,
        StudioMarker::PolicyObject,
        StudioMarker::Provenance,
        StudioMarker::Readiness,
        StudioMarker::Scenario,
        StudioMarker::Source,
        StudioMarker::TerminologyMap,
        StudioMarker::WorkflowIntent,
        StudioMarker::Workspace,
    ];
    let mut uncovered: Vec<StudioMarker> = Vec::new();
    for m in all_markers {
        if !markers_seen.contains(&m) && !exempted.contains(&m) {
            uncovered.push(m);
        }
    }
    assert!(
        uncovered.is_empty(),
        "markers without an example: {uncovered:?} — author one or add to \
         MARKERS_NOT_YET_EXEMPLIFIED",
    );
}

fn collect_json_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "json")
        {
            out.push(path);
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = studio/crates/wos-studio-model
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("studio workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}
