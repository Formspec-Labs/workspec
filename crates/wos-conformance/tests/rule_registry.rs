// Rust guideline compliant 2026-02-21

//! T3 conformance-registry invariants: every reified T3 rule appears, and
//! rules promoted past `Draft` carry at least one fixture link. The Task 1
//! bootstrap test (every rule at Draft with empty fixtures) is superseded
//! by Task 2, which backfills fixture links for rules with real evidence.
//!
//! Task 3 of §4.2 adds the rule-coverage ratchet: every `Tested` /
//! `LoadBearing` promotion MUST either point at an executable fixture that
//! the conformance harness actually runs, or carry an explicit evidence
//! annotation explaining the indirection (mirrors the AI-004 / AI-050 /
//! K-EXT-002 pattern — the 2026-04-18 review evidence-quality warnings).
//!
//! Task 6 of §4.2 adds the LoadBearing gate: ≥2 executable fixtures required.
//!
//! Task 7 of §4.2 adds promotion-candidate discovery: surfaces Draft rules
//! that already have discoverable fixture evidence. Does NOT fail the suite —
//! writes a report to `target/rule-coverage-promotion-candidates.txt`.

use std::path::{Path, PathBuf};

use wos_conformance::{
    coverage::{EvidenceMatchKind, compute_coverage},
    rules::all_rules,
};
use wos_lint::{Graduation, RuleMetadata, all_lint_rules};

#[test]
fn all_conformance_rules_registry_is_non_empty() {
    assert!(
        !all_rules().is_empty(),
        "wos-conformance rule registry must list every implemented T3 rule"
    );
}

#[test]
fn every_non_draft_conformance_rule_has_at_least_one_fixture() {
    let mut violations: Vec<&str> = Vec::new();
    for rule in all_rules() {
        let is_draft = matches!(rule.graduation, Graduation::Draft);
        // Empty-fixture promotions are allowed only when the registry
        // carries an explicit inline-evidence annotation.
        if !is_draft
            && rule.fixtures.is_empty()
            && !has_evidence_annotation(rule.id, &conformance_registry_src())
        {
            violations.push(rule.id);
        }
    }
    assert!(
        violations.is_empty(),
        "conformance rules promoted past Draft but missing fixture links \
         and missing inline-evidence annotation: {:?}",
        violations
    );
}

#[test]
fn draft_conformance_rules_have_empty_fixtures() {
    for rule in all_rules() {
        if matches!(rule.graduation, Graduation::Draft) {
            assert!(
                rule.fixtures.is_empty(),
                "Draft conformance rule {} must not have fixture links until promoted",
                rule.id
            );
        }
    }
}

// ── Rule-coverage ratchet (§4.2 Task 3) ─────────────────────────────────────

/// Every `Tested` / `LoadBearing` rule MUST satisfy at least one of:
///
/// 1. An *executable* fixture — the listed path resolves to a real `.json`
///    file under one of the runner-scanned fixture directories AND is wired
///    into a `tests/*.rs` harness (not just referenced in a doc comment).
///
/// 2. An explicit registry *evidence annotation* — a `//` comment block
///    immediately preceding the `RuleMetadata` literal that names the
///    indirection (keyword `inline`, `inline-evidence`, `indirect`,
///    `indirection`, or a concrete test-function identifier containing `_`).
///
/// This test locks the ratchet: a rule cannot be promoted past `Draft`
/// without evidence the harness can point at.
#[test]
fn every_promoted_conformance_rule_has_executable_or_annotated_evidence() {
    let registry_src = conformance_registry_src();
    let test_source_index = collect_test_source_text();
    let workspace = workspace_root();

    let mut violations: Vec<String> = Vec::new();
    for rule in all_rules() {
        if !is_promoted(rule) {
            continue;
        }
        if rule_has_executable_fixture(rule, &workspace, &test_source_index) {
            continue;
        }
        if has_evidence_annotation(rule.id, &registry_src) {
            continue;
        }
        violations.push(describe_gap(rule, &workspace));
    }

    assert!(
        violations.is_empty(),
        "conformance ratchet violations — rule promoted past Draft without \
         executable fixture or evidence annotation:\n  {}",
        violations.join("\n  ")
    );
}

