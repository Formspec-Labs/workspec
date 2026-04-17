//! Event evaluation — the bridge between the stored instance blob and
//! `wos_core::Evaluator`.
//!
//! Two public entry points:
//! * [`EvalService::submit_event`] — applies an event atomically, persisting
//!   the new `CaseInstance` snapshot and appending **every**
//!   `wos_core::ProvenanceRecord` the evaluator emitted during the step,
//!   hash-chained.
//! * [`EvalService::available_transitions`] — dry-run enumeration of the
//!   transitions whose source state is active, useful for UX.

use std::sync::Arc;

use wos_core::instance::CaseInstance;
use wos_core::provenance::ProvenanceRecord;
use wos_core::{Evaluator, KernelDocument};

use crate::domain::{AvailableTransitionView, EvaluationResultView, SubmitEventRequest};
use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::services::provenance_service::{ProvenanceService, row_to_response};

use crate::storage::StorageHandle;

pub struct EvalService {
    storage: StorageHandle,
    bundle: Arc<BundleService>,
    provenance: Arc<ProvenanceService>,
}

impl EvalService {
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

    pub async fn submit_event(
        &self,
        instance_id: &str,
        req: &SubmitEventRequest,
    ) -> ApiResult<EvaluationResultView> {
        let row = self
            .storage
            .get_instance(instance_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let kernel_row = self
            .bundle
            .get(&row.definition_url)
            .await
            .ok_or_else(|| ApiError::ServiceUnavailable(format!(
                "kernel `{}` not loaded in registry",
                row.definition_url,
            )))?;

        let kernel: KernelDocument = serde_json::from_value(kernel_row.document.clone())
            .map_err(|e| ApiError::ServiceUnavailable(format!("kernel parse failed: {e}")))?;
        let instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(format!(
                "instance rehydration failed: {e}",
            )))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let mut evaluator = Evaluator::from_instance(kernel, &instance, now_ms)
            .map_err(|e| ApiError::ServiceUnavailable(format!("evaluator init failed: {e}")))?;

        let previous_configuration = evaluator.configuration().active_states().to_vec();
        let prov_cursor_before = evaluator.provenance().records().len();

        let _fired = evaluator
            .process_event(&req.event, Some(&req.actor_id), req.data.as_ref())
            .map_err(|e| ApiError::BadRequest(format!("evaluator rejected event: {e}")))?;

        let new_configuration = evaluator.configuration().active_states().to_vec();
        let events_fired = evaluator
            .transitions()
            .iter()
            .map(|t| t.event.clone())
            .collect::<Vec<_>>();

        let new_instance_json = build_instance_snapshot(&evaluator, &instance, instance_id);
        let case_state_before = instance.case_state.clone();
        let case_state_after = new_instance_json
            .get("caseState")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let mutations = diff_case_state(&case_state_before, &case_state_after);

        // Take the delta: every record emitted during this event.
        let mut new_records: Vec<ProvenanceRecord> = evaluator
            .provenance()
            .records()
            .iter()
            .skip(prov_cursor_before)
            .cloned()
            .collect();

        // If the evaluator emitted nothing (unmatched event, no side-effects),
        // still record the submission attempt so the chain reflects it.
        if new_records.is_empty() {
            let mut fallback = ProvenanceRecord::state_transition(
                previous_configuration.first().map(String::as_str).unwrap_or(""),
                new_configuration.first().map(String::as_str).unwrap_or(""),
                &req.event,
                Some(&req.actor_id),
            );
            fallback.audit_layer = Some("facts".into());
            fallback.data = req.data.clone();
            new_records.push(fallback);
        }

        let prov_rows = self.provenance.prepare_batch(instance_id, &new_records).await?;
        let head_response = prov_rows.first().map(row_to_response).transpose()?;

        let status = status_from_snapshot(&new_instance_json);
        let impact_level = row.impact_level.clone();

        let new_instance_json_mut = std::sync::Arc::new(new_instance_json);
        let prov_rows_mut = std::sync::Arc::new(prov_rows);
        let status_cloned = status.clone();
        let impact_level_cloned = impact_level.clone();

        self.storage
            .update_instance_atomic(instance_id, &move |current| {
                current.instance_json = (*new_instance_json_mut).clone();
                current.status = status_cloned.clone();
                current.impact_level = impact_level_cloned.clone();
                Ok((*prov_rows_mut).clone())
            })
            .await?;

        Ok(EvaluationResultView {
            previous_configuration,
            new_configuration,
            events_fired,
            head_record: head_response,
            case_state_mutations: mutations,
        })
    }

