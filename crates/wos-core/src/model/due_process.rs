// Rust guideline compliant 2026-02-21
//! Closed reference due-process configuration, commands, and retained runtime facts.
//!
//! These types describe `referenceDueProcess/1`. Deserialization establishes shape;
//! runtime admission establishes authority, causal evidence, and clock facts.

use serde::{Deserialize, Serialize};

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Bound native operation kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    /// IssueNotice value.
    IssueNotice,
    /// FileAppeal value.
    FileAppeal,
    /// ScheduleHearing value.
    ScheduleHearing,
    /// HoldHearing value.
    HoldHearing,
    /// RecordDisposition value.
    RecordDisposition,
    /// RequestServiceTermination value.
    RequestServiceTermination,
    /// ExtendPreNoticeWindow value.
    ExtendPreNoticeWindow,
    /// ApplyAdverseEffect value.
    ApplyAdverseEffect,
}

/// Closed native refusal reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefusalCode {
    /// UnsupportedProfile value.
    UnsupportedProfile,
    /// InvalidConfiguration value.
    InvalidConfiguration,
    /// UnboundTransition value.
    UnboundTransition,
    /// ActorRejected value.
    ActorRejected,
    /// TargetRejected value.
    TargetRejected,
    /// SelectionRejected value.
    SelectionRejected,
    /// InvocationRequired value.
    InvocationRequired,
    /// ReplayConflict value.
    ReplayConflict,
    /// SignatureRejected value.
    SignatureRejected,
    /// ExplanationRejected value.
    ExplanationRejected,
    /// NoticePending value.
    NoticePending,
    /// ClockRejected value.
    ClockRejected,
    /// ContinuationFrozen value.
    ContinuationFrozen,
    /// OperationStateRejected value.
    OperationStateRejected,
    /// OverrideRejected value.
    OverrideRejected,
    /// LimitReached value.
    LimitReached,
    /// StoreRejected value.
    StoreRejected,
}

/// Selected bounded execution profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceProfile {
    /// #[serde(rename value.
    #[serde(rename = "referenceDueProcess/1")]
    ReferenceDueProcessV1,
}

/// Admitted appeal channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    /// Phone value.
    Phone,
    /// #[serde(rename value.
    #[serde(rename = "in-person")]
    InPerson,
    /// Written value.
    Written,
}

/// Software fixture delivery channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryChannel {
    /// Mail value.
    Mail,
    /// Portal value.
    Portal,
}

/// Admitted local fixture evidence class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceClass {
    /// FixtureQualifiedReceipt value.
    FixtureQualifiedReceipt,
}

/// Supported appeal anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppealAnchor {
    /// NoticeIssued value.
    NoticeIssued,
}

/// Supported grace anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraceAnchor {
    /// QualifiedReceipt value.
    QualifiedReceipt,
}

/// Selected household choice requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationChoicePolicy {
    /// ElectionRequired value.
    ElectionRequired,
    /// DefaultUnlessWaived value.
    DefaultUnlessWaived,
}

/// Actual household continuation choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationChoice {
    /// Elect value.
    Elect,
    /// Waive value.
    Waive,
    /// Unspecified value.
    Unspecified,
}

/// Current protected service continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationStatus {
    /// Frozen value.
    Frozen,
    /// Waived value.
    Waived,
    /// Released value.
    Released,
}

/// Supported original service level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationScope {
    /// CurrentLevel value.
    CurrentLevel,
}

/// Selected continuation ending condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContinuationEnd {
    /// DispositionOrHouseholdTermination value.
    DispositionOrHouseholdTermination,
}

/// Canonical independence requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExcludedDeterminationActors {
    /// DeterminerAndSupervisor value.
    DeterminerAndSupervisor,
}

/// Actual required hearing milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HearingPurpose {
    /// HearingHeld value.
    HearingHeld,
}

/// Recorded hearing disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DispositionOutcome {
    /// Upheld value.
    Upheld,
    /// Reversed value.
    Reversed,
}

/// Local notice delivery state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeStatus {
    /// Pending value.
    Pending,
    /// Qualified value.
    Qualified,
    /// Blocked value.
    Blocked,
    /// Exhausted value.
    Exhausted,
}

/// Retained source categorization, without legal qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceAuthority {
    /// Statute value.
    Statute,
    /// Regulation value.
    Regulation,
    /// Policy value.
    Policy,
    /// Guideline value.
    Guideline,
}

/// Executed hypothetical alternative family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CounterfactualKind {
    /// Positive value.
    Positive,
    /// Negative value.
    Negative,
}

/// Reasoning provenance tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReasoningTier {
    /// Reasoning value.
    Reasoning,
}

/// Counterfactual provenance tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CounterfactualTier {
    /// Counterfactual value.
    Counterfactual,
}

/// Actual transport acceptance fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AcknowledgementKind {
    /// Accepted value.
    Accepted,
}

