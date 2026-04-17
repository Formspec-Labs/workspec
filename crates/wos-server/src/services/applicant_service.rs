use std::sync::Arc;

use crate::domain::{
    AiDisclosureView, ApplicantDeterminationView, CounterfactualsView, MilestoneView,
};
use crate::error::{ApiError, ApiResult};
use crate::storage::StorageHandle;

use super::bundle_service::BundleService;
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

        let provenance = self.provenance.list(instance_id).await?;
        let decision = row
            .case_state
            .get("decision")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();
        let summary = row
            .case_state
            .get("determinationSummary")
            .and_then(|v| v.as_str())
            .unwrap_or("Determination in progress.")
            .to_string();

        let rules_applied: Vec<String> = provenance
            .iter()
            .flat_map(|p| p.reasoning.iter().flat_map(|r| r.rules_applied.iter().cloned()))
            .collect();
        let evidence_considered: Vec<String> = row
            .case_state
            .get("evidence")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let milestones = provenance
            .iter()
            .enumerate()
            .map(|(i, p)| MilestoneView {
                id: p.id.clone(),
                label: p.event.clone(),
                status: if i + 1 == provenance.len() {
                    "current".into()
                } else {
                    "completed".into()
                },
                description: p
                    .reasoning
                    .as_ref()
                    .and_then(|r| r.explanation.clone())
                    .unwrap_or_default(),
                date: Some(p.timestamp.clone()),
            })
            .collect();

        Ok(Some(ApplicantDeterminationView {
            instance_id: instance_id.into(),
            program_name,
            decision,
            date_issued: row.updated_at.to_rfc3339(),
            deadline_date: row
                .timers
                .as_array()
                .and_then(|a| a.first())
                .and_then(|t| t.get("deadline"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            benefits_continue: row
                .case_state
                .get("benefitsContinue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            summary,
            evidence_considered,
            rules_applied,
            ai_disclosure: AiDisclosureView {
                was_used: provenance.iter().any(|p| p.ai_narrative.is_some()),
                description: None,
                human_reviewer: None,
            },
            counterfactuals: CounterfactualsView {
                positive: Vec::new(),
                negative: Vec::new(),
            },
            appeal_status: row
                .case_state
                .get("appealStatus")
                .and_then(|v| v.as_str())
                .unwrap_or("not-filed")
                .to_string(),
            milestones,
        }))
    }

    pub async fn submit_appeal(&self, instance_id: &str, reason: &str) -> ApiResult<()> {
        // Pre-compute the next provenance row (read-before-txn: safe here
        // because the SQLite writer is single-threaded in WAL mode).
        let payload = serde_json::json!({
            "event": "appealFiled",
            "actor": { "id": "applicant", "type": "human", "name": "Applicant" },
            "facts": { "inputs": { "reason": reason }, "outputs": {}, "metadata": {} },
        });
        let prov_row = self.provenance.prepare_next(instance_id, "facts", payload).await?;
        let reason_s = reason.to_string();

        self.storage
            .update_instance_atomic(
                instance_id,
                &move |row| {
                    let obj = row
                        .case_state
                        .as_object_mut()
                        .ok_or_else(|| crate::storage::StorageError::Other(
                            "case_state is not an object".into(),
                        ))?;
                    obj.insert(
                        "appealStatus".into(),
                        serde_json::Value::String("filed".into()),
                    );
                    obj.insert(
                        "appealReason".into(),
                        serde_json::Value::String(reason_s.clone()),
                    );
                    Ok(vec![prov_row.clone()])
                },
            )
            .await
            .map_err(ApiError::Storage)?;
        Ok(())
    }
}
