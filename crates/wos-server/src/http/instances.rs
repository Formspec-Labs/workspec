use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::Router;

use crate::AppState;
use crate::domain::{
    AvailableTransitionView, EvaluationResultView, InstanceResponse, ListQuery, PaginatedView,
    SubmitEventRequest,
};
use crate::domain::provenance::ProvenanceResponse;
use crate::error::{ApiError, ApiResult};
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
) -> ApiResult<Json<PaginatedView<InstanceResponse>>> {
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
    let mut items = Vec::with_capacity(page_result.items.len());
    for row in &page_result.items {
        items.push(s.services.instance.to_response(row).await?);
    }
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
) -> ApiResult<Json<InstanceResponse>> {
    let row = s
        .storage
        .get_instance(&id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(s.services.instance.to_response(&row).await?))
}

async fn provenance(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ProvenanceResponse>>> {
    Ok(Json(s.services.provenance.list(&id).await?))
}

async fn transitions(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<AvailableTransitionView>>> {
    Ok(Json(s.services.eval.available_transitions(&id).await?))
}

async fn submit_event(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SubmitEventRequest>,
) -> ApiResult<Json<EvaluationResultView>> {
    Ok(Json(s.services.eval.submit_event(&id, &req).await?))
}
