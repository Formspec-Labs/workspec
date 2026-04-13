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
const AI_REGISTRATION_BATCHES: &[u8] = &[3, 4, 5, 10];
const AI_CONFIDENCE_BATCHES: &[u8] = &[5];

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

fn ai_family_specs() -> Vec<FixtureSpec> {
    load_fixture_specs()
        .into_iter()
        .filter(|spec| {
            ["AI-", "AG-", "AC-"]
                .iter()
                .any(|prefix| spec.fixture.rule.starts_with(prefix))
        })
        .collect()
}

fn assert_ai_family_batches_declared(specs: &[FixtureSpec]) {
    let missing_batches: Vec<&str> = specs
        .iter()
        .filter(|spec| spec.fixture.batch.is_none())
        .map(|spec| spec.file_name.as_str())
        .collect();

    assert!(
        missing_batches.is_empty(),
        "AI-family fixtures missing batch metadata: {}",
        missing_batches.join(", ")
    );
}

fn assert_profile_batches_present(
    profile_name: &str,
    specs: &[FixtureSpec],
    required_batches: &[u8],
) {
    let present_batches: BTreeSet<u8> =
        specs.iter().filter_map(|spec| spec.fixture.batch).collect();
    let missing_batches: Vec<String> = required_batches
        .iter()
        .copied()
        .filter(|batch| !present_batches.contains(batch))
        .map(|batch| batch.to_string())
        .collect();

    assert!(
        missing_batches.is_empty(),
        "profile '{profile_name}' is missing fixture coverage for batches: {}",
        missing_batches.join(", ")
    );
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
    let ai_specs = ai_family_specs();
    assert_ai_family_batches_declared(&ai_specs);
    assert_profile_batches_present("AI Registration", &ai_specs, AI_REGISTRATION_BATCHES);

    assert_profile_passes(
        "AI Registration",
        |spec| {
            spec.fixture
                .batch
                .is_some_and(|batch| AI_REGISTRATION_BATCHES.contains(&batch))
        },
        None,
    );
}

#[test]
fn ai_confidence_framework_profile_passes() {
    let ai_specs = ai_family_specs();
    assert_ai_family_batches_declared(&ai_specs);
    assert_profile_batches_present("AI Confidence Framework", &ai_specs, AI_CONFIDENCE_BATCHES);

    assert_profile_passes(
        "AI Confidence Framework",
        |spec| {
            spec.fixture
                .batch
                .is_some_and(|batch| AI_CONFIDENCE_BATCHES.contains(&batch))
        },
        None,
    );
}
