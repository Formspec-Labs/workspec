// Rust guideline compliant 2026-02-21

//! Processor-level conformance claim verification.
//!
//! Batch 16 meta-rules are claims about the processor as a whole rather than
//! about a single fixture. This module verifies those claims by combining:
//!
//! - aggregate profile execution over the fixture inventory,
//! - explicit processor claims, and
//! - architectural evidence for delegation and proxy boundaries.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wos_core::traits::ContractValidator;

use crate::{ConformanceError, ConformanceFixture, run_fixture};

pub const GOVERNANCE_BASIC_RULES: &[&str] = &[
    "G-002", "G-006", "G-007", "G-010", "G-016", "G-017", "G-018",
];
pub const AI_REGISTRATION_BATCHES: &[u8] = &[3, 4, 5, 10];
pub const AI_CONFIDENCE_BATCHES: &[u8] = &[5];

#[derive(Debug)]
struct FixtureSpec {
    file_name: String,
    base_dir: String,
    fixture: ConformanceFixture,
    json: String,
}

#[derive(Debug)]
struct ProfileCheck {
    passed: bool,
    message: String,
}

/// Processor claims that can be asserted in Batch 16.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessorClaims {
    /// Claims Governance Basic conformance (G-051).
    #[serde(default, alias = "governanceBasic")]
    pub governance_basic: bool,
    /// Claims Governance Complete conformance (G-052).
    #[serde(default, alias = "governanceComplete")]
    pub governance_complete: bool,
    /// Claims agent registration support (AI-001).
    #[serde(default, alias = "agentRegistration")]
    pub agent_registration: bool,
    /// Claims confidence framework support (AI-002).
    #[serde(default, alias = "confidenceFramework")]
    pub confidence_framework: bool,
    /// Claims delegation of Formspec evaluation to a conformant processor (AI-004).
    #[serde(default, alias = "delegatesFormspecEvaluation")]
    pub delegates_formspec_evaluation: bool,
    /// Claims Assist Governance Proxy conformance preservation (AI-050).
    #[serde(default, alias = "assistGovernanceProxyConformant")]
    pub assist_governance_proxy_conformant: bool,
}

/// Architectural evidence carried alongside processor claims.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessorEvidence {
    /// Evidence for AI-004.
    #[serde(default, alias = "delegatedFormspecEvaluation")]
    pub delegated_formspec_evaluation: Option<DelegatedFormspecEvaluationEvidence>,
    /// Evidence for AI-050.
    #[serde(default, alias = "assistGovernanceProxy")]
    pub assist_governance_proxy: Option<AssistGovernanceProxyEvidence>,
}

/// Evidence that Formspec evaluation is delegated through an adapter boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedFormspecEvaluationEvidence {
    /// Whether a contract-validation adapter seam was exercised under test.
    #[serde(alias = "adapterExercised")]
    pub adapter_exercised: bool,
    /// Profile or identifier of the delegated processor.
    #[serde(alias = "delegatedProcessorProfile")]
    pub delegated_processor_profile: String,
    /// Whether the processor validated the full Response envelope, not a partial projection.
    #[serde(alias = "fullResponseEnvelopeValidated")]
    pub full_response_envelope_validated: bool,
}

/// Re-exported from `wos_core::proxy`.
pub use wos_core::proxy::AssistGovernanceProxyEvidence;

/// Processor manifest consumed by the Batch 16 verifier.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessorManifest {
    /// Human-readable processor name.
    #[serde(default, alias = "processorName")]
    pub processor_name: String,
    /// Explicit conformance claims.
    #[serde(default)]
    pub claims: ProcessorClaims,
    /// Architectural evidence attached to the claims.
    #[serde(default)]
    pub evidence: ProcessorEvidence,
}

/// Verification status for a single meta-rule claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// The processor does not claim this rule.
    NotClaimed,
    /// The claim is backed by the currently available evidence.
    Verified,
    /// The processor claimed the rule but the evidence was insufficient.
    Failed,
}

/// Verification result for one Batch 16 rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimVerification {
    /// Rule identifier, e.g. `G-051`.
    pub rule_id: String,
    /// Verification status.
    pub status: ClaimStatus,
    /// Human-readable explanation of the result.
    pub message: String,
}

