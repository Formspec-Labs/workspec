use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post};

use crate::AppState;
use crate::auth::{Adjudicator, RequireRole, Supervisor};
use crate::error::ApiResult;
use crate::services::assurance_service::{
    AssuranceChainResponse, AssuranceService, IdentityFactView, RecordFactRequest, UpgradeRequest,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/instances/{id}/identity-facts",
            post(record).get(list_for_instance),
        )
        .route(
            "/instances/{id}/identity-facts/{factId}/upgrade",
            post(upgrade),
        )
        .route("/subjects/{ref}/assurance-chain", get(assurance_chain))
}

async fn record(
    State(s): State<AppState>,
    _: RequireRole<Adjudicator>,
    Path(id): Path<String>,
    Json(req): Json<RecordFactRequest>,
) -> ApiResult<Json<IdentityFactView>> {
    Ok(Json(
        AssuranceService::record_fact(&s.storage, &id, req).await?,
    ))
}

async fn list_for_instance(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<IdentityFactView>>> {
    Ok(Json(
        AssuranceService::list_for_instance(&s.storage, &id).await?,
    ))
}

async fn upgrade(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path((_instance, fact_id)): Path<(String, String)>,
    Json(req): Json<UpgradeRequest>,
) -> ApiResult<Json<IdentityFactView>> {
    Ok(Json(
        AssuranceService::upgrade(&s.storage, &fact_id, req).await?,
    ))
}

async fn assurance_chain(
    State(s): State<AppState>,
    Path(subject_ref): Path<String>,
) -> ApiResult<Json<AssuranceChainResponse>> {
    Ok(Json(
        AssuranceService::assurance_chain_with_validation(&s.storage, &subject_ref).await?,
    ))
}
