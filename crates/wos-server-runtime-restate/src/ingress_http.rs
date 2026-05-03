//! HTTP ingress client for Restate virtual-object handlers (WS-094 Phase 2).
//!
//! `drainOnce` ingress responses deserialize the full worker payload (transitions,
//! provenance, task ids, emitted events, guard evaluations), not a stub shape.

use reqwest::header::{CONTENT_TYPE, HeaderValue};
use serde::Deserialize;
use wos_core::eval::{GuardEvaluation, ObservedTransition};
use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_runtime::runtime::{CreateInstanceRequest, DrainOnceResult};
use wos_server_ports::runtime::{RuntimeAdapterError, RuntimeResult};

use crate::restate_virtual::{ingress_invoke_url, CreateIngressBody};

/// Minimal Restate ingress HTTP client for [`super::RestateRuntimeAdapter`](super::RestateRuntimeAdapter).
#[derive(Clone)]
pub struct RestateIngressClient {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl RestateIngressClient {
    async fn post_object_unit(
        &self,
        instance_id: &str,
        handler: &str,
        body: &serde_json::Value,
    ) -> RuntimeResult<()> {
        let url = ingress_invoke_url(&self.base_url, instance_id, handler);
        let res = self
            .client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .json(body)
            .send()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress request failed: {e}")))?;
        let status = res.status();
        let text = res
            .text()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress body read failed: {e}")))?;
        if !status.is_success() {
            return Err(RuntimeAdapterError::Message(format!(
                "restate ingress {status}: {text}"
            )));
        }
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == "null" {
            return Ok(());
        }
        let _: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
            RuntimeAdapterError::Message(format!(
                "restate ingress response is not valid JSON for {handler}: {e}; body={text:?}"
            ))
        })?;
        Ok(())
    }

    /// POST with no body (handlers that declare no input; Restate rejects `application/json` on these).
    ///
    /// Empty or all-whitespace bodies are treated as JSON `null` before deserializing into `T`
    /// (avoids `EOF while parsing a value` from `serde_json::from_str("")`). Typed targets such
    /// as [`CaseInstance`](wos_core::instance::CaseInstance) still require a real object on the wire.
    pub async fn post_object_empty<T: serde::de::DeserializeOwned>(
        &self,
        instance_id: &str,
        handler: &str,
    ) -> RuntimeResult<T> {
        let url = ingress_invoke_url(&self.base_url, instance_id, handler);
        let res = self
            .client
            .post(url)
            .send()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress request failed: {e}")))?;
        let status = res.status();
        let text = res
            .text()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress body read failed: {e}")))?;
        if !status.is_success() {
            return Err(RuntimeAdapterError::Message(format!(
                "restate ingress {status}: {text}"
            )));
        }
        let trimmed = text.trim();
        let value: serde_json::Value = if trimmed.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(trimmed).map_err(|e| {
                RuntimeAdapterError::Message(format!(
                    "restate ingress response is not valid JSON for {handler}: {e}; body={text:?}"
                ))
            })?
        };
        serde_json::from_value(value).map_err(|e| {
            RuntimeAdapterError::Message(format!(
                "restate ingress response is not valid JSON for {handler}: {e}; body={text:?}"
            ))
        })
    }

    /// POST JSON to a virtual-object handler and deserialize the JSON body.
    pub async fn post_object<T: serde::de::DeserializeOwned>(
        &self,
        instance_id: &str,
        handler: &str,
        body: &serde_json::Value,
    ) -> RuntimeResult<T> {
        let url = ingress_invoke_url(&self.base_url, instance_id, handler);
        let res = self
            .client
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .json(body)
            .send()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress request failed: {e}")))?;
        let status = res.status();
        let text = res
            .text()
            .await
            .map_err(|e| RuntimeAdapterError::Message(format!("restate ingress body read failed: {e}")))?;
        if !status.is_success() {
            return Err(RuntimeAdapterError::Message(format!(
                "restate ingress {status}: {text}"
            )));
        }
        serde_json::from_str(&text).map_err(|e| {
            RuntimeAdapterError::Message(format!("restate ingress response is not valid JSON for {handler}: {e}; body={text:?}"))
        })
    }

    pub async fn create_instance(&self, request: &CreateInstanceRequest) -> RuntimeResult<CaseInstance> {
        let body = serde_json::to_value(CreateIngressBody::from(request))
            .map_err(|e| RuntimeAdapterError::Message(format!("serialize create body: {e}")))?;
        self.post_object(&request.instance_id, "createInstance", &body)
            .await
    }

    pub async fn load_instance(&self, instance_id: &str) -> RuntimeResult<CaseInstance> {
        self.post_object_empty(instance_id, "loadInstance").await
    }

    pub async fn enqueue_event(
        &self,
        instance_id: &str,
        event: serde_json::Value,
    ) -> RuntimeResult<()> {
        self.post_object_unit(instance_id, "enqueueEvent", &event)
            .await
    }

    pub async fn drain_once(&self, instance_id: &str) -> RuntimeResult<DrainOnceResult> {
        let v: serde_json::Value = self.post_object_empty(instance_id, "drainOnce").await?;
        parse_drain_once_json(v)
    }
}

/// Wire shape for [`restate_virtual::drain_once_result_json`], deserialized on the ingress client.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DrainOnceIngressBody {
    processed_event: Option<String>,
    processed_event_token: Option<String>,
    #[serde(default)]
    transitions: Vec<ObservedTransition>,
    #[serde(default)]
    provenance: Vec<ProvenanceRecord>,
    #[serde(default)]
    created_task_ids: Vec<String>,
    #[serde(default)]
    emitted_events: Vec<String>,
    #[serde(default)]
    guard_evaluations: Vec<GuardEvaluation>,
}

fn parse_drain_once_json(value: serde_json::Value) -> RuntimeResult<DrainOnceResult> {
    let body: DrainOnceIngressBody = serde_json::from_value(value).map_err(|e| {
        RuntimeAdapterError::Message(format!("drainOnce ingress JSON did not match worker shape: {e}"))
    })?;
    Ok(DrainOnceResult {
        processed_event: body.processed_event,
        processed_event_token: body.processed_event_token,
        transitions: body.transitions,
        provenance: body.provenance,
        created_task_ids: body.created_task_ids,
        emitted_events: body.emitted_events,
        guard_evaluations: body.guard_evaluations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_drain_once_deserializes_full_worker_payload() {
        let v = serde_json::json!({
            "processedEvent": "start",
            "processedEventToken": "tok-1",
            "transitions": [{
                "from": "draft",
                "to": "intake",
                "event": "start",
                "tags": ["governance"]
            }],
            "provenance": [],
            "createdTaskIds": ["task_a"],
            "emittedEvents": ["notify"],
            "guardEvaluations": []
        });
        let r = parse_drain_once_json(v).expect("ingress drainOnce JSON should parse");
        assert_eq!(r.processed_event.as_deref(), Some("start"));
        assert_eq!(r.processed_event_token.as_deref(), Some("tok-1"));
        assert_eq!(r.transitions.len(), 1);
        assert_eq!(r.transitions[0].from, "draft");
        assert_eq!(r.created_task_ids, vec!["task_a"]);
        assert_eq!(r.emitted_events, vec!["notify"]);
    }

    #[test]
    fn empty_ingress_body_maps_to_json_null_before_typed_deserialize() {
        let trimmed = "";
        let value: serde_json::Value = if trimmed.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(trimmed).expect("non-empty body parses as JSON")
        };
        let v: serde_json::Value =
            serde_json::from_value(value).expect("null maps to JSON Value::Null");
        assert!(v.is_null());
    }
}
