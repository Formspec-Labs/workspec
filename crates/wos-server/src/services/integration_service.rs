//! Integration Profile — inbound CloudEvents + integration-profile reads.
//!
//! Inbound CloudEvent handling is idempotent: the `id` field is used as
//! a dedupe key against `integration_inbound`. The envelope is persisted
//! and the `data` payload is enqueued as an event on the target instance
//! (if `instanceId` is present in the envelope).
//!
//! Tool / Arazzo / policy-engine binding invocation is a Phase 9 stretch
//! goal and currently echoes the binding + inputs — enough for consumers
//! to assert shape conformance without a real integration adapter.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::runtime::AppRuntime;
use crate::services::bundle_service::BundleService;
use crate::storage::{InboundCloudEventRow, StorageHandle};

/// CloudEvents 1.0 envelope (minimal subset).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudEvent {
    pub id: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub specversion: String,
    /// WOS extension: which instance should receive the event.
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub datacontenttype: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundAck {
    pub cloud_event_id: String,
    pub deduplicated: bool,
    pub enqueued: bool,
    pub reason: Option<String>,
}

pub struct IntegrationService;

impl IntegrationService {
    pub async fn accept_inbound(state: &AppState, ev: CloudEvent) -> ApiResult<InboundAck> {
        // Dedupe by CloudEvent id.
        if state.storage.get_inbound_cloud_event(&ev.id).await?.is_some() {
            return Ok(InboundAck {
                cloud_event_id: ev.id,
                deduplicated: true,
                enqueued: false,
                reason: Some("already received".into()),
            });
        }

        // Persist the envelope for audit.
        let row = InboundCloudEventRow {
            cloud_event_id: ev.id.clone(),
            instance_id: ev.instance_id.clone().unwrap_or_default(),
            binding: ev.event_type.clone(),
            received_at: Utc::now(),
            payload_json: serde_json::to_value(&ev).unwrap_or(serde_json::Value::Null),
        };
        state.storage.insert_inbound_cloud_event(&row).await?;

        // Route to the target instance, if any, via the runtime's event queue.
        match &ev.instance_id {
            Some(id) if !id.is_empty() => {
                let envelope = serde_json::json!({
                    "event": ev.event_type,
                    "actor": "system:cloudevents",
                    "data": ev.data.clone().unwrap_or(serde_json::Value::Null),
                });
                state
                    .runtime
                    .enqueue_event(id, envelope)
                    .await
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
                Ok(InboundAck {
                    cloud_event_id: ev.id,
                    deduplicated: false,
                    enqueued: true,
                    reason: None,
                })
            }
            _ => Ok(InboundAck {
                cloud_event_id: ev.id,
                deduplicated: false,
                enqueued: false,
                reason: Some("no instanceId extension; envelope stored but not routed".into()),
            }),
        }
    }

    pub async fn integration_profile(
        bundle: &BundleService,
        workflow_url: &str,
    ) -> ApiResult<serde_json::Value> {
        let bundle = bundle
            .full_bundle(workflow_url)
            .await
            .ok_or(ApiError::NotFound)?;
        bundle
            .integration_profile
            .ok_or_else(|| ApiError::NotFound)
    }

    pub async fn invoke_binding(
        bundle: &BundleService,
        workflow_url: &str,
        binding: &str,
        inputs: &serde_json::Value,
    ) -> ApiResult<serde_json::Value> {
        let _profile = Self::integration_profile(bundle, workflow_url).await?;
        // Stub: echo the binding name + inputs with a shape compatible with
        // `IntegrationBinding` consumers. Real dispatch (Arazzo / Tool /
        // PolicyEngine) requires the wos-runtime integration handlers, which
        // live on a different call path and are the focus of a later round.
        Ok(serde_json::json!({
            "binding": binding,
            "inputs": inputs,
            "output": {
                "status": "echoed",
                "note": "integration binding invocation is stubbed pending adapter wiring",
            }
        }))
    }
}

#[allow(dead_code)]
fn _keep_runtime_import() -> Option<AppRuntime> {
    None
}

#[allow(dead_code)]
fn _keep_storage_import(_s: StorageHandle) {}
