// Rust guideline compliant 2026-02-21
//! Every maintained example workflow lints without errors beside its delivery sidecar.
//!
//! Authors and authoring agents copy the examples (ADR 0076 D-9), and the
//! benefits BLUF claims their cross-references resolve. Each set is one
//! `<stem>.workflow.json` plus its `<stem>.delivery.json` when present, linted
//! as one project the way `lint_project` loads a directory.

use std::path::{Path, PathBuf};
use wos_lint::LintSeverity;

/// Retained historical bytes; `tests/schemas/test_example_workflows.py` names
/// the tests that pin their refusals.
const HISTORICAL: [&str; 2] = [
    "d1-passport.workflow.json",
    "d1-passport-complete-deferred.workflow.json",
];

/// Maintained sets with known Tier 2 errors, pinned as a ratchet by rule id.
///
/// The D1 seed's own test (`wos-conformance/tests/d1_complete_workflow.rs`)
/// runs only Tier 1 (`lint_document`); as a project with its delivery sidecar,
/// Tier 2 reports these. Fixing one fails this test until the entry shrinks.
const KNOWN_ERROR_RULES: [(&str, &[&str]); 1] = [(
    "d1-passport-complete",
    &["G-005", "G-014", "G-015", "G-062"],
)];

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Stems of every maintained `*.workflow.json` in `dir`.
fn maintained_stems(dir: &Path) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".workflow.json") && !HISTORICAL.contains(&name.as_str()))
        .map(|name| name.trim_end_matches(".workflow.json").to_owned())
        .collect();
    stems.sort();
    stems
}

/// Lints one example set and returns its Error diagnostics as (rule, detail).
fn example_errors(dir: &Path, stem: &str) -> Vec<(String, String)> {
    let project = tempfile::tempdir().unwrap();
    for kind in ["workflow", "delivery"] {
        let name = format!("{stem}.{kind}.json");
        if dir.join(&name).is_file() {
            std::fs::copy(dir.join(&name), project.path().join(&name)).unwrap();
        }
    }
    wos_lint::lint_project(project.path())
        .unwrap()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == LintSeverity::Error)
        .map(|diagnostic| {
            (
                diagnostic.rule_id.to_owned(),
                format!("{}: {}", diagnostic.path, diagnostic.message),
            )
        })
        .collect()
}

#[test]
fn maintained_example_sets_lint_without_errors() {
    let dir = examples_dir();
    for name in HISTORICAL {
        assert!(
            dir.join(name).is_file(),
            "historical exemption {name} is stale"
        );
    }
    let stems = maintained_stems(&dir);
    assert!(
        stems.iter().any(|stem| stem == "benefits-adjudication"),
        "{stems:?}"
    );
    for stem in stems {
        let errors = example_errors(&dir, &stem);
        let known = KNOWN_ERROR_RULES
            .iter()
            .find(|(known_stem, _)| *known_stem == stem)
            .map_or(&[][..], |(_, rules)| *rules);
        let mut rules: Vec<&str> = errors.iter().map(|(rule, _)| rule.as_str()).collect();
        rules.sort_unstable();
        rules.dedup();
        assert_eq!(rules, known, "{stem}: {errors:#?}");
    }
}
