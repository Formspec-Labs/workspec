use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_core::instance::PendingEvent;
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult};
use wos_runtime::{PersistDraftResult, TaskSubmissionResult};
use wos_server_ports::runtime::{
    RuntimeAdapterError, RuntimeOps, RuntimeResult, SeamAccess, TimerCoord,
};

#[derive(Default)]
struct RestateState {
    instances: HashMap<String, CaseInstance>,
    queue: HashMap<String, VecDeque<PendingEvent>>,
}

pub struct RestateRuntimeAdapter {
    state: Arc<Mutex<RestateState>>,
}

impl RestateRuntimeAdapter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RestateState::default())),
        }
    }

    fn unsupported(op: &str) -> RuntimeAdapterError {
        RuntimeAdapterError::Message(format!("WS-094: `{op}` is not yet supported by restate adapter"))
    }
}

#[async_trait]
impl RuntimeOps for RestateRuntimeAdapter {
    async fn create_instance(&self, request: CreateInstanceRequest) -> RuntimeResult<CaseInstance> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| RuntimeAdapterError::Message("restate adapter state lock poisoned".into()))?;
        if guard.instances.contains_key(&request.instance_id) {
            return Err(RuntimeAdapterError::Message(format!(
                "instance `{}` already exists",
                request.instance_id
            )));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let instance: CaseInstance = serde_json::from_value(serde_json::json!({
            "instanceId": request.instance_id,
            "definitionUrl": request.definition_url,
            "definitionVersion": request.definition_version,
            "configuration": ["intake"],
            "caseState": request.initial_case_state.unwrap_or_else(|| serde_json::json!({})),
            "provenancePosition": 0,
            "nextTaskSequence": 0,
            "timers": [],
            "activeTasks": [],
            "historyStore": {},
            "compensationLogs": {},
            "status": "active",
            "pendingEvents": [],
            "createdAt": now,
            "updatedAt": now,
            "firedMilestones": [],
            "pendingCallbacks": {},
            "extensions": {}
        }))
        .map_err(|e| RuntimeAdapterError::Message(format!("failed to build instance: {e}")))?;
        guard
            .instances
            .insert(instance.instance_id.clone(), instance.clone());
        guard
            .queue
            .insert(instance.instance_id.clone(), VecDeque::new());
        Ok(instance)
    }
    async fn load_instance(&self, instance_id: &str) -> RuntimeResult<CaseInstance> {
        let guard = self
            .state
            .lock()
            .map_err(|_| RuntimeAdapterError::Message("restate adapter state lock poisoned".into()))?;
        guard
            .instances
            .get(instance_id)
            .cloned()
            .ok_or_else(|| RuntimeAdapterError::Message(format!("instance `{instance_id}` not found")))
    }
    async fn enqueue_event(&self, instance_id: &str, event: serde_json::Value) -> RuntimeResult<()> {
        let pending: PendingEvent = serde_json::from_value(event)
            .map_err(|e| RuntimeAdapterError::Message(format!("invalid event payload: {e}")))?;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| RuntimeAdapterError::Message("restate adapter state lock poisoned".into()))?;
        let queue = guard
            .queue
            .get_mut(instance_id)
            .ok_or_else(|| RuntimeAdapterError::Message(format!("instance `{instance_id}` not found")))?;
        queue.push_back(pending);
        Ok(())
    }
    async fn drain_once(&self, instance_id: &str) -> RuntimeResult<DrainOnceResult> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| RuntimeAdapterError::Message("restate adapter state lock poisoned".into()))?;
        let queue = guard
            .queue
            .get_mut(instance_id)
            .ok_or_else(|| RuntimeAdapterError::Message(format!("instance `{instance_id}` not found")))?;
        let Some(_event) = queue.pop_front() else {
            return Ok(DrainOnceResult {
                processed_event: None,
                processed_event_token: None,
                transitions: vec![],
                provenance: vec![],
                created_task_ids: vec![],
                emitted_events: vec![],
                guard_evaluations: vec![],
            });
        };
        Ok(DrainOnceResult {
            processed_event: Some("event".to_string()),
            processed_event_token: None,
            transitions: vec![],
            provenance: vec![],
            created_task_ids: vec![],
            emitted_events: vec![],
            guard_evaluations: vec![],
        })
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
}

impl SeamAccess for RestateRuntimeAdapter {
    type SignerError = std::convert::Infallible;
    type RendererError = std::convert::Infallible;

    fn signer(&self) -> &(dyn wos_core::traits::ProvenanceSigner<Error = Self::SignerError> + Send + Sync) {
        panic!("WS-094: signer seam not wired for restate adapter")
    }
    fn renderer(&self) -> &(dyn wos_core::traits::ReportRenderer<Error = Self::RendererError> + Send + Sync) {
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
            definition_url: "urn:wos:workflow:test:1.0.0".into(),
            definition_version: "1.0.0".into(),
            instance_id: instance_id.into(),
            tenant: None,
            initial_case_state: None,
        }
    }

    #[tokio::test]
    async fn create_load_enqueue_and_drain_lifecycle_works() {
        let adapter = RestateRuntimeAdapter::new();
        let created = adapter
            .create_instance(request("urn:wos:instance:restate:test"))
            .await
            .expect("create should succeed");
        assert_eq!(created.configuration, vec!["intake"]);

        let loaded = adapter
            .load_instance("urn:wos:instance:restate:test")
            .await
            .expect("load should succeed");
        assert_eq!(loaded.instance_id, "urn:wos:instance:restate:test");

        adapter
            .enqueue_event(
                "urn:wos:instance:restate:test",
                serde_json::json!({
                    "event": "case.submit",
                    "actorId": "system:test",
                    "data": {},
                    "timestamp": "2026-01-01T00:00:00Z"
                }),
            )
            .await
            .expect("enqueue should succeed");

        let steps = adapter
            .drain_until_idle("urn:wos:instance:restate:test")
            .await
            .expect("drain should succeed");
        assert!(steps.len() >= 2);
        assert!(steps[0].processed_event.is_some());
        assert!(steps.last().expect("tail step").processed_event.is_none());
    }

    #[tokio::test]
    async fn unsupported_ops_fail_explicitly() {
        let adapter = RestateRuntimeAdapter::new();
        adapter
            .create_instance(request("urn:wos:instance:restate:unsupported"))
            .await
            .expect("instance create should succeed");

        let err = adapter
            .persist_task_draft(
                "task-1",
                serde_json::json!({}),
                "actor-1",
                Some("idem-1"),
            )
            .await
            .expect_err("persist_task_draft should be unsupported");
        assert!(err.to_string().contains("not yet supported"));

        let err = adapter
            .load_provenance_window("urn:wos:instance:restate:unsupported", 0, 10)
            .await
            .expect_err("load_provenance_window should be unsupported");
        assert!(err.to_string().contains("not yet supported"));
    }
}
