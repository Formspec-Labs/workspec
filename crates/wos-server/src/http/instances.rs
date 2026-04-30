use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;
use wos_runtime::runtime::CreateInstanceRequest;

use crate::AppState;
use crate::auth::{Adjudicator, RequireRole, Supervisor};
use crate::domain::provenance::ProvenanceResponse;
use crate::domain::{
    AvailableTransitionView, EvaluationResultView, InstanceResponse, ListQuery, PaginatedView,
    SubmitEventRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::services::hold_service::{HoldService, HoldServiceError};
use crate::services::provenance_service::{row_to_response, verify_chain};
use crate::services::semantic_service::{Format as ExportFormat, SemanticService};
use crate::storage::InstanceQuery;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/instances", get(list).post(create))
        .route("/instances/{id}", get(get_one))
        .route("/instances/{id}/explain", get(explain))
        .route("/instances/{id}/provenance", get(provenance))
        .route("/instances/{id}/provenance/verify", get(verify_provenance))
        .route("/instances/{id}/provenance/export", get(export_provenance))
        .route("/instances/{id}/transitions", get(transitions))
        .route("/instances/{id}/events", post(submit_event))
        .route("/instances/{id}/drain", post(drain))
        .route("/instances/{id}/holds", get(list_holds).post(create_hold))
        .route("/instances/{id}/holds/{hold_idx}", delete(release_hold))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainQuery {
    pub transition_id: String,
    #[serde(default)]
    pub tags: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResponse {
    #[serde(flatten)]
    pub explanation: wos_core::explain::Explanation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered: Option<String>,
}

async fn explain(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ExplainQuery>,
) -> ApiResult<Json<ExplainResponse>> {
    let _instance = s.storage.get_instance(&id).await?.ok_or(ApiError::NotFound)?;

    let prov_responses = s.services.provenance.list(&id).await?;
    let records: Vec<wos_core::provenance::ProvenanceRecord> =
        prov_responses.into_iter().map(|r| r.record).collect();

    let tags: Vec<String> = q
        .tags
        .as_deref()
        .map(|t| {
            t.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_else(|| vec!["adverse-decision".into()]);

    let assembled_at = chrono::Utc::now().to_rfc3339();
    let explanation = wos_core::explain::assemble_explanation(
        &records,
        &q.transition_id,
        &tags,
        &assembled_at,
    );

    let explanation_value = serde_json::to_value(&explanation)
        .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?;

    let rendered = s
        .runtime
        .renderer()
        .render_explanation(&explanation_value, "default")
        .ok();

    Ok(Json(ExplainResponse {
        explanation,
        rendered,
    }))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PaginatedView<InstanceResponse>>> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(25);
    let storage_query = InstanceQuery {
        status: ListQuery::csv(&q.status),
        impact_level: ListQuery::csv(&q.impact_level),
        definition_url: q.definition_url.map(|d| vec![d]),
        page,
        page_size,
    };
    let page_result = s.storage.list_instances(storage_query).await?;
    let mut items = Vec::with_capacity(page_result.items.len());
    for row in &page_result.items {
        items.push(s.services.instance.to_response(row).await?);
    }
    Ok(Json(PaginatedView::new(
        items,
        page_result.total,
        page_result.page,
        page_result.page_size,
    )))
}

async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<InstanceResponse>> {
    let row = s
        .storage
        .get_instance(&id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(s.services.instance.to_response(&row).await?))
}

/// `POST /api/instances` — create a fresh case instance from a
/// `{ definitionUrl, definitionVersion?, instanceId?, initialCaseState? }`
/// body. Wos-runtime assigns and enters the initial state, writes any
/// onEntry provenance, and returns the canonical `CaseInstance`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceBody {
    pub definition_url: String,
    pub definition_version: Option<String>,
    pub instance_id: Option<String>,
    #[serde(default)]
    pub initial_case_state: Option<serde_json::Value>,
}

async fn create(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Json(body): Json<CreateInstanceBody>,
) -> ApiResult<Json<InstanceResponse>> {
    let kernel = s
        .services
        .bundle
        .get(&body.definition_url)
        .await
        .ok_or_else(|| ApiError::NotFound)?;
    let version = body
        .definition_version
        .unwrap_or_else(|| kernel.version.clone());
    let instance_id = body
        .instance_id
        .unwrap_or_else(|| format!("urn:wos:instance:{}", uuid::Uuid::now_v7()));

    let req = CreateInstanceRequest {
        instance_id,
        tenant: None,
        definition_url: body.definition_url,
        definition_version: version,
        initial_case_state: body.initial_case_state,
    };
    let instance = s.runtime.create_instance(req).await?;
    Ok(Json(
        s.services
            .instance
            .from_instance(instance, &kernel.impact_level)
            .await?,
    ))
}

async fn provenance(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ProvenanceResponse>>> {
    Ok(Json(s.services.provenance.list(&id).await?))
}

/// Response shape for `GET /api/instances/{id}/provenance/verify`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainVerifyResponse {
    pub valid: bool,
    pub broken_at: Option<i64>,
}

/// `GET /api/instances/:id/provenance/verify` — verify the sha256 hash-chain
/// integrity of every provenance row for the instance. Returns `{ valid,
/// brokenAt }` where `brokenAt` is the 1-indexed `seq` of the first broken
/// link, or `null` when the chain is clean.
async fn verify_provenance(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ChainVerifyResponse>> {
    let rows = s.storage.list_provenance(&id).await?;
    if rows.is_empty() {
        return Ok(Json(ChainVerifyResponse {
            valid: true,
            broken_at: None,
        }));
    }
    match verify_chain(&rows) {
        Ok(()) => Ok(Json(ChainVerifyResponse {
            valid: true,
            broken_at: None,
        })),
        Err(idx) => Ok(Json(ChainVerifyResponse {
            valid: false,
            broken_at: Some(rows[idx].seq),
        })),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportQuery {
    /// `prov-o` | `xes` | `ocel`. Defaults to `prov-o`.
    #[serde(default)]
    pub format: Option<ExportFormat>,
    /// PROV-O IRI namespace prefix. Must end with `:` or `/`.
    pub namespace: Option<String>,
}

/// `GET /api/instances/:id/provenance/export?format=prov-o|xes|ocel` —
/// Semantic Profile export. Serves PROV-O as `application/ld+json`,
/// XES as `application/xml`, OCEL as `application/json`.
async fn export_provenance(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let format = q.format.unwrap_or(ExportFormat::ProvO);
    let payload =
        SemanticService::export(&s.services.provenance, &id, format, q.namespace).await?;
    let ct = payload.content_type().to_string();
    let body = payload.body();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&ct).unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Ok((headers, body))
}

async fn transitions(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<AvailableTransitionView>>> {
    Ok(Json(s.services.eval.available_transitions(&id).await?))
}

/// `POST /api/instances/:id/events` — enqueue an event and immediately
/// `drain_once` so the response carries the transitions / provenance /
/// case-state mutations produced by this step.
///
/// When `idempotencyToken` is present, a duplicate request with the same
/// `(instance_id, token)` pair returns the cached result without
/// re-processing. The Restate adapter handles this natively via journaled
/// execution; this cache is the reference-server defense-in-depth.
async fn submit_event(
    State(s): State<AppState>,
    _: RequireRole<Adjudicator>,
    Path(id): Path<String>,
    Json(req): Json<SubmitEventRequest>,
) -> ApiResult<Json<EvaluationResultView>> {
    if let Some(ref token) = req.idempotency_token {
        let cache_key = format!("{id}::{token}");
        if let Ok(cache) = s.event_idempotency.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(Json(cached.clone()));
            }
        }
    }

    // Capture previous configuration before we touch the runtime.
    let before = s.storage.get_instance(&id).await?.ok_or(ApiError::NotFound)?;
    let before_instance: wos_core::instance::CaseInstance =
        serde_json::from_value(before.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?;
    let previous_configuration = before_instance.configuration.clone();
    let case_state_before = before_instance.case_state.clone();

    let envelope = serde_json::json!({
        "event": req.event,
        "actorId": req.actor_id,
        "data": req.data,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "idempotencyToken": req.idempotency_token,
    });
    s.runtime
        .enqueue_event(&id, envelope)
        .await
        ?;
    let drain = s
        .runtime
        .drain_once(&id)
        .await
        ?;

    let after = s.storage.get_instance(&id).await?.ok_or(ApiError::NotFound)?;
    let after_instance: wos_core::instance::CaseInstance =
        serde_json::from_value(after.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?;
    let new_configuration = after_instance.configuration.clone();
    let mutations = diff_case_state(&case_state_before, &after_instance.case_state);

    let head_record = match s.storage.last_provenance(&id).await? {
        Some(_tail) if drain.provenance.is_empty() => None,
        Some(tail) => {
            let head_seq = tail.seq - drain.provenance.len() as i64 + 1;
            let rows = s.storage.list_provenance(&id).await?;
            rows.iter()
                .find(|r| r.seq == head_seq)
                .map(row_to_response)
                .transpose()?
        }
        None => None,
    };

    let result = EvaluationResultView {
        previous_configuration,
        new_configuration,
        events_fired: drain
            .transitions
            .iter()
            .map(|t| t.event.clone())
            .collect(),
        head_record,
        case_state_mutations: mutations,
    };

    if let Some(ref token) = req.idempotency_token {
        let cache_key = format!("{id}::{token}");
        if let Ok(mut cache) = s.event_idempotency.lock() {
            cache.entry(cache_key).or_insert(result.clone());
        }
    }

    Ok(Json(result))
}

/// `POST /api/instances/:id/drain` — drain every queued event for this
/// instance, returning a summary per step.
async fn drain(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<DrainStepSummary>>> {
    let results = s
        .runtime
        .drain_until_idle(&id)
        .await
        ?;
    Ok(Json(
        results
            .into_iter()
            .map(|d| DrainStepSummary {
                processed_event: d.processed_event,
                transitions_count: d.transitions.len(),
                provenance_count: d.provenance.len(),
                created_task_ids: d.created_task_ids,
                emitted_events: d.emitted_events,
            })
            .collect(),
    ))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainStepSummary {
    pub processed_event: Option<String>,
    pub transitions_count: usize,
    pub provenance_count: usize,
    pub created_task_ids: Vec<String>,
    pub emitted_events: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHoldRequest {
    pub hold_type: String,
    pub resume_trigger: String,
    #[serde(default)]
    pub expected_end: Option<String>,
    #[serde(default)]
    pub hold_state: Option<String>,
}

async fn list_holds(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let holds = HoldService::list(&s.storage, &id).await?;
    let json = holds
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json))
}

async fn create_hold(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path(id): Path<String>,
    Json(req): Json<CreateHoldRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Resolve the default `holdState` (configuration[0]) before mutating.
    // Done outside the service so HoldService stays a pure typed CRUD —
    // append takes a fully-formed ActiveHold.
    let row = s
        .storage
        .get_instance(&id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let instance: wos_core::instance::CaseInstance =
        serde_json::from_value(row.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?;
    let default_state = instance.configuration.first().cloned();
    let hold = wos_core::instance::ActiveHold {
        hold_type: req.hold_type,
        started_at: chrono::Utc::now().to_rfc3339(),
        expected_end: req.expected_end,
        resume_trigger: req.resume_trigger,
        hold_state: req.hold_state.or(default_state),
    };
    let idx = HoldService::append(&s.storage, &id, hold).await?;
    Ok(Json(
        serde_json::json!({ "ok": true, "holdIndex": idx }),
    ))
}

async fn release_hold(
    State(s): State<AppState>,
    _: RequireRole<Supervisor>,
    Path((id, hold_idx)): Path<(String, usize)>,
) -> ApiResult<Json<serde_json::Value>> {
    match HoldService::release(&s.storage, &id, hold_idx).await {
        Ok(released) => {
            let released_value = serde_json::to_value(released)?;
            Ok(Json(
                serde_json::json!({ "ok": true, "released": released_value }),
            ))
        }
        Err(HoldServiceError::NotFound { .. }) => Err(ApiError::NotFound),
        Err(HoldServiceError::Storage(other)) => Err(other.into()),
    }
}

fn diff_case_state(before: &serde_json::Value, after: &serde_json::Value) -> serde_json::Value {
    let b = before.as_object();
    let a = after.as_object();
    let mut out = serde_json::Map::new();
    if let (Some(b), Some(a)) = (b, a) {
        for (k, v) in a {
            if b.get(k) != Some(v) {
                out.insert(k.clone(), v.clone());
            }
        }
        for k in b.keys() {
            if !a.contains_key(k) {
                out.insert(k.clone(), serde_json::Value::Null);
            }
        }
    } else {
        out.insert("_replaced".into(), after.clone());
    }
    serde_json::Value::Object(out)
}
