// Rust guideline compliant 2026-02-21

//! Processor-level conformance claim verification.
//!
//! Batch 16 meta-rules are claims about the processor as a whole rather than
//! about a single fixture. This module verifies those claims by combining:
//!
//! - aggregate profile execution over the fixture inventory,
//! - explicit processor claims, and
//! - architectural evidence for delegation and proxy boundaries.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wos_core::deontic;
use wos_core::event_handler;
use wos_core::model::ai::{AIIntegrationDocument, ViolationAction};
use wos_core::model::kernel::ImpactLevel;
use wos_core::provenance::ProvenanceKind;
use wos_core::traits::ContractValidator;

use crate::{ConformanceError, ConformanceFixture, run_fixture};

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

/// Evidence that an Assist Governance Proxy preserves required constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistGovernanceProxyEvidence {
    /// Whether proxy-on vs proxy-off behavior was compared under test.
    #[serde(alias = "differentialCheckPassed")]
    pub differential_check_passed: bool,
    /// Whether the proxy remained identical or stricter on required checks.
    #[serde(alias = "strictnessPreserved")]
    pub strictness_preserved: bool,
    /// Whether required provenance remained present with the proxy enabled.
    #[serde(alias = "provenancePreserved")]
    pub provenance_preserved: bool,
}

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

#[derive(Debug)]
struct ProxyBehavior {
    blocked: bool,
    requires_escalation: bool,
    violation_ids: BTreeSet<String>,
    proxy_invocation_recorded: bool,
    invocation_source_preserved: bool,
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
    validator
        .validate(contract_ref, response_envelope)
        .map_err(|error| {
            ConformanceError::Engine(format!(
                "delegated contract validation failed for '{contract_ref}': {error}"
            ))
        })?;

    Ok(DelegatedFormspecEvaluationEvidence {
        adapter_exercised: true,
        delegated_processor_profile: delegated_processor_profile.to_string(),
        full_response_envelope_validated: true,
    })
}

/// Compare direct and proxied agent execution to derive AI-050 evidence.
///
/// The proxy path must not weaken enforcement. It may add provenance and may
/// be stricter, but it cannot reduce violations or relax escalation/blocking.
pub fn observe_assist_governance_proxy(
    ai_doc: &AIIntegrationDocument,
    actor_id: &str,
    event_name: &str,
    event_data: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: ImpactLevel,
) -> AssistGovernanceProxyEvidence {
    let direct = observe_proxy_behavior(
        ai_doc,
        actor_id,
        event_name,
        event_data,
        case_state,
        impact_level,
        None,
    );
    let proxy = observe_proxy_behavior(
        ai_doc,
        actor_id,
        event_name,
        event_data,
        case_state,
        impact_level,
        Some("assist-proxy"),
    );

    AssistGovernanceProxyEvidence {
        differential_check_passed: true,
        strictness_preserved: severity_rank(&proxy) >= severity_rank(&direct)
            && proxy.violation_ids.is_superset(&direct.violation_ids),
        provenance_preserved: proxy.proxy_invocation_recorded && proxy.invocation_source_preserved,
    }
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

fn observe_proxy_behavior(
    ai_doc: &AIIntegrationDocument,
    actor_id: &str,
    event_name: &str,
    event_data: &serde_json::Value,
    case_state: &HashMap<String, serde_json::Value>,
    impact_level: ImpactLevel,
    invocation_source: Option<&str>,
) -> ProxyBehavior {
    let mut data = event_data.clone();
    let Some(data_object) = data.as_object_mut() else {
        return ProxyBehavior {
            blocked: false,
            requires_escalation: false,
            violation_ids: BTreeSet::new(),
            proxy_invocation_recorded: false,
            invocation_source_preserved: false,
        };
    };

    if let Some(source) = invocation_source {
        data_object.insert("invocationSource".to_string(), serde_json::json!(source));
    } else {
        data_object.remove("invocationSource");
    }

    let output = data.get("output").unwrap_or(&serde_json::Value::Null);
    let bypass = data
        .get("deonticBypass")
        .or_else(|| data.get("bypass"))
        .and_then(|value| value.get("rationale"))
        .and_then(|value| value.as_str());
    let escalation_active = data
        .get("escalationActive")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let deontic_result = deontic::evaluate_deontic_constraints(
        ai_doc,
        actor_id,
        output,
        case_state,
        &impact_level,
        bypass,
        escalation_active,
        invocation_source,
    );

    let mut seen_idempotency_keys = HashSet::new();
    let handler_result = event_handler::evaluate_event(
        event_name,
        actor_id,
        &data,
        ai_doc.agents.iter().any(|agent| agent.id == actor_id),
        None,
        &HashMap::new(),
        &mut seen_idempotency_keys,
    );

    let violation_ids = deontic_result
        .provenance
        .iter()
        .filter(|record| record.record_kind == ProvenanceKind::DeonticViolation)
        .filter_map(|record| {
            record
                .data
                .as_ref()
                .and_then(|data| data.get("constraintId"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        })
        .collect();

    let invocation_source_preserved = if let Some(source) = invocation_source {
        deontic_result
            .provenance
            .iter()
            .filter(|record| record.record_kind == ProvenanceKind::DeonticViolation)
            .all(|record| {
                record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("invocationSource"))
                    .and_then(|value| value.as_str())
                    == Some(source)
            })
    } else {
        true
    };

    let proxy_invocation_recorded = invocation_source.is_some_and(|source| {
        handler_result.provenance.iter().any(|record| {
            record.record_kind == ProvenanceKind::ProxyInvocation
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("source"))
                    .and_then(|value| value.as_str())
                    == Some(source)
        })
    });

    ProxyBehavior {
        blocked: handler_result.blocked
            || matches!(
                deontic_result.effective_action,
                Some(ViolationAction::Reject)
            ),
        requires_escalation: handler_result.requires_escalation
            || matches!(
                deontic_result.effective_action,
                Some(ViolationAction::EscalateToHuman)
            ),
        violation_ids,
        proxy_invocation_recorded,
        invocation_source_preserved,
    }
}

fn severity_rank(behavior: &ProxyBehavior) -> u8 {
    if behavior.blocked {
        return 2;
    }
    if behavior.requires_escalation {
        return 1;
    }
    0
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
