use serde::{Deserialize, Serialize};

/// `ApplicantDeterminationView` in `WosPorts.ts:236`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicantDeterminationView {
    pub instance_id: String,
    pub program_name: String,
    pub decision: String,
    pub date_issued: String,
    pub deadline_date: String,
    pub benefits_continue: bool,
    pub summary: String,
    pub evidence_considered: Vec<String>,
    pub rules_applied: Vec<String>,
    pub ai_disclosure: AiDisclosureView,
    pub counterfactuals: CounterfactualsView,
    pub appeal_status: String,
    pub milestones: Vec<MilestoneView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDisclosureView {
    pub was_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_reviewer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualsView {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneView {
    pub id: String,
    pub label: String,
    pub status: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// `POST /api/applicant/:id/appeal` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealRequest {
    pub reason: String,
}