/// Actual qualified fixture receipt fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReceiptKind {
    /// QualifiedReceipt value.
    QualifiedReceipt,
}

/// Failure establishing no external effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    /// NoEffectFailure value.
    NoEffectFailure,
}

/// Supported deterministic fixture failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureCode {
    /// InjectedFailure value.
    InjectedFailure,
    /// AttemptsExhausted value.
    AttemptsExhausted,
}

/// Exact selected transition location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionLocation {
    /// JSON Pointer to the owning state in the selected workflow.
    #[serde(alias = "state", alias = "state_pointer")]
    pub state_pointer: String,
    /// Zero-based position of the transition in the source state's authored transition list.
    pub transition_index: usize,
}

/// Actual saved transition firing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FiringRef {
    /// Canonical SHA-256 digest of the immutable workflow that supplied the transition.
    pub definition_sha256: String,
    /// Owning state pointer and transition index captured by the evaluator at selection.
    pub location: TransitionLocation,
    /// ID of the actual saved state-transition provenance record for this firing.
    pub provenance_record_id: String,
}

/// Typed finite scalar in policy and native observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum Scalar {
    /// String value.
    String { value: String },
    /// Number value.
    Number { value: serde_json::Number },
    /// Boolean value.
    Boolean { value: bool },
}

/// Original retained source selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRef {
    /// Selected evidence ID used to locate the retained bytes with the required role.
    pub source_version: String,
    /// Section locator within the selected evidence.
    pub section_anchor: String,
    /// SHA-256 digest of the exact retained bytes addressed by this source reference.
    pub retained_sha256: String,
}

/// Original retained substantive review selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRef {
    /// ID of the selected retained review evidence.
    pub id: String,
    /// SHA-256 digest of the exact review bytes selected by this reference.
    pub sha256: String,
}

/// One scalar replacement in a private hypothetical snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseChange {
    /// JSON Pointer to the existing scalar field in the determination snapshot to vary.
    pub case_pointer: String,
    /// Replacement scalar used only in the isolated counterfactual evaluation.
    pub value: Scalar,
}

/// Selected source-linked hypothetical alternative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseAlternative {
    /// Authored identifier for this alternative within the selected explanation.
    pub id: String,
    /// Scalar replacements evaluated together on a private clone of the retained snapshot.
    pub changes: Vec<CaseChange>,
    /// Authored policy text describing this hypothetical set of case changes, copied verbatim into
    /// counterfactual provenance; it is not an evaluation result.
    pub description: String,
    /// Retained sources grounding the selection of this alternative and its changes.
    pub source_refs: Vec<SourceRef>,
}

/// Policy text linked to an actual selected table row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReasonBinding {
    /// Identifier of the native decision table whose selected row supplies the reason.
    pub table_ref: String,
    /// Identifier of the authored decision table row associated with this reason binding.
    pub row_id: String,
    /// Authored reason identifier recorded as the reasoning rule ID for this row.
    pub reason_code: String,
    /// Classification of the policy authority that supports this reason.
    pub source_authority: SourceAuthority,
    /// Explanation of the authored reason associated with a decision table row.
    pub description: String,
    /// Retained sources supporting the reason text and its claimed authority.
    pub source_refs: Vec<SourceRef>,
}

/// Explicit source-backed protected scalar selected for isolated negative evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedCharacteristicBinding {
    /// Case-state JSON pointer identifying the selected protected scalar.
    pub case_pointer: String,
    /// Exact retained source evidence supporting this selection.
    pub source_refs: Vec<SourceRef>,
}

/// Selected native causal explanation requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExplanationBinding {
    /// Selected protected scalars; rights-impacting execution requires isolated evidence for each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_characteristics: Vec<ProtectedCharacteristicBinding>,
    /// Identifier of the native decision table used by the determining transition's guard.
    pub table_ref: String,
    /// Output column of the selected decision table used to compare alternative outcomes.
    pub output_column: String,
    /// Result that the selected output column must produce in a successful positive alternative.
    pub beneficial_output: Scalar,
    /// Reason bindings keyed by the actual row selected by the native decision table.
    pub reasons: Vec<ReasonBinding>,
    /// Alternatives evaluated for a different outcome and the configured beneficial result.
    pub positive: Vec<CaseAlternative>,
    /// Isolated protected-value changes that each preserve the original output and adverse guard
    /// result.
    pub negative: Vec<CaseAlternative>,
}

/// Exactly bound adverse transition and protected service values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationBinding {
    /// Authored identifier used by operation bindings and retained determination records.
    pub id: String,
    /// Exact state and transition index that produces this determination when it fires.
    pub transition: TransitionLocation,
    /// Workflow pointer to the determining transition's immediate target before adverse effect.
    pub pending_state_pointer: String,
    /// Workflow state pointers that may be entered through the authorized adverse-effect operation.
    pub applied_state_pointers: Vec<String>,
    /// Case-state pointers whose retained values are protected until adverse effect is admitted.
    pub service_pointers: Vec<String>,
    /// Workflow task reference for the verified signature bound to this exact determination.
    pub signature_task_ref: String,
    /// Decision table, reasons and counterfactuals used to produce the determination explanation.
    pub explanation: ExplanationBinding,
    /// Retained sources grounding the selected determination binding.
    pub source_refs: Vec<SourceRef>,
}

