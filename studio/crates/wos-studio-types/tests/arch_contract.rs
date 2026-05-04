// Rust guideline compliant 2026-05-04

//! Stage 7 conformance skeleton — shape-only tests against the
//! `wos_studio_types::arch` trait surface.
//!
//! These tests assert that the architectural contract code stays
//! coherent: type aliases serialize, the ConfidenceRecord shape
//! holds, and trait stubs compile against minimal in-test
//! implementations. Stage 8+ adapters add behavioral conformance on
//! top.
//!
//! See [`reference-architecture.md`](../../../specs/reference-architecture.md)
//! §"Stage 7 conformance skeletons" and ADR 0091 §2.5.

use serde_json;
use std::error::Error;
use std::fmt;
use wos_studio_types::arch::{
    AIInvocationRef, AILineageExt, ApprovalPackageRef, ConfidenceRecord, ExportBundleRef,
    HumanReviewState, ProjectionRef, ProjectionTarget, RiskTier, SchemaValidationResult,
    SourceBlobRef, StudioPortError, VerifierResult,
};

// --- Type-alias serialization round-trip ----------------------------

#[test]
fn projection_ref_round_trips() {
    let r = ProjectionRef("wos-workflow:abc123".to_string());
    let s = serde_json::to_string(&r).unwrap();
    let back: ProjectionRef = serde_json::from_str(&s).unwrap();
    assert_eq!(r, back);
}

#[test]
fn approval_package_ref_round_trips() {
    let r = ApprovalPackageRef("approval:xyz789".to_string());
    let s = serde_json::to_string(&r).unwrap();
    let back: ApprovalPackageRef = serde_json::from_str(&s).unwrap();
    assert_eq!(r, back);
}

#[test]
fn export_bundle_ref_round_trips() {
    let r = ExportBundleRef("bundle:2026-05-04".to_string());
    let s = serde_json::to_string(&r).unwrap();
    let back: ExportBundleRef = serde_json::from_str(&s).unwrap();
    assert_eq!(r, back);
}

// --- ConfidenceRecord shape (load-bearing per ADR 0088 §2.1) --------

#[test]
fn confidence_record_carries_six_signals() {
    let cr = ConfidenceRecord {
        schema_validation_result: SchemaValidationResult::Passed,
        citation_support_score: 0.92,
        retrieval_score: 0.81,
        verifier_result: VerifierResult::Agreed,
        risk_tier: RiskTier::Medium,
        human_review_state: HumanReviewState::Approved,
    };
    let s = serde_json::to_string(&cr).unwrap();
    let back: ConfidenceRecord = serde_json::from_str(&s).unwrap();
    assert_eq!(cr, back);
    // All six signals must be present in serialized form.
    for key in [
        "schemaValidationResult",
        "citationSupportScore",
        "retrievalScore",
        "verifierResult",
        "riskTier",
        "humanReviewState",
    ] {
        assert!(
            s.contains(key),
            "ConfidenceRecord missing field {key} in {s}"
        );
    }
}

#[test]
fn risk_tier_block_is_distinct_from_high() {
    // Per ADR 0088 §2.1: riskTier=block MUST NOT be approved by any
    // single reviewer. Test that block is a distinct enum value, not
    // an alias.
    assert_ne!(RiskTier::Block, RiskTier::High);
}

#[test]
fn human_review_state_includes_revised_then_approved() {
    // Per ADR 0088 §2.1: humanReviewState ∈ { Approved,
    // RevisedThenApproved } before durability. Test both are present.
    let approved = serde_json::to_string(&HumanReviewState::Approved).unwrap();
    let revised = serde_json::to_string(&HumanReviewState::RevisedThenApproved).unwrap();
    assert_ne!(approved, revised);
}

// --- AILineageExt extends authoring-provenance AI subtype -----------

#[test]
fn ai_lineage_ext_carries_replay_primitive_and_confidence() {
    let lineage = AILineageExt {
        invocation: AIInvocationRef("inv:001".to_string()),
        input_hash: "sha256:aaa".to_string(),
        output_hash: "sha256:bbb".to_string(),
        retrieval_set_hash: "sha256:ccc".to_string(),
        validator_verdicts: vec!["schema:passed".to_string()],
        confidence: ConfidenceRecord {
            schema_validation_result: SchemaValidationResult::Passed,
            citation_support_score: 0.99,
            retrieval_score: 0.85,
            verifier_result: VerifierResult::Agreed,
            risk_tier: RiskTier::Low,
            human_review_state: HumanReviewState::Pending,
        },
    };
    let s = serde_json::to_string(&lineage).unwrap();
    let back: AILineageExt = serde_json::from_str(&s).unwrap();
    assert_eq!(lineage, back);
}

// --- ProjectionTarget trait stub compiles against a minimal impl ----

#[derive(Debug)]
struct DemoErr;
impl fmt::Display for DemoErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "demo")
    }
}
impl Error for DemoErr {}
impl StudioPortError for DemoErr {}

struct NoopProjectionTarget;

impl ProjectionTarget for NoopProjectionTarget {
    type Error = DemoErr;
    type Knowledge = ();
    type Intent = ();
    type Artifact = ProjectionRef;
    type ValidationReport = bool;

    fn project(
        &self,
        _knowledge: &Self::Knowledge,
        _intent: &Self::Intent,
    ) -> Result<Self::Artifact, Self::Error> {
        Ok(ProjectionRef("noop:0".to_string()))
    }

    fn validate(&self, _artifact: &Self::Artifact) -> Self::ValidationReport {
        true
    }
}

#[test]
fn projection_target_trait_admits_minimal_impl() {
    let t = NoopProjectionTarget;
    let r = t.project(&(), &()).unwrap();
    assert_eq!(r.0, "noop:0");
    assert!(t.validate(&r));
}

// --- SourceBlobRef serializes -------------------------------------

#[test]
fn source_blob_ref_round_trips() {
    let r = SourceBlobRef("blake3:deadbeef".to_string());
    let s = serde_json::to_string(&r).unwrap();
    let back: SourceBlobRef = serde_json::from_str(&s).unwrap();
    assert_eq!(r, back);
}