/// Batch 16 report for a processor manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConformanceReport {
    /// Processor name from the manifest.
    pub processor_name: String,
    /// Verification result per meta-rule.
    pub claims: Vec<ClaimVerification>,
}

impl ProcessorConformanceReport {
    /// Look up a claim result by rule id.
    pub fn claim(&self, rule_id: &str) -> Option<&ClaimVerification> {
        self.claims.iter().find(|claim| claim.rule_id == rule_id)
    }
}

/// Exercise a delegated contract-validation seam and derive AI-004 evidence.
///
/// This helper exists so Batch 16 tests can derive evidence from an observed
/// validator call instead of constructing AI-004 evidence declaratively.
///
/// # Errors
///
/// Returns an engine error if the delegated validator cannot be exercised.
pub fn observe_delegated_formspec_evaluation<V>(
    validator: &V,
    contract_ref: &str,
    response_envelope: &serde_json::Value,
    delegated_processor_profile: &str,
) -> Result<DelegatedFormspecEvaluationEvidence, ConformanceError>
where
    V: ContractValidator,
{
    let validation_result = validator
        .validate(contract_ref, response_envelope)
        .map_err(|error| {
            ConformanceError::Engine(format!(
                "delegated contract validation failed for '{contract_ref}': {error}"
            ))
        })?;

    Ok(DelegatedFormspecEvaluationEvidence {
        adapter_exercised: true,
        delegated_processor_profile: delegated_processor_profile.to_string(),
        full_response_envelope_validated: validation_result.valid,
    })
}

/// Verify a processor manifest against the built-in fixture inventory.
///
/// Batch 16 rules split into two evidence sources:
///
/// - `G-051`, `G-052`, `AI-001`, `AI-002` are verified by aggregate profile execution.
/// - `AI-004`, `AI-050` are verified by explicit architectural evidence declarations.
///
/// # Errors
///
/// Returns an error only if the fixture inventory itself cannot be loaded.
pub fn verify_processor_manifest(
    manifest: &ProcessorManifest,
    fixtures_dir: &Path,
) -> Result<ProcessorConformanceReport, ConformanceError> {
    let fixture_specs = load_fixture_specs(fixtures_dir)?;

    let governance_basic = evaluate_governance_basic(&fixture_specs);
    let governance_complete = evaluate_governance_complete(&fixture_specs);
    let ai_registration = evaluate_ai_registration(&fixture_specs);
    let ai_confidence = evaluate_ai_confidence(&fixture_specs);

    let claims = vec![
        verify_profile_claim("G-051", manifest.claims.governance_basic, &governance_basic),
        verify_profile_claim(
            "G-052",
            manifest.claims.governance_complete,
            &governance_complete,
        ),
        verify_profile_claim(
            "AI-001",
            manifest.claims.agent_registration,
            &ai_registration,
        ),
        verify_profile_claim(
            "AI-002",
            manifest.claims.confidence_framework,
            &ai_confidence,
        ),
        verify_ai004_claim(manifest),
        verify_ai050_claim(manifest),
    ];

    Ok(ProcessorConformanceReport {
        processor_name: manifest.processor_name.clone(),
        claims,
    })
}

fn verify_profile_claim(rule_id: &str, claimed: bool, profile: &ProfileCheck) -> ClaimVerification {
    if !claimed {
        return ClaimVerification {
            rule_id: rule_id.to_string(),
            status: ClaimStatus::NotClaimed,
            message: "claim not asserted by manifest".to_string(),
        };
    }

    ClaimVerification {
        rule_id: rule_id.to_string(),
        status: if profile.passed {
            ClaimStatus::Verified
        } else {
            ClaimStatus::Failed
        },
        message: profile.message.clone(),
    }
}