/// Actor-admitted operation with an exact lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationBinding {
    /// Authored operation identifier supplied by requests and recorded on successful operations.
    pub id: String,
    /// Governed command or workflow-owned action this binding authorizes.
    pub kind: OperationKind,
    /// Authored determination binding ID this operation applies to.
    pub determination_ref: String,
    /// Declared workflow actors eligible for this operation, subject to native authorization.
    pub actors: Vec<String>,
    /// Workflow pointers to active states in which the operation may run.
    pub required_state_pointers: Vec<String>,
    /// Workflow event that triggers the transition associated with this governance operation
    /// binding.
    pub trigger_event: String,
    /// Exact workflow transition location that the trusted operation must select.
    pub transition: TransitionLocation,
    /// Retained sources grounding the operation and its preconditions.
    pub source_refs: Vec<SourceRef>,
}

/// One selected locale with retained template review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeTemplateSelection {
    /// Locale tag used to select the notice template for a required language variant.
    pub locale: String,
    /// Key of the selected Delivery template to render for this locale.
    pub template_key: String,
    /// Reference to the retained review evidence for this selected template variant.
    pub review: ReviewRef,
}

/// Qualified local fixture notice delivery requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticePolicy {
    /// JSON Pointer used to read the recipient from the retained determination snapshot.
    pub recipient_pointer: String,
    /// Selected channel for staging notice work and using the fixture transport.
    pub delivery_channel: DeliveryChannel,
    /// Ordered Delivery template variants with review evidence, including the primary template
    /// first.
    pub templates: Vec<NoticeTemplateSelection>,
    /// Required class of receipt evidence before the grace clock can qualify.
    pub evidence_class: EvidenceClass,
    /// Notice issuance is the starting point for the appeal window.
    pub appeal_anchor: AppealAnchor,
    /// Qualified receipt is the starting point for the adverse-effect grace period.
    pub grace_anchor: GraceAnchor,
    /// Retained sources grounding the selected notice policy.
    pub source_refs: Vec<SourceRef>,
}

/// Whole UTC calendar days to an actual held hearing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HearingDeadline {
    /// Maximum days from admission of the appeal to holding the hearing.
    pub calendar_days: u32,
    /// Hearing milestone that must occur by the selected deadline.
    pub purpose: HearingPurpose,
}

/// Selected continuation of original protected services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuationPolicy {
    /// Rule for when the household must elect continuation or may waive it.
    pub choice: ContinuationChoicePolicy,
    /// Current level of services protected while the continuation rule applies.
    pub scope: ContinuationScope,
    /// Disposition or permitted household termination ends frozen continuation.
    pub ends_on: ContinuationEnd,
}

/// Standing, canonical independence, and hearing requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealPolicy {
    /// Filing channels available for appeals under this selected policy.
    pub channels: Vec<Channel>,
    /// Declared household actors eligible to file an appeal, subject to identity and access checks.
    pub standing_actors: Vec<String>,
    /// Declared reviewer actors eligible to act, subject to canonical independence checks.
    pub reviewer_actors: Vec<String>,
    /// Independence rule excluding the determining human and the signing supervisor from review.
    pub excluded_determination_actors: ExcludedDeterminationActors,
    /// Maximum calendar days from filing to the actual holding of the hearing.
    pub hearing_deadline: HearingDeadline,
    /// Selected rule for retaining current services while the appeal is pending.
    pub continuation: ContinuationPolicy,
    /// Retained sources grounding the selected appeal and service-continuation terms.
    pub source_refs: Vec<SourceRef>,
}

/// Bound pre-notice timer extension limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionPolicy {
    /// Identifier of the native pre-notice timer whose existing deadline may be extended.
    pub timer_id: String,
    /// Workflow state pointers in which the pre-notice extension may be admitted.
    pub allowed_state_pointers: Vec<String>,
    /// Cap on the sum of extensions accepted for the retained timer.
    pub max_cumulative_calendar_days: u32,
    /// Retained sources grounding the pre-notice extension and its cumulative limit.
    pub source_refs: Vec<SourceRef>,
}

/// Bounded retained reference execution resource use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    /// Maximum number of determination records the native governor may retain for one process.
    pub max_determinations: usize,
    /// Maximum number of notice records the native governor may retain for one process.
    pub max_notices: usize,
    /// Maximum number of appeal records the native governor may retain for one process.
    pub max_appeals: usize,
    /// Maximum number of operation records the native governor may retain for one process.
    pub max_operations: usize,
    /// Maximum retained notices without a qualified receipt before the runtime refuses to stage another.
    pub max_outstanding_notice_work: usize,
    /// Maximum number of transport attempts permitted for a single retained notice.
    pub max_transport_attempts: usize,
    /// Maximum size in bytes of one rendered notice artifact admitted by the native governor.
    pub max_notice_bytes: usize,
}

