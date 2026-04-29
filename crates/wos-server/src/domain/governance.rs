use serde::{Deserialize, Serialize};

/// `AgentView` in `WosPorts.ts:55`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub version: String,
    pub status: String,
    pub capabilities: Vec<AgentCapabilityView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_floor: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilityView {
    pub name: String,
    pub autonomy: String,
}

/// `DelegationEntry` in `WosPorts.ts:65`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationEntryView {
    pub id: String,
    pub delegator: String,
    pub delegate: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_instrument: Option<String>,
    pub start_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    pub status: String,
}

/// `PolicyVersionView` in `WosPorts.ts:77`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyVersionView {
    pub id: String,
    pub label: String,
    pub effective_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    pub parameter_count: u64,
    pub status: String,
}

/// `ResolvedPolicyView` — payload for `/policy/{url}/resolve` (WS-034).
/// Returns the version of the
/// `policy-parameters` sidecar active at the requested instant, with the full
/// parameters object inlined so callers do not need a second fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPolicyView {
    pub id: String,
    pub label: String,
    pub effective_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    pub parameters: serde_json::Value,
    pub resolved_as_of: String,
}

/// `CalendarEventView` in `WosPorts.ts:86`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventView {
    pub id: String,
    pub name: String,
    pub date: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub impacts_deadlines: bool,
}

/// `ServiceHealthView` in `WosPorts.ts:94`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthView {
    pub id: String,
    pub name: String,
    pub status: String,
    pub latency: String,
    pub error_rate: String,
    pub last_check: String,
}

/// `DeonticConstraintView` in `WosPorts.ts:103`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeonticConstraintView {
    pub kind: String,
    pub id: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_violation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypassable: Option<bool>,
}

/// `QualityControlsView` in `WosPorts.ts:112`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityControlsView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_sampling: Option<ReviewSamplingView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separation_of_duties: Option<SeparationOfDutiesView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_authority: Option<OverrideAuthorityView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSamplingView {
    pub rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparationOfDutiesView {
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverrideAuthorityView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_structured_rationale: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_authority_verification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_supporting_evidence: Option<bool>,
}

/// `PipelineView` and `PipelineStageView` in `WosPorts.ts:118-131`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineView {
    pub id: String,
    pub stages: Vec<PipelineStageView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageView {
    pub id: String,
    #[serde(rename = "type")]
    pub stage_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertions: Option<Vec<PipelineAssertionView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineAssertionView {
    #[serde(rename = "type")]
    pub assertion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_policy: Option<String>,
}

/// `VerificationReportView` in `WosPorts.ts:141`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationReportView {
    pub solver: SolverView,
    pub results: Vec<VerificationResultView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<VerificationSummaryView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverView {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResultView {
    pub constraint_ref: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<VerificationCounterexampleView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationCounterexampleView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSummaryView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_constraints: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proven_safe: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proven_unsafe: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inconclusive: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_solver_time_ms: Option<u64>,
}

/// `AdverseDecisionNoticeView` — rendered adverse-decision notice with
/// due-process semantics stamped from the `dueProcess` sidecar (Gov §3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdverseDecisionNoticeView {
    pub template_id: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appeal_window: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_to_contest: Option<String>,
    pub channels: Vec<String>,
}

/// `EquityConfigView` in `WosPorts.ts:161`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityConfigView {
    pub protected_categories: Vec<EquityCategoryView>,
    pub disparity_methods: Vec<EquityDisparityMethodView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_schedule: Option<EquityReportingScheduleView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation_triggers: Option<Vec<EquityRemediationTriggerView>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityCategoryView {
    pub id: String,
    pub group_by_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityDisparityMethodView {
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityReportingScheduleView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityRemediationTriggerView {
    pub condition: String,
    pub action: String,
    pub notify_roles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