fn verify_ai004_claim(manifest: &ProcessorManifest) -> ClaimVerification {
    if !manifest.claims.delegates_formspec_evaluation {
        return ClaimVerification {
            rule_id: "AI-004".to_string(),
            status: ClaimStatus::NotClaimed,
            message: "claim not asserted by manifest".to_string(),
        };
    }

    let Some(evidence) = &manifest.evidence.delegated_formspec_evaluation else {
        return ClaimVerification {
            rule_id: "AI-004".to_string(),
            status: ClaimStatus::Failed,
            message: "missing delegated Formspec evaluation evidence".to_string(),
        };
    };

    let mut missing = Vec::new();
    if !evidence.adapter_exercised {
        missing.push("adapter seam not exercised");
    }
    if evidence.delegated_processor_profile.trim().is_empty() {
        missing.push("delegated processor profile not declared");
    }
    if !evidence.full_response_envelope_validated {
        missing.push("full Response envelope validation not evidenced");
    }

    ClaimVerification {
        rule_id: "AI-004".to_string(),
        status: if missing.is_empty() {
            ClaimStatus::Verified
        } else {
            ClaimStatus::Failed
        },
        message: if missing.is_empty() {
            format!(
                "delegated Formspec evaluation evidenced via adapter boundary against '{}'",
                evidence.delegated_processor_profile
            )
        } else {
            missing.join("; ")
        },
    }
}

fn verify_ai050_claim(manifest: &ProcessorManifest) -> ClaimVerification {
    if !manifest.claims.assist_governance_proxy_conformant {
        return ClaimVerification {
            rule_id: "AI-050".to_string(),
            status: ClaimStatus::NotClaimed,
            message: "claim not asserted by manifest".to_string(),
        };
    }

    let Some(evidence) = &manifest.evidence.assist_governance_proxy else {
        return ClaimVerification {
            rule_id: "AI-050".to_string(),
            status: ClaimStatus::Failed,
            message: "missing Assist Governance Proxy evidence".to_string(),
        };
    };

    let mut missing = Vec::new();
    if !evidence.differential_check_passed {
        missing.push("proxy differential check not evidenced");
    }
    if !evidence.strictness_preserved {
        missing.push("proxy strictness preservation not evidenced");
    }
    if !evidence.provenance_preserved {
        missing.push("proxy provenance preservation not evidenced");
    }

    ClaimVerification {
        rule_id: "AI-050".to_string(),
        status: if missing.is_empty() {
            ClaimStatus::Verified
        } else {
            ClaimStatus::Failed
        },
        message: if missing.is_empty() {
            "assist-governance proxy evidence shows identical-or-stricter enforcement".to_string()
        } else {
            missing.join("; ")
        },
    }
}

/// Run a named profile against all fixtures in a directory.
///
/// Returns `(passed, message)` where `passed` is true when every fixture
/// matching `matches_profile` succeeds, and all `required_rules` are covered.
///
/// This is the public entry point for `tests/profile_conformance.rs` so that
/// profile aggregation logic lives in one place.
pub fn run_profile_against_fixtures<F>(
    fixtures_dir: &Path,
    profile_name: &str,
    matches_profile: F,
    required_rules: Option<&[&str]>,
) -> (bool, String)
where
    F: Fn(&ConformanceFixture) -> bool,
{
    let specs = match load_fixture_specs(fixtures_dir) {
        Ok(specs) => specs,
        Err(error) => return (false, error.to_string()),
    };

    let selected: Vec<&FixtureSpec> = specs
        .iter()
        .filter(|spec| matches_profile(&spec.fixture))
        .collect();

    if selected.is_empty() {
        return (
            false,
            format!("profile '{profile_name}' did not match any fixtures"),
        );
    }

    let mut matched_rule_counts = BTreeMap::new();
    for spec in &selected {
        *matched_rule_counts
            .entry(spec.fixture.rule.clone())
            .or_insert(0usize) += 1;
    }

    if let Some(required_rules) = required_rules {
        let missing: Vec<&str> = required_rules
            .iter()
            .copied()
            .filter(|rule_id| !matched_rule_counts.contains_key(*rule_id))
            .collect();
        if !missing.is_empty() {
            return (
                false,
                format!(
                    "profile '{profile_name}' is missing fixture coverage for rules: {}",
                    missing.join(", ")
                ),
            );
        }
    }

    let mut failures = Vec::new();
    for spec in selected {
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

    if failures.is_empty() {
        let rules: BTreeSet<&str> = matched_rule_counts.keys().map(String::as_str).collect();
        (
            true,
            format!(
                "profile '{profile_name}' passed across {} fixtures covering {} rules ({})",
                matched_rule_counts.values().sum::<usize>(),
                rules.len(),
                rules.into_iter().collect::<Vec<_>>().join(", ")
            ),
        )
    } else {
        (
            false,
            format!(
                "profile '{profile_name}' failed:\n{}",
                failures.join("\n\n")
            ),
        )
    }
}

/// Validate that AI-family fixtures in `fixtures_dir` declare batch metadata and
/// that `required_batches` are present.
///
/// Returns `Ok(())` if all checks pass, or `Err(message)` with a description
/// of what is missing.
pub fn validate_ai_family_batch_coverage(
    fixtures_dir: &Path,
    required_batches: &[u8],
) -> Result<(), String> {
    let specs = load_fixture_specs(fixtures_dir).map_err(|e| e.to_string())?;

    let ai_specs: Vec<&FixtureSpec> = specs
        .iter()
        .filter(|spec| {
            ["AI-", "AG-", "AC-"]
                .iter()
                .any(|prefix| spec.fixture.rule.starts_with(prefix))
        })
        .collect();

    let missing_batches: Vec<&str> = ai_specs
        .iter()
        .filter(|spec| spec.fixture.batch.is_none())
        .map(|spec| spec.file_name.as_str())
        .collect();
    if !missing_batches.is_empty() {
        return Err(format!(
            "AI-family fixtures missing batch metadata: {}",
            missing_batches.join(", ")
        ));
    }

    let present_batches: std::collections::BTreeSet<u8> = ai_specs
        .iter()
        .filter_map(|spec| spec.fixture.batch)
        .collect();
    let missing: Vec<String> = required_batches
        .iter()
        .copied()
        .filter(|batch| !present_batches.contains(batch))
        .map(|batch| batch.to_string())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "missing fixture coverage for batches: {}",
            missing.join(", ")
        ));
    }

    Ok(())
}

