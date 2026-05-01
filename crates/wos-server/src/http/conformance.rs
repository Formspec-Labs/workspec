use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::post;

use crate::AppState;
use crate::error::ApiResult;
use crate::services::conformance_service::{self, FixtureRequest, FixtureResponse};

pub fn routes() -> Router<AppState> {
    Router::new().route("/conformance/fixture", post(run_fixture))
}

async fn run_fixture(
    State(s): State<AppState>,
    Json(req): Json<FixtureRequest>,
) -> ApiResult<Json<FixtureResponse>> {
    let default_base_dir = s.cfg.fixtures_dir.to_string_lossy().to_string();
    Ok(Json(conformance_service::run(&req, &default_base_dir)?))
}
