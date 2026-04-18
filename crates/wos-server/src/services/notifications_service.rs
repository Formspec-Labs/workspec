//! Notification template rendering (sidecars/notification-template).
//!
//! Renders the `body` of a notification template with `${variable}`
//! placeholders substituted from a supplied context map.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::services::json_util::lookup_dotted;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub template_id: String,
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResponse {
    pub template_id: String,
    pub subject: Option<String>,
    pub body: String,
    pub channels: Vec<String>,
    pub rendered_variables: Vec<String>,
}

pub struct NotificationsService {
    bundle: Arc<BundleService>,
}

impl NotificationsService {
    pub fn new(bundle: Arc<BundleService>) -> Self {
        Self { bundle }
    }

    pub async fn render(
        &self,
        workflow_url: &str,
        req: &RenderRequest,
    ) -> ApiResult<RenderResponse> {
        let bundle = self
            .bundle
            .full_bundle(workflow_url)
            .await
            .ok_or(ApiError::NotFound)?;
        let templates = bundle
            .notification_templates
            .ok_or_else(|| ApiError::BadRequest(
                "no notification-template sidecar attached to this workflow".into(),
            ))?;
        let tmpl = find_template(&templates, &req.template_id)
            .ok_or(ApiError::NotFound)?;
        let subject = tmpl
            .get("subject")
            .and_then(|v| v.as_str())
            .map(|s| interpolate(s, &req.context));
        let body_template = tmpl
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::BadRequest(
                "template has no `body` field".into(),
            ))?;
        let body = interpolate(body_template, &req.context);
        let channels = tmpl
            .get("channels")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let rendered_variables = extract_variables(body_template);
        Ok(RenderResponse {
            template_id: req.template_id.clone(),
            subject,
            body,
            channels,
            rendered_variables,
        })
    }
}

fn find_template<'a>(sidecar: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    sidecar
        .get("templates")
        .and_then(|t| t.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id))
        })
}

fn interpolate(template: &str, context: &serde_json::Value) -> String {
    // Minimal `${var}` replacement using a JSONPath-lite lookup (dots only).
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        if let Some(end) = tail.find('}') {
            let key = &tail[..end];
            let value = lookup_dotted(context, key).unwrap_or_else(|| format!("${{{key}}}"));
            out.push_str(&value);
            rest = &tail[end + 1..];
        } else {
            // Unterminated placeholder — emit verbatim.
            out.push_str(&rest[start..]);
            break;
        }
    }
    out.push_str(rest);
    out
}

fn extract_variables(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        let tail = &rest[start + 2..];
        if let Some(end) = tail.find('}') {
            out.push(tail[..end].to_string());
            rest = &tail[end + 1..];
        } else {
            break;
        }
    }
    out.sort();
    out.dedup();
    out
}
