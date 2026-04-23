// Rust guideline compliant 2026-02-21

//! Registry invariants: every implemented lint rule appears, and rules
//! promoted past `Draft` carry at least one fixture link. The Task 1
//! bootstrap test (every rule at Draft with empty fixtures) is superseded
//! by Task 2, which backfills fixture links for rules with real evidence.
//!
//! Task 3 of §4.2 adds the rule-coverage ratchet: every `Tested` /
//! `LoadBearing` promotion MUST either point at an executable fixture that
//! the test harness actually runs, or carry an explicit evidence annotation
//! explaining the indirection (mirrors the AI-004 / AI-050 / K-EXT-002
//! pattern — the 2026-04-18 review evidence-quality warnings).

use std::path::{Path, PathBuf};

use wos_lint::{Graduation, RuleMetadata, all_lint_rules};

#[test]
fn all_lint_rules_registry_is_non_empty() {
    assert!(
        !all_lint_rules().is_empty(),
        "wos-lint rule registry must list every implemented rule"
    );
}

#[test]
fn every_non_draft_rule_has_at_least_one_fixture() {
    let mut violations: Vec<&str> = Vec::new();
    for rule in all_lint_rules() {
        let is_draft = matches!(rule.graduation, Graduation::Draft);
        if !is_draft
            && rule.fixtures.is_empty()
            && !has_evidence_annotation(rule.id, &lint_registry_src())
        {
            violations.push(rule.id);
        }
    }
    assert!(
        violations.is_empty(),
        "rules promoted past Draft but missing fixture links and missing \
         inline-evidence annotation: {:?}",
        violations
    );
}

#[test]
fn draft_rules_have_empty_fixtures() {
    for rule in all_lint_rules() {
        if matches!(rule.graduation, Graduation::Draft) {
            assert!(
                rule.fixtures.is_empty(),
                "Draft rule {} must not have fixture links until promoted",
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
fn every_promoted_rule_has_executable_or_annotated_evidence() {
    let registry_src = lint_registry_src();
    let test_source_index = collect_test_source_text();
    let workspace = workspace_root();

    let mut violations: Vec<String> = Vec::new();
    for rule in all_lint_rules() {
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
        "lint ratchet violations — rule promoted past Draft without \
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
fn every_load_bearing_lint_rule_has_at_least_two_executable_fixtures() {
    let workspace = workspace_root();
    let mut violations: Vec<String> = Vec::new();

    for rule in all_lint_rules() {
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
    // `crates/wos-conformance/tests/fixtures/` is auto-scanned by the
    // `load_fixture_specs` dir-walk in `wos-conformance`, so any `.json`
    // there participates in processor/profile conformance runs.
    if rel_path.starts_with("crates/wos-conformance/tests/fixtures/") {
        return true;
    }
    // Other fixture locations (notably `crates/wos-conformance/fixtures/`)
    // are hand-enumerated by test files. Accept them only when the filename
    // appears in a test source file outside of `//`-comment lines.
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
    let needle = format!("id: \"{rule_id}\"");
    let Some(anchor) = registry_src.find(&needle) else {
        return false;
    };
    // Walk upward from the `id: "..."` line, skip any lines that are not
    // leading `//` comments (the surrounding `RuleMetadata {` opener, etc.),
    // then collect the contiguous comment block above that.
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

fn lint_registry_src() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules/registry.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read lint registry {}: {e}", path.display()))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

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
