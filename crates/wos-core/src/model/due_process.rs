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
    /// Retained `state_pointer` value.
    pub state_pointer: String,
    /// Retained `transition_index` value.
    pub transition_index: usize,
}

/// Actual saved transition firing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FiringRef {
    /// Retained `definition_sha256` value.
    pub definition_sha256: String,
    /// Retained `location` value.
    pub location: TransitionLocation,
    /// Retained `provenance_record_id` value.
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
    /// Retained `source_version` value.
    pub source_version: String,
    /// Retained `section_anchor` value.
    pub section_anchor: String,
    /// Retained `retained_sha256` value.
    pub retained_sha256: String,
}

/// Original retained substantive review selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRef {
    /// Retained `id` value.
    pub id: String,
    /// Retained `sha256` value.
    pub sha256: String,
}

/// One scalar replacement in a private hypothetical snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseChange {
    /// Retained `case_pointer` value.
    pub case_pointer: String,
    /// Retained `value` value.
    pub value: Scalar,
}

/// Selected source-linked hypothetical alternative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseAlternative {
    /// Retained `id` value.
    pub id: String,
    /// Retained `changes` value.
    pub changes: Vec<CaseChange>,
    /// Retained `description` value.
    pub description: String,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Policy text linked to an actual selected table row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReasonBinding {
    /// Retained `table_ref` value.
    pub table_ref: String,
    /// Retained `row_id` value.
    pub row_id: String,
    /// Retained `reason_code` value.
    pub reason_code: String,
    /// Retained `source_authority` value.
    pub source_authority: SourceAuthority,
    /// Retained `description` value.
    pub description: String,
    /// Retained `source_refs` value.
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
    /// Retained `table_ref` value.
    pub table_ref: String,
    /// Retained `output_column` value.
    pub output_column: String,
    /// Retained `beneficial_output` value.
    pub beneficial_output: Scalar,
    /// Retained `reasons` value.
    pub reasons: Vec<ReasonBinding>,
    /// Retained `positive` value.
    pub positive: Vec<CaseAlternative>,
    /// Retained `negative` value.
    pub negative: Vec<CaseAlternative>,
}

/// Exactly bound adverse transition and protected service values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationBinding {
    /// Retained `id` value.
    pub id: String,
    /// Retained `transition` value.
    pub transition: TransitionLocation,
    /// Retained `pending_state_pointer` value.
    pub pending_state_pointer: String,
    /// Retained `applied_state_pointers` value.
    pub applied_state_pointers: Vec<String>,
    /// Retained `service_pointers` value.
    pub service_pointers: Vec<String>,
    /// Retained `signature_task_ref` value.
    pub signature_task_ref: String,
    /// Retained `explanation` value.
    pub explanation: ExplanationBinding,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Actor-admitted operation with an exact lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationBinding {
    /// Retained `id` value.
    pub id: String,
    /// Retained `kind` value.
    pub kind: OperationKind,
    /// Retained `determination_ref` value.
    pub determination_ref: String,
    /// Retained `actors` value.
    pub actors: Vec<String>,
    /// Retained `required_state_pointers` value.
    pub required_state_pointers: Vec<String>,
    /// Retained `trigger_event` value.
    pub trigger_event: String,
    /// Retained `transition` value.
    pub transition: TransitionLocation,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// One selected locale with retained template review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeTemplateSelection {
    /// Retained `locale` value.
    pub locale: String,
    /// Retained `template_key` value.
    pub template_key: String,
    /// Retained `review` value.
    pub review: ReviewRef,
}

/// Qualified local fixture notice delivery requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticePolicy {
    /// Retained `recipient_pointer` value.
    pub recipient_pointer: String,
    /// Retained `delivery_channel` value.
    pub delivery_channel: DeliveryChannel,
    /// Retained `templates` value.
    pub templates: Vec<NoticeTemplateSelection>,
    /// Retained `evidence_class` value.
    pub evidence_class: EvidenceClass,
    /// Retained `appeal_anchor` value.
    pub appeal_anchor: AppealAnchor,
    /// Retained `grace_anchor` value.
    pub grace_anchor: GraceAnchor,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Whole UTC calendar days to an actual held hearing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HearingDeadline {
    /// Retained `calendar_days` value.
    pub calendar_days: u32,
    /// Retained `purpose` value.
    pub purpose: HearingPurpose,
}

