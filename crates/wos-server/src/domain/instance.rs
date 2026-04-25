//! Server-specific additions to the spec's instance model.
//!
//! `InstanceResponse` wraps a `wos_core::instance::CaseInstance` with
//! server-resolved fields that don't live on the instance itself
//! (currently `impactLevel` and `definitionTitle`). The instance payload
//! is **flattened** at the serde layer so the JSON body matches
//! `CaseInstance`'s spec-defined shape plus the extra fields at the
//! top level.

use serde::{Deserialize, Serialize};
use wos_core::instance::CaseInstance;

/// `GET /api/instances/:id` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceResponse {
    #[serde(flatten)]
    pub instance: CaseInstance,
    pub impact_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_title: Option<String>,
}

/// `AvailableTransition` is not a spec type; the enumeration exists only
/// to drive client-side UX. Guard satisfaction reporting is best-effort
/// (authoritative evaluation happens inside `Evaluator::process_event`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableTransitionView {
    pub event: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    pub guard_satisfied: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `POST /api/instances/:id/events` response. Summarises the effect of an
/// evaluator step from the caller's perspective. The full per-record
/// provenance chain is still available via `/instances/:id/provenance`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResultView {
    pub previous_configuration: Vec<String>,
    pub new_configuration: Vec<String>,
    pub events_fired: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_record: Option<crate::domain::provenance::ProvenanceResponse>,
    pub case_state_mutations: serde_json::Value,
}

/// A task-listing row: flattens `wos_core::instance::ActiveTask` and
/// joins per-instance context (definition url, title, case state) so
/// inbox UIs don't need to round-trip back for each task.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListItem {
    #[serde(flatten)]
    pub task: wos_core::instance::ActiveTask,
    pub instance_id: String,
    pub definition_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_title: Option<String>,
    pub configuration: Vec<String>,
    pub case_state: serde_json::Value,
}

/// Generic paginated wrapper. Used wherever the server has to page through
/// a storage query (instances, tasks). 1-indexed `page` + `pageSize` to
/// match the most common REST pagination convention.
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

/// `POST /api/instances/:id/events` request body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEventRequest {
    pub event: String,
    pub actor_id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub idempotency_token: Option<String>,
}

/// Shared query-string shape for list endpoints.
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
