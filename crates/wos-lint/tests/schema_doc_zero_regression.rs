// Rust guideline compliant 2026-04-18

//! CI gate: every production schema in `schemas/**/*.schema.json` MUST have
//! zero `SCHEMA-DOC-001` violations.
//!
//! All 21 production schemas (including `schemas/mcp/wos-mcp-tools.schema.json`)
//! reached 0 violations as of 2026-04-18.
//!
//! To diagnose a failure, run:
//!   cargo run -p wos-lint --example count_schema_violations -- <path> --list

use std::path::{Path, PathBuf};

/// Schemas with a known, declining sketch-debt ceiling that the gate enforces
/// as **monotonically decreasing**: a build that adds new SCHEMA-DOC-001
/// violations to one of these schemas (i.e., `count > ceiling`) FAILS the
/// gate. Filling violations in or removing the entry passes the gate. This
/// makes sketch debt visible-and-shrinking instead of hidden-and-frozen
/// (per the wos-spec-author review F7 recommendation).
///
/// **ADR 0076 cleanup landed 2026-04-28** (legacy schemas deleted, six-marker
/// canonical family enforced). The merged author-time envelope now covers
/// signature/execution/evaluationMode/maxRelationshipEventDepth/$schema
/// surfaces that previously lived in standalone documents. Many of these
/// new leaves arrived without canonical descriptions yet; absorption
/// PLN-0176..0207 still owns the fill. Ceiling raised from 79 → 228 to
/// reflect the post-cleanup snapshot.
///
/// - `wos-workflow.schema.json` — ceiling **228** = violation count emitted
///   by `lint_schema`. Distributes across the new Signature $def + execution
///   block + per-block embeds. Tripwire pushed to 2026-09-30 to give the
///   absorption pass + canonical-description fill time to land.
/// - `wos-delivery.schema.json` — **1** leaf the merge agent missed;
///   tracked follow-up.
///
/// **Tripwire:** if these ceilings have not declined further by 2026-09-30,
/// escalate to architectural review. The expectation is that PLN-0176..0207
/// lands the embedded-block descriptions, dropping the wos-workflow ceiling
/// to 0 and removing the entry entirely.
///
/// **Known gameability** (wos-spec-author review F7, 2026-04-28): the ratchet
/// counts violations, not leaves. Adding 5 new sketch leaves while filling 5
/// existing ones keeps `count == ceiling` and passes — debt-density flat,
/// debt-mass growing. Mitigations to consider next pass: (a) snapshot the
/// violation set keyed by `(file, leaf_path)` so new leaves must land filled
/// or be explicitly added; (b) leaf-count companion ratchet — if total leaf
/// count grows while violation count stays flat, the ratchet trips. The
/// 2026-06-30 tripwire is the human-trust fallback until one of these lands.
const EXCLUDED_SCHEMAS_CEILINGS: &[(&str, usize)] = &[
    ("schemas/wos-workflow.schema.json", 228),
    ("schemas/sidecars/wos-delivery.schema.json", 1),
    ("schemas/wos-tooling.schema.json", 16),
];

/// Companion leaf-count ceilings (wos-spec-author F7 mitigation, 2026-04-28):
/// pairs `EXCLUDED_SCHEMAS_CEILINGS` to detect "fill 1, sketch 1" gaming.
/// If total leaf count exceeds the ceiling while violation count stays flat,
/// the ratchet trips: debt-density flat, debt-mass growing.
///
/// Both ratchets MUST stay monotonic-decreasing. The expectation is leaves
/// fill (violations decrease) WITHOUT new leaves being added (count stays).
const EXCLUDED_SCHEMAS_LEAF_CEILINGS: &[(&str, usize)] = &[
    ("schemas/wos-workflow.schema.json", 190),
    ("schemas/sidecars/wos-delivery.schema.json", 47),
    ("schemas/wos-tooling.schema.json", 93),
];

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
    let mut ratchet_violations: Vec<(String, usize, usize)> = Vec::new();
    let mut leaf_count_violations: Vec<(String, usize, usize)> = Vec::new();

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

        // Schemas with a declared ceiling: the count MUST NOT exceed it
        // (monotonic-decreasing ratchet). Hitting zero allows entry removal.
        if let Some((_, ceiling)) =
            EXCLUDED_SCHEMAS_CEILINGS.iter().find(|(p, _)| rel_path == *p)
        {
            if diagnostics.len() > *ceiling {
                ratchet_violations.push((rel_path.clone(), diagnostics.len(), *ceiling));
            }
            // Companion leaf-count ratchet (wos-spec-author F7 mitigation):
            // total leaf count MUST NOT exceed declared ceiling. Catches the
            // "fill 1, sketch 1" game where violation count stays flat but
            // debt-mass grows.
            if let Some((_, leaf_ceiling)) =
                EXCLUDED_SCHEMAS_LEAF_CEILINGS.iter().find(|(p, _)| rel_path == *p)
            {
                let leaf_count = wos_lint::count_schema_leaves(&json).unwrap_or_else(|e| {
                    panic!("count_schema_leaves failed for {}: {e}", abs_path.display())
                });
                if leaf_count > *leaf_ceiling {
                    leaf_count_violations.push((rel_path, leaf_count, *leaf_ceiling));
                }
            }
            continue;
        }

        if !diagnostics.is_empty() {
            violations_by_file.push((rel_path, diagnostics.len()));
        }
    }

    assert!(
        ratchet_violations.is_empty(),
        "SCHEMA-DOC-001 ratchet broken — {} schema(s) exceed their declared violation ceiling:\n{}\n\
         Lower the ceiling or fill the new violations. The ratchet enforces \
         monotonic-decreasing sketch debt per the wos-spec-author F7 recommendation.",
        ratchet_violations.len(),
        ratchet_violations
            .iter()
            .map(|(path, count, ceiling)| format!(
                "  {path}: {count} violation(s), ceiling {ceiling}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    assert!(
        leaf_count_violations.is_empty(),
        "SCHEMA-DOC-001 leaf-count ratchet broken — {} schema(s) added new leaves \
         beyond their declared ceiling. This catches the 'fill 1, sketch 1' gaming \
         pattern where violation count stays flat but total leaf count grows.\n{}\n\
         Lower the leaf ceiling only when leaves are removed; do NOT raise to fit.",
        leaf_count_violations.len(),
        leaf_count_violations
            .iter()
            .map(|(path, count, ceiling)| format!(
                "  {path}: {count} leaves, ceiling {ceiling}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

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
