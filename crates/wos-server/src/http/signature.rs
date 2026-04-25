use axum::Json;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;

use crate::AppState;
use crate::error::ApiResult;
use crate::domain::provenance::ProvenanceResponse;
use crate::services::signature_service::SignatureService;

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/instances/{id}/signature-affirmations",
        get(list_affirmations),
    )
}

async fn list_affirmations(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ProvenanceResponse>>> {
    Ok(Json(
        SignatureService::list(&s.services.provenance, &id).await?,
    ))
}
