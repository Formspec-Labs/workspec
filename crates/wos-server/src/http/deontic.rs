use axum::Json;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;

use crate::AppState;
use crate::error::ApiResult;
use crate::services::deontic_service::{DeonticService, ViolationView};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/instances/{id}/deontic-violations",
        get(list_violations),
    )
}

async fn list_violations(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ViolationView>>> {
    Ok(Json(
        DeonticService::list(&s.services.provenance, &id).await?,
    ))
}