/// Selected complete reference due-process execution policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessExecution {
    /// Versioned marker for the supported native due-process execution profile.
    pub profile: ReferenceProfile,
    /// Exact bindings for reachable adverse-decision transitions in the selected workflow.
    pub determinations: Vec<DeterminationBinding>,
    /// Governed operations bound to their actual workflow transitions.
    pub operations: Vec<OperationBinding>,
    /// Selected policy for rendering, staging and qualifying an adverse-decision notice.
    pub notice: NoticePolicy,
    /// Selected policy for appeals, independent hearings and continuation of services.
    pub appeal: AppealPolicy,
    /// Policy authorizing bounded extensions of the retained pre-notice timer, when configured.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub pre_notice_extension: Option<ExtensionPolicy>,
    /// Bounds for retained records, outstanding work, transport attempts and rendered notice bytes.
    pub limits: Limits,
}

/// Declared slot and independently admitted actual actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceActor {
    /// Workflow actor slot checked against the request's canonical acting identity.
    pub declared_actor: String,
    /// Independently admitted actual actor: the retained canonical human for a human role, or the
    /// exact declared nonhuman actor otherwise. A request cannot authenticate its own actor.
    pub acting_actor: String,
}

/// Closed command data; timestamps never establish operation time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum GovernanceCommand {
    /// IssueNotice value.
    IssueNotice,
    /// FileAppeal value.
    FileAppeal {
        channel: Channel,
        continuation_choice: ContinuationChoice,
    },
    /// ScheduleHearing value.
    ScheduleHearing {
        appeal_id: String,
        reviewer_actor: String,
        scheduled_for: String,
    },
    /// HoldHearing value.
    HoldHearing {
        appeal_id: String,
        evidence_refs: Vec<SourceRef>,
    },
    /// RecordDisposition value.
    RecordDisposition {
        appeal_id: String,
        outcome: DispositionOutcome,
        rationale: String,
        evidence_refs: Vec<SourceRef>,
    },
    /// RequestServiceTermination value.
    RequestServiceTermination {
        appeal_id: String,
        rationale: String,
    },
    /// ExtendPreNoticeWindow value.
    ExtendPreNoticeWindow {
        calendar_days: u32,
        rationale: String,
        evidence_refs: Vec<SourceRef>,
    },
    /// ApplyAdverseEffect value.
    ApplyAdverseEffect,
}

/// Public actor-admitted idempotent native operation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceRequest {
    /// Authored operation binding ID that the native dispatcher must resolve.
    pub operation_ref: String,
    /// Saved determination ID targeted by the request. Omitted for a pre-notice extension.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub determination_id: Option<String>,
    /// Process-local token for replaying the exact accepted request and its original result.
    pub idempotency_token: String,
    /// Declared workflow actor and canonical acting identity submitted for native admission.
    pub actor: GovernanceActor,
    /// Typed operation arguments the dispatcher checks against the selected operation binding.
    pub command: GovernanceCommand,
}

/// Actual committed operation outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum GovernanceOutcome {
    /// NoticeStaged value.
    NoticeStaged { notice_id: String },
    /// AppealFiled value.
    AppealFiled { appeal_id: String },
    /// HearingScheduled value.
    HearingScheduled {
        appeal_id: String,
        scheduled_for: String,
    },
    /// HearingHeld value.
    HearingHeld { appeal_id: String, held_at: String },
    /// DispositionRecorded value.
    DispositionRecorded {
        appeal_id: String,
        outcome: DispositionOutcome,
    },
    /// ServiceTerminationRequested value.
    ServiceTerminationRequested { appeal_id: String },
    /// PreNoticeWindowExtended value.
    PreNoticeWindowExtended {
        override_record_id: String,
        new_deadline: String,
    },
    /// AdverseEffectApplied value.
    AdverseEffectApplied { applied_at: String },
}

/// Committed saved transition or a native refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
pub enum GovernanceResult {
    /// Committed value.
    Committed {
        operation_record_id: String,
        outcome: GovernanceOutcome,
    },
    /// Refused value.
    Refused { code: RefusalCode },
}

/// Actual original immutable determination snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotRef {
    /// Actual provenance record that owns the retained pre-transition snapshot.
    pub record_id: String,
    /// SHA-256 of the canonical retained snapshot, encoded as lowercase hexadecimal.
    pub sha256: String,
}

/// Accepted supervisor signature associated with exact actual content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationSignature {
    /// Identity of the original task that supplied the admitted signature for this determination.
    pub task_id: String,
    /// Provenance record ID of the accepted, verified SignatureAffirmation.
    pub affirmation_record_id: String,
    /// Canonical human identity established by the accepted supervisor signature.
    pub supervisor_human: String,
    /// SHA-256 digest of the full signed Formspec Response payload.
    pub signed_content_sha256: String,
}

