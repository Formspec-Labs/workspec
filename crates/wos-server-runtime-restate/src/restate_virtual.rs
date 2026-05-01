//! Restate [`restate_sdk::object`] surface for WOS instances (ADR 0084 D1).
//!
//! Virtual-object handlers persist [`CaseInstance`] and a pending-event queue in Restate K/V.
//! The in-memory [`crate::RestateRuntimeAdapter`](crate::RestateRuntimeAdapter) remains available;
//! when `WOS_RESTATE_INGRESS_URL` is set, the adapter delegates to Restate ingress HTTP (Phase 2).

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use urlencoding::encode;

use crate::instance_seed;
use wos_core::instance::{CaseInstance, PendingEvent};
use wos_core::provenance::ProvenanceRecord;
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult};
use wos_runtime::store::{RuntimeRecord, runtime_aux_from_json, runtime_aux_to_json};
use wos_runtime::{restate_signature_fixture_runtime, SharedInMemoryStore};
use wos_runtime::{InMemoryStore, RuntimeStore};

/// Stable Restate service name for HTTP ingress (`/{service}/{key}/{handler}`).
pub const WOS_INSTANCE_SERVICE: &str = "WosInstance";

/// One Restate Virtual Object key equals one WOS `instance_id` (ADR 0084 D1).
#[restate_sdk::object]
#[name = "WosInstance"]
pub trait WosInstanceVirtualObject {
    /// Shared handler for health checks and wiring tests (spec R-1.2).
    #[shared]
    async fn probe() -> Result<String, HandlerError>;

    /// Materializes instance state on first create (exclusive).
    #[name = "createInstance"]
    async fn create_instance(body: Json<CreateIngressBody>) -> Result<Json<CaseInstance>, TerminalError>;

    /// Returns the stored [`CaseInstance`] snapshot (queue lives under separate K/V).
    #[shared]
    #[name = "loadInstance"]
    async fn load_instance() -> Result<Json<CaseInstance>, TerminalError>;

    /// Appends one pending event to the durable queue.
    #[name = "enqueueEvent"]
    async fn enqueue_event(event: Json<serde_json::Value>) -> Result<(), TerminalError>;

    /// Pops one queued event and returns a stub [`wos_runtime::runtime::DrainOnceResult`](wos_runtime::runtime::DrainOnceResult) JSON (camelCase).
    #[name = "drainOnce"]
    async fn drain_once() -> Result<Json<serde_json::Value>, TerminalError>;
}

/// JSON body for [`WosInstanceVirtualObject::create_instance`] (matches `CreateInstanceRequest` wire shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIngressBody {
    /// Stable WOS instance identifier (must match Restate virtual object key).
    pub instance_id: String,
    pub definition_url: String,
    pub definition_version: String,
    #[serde(default)]
    pub tenant: Option<String>,
    #[serde(default)]
    pub initial_case_state: Option<serde_json::Value>,
}

impl CreateIngressBody {
    /// Converts to a runtime [`CreateInstanceRequest`].
    pub fn into_create_request(self) -> CreateInstanceRequest {
        CreateInstanceRequest {
            instance_id: self.instance_id,
            tenant: self.tenant,
            definition_url: self.definition_url,
            definition_version: self.definition_version,
            initial_case_state: self.initial_case_state,
        }
    }
}

impl From<&CreateInstanceRequest> for CreateIngressBody {
    fn from(r: &CreateInstanceRequest) -> Self {
        Self {
            instance_id: r.instance_id.clone(),
            definition_url: r.definition_url.clone(),
            definition_version: r.definition_version.clone(),
            tenant: r.tenant.clone(),
            initial_case_state: r.initial_case_state.clone(),
        }
    }
}

fn drain_once_result_json(step: &DrainOnceResult) -> serde_json::Value {
    let transitions: Vec<serde_json::Value> = step
        .transitions
        .iter()
        .map(|t| {
            serde_json::json!({
                "from": t.from,
                "to": t.to,
                "event": t.event,
                "tags": t.tags,
            })
        })
        .collect();
    let provenance: Vec<serde_json::Value> = step
        .provenance
        .iter()
        .filter_map(|p| serde_json::to_value(p).ok())
        .collect();
    let guard_evaluations: Vec<serde_json::Value> = step
        .guard_evaluations
        .iter()
        .filter_map(|g| serde_json::to_value(g).ok())
        .collect();
    serde_json::json!({
        "processedEvent": step.processed_event,
        "processedEventToken": step.processed_event_token,
        "transitions": transitions,
        "provenance": provenance,
        "createdTaskIds": step.created_task_ids,
        "emittedEvents": step.emitted_events,
        "guardEvaluations": guard_evaluations,
    })
}