fn is_promoted(rule: &RuleMetadata) -> bool {
    matches!(
        rule.graduation,
        Graduation::Tested | Graduation::Stable | Graduation::LoadBearing
    )
}

// ── LoadBearing promotion gate (§4.2 Task 6) ────────────────────────────────

/// A rule promoted to `LoadBearing` MUST have ≥2 executable fixtures.
///
/// `LoadBearing` rules are production-critical and require multi-fixture
/// coverage. A single fixture is insufficient — it means one breakage scenario
/// covers what should be a broad foundation. Without this gate, marking a rule
/// `LoadBearing` is just a rename with no coverage improvement.
///
/// For each `LoadBearing` rule the test counts resolvable `fixture_links` —
/// paths that point to real `.json` files under the workspace. Annotation-only
/// rules (empty fixtures slice) never qualify for `LoadBearing` and will fail
/// with a clear "0 of ≥2 required" message.
#[test]
fn every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures() {
    let workspace = workspace_root();
    let mut violations: Vec<String> = Vec::new();

    for rule in all_rules() {
        if !matches!(rule.graduation, Graduation::LoadBearing) {
            continue;
        }
        let executable_count = rule
            .fixtures
            .iter()
            .filter(|path| workspace.join(path).is_file())
            .count();
        if executable_count < 2 {
            violations.push(format!(
                "{}: LoadBearing but only {executable_count} of ≥2 required executable fixtures; \
                 listed fixtures: [{}]",
                rule.id,
                rule.fixtures.join(", "),
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "LoadBearing promotion gate — rules must have ≥2 executable fixtures:\n  {}",
        violations.join("\n  ")
    );
}

fn rule_has_executable_fixture(
    rule: &RuleMetadata,
    workspace: &Path,
    test_source_index: &str,
) -> bool {
    rule.fixtures
        .iter()
        .any(|path| fixture_is_executable(path, workspace, test_source_index))
}

fn fixture_is_executable(rel_path: &str, workspace: &Path, test_source_index: &str) -> bool {
    let absolute = workspace.join(rel_path);
    if !absolute.is_file() {
        return false;
    }
    // `tests/fixtures/` is auto-scanned by `load_fixture_specs`, so any
    // `.json` there participates in the processor/profile conformance run.
    if rel_path.starts_with("crates/wos-conformance/tests/fixtures/") {
        return true;
    }
    // `crates/wos-conformance/fixtures/` is hand-enumerated by test files
    // (notably `trace_parity.rs`). Accept it only if the filename appears
    // in a test source file outside of `//`-comment lines.
    let Some(filename) = absolute.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    filename_referenced_in_code(filename, test_source_index)
}

fn filename_referenced_in_code(filename: &str, test_source_index: &str) -> bool {
    test_source_index
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .any(|line| line.contains(filename))
}

fn has_evidence_annotation(rule_id: &str, registry_src: &str) -> bool {
    // Locate the `id: "<rule_id>"` anchor, walk upward past the
    // `RuleMetadata {` opener line, then collect the contiguous `//`
    // comment block immediately above and scan it for annotation keywords.
    let needle = format!("id: \"{rule_id}\"");
    let Some(anchor) = registry_src.find(&needle) else {
        return false;
    };
    let preamble_lines: Vec<&str> = registry_src[..anchor].lines().collect();
    let mut iter = preamble_lines.iter().rev().peekable();
    // Skip anything between the `id:` anchor and the comment block above —
    // the `RuleMetadata {` opener line, any blank lines, indentation, etc.
    while let Some(line) = iter.peek() {
        if line.trim_start().starts_with("//") {
            break;
        }
        iter.next();
    }
    let mut comment_lines: Vec<&str> = Vec::new();
    for line in iter {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.is_empty() {
            comment_lines.push(line);
        } else {
            break;
        }
    }
    comment_lines.reverse();
    let comment_block = comment_lines.join("\n").to_lowercase();

    const KEYWORDS: &[&str] = &["inline", "indirect", "indirection"];
    if KEYWORDS.iter().any(|kw| comment_block.contains(kw)) {
        return true;
    }
    // Test-function identifier heuristic: any `snake_case`-shaped word
    // (contains `_`) strongly suggests a named inline test function.
    comment_block
        .split_whitespace()
        .any(|word| word.contains('_') && word.chars().any(|c| c.is_ascii_alphabetic()))
}

fn describe_gap(rule: &RuleMetadata, workspace: &Path) -> String {
    if rule.fixtures.is_empty() {
        return format!(
            "{}: promoted to {:?} with empty fixtures and no evidence annotation",
            rule.id, rule.graduation
        );
    }
    let details: Vec<String> = rule
        .fixtures
        .iter()
        .map(|path| {
            let exists = workspace.join(path).is_file();
            format!("{path} (exists={exists})")
        })
        .collect();
    format!(
        "{} ({:?}): no fixture is executable by the harness; listed: [{}]",
        rule.id,
        rule.graduation,
        details.join(", ")
    )
}

fn conformance_registry_src() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "could not read conformance registry {}: {e}",
            path.display()
        )
    })
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