/// Selected continuation of original protected services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuationPolicy {
    /// Retained `choice` value.
    pub choice: ContinuationChoicePolicy,
    /// Retained `scope` value.
    pub scope: ContinuationScope,
    /// Retained `ends_on` value.
    pub ends_on: ContinuationEnd,
}

/// Standing, canonical independence, and hearing requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealPolicy {
    /// Retained `channels` value.
    pub channels: Vec<Channel>,
    /// Retained `standing_actors` value.
    pub standing_actors: Vec<String>,
    /// Retained `reviewer_actors` value.
    pub reviewer_actors: Vec<String>,
    /// Retained `excluded_determination_actors` value.
    pub excluded_determination_actors: ExcludedDeterminationActors,
    /// Retained `hearing_deadline` value.
    pub hearing_deadline: HearingDeadline,
    /// Retained `continuation` value.
    pub continuation: ContinuationPolicy,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Bound pre-notice timer extension limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionPolicy {
    /// Retained `timer_id` value.
    pub timer_id: String,
    /// Retained `allowed_state_pointers` value.
    pub allowed_state_pointers: Vec<String>,
    /// Retained `max_cumulative_calendar_days` value.
    pub max_cumulative_calendar_days: u32,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Bounded retained reference execution resource use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    /// Retained `max_determinations` value.
    pub max_determinations: usize,
    /// Retained `max_notices` value.
    pub max_notices: usize,
    /// Retained `max_appeals` value.
    pub max_appeals: usize,
    /// Retained `max_operations` value.
    pub max_operations: usize,
    /// Retained `max_outstanding_notice_work` value.
    pub max_outstanding_notice_work: usize,
    /// Retained `max_transport_attempts` value.
    pub max_transport_attempts: usize,
    /// Retained `max_notice_bytes` value.
    pub max_notice_bytes: usize,
}

/// Selected complete reference due-process execution policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessExecution {
    /// Retained `profile` value.
    pub profile: ReferenceProfile,
    /// Retained `determinations` value.
    pub determinations: Vec<DeterminationBinding>,
    /// Retained `operations` value.
    pub operations: Vec<OperationBinding>,
    /// Retained `notice` value.
    pub notice: NoticePolicy,
    /// Retained `appeal` value.
    pub appeal: AppealPolicy,
    /// Retained `pre_notice_extension` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub pre_notice_extension: Option<ExtensionPolicy>,
    /// Retained `limits` value.
    pub limits: Limits,
}

/// Declared slot and independently admitted actual actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceActor {
    /// Retained `declared_actor` value.
    pub declared_actor: String,
    /// Retained `acting_actor` value.
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
    /// Retained `operation_ref` value.
    pub operation_ref: String,
    /// Retained `determination_id` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub determination_id: Option<String>,
    /// Retained `idempotency_token` value.
    pub idempotency_token: String,
    /// Retained `actor` value.
    pub actor: GovernanceActor,
    /// Retained `command` value.
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
    /// Retained `record_id` value.
    pub record_id: String,
    /// Retained `sha256` value.
    pub sha256: String,
}

/// Accepted supervisor signature associated with exact actual content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationSignature {
    /// Retained `task_id` value.
    pub task_id: String,
    /// Retained `affirmation_record_id` value.
    pub affirmation_record_id: String,
    /// Retained `supervisor_human` value.
    pub supervisor_human: String,
    /// Retained `signed_content_sha256` value.
    pub signed_content_sha256: String,
}

/// Actual bound applied effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdverseEffect {
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `applied_at` value.
    pub applied_at: String,
}

