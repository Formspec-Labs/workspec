// Rust guideline compliant 2026-02-21
//! Compares actual native cohort and retained notice execution evidence.

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use integrity_canonical::{DigestAlgorithm, canonical_json_bytes, compute_digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use wos_core::model::due_process::{NoticeState, ReviewRef, SourceRef};

use crate::ConformanceError;
use crate::native_observations::{NativeResolution, NativeSelector, resolve_observation};
use crate::studio_api::{
    PreparedReferenceReplay, ReferenceReplayAdapters, ReplayRequest, ReplayResult,
    prepare_reference_replay, replay_reference,
};

const PAIRS: usize = 100;

fn refused(message: impl Into<String>) -> ConformanceError {
    ConformanceError::UnsupportedOperation(format!("reference-comparison: {}", message.into()))
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", compute_digest(bytes, DigestAlgorithm::Sha256))
}
pub(crate) fn content_digest(value: &impl Serialize) -> Result<String, ConformanceError> {
    let value = serde_json::to_value(value).map_err(|error| refused(error.to_string()))?;
    canonical_json_bytes(&value)
        .map(|bytes| digest(&bytes))
        .map_err(refused)
}
fn canonical_time(value: &str) -> Result<OffsetDateTime, ConformanceError> {
    let parsed =
        OffsetDateTime::parse(value, &Rfc3339).map_err(|error| refused(error.to_string()))?;
    let nanos = parsed.unix_timestamp_nanos();
    if nanos < 0 || nanos % 1_000_000 != 0 {
        return Err(refused(
            "review time is outside native unsigned millisecond precision",
        ));
    }
    let canonical = OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map_err(|error| refused(error.to_string()))?
        .format(&Rfc3339)
        .map_err(|error| refused(error.to_string()))?;
    if canonical != value {
        return Err(refused(
            "review time is not canonical native UTC milliseconds",
        ));
    }
    Ok(parsed)
}
fn scalar(value: &Value) -> bool {
    value.is_string() || value.is_boolean() || value.is_number()
}

/// Complete independently authored inputs for one cohort member.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CohortMember {
    /// Unique identity across all 200 declared members.
    pub id: String,
    /// Complete original native request.
    pub request: ReplayRequest,
    /// Complete explicitly selected local capabilities.
    pub adapters: ReferenceReplayAdapters,
}

/// Explicit matching inputs with one protected scalar difference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairedReplayMembers {
    /// Selected left protected category.
    pub left: CohortMember,
    /// Selected right protected category.
    pub right: CohortMember,
}

/// Strict source-reviewed comparison of 100 matched pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairedApprovalRateRequest {
    /// Exactly 100 explicit pairs, with 200 unique member identities.
    pub pairs: Vec<PairedReplayMembers>,
    /// JSON pointer within each fixture's initial_case_state.
    pub protected_case_pointer: String,
    /// Exact selected left scalar.
    pub left_value: Value,
    /// Exact selected right scalar.
    pub right_value: Value,
    /// Reviewed integer percentage-point threshold; comparison is strictly less.
    pub threshold_percentage_points: u32,
    /// Exact retained source selections supporting this fixture policy.
    pub source_refs: Vec<SourceRef>,
    /// Exact retained independent fixture review selection.
    pub review: ReviewRef,
    /// JSON pointer to the unique selected native favorable final state.
    pub favorable_state_pointer: String,
    /// JSON pointer to the unique selected native adverse final state.
    pub adverse_state_pointer: String,
}

/// Actual selected terminal outcome, never inferred from missing execution.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CohortClassification {
    /// Actual favorable final configuration.
    Favorable,
    /// Actual adverse final configuration.
    Adverse,
}

/// Complete actual execution and classification for one declared member.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CohortMemberResult {
    /// Original declared unique identity.
    pub id: String,
    /// Complete native execution, when execution returned one.
    pub result: Option<ReplayResult>,
    /// Actual final classification; absent when the metric refuses.
    pub classification: Option<CohortClassification>,
    /// Exact preparation, execution or classification refusal.
    pub refusal: Option<String>,
}

