//! WOS Restate runtime adapter (`RuntimeOps` + Restate SDK).
//!
//! See [ADR 0084](../../../thoughts/adr/0084-wos-restate-durable-runtime-adapter.md) and
//! [`restate_virtual`](crate::restate_virtual) for the Virtual Object keying model.
//!
//! ## Environment (ingress mode)
//!
//! When **`WOS_RESTATE_INGRESS_URL`** is set to a Restate ingress base (for example
//! `http://127.0.0.1:8080`), [`RestateRuntimeAdapter::from_env`] delegates
//! `create_instance` / `load_instance` / `enqueue_event` / `drain_once` to the
//! `WosInstance` virtual object via HTTP. The Restate worker process must register the
//! same service (see [`restate_virtual::wos_instance_endpoint`]). Axum `wos-server` stays
//! on its own port; Restate uses [`restate_sdk::HttpServer`] on a worker port (ADR D2).

mod ingress_http;
mod instance_seed;
pub mod restate_virtual;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wos_core::instance::CaseInstance;
use wos_core::instance::PendingEvent;
use wos_core::provenance::ProvenanceRecord;
use wos_core::typeid;
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult, MigrationMap, MigrationOutcome};
use wos_runtime::{InMemoryStore, PersistDraftResult, RuntimeStore, TaskSubmissionResult};
use wos_runtime::{restate_signature_fixture_runtime, SharedInMemoryStore};
use wos_server_ports::runtime::{
    RuntimeAdapterError, RuntimeOps, RuntimeResult, SeamAccess, TimerCoord,
};

use ingress_http::RestateIngressClient;

#[derive(Default)]
struct RestateMemoryState {
    /// Full [`wos_runtime::store::RuntimeRecord`] per instance (WS-094 Phase 3 durable slice).
    records: HashMap<String, wos_runtime::store::RuntimeRecord>,
}

enum RestateRuntimeBackend {
    Memory(Arc<Mutex<RestateMemoryState>>),
    Ingress(RestateIngressClient),
}

/// [`RuntimeOps`] implementation: in-memory by default, or Restate ingress when configured.
pub struct RestateRuntimeAdapter {
    backend: RestateRuntimeBackend,
}

impl RestateRuntimeAdapter {
    /// In-memory adapter (default for unit tests and local `wos-server` scaffolds).
    pub fn new() -> Self {
        Self {
            backend: RestateRuntimeBackend::Memory(Arc::new(Mutex::new(
                RestateMemoryState::default(),
            ))),
        }
    }

    /// Delegates lifecycle calls to Restate ingress HTTP (`WOS_RESTATE_INGRESS_URL` / ADR D4).
    pub fn with_restate_ingress(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            backend: RestateRuntimeBackend::Ingress(RestateIngressClient {
                client,
                base_url: base_url.into(),
            }),
        }
    }

    /// Uses [`RestateRuntimeAdapter::with_restate_ingress`] when `WOS_RESTATE_INGRESS_URL` is set;
    /// otherwise [`RestateRuntimeAdapter::new`].
    pub fn from_env() -> Self {
        match std::env::var("WOS_RESTATE_INGRESS_URL") {
            Ok(base) if !base.trim().is_empty() => {
                Self::with_restate_ingress(reqwest::Client::new(), base)
            }
            _ => Self::new(),
        }
    }

    fn unsupported(op: &str) -> RuntimeAdapterError {
        RuntimeAdapterError::Message(format!(
            "WS-094: `{op}` is not yet supported by restate adapter"
        ))
    }
}

#[async_trait]
impl RuntimeOps for RestateRuntimeAdapter {
    async fn create_instance(&self, request: CreateInstanceRequest) -> RuntimeResult<CaseInstance> {
        match &self.backend {
            RestateRuntimeBackend::Ingress(client) => client.create_instance(&request).await,
            RestateRuntimeBackend::Memory(state) => {
                let mut guard = state.lock().map_err(|_| {
                    RuntimeAdapterError::Message("restate adapter state lock poisoned".into())
                })?;
                if !CaseInstance::is_case_id(&request.instance_id) {
                    return Err(RuntimeAdapterError::Message(
                        "restate adapter requires a WOS case TypeID (`tenant_case_<uuidv7>`) as instanceId (WS-094)"
                            .into(),
                    ));
                }
                if guard.records.contains_key(&request.instance_id) {
                    return Err(RuntimeAdapterError::Message(format!(
                        "instance `{}` already exists",
                        request.instance_id
                    )));
                }
                let shared = SharedInMemoryStore(Arc::new(Mutex::new(InMemoryStore::new())));
                let mut runtime = restate_signature_fixture_runtime(shared.clone());
                let instance = runtime
                    .create_instance(request)
                    .map_err(|e| RuntimeAdapterError::Message(e.to_string()))?;
                let record = {
                    let store = shared
                        .0
                        .lock()
                        .map_err(|_| RuntimeAdapterError::Message("store mutex poisoned".into()))?;
                    store
                        .load_record(&instance.instance_id)
                        .map_err(|e| RuntimeAdapterError::Message(e.to_string()))?
                };
                guard.records.insert(instance.instance_id.clone(), record);
                Ok(instance)
            }
        }
    }