fn evaluate_governance_basic(specs: &[FixtureSpec]) -> ProfileCheck {
    evaluate_profile(
        specs,
        "Governance Basic",
        |spec| GOVERNANCE_BASIC_RULES.contains(&spec.fixture.rule.as_str()),
        Some(GOVERNANCE_BASIC_RULES),
    )
}

fn evaluate_governance_complete(specs: &[FixtureSpec]) -> ProfileCheck {
    evaluate_profile(
        specs,
        "Governance Complete",
        |spec| spec.fixture.rule.starts_with("G-"),
        None,
    )
}

fn evaluate_ai_registration(specs: &[FixtureSpec]) -> ProfileCheck {
    let ai_specs = ai_family_specs(specs);
    if let Some(message) = ai_batch_declaration_failure(&ai_specs) {
        return ProfileCheck {
            passed: false,
            message,
        };
    }
    if let Some(message) =
        missing_batches_message("AI Registration", &ai_specs, AI_REGISTRATION_BATCHES)
    {
        return ProfileCheck {
            passed: false,
            message,
        };
    }

    evaluate_profile(
        specs,
        "AI Registration",
        |spec| {
            spec.fixture
                .batch
                .is_some_and(|batch| AI_REGISTRATION_BATCHES.contains(&batch))
        },
        None,
    )
}

fn evaluate_ai_confidence(specs: &[FixtureSpec]) -> ProfileCheck {
    let ai_specs = ai_family_specs(specs);
    if let Some(message) = ai_batch_declaration_failure(&ai_specs) {
        return ProfileCheck {
            passed: false,
            message,
        };
    }
    if let Some(message) =
        missing_batches_message("AI Confidence Framework", &ai_specs, AI_CONFIDENCE_BATCHES)
    {
        return ProfileCheck {
            passed: false,
            message,
        };
    }

    evaluate_profile(
        specs,
        "AI Confidence Framework",
        |spec| {
            spec.fixture
                .batch
                .is_some_and(|batch| AI_CONFIDENCE_BATCHES.contains(&batch))
        },
        None,
    )
}

