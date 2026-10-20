// Rust guideline compliant 2026-02-21
//! Decides authoring verification from live, selection-bound native check evidence.
//!
//! Static validation and replay keep ownership of their own stages and reports.
//! This module checks required inventory, selection binding, and explicit absence.
//! Saved serialized reports cannot construct static invocation evidence.
use super::{
    ConformanceError, PreparedReferenceReplay, PreparedReplay, ReplayResult, content_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wos_lint::studio_api::{CheckStatus, ValidationReport, ValidationRequest, ValidationScope};

/// Actual static invocation and its exact selected inputs and producer identity.
#[derive(Debug)]
pub struct StaticCheck {
    report: ValidationReport,
    identity: String,
    owner_identity: Value,
}
impl StaticCheck {
    /// Returns the unchanged native validation report.
    pub fn report(&self) -> &ValidationReport {
        &self.report
    }
    /// Returns the complete selected static invocation identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Returns the maintained native source and schema build identity.
    pub fn owner_identity(&self) -> &Value {
        &self.owner_identity
    }
}
fn owner_identity() -> Value {
    serde_json::from_str(env!("WOS_REPLAY_BUILD_IDENTITY")).expect("build identity is JSON")
}
fn static_identity(request: &ValidationRequest<'_>) -> Result<String, ConformanceError> {
    content_digest(
        &json!({"candidate":request.candidate,"dependencies":request.dependencies,
        "scope":request.scope,"owner":owner_identity()}),
    )
}
/// Runs the static owner once and binds its report to that invocation.
///
/// # Errors
/// Returns canonical encoding errors for the selected immutable JSON values.
pub fn validate_static(request: ValidationRequest<'_>) -> Result<StaticCheck, ConformanceError> {
    let identity = static_identity(&request)?;
    let report = wos_lint::studio_api::validate(request);
    Ok(StaticCheck {
        report,
        identity,
        owner_identity: owner_identity(),
    })
}
/// Static evidence compared with the independently selected current request.
#[derive(Debug)]
pub enum StaticEvidence<'a> {
    /// Exact current selections and actual invocation-bound report.
    Completed {
        request: ValidationRequest<'a>,
        check: &'a StaticCheck,
    },
    /// Actual owner invocation/preparation error.
    Error(&'a str),
    /// Explicitly unperformed static check.
    NotRun(&'a str),
}
/// Compact current selection, produced only from an actual prepared request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySelection {
    identity: String,
    candidate_sha256: Option<String>,
}
impl ReplaySelection {
    /// Retains the current independently prepared native selection.
    pub fn native(prepared: &PreparedReplay) -> Self {
        Self {
            identity: prepared.identity().into(),
            candidate_sha256: prepared
                .inputs
                .iter()
                .find(|input| input.role == "kernel")
                .map(|input| input.canonical_sha256.clone()),
        }
    }
    /// Retains the current independently prepared reference selection.
    pub fn reference(prepared: &PreparedReferenceReplay) -> Self {
        Self::native(&prepared.prepared)
    }
}
/// Compact actual invocation outcome; complete reports stay with the caller.
#[derive(Debug, Clone)]
pub struct ReplayCheck {
    selection: ReplaySelection,
    codes: Vec<&'static str>,
}
impl ReplayCheck {
    /// Captures the actual typed native result before its report is serialized.
    pub fn native(prepared: &PreparedReplay, result: &ReplayResult) -> Self {
        let mut codes = vec![];
        if result.identity != prepared.identity()
            || &result.runtime_identity != prepared.runtime_identity()
            || result.inputs != prepared.inputs
        {
            codes.push("selection-mismatch");
        }
        if !result.passed || result.status != "pass" {
            codes.push(if result.status == "refused" {
                "refused"
            } else {
                "failed"
            });
        }
        Self {
            selection: ReplaySelection::native(prepared),
            codes,
        }
    }
    /// Captures the actual result and immutable selection retained by a live session.
    pub fn session(session: &super::NativeReplaySession) -> Self {
        Self::reference(session.prepared(), session.result())
    }
    /// Captures the actual typed reference result before its report is serialized.
    pub fn reference(prepared: &PreparedReferenceReplay, result: &ReplayResult) -> Self {
        Self::native(&prepared.prepared, result)
    }
}
/// Actual replay receipt compared with an independently current prepared selection.
#[derive(Debug)]
pub enum ReplayOutcome<'a> {
    /// Compact actual check and current expected selection.
    Completed {
        selection: &'a ReplaySelection,
        check: &'a ReplayCheck,
    },
    /// Actual preparation or execution error.
    Error(&'a str),
    /// Explicitly unperformed replay.
    NotRun(&'a str),
}
/// One configured scenario result; its ID does not define required inventory.
#[derive(Debug)]
pub struct ReplayEvidence<'a> {
    /// Scenario identity from the existing caller configuration.
    pub id: &'a str,
    /// Actual owner result or explicit absence.
    pub outcome: ReplayOutcome<'a>,
}
/// Existing Studio requirements and typed specialized native comparisons.
#[derive(Debug)]
pub enum RequirementOutcome<'a> {
    /// Existing Studio-owned requirement passed.
    Passed,
    /// Existing Studio-owned requirement failed.
    Failed,
    /// Existing requirement was not performed.
    NotRun,
    /// Existing accepted gap remains unverified for required handoff.
    AcceptedGap,
    /// Actual specialized native check with its independently current selection.
    Comparison {
        selection: &'a ComparisonSelection,
        check: &'a ComparisonCheck,
    },
}
/// A requirement translated from an existing Studio record.
#[derive(Debug)]
pub struct AdditionalRequirement<'a> {
    /// Existing requirement identity.
    pub id: &'a str,
    /// Existing owning source reference.
    pub source: &'a str,
    /// Whether this requirement blocks handoff.
    pub required: bool,
    /// Existing owner outcome, never synthesized native success.
    pub outcome: RequirementOutcome<'a>,
}
/// Independently configured required scenario inventory and explicit draft mode.
#[derive(Debug)]
pub struct AuthoringRequirements<'a> {
    /// Required IDs derived from existing configuration before execution.
    pub required_scenarios: &'a [String],
    /// Explicit static-only draft; never verified for handoff.
    pub draft_only: bool,
}
/// Shared authoring result, independent of publication approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringStatus {
    /// Required native and existing Studio checks passed for selected inputs.
    Verified,
    /// Static-only draft; runtime behavior is unverified.
    Draft,
    /// Required checks failed, were absent, or did not match selected inputs.
    Blocked,
}
/// One stable reason and its owning check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringReason {
    /// Owning configured check ID.
    pub check_id: String,
    /// Stable machine-readable explanation.
    pub code: String,
    /// Existing source reference or actual error detail when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Whether this reason prevents verified handoff.
    pub blocking: bool,
}
/// One shared authoring decision and deterministic ordered reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringDecision {
    /// Verified, explicit draft, or blocked.
    pub status: AuthoringStatus,
    /// All nonpassing required and optional checks, in deterministic order.
    pub reasons: Vec<AuthoringReason>,
}
fn reason(
    reasons: &mut Vec<AuthoringReason>,
    id: &str,
    code: &str,
    source: Option<&str>,
    blocking: bool,
) {
    reasons.push(AuthoringReason {
        check_id: id.into(),
        code: code.into(),
        source: source.map(str::to_owned),
        blocking,
    });
}
/// Decides required checks using actual native results and current selections.
///
/// Required inventory comes from configuration, never from returned reports.
/// Static request hashing checks freshness without running validation again.
/// Static and replay dependencies intentionally remain independent selections.
pub fn decide_authoring(
    requirements: AuthoringRequirements<'_>,
    static_evidence: StaticEvidence<'_>,
    replays: &[ReplayEvidence<'_>],
    additional: &[AdditionalRequirement<'_>],
) -> AuthoringDecision {
    let mut reasons = vec![];
    let candidate_digest = match static_evidence {
        StaticEvidence::Completed { request, check } => {
            if static_identity(&request).ok().as_deref() != Some(check.identity()) {
                reason(&mut reasons, "static", "selection-mismatch", None, true);
            }
            if !requirements.draft_only && request.scope != ValidationScope::Project {
                reason(&mut reasons, "static", "project-scope-required", None, true);
            }
            match check.report().status {
                CheckStatus::Pass => {}
                CheckStatus::Fail => reason(&mut reasons, "static", "failed", None, true),
                CheckStatus::NotRun => reason(&mut reasons, "static", "not-run", None, true),
            }
            content_digest(request.candidate).ok()
        }
        StaticEvidence::Error(error) => {
            reason(&mut reasons, "static", "error", Some(error), true);
            None
        }
        StaticEvidence::NotRun(why) => {
            reason(&mut reasons, "static", "not-run", Some(why), true);
            None
        }
    };
    if requirements.draft_only {
        reason(&mut reasons, "replay", "static-only-draft", None, false);
        if !requirements.required_scenarios.is_empty() || !replays.is_empty() {
            reason(&mut reasons, "replay", "draft-has-scenarios", None, true);
        }
    } else if requirements.required_scenarios.is_empty() {
        reason(&mut reasons, "replay", "no-required-scenarios", None, true);
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in requirements.required_scenarios {
        if id.trim().is_empty() || !seen.insert(id) {
            reason(&mut reasons, id, "invalid-required-inventory", None, true);
        }
        match replays.iter().filter(|record| record.id == id).count() {
            0 => reason(&mut reasons, id, "missing-required-result", None, true),
            1 => {}
            _ => reason(&mut reasons, id, "duplicate-result", None, true),
        }
    }
    for record in replays {
        let required = requirements
            .required_scenarios
            .iter()
            .any(|id| id == record.id);
        match &record.outcome {
            ReplayOutcome::Completed { selection, check } => {
                if *selection != &check.selection {
                    reason(
                        &mut reasons,
                        record.id,
                        "selection-mismatch",
                        None,
                        required,
                    );
                }
                if candidate_digest.is_none() || candidate_digest != selection.candidate_sha256 {
                    reason(
                        &mut reasons,
                        record.id,
                        "candidate-mismatch",
                        None,
                        required,
                    );
                }
                for code in &check.codes {
                    reason(&mut reasons, record.id, code, None, required);
                }
            }
            ReplayOutcome::Error(error) => {
                reason(&mut reasons, record.id, "error", Some(error), required)
            }
            ReplayOutcome::NotRun(why) => {
                reason(&mut reasons, record.id, "not-run", Some(why), required)
            }
        }
    }
    for requirement in additional {
        let code = match &requirement.outcome {
            RequirementOutcome::Passed => None,
            RequirementOutcome::Failed => Some("failed"),
            RequirementOutcome::NotRun => Some("not-run"),
            RequirementOutcome::AcceptedGap => Some("accepted-gap"),
            RequirementOutcome::Comparison { selection, check } => {
                if *selection != &check.selection {
                    Some("selection-mismatch")
                } else {
                    check.code
                }
            }
        };
        if let Some(code) = code {
            reason(
                &mut reasons,
                requirement.id,
                code,
                Some(requirement.source),
                requirement.required,
            );
        }
    }
    let status = if reasons.iter().any(|reason| reason.blocking) {
        AuthoringStatus::Blocked
    } else if requirements.draft_only {
        AuthoringStatus::Draft
    } else {
        AuthoringStatus::Verified
    };
    AuthoringDecision { status, reasons }
}

/// Exact specialized check selection, independent of returned outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonSelection(String);
impl ComparisonSelection {
    /// Identifies a selected observation and independent expected value.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn observation(
        prepared: &PreparedReferenceReplay,
        selector: &super::NativeSelector,
        expected: &super::ObservationExpectation,
    ) -> Result<Self, ConformanceError> {
        observation_selection(prepared.identity(), selector, expected)
    }
    /// Identifies a native observation and independent expected value.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn observation_native(
        prepared: &PreparedReplay,
        selector: &super::NativeSelector,
        expected: &super::ObservationExpectation,
    ) -> Result<Self, ConformanceError> {
        observation_selection(prepared.identity(), selector, expected)
    }
    /// Identifies native provenance and its independent partial expectation.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn provenance_native(
        prepared: &PreparedReplay,
        attempt: &super::NativeAttemptSelector,
        expected: &Value,
    ) -> Result<Self, ConformanceError> {
        provenance_selection(prepared.identity(), attempt, expected)
    }
    /// Identifies selected provenance and its independent partial expectation.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn provenance(
        prepared: &PreparedReferenceReplay,
        attempt: &super::NativeAttemptSelector,
        expected: &Value,
    ) -> Result<Self, ConformanceError> {
        provenance_selection(prepared.identity(), attempt, expected)
    }
    /// Identifies the current native session, retained review, and comparison time.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn rendered_notice(
        prepared: &PreparedReferenceReplay,
        assertion: &super::RenderedNoticeReviewAssertion,
    ) -> Result<Self, ConformanceError> {
        notice_selection(prepared.identity(), assertion)
    }
    /// Uses the paired comparison owner's complete request identity.
    /// # Errors
    /// Returns canonical encoding errors for selected JSON values.
    pub fn paired_approval(
        request: &super::PairedApprovalRateRequest,
    ) -> Result<Self, ConformanceError> {
        crate::reference_comparisons::content_digest(request).map(Self)
    }
}
fn observation_selection(
    identity: &str,
    selector: &super::NativeSelector,
    expected: &super::ObservationExpectation,
) -> Result<ComparisonSelection, ConformanceError> {
    content_digest(&json!({"replay":identity,"observation":selector,"expected":expected}))
        .map(ComparisonSelection)
}
fn provenance_selection(
    identity: &str,
    attempt: &super::NativeAttemptSelector,
    expected: &Value,
) -> Result<ComparisonSelection, ConformanceError> {
    content_digest(&json!({"replay":identity,"provenance":attempt,"expected":expected}))
        .map(ComparisonSelection)
}
fn notice_selection(
    identity: &str,
    assertion: &super::RenderedNoticeReviewAssertion,
) -> Result<ComparisonSelection, ConformanceError> {
    content_digest(&json!({"replay":identity,"notice":assertion})).map(ComparisonSelection)
}
/// Actual specialized owner outcome; serialized reports cannot create this receipt.
#[derive(Debug, Clone)]
pub struct ComparisonCheck {
    selection: ComparisonSelection,
    code: Option<&'static str>,
}
fn comparison_code(result: &super::NativeComparison) -> Option<&'static str> {
    match result {
        super::NativeComparison::Pass => None,
        super::NativeComparison::Fail { .. } => Some("failed"),
        super::NativeComparison::Refused { .. } => Some("refused"),
    }
}
fn status_code(status: &str) -> Option<&'static str> {
    if status == "pass" {
        None
    } else if status == "refused" {
        Some("refused")
    } else {
        Some("failed")
    }
}
/// Runs the native observation comparison once and retains its bound decision evidence.
/// # Errors
/// Returns canonical encoding errors for selected JSON values.
pub fn check_observation(
    result: &ReplayResult,
    selector: &super::NativeSelector,
    expected: &super::ObservationExpectation,
) -> Result<(super::NativeComparison, ComparisonCheck), ConformanceError> {
    let selection = observation_selection(&result.identity, selector, expected)?;
    let comparison = super::compare_observation(result, selector, expected);
    let code = comparison_code(&comparison);
    Ok((comparison, ComparisonCheck { selection, code }))
}
/// Runs the native provenance comparison once and retains its bound decision evidence.
/// # Errors
/// Returns canonical encoding errors for selected JSON values.
pub fn check_provenance_observation(
    result: &ReplayResult,
    attempt: &super::NativeAttemptSelector,
    expected: &Value,
) -> Result<(super::NativeComparison, ComparisonCheck), ConformanceError> {
    let selection = provenance_selection(&result.identity, attempt, expected)?;
    let comparison = super::compare_provenance_observation(result, attempt, expected);
    let code = comparison_code(&comparison);
    Ok((comparison, ComparisonCheck { selection, code }))
}
/// Runs the notice comparison once and retains its exact session and review selection.
/// # Errors
/// Returns canonical encoding errors for selected JSON values.
pub fn check_rendered_notice(
    session: &super::NativeReplaySession,
    prepared: &PreparedReferenceReplay,
    assertion: &super::RenderedNoticeReviewAssertion,
) -> Result<(super::RenderedNoticeComparison, ComparisonCheck), ConformanceError> {
    let selection = notice_selection(session.identity(), assertion)?;
    let result = super::compare_rendered_notice(session, prepared, assertion);
    let code = status_code(&result.status);
    Ok((result, ComparisonCheck { selection, code }))
}
/// Runs the paired owner once and retains its complete member and threshold selection.
/// # Errors
/// Returns canonical encoding errors for selected JSON values.
pub fn check_paired_approval_rates(
    request: &super::PairedApprovalRateRequest,
) -> Result<(super::PairedApprovalRateResult, ComparisonCheck), ConformanceError> {
    let selection = ComparisonSelection::paired_approval(request)?;
    let result = super::compare_paired_approval_rates(request);
    let code = if result.identity != selection.0 {
        Some("selection-mismatch")
    } else {
        status_code(&result.status)
    };
    Ok((result, ComparisonCheck { selection, code }))
}
