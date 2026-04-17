use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Router;

use crate::AppState;
use crate::domain::{
    AvailableTransitionView, CaseInstanceView, EvaluationResultView, ListQuery, PaginatedView,
    ProvenanceRecordView, SubmitEventRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::services::instance_service::InstanceService;
use crate::storage::InstanceQuery;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/instances", get(list))
        .route("/instances/{id}", get(get_one))
        .route("/instances/{id}/provenance", get(provenance))
        .route("/instances/{id}/transitions", get(transitions))
        .route("/instances/{id}/events", post(submit_event))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PaginatedView<CaseInstanceView>>> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(25);
    let storage_query = InstanceQuery {
        status: ListQuery::csv(&q.status),
        impact_level: ListQuery::csv(&q.impact_level),
        definition_url: q.definition_url.map(|d| vec![d]),
        page,
        page_size,
    };
    let page_result = s.storage.list_instances(storage_query).await?;
    let items: Vec<CaseInstanceView> = page_result
        .items
        .iter()
        .map(InstanceService::map_row)
        .collect();
    Ok(Json(PaginatedView::new(
        items,
        page_result.total,
        page_result.page,
        page_result.page_size,
    )))
}

async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CaseInstanceView>> {
    let row = s
        .storage
        .get_instance(&id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(InstanceService::map_row(&row)))
}

async fn provenance(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ProvenanceRecordView>>> {
    Ok(Json(s.services.provenance.list(&id).await?))
}

async fn transitions(
    State(_s): State<AppState>,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<AvailableTransitionView>>> {
    // Populated by the eval service (step 7). Until then, return none so the
    // studio's UI shows a disabled action bar rather than 500ing.
    Ok(Json(Vec::new()))
}

async fn submit_event(
    State(_s): State<AppState>,
    Path(_id): Path<String>,
    Json(_req): Json<SubmitEventRequest>,
) -> ApiResult<Json<EvaluationResultView>> {
    // Wired up by the eval service in step 7.
    Err(ApiError::ServiceUnavailable(
        "eval service not yet wired; see plan step 7".into(),
    ))
}
