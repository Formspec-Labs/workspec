use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;

use crate::AppState;
use crate::domain::{ListQuery, PaginatedView, TaskListItem};
use crate::error::{ApiError, ApiResult};
use crate::storage::InstanceQuery;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tasks", get(list))
        .route("/tasks/{id}", get(get_one))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PaginatedView<TaskListItem>>> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(25);
    let storage_query = InstanceQuery {
        status: ListQuery::csv(&q.status),
        impact_level: ListQuery::csv(&q.impact_level),
        definition_url: q.definition_url.map(|d| vec![d]),
        page: 1,
        page_size: 500,
    };
    let source = s.storage.list_instances(storage_query).await?;
    let mut all_tasks: Vec<TaskListItem> = Vec::new();
    for row in &source.items {
        all_tasks.extend(s.services.instance.tasks_for(row).await?);
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
    let storage_query = InstanceQuery {
        page: 1,
        page_size: 500,
        ..Default::default()
    };
    let source = s.storage.list_instances(storage_query).await?;
    for row in &source.items {
        for t in s.services.instance.tasks_for(row).await? {
            if t.task.task_id == task_id {
                return Ok(Json(t));
            }
        }
    }
    Err(ApiError::NotFound)
}