/// Actual saved pending adverse determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationState {
    /// Retained `id` value.
    pub id: String,
    /// Retained `binding_ref` value.
    pub binding_ref: String,
    /// Retained `firing` value.
    pub firing: FiringRef,
    /// Retained `determined_at` value.
    pub determined_at: String,
    /// Retained `snapshot` value.
    pub snapshot: SnapshotRef,
    /// Retained `determining_human` value.
    pub determining_human: String,
    /// Retained `service_pointers` value.
    pub service_pointers: Vec<String>,
    /// Retained `signature` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub signature: Option<DeterminationSignature>,
    /// Retained `effect` value.
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
    /// Retained `locale` value.
    pub locale: String,
    /// Retained `template_key` value.
    pub template_key: String,
    /// Retained `review` value.
    pub review: ReviewRef,
    /// Retained `body` value.
    pub body: String,
    /// Retained `body_sha256` value.
    pub body_sha256: String,
}

/// Actual fixture transport acceptance, without delivery qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransportAcknowledgement {
    /// Retained `kind` value.
    pub kind: AcknowledgementKind,
    /// Retained `notice_id` value.
    pub notice_id: String,
    /// Retained `transport_key` value.
    pub transport_key: String,
    /// Retained `correlation` value.
    pub correlation: String,
    /// Retained `artifact_sha256` value.
    pub artifact_sha256: String,
    /// Retained `channel` value.
    pub channel: DeliveryChannel,
    /// Retained `accepted_at` value.
    pub accepted_at: String,
    /// Retained `adapter_result_id` value.
    pub adapter_result_id: String,
}

/// Actual same-instant qualified fixture receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualifiedNoticeReceipt {
    /// Retained `kind` value.
    pub kind: ReceiptKind,
    /// Retained `notice_id` value.
    pub notice_id: String,
    /// Retained `transport_key` value.
    pub transport_key: String,
    /// Retained `correlation` value.
    pub correlation: String,
    /// Retained `artifact_sha256` value.
    pub artifact_sha256: String,
    /// Retained `channel` value.
    pub channel: DeliveryChannel,
    /// Retained `evidence_class` value.
    pub evidence_class: EvidenceClass,
    /// Retained `qualified_at` value.
    pub qualified_at: String,
    /// Retained `adapter_result_id` value.
    pub adapter_result_id: String,
}

/// Actual no-effect fixture failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransportFailure {
    /// Retained `kind` value.
    pub kind: FailureKind,
    /// Retained `code` value.
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
    /// Retained `id` value.
    pub id: String,
    /// Retained `determination_id` value.
    pub determination_id: String,
    /// Retained `artifact_sha256` value.
    pub artifact_sha256: String,
    /// Retained `artifact_bytes_base64` value.
    pub artifact_bytes_base64: String,
    /// Retained `issue_at` value.
    pub issue_at: String,
    /// Retained `appeal_deadline` value.
    pub appeal_deadline: String,
    /// Retained `effect_eligible_at` value.
    pub effect_eligible_at: String,
    /// Retained `recipient` value.
    pub recipient: String,
    /// Retained `channel` value.
    pub channel: DeliveryChannel,
    /// Retained `variants` value.
    pub variants: Vec<NoticeVariant>,
    /// Retained `transport_key` value.
    pub transport_key: String,
    /// Retained `correlation` value.
    pub correlation: String,
    /// Retained `status` value.
    pub status: NoticeStatus,
    /// Retained `attempts` value.
    pub attempts: usize,
    /// Retained `acknowledgements` value.
    pub acknowledgements: Vec<TransportAcknowledgement>,
    /// Distinct actual typed results in first saved runtime observation order.
    ///
    /// These diagnostics grant no delivery qualification or invocation count.
    pub transport_results: Vec<TransportResult>,
    /// Retained `receipt` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub receipt: Option<QualifiedNoticeReceipt>,
    /// Retained `failure` value.
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
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `scheduled_at` value.
    pub scheduled_at: String,
    /// Retained `scheduled_for` value.
    pub scheduled_for: String,
    /// Retained `reviewer_actor` value.
    pub reviewer_actor: String,
    /// Retained `reviewer_human` value.
    pub reviewer_human: String,
}