/// Actual bound applied effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdverseEffect {
    /// Provenance ID of the actual admitted transition that applied the adverse effect.
    pub operation_record_id: String,
    /// Time the adverse-effect operation applied the previously retained determination.
    pub applied_at: String,
}

/// Actual saved pending adverse determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationState {
    /// Actual determining transition's saved provenance record ID.
    pub id: String,
    /// Authored determination binding ID used by the actual determining transition.
    pub binding_ref: String,
    /// Exact location, workflow digest and provenance identity of the determining transition.
    pub firing: FiringRef,
    /// Time of the actual determining transition.
    pub determined_at: String,
    /// Reference to the pre-transition case snapshot that supports this determination.
    pub snapshot: SnapshotRef,
    /// Canonical human whose admitted transition produced the determination.
    pub determining_human: String,
    /// Protected case-state fields retained from the selected determination binding.
    pub service_pointers: Vec<String>,
    /// Verified signature bound to this determination and its retained Formspec Response.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub signature: Option<DeterminationSignature>,
    /// Saved operation and time when the adverse effect was applied, if it occurred.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub effect: Option<AdverseEffect>,
}

/// Immutable rendered notice locale and source review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeVariant {
    /// Locale tag identifying the language variant represented by the rendered notice body.
    pub locale: String,
    /// Key of the selected Delivery template that produced the rendered body.
    pub template_key: String,
    /// Retained review evidence selected for this template and locale.
    pub review: ReviewRef,
    /// Original rendered body text for this locale in the retained notice artifact.
    pub body: String,
    /// SHA-256 digest of the exact rendered body bytes for this locale.
    pub body_sha256: String,
}

/// Actual fixture transport acceptance, without delivery qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransportAcknowledgement {
    /// Discriminator for a fixture transport report of acceptance.
    pub kind: AcknowledgementKind,
    /// Notice ID reported by the fixture transport when it accepted the work.
    pub notice_id: String,
    /// Stable key for execution and recovery of the exact staged transport work.
    pub transport_key: String,
    /// Correlation token that must match the staged notice and its transport work.
    pub correlation: String,
    /// SHA-256 digest of the staged notice bytes acknowledged by the transport.
    pub artifact_sha256: String,
    /// Transport channel reported for the accepted notice work.
    pub channel: DeliveryChannel,
    /// Acceptance time reported by the fixture transport.
    pub accepted_at: String,
    /// Adapter-assigned identity for the retained successful transport result.
    pub adapter_result_id: String,
}

/// Actual same-instant qualified fixture receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualifiedNoticeReceipt {
    /// Discriminator for the fixture transport's receipt claim.
    pub kind: ReceiptKind,
    /// Staged notice ID reported in the fixture transport receipt.
    pub notice_id: String,
    /// Transport work key reported in the fixture transport receipt.
    pub transport_key: String,
    /// Correlation token the fixture transport reports for this receipt.
    pub correlation: String,
    /// SHA-256 digest the fixture transport reports for the staged notice bytes.
    pub artifact_sha256: String,
    /// Transport channel reported in the fixture transport receipt.
    pub channel: DeliveryChannel,
    /// Evidence class reported by the fixture transport for this receipt.
    pub evidence_class: EvidenceClass,
    /// Qualification time reported by the fixture transport for this receipt.
    pub qualified_at: String,
    /// Result ID the fixture transport reports for this receipt.
    pub adapter_result_id: String,
}

/// Actual no-effect fixture failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransportFailure {
    /// Discriminator indicating a confirmed failure without an external effect.
    pub kind: FailureKind,
    /// Failure reason distinguishing an injected no-effect failure from exhausted attempts.
    pub code: FailureCode,
}

/// Actual immutable fixture transport outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum TransportResult {
    /// AcceptedOnly value.
    AcceptedOnly {
        acknowledgement: TransportAcknowledgement,
    },
    /// Qualified value.
    Qualified {
        acknowledgement: TransportAcknowledgement,
        receipt: QualifiedNoticeReceipt,
    },
    /// Failed value.
    Failed { failure: TransportFailure },
}

