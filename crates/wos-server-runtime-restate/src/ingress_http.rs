//! HTTP ingress client for Restate virtual-object handlers (WS-094 Phase 2).

use reqwest::header::{CONTENT_TYPE, HeaderValue};
use wos_core::instance::CaseInstance;
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
        let empty = serde_json::json!({});
        self.post_object(instance_id, "loadInstance", &empty).await
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
        let empty = serde_json::json!({});
        let v: serde_json::Value = self
            .post_object(instance_id, "drainOnce", &empty)
            .await?;
        Ok(parse_drain_once_json(v))
    }
}

fn parse_drain_once_json(value: serde_json::Value) -> DrainOnceResult {
    let processed_event = value
        .get("processedEvent")
        .and_then(|v| v.as_str())
        .map(String::from);
    let processed_event_token = value
        .get("processedEventToken")
        .and_then(|v| v.as_str())
        .map(String::from);
    DrainOnceResult {
        processed_event,
        processed_event_token,
        transitions: vec![],
        provenance: vec![],
        created_task_ids: vec![],
        emitted_events: vec![],
        guard_evaluations: vec![],
    }
}