/// Actual held-hearing evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeldHearing {
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `held_at` value.
    pub held_at: String,
    /// Retained `reviewer_human` value.
    pub reviewer_human: String,
    /// Retained `evidence_refs` value.
    pub evidence_refs: Vec<SourceRef>,
}

/// Actual held-hearing disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Disposition {
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `recorded_at` value.
    pub recorded_at: String,
    /// Retained `outcome` value.
    pub outcome: DispositionOutcome,
    /// Retained `rationale` value.
    pub rationale: String,
    /// Retained `evidence_refs` value.
    pub evidence_refs: Vec<SourceRef>,
}

/// Actual admitted household request to end continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HouseholdTermination {
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `requested_at` value.
    pub requested_at: String,
    /// Retained `requested_by` value.
    pub requested_by: String,
    /// Retained `rationale` value.
    pub rationale: String,
}

/// Saved timely appeal and separate hearing milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealState {
    /// Retained `id` value.
    pub id: String,
    /// Retained `determination_id` value.
    pub determination_id: String,
    /// Retained `notice_id` value.
    pub notice_id: String,
    /// Retained `filed_at` value.
    pub filed_at: String,
    /// Retained `filed_by` value.
    pub filed_by: String,
    /// Retained `channel` value.
    pub channel: Channel,
    /// Retained `continuation_choice` value.
    pub continuation_choice: ContinuationChoice,
    /// Retained `continuation` value.
    pub continuation: ContinuationStatus,
    /// Retained `hearing_deadline` value.
    pub hearing_deadline: String,
    /// Retained `schedule` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub schedule: Option<HearingSchedule>,
    /// Retained `hearing` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub hearing: Option<HeldHearing>,
    /// Retained `disposition` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub disposition: Option<Disposition>,
    /// Retained `termination` value.
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
    /// Retained `summary` value.
    pub summary: String,
}

/// Actual role authority verification for an override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverrideAuthorityVerification {
    /// Retained `actor_id` value.
    pub actor_id: String,
    /// Retained `method` value.
    pub method: OverrideAuthorityMethod,
    /// Retained `verified_at` value.
    pub verified_at: String,
    /// Retained `authority_source` value.
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
    /// Retained `kind` value.
    pub kind: OverrideEvidenceKind,
    /// Retained `uri` value.
    pub uri: String,
    /// Retained `summary` value.
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
    /// Retained `id` value.
    pub id: String,
    /// Retained `case_id` value.
    pub case_id: String,
    /// Retained `rationale` value.
    pub rationale: OverrideRationale,
    /// Retained `authority_verification` value.
    pub authority_verification: OverrideAuthorityVerification,
    /// Retained `supporting_evidence` value.
    pub supporting_evidence: Vec<OverrideEvidenceReference>,
}

/// Actual timer extension and canonical override evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionState {
    /// Retained `operation_record_id` value.
    pub operation_record_id: String,
    /// Retained `determination_binding_ref` value.
    pub determination_binding_ref: String,
    /// Retained `timer_id` value.
    pub timer_id: String,
    /// Retained `calendar_days` value.
    pub calendar_days: u32,
    /// Retained `cumulative_calendar_days` value.
    pub cumulative_calendar_days: u32,
    /// Retained `prior_deadline` value.
    pub prior_deadline: String,
    /// Retained `new_deadline` value.
    pub new_deadline: String,
    /// Retained `override_record` value.
    pub override_record: OverrideRecord,
}

/// Saved actual bound operation transition and outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationState {
    /// Retained `record_id` value.
    pub record_id: String,
    /// Retained `operation_ref` value.
    pub operation_ref: String,
    /// Retained `determination_id` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub determination_id: Option<String>,
    /// Retained `kind` value.
    pub kind: OperationKind,
    /// Retained `performed_at` value.
    pub performed_at: String,
    /// Retained `actor` value.
    pub actor: GovernanceActor,
    /// Retained `firing` value.
    pub firing: FiringRef,
    /// Retained `outcome` value.
    pub outcome: GovernanceOutcome,
}

