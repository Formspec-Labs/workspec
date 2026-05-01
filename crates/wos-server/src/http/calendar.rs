use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::post;

use crate::AppState;
use crate::error::ApiResult;
use crate::services::calendar_service::{ComputeDeadlineRequest, ComputeDeadlineResponse};

pub fn routes() -> Router<AppState> {
    Router::new().route("/calendar/{url}/compute-deadline", post(compute_deadline))
}

async fn compute_deadline(
    State(s): State<AppState>,
    Path(url): Path<String>,
    Json(req): Json<ComputeDeadlineRequest>,
) -> ApiResult<Json<ComputeDeadlineResponse>> {
    Ok(Json(
        s.services.calendar.compute_deadline(&url, &req).await?,
    ))
}