/// Concatenate every `crates/*/tests/**/*.rs` file into one string so we can
/// ask "is this fixture filename referenced in test code?" without pulling
/// in a recursive-walk crate. Manifest-local; only runs at test time.
fn collect_test_source_text() -> String {
    let mut buffer = String::new();
    let crates_dir = workspace_root().join("crates");
    let Ok(entries) = std::fs::read_dir(&crates_dir) else {
        return buffer;
    };
    for crate_entry in entries.flatten() {
        let tests_dir = crate_entry.path().join("tests");
        if !tests_dir.is_dir() {
            continue;
        }
        append_rs_files(&tests_dir, &mut buffer);
    }
    buffer
}

fn append_rs_files(dir: &Path, buffer: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            append_rs_files(&path, buffer);
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(contents) = std::fs::read_to_string(&path)
        {
            buffer.push_str(&contents);
            buffer.push('\n');
        }
    }
}

// ── Promotion-candidate discovery (§4.2 Task 7) ─────────────────────────────

/// Report Draft rules that have discoverable fixture evidence — but do NOT fail.
///
/// Runs `compute_coverage` over both registries and writes promotion candidates
/// to `target/rule-coverage-promotion-candidates.txt`. When run with `--nocapture`
/// the candidates also print to stdout so CI job logs surface them directly.
///
/// This test intentionally does not assert — it is purely a data-collection
/// step. CI wires the output file into a PR annotation (see
/// `.github/workflows/rule-coverage.yml`). The failing gates are in
/// `every_promoted_conformance_rule_has_executable_or_annotated_evidence`
/// (Tested/Stable) and `every_load_bearing_conformance_rule_has_at_least_two_executable_fixtures`
/// (LoadBearing).
#[test]
fn discover_and_report_promotion_candidates() {
    let registries: &[&'static [wos_lint::RuleMetadata]] = &[all_lint_rules(), all_rules()];
    let fixtures_dir = workspace_root().join("fixtures");
    let report = compute_coverage(registries, Some(&fixtures_dir));

    if report.promotion_candidates.is_empty() {
        println!("promotion-candidate discovery: no Draft rules with discoverable fixtures");
        return;
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "# Promotion candidates — {} Draft rule(s) with discoverable fixture evidence",
        report.promotion_candidates.len()
    ));
    lines.push(String::new());
    lines.push(
        "Promote these rules to `Tested` in their registry entry once evidence is confirmed."
            .to_string(),
    );
    lines.push(String::new());

    for candidate in &report.promotion_candidates {
        for ev in &candidate.evidence {
            let kind_label = match ev.match_kind {
                EvidenceMatchKind::FilenameStem => "filename-stem",
                EvidenceMatchKind::RuleField => "rule-field",
            };
            lines.push(format!(
                "  {} → {} [{}]",
                candidate.rule_id, ev.fixture_path, kind_label
            ));
        }
    }

    let report_text = lines.join("\n") + "\n";

    // Print to stdout so `cargo nextest run --no-capture` surfaces it in CI logs.
    print!("{}", report_text);

    // Write to target/ so CI can read the file as an artifact.
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|ws| ws.join("target"))
        .unwrap_or_else(|| PathBuf::from("target"));
    let out_path = target_dir.join("rule-coverage-promotion-candidates.txt");
    // Ignore write errors — the test must never fail due to filesystem issues.
    if let Ok(()) = std::fs::create_dir_all(&target_dir) {
        let _ = std::fs::write(&out_path, &report_text);
    }
    // No assert: this test is purely observational.
}
