use axum::Json;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::Router;

use crate::AppState;
use crate::domain::{AppealRequest, ApplicantDeterminationView};
use crate::error::{ApiError, ApiResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/applicant/{id}/determination", get(determination))
        .route("/applicant/{id}/appeal", post(appeal))
}

async fn determination(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApplicantDeterminationView>> {
    s.services
        .applicant
        .determination(&id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn appeal(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AppealRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    s.services.applicant.submit_appeal(&id, &body.reason).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
