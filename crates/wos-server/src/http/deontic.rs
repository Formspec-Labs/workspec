use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;

use crate::AppState;
use crate::domain::provenance::ProvenanceResponse;
use crate::error::ApiResult;
use crate::services::deontic_service::DeonticService;

pub fn routes() -> Router<AppState> {
    Router::new().route("/instances/{id}/deontic-violations", get(list_violations))
}

async fn list_violations(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ProvenanceResponse>>> {
    Ok(Json(
        DeonticService::list(&s.services.provenance, &id).await?,
    ))
}
