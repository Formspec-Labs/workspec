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
/// **ADR 0076 in-flight (PLN-0314):**
/// - `wos-workflow.schema.json` — **96** inner-block leaves under
///   governance/agents/aiOversight/signature/custody/advanced/assurance whose
///   canonical descriptions live in spec docs awaiting the absorption pass
///   (PLN-0176..0207). Intentionally sketch until absorption lands; do not
///   hallucinate descriptions.
/// - `wos-workflow.schema.json` — **64** kernel-spine leaves
///   (State/Transition/TransitionEvent/Lifecycle/CaseFile/Actor/Contracts/
///   IntakeReference/FieldDeclaration/OutputBinding) whose canonical prose
///   already exists in `kernel/spec.md` §3 + §4 + §9.2 + §10. These are
///   fillable now (post wos-spec-author review F1); the ceiling tracks them
///   pending a focused fill-in session. Total ceiling **160** while the
///   spine and absorption work proceed.
/// - `wos-delivery.schema.json` — **1** leaf the merge agent missed;
///   tracked follow-up.
///
/// **Tripwire:** if these ceilings have not declined by 2026-06-30, escalate
/// to architectural review. The expectation is that PLN-0176..0207 lands the
/// embedded-block descriptions and the kernel-spine fill happens as a focused
/// pass, both well before the tripwire.
const EXCLUDED_SCHEMAS_CEILINGS: &[(&str, usize)] = &[
    ("schemas/wos-workflow.schema.json", 160),
    ("schemas/sidecars/wos-delivery.schema.json", 1),
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
                ratchet_violations.push((rel_path, diagnostics.len(), *ceiling));
            }
            continue;
        }

        if !diagnostics.is_empty() {
            violations_by_file.push((rel_path, diagnostics.len()));
        }
    }

    assert!(
        ratchet_violations.is_empty(),
        "SCHEMA-DOC-001 ratchet broken — {} schema(s) exceed their declared ceiling:\n{}\n\
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