/// Strict metric with all actual member evidence retained.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedApprovalRateResult {
    /// pass, fail, or refused; refused never supplies a numerical claim.
    pub status: String,
    /// Digest of complete original declarations and selected threshold.
    pub identity: String,
    /// Every attempted native member, in declared left/right pair order.
    pub members: Vec<CohortMemberResult>,
    /// Actual favorable count, only when all 200 members qualify.
    pub approved_left: Option<usize>,
    /// Actual favorable count, only when all 200 members qualify.
    pub approved_right: Option<usize>,
    /// Fixed independently declared denominator for each side.
    pub denominator_per_side: usize,
    /// Reviewed source threshold retained without relaxation.
    pub threshold_percentage_points: u32,
    /// Actual invalid input or incomplete evidence reasons.
    pub refusals: Vec<String>,
}

fn paired_inputs(
    prepared: &PreparedReferenceReplay,
    pointer: &str,
    expected: &Value,
) -> Result<Value, ConformanceError> {
    let mut original = prepared.original_inputs();
    let request = original
        .get_mut("request")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| refused("missing original request"))?;
    // Original fixture formatting is not a second authority over its parsed values.
    request.remove("fixtureJson");
    let case = original
        .get_mut("fixture")
        .and_then(|fixture| fixture.get_mut("initial_case_state"))
        .ok_or_else(|| refused("missing actual initial case"))?;
    let selected = case
        .pointer_mut(pointer)
        .ok_or_else(|| refused("missing protected scalar"))?;
    if selected != expected || !scalar(selected) {
        return Err(refused("protected scalar differs from selected category"));
    }
    *selected = json!({"$nativePairedScalar":true});
    Ok(original)
}

#[derive(Debug)]
struct SelectedTerminal {
    id: String,
    allowed_configuration: BTreeSet<String>,
}
fn final_state(candidate: &[u8], pointer: &str) -> Result<SelectedTerminal, ConformanceError> {
    let candidate: Value =
        serde_json::from_slice(candidate).map_err(|error| refused(error.to_string()))?;
    let mut states = Vec::new();
    fn collect(states: &Value, base: &str, output: &mut Vec<(String, String, bool)>) {
        let Some(states) = states.as_object() else {
            return;
        };
        for (id, state) in states {
            let path = format!("{base}/{}", id.replace('~', "~0").replace('/', "~1"));
            output.push((
                path.clone(),
                id.clone(),
                state.get("type").and_then(Value::as_str) == Some("final"),
            ));
            if let Some(children) = state.get("states") {
                collect(children, &format!("{path}/states"), output);
            }
            if let Some(regions) = state.get("regions").and_then(Value::as_object) {
                for (region, value) in regions {
                    let escaped = region.replace('~', "~0").replace('/', "~1");
                    if let Some(children) = value.get("states") {
                        collect(
                            children,
                            &format!("{path}/regions/{escaped}/states"),
                            output,
                        );
                    }
                }
            }
        }
    }
    collect(
        candidate
            .pointer("/lifecycle/states")
            .unwrap_or(&Value::Null),
        "/lifecycle/states",
        &mut states,
    );
    let selected = states
        .iter()
        .find(|(path, _, _)| path == pointer)
        .ok_or_else(|| refused("selected final state pointer is absent"))?;
    if !selected.2 || states.iter().filter(|(_, id, _)| id == &selected.1).count() != 1 {
        return Err(refused(
            "selected state is not a uniquely owned final state",
        ));
    }
    let allowed_configuration = states
        .iter()
        .filter(|(path, _, _)| path == pointer || pointer.starts_with(&format!("{path}/states/")))
        .map(|(_, id, _)| id.clone())
        .collect();
    Ok(SelectedTerminal {
        id: selected.1.clone(),
        allowed_configuration,
    })
}

