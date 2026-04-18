//! AI governance L2 — agent registry, confidence, fallback, drift,
//! lifecycle transitions, tool-invocation checks.
//!
//! Confidence and drift are computed from the provenance chain where
//! possible; where the adapter surface isn't wired yet (drift detector,
//! confidence aggregation over live sessions), the service returns a
//! well-shaped stub so clients see the same envelope regardless of
//! backing implementation.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::storage::{AgentRow, StorageHandle};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgentRequest {
    pub workflow_url: String,
    pub name: String,
    /// `deterministic` | `statistical` | `generative`.
    pub kind: String,
    pub version: String,
    #[serde(default)]
    pub autonomy: Option<String>,
    #[serde(default)]
    pub confidence_floor: Option<f64>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub workflow_url: String,
    pub name: String,
    pub kind: String,
    pub version: String,
    pub status: String,
    pub autonomy: Option<String>,
    pub confidence_floor: Option<f64>,
    pub deployment_state: String,
    pub config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AgentRow> for AgentView {
    fn from(r: AgentRow) -> Self {
        Self {
            id: r.id,
            workflow_url: r.workflow_url,
            name: r.name,
            kind: r.kind,
            version: r.version,
            status: r.status,
            autonomy: r.autonomy,
            confidence_floor: r.confidence_floor,
            deployment_state: r.deployment_state,
            config: r.config_json,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleTransitionRequest {
    /// `active` | `degraded` | `suspended` | `retired`.
    pub target_state: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftReport {
    pub agent_id: String,
    pub window_days: u32,
    pub psi: Option<f64>,
    pub ks: Option<f64>,
    pub alert: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvocationCheck {
    pub allowed: bool,
    pub reason: String,
}

pub struct AgentService;

impl AgentService {
    pub async fn register(
        storage: &StorageHandle,
        req: RegisterAgentRequest,
    ) -> ApiResult<AgentView> {
        let now = Utc::now();
        let row = AgentRow {
            id: format!("urn:wos:agent:{}", Uuid::new_v4()),
            workflow_url: req.workflow_url,
            name: req.name,
            kind: req.kind,
            version: req.version,
            status: "active".into(),
            autonomy: req.autonomy,
            confidence_floor: req.confidence_floor,
            config_json: req.config,
            deployment_state: "production".into(),
            created_at: now,
            updated_at: now,
        };
        storage.upsert_agent(&row).await?;
        Ok(row.into())
    }

    pub async fn get(storage: &StorageHandle, id: &str) -> ApiResult<AgentView> {
        storage
            .get_agent(id)
            .await?
            .map(Into::into)
            .ok_or(ApiError::NotFound)
    }

    pub async fn list(
        storage: &StorageHandle,
        workflow_url: &str,
    ) -> ApiResult<Vec<AgentView>> {
        Ok(storage
            .list_agents(workflow_url)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn transition_lifecycle(
        storage: &StorageHandle,
        id: &str,
        req: LifecycleTransitionRequest,
    ) -> ApiResult<AgentView> {
        let mut agent = storage
            .get_agent(id)
            .await?
            .ok_or(ApiError::NotFound)?;
        validate_lifecycle_target(&req.target_state)?;
        agent.status = req.target_state;
        agent.updated_at = Utc::now();
        if let Some(reason) = req.reason {
            if let Some(obj) = agent.config_json.as_object_mut() {
                obj.insert(
                    "lastTransitionReason".into(),
                    serde_json::Value::String(reason),
                );
            }
        }
        storage.upsert_agent(&agent).await?;
        Ok(agent.into())
    }

    pub async fn set_deployment(
        storage: &StorageHandle,
        id: &str,
        target: &str,
    ) -> ApiResult<AgentView> {
        let mut agent = storage
            .get_agent(id)
            .await?
            .ok_or(ApiError::NotFound)?;
        validate_deployment_state(target)?;
        agent.deployment_state = target.to_string();
        agent.updated_at = Utc::now();
        storage.upsert_agent(&agent).await?;
        Ok(agent.into())
    }

    /// `GET /api/agents/:id/drift` — stub drift report.
    pub async fn drift_report(storage: &StorageHandle, id: &str) -> ApiResult<DriftReport> {
        let agent = storage
            .get_agent(id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Ok(DriftReport {
            agent_id: agent.id,
            window_days: 30,
            psi: None,
            ks: None,
            alert: false,
            note: "drift detector not configured; configure via \
                   WOS_DRIFT_DETECTOR in ServerConfig for real readings"
                .into(),
        })
    }

    /// `POST /api/agents/:id/tool-invocation-check` — stub authorization.
    pub async fn tool_invocation_check(
        storage: &StorageHandle,
        id: &str,
    ) -> ApiResult<ToolInvocationCheck> {
        let agent = storage
            .get_agent(id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let allowed = agent.status == "active" && agent.deployment_state == "production";
        Ok(ToolInvocationCheck {
            allowed,
            reason: if allowed {
                "agent active + in production deployment state".into()
            } else {
                format!(
                    "agent blocked: status={}, deploymentState={}",
                    agent.status, agent.deployment_state
                )
            },
        })
    }
}

fn validate_lifecycle_target(s: &str) -> ApiResult<()> {
    match s {
        "active" | "degraded" | "suspended" | "retired" => Ok(()),
        _ => Err(ApiError::BadRequest(format!(
            "invalid lifecycle target `{s}` — expected active|degraded|suspended|retired"
        ))),
    }
}

fn validate_deployment_state(s: &str) -> ApiResult<()> {
    match s {
        "production" | "canary" | "shadow" => Ok(()),
        _ => Err(ApiError::BadRequest(format!(
            "invalid deploymentState `{s}` — expected production|canary|shadow"
        ))),
    }
}