fn evaluate_profile<F>(
    specs: &[FixtureSpec],
    profile_name: &str,
    matches_profile: F,
    required_rules: Option<&[&str]>,
) -> ProfileCheck
where
    F: Fn(&FixtureSpec) -> bool,
{
    let selected_fixtures: Vec<&FixtureSpec> =
        specs.iter().filter(|spec| matches_profile(spec)).collect();

    if selected_fixtures.is_empty() {
        return ProfileCheck {
            passed: false,
            message: format!("profile '{profile_name}' did not match any fixtures"),
        };
    }

    let mut matched_rule_counts = BTreeMap::new();
    for spec in &selected_fixtures {
        *matched_rule_counts
            .entry(spec.fixture.rule.clone())
            .or_insert(0usize) += 1;
    }

    if let Some(required_rules) = required_rules {
        let missing_rules: Vec<&str> = required_rules
            .iter()
            .copied()
            .filter(|rule_id| !matched_rule_counts.contains_key(*rule_id))
            .collect();

        if !missing_rules.is_empty() {
            return ProfileCheck {
                passed: false,
                message: format!(
                    "profile '{profile_name}' is missing fixture coverage for rules: {}",
                    missing_rules.join(", ")
                ),
            };
        }
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

    if failures.is_empty() {
        let matched_rules: BTreeSet<&str> =
            matched_rule_counts.keys().map(String::as_str).collect();
        ProfileCheck {
            passed: true,
            message: format!(
                "profile '{profile_name}' passed across {} fixtures covering {} rules ({})",
                matched_rule_counts.values().sum::<usize>(),
                matched_rules.len(),
                matched_rules.into_iter().collect::<Vec<_>>().join(", ")
            ),
        }
    } else {
        ProfileCheck {
            passed: false,
            message: format!(
                "profile '{profile_name}' failed:\n{}",
                failures.join("\n\n")
            ),
        }
    }
}

fn ai_family_specs<'a>(specs: &'a [FixtureSpec]) -> Vec<&'a FixtureSpec> {
    specs
        .iter()
        .filter(|spec| {
            ["AI-", "AG-", "AC-"]
                .iter()
                .any(|prefix| spec.fixture.rule.starts_with(prefix))
        })
        .collect()
}

fn ai_batch_declaration_failure(specs: &[&FixtureSpec]) -> Option<String> {
    let missing_batches: Vec<&str> = specs
        .iter()
        .filter(|spec| spec.fixture.batch.is_none())
        .map(|spec| spec.file_name.as_str())
        .collect();

    (!missing_batches.is_empty()).then(|| {
        format!(
            "AI-family fixtures missing batch metadata: {}",
            missing_batches.join(", ")
        )
    })
}

fn missing_batches_message(
    profile_name: &str,
    specs: &[&FixtureSpec],
    required_batches: &[u8],
) -> Option<String> {
    let present_batches: BTreeSet<u8> =
        specs.iter().filter_map(|spec| spec.fixture.batch).collect();
    let missing_batches: Vec<String> = required_batches
        .iter()
        .copied()
        .filter(|batch| !present_batches.contains(batch))
        .map(|batch| batch.to_string())
        .collect();

    (!missing_batches.is_empty()).then(|| {
        format!(
            "profile '{profile_name}' is missing fixture coverage for batches: {}",
            missing_batches.join(", ")
        )
    })
}

fn load_fixture_specs(fixtures_dir: &Path) -> Result<Vec<FixtureSpec>, ConformanceError> {
    let mut fixture_paths: Vec<PathBuf> = std::fs::read_dir(fixtures_dir)
        .map_err(|error| ConformanceError::Parse(format!("fixtures dir read error: {error}")))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| ConformanceError::Parse(format!("fixture entry error: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    fixture_paths.sort();

    fixture_paths
        .into_iter()
        // Only loose `*.json` files in the fixtures directory root are
        // standard `ConformanceFixture` entries. Subdirectories (notably
        // `export/`, which holds export-conformance fixtures with a
        // distinct envelope) are skipped: `read_dir` does not recurse, so
        // directory entries have no `.json` extension and fall out here.
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| {
            let json = std::fs::read_to_string(&path).map_err(|error| {
                ConformanceError::Parse(format!("read fixture '{}': {error}", path.display()))
            })?;
            let fixture: ConformanceFixture = serde_json::from_str(&json).map_err(|error| {
                ConformanceError::Parse(format!("parse fixture '{}': {error}", path.display()))
            })?;

            Ok(FixtureSpec {
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
            })
        })
        .collect()
}
