use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::AppState;
use crate::auth::{RequireRole, Supervisor};
use crate::error::{ApiError, ApiResult};
use crate::services::advanced_service::{
    self, ConstraintZoneView, EquityEvaluateRequest, EquityReport, ValidActionsResponse,
    VerifyRequest, VerifyResponse,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/verification/verify", post(verify))
        .route("/equity/evaluate", post(evaluate_equity))
        .route(
            "/governance/{url}/constraint-zones",
            get(list_constraint_zones),
        )
        .route(
            "/instances/{id}/constraint-zones/{zone}/valid-actions",
            get(valid_actions_in_zone),
        )
}

async fn verify(
    State(s): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> ApiResult<Json<VerifyResponse>> {
    Ok(Json(
        advanced_service::verify(&s.services.bundle, &req).await?,
    ))
}

async fn evaluate_equity(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Json(req): Json<EquityEvaluateRequest>,
) -> ApiResult<Json<EquityReport>> {
    Ok(Json(
        advanced_service::evaluate_equity(&s.storage, &req).await?,
    ))
}

async fn list_constraint_zones(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<Vec<ConstraintZoneView>>> {
    Ok(Json(
        advanced_service::list_zones(&s.services.bundle, &url).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZoneQuery {
    workflow_url: String,
}

async fn valid_actions_in_zone(
    State(s): State<AppState>,
    Path((instance_id, zone)): Path<(String, String)>,
    Query(q): Query<ZoneQuery>,
) -> ApiResult<Json<ValidActionsResponse>> {
    // Instance existence check — fail fast with 404 for unknown ids.
    if s.storage.get_instance(&instance_id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(
        advanced_service::valid_actions_in_zone(
            &s.services.bundle,
            &instance_id,
            &zone,
            &q.workflow_url,
        )
        .await?,
    ))
}
