use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::AppState;
use crate::auth::{RequireRole, Supervisor};
use crate::error::{ApiError, ApiResult};
use crate::services::agent_service::{
    AgentService, AgentView, DeploymentState, DriftReport, LifecycleTransitionRequest,
    RegisterAgentRequest, ToolInvocationCheck,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents", get(list).post(register))
        .route("/agents/{id}", get(get_one))
        .route(
            "/agents/{id}/lifecycle-transition",
            post(lifecycle_transition),
        )
        .route("/agents/{id}/canary", post(canary))
        .route("/agents/{id}/shadow", post(shadow))
        .route("/agents/{id}/drift", get(drift))
        .route(
            "/agents/{id}/tool-invocation-check",
            post(tool_invocation_check),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    workflow_url: Option<String>,
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<AgentView>>> {
    let workflow_url = q
        .workflow_url
        .ok_or_else(|| ApiError::BadRequest("workflowUrl query param is required".into()))?;
    Ok(Json(AgentService::list(&s.storage, &workflow_url).await?))
}

async fn register(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Json(req): Json<RegisterAgentRequest>,
) -> ApiResult<Json<AgentView>> {
    Ok(Json(AgentService::register(&s.storage, req).await?))
}

async fn get_one(State(s): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<AgentView>> {
    Ok(Json(AgentService::get(&s.storage, &id).await?))
}

async fn lifecycle_transition(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path(id): Path<String>,
    Json(req): Json<LifecycleTransitionRequest>,
) -> ApiResult<Json<AgentView>> {
    Ok(Json(
        AgentService::transition_lifecycle(&s.storage, &id, req).await?,
    ))
}

async fn canary(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentView>> {
    Ok(Json(
        AgentService::set_deployment(&s.storage, &id, DeploymentState::Canary).await?,
    ))
}

async fn shadow(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path(id): Path<String>,
) -> ApiResult<Json<AgentView>> {
    Ok(Json(
        AgentService::set_deployment(&s.storage, &id, DeploymentState::Shadow).await?,
    ))
}

async fn drift(State(s): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<DriftReport>> {
    Ok(Json(AgentService::drift_report(&s.storage, &id).await?))
}

async fn tool_invocation_check(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ToolInvocationCheck>> {
    Ok(Json(
        AgentService::tool_invocation_check(&s.storage, &id).await?,
    ))
}
