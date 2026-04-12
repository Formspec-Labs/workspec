// Rust guideline compliant 2026-02-21

//! Aggregate conformance profile tests.
//!
//! These tests map profile-level claims to the existing fixture inventory instead
//! of introducing synthetic profile fixtures. A profile passes only when every
//! constituent fixture passes under the current processor implementation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use wos_conformance::{ConformanceFixture, run_fixture};

const GOVERNANCE_BASIC_RULES: &[&str] = &[
    "G-002", "G-006", "G-007", "G-010", "G-016", "G-017", "G-018",
];

const AI_REGISTRATION_RULES: &[&str] = &[
    "AI-005", "AI-006", "AI-008", "AI-009", "AI-010", "AI-011", "AI-012", "AI-013", "AI-014",
    "AI-015", "AI-016", "AI-017", "AI-019", "AI-021", "AI-022", "AI-025", "AI-027", "AI-028",
    "AI-029", "AI-030", "AI-032", "AI-033", "AI-034", "AI-035", "AI-036", "AI-037", "AI-038",
    "AI-039", "AI-040", "AI-044", "AI-047", "AI-051", "AI-052", "AI-053", "AI-054", "AI-055",
    "AI-057", "AC-001", "AC-002", "AG-004", "AG-005", "AG-006", "AG-007", "AG-009", "AG-016",
];

const AI_REGISTRATION_EXCLUDED_RULES: &[&str] =
    &["AG-001", "AG-002", "AG-003", "AG-015", "AI-045", "AI-048"];

const AI_CONFIDENCE_RULES: &[&str] = &[
    "AI-034", "AI-035", "AI-036", "AI-037", "AI-038", "AG-004", "AG-016",
];

const AI_CONFIDENCE_EXCLUDED_RULES: &[&str] = &[
    "AI-005", "AI-006", "AI-008", "AI-009", "AI-010", "AI-011", "AI-012", "AI-013", "AI-014",
    "AI-015", "AI-016", "AI-017", "AI-019", "AI-021", "AI-022", "AI-025", "AI-027", "AI-028",
    "AI-029", "AI-030", "AI-032", "AI-033", "AI-039", "AI-040", "AI-044", "AI-047", "AI-051",
    "AI-052", "AI-053", "AI-054", "AI-055", "AI-057", "AC-001", "AC-002", "AG-005", "AG-006",
    "AG-007", "AG-009",
];

