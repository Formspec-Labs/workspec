use axum::Json;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::Router;

use crate::AppState;
use crate::auth::{Adjudicator, RequireRole};
use crate::error::ApiResult;
use crate::services::integration_service::{InboundAck, IntegrationService, WosInboundEvent};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/events/inbound", post(inbound))
        .route("/integration/{url}/profile", get(profile))
        .route("/integration/{url}/invoke/{binding}", post(invoke))
}

async fn inbound(
    State(s): State<AppState>,
    _: RequireRole<Adjudicator>,
    Json(ev): Json<WosInboundEvent>,
) -> ApiResult<Json<InboundAck>> {
    Ok(Json(IntegrationService::accept_inbound(&s, ev).await?))
}

async fn profile(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        IntegrationService::integration_profile(&s.services.bundle, &url).await?,
    ))
}

async fn invoke(
    State(s): State<AppState>,
    _: RequireRole<Adjudicator>,
    Path((url, binding)): Path<(String, String)>,
    Json(inputs): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        IntegrationService::invoke_binding(&s.services.bundle, &url, &binding, &inputs).await?,
    ))
}