/// One exact independently reviewed original member input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricMemberDescriptor {
    /// Original unique member identity.
    pub id: String,
    /// Full inputs excluding only this metric review's own bytes and digest.
    pub independent_input_sha256: String,
}
/// Exact reviewed metric, threshold, selected outcomes and every original member.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricDescriptor {
    /// Exactly absoluteApprovalRateDifference.
    pub metric: String,
    /// Exactly lt; equality fails.
    pub operator: String,
    /// Reviewed source threshold without relaxation.
    pub threshold_percentage_points: u32,
    /// Actual protected initial case scalar pointer.
    pub protected_case_pointer: String,
    /// Reviewed left category.
    pub left_value: Value,
    /// Reviewed right category.
    pub right_value: Value,
    /// Exact favorable final state source pointer.
    pub favorable_state_pointer: String,
    /// Exact adverse final state source pointer.
    pub adverse_state_pointer: String,
    /// Exact selected original source references.
    pub source_refs: Vec<SourceRef>,
    /// Retained review identity, excluding its cyclic content digest.
    pub review_id: String,
    /// Exact 200 original member declarations, preserving order.
    pub members: Vec<MetricMemberDescriptor>,
}
/// Explicit attributed independent engineering judgment about the metric inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricReview {
    /// Exactly softwareFixture.
    pub scope: String,
    /// Exactly accepted for execution; rejected or inconclusive refuses.
    pub decision: String,
    /// Honest engineering model attribution, not agency or human authority.
    pub reviewer: EngineeringReviewer,
    /// Valid RFC3339 recorded review time.
    pub reviewed_at: String,
    /// Concrete review reasoning about source threshold and member construction.
    pub rationale: String,
    /// Every exact reviewed input and selected metric.
    pub descriptor: MetricDescriptor,
}
/// Builds exact review inputs without executing or populating expected results.
///
/// # Errors
/// Refuses malformed fixture JSON or missing/ambiguous selected review entries.
pub fn paired_metric_descriptor(
    request: &PairedApprovalRateRequest,
) -> Result<MetricDescriptor, ConformanceError> {
    let mut members = Vec::new();
    for pair in &request.pairs {
        for member in [&pair.left, &pair.right] {
            let mut original =
                serde_json::to_value(member).map_err(|error| refused(error.to_string()))?;
            let fixture: Value = serde_json::from_str(&member.request.fixture_json)
                .map_err(|error| refused(error.to_string()))?;
            original
                .get_mut("request")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| refused("member request missing"))?
                .remove("fixtureJson");
            original["fixture"] = fixture;
            let evidence = original
                .pointer_mut("/adapters/retainedEvidence")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| refused("member evidence missing"))?;
            let mut matching = evidence.iter_mut().filter(|entry| {
                entry.get("id").and_then(Value::as_str) == Some(request.review.id.as_str())
                    && entry.get("role").and_then(Value::as_str) == Some("review")
            });
            let selected = matching
                .next()
                .ok_or_else(|| refused("metric review selection missing"))?;
            if matching.next().is_some() {
                return Err(refused("metric review selection ambiguous"));
            }
            let selected = selected
                .as_object_mut()
                .ok_or_else(|| refused("review selection is not an object"))?;
            // Only the selected metric review's own content is cyclic; retain its id/role.
            selected.remove("bytesBase64");
            selected.remove("sha256");
            members.push(MetricMemberDescriptor {
                id: member.id.clone(),
                independent_input_sha256: content_digest(&original)?,
            });
        }
    }
    Ok(MetricDescriptor {
        metric: "absoluteApprovalRateDifference".into(),
        operator: "lt".into(),
        threshold_percentage_points: request.threshold_percentage_points,
        protected_case_pointer: request.protected_case_pointer.clone(),
        left_value: request.left_value.clone(),
        right_value: request.right_value.clone(),
        favorable_state_pointer: request.favorable_state_pointer.clone(),
        adverse_state_pointer: request.adverse_state_pointer.clone(),
        source_refs: request.source_refs.clone(),
        review_id: request.review.id.clone(),
        members,
    })
}

fn source_review(
    prepared: &PreparedReferenceReplay,
    request: &PairedApprovalRateRequest,
    descriptor: &MetricDescriptor,
) -> Result<(), ConformanceError> {
    if request.source_refs.is_empty() || request.review.id.trim().is_empty() {
        return Err(refused("missing metric source or review"));
    }
    for source in &request.source_refs {
        let bytes = prepared
            .retained_evidence(&source.source_version, "source")
            .ok_or_else(|| refused("metric source bytes are not selected"))?;
        if source.section_anchor.trim().is_empty() || digest(bytes) != source.retained_sha256 {
            return Err(refused("metric source identity mismatch"));
        }
    }
    let review = prepared
        .retained_evidence(&request.review.id, "review")
        .ok_or_else(|| refused("metric review bytes are not selected"))?;
    if digest(review) != request.review.sha256 {
        return Err(refused("metric review identity mismatch"));
    }
    let review: MetricReview =
        serde_json::from_slice(review).map_err(|error| refused(error.to_string()))?;
    if review.scope != "softwareFixture"
        || review.decision != "accepted"
        || review.reviewer.kind != "model"
        || review.reviewer.id.trim().is_empty()
        || review.reviewer.method.trim().is_empty()
        || review.rationale.trim().is_empty()
        || &review.descriptor != descriptor
    {
        return Err(refused(
            "typed metric review does not accept these exact original inputs",
        ));
    }
    canonical_time(&review.reviewed_at)?;
    Ok(())
}