#[derive(Debug)]
struct FixtureSpec {
    file_name: String,
    base_dir: String,
    fixture: ConformanceFixture,
    json: String,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_fixture_specs() -> Vec<FixtureSpec> {
    let mut fixture_paths: Vec<PathBuf> = std::fs::read_dir(fixtures_dir())
        .expect("fixtures dir exists")
        .map(|entry| entry.expect("readable fixture entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    fixture_paths.sort();

    fixture_paths
        .into_iter()
        .map(|path| {
            let json = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read fixture '{}': {error}", path.display()));
            let fixture: ConformanceFixture = serde_json::from_str(&json)
                .unwrap_or_else(|error| panic!("parse fixture '{}': {error}", path.display()));

            FixtureSpec {
                file_name: path
                    .file_name()
                    .expect("fixture file name exists")
                    .to_string_lossy()
                    .into_owned(),
                base_dir: path
                    .parent()
                    .expect("fixture has parent directory")
                    .to_string_lossy()
                    .into_owned(),
                fixture,
                json,
            }
        })
        .collect()
}

fn assert_required_rules_present(
    profile_name: &str,
    matched_rule_counts: &BTreeMap<String, usize>,
    required_rules: &[&str],
) {
    let missing_rules: Vec<&str> = required_rules
        .iter()
        .copied()
        .filter(|rule_id| !matched_rule_counts.contains_key(*rule_id))
        .collect();

    assert!(
        missing_rules.is_empty(),
        "profile '{profile_name}' is missing fixture coverage for rules: {}",
        missing_rules.join(", ")
    );
}

fn assert_rule_partition(
    partition_name: &str,
    universe_rules: &BTreeSet<String>,
    included_rules: &[&str],
    excluded_rules: &[&str],
) {
    let included: BTreeSet<String> = included_rules
        .iter()
        .map(|rule| (*rule).to_string())
        .collect();
    let excluded: BTreeSet<String> = excluded_rules
        .iter()
        .map(|rule| (*rule).to_string())
        .collect();

    let overlap: Vec<&String> = included.intersection(&excluded).collect();
    assert!(
        overlap.is_empty(),
        "partition '{partition_name}' overlaps between include/exclude sets: {}",
        overlap
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let covered: BTreeSet<String> = included.union(&excluded).cloned().collect();
    let missing: Vec<&String> = universe_rules.difference(&covered).collect();
    let unknown: Vec<&String> = covered.difference(universe_rules).collect();

    assert!(
        missing.is_empty(),
        "partition '{partition_name}' is missing rules: {}",
        missing
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        unknown.is_empty(),
        "partition '{partition_name}' references unknown rules: {}",
        unknown
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn fixture_rules_with_prefixes(prefixes: &[&str]) -> BTreeSet<String> {
    load_fixture_specs()
        .into_iter()
        .map(|spec| spec.fixture.rule)
        .filter(|rule| prefixes.iter().any(|prefix| rule.starts_with(prefix)))
        .collect()
}

fn assert_profile_passes<F>(profile_name: &str, matches_profile: F, required_rules: Option<&[&str]>)
where
    F: Fn(&FixtureSpec) -> bool,
{
    let selected_fixtures: Vec<FixtureSpec> = load_fixture_specs()
        .into_iter()
        .filter(matches_profile)
        .collect();

    assert!(
        !selected_fixtures.is_empty(),
        "profile '{profile_name}' did not match any fixtures"
    );

    let mut matched_rule_counts = BTreeMap::new();
    for spec in &selected_fixtures {
        *matched_rule_counts
            .entry(spec.fixture.rule.clone())
            .or_insert(0usize) += 1;
    }

    if let Some(required_rules) = required_rules {
        assert_required_rules_present(profile_name, &matched_rule_counts, required_rules);
    }

    let mut failures = Vec::new();

    for spec in selected_fixtures {
        match run_fixture(&spec.json, &spec.base_dir) {
            Ok(result) if result.passed => {}
            Ok(result) => failures.push(format!(
                "{} [{}]:\n{}",
                spec.file_name,
                spec.fixture.rule,
                result.failures.join("\n")
            )),
            Err(error) => failures.push(format!(
                "{} [{}]: engine error: {error}",
                spec.file_name, spec.fixture.rule
            )),
        }
    }

    let matched_rules: BTreeSet<&str> = matched_rule_counts.keys().map(String::as_str).collect();

    assert!(
        failures.is_empty(),
        "profile '{profile_name}' failed across {} fixtures covering {} rules ({}):\n{}",
        matched_rule_counts.values().sum::<usize>(),
        matched_rules.len(),
        matched_rules.into_iter().collect::<Vec<_>>().join(", "),
        failures.join("\n\n")
    );
}

#[test]
fn governance_basic_profile_passes() {
    assert_profile_passes(
        "Governance Basic",
        |spec| GOVERNANCE_BASIC_RULES.contains(&spec.fixture.rule.as_str()),
        Some(GOVERNANCE_BASIC_RULES),
    );
}

#[test]
fn governance_complete_profile_passes() {
    assert_profile_passes(
        "Governance Complete",
        |spec| spec.fixture.rule.starts_with("G-"),
        None,
    );
}

#[test]
fn ai_registration_profile_passes() {
    let agent_rule_universe = fixture_rules_with_prefixes(&["AI-", "AG-", "AC-"]);
    assert_rule_partition(
        "AI Registration",
        &agent_rule_universe,
        AI_REGISTRATION_RULES,
        AI_REGISTRATION_EXCLUDED_RULES,
    );

    assert_profile_passes(
        "AI Registration",
        |spec| AI_REGISTRATION_RULES.contains(&spec.fixture.rule.as_str()),
        Some(AI_REGISTRATION_RULES),
    );
}

#[test]
fn ai_confidence_framework_profile_passes() {
    let ai_registration_universe: BTreeSet<String> = AI_REGISTRATION_RULES
        .iter()
        .map(|rule| (*rule).to_string())
        .collect();
    assert_rule_partition(
        "AI Confidence Framework",
        &ai_registration_universe,
        AI_CONFIDENCE_RULES,
        AI_CONFIDENCE_EXCLUDED_RULES,
    );

    assert_profile_passes(
        "AI Confidence Framework",
        |spec| AI_CONFIDENCE_RULES.contains(&spec.fixture.rule.as_str()),
        Some(AI_CONFIDENCE_RULES),
    );
}
