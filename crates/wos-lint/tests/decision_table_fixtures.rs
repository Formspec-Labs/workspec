// Rust guideline compliant 2026-02-21

//! K-051 / K-052 / K-053 conformance fixture tests.
//!
//! Loads each `crates/wos-conformance/fixtures/K-05[123]-*.json` fixture,
//! deserializes its `inline_documents.kernel` as a `KernelDocument`, runs
//! the decision-table lint rules, and asserts that the diagnostic stream
//! matches `expected_errors[]` substring-style.
//!
//! This test target is the Wave-1' deliverable from the corrected Stage 4
//! plan (`thoughts/plans/2026-05-01-stage-4-decision-table-lint-rules.md`).
//! The conformance harness in `crates/wos-conformance` does NOT auto-
//! discover lint rules; this file provides the missing wiring.
//!
//! Fixtures live at the repo's `crates/wos-conformance/fixtures/` so they
//! can also be exercised by the conformance engine (when it grows
//! lint-dispatch — currently runtime-only). The sibling location is
//! intentional: one source of truth, two consumers.

use std::path::PathBuf;

use wos_core::model::kernel::KernelDocument;
use wos_lint::LintDiagnostic;

#[derive(Debug, serde::Deserialize)]
struct Fixture {
    id: String,
    rule: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    inline_documents: InlineDocuments,
    #[serde(default)]
    expected_errors: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct InlineDocuments {
    #[serde(default)]
    kernel: Option<serde_json::Value>,
}

fn workspace_root() -> PathBuf {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above crates/wos-lint")
        .to_path_buf();

    let cwd = std::env::current_dir().ok();
    for candidate in [Some(manifest_root), cwd].into_iter().flatten() {
        for ancestor in candidate.ancestors() {
            if ancestor.join("fixtures").is_dir()
                && ancestor.join("schemas/wos-workflow.schema.json").is_file()
            {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("could not resolve workspace root with fixtures/ and schemas/");
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("crates/wos-conformance/fixtures")
}

fn load_fixtures(rule_prefix: &str) -> Vec<Fixture> {
    let dir = fixtures_dir();
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!("failed to read fixtures dir {dir:?}: {e}");
    });
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with(rule_prefix) || !name.ends_with(".json") {
            continue;
        }
        let json = std::fs::read_to_string(&path).expect("read fixture");
        let fx: Fixture =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse fixture {name}: {e}"));
        out.push(fx);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn run_lint_against_fixture(fx: &Fixture) -> Vec<LintDiagnostic> {
    let kernel_value = fx
        .inline_documents
        .kernel
        .as_ref()
        .unwrap_or_else(|| panic!("fixture {} has no inline_documents.kernel", fx.id));
    let kernel: KernelDocument = serde_json::from_value(kernel_value.clone())
        .unwrap_or_else(|e| panic!("fixture {}: deserialize KernelDocument: {e}", fx.id));
    let mut diagnostics = Vec::new();
    wos_lint::rules::decision_table::check(&kernel, &mut diagnostics);
    diagnostics
}

fn assert_fixture_matches(fx: &Fixture, diags: &[LintDiagnostic]) {
    if fx.expected_errors.is_empty() {
        // Positive fixture: must produce 0 diagnostics for the targeted rule.
        let rule_diags: Vec<&LintDiagnostic> =
            diags.iter().filter(|d| d.rule_id == fx.rule).collect();
        assert!(
            rule_diags.is_empty(),
            "positive fixture {} (rule {}) expected 0 diagnostics, got {}: {:?}",
            fx.id,
            fx.rule,
            rule_diags.len(),
            rule_diags.iter().map(|d| &d.message).collect::<Vec<_>>(),
        );
    } else {
        // Negative fixture: each expected_errors substring must appear in
        // some diagnostic's message.
        for expected in &fx.expected_errors {
            let found = diags.iter().any(|d| d.message.contains(expected.as_str()));
            assert!(
                found,
                "fixture {} (rule {}): expected_errors substring not found in diagnostics.\n  expected substring: {:?}\n  actual diagnostics: {:#?}",
                fx.id,
                fx.rule,
                expected,
                diags
                    .iter()
                    .map(|d| (&d.rule_id, &d.message))
                    .collect::<Vec<_>>(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// K-051 fixtures
// ---------------------------------------------------------------------------

#[test]
fn k051_fixtures_match_expected_diagnostics() {
    let fixtures = load_fixtures("K-051");
    assert!(!fixtures.is_empty(), "no K-051 fixtures found");
    for fx in &fixtures {
        let diags = run_lint_against_fixture(fx);
        assert_fixture_matches(fx, &diags);
    }
}

// ---------------------------------------------------------------------------
// K-052 fixtures
// ---------------------------------------------------------------------------

#[test]
fn k052_fixtures_match_expected_diagnostics() {
    let fixtures = load_fixtures("K-052");
    assert!(!fixtures.is_empty(), "no K-052 fixtures found");
    for fx in &fixtures {
        let diags = run_lint_against_fixture(fx);
        assert_fixture_matches(fx, &diags);
    }
}

// ---------------------------------------------------------------------------
// K-053 fixtures
// ---------------------------------------------------------------------------

#[test]
fn k053_fixtures_match_expected_diagnostics() {
    let fixtures = load_fixtures("K-053");
    assert!(!fixtures.is_empty(), "no K-053 fixtures found");
    for fx in &fixtures {
        let diags = run_lint_against_fixture(fx);
        assert_fixture_matches(fx, &diags);
    }
}

// ---------------------------------------------------------------------------
// Sanity: every K-05X fixture has a non-empty inline kernel
// ---------------------------------------------------------------------------

#[test]
fn fixtures_well_formed() {
    for rule in &["K-051", "K-052", "K-053"] {
        let fixtures = load_fixtures(rule);
        assert!(!fixtures.is_empty(), "no {rule} fixtures discovered");
        for fx in &fixtures {
            assert_eq!(
                fx.rule, *rule,
                "fixture {} declares rule {}",
                fx.id, fx.rule
            );
            assert!(
                fx.inline_documents.kernel.is_some(),
                "fixture {} missing inline_documents.kernel",
                fx.id
            );
        }
    }
}