    pub async fn available_transitions(
        &self,
        instance_id: &str,
    ) -> ApiResult<Vec<AvailableTransitionView>> {
        let row = self
            .storage
            .get_instance(instance_id)
            .await?
            .ok_or(ApiError::NotFound)?;
        let kernel_row = self
            .bundle
            .get(&row.definition_url)
            .await
            .ok_or_else(|| ApiError::ServiceUnavailable(format!(
                "kernel `{}` not loaded",
                row.definition_url
            )))?;
        let kernel: KernelDocument = match serde_json::from_value(kernel_row.document.clone()) {
            Ok(k) => k,
            Err(_) => return Ok(Vec::new()),
        };
        let instance: CaseInstance = match serde_json::from_value(row.instance_json.clone()) {
            Ok(i) => i,
            Err(_) => return Ok(Vec::new()),
        };

        let active: std::collections::HashSet<String> = instance.configuration.into_iter().collect();
        let mut out = Vec::new();
        walk_states(&kernel.lifecycle.states, &active, &mut out);
        Ok(out)
    }
}

fn walk_states(
    states: &indexmap::IndexMap<String, wos_core::State>,
    active: &std::collections::HashSet<String>,
    out: &mut Vec<AvailableTransitionView>,
) {
    for (id, state) in states {
        if active.contains(id) {
            for t in &state.transitions {
                out.push(AvailableTransitionView {
                    event: t.event.clone(),
                    target: t.target.clone(),
                    guard: t.guard.clone(),
                    guard_satisfied: t.guard.is_none(),
                    tags: state.tags.clone(),
                    description: t.description.clone(),
                });
            }
        }
        walk_states(&state.states, active, out);
        for region in state.regions.values() {
            walk_states(&region.states, active, out);
        }
    }
}

fn build_instance_snapshot(
    evaluator: &Evaluator,
    prior: &CaseInstance,
    instance_id: &str,
) -> serde_json::Value {
    let configuration = evaluator.configuration().active_states().to_vec();
    let case_state = evaluator.case_state_json();
    let history_store = evaluator.history_store().clone();

    let now = chrono::Utc::now().to_rfc3339();
    let snapshot = CaseInstance {
        instance_id: instance_id.to_string(),
        definition_url: prior.definition_url.clone(),
        definition_version: prior.definition_version.clone(),
        configuration,
        case_state,
        provenance_position: prior.provenance_position + 1,
        next_task_sequence: prior.next_task_sequence,
        timers: prior.timers.clone(),
        active_tasks: prior.active_tasks.clone(),
        history_store,
        compensation_logs: prior.compensation_logs.clone(),
        status: prior.status,
        pending_events: Vec::new(),
        governance_state: prior.governance_state.clone(),
        volume_counters: prior.volume_counters.clone(),
        created_at: prior.created_at.clone(),
        updated_at: now,
        fired_milestones: prior.fired_milestones.clone(),
        pending_callbacks: prior.pending_callbacks.clone(),
        extensions: prior.extensions.clone(),
    };
    serde_json::to_value(&snapshot).unwrap_or(serde_json::json!({}))
}

fn status_from_snapshot(v: &serde_json::Value) -> String {
    v.get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("active")
        .to_string()
}

fn diff_case_state(before: &serde_json::Value, after: &serde_json::Value) -> serde_json::Value {
    let b = before.as_object();
    let a = after.as_object();
    let mut out = serde_json::Map::new();
    if let (Some(b), Some(a)) = (b, a) {
        for (k, v) in a {
            if b.get(k) != Some(v) {
                out.insert(k.clone(), v.clone());
            }
        }
        for k in b.keys() {
            if !a.contains_key(k) {
                out.insert(k.clone(), serde_json::Value::Null);
            }
        }
    } else {
        out.insert("_replaced".into(), after.clone());
    }
    serde_json::Value::Object(out)
}
