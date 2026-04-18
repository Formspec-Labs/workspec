use std::sync::Arc;

use crate::domain::{
    AiDisclosureView, ApplicantDeterminationView, CounterfactualsView, MilestoneView,
};
use crate::error::{ApiError, ApiResult};
use crate::storage::StorageHandle;

use super::bundle_service::BundleService;
use super::instance_service::InstanceService;
use super::provenance_service::ProvenanceService;

pub struct ApplicantService {
    storage: StorageHandle,
    bundle: Arc<BundleService>,
    provenance: Arc<ProvenanceService>,
}

impl ApplicantService {
    pub fn new(
        storage: StorageHandle,
        bundle: Arc<BundleService>,
        provenance: Arc<ProvenanceService>,
    ) -> Self {
        Self {
            storage,
            bundle,
            provenance,
        }
    }

    pub async fn determination(
        &self,
        instance_id: &str,
    ) -> ApiResult<Option<ApplicantDeterminationView>> {
        let Some(row) = self.storage.get_instance(instance_id).await? else {
            return Ok(None);
        };
        let program_name = self
            .bundle
            .get(&row.definition_url)
            .await
            .map(|k| k.title)
            .unwrap_or_else(|| row.definition_url.clone());

        let instance = InstanceService::parse(&row)?;
        let provenance = self.provenance.list(instance_id).await?;

        let case_state = instance.case_state.clone();
        let decision = case_state
            .get("decision")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();
        let summary = case_state
            .get("determinationSummary")
            .and_then(|v| v.as_str())
            .unwrap_or("Determination in progress.")
            .to_string();

        let evidence_considered: Vec<String> = case_state
            .get("evidence")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Provenance-derived rules/milestones pull from the spec-shaped
        // `wos_core::ProvenanceRecord` embedded in each response.
        let rules_applied: Vec<String> = provenance
            .iter()
            .flat_map(|p| {
                p.record
                    .data
                    .as_ref()
                    .and_then(|d| d.get("rulesApplied"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        let milestones = provenance
            .iter()
            .enumerate()
            .map(|(i, p)| MilestoneView {
                id: p.id.clone(),
                label: p.record.event.clone().unwrap_or_default(),
                status: if i + 1 == provenance.len() {
                    "current".into()
                } else {
                    "completed".into()
                },
                description: String::new(),
                date: Some(p.record.timestamp.clone()).filter(|s| !s.is_empty()),
            })
            .collect();

        Ok(Some(ApplicantDeterminationView {
            instance_id: instance_id.into(),
            program_name,
            decision,
            date_issued: row.updated_at.to_rfc3339(),
            deadline_date: instance
                .timers
                .first()
                .map(|t| t.deadline.clone())
                .unwrap_or_default(),
            benefits_continue: case_state
                .get("benefitsContinue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            summary,
            evidence_considered,
            rules_applied,
            ai_disclosure: AiDisclosureView {
                // `actor_type == "agent"` is the spec's marker for AI-mediated
                // actions (`wos_core::provenance::ProvenanceRecord.actor_type`).
                was_used: provenance
                    .iter()
                    .any(|p| p.record.actor_type.as_deref() == Some("agent")),
                description: None,
                human_reviewer: None,
            },
            counterfactuals: CounterfactualsView {
                positive: Vec::new(),
                negative: Vec::new(),
            },
            appeal_status: case_state
                .get("appealStatus")
                .and_then(|v| v.as_str())
                .unwrap_or("not-filed")
                .to_string(),
            milestones,
        }))
    }

    /// Submit an appeal. Enqueues an `appealFiled` event on the instance and
    /// drains it through the runtime so the kernel-defined appeal workflow
    /// (if present) gets a chance to react. If the kernel doesn't define
    /// `appealFiled`, the runtime records an `UnmatchedEvent` in provenance —
    /// still a durable, tamper-evident audit record.
    pub async fn submit_appeal(
        &self,
        runtime: &crate::runtime::AppRuntime,
        instance_id: &str,
        reason: &str,
    ) -> ApiResult<()> {
        let envelope = serde_json::json!({
            "event": "appealFiled",
            "actor": "applicant",
            "data": { "reason": reason },
        });
        runtime
            .enqueue_event(instance_id, envelope)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        runtime
            .drain_once(instance_id)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        // Mirror the appeal status into case_state so the applicant view
        // renders even without a dedicated appeal workflow in the kernel.
        let reason_s = reason.to_string();
        self.storage
            .update_instance_atomic(
                instance_id,
                &move |row| {
                    let inst = row.instance_json.as_object_mut().ok_or_else(|| {
                        crate::storage::StorageError::Other(
                            "instance_json is not an object".into(),
                        )
                    })?;
                    let case_state = inst
                        .entry("caseState".to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    let obj = case_state.as_object_mut().ok_or_else(|| {
                        crate::storage::StorageError::Other(
                            "caseState is not an object".into(),
                        )
                    })?;
                    obj.insert(
                        "appealStatus".into(),
                        serde_json::Value::String("filed".into()),
                    );
                    obj.insert(
                        "appealReason".into(),
                        serde_json::Value::String(reason_s.clone()),
                    );
                    Ok(Vec::new())
                },
            )
            .await
            .map_err(ApiError::Storage)?;
        Ok(())
    }
}