fn classify(
    result: &ReplayResult,
    favorable: &SelectedTerminal,
    adverse: &SelectedTerminal,
) -> Result<CohortClassification, ConformanceError> {
    if !result.passed
        || result.status != "pass"
        || !result.refusals.is_empty()
        || !result.failures.is_empty()
        || result.attempts.iter().any(|attempt| {
            attempt.error.is_some()
                || !attempt.refusals.is_empty()
                || !attempt.observation_errors.is_empty()
                || attempt
                    .submission_result
                    .as_ref()
                    .is_some_and(|value| value.get("completed").is_none())
        })
    {
        return Err(refused(
            "member execution did not pass every required observation",
        ));
    }
    let configuration = &result
        .final_snapshot
        .as_ref()
        .ok_or_else(|| refused("member has no final snapshot"))?
        .configuration;
    match (
        configuration
            .iter()
            .filter(|id| id.as_str() == favorable.id)
            .count(),
        configuration
            .iter()
            .filter(|id| id.as_str() == adverse.id)
            .count(),
    ) {
        (1, 0)
            if configuration
                .iter()
                .all(|id| favorable.allowed_configuration.contains(id)) =>
        {
            Ok(CohortClassification::Favorable)
        }
        (0, 1)
            if configuration
                .iter()
                .all(|id| adverse.allowed_configuration.contains(id)) =>
        {
            Ok(CohortClassification::Adverse)
        }
        _ => Err(refused(
            "member did not finish in exactly one selected terminal outcome",
        )),
    }
}

/// Executes every declared native member and computes the strict paired metric.
///
/// Invalid declarations and incomplete member execution produce refused evidence.
pub fn compare_paired_approval_rates(
    request: &PairedApprovalRateRequest,
) -> PairedApprovalRateResult {
    let mut report = PairedApprovalRateResult {
        status: "refused".into(),
        identity: content_digest(request).unwrap_or_default(),
        members: Vec::new(),
        approved_left: None,
        approved_right: None,
        denominator_per_side: PAIRS,
        threshold_percentage_points: request.threshold_percentage_points,
        refusals: Vec::new(),
    };
    let prepare =
        || -> Result<Vec<(String, PreparedReferenceReplay, SelectedTerminal, SelectedTerminal)>, ConformanceError> {
            if request.pairs.len() != PAIRS
                || !request.protected_case_pointer.starts_with('/')
                || !scalar(&request.left_value)
                || !scalar(&request.right_value)
                || request.left_value == request.right_value
                || !(1..=100).contains(&request.threshold_percentage_points)
            {
                return Err(refused(
                    "metric requires 100 explicit distinct-category pairs and a valid reviewed threshold",
                ));
            }
            let descriptor = paired_metric_descriptor(request)?;
            let mut common_native: Option<Value> = None;
            let mut ids = BTreeSet::new();
            let mut prepared = Vec::with_capacity(PAIRS * 2);
            for pair in &request.pairs {
                for member in [&pair.left, &pair.right] {
                    if member.id.trim().is_empty() || !ids.insert(member.id.clone()) {
                        return Err(refused("cohort member identity missing or duplicated"));
                    }
                }
                let left = prepare_reference_replay(
                    pair.left.request.clone(),
                    pair.left.adapters.clone(),
                )?;
                let right = prepare_reference_replay(
                    pair.right.request.clone(),
                    pair.right.adapters.clone(),
                )?;
                if paired_inputs(&left, &request.protected_case_pointer, &request.left_value)?
                    != paired_inputs(
                        &right,
                        &request.protected_case_pointer,
                        &request.right_value,
                    )?
                {
                    return Err(refused(
                        "paired original inputs differ outside the protected scalar",
                    ));
                }
                for (member, prepared_member) in [(&pair.left, left), (&pair.right, right)] {
                    source_review(&prepared_member, request, &descriptor)?;
                    let native = json!({"candidate":member.request.candidate_bytes,"dependencies":member.request.dependencies,"runtime":prepared_member.cohort_runtime_configuration()?});
                    if common_native
                        .as_ref()
                        .is_some_and(|common| common != &native)
                    {
                        return Err(refused(
                            "cohort members select different native candidate, dependencies or runtime",
                        ));
                    }
                    common_native = Some(native);
                    let favorable = final_state(
                        &member.request.candidate_bytes,
                        &request.favorable_state_pointer,
                    )?;
                    let adverse = final_state(
                        &member.request.candidate_bytes,
                        &request.adverse_state_pointer,
                    )?;
                    if favorable.id == adverse.id {
                        return Err(refused("selected terminal outcomes must differ"));
                    }
                    prepared.push((member.id.clone(), prepared_member, favorable, adverse));
                }
            }
            Ok(prepared)
        };
    let prepared = match prepare() {
        Ok(value) => value,
        Err(error) => {
            report.refusals.push(error.to_string());
            return report;
        }
    };
    let mut counts = [0_usize; 2];
    for (index, (id, prepared, favorable, adverse)) in prepared.into_iter().enumerate() {
        let mut member = CohortMemberResult {
            id,
            result: None,
            classification: None,
            refusal: None,
        };
        match replay_reference(&prepared) {
            Ok(result) => {
                match classify(&result, &favorable, &adverse) {
                    Ok(classification) => {
                        if classification == CohortClassification::Favorable {
                            counts[index % 2] += 1;
                        }
                        member.classification = Some(classification);
                    }
                    Err(error) => member.refusal = Some(error.to_string()),
                }
                member.result = Some(result);
            }
            Err(error) => member.refusal = Some(error.to_string()),
        }
        if let Some(reason) = &member.refusal {
            report.refusals.push(format!("{}: {reason}", member.id));
        }
        report.members.push(member);
    }
    if report.refusals.is_empty() {
        report.approved_left = Some(counts[0]);
        report.approved_right = Some(counts[1]);
        // Fixed denominator100 makes an integer count difference exactly percentage points.
        report.status =
            if counts[0].abs_diff(counts[1]) < request.threshold_percentage_points as usize {
                "pass"
            } else {
                "fail"
            }
            .into();
    }
    report
}