/// Immutable saved notice bytes, anchors, and monotonic delivery evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeState {
    /// Saved state-transition provenance ID of the operation that staged this notice.
    pub id: String,
    /// Saved determination ID that this rendered notice addresses.
    pub determination_id: String,
    /// SHA-256 digest of the exact rendered notice artifact retained for delivery.
    pub artifact_sha256: String,
    /// Exact rendered notice artifact bytes encoded as canonical standard base64.
    pub artifact_bytes_base64: String,
    /// Time the issue operation staged the notice. Only an admitted qualified receipt establishes
    /// issuance at this time.
    pub issue_at: String,
    /// Filing deadline computed at staging and effective once issuance qualifies.
    pub appeal_deadline: String,
    /// Proposed adverse-effect threshold computed from staging time and grace, usable only after
    /// qualification.
    pub effect_eligible_at: String,
    /// Exact nonblank recipient value resolved from the notice policy's recipientPointer in the
    /// retained determination snapshot.
    pub recipient: String,
    /// Selected fixture channel retained with the immutable staged notice.
    pub channel: DeliveryChannel,
    /// Reviewed locale variants rendered into the retained notice artifact.
    pub variants: Vec<NoticeVariant>,
    /// Stable transport work key saved with the staged notice, reused on retries.
    pub transport_key: String,
    /// Token binding transport results to this staged notice.
    pub correlation: String,
    /// Current delivery state after staging, transport admission or attempt exhaustion.
    pub status: NoticeStatus,
    /// Count of transport attempts already recorded for this notice in native execution.
    pub attempts: usize,
    /// Actual transport acceptances retained against this notice.
    pub acknowledgements: Vec<TransportAcknowledgement>,
    /// Distinct actual typed results in first saved runtime observation order.
    ///
    /// These diagnostics grant no delivery qualification or invocation count.
    pub transport_results: Vec<TransportResult>,
    /// Admitted fixture receipt that qualifies the notice and anchors the grace period.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub receipt: Option<QualifiedNoticeReceipt>,
    /// Retained no-effect failure or exhausted-attempts status, if recorded.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub failure: Option<TransportFailure>,
}

/// Actual independent-reviewer hearing schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HearingSchedule {
    /// Saved transition provenance ID of the operation that scheduled the hearing.
    pub operation_record_id: String,
    /// Actual time the schedule operation committed.
    pub scheduled_at: String,
    /// Hearing appointment time chosen in the accepted scheduling operation.
    pub scheduled_for: String,
    /// Declared workflow actor selected to conduct the hearing.
    pub reviewer_actor: String,
    /// Canonical human bound to the selected independent reviewer.
    pub reviewer_human: String,
}

/// Actual held-hearing evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeldHearing {
    /// Saved transition provenance ID of the operation that held the hearing.
    pub operation_record_id: String,
    /// Actual recorded time of the admitted hearing.
    pub held_at: String,
    /// Canonical human who conducted the hearing after the independence check.
    pub reviewer_human: String,
    /// Retained hearing evidence submitted for the admitted hearing operation.
    pub evidence_refs: Vec<SourceRef>,
}

/// Actual held-hearing disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Disposition {
    /// Saved transition provenance ID of the operation that recorded the disposition.
    pub operation_record_id: String,
    /// Time the independent reviewer's disposition was recorded.
    pub recorded_at: String,
    /// Whether the disposition upheld or reversed the determination.
    pub outcome: DispositionOutcome,
    /// Reason retained with the disposition to explain the reviewer's decision on the appeal.
    pub rationale: String,
    /// Retained hearing evidence cited in the disposition.
    pub evidence_refs: Vec<SourceRef>,
}

/// Actual admitted household request to end continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HouseholdTermination {
    /// Saved transition provenance ID of the accepted service-termination request.
    pub operation_record_id: String,
    /// Time of the accepted household request to terminate service continuation.
    pub requested_at: String,
    /// Canonical human whose household authority was checked for the request.
    pub requested_by: String,
    /// Reason retained with the household's completed request to end continuation of service.
    pub rationale: String,
}

/// Saved timely appeal and separate hearing milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealState {
    /// ID of the saved AppealFiled provenance record for the admitted appeal.
    pub id: String,
    /// Saved determination ID challenged by this appeal.
    pub determination_id: String,
    /// ID of the issued notice from which appeal eligibility was checked.
    pub notice_id: String,
    /// Actual time the appeal filing was admitted.
    pub filed_at: String,
    /// Canonical human whose standing and access were checked for the filing.
    pub filed_by: String,
    /// Selected filing channel actually used for the admitted appeal.
    pub channel: Channel,
    /// Household election, waiver or unspecified choice recorded at filing.
    pub continuation_choice: ContinuationChoice,
    /// Current service-continuation status under the selected policy and saved milestones.
    pub continuation: ContinuationStatus,
    /// Deadline for actually holding the hearing, calculated from the admitted filing time.
    pub hearing_deadline: String,
    /// Accepted appointment and independent reviewer for the hearing, if scheduled.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub schedule: Option<HearingSchedule>,
    /// Saved reviewer, time and retained evidence from the actual hearing, if held.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub hearing: Option<HeldHearing>,
    /// Saved disposition and evidence from the independent reviewer, if recorded.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub disposition: Option<Disposition>,
    /// Saved authorized household request to end frozen continuation, if recorded.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub termination: Option<HouseholdTermination>,
}

/// Canonical override rationale summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverrideRationale {
    /// Human-readable rationale supplied for the accepted extension.
    pub summary: String,
}

