use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;

use crate::AppState;
use crate::auth::{RequireRole, Supervisor};
use crate::domain::{
    AdverseDecisionNoticeView, AgentView, CalendarEventView, DelegationEntryView,
    DeonticConstraintView, EquityConfigView, PipelineView, PolicyVersionView, QualityControlsView,
    ResolvedPolicyView, ServiceHealthView, VerificationReportView,
};
use crate::error::{ApiError, ApiResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/governance/{url}/agents", get(agents))
        .route(
            "/governance/{url}/deontic-constraints",
            get(deontic_constraints),
        )
        .route("/governance/{url}/quality-controls", get(quality_controls))
        .route("/governance/{url}/pipelines", get(pipelines))
        .route(
            "/governance/{url}/verification-report",
            get(verification_report),
        )
        .route("/governance/{url}/equity-config", get(equity_config))
        .route(
            "/governance/{url}/delegations",
            get(delegations_list).post(delegation_create),
        )
        .route(
            "/governance/{url}/delegations/{id}",
            delete(delegation_revoke),
        )
        .route("/governance/{url}/policy-versions", get(policy_versions))
        .route("/policy/{url}/resolve", get(policy_resolve_get))
        .route("/governance/{url}/calendar-events", get(calendar_events))
        .route(
            "/governance/{url}/notices/{template}/render",
            post(render_adverse_notice),
        )
        .route("/health", get(health))
}

async fn agents(State(s): State<AppState>, Path(url): Path<String>) -> Json<Vec<AgentView>> {
    Json(s.services.governance.agents(&url).await)
}

async fn deontic_constraints(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> Json<Vec<DeonticConstraintView>> {
    Json(s.services.governance.deontic_constraints(&url).await)
}

async fn quality_controls(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<QualityControlsView>> {
    s.services
        .governance
        .quality_controls(&url)
        .await
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn pipelines(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> Json<Vec<PipelineView>> {
    Json(s.services.governance.pipelines(&url).await)
}

async fn verification_report(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<VerificationReportView>> {
    s.services
        .governance
        .verification_report(&url)
        .await
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn equity_config(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<EquityConfigView>> {
    s.services
        .governance
        .equity_config(&url)
        .await
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn delegations_list(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> ApiResult<Json<Vec<DelegationEntryView>>> {
    Ok(Json(s.services.governance.delegations(&url).await?))
}

async fn delegation_create(
    State(s): State<AppState>,
    Path(url): Path<String>,
    _: RequireRole<Supervisor>,
    Json(entry): Json<DelegationEntryView>,
) -> ApiResult<Json<serde_json::Value>> {
    s.services.governance.create_delegation(&url, &entry).await?;
    Ok(Json(serde_json::json!({ "ok": true, "id": entry.id })))
}

async fn delegation_revoke(
    State(s): State<AppState>,
    Path((url, id)): Path<(String, String)>,
    _: RequireRole<Supervisor>,
) -> ApiResult<Json<serde_json::Value>> {
    s.services.governance.revoke_delegation(&url, &id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn policy_versions(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> Json<Vec<PolicyVersionView>> {
    Json(s.services.governance.policy_versions(&url).await)
}

/// Query string for `GET /api/policy/{url}/resolve?asOf=...` (WS-034). Picks
/// a URL-addressable shape so callers can link, cache, and pin a date-resolved
/// parameter set by URL alone — that round-trip is the whole point of the
/// `policy-parameters` sidecar.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyResolveQuery {
    pub as_of: String,
}

async fn policy_resolve_get(
    State(s): State<AppState>,
    Path(url): Path<String>,
    Query(q): Query<PolicyResolveQuery>,
) -> ApiResult<Json<ResolvedPolicyView>> {
    let as_of = chrono::DateTime::parse_from_rfc3339(&q.as_of)
        .map_err(|e| ApiError::BadRequest(format!("invalid asOf: {e}")))?
        .with_timezone(&chrono::Utc);
    s.services
        .governance
        .resolve_policy(&url, &as_of)
        .await
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn calendar_events(
    State(s): State<AppState>,
    Path(url): Path<String>,
) -> Json<Vec<CalendarEventView>> {
    Json(s.services.governance.calendar_events(&url).await)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdverseNoticeRenderRequest {
    #[serde(default)]
    pub context: serde_json::Value,
}

async fn render_adverse_notice(
    State(s): State<AppState>,
    Path((url, template)): Path<(String, String)>,
    Json(req): Json<AdverseNoticeRenderRequest>,
) -> ApiResult<Json<AdverseDecisionNoticeView>> {
    Ok(Json(
        s.services
            .governance
            .render_adverse_notice(&url, &template, &req.context)
            .await?,
    ))
}

async fn health(State(s): State<AppState>) -> Json<Vec<ServiceHealthView>> {
    Json(s.services.governance.health().await)
}