/// Opaque retained actual execution; callers cannot reconstruct it from JSON.
#[derive(Debug)]
pub struct NativeReplaySession {
    prepared: PreparedReferenceReplay,
    result: ReplayResult,
}
impl NativeReplaySession {
    pub(crate) fn prepared(&self) -> &PreparedReferenceReplay {
        &self.prepared
    }
    /// Returns immutable actual execution for inspection and review.
    pub fn result(&self) -> &ReplayResult {
        &self.result
    }
    /// Returns the original complete frozen input identity.
    pub fn identity(&self) -> &str {
        self.prepared.identity()
    }
}

/// Executes once and retains the original native prepared inputs and result.
///
/// # Errors
/// Returns the actual native execution error, without creating a review judgment.
pub fn begin_reference_replay(
    prepared: PreparedReferenceReplay,
) -> Result<NativeReplaySession, ConformanceError> {
    let result = replay_reference(&prepared)?;
    Ok(NativeReplaySession { prepared, result })
}

/// Narrow engineering claims; none certifies law, translation or accessibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineeringClaim {
    /// Both variants retain the selected structured facts.
    BilingualStructuredFacts,
    /// Explicit model review observations about plain wording.
    PlainLanguageEngineeringReview,
    /// Explicit model review observations about rendered structure.
    AccessibilityStructureReview,
}

/// Exact actual locale/body identity reviewed after execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewedNoticeVariant {
    /// Actual rendered locale.
    pub locale: String,
    /// SHA-256 of actual rendered UTF-8 body bytes.
    pub body_sha256: String,
}

/// Explicit model attribution; it never represents an agency or human approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineeringReviewer {
    /// Must be model.
    pub kind: String,
    /// Actual model/agent identity used by this engineering review.
    pub id: String,
    /// Explicit review method and scope description.
    pub method: String,
}

/// Explicit attributed engineering judgment, separate from native execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EngineeringVerdict {
    /// Reviewer supports this exact narrowly scoped engineering claim.
    Supported,
    /// Reviewer found contrary evidence.
    Refuted,
    /// Reviewer could not establish the claim.
    Inconclusive,
}
/// One recorded judgment with concrete observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineeringObservation {
    /// Explicit support, contrary evidence, or unresolved judgment.
    pub verdict: EngineeringVerdict,
    /// Concrete reviewer findings about actual content.
    pub observations: String,
}