/// Actual role authority verification for an override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverrideAuthorityVerification {
    /// Canonical identity whose authority to extend the timer was verified.
    pub actor_id: String,
    /// How the runtime verified the actor's override authority.
    pub method: OverrideAuthorityMethod,
    /// Time the runtime verified the actor's override authority.
    pub verified_at: String,
    /// Workflow definition URI and declared actor role used to verify authority.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub authority_source: Option<String>,
}

/// Supported canonical role authority verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverrideAuthorityMethod {
    /// RoleAssignment value.
    RoleAssignment,
}

/// Canonical retained supporting document reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverrideEvidenceReference {
    /// Category of the artifact supporting the accepted override.
    pub kind: OverrideEvidenceKind,
    /// Reference URI identifying the retained supporting document by its digest.
    pub uri: String,
    /// Optional human-readable description of what the cited evidence shows.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub summary: Option<String>,
}

/// Retained supporting document evidence kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverrideEvidenceKind {
    /// Document value.
    Document,
}

/// Canonical Governance §7.3 override record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverrideRecord {
    /// Identity minted for this canonical override record.
    pub id: String,
    /// Identity of the governed case ledger to which the override belongs.
    pub case_id: String,
    /// Human rationale for the override, supplied with the accepted command.
    pub rationale: OverrideRationale,
    /// Saved identity and role verification that authorized the override.
    pub authority_verification: OverrideAuthorityVerification,
    /// References to the retained documents cited by the accepted override.
    pub supporting_evidence: Vec<OverrideEvidenceReference>,
}

/// Actual timer extension and canonical override evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionState {
    /// Saved transition provenance ID of the accepted extension operation.
    pub operation_record_id: String,
    /// Authored determination binding to which the extension operation belongs.
    pub determination_binding_ref: String,
    /// Identifier of the actual retained timer whose deadline was extended.
    pub timer_id: String,
    /// Calendar days added to the timer deadline by this operation.
    pub calendar_days: u32,
    /// Total accepted extension days for this timer including this operation.
    pub cumulative_calendar_days: u32,
    /// Retained timer deadline immediately before this extension.
    pub prior_deadline: String,
    /// Prior deadline plus the calendar days accepted for this extension.
    pub new_deadline: String,
    /// Canonical OverrideRecord emitted for the extension and retained in operation state.
    pub override_record: OverrideRecord,
}

/// Saved actual bound operation transition and outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationState {
    /// Actual saved state-transition provenance ID for the accepted operation.
    pub record_id: String,
    /// Authored binding ID used to admit and dispatch the operation.
    pub operation_ref: String,
    /// Saved determination targeted by the operation, when it has a determination target.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub determination_id: Option<String>,
    /// Selected native operation executed at the bound transition.
    pub kind: OperationKind,
    /// Time of the actual state transition that performed the operation.
    pub performed_at: String,
    /// Declared actor and canonical identity admitted for the operation.
    pub actor: GovernanceActor,
    /// Exact selected transition and provenance ID of the recorded operation firing.
    pub firing: FiringRef,
    /// Typed result of the already committed native operation.
    pub outcome: GovernanceOutcome,
}

/// Immutable exact successful public request replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayState {
    /// Exact accepted public request retained for idempotent replay.
    pub request: GovernanceRequest,
    /// Original committed result returned on exact replay of the accepted request.
    pub result: GovernanceResult,
}

/// Retained native due-process facts for one process.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessState {
    /// Retained determination facts, snapshot references and accepted signature or effect
    /// milestones.
    pub determinations: Vec<DeterminationState>,
    /// Retained rendered notices and their delivery attempts, results and clock anchors.
    pub notices: Vec<NoticeState>,
    /// Admitted appeals and their continuation, hearing and disposition milestones.
    pub appeals: Vec<AppealState>,
    /// Accepted pre-notice timer extensions and their canonical override records.
    pub extensions: Vec<ExtensionState>,
    /// Committed native operations and their actual transition provenance links.
    pub operations: Vec<OperationState>,
    /// Exact accepted public requests and original results retained for idempotent replay.
    pub replays: Vec<ReplayState>,
}

/// Complete immutable transport work retained before invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingNoticeWork {
    /// Identity of the native process that owns this pending item of notice transport work.
    pub process_id: String,
    /// ID of the staged notice whose exact bytes are submitted to the adapter.
    pub notice_id: String,
    /// Saved determination ID bound to the staged notice work.
    pub determination_id: String,
    /// Stable key used with the process ID to look up and recover transport results.
    pub transport_key: String,
    /// Token the adapter result must echo to bind it to this staged work.
    pub correlation: String,
    /// SHA-256 digest of the exact staged notice artifact supplied to the adapter.
    pub artifact_sha256: String,
    /// Rendered notice artifact bytes supplied to transport as canonical standard base64.
    pub artifact_bytes_base64: String,
    /// Exact recipient value of the issued notice that this pending transport work must deliver.
    pub recipient: String,
    /// Fixture channel selected for the staged notice work.
    pub channel: DeliveryChannel,
    /// Saved time the notice was originally staged for transport.
    pub issue_at: String,
}

