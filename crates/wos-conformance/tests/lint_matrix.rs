// Rust guideline compliant 2026-02-21
//! `LINT-MATRIX.md` is the rendering of the rule registries and must not drift.
//!
//! `make rust-test` (and the stack's `make test`) run this, so a registry or
//! fixture-evidence change without regeneration fails locally and in CI.

use wos_conformance::coverage::{render_matrix, unified_coverage};

#[test]
fn lint_matrix_matches_the_rule_registries() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let expected = render_matrix(&unified_coverage(&root.join("fixtures")));
    let committed = std::fs::read_to_string(root.join("LINT-MATRIX.md")).unwrap();
    let drift = committed
        .lines()
        .zip(expected.lines())
        .position(|(committed, expected)| committed != expected)
        .map(|index| index + 1);
    assert!(
        committed == expected,
        "LINT-MATRIX.md is stale at line {drift:?}; regenerate with \
         `cargo run -p wos-conformance --bin wos-rule-coverage -- --generate-matrix`"
    );
}