/// Closed retained review of the exact actual notice artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineeringNoticeReview {
    /// Must be engineeringFixture.
    pub scope: String,
    /// Honest engineering reviewer attribution.
    pub reviewer: EngineeringReviewer,
    /// RFC3339 review time after artifact availability.
    pub reviewed_at: String,
    /// Full native artifact original-byte hash.
    pub artifact_sha256: String,
    /// Every actual rendered locale/body, exactly once.
    pub variants: Vec<ReviewedNoticeVariant>,
    /// Explicit narrowly scoped judgments.
    pub claims: Vec<EngineeringClaim>,
    /// Concrete review observation for every declared claim.
    pub observations: BTreeMap<EngineeringClaim, EngineeringObservation>,
    /// Visible unqualified source/publication/human-review limitations.
    pub limitations: Vec<String>,
}

/// Required post-run byte-bound review assertion for one actual notice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderedNoticeReviewAssertion {
    /// Actual native attempt and notice target.
    pub selector: NativeSelector,
    /// Independently required exact full artifact identity.
    pub artifact_sha256: String,
    /// Independently required exact locale/body identities.
    pub variants: Vec<ReviewedNoticeVariant>,
    /// Required engineering claims; no human claims are accepted by this route.
    pub required_claims: Vec<EngineeringClaim>,
    /// Original retained review bytes, never a pass flag.
    pub review_bytes: Vec<u8>,
    /// SHA-256 of those original review bytes.
    pub review_sha256: String,
    /// Explicit current comparison clock, distinct from replay execution time.
    pub comparison_clock_ms: u64,
}

/// Exact review comparison result, separate from actual replay execution.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedNoticeComparison {
    /// pass, fail, or refused; no unavailable judgment counts as passing.
    pub status: String,
    /// Original replay identity for this same retained session.
    pub replay_identity: String,
    /// Original exact retained review identity.
    pub review_sha256: String,
    /// Explicit comparison time.
    pub comparison_clock_ms: u64,
    /// Exact missing, stale or invalid review reason.
    pub refusals: Vec<String>,
    /// Explicit narrow engineering scope, never human approval.
    pub scope: String,
    /// Actual retained reviewer attribution when its document parses.
    pub reviewer: Option<EngineeringReviewer>,
    /// Original broader source claims remain unqualified.
    pub unqualified_claims: Vec<String>,
    /// Explicit contrary engineering judgments.
    pub failures: Vec<String>,
}