    async fn load_instance(&self, instance_id: &str) -> RuntimeResult<CaseInstance> {
        match &self.backend {
            RestateRuntimeBackend::Ingress(client) => client.load_instance(instance_id).await,
            RestateRuntimeBackend::Memory(state) => {
                let guard = state.lock().map_err(|_| {
                    RuntimeAdapterError::Message("restate adapter state lock poisoned".into())
                })?;
                guard
                    .records
                    .get(instance_id)
                    .map(|r| r.instance.clone())
                    .ok_or_else(|| {
                        RuntimeAdapterError::Message(format!("instance `{instance_id}` not found"))
                    })
            }
        }
    }

    async fn enqueue_event(
        &self,
        instance_id: &str,
        event: serde_json::Value,
    ) -> RuntimeResult<()> {
        match &self.backend {
            RestateRuntimeBackend::Ingress(client) => {
                client.enqueue_event(instance_id, event).await
            }
            RestateRuntimeBackend::Memory(state) => {
                let pending: PendingEvent = serde_json::from_value(event)
                    .map_err(|e| RuntimeAdapterError::Message(format!("invalid event payload: {e}")))?;
                let mut guard = state.lock().map_err(|_| {
                    RuntimeAdapterError::Message("restate adapter state lock poisoned".into())
                })?;
                let record = guard.records.get_mut(instance_id).ok_or_else(|| {
                    RuntimeAdapterError::Message(format!("instance `{instance_id}` not found"))
                })?;
                record.instance.pending_events.push(pending);
                Ok(())
            }
        }
    }

    async fn drain_once(&self, instance_id: &str) -> RuntimeResult<DrainOnceResult> {
        match &self.backend {
            RestateRuntimeBackend::Ingress(client) => client.drain_once(instance_id).await,
            RestateRuntimeBackend::Memory(state) => {
                let mut guard = state.lock().map_err(|_| {
                    RuntimeAdapterError::Message("restate adapter state lock poisoned".into())
                })?;
                let record = guard.records.get(instance_id).cloned().ok_or_else(|| {
                    RuntimeAdapterError::Message(format!("instance `{instance_id}` not found"))
                })?;
                let shared = SharedInMemoryStore(Arc::new(Mutex::new(InMemoryStore::new())));
                {
                    let mut store = shared
                        .0
                        .lock()
                        .map_err(|_| RuntimeAdapterError::Message("store mutex poisoned".into()))?;
                    store
                        .create_record(record)
                        .map_err(|e| RuntimeAdapterError::Message(e.to_string()))?;
                }
                let mut runtime = restate_signature_fixture_runtime(shared.clone());
                let step = runtime
                    .drain_once(instance_id)
                    .map_err(|e| RuntimeAdapterError::Message(e.to_string()))?;
                let updated = {
                    let store = shared
                        .0
                        .lock()
                        .map_err(|_| RuntimeAdapterError::Message("store mutex poisoned".into()))?;
                    store
                        .load_record(instance_id)
                        .map_err(|e| RuntimeAdapterError::Message(e.to_string()))?
                };
                guard.records.insert(instance_id.to_string(), updated);
                Ok(step)
            }
        }
    }

    async fn drain_until_idle(&self, instance_id: &str) -> RuntimeResult<Vec<DrainOnceResult>> {
        let mut out = Vec::new();
        loop {
            let step = self.drain_once(instance_id).await?;
            let idle = step.processed_event.is_none();
            out.push(step);
            if idle {
                break;
            }
        }
        Ok(out)
    }

    async fn persist_task_draft(
        &self,
        _task_id: &str,
        _response: serde_json::Value,
        _actor_id: &str,
        _idempotency_token: Option<&str>,
    ) -> RuntimeResult<PersistDraftResult> {
        Err(Self::unsupported("persist_task_draft"))
    }

    async fn submit_task_response(
        &self,
        _task_id: &str,
        _response: serde_json::Value,
        _actor_id: &str,
        _idempotency_token: Option<&str>,
    ) -> RuntimeResult<TaskSubmissionResult> {
        Err(Self::unsupported("submit_task_response"))
    }

    async fn dismiss_task(&self, _task_id: &str, _reason: &str) -> RuntimeResult<()> {
        Err(Self::unsupported("dismiss_task"))
    }

    async fn load_provenance_window(
        &self,
        _instance_id: &str,
        _offset: u64,
        _limit: usize,
    ) -> RuntimeResult<Vec<ProvenanceRecord>> {
        Err(Self::unsupported("load_provenance_window"))
    }

