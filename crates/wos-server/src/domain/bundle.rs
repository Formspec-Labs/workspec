use serde::{Deserialize, Serialize};

/// `KernelSummary` in `WosBackend.ts:131`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelSummaryView {
    pub url: String,
    pub title: String,
    pub version: String,
    pub status: String,
    pub impact_level: String,
}

/// `WosDocumentBundle` in `WosBackend.ts:110` — the kernel plus optional
/// sidecar documents. Sidecars are kept as `serde_json::Value` so we don't
/// bind the server to the studio's ever-growing sidecar typing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleView {
    pub kernel: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "dueProcess")]
    pub due_process: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "assertionGates")]
    pub assertion_gates: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "policyParameters")]
    pub policy_parameters: Option<serde_json::Value>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "notificationTemplates"
    )]
    pub notification_templates: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "businessCalendar")]
    pub business_calendar: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "driftMonitor")]
    pub drift_monitor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "agentConfigs")]
    pub agent_configs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "verificationReport")]
    pub verification_report: Option<serde_json::Value>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "correspondenceMetadata"
    )]
    pub correspondence_metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "semanticProfile")]
    pub semantic_profile: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "integrationProfile")]
    pub integration_profile: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "lifecycleDetail")]
    pub lifecycle_detail: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "caseInstances")]
    pub case_instances: Option<serde_json::Value>,
}

/// `WosValidationResult` / `WosValidationIssue` in `WosPorts.ts:43`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResultView {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssueView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssueView {
    pub severity: String,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}
