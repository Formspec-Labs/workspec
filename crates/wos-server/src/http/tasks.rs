use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use wos_runtime::{PersistDraftResult, TaskSubmissionResult};

use crate::AppState;
use crate::domain::{ListQuery, PaginatedView, TaskListItem};
use crate::error::{ApiError, ApiResult};
use crate::storage::{self, InstanceQuery, LIST_INSTANCES_PAGE_SIZE_MAX};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tasks", get(list))
        .route("/tasks/{id}", get(get_one))
        .route("/tasks/{id}/draft", post(persist_draft))
        .route("/tasks/{id}/response", post(submit_response))
        .route("/tasks/{id}/dismiss", post(dismiss))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PaginatedView<TaskListItem>>> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(25);
    let filter = InstanceQuery {
        status: ListQuery::csv(&q.status),
        impact_level: ListQuery::csv(&q.impact_level),
        definition_url: q.definition_url.map(|d| vec![d]),
        page: 1,
        page_size: LIST_INSTANCES_PAGE_SIZE_MAX,
    };
    let rows = storage::list_instances_all_pages(
        &s.storage,
        filter,
        LIST_INSTANCES_PAGE_SIZE_MAX,
    )
    .await?;
    let mut all_tasks: Vec<TaskListItem> = Vec::new();
    for row in rows {
        all_tasks.extend(s.services.instance.tasks_for(&row).await?);
    }
    let total = all_tasks.len() as u64;
    let start = ((page.saturating_sub(1)) * page_size) as usize;
    let end = start + page_size as usize;
    let slice: Vec<TaskListItem> = all_tasks
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    Ok(Json(PaginatedView::new(slice, total, page, page_size)))
}

async fn get_one(
    State(s): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<Json<TaskListItem>> {
    let rows = storage::list_instances_all_pages(
        &s.storage,
        InstanceQuery::default(),
        LIST_INSTANCES_PAGE_SIZE_MAX,
    )
    .await?;
    for row in rows {
        for t in s.services.instance.tasks_for(&row).await? {
            if t.task.task_id == task_id {
                return Ok(Json(t));
            }
        }
    }
    Err(ApiError::NotFound)
}

// ── Task binding surface (wos-runtime task methods) ────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDraftRequest {
    /// The task response document. Must carry `status: "in-progress" |
    /// "amended" | "stopped"`.
    pub response: serde_json::Value,
    pub actor_id: String,
    #[serde(default)]
    pub idempotency_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSubmitRequest {
    /// Task response document; `status: "completed"` triggers lifecycle
    /// advancement. Any other status produces a `Rejected` result.
    pub response: serde_json::Value,
    pub actor_id: String,
    #[serde(default)]
    pub idempotency_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDismissRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDraftView {
    pub artifact_id: String,
}

impl From<PersistDraftResult> for TaskDraftView {
    fn from(r: PersistDraftResult) -> Self {
        Self {
            artifact_id: r.artifact_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "outcome")]
pub enum TaskSubmissionView {
    Completed {
        artifact_id: String,
        case_mutated: bool,
        emitted_event: Option<String>,
    },
    Failed {
        code: String,
        emitted_event: Option<String>,
    },
    Rejected {
        code: String,
    },
}

impl From<TaskSubmissionResult> for TaskSubmissionView {
    fn from(r: TaskSubmissionResult) -> Self {
        match r {
            TaskSubmissionResult::Completed {
                artifact_id,
                case_mutated,
                emitted_event,
            } => Self::Completed {
                artifact_id,
                case_mutated,
                emitted_event,
            },
            TaskSubmissionResult::Failed {
                code,
                emitted_event,
            } => Self::Failed {
                code,
                emitted_event,
            },
            TaskSubmissionResult::Rejected { code } => Self::Rejected { code },
        }
    }
}

async fn persist_draft(
    State(s): State<AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<TaskDraftRequest>,
) -> ApiResult<Json<TaskDraftView>> {
    let out = s
        .runtime
        .persist_task_draft(
            &task_id,
            req.response,
            &req.actor_id,
            req.idempotency_token.as_deref(),
        )
        .await
        ?;
    Ok(Json(out.into()))
}

async fn submit_response(
    State(s): State<AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<TaskSubmitRequest>,
) -> ApiResult<Json<TaskSubmissionView>> {
    let out = s
        .runtime
        .submit_task_response(
            &task_id,
            req.response,
            &req.actor_id,
            req.idempotency_token.as_deref(),
        )
        .await
        ?;
    Ok(Json(out.into()))
}

async fn dismiss(
    State(s): State<AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<TaskDismissRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    s.runtime
        .dismiss_task(&task_id, &req.reason)
        .await
        ?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