    async fn migrate_instance(
        &self,
        _instance_id: &str,
        _target_definition_version: &str,
        _migration_map: MigrationMap,
        _operator_actor_id: Option<&str>,
    ) -> RuntimeResult<MigrationOutcome> {
        Err(Self::unsupported("migrate_instance"))
    }
}

impl SeamAccess for RestateRuntimeAdapter {
    type SignerError = std::convert::Infallible;
    type RendererError = std::convert::Infallible;

    fn signer(
        &self,
    ) -> &(dyn wos_core::traits::ProvenanceSigner<Error = Self::SignerError> + Send + Sync) {
        panic!("WS-094: signer seam not wired for restate adapter")
    }

    fn renderer(
        &self,
    ) -> &(dyn wos_core::traits::ReportRenderer<Error = Self::RendererError> + Send + Sync) {
        panic!("WS-094: renderer seam not wired for restate adapter")
    }
}

#[async_trait]
impl TimerCoord for RestateRuntimeAdapter {
    async fn tick_once(&self) -> RuntimeResult<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(instance_id: &str) -> CreateInstanceRequest {
        CreateInstanceRequest {
            definition_url: "urn:test:signature-runtime".into(),
            definition_version: "1.0.0".into(),
            instance_id: instance_id.into(),
            tenant: None,
            initial_case_state: None,
        }
    }

    #[tokio::test]
    async fn create_load_enqueue_and_drain_lifecycle_works() {
        let case_id = typeid::mint_case_id();
        let adapter = RestateRuntimeAdapter::new();
        let created = adapter
            .create_instance(request(&case_id))
            .await
            .expect("create should succeed");
        assert_eq!(created.configuration, vec!["draft"]);

        let loaded = adapter
            .load_instance(&case_id)
            .await
            .expect("load should succeed");
        assert_eq!(loaded.instance_id, case_id);

        adapter
            .enqueue_event(
                &case_id,
                serde_json::json!({
                    "event": "start",
                    "actorId": "system:test",
                    "data": {},
                    "timestamp": "2026-01-01T00:00:00Z"
                }),
            )
            .await
            .expect("enqueue should succeed");

        let steps = adapter
            .drain_until_idle(&case_id)
            .await
            .expect("drain should succeed");
        assert!(steps.len() >= 2);
        assert!(steps[0].processed_event.is_some());
        assert!(steps.last().expect("tail step").processed_event.is_none());
    }

    #[tokio::test]
    async fn unsupported_ops_fail_explicitly() {
        let adapter = RestateRuntimeAdapter::new();
        let case_id = typeid::mint_case_id();
        adapter
            .create_instance(request(&case_id))
            .await
            .expect("instance create should succeed");

        let err = adapter
            .persist_task_draft("task-1", serde_json::json!({}), "actor-1", Some("idem-1"))
            .await
            .expect_err("persist_task_draft should be unsupported");
        assert!(err.to_string().contains("not yet supported"));

        let err = adapter
            .load_provenance_window(&case_id, 0, 10)
            .await
            .expect_err("load_provenance_window should be unsupported");
        assert!(err.to_string().contains("not yet supported"));

        let err = adapter
            .migrate_instance(
                &case_id,
                "1.1.0",
                MigrationMap::default(),
                None,
            )
            .await
            .expect_err("migrate_instance should be unsupported");
        assert!(err.to_string().contains("not yet supported"));
        assert!(err.to_string().contains("migrate_instance"));
    }

    /// Manual / CI-with-Restate: run with `WOS_RESTATE_IT_URL=http://127.0.0.1:8080` after registering the worker.
    #[tokio::test]
    #[ignore = "requires a running Restate cluster and registered WosInstance worker (WS-094 Phase 4)"]
    async fn ingress_create_load_probe_smoke() {
        let base = std::env::var("WOS_RESTATE_IT_URL").expect("set WOS_RESTATE_IT_URL for this test");
        let client = reqwest::Client::new();
        let adapter = RestateRuntimeAdapter::with_restate_ingress(client.clone(), base.clone());
        let id = typeid::mint_case_id();
        let _ = adapter
            .create_instance(CreateInstanceRequest {
                definition_url: "urn:test:signature-runtime".into(),
                definition_version: "1.0.0".into(),
                instance_id: id.clone(),
                tenant: None,
                initial_case_state: None,
            })
            .await
            .expect("ingress create");

        let loaded = adapter.load_instance(&id).await.expect("ingress load");
        assert_eq!(loaded.instance_id, id);

        let probe_url = crate::restate_virtual::ingress_invoke_url(&base, &id, "probe");
        let probe: String = client
            .post(probe_url)
            .header("content-type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("probe send")
            .json()
            .await
            .expect("probe json");
        assert_eq!(probe, id);
    }
}