/// One actual native table input or output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedScalar {
    /// Name of the decision-table input or output represented by this value.
    pub name: String,
    /// Scalar actually observed for the named decision-table input or output.
    pub value: Scalar,
}

/// Actual native decision-table guard observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDecisionEvaluation {
    /// Exact workflow state and transition index where the guard was evaluated.
    pub location: TransitionLocation,
    /// Identifier of the native decision table invoked by the guard.
    pub table_ref: String,
    /// Concrete input names and values passed to the native decision table.
    pub inputs: Vec<NamedScalar>,
    /// IDs of the rows the decision table actually matched during evaluation.
    pub matched_row_ids: Vec<String>,
    /// Identifier of the decision table row selected by the retained native evaluation.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub selected_row_id: Option<String>,
    /// Concrete output names and values returned by the native decision table.
    pub outputs: Vec<NamedScalar>,
    /// Actual Boolean result of the transition guard in the retained native evaluation.
    pub guard_result: bool,
}

/// Source-linked factual selected-row explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationReasoningData {
    /// Audit tier that identifies this native determination reasoning payload.
    pub tier: ReasoningTier,
    /// Identity of the saved determination whose firing the retained reasoning evidence explains.
    pub related_transition: String,
    /// Selected reason code corresponding to the actual matched decision-table row.
    pub rule_id: String,
    /// Source authority recorded from the selected reason binding.
    pub authority: SourceAuthority,
    /// The selected row's reason binding description, copied verbatim as policy text; actual inputs
    /// and outputs are retained separately in evaluation.
    pub description: String,
    /// Timestamp of the determining transition this reasoning explains.
    pub timestamp: String,
    /// Actual decision-table evaluation whose selected row supplied the reason.
    pub evaluation: NativeDecisionEvaluation,
    /// Retained sources carried from the selected reason binding.
    pub source_refs: Vec<SourceRef>,
}

/// Actual private hypothetical evaluation with immutable snapshot association.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationCounterfactualData {
    /// Audit tier that identifies this native determination counterfactual payload.
    pub tier: CounterfactualTier,
    /// Identity of the saved determination whose firing the counterfactual evidence describes.
    pub related_transition: String,
    /// Positive outcome change or negative outcome preservation verified for this alternative.
    #[serde(rename = "type")]
    pub kind: CounterfactualKind,
    /// Authored alternative ID copied into the counterfactual factor field.
    pub factor: String,
    /// The selected alternative's authored description, copied verbatim; the evaluation results are
    /// retained separately in originalEvaluation and alternateEvaluation.
    pub description: String,
    /// Selected alternative and its scalar changes applied to a private snapshot clone.
    pub alternative: CaseAlternative,
    /// Lowercase hexadecimal SHA-256 of the original retained determination snapshot.
    pub original_snapshot_sha256: String,
    /// Lowercase hexadecimal SHA-256 of the snapshot with the alternative applied.
    pub alternate_snapshot_sha256: String,
    /// Actual decision-table observation for the original determination.
    pub original_evaluation: NativeDecisionEvaluation,
    /// Decision-table observation obtained after evaluating the isolated alternative.
    pub alternate_evaluation: NativeDecisionEvaluation,
    /// Retained sources carried from the selected counterfactual alternative.
    pub source_refs: Vec<SourceRef>,
}

impl GovernanceCommand {
    /// Returns the exactly corresponding configured operation kind.
    #[must_use]
    pub const fn operation_kind(&self) -> OperationKind {
        match self {
            Self::IssueNotice => OperationKind::IssueNotice,
            Self::FileAppeal { .. } => OperationKind::FileAppeal,
            Self::ScheduleHearing { .. } => OperationKind::ScheduleHearing,
            Self::HoldHearing { .. } => OperationKind::HoldHearing,
            Self::RecordDisposition { .. } => OperationKind::RecordDisposition,
            Self::RequestServiceTermination { .. } => OperationKind::RequestServiceTermination,
            Self::ExtendPreNoticeWindow { .. } => OperationKind::ExtendPreNoticeWindow,
            Self::ApplyAdverseEffect => OperationKind::ApplyAdverseEffect,
        }
    }
}
impl Scalar {
    /// Returns the exact JSON scalar represented by this typed value.
    #[must_use]
    pub fn value(&self) -> serde_json::Value {
        match self {
            Self::String { value } => serde_json::Value::String(value.clone()),
            Self::Number { value } => serde_json::Value::Number(value.clone()),
            Self::Boolean { value } => serde_json::Value::Bool(*value),
        }
    }
    /// Admits a scalar JSON value, refusing null and structured values.
    #[must_use]
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(value) if value.len() <= 4096 => Some(Self::String {
                value: value.clone(),
            }),
            serde_json::Value::Number(value) => Some(Self::Number {
                value: value.clone(),
            }),
            serde_json::Value::Bool(value) => Some(Self::Boolean { value: *value }),
            _ => None,
        }
    }
}
