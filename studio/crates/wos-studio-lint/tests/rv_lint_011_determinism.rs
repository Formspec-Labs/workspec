// Rust guideline compliant 2026-05-02

//! SA-MUST-rv-011 — Rule evaluation MUST be deterministic.
//!
//! Re-running `lint_workspace` over the same fixture 10× MUST produce
//! the byte-identical diagnostic stream. This catches HashMap /
//! HashSet iteration-order leaks across calls within one process
//! (`HashMap::RandomState` is process-stable, so per-call jitter
//! here would surface as a real bug, not seed drift).
//!
//! Lives as an integration test rather than inline in
//! `workspace_rules.rs::tests` so the burndown wave doesn't co-mingle
//! with lint rule code (LINT territory per studio/CLAUDE.md and the
//! D-wave scoping that opened STUDIO-DEFER-004-FIXTURE).

use std::path::Path;

use wos_studio_lint::{Workspace, lint_workspace};

fn load_workspace(rel_path: &str) -> Workspace {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(rel_path);
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {rel_path}: {e}"));
    let pairs: Vec<(String, serde_json::Value)> = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse fixture {rel_path}: {e}"));
    Workspace::from_iter(pairs.into_iter().map(|(p, v)| (p, v.to_string())))
}

#[test]
fn rv_lint_011_workspace_lint_is_deterministic_across_repeats() {
    let ws = load_workspace("cross_cutting/rv_lint_011_deterministic_evaluation.json");
    let baseline = lint_workspace(&ws);
    // Serialize the diagnostic stream so any field-order or ordering
    // drift surfaces as a string mismatch (not just a structural
    // equality pass).
    let baseline_str = format!("{baseline:?}");
    for i in 1..10 {
        let next = lint_workspace(&ws);
        let next_str = format!("{next:?}");
        assert_eq!(
            next_str, baseline_str,
            "iteration {i}: lint_workspace drift — non-deterministic rule evaluation"
        );
    }
    // Sanity: the fixture is non-empty (otherwise the test is
    // vacuously stable). The fixture intentionally exercises multiple
    // rules so the diagnostic stream is non-trivial.
    assert!(
        !baseline.is_empty(),
        "rv_lint_011 fixture should produce ≥1 diagnostic so determinism \
         is checked over a non-empty stream; got 0"
    );
}
