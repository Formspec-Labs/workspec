use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use wos_runtime::runtime::CreateInstanceRequest;

use crate::AppState;
use crate::domain::provenance::ProvenanceResponse;
use crate::domain::{
    AvailableTransitionView, EvaluationResultView, InstanceResponse, ListQuery, PaginatedView,
    SubmitEventRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::services::provenance_service::row_to_response;
use crate::services::semantic_service::{Format as ExportFormat, SemanticService};
use crate::storage::InstanceQuery;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/instances", get(list).post(create))
        .route("/instances/{id}", get(get_one))
        .route("/instances/{id}/provenance", get(provenance))
        .route("/instances/{id}/provenance/export", get(export_provenance))
        .route("/instances/{id}/transitions", get(transitions))
        .route("/instances/{id}/events", post(submit_event))
        .route("/instances/{id}/drain", post(drain))
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
        .unwrap_or_else(|| format!("urn:wos:instance:{}", uuid::Uuid::new_v4()));

    let req = CreateInstanceRequest {
        instance_id,
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
async fn submit_event(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SubmitEventRequest>,
) -> ApiResult<Json<EvaluationResultView>> {
    // Capture previous configuration before we touch the runtime.
    let before = s.storage.get_instance(&id).await?.ok_or(ApiError::NotFound)?;
    let before_instance: wos_core::instance::CaseInstance =
        serde_json::from_value(before.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(e.to_string()))?;
    let previous_configuration = before_instance.configuration.clone();
    let case_state_before = before_instance.case_state.clone();

    let envelope = serde_json::json!({
        "event": req.event,
        "actor": req.actor_id,
        "data": req.data,
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

    // The first stored provenance row appended by this step is the "head" —
    // look it up by computing the tail count before vs after. The storage
    // layer writes provenance in order within a single atomic txn so
    // `last_provenance` with seq <= tail.seq + drain.provenance.len()
    // captures the right range; for the head we need the first of the new
    // tail, which is the one at seq = prev_tail_len + 1.
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

    Ok(Json(EvaluationResultView {
        previous_configuration,
        new_configuration,
        events_fired: drain
            .transitions
            .iter()
            .map(|t| t.event.clone())
            .collect(),
        head_record,
        case_state_mutations: mutations,
    }))
}

/// `POST /api/instances/:id/drain` — drain every queued event for this
/// instance, returning a summary per step.
async fn drain(
    State(s): State<AppState>,
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