fn check_notice_review(
    result: &ReplayResult,
    assertion: &RenderedNoticeReviewAssertion,
) -> Result<bool, ConformanceError> {
    let NativeResolution::Resolved { pointer } = resolve_observation(result, &assertion.selector)
    else {
        return Err(refused("notice selector rejected"));
    };
    let document = serde_json::to_value(result).map_err(|error| refused(error.to_string()))?;
    let actual: NoticeState = serde_json::from_value(
        document
            .pointer(&pointer)
            .cloned()
            .ok_or_else(|| refused("actual notice missing"))?,
    )
    .map_err(|error| refused(format!("selected subject is not actual notice: {error}")))?;
    let artifact = base64::engine::general_purpose::STANDARD
        .decode(&actual.artifact_bytes_base64)
        .map_err(|error| refused(error.to_string()))?;
    if digest(&artifact) != actual.artifact_sha256
        || assertion.artifact_sha256 != actual.artifact_sha256
    {
        return Err(refused("actual notice artifact identity differs"));
    }
    let actual_variants: Vec<_> = actual
        .variants
        .iter()
        .map(|variant| ReviewedNoticeVariant {
            locale: variant.locale.clone(),
            body_sha256: variant.body_sha256.clone(),
        })
        .collect();
    if actual.variants.is_empty()
        || actual
            .variants
            .iter()
            .any(|variant| digest(variant.body.as_bytes()) != variant.body_sha256)
    {
        return Err(refused("actual rendered body identity differs"));
    }
    let variants=|values:&[ReviewedNoticeVariant]| -> Result<BTreeSet<ReviewedNoticeVariant>,ConformanceError> {
        let unique:BTreeSet<_>=values.iter().cloned().collect();
        let locales:BTreeSet<_>=values.iter().map(|value| &value.locale).collect();
        if values.is_empty() || unique.len()!=values.len() || locales.len()!=values.len() || values.iter().any(|value| value.locale.trim().is_empty()) {return Err(refused("notice variant coverage missing or duplicated"));}
        Ok(unique)
    };
    if variants(&actual_variants)? != variants(&assertion.variants)? {
        return Err(refused("asserted rendered variants differ"));
    }
    if digest(&assertion.review_bytes) != assertion.review_sha256 {
        return Err(refused("retained review byte identity differs"));
    }
    let review: EngineeringNoticeReview = serde_json::from_slice(&assertion.review_bytes)
        .map_err(|error| refused(error.to_string()))?;
    if review.scope != "engineeringFixture"
        || review.reviewer.kind != "model"
        || review.reviewer.id.trim().is_empty()
        || review.reviewer.method.trim().is_empty()
        || review.artifact_sha256 != actual.artifact_sha256
        || variants(&review.variants)? != variants(&actual_variants)?
        || review.limitations.is_empty()
        || review
            .limitations
            .iter()
            .any(|limit| limit.trim().is_empty())
    {
        return Err(refused(
            "engineering review attribution, scope, content or limitations invalid",
        ));
    }
    let required: BTreeSet<_> = assertion.required_claims.iter().copied().collect();
    let claims: BTreeSet<_> = review.claims.iter().copied().collect();
    if required.is_empty()
        || required.len() != assertion.required_claims.len()
        || claims.len() != review.claims.len()
        || claims != required
        || review.observations.len() != required.len()
        || required.iter().any(|claim| {
            review
                .observations
                .get(claim)
                .is_none_or(|finding| finding.observations.trim().is_empty())
        })
    {
        return Err(refused(
            "engineering review does not cover required explicit claims",
        ));
    }
    let issued = canonical_time(&actual.issue_at)?;
    let reviewed = canonical_time(&review.reviewed_at)?;
    let comparison = OffsetDateTime::from_unix_timestamp_nanos(
        i128::from(assertion.comparison_clock_ms) * 1_000_000,
    )
    .map_err(|error| refused(error.to_string()))?;
    if reviewed < issued || reviewed > comparison {
        return Err(refused(
            "engineering review time precedes artifact or exceeds comparison clock",
        ));
    }
    if review
        .observations
        .values()
        .any(|finding| finding.verdict == EngineeringVerdict::Inconclusive)
    {
        return Err(refused(
            "engineering review has an inconclusive required judgment",
        ));
    }
    Ok(review
        .observations
        .values()
        .all(|finding| finding.verdict == EngineeringVerdict::Supported))
}

/// Compares review with the same retained run and freshly prepared current inputs.
pub fn compare_rendered_notice(
    session: &NativeReplaySession,
    current: &PreparedReferenceReplay,
    assertion: &RenderedNoticeReviewAssertion,
) -> RenderedNoticeComparison {
    let result = if current.identity() != session.prepared.identity()
        || current.runtime_identity() != session.prepared.runtime_identity()
    {
        Err(refused(
            "current prepared inputs or native runtime differ from retained session",
        ))
    } else if !session.result.passed || session.result.status != "pass" {
        Err(refused("original native execution did not pass"))
    } else {
        check_notice_review(&session.result, assertion)
    };
    RenderedNoticeComparison {
        status: match &result {
            Ok(true) => "pass",
            Ok(false) => "fail",
            Err(_) => "refused",
        }
        .into(),
        replay_identity: session.identity().into(),
        review_sha256: assertion.review_sha256.clone(),
        comparison_clock_ms: assertion.comparison_clock_ms,
        failures: if matches!(&result, Ok(false)) {
            vec!["required engineering claim explicitly refuted".into()]
        } else {
            Vec::new()
        },
        scope: "engineeringFixture".into(),
        reviewer: serde_json::from_slice::<EngineeringNoticeReview>(&assertion.review_bytes)
            .ok()
            .map(|review| review.reviewer),
        unqualified_claims: vec![
            "substantiveLanguageEquivalence".into(),
            "section508Review".into(),
            "FNSApproval".into(),
            "legalSufficiency".into(),
            "publicationApproval".into(),
        ],
        refusals: result
            .err()
            .map(|error| vec![error.to_string()])
            .unwrap_or_default(),
    }
}
