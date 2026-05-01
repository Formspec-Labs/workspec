//! Integration Profile — inbound CloudEvents + integration-profile reads.
//!
//! Inbound CloudEvent handling is idempotent: the `id` field is used as
//! a dedupe key against `integration_inbound` via `INSERT OR IGNORE`, so
//! concurrent duplicates return a clean `deduplicated: true` ack instead of
//! a primary-key error. The envelope is persisted and the `data` payload is
//! enqueued as an event on the target instance (if `instanceId` is present
//! in the envelope).
//!
//! Tool / Arazzo / policy-engine binding invocation currently echoes the
//! binding + inputs — enough for consumers to assert shape conformance
//! while real dispatch through `wos_runtime::integration_handlers`
//! is wired up.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wos_runtime::cloudevents::CloudEvent;

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::storage::InboundCloudEventRow;

/// CloudEvents 1.0 envelope plus the WOS-specific `instanceId` extension
/// used to route inbound events to a target case instance.
///
/// The envelope fields are reused from `wos_runtime::cloudevents::CloudEvent`
/// via `#[serde(flatten)]` so ingress validation (`validate_ingress`) and
/// the canonical wire shape stay in one place.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WosInboundEvent {
    #[serde(flatten)]
    pub envelope: CloudEvent,
    /// WOS extension: which instance should receive the event.
    #[serde(
        default,
        rename = "instanceId",
        skip_serializing_if = "Option::is_none"
    )]
    pub instance_id: Option<String>,
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
    pub async fn accept_inbound(
        state: &AppState,
        inbound: WosInboundEvent,
    ) -> ApiResult<InboundAck> {
        let WosInboundEvent {
            envelope,
            instance_id,
        } = inbound;
        envelope
            .validate_ingress()
            .map_err(|e| ApiError::BadRequest(format!("invalid CloudEvent: {e}")))?;

        let row = InboundCloudEventRow {
            cloud_event_id: envelope.id.clone(),
            instance_id: instance_id.clone().unwrap_or_default(),
            binding: envelope.event_type.clone(),
            received_at: Utc::now(),
            payload_json: envelope.to_provenance_data(),
        };
        let inserted = state.storage.insert_inbound_cloud_event(&row).await?;
        if !inserted {
            return Ok(InboundAck {
                cloud_event_id: envelope.id,
                deduplicated: true,
                enqueued: false,
                reason: Some("already received".into()),
            });
        }

        match &instance_id {
            Some(id) if !id.is_empty() => {
                let event_envelope = serde_json::json!({
                    "event": envelope.event_type,
                    "actor": "system:cloudevents",
                    "data": envelope.data.clone().unwrap_or(serde_json::Value::Null),
                });
                state.runtime.enqueue_event(id, event_envelope).await?;
                Ok(InboundAck {
                    cloud_event_id: envelope.id,
                    deduplicated: false,
                    enqueued: true,
                    reason: None,
                })
            }
            _ => Ok(InboundAck {
                cloud_event_id: envelope.id,
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
        bundle.integration_profile.ok_or_else(|| ApiError::NotFound)
    }

    pub async fn invoke_binding(
        bundle: &BundleService,
        workflow_url: &str,
        binding: &str,
        inputs: &serde_json::Value,
    ) -> ApiResult<serde_json::Value> {
        let _profile = Self::integration_profile(bundle, workflow_url).await?;
        let correlation_token = Uuid::now_v7().to_string();
        if binding.eq_ignore_ascii_case("http")
            && let (Some(url), Some(method)) = (
                inputs.get("url").and_then(|v| v.as_str()),
                inputs.get("method").and_then(|v| v.as_str()),
            )
        {
            let client = reqwest::Client::new();
            let method = method
                .parse::<reqwest::Method>()
                .map_err(|e| ApiError::BadRequest(format!("invalid HTTP method: {e}")))?;
            let mut request = client.request(method, url);
            if let Some(body) = inputs.get("body") {
                request = request.json(body);
            }
            let response = request
                .send()
                .await
                .map_err(|e| ApiError::ServiceUnavailable(format!("HTTP dispatch failed: {e}")))?;
            let status = response.status().as_u16();
            let output = response
                .json::<serde_json::Value>()
                .await
                .unwrap_or_else(|_| serde_json::json!({}));
            return Ok(serde_json::json!({
                "binding": binding,
                "inputs": inputs,
                "correlationToken": correlation_token,
                "output": {
                    "status": "dispatched",
                    "httpStatus": status,
                    "payload": output,
                }
            }));
        }

        // Fallback: keep schema-compatible output for non-HTTP bindings until
        // deeper runtime integration adapter wiring lands.
        Ok(serde_json::json!({
            "binding": binding,
            "inputs": inputs,
            "correlationToken": correlation_token,
            "output": {
                "status": "accepted",
                "note": "binding accepted; concrete adapter dispatch pending",
            }
        }))
    }
}