fn persist_runtime_record(ctx: &ObjectContext<'_>, record: &RuntimeRecord) -> Result<(), TerminalError> {
    let instance_bytes = serde_json::to_vec(&record.instance)
        .map_err(|e| TerminalError::new(format!("serialize instance: {e}")))?;
    let provenance_bytes = serde_json::to_vec(&record.provenance_log)
        .map_err(|e| TerminalError::new(format!("serialize provenance: {e}")))?;
    let aux_val = runtime_aux_to_json(record);
    let aux_bytes =
        serde_json::to_vec(&aux_val).map_err(|e| TerminalError::new(format!("serialize aux: {e}")))?;
    ctx.set(instance_seed::STATE_INSTANCE, instance_bytes);
    ctx.set(instance_seed::STATE_PROVENANCE_V1, provenance_bytes);
    ctx.set(instance_seed::STATE_AUX_V1, aux_bytes);
    let empty_queue = serde_json::to_vec(&Vec::<PendingEvent>::new())
        .map_err(|e| TerminalError::new(format!("serialize queue: {e}")))?;
    ctx.set(instance_seed::STATE_QUEUE, empty_queue);
    Ok(())
}

async fn load_runtime_record(ctx: &ObjectContext<'_>) -> Result<Option<RuntimeRecord>, TerminalError> {
    let Some(inst_bytes) = ctx
        .get::<Vec<u8>>(instance_seed::STATE_INSTANCE)
        .await?
    else {
        return Ok(None);
    };
    let instance: CaseInstance = serde_json::from_slice(&inst_bytes)
        .map_err(|e| TerminalError::new(format!("deserialize instance: {e}")))?;
    let prov_bytes = ctx
        .get::<Vec<u8>>(instance_seed::STATE_PROVENANCE_V1)
        .await?
        .unwrap_or_else(|| br#"[]"#.to_vec());
    let provenance_log: Vec<ProvenanceRecord> = serde_json::from_slice(&prov_bytes)
        .map_err(|e| TerminalError::new(format!("deserialize provenance: {e}")))?;
    let aux_bytes = ctx
        .get::<Vec<u8>>(instance_seed::STATE_AUX_V1)
        .await?
        .unwrap_or_else(|| b"{}".to_vec());
    let aux_val: serde_json::Value = serde_json::from_slice(&aux_bytes)
        .map_err(|e| TerminalError::new(format!("deserialize aux: {e}")))?;
    let aux = runtime_aux_from_json(&aux_val);
    let mut record = RuntimeRecord {
        instance,
        provenance_log,
        step_results: aux.step_results,
        artifacts: aux.artifacts,
        replay_entries: aux.replay_entries,
    };
    if let Some(queue_bytes) = ctx.get::<Vec<u8>>(instance_seed::STATE_QUEUE).await? {
        let mut legacy_queue: Vec<PendingEvent> = serde_json::from_slice(&queue_bytes)
            .map_err(|e| TerminalError::new(format!("deserialize queue: {e}")))?;
        if !legacy_queue.is_empty() {
            record.instance.pending_events.append(&mut legacy_queue);
        }
    }
    Ok(Some(record))
}

/// Default [`WosInstanceVirtualObject`] implementation (journal-backed instance shell + queue).
pub struct WosInstanceVirtualObjectImpl;

impl WosInstanceVirtualObject for WosInstanceVirtualObjectImpl {
    async fn probe(&self, ctx: SharedObjectContext<'_>) -> Result<String, HandlerError> {
        Ok(ctx.key().to_string())
    }

    async fn create_instance(
        &self,
        ctx: ObjectContext<'_>,
        body: Json<CreateIngressBody>,
    ) -> Result<Json<CaseInstance>, TerminalError> {
        let body = body.into_inner();
        if ctx.key() != body.instance_id {
            return Err(TerminalError::new(
                "virtual object key must match payload instanceId",
            ));
        }
        if ctx
            .get::<Vec<u8>>(instance_seed::STATE_INSTANCE)
            .await?
            .is_some()
        {
            return Err(TerminalError::new("instance already exists"));
        }
        let request = body.into_create_request();
        let shared = SharedInMemoryStore(std::sync::Arc::new(std::sync::Mutex::new(
            InMemoryStore::new(),
        )));
        let mut runtime = restate_signature_fixture_runtime(shared.clone());
        let instance = runtime
            .create_instance(request)
            .map_err(|e| TerminalError::new(e.to_string()))?;
        let record = {
            let store = shared.0.lock().map_err(|_| TerminalError::new("store mutex poisoned"))?;
            store
                .load_record(&instance.instance_id)
                .map_err(|e| TerminalError::new(e.to_string()))?
        };
        persist_runtime_record(&ctx, &record)?;
        Ok(Json(instance))
    }

    async fn load_instance(&self, ctx: SharedObjectContext<'_>) -> Result<Json<CaseInstance>, TerminalError> {
        let Some(bytes) = ctx
            .get::<Vec<u8>>(instance_seed::STATE_INSTANCE)
            .await?
        else {
            return Err(TerminalError::new_with_code(404, "instance not found"));
        };
        let instance: CaseInstance = serde_json::from_slice(&bytes)
            .map_err(|e| TerminalError::new(format!("deserialize instance: {e}")))?;
        Ok(Json(instance))
    }

    async fn enqueue_event(
        &self,
        ctx: ObjectContext<'_>,
        event: Json<serde_json::Value>,
    ) -> Result<(), TerminalError> {
        let event = event.into_inner();
        let pending: PendingEvent = serde_json::from_value(event)
            .map_err(|e| TerminalError::new(format!("invalid event payload: {e}")))?;
        let mut record = load_runtime_record(&ctx)
            .await?
            .ok_or_else(|| TerminalError::new_with_code(404, "instance not found"))?;
        record.instance.pending_events.push(pending);
        persist_runtime_record(&ctx, &record)?;
        Ok(())
    }

    async fn drain_once(&self, ctx: ObjectContext<'_>) -> Result<Json<serde_json::Value>, TerminalError> {
        let record = load_runtime_record(&ctx)
            .await?
            .ok_or_else(|| TerminalError::new_with_code(404, "instance not found"))?;
        let instance_id = record.instance.instance_id.clone();
        let shared = SharedInMemoryStore(std::sync::Arc::new(std::sync::Mutex::new(
            InMemoryStore::new(),
        )));
        {
            let mut store = shared
                .0
                .lock()
                .map_err(|_| TerminalError::new("store mutex poisoned"))?;
            store
                .create_record(record)
                .map_err(|e| TerminalError::new(e.to_string()))?;
        }
        let mut runtime = restate_signature_fixture_runtime(shared.clone());
        let step = runtime
            .drain_once(&instance_id)
            .map_err(|e| TerminalError::new(e.to_string()))?;
        let updated = {
            let store = shared
                .0
                .lock()
                .map_err(|_| TerminalError::new("store mutex poisoned"))?;
            store
                .load_record(&instance_id)
                .map_err(|e| TerminalError::new(e.to_string()))?
        };
        persist_runtime_record(&ctx, &updated)?;
        Ok(Json(drain_once_result_json(&step)))
    }
}

/// Builds a Restate [`Endpoint`] that exposes [`WosInstanceVirtualObject`] (ADR D2).
pub fn wos_instance_endpoint() -> Endpoint {
    Endpoint::builder()
        .bind(WosInstanceVirtualObjectImpl.serve())
        .build()
}

/// Builds an HTTP ingress URL for a virtual-object handler (Restate public HTTP API).
pub fn ingress_invoke_url(ingress_base: &str, instance_id: &str, handler: &str) -> String {
    let base = ingress_base.trim_end_matches('/');
    let enc = encode(instance_id);
    format!("{base}/{}/{enc}/{handler}", WOS_INSTANCE_SERVICE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use restate_sdk::service::Discoverable;

    #[test]
    fn wos_instance_endpoint_builds() {
        let _endpoint = wos_instance_endpoint();
    }

    #[test]
    fn discovery_service_name_is_stable_for_ingress() {
        let svc = ServeWosInstanceVirtualObject::<WosInstanceVirtualObjectImpl>::discover();
        assert_eq!(svc.name.to_string(), WOS_INSTANCE_SERVICE);
    }

    #[test]
    fn ingress_url_encodes_object_key() {
        let u = ingress_invoke_url("http://127.0.0.1:8080", "urn:wos:x:y", "probe");
        assert!(u.contains("urn%3Awos%3Ax%3Ay"), "{u}");
        assert!(u.ends_with("/probe"));
    }
}
