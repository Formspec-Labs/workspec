use serde::{Deserialize, Serialize};

/// `CaseInstanceView` in `WosBackend.ts:37`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseInstanceView {
    pub instance_id: String,
    pub definition_url: String,
    pub definition_version: String,
    pub status: String,
    pub configuration: Vec<String>,
    pub case_state: serde_json::Value,
    pub active_tasks: Vec<ActiveTaskView>,
    pub timers: Vec<TimerView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_state: Option<GovernanceStateView>,
    pub impact_level: String,
    pub created_at: String,
    pub updated_at: String,
}

/// `ActiveTaskView` in `WosBackend.ts:56`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTaskView {
    pub task_id: String,
    pub task_ref: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// `TimerView` in `WosBackend.ts:69`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerView {
    pub timer_id: String,
    pub deadline: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceStateView {
    pub active_delegations: Vec<DelegationShortView>,
    pub active_holds: Vec<HoldView>,
    pub review_state: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationShortView {
    pub delegator_id: String,
    pub delegate_id: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    pub granted_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldView {
    pub hold_type: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_end: Option<String>,
    pub resume_trigger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_state: Option<String>,
}

/// `EvaluationResult` in `WosBackend.ts:93`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResultView {
    pub previous_configuration: Vec<String>,
    pub new_configuration: Vec<String>,
    pub events_fired: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_record: Option<crate::domain::ProvenanceRecordView>,
    pub case_state_mutations: serde_json::Value,
}

/// `AvailableTransition` in `WosBackend.ts:101`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableTransitionView {
    pub event: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    pub guard_satisfied: bool,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `PaginatedResult<T>` in `WosBackend.ts:146`. Note `totalPages`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedView<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

impl<T> PaginatedView<T> {
    pub fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        let total_pages = if page_size == 0 {
            0
        } else {
            ((total + page_size as u64 - 1) / page_size as u64) as u32
        };
        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// `TaskListItem` in `WosPorts.ts:10`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListItemView {
    pub task_id: String,
    pub instance_id: String,
    pub task_ref: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<String>,
    pub configuration: Vec<String>,
    pub case_state: serde_json::Value,
    pub definition_title: String,
    pub definition_url: String,
    pub created_at: String,
}

/// `POST /api/instances/:id/events` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEventRequest {
    pub event: String,
    pub actor_id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// Query params for list-instances / list-tasks.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    pub status: Option<String>,
    pub impact_level: Option<String>,
    pub definition_url: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl ListQuery {
    pub fn csv(v: &Option<String>) -> Option<Vec<String>> {
        v.as_ref().map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
    }
}
