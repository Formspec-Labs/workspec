// Rust guideline compliant 2026-02-21
//! Every maintained example workflow lints without errors beside its delivery sidecar.
//!
//! Authors and authoring agents copy the examples (ADR 0076 D-9), and the
//! benefits BLUF claims their cross-references resolve. Each set is one
//! `<stem>.workflow.json` plus its `<stem>.delivery.json` when present, linted
//! as one project the way `lint_project` loads a directory.

use std::path::{Path, PathBuf};
use wos_lint::LintSeverity;

/// Maintained sets with known Tier 2 errors, pinned as a ratchet by (rule, path).
///
/// fs-a6om item 1: the D1 0.3.0 example lacks counterfactualRequired and the
/// audit tiers, and its denial templates lack appealInstructions. Its own test
/// (`wos-conformance/tests/d1_complete_workflow.rs`) runs only Tier 1; as a
/// project with its delivery sidecar, Tier 2 reports these. A new error, even
/// of a listed rule, or a fixed one fails this test until the entry changes.
const KNOWN_ERRORS: [(&str, &[(&str, &str)]); 1] = [(
    "d1-passport-complete",
    &[
        (
            "G-005",
            "/governance/dueProcess/adverseDecisionPolicy/counterfactualRequired",
        ),
        ("G-014", "/governance/audit/reasoningTier"),
        ("G-015", "/governance/audit/counterfactualTier"),
        ("G-062", "/notifications/templates/d1DeniedEnUS"),
        ("G-062", "/notifications/templates/d1DeniedEsUS"),
        ("G-062", "/notifications/templates/d1NoResponseDeniedEnUS"),
        ("G-062", "/notifications/templates/d1NoResponseDeniedEsUS"),
    ],
)];

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Historical examples, exempt by name; the shared list
/// `examples/historical-examples.json` names the test that pins each refusal.
fn historical(dir: &Path) -> Vec<String> {
    let list: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("historical-examples.json")).unwrap(),
    )
    .unwrap();
    list["historicalExamples"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// Stems of every maintained `*.workflow.json` in `dir`.
fn maintained_stems(dir: &Path, historical: &[String]) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".workflow.json") && !historical.contains(name))
        .map(|name| name.trim_end_matches(".workflow.json").to_owned())
        .collect();
    stems.sort();
    stems
}

/// Lints one example set and returns its Error diagnostics as sorted (rule, path).
fn example_errors(dir: &Path, stem: &str) -> Vec<(String, String)> {
    let project = tempfile::tempdir().unwrap();
    for kind in ["workflow", "delivery"] {
        let name = format!("{stem}.{kind}.json");
        if dir.join(&name).is_file() {
            std::fs::copy(dir.join(&name), project.path().join(&name)).unwrap();
        }
    }
    let mut errors: Vec<(String, String)> = wos_lint::lint_project(project.path())
        .unwrap()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == LintSeverity::Error)
        .map(|diagnostic| (diagnostic.rule_id.to_owned(), diagnostic.path))
        .collect();
    errors.sort();
    errors
}

#[test]
fn maintained_example_sets_lint_without_errors() {
    let dir = examples_dir();
    let historical = historical(&dir);
    for name in &historical {
        assert!(
            dir.join(name).is_file(),
            "historical exemption {name} is stale"
        );
    }
    let stems = maintained_stems(&dir, &historical);
    assert!(
        stems.iter().any(|stem| stem == "benefits-adjudication"),
        "{stems:?}"
    );
    for stem in stems {
        let known: Vec<(String, String)> = KNOWN_ERRORS
            .iter()
            .find(|(known_stem, _)| *known_stem == stem)
            .map_or(&[][..], |(_, errors)| *errors)
            .iter()
            .map(|(rule, path)| ((*rule).to_owned(), (*path).to_owned()))
            .collect();
        assert_eq!(example_errors(&dir, &stem), known, "{stem}");
    }
}