/// Immutable exact successful public request replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayState {
    /// Retained `request` value.
    pub request: GovernanceRequest,
    /// Retained `result` value.
    pub result: GovernanceResult,
}

/// Retained native due-process facts for one process.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessState {
    /// Retained `determinations` value.
    pub determinations: Vec<DeterminationState>,
    /// Retained `notices` value.
    pub notices: Vec<NoticeState>,
    /// Retained `appeals` value.
    pub appeals: Vec<AppealState>,
    /// Retained `extensions` value.
    pub extensions: Vec<ExtensionState>,
    /// Retained `operations` value.
    pub operations: Vec<OperationState>,
    /// Retained `replays` value.
    pub replays: Vec<ReplayState>,
}

/// Complete immutable transport work retained before invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingNoticeWork {
    /// Retained `process_id` value.
    pub process_id: String,
    /// Retained `notice_id` value.
    pub notice_id: String,
    /// Retained `determination_id` value.
    pub determination_id: String,
    /// Retained `transport_key` value.
    pub transport_key: String,
    /// Retained `correlation` value.
    pub correlation: String,
    /// Retained `artifact_sha256` value.
    pub artifact_sha256: String,
    /// Retained `artifact_bytes_base64` value.
    pub artifact_bytes_base64: String,
    /// Retained `recipient` value.
    pub recipient: String,
    /// Retained `channel` value.
    pub channel: DeliveryChannel,
    /// Retained `issue_at` value.
    pub issue_at: String,
}

/// One actual native table input or output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedScalar {
    /// Retained `name` value.
    pub name: String,
    /// Retained `value` value.
    pub value: Scalar,
}

/// Actual native decision-table guard observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDecisionEvaluation {
    /// Retained `location` value.
    pub location: TransitionLocation,
    /// Retained `table_ref` value.
    pub table_ref: String,
    /// Retained `inputs` value.
    pub inputs: Vec<NamedScalar>,
    /// Retained `matched_row_ids` value.
    pub matched_row_ids: Vec<String>,
    /// Retained `selected_row_id` value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub selected_row_id: Option<String>,
    /// Retained `outputs` value.
    pub outputs: Vec<NamedScalar>,
    /// Retained `guard_result` value.
    pub guard_result: bool,
}

/// Source-linked factual selected-row explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationReasoningData {
    /// Retained `tier` value.
    pub tier: ReasoningTier,
    /// Retained `related_transition` value.
    pub related_transition: String,
    /// Retained `rule_id` value.
    pub rule_id: String,
    /// Retained `authority` value.
    pub authority: SourceAuthority,
    /// Retained `description` value.
    pub description: String,
    /// Retained `timestamp` value.
    pub timestamp: String,
    /// Retained `evaluation` value.
    pub evaluation: NativeDecisionEvaluation,
    /// Retained `source_refs` value.
    pub source_refs: Vec<SourceRef>,
}

/// Actual private hypothetical evaluation with immutable snapshot association.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeterminationCounterfactualData {
    /// Retained `tier` value.
    pub tier: CounterfactualTier,
    /// Retained `related_transition` value.
    pub related_transition: String,
    /// Retained `kind` value.
    #[serde(rename = "type")]
    pub kind: CounterfactualKind,
    /// Retained `factor` value.
    pub factor: String,
    /// Retained `description` value.
    pub description: String,
    /// Retained `alternative` value.
    pub alternative: CaseAlternative,
    /// Retained `original_snapshot_sha256` value.
    pub original_snapshot_sha256: String,
    /// Retained `alternate_snapshot_sha256` value.
    pub alternate_snapshot_sha256: String,
    /// Retained `original_evaluation` value.
    pub original_evaluation: NativeDecisionEvaluation,
    /// Retained `alternate_evaluation` value.
    pub alternate_evaluation: NativeDecisionEvaluation,
    /// Retained `source_refs` value.
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
