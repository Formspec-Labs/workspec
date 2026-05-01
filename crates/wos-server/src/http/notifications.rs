use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::post;

use crate::AppState;
use crate::error::ApiResult;
use crate::services::notifications_service::{RenderRequest, RenderResponse};

pub fn routes() -> Router<AppState> {
    Router::new().route("/notifications/{url}/render", post(render))
}

async fn render(
    State(s): State<AppState>,
    Path(url): Path<String>,
    Json(req): Json<RenderRequest>,
) -> ApiResult<Json<RenderResponse>> {
    Ok(Json(s.services.notifications.render(&url, &req).await?))
}
