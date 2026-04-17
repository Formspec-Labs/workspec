//! Event evaluation — the bridge between the stored `InstanceRow` and
//! `wos-core::Evaluator`.
//!
//! Two public entry points:
//! * [`EvalService::submit_event`] — applies an event atomically, persisting
//!   the new configuration + case state and appending a provenance record
//!   to the hash chain in the same txn.
//! * [`EvalService::available_transitions`] — dry-run enumeration of the
//!   transitions whose source state is active, with guard satisfaction
//!   computed against the current case state.

use std::sync::Arc;

use wos_core::instance::CaseInstance;
use wos_core::{Evaluator, KernelDocument};

use crate::domain::{
    AvailableTransitionView, EvaluationResultView, ProvenanceRecordView, SubmitEventRequest,
};
use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::services::provenance_service::ProvenanceService;
use crate::storage::{ProvenanceRow, StorageHandle};

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

    /// Apply `req` to the instance, returning the studio-shaped
    /// [`EvaluationResultView`].
    pub async fn submit_event(
        &self,
        instance_id: &str,
        req: &SubmitEventRequest,
    ) -> ApiResult<EvaluationResultView> {
        // Fetch the stored instance + owning kernel up-front.
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

        // Rehydrate the CaseInstance from stored JSON. Defaults fill any
        // fields that are absent (new row, older persist format).
        let instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(format!(
                "instance rehydration failed: {e}",
            )))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let mut evaluator = Evaluator::from_instance(kernel, &instance, now_ms)
            .map_err(|e| ApiError::ServiceUnavailable(format!("evaluator init failed: {e}")))?;

        let previous_configuration = evaluator.configuration().active_states().to_vec();
        // Snapshot the evaluator's provenance depth BEFORE the event — any
        // records produced during `process_event` land strictly after this
        // cursor. Persisting only the delta (not the entire log each time)
        // keeps our stored chain strictly append-only.
        let prov_cursor_before = evaluator.provenance().records().len();

        let fired = evaluator
            .process_event(&req.event, Some(&req.actor_id), req.data.as_ref())
            .map_err(|e| ApiError::BadRequest(format!("evaluator rejected event: {e}")))?;

        let new_configuration = evaluator.configuration().active_states().to_vec();
        let events_fired = evaluator
            .transitions()
            .iter()
            .map(|t| t.event.clone())
            .collect::<Vec<_>>();

        // Build the new CaseInstance snapshot from the evaluator's state.
        let new_instance_json = build_instance_snapshot(&evaluator, &instance, instance_id);
        let case_state_before = instance.case_state.clone();
        let case_state_after = new_instance_json
            .get("caseState")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let mutations = diff_case_state(&case_state_before, &case_state_after);

        // Map every new evaluator-emitted record to a stored payload. Records
        // are chained sha256-linked in order (see `prepare_batch`).
        let new_records: Vec<wos_core::provenance::ProvenanceRecord> = evaluator
            .provenance()
            .records()
            .iter()
            .skip(prov_cursor_before)
            .cloned()
            .collect();

        let mut payloads: Vec<(String, serde_json::Value)> = new_records
            .iter()
            .map(|r| (classify_tier(r), record_to_payload(r, &req.actor_id)))
            .collect();

        // If the evaluator emitted nothing (rare: no transition, no action),
        // still record the submission attempt so the provenance chain
        // reflects the event — matching the pre-refactor behaviour.
        if payloads.is_empty() {
            payloads.push((
                "facts".into(),
                serde_json::json!({
                    "event": req.event,
                    "sourceState": previous_configuration.first().cloned().unwrap_or_default(),
                    "targetState": new_configuration.first().cloned().unwrap_or_default(),
                    "actor": { "id": req.actor_id, "type": "human", "name": req.actor_id },
                    "facts": {
                        "inputs": req.data.clone().unwrap_or(serde_json::json!({})),
                        "outputs": case_state_after,
                        "metadata": { "fired": fired, "note": "no transition" },
                    },
                }),
            ));
        }

        let prov_rows = self.provenance.prepare_batch(instance_id, &payloads).await?;
        let head_record_for_view = prov_rows.first().cloned();

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
            provenance_record: head_record_for_view.as_ref().map(row_to_view),
            case_state_mutations: mutations,
        })
    }

    /// Enumerate the outgoing transitions from every currently-active state.
    /// Dry-run: does not mutate the stored instance.
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

        let active: std::collections::HashSet<String> = row.configuration().into_iter().collect();
        let mut out = Vec::new();
        walk_states(&kernel.lifecycle.states, &active, &mut out);
        Ok(out)
    }
}

/// Recursively walk the kernel's state tree and append `AvailableTransitionView`s
/// for every transition whose owning state is currently active.
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
                    // Guard satisfaction is best-effort here — full eval lives
                    // inside `process_event`. Flag unguarded transitions as
                    // satisfied and let the client probe guarded ones.
                    guard_satisfied: t.guard.is_none(),
                    tags: state.tags.clone(),
                    description: t.description.clone(),
                });
            }
        }
        // Recurse into compound children.
        walk_states(&state.states, active, out);
        for region in state.regions.values() {
            walk_states(&region.states, active, out);
        }
    }
}

/// Map a wos-core `ProvenanceRecord` into the loose `payload` JSON shape
/// our storage layer expects. Every server-side provenance row follows this
/// envelope, which in turn is what `ProvenanceService::row_to_view` and the
/// export CLI read back.
fn record_to_payload(
    r: &wos_core::provenance::ProvenanceRecord,
    fallback_actor: &str,
) -> serde_json::Value {
    let actor_id = r.actor_id.clone().unwrap_or_else(|| fallback_actor.to_string());
    let actor_type = r.actor_type.clone().unwrap_or_else(|| "system".into());
    serde_json::json!({
        "recordKind": serde_json::to_value(r.record_kind).unwrap_or(serde_json::Value::Null),
        "event": r.event.clone().unwrap_or_default(),
        "sourceState": r.from_state.clone().unwrap_or_default(),
        "targetState": r.to_state.clone().unwrap_or_default(),
        "actor": {
            "id": actor_id,
            "type": actor_type,
            "name": r.actor_id.clone().unwrap_or_else(|| fallback_actor.to_string()),
        },
        "facts": {
            "inputs": serde_json::json!({}),
            "outputs": r.data.clone().unwrap_or(serde_json::json!({})),
            "metadata": {
                "inputs": r.inputs,
                "outputs": r.outputs,
                "inputDigest": r.input_digest,
                "outputDigest": r.output_digest,
                "lifecycleState": r.lifecycle_state,
                "definitionVersion": r.definition_version,
            },
        },
    })
}

/// Route a wos-core record to one of the four Semantic-Profile tiers the
/// studio recognises: `facts | reasoning | ai-narrative | counterfactual`.
/// Deontic, autonomy, and confidence checks produce reasoning-tier records;
/// everything else defaults to `facts` (S6.5 / the ProvenanceRecord
/// `audit_layer` column in wos-core).
fn classify_tier(r: &wos_core::provenance::ProvenanceRecord) -> String {
    if let Some(tier) = r.audit_layer.as_deref() {
        return tier.to_string();
    }
    use wos_core::provenance::ProvenanceKind as K;
    match r.record_kind {
        K::DeonticViolation
        | K::DeonticEvaluation
        | K::DeonticResolution
        | K::DeonticBypass
        | K::RightsViolation
        | K::ConsistencyViolation
        | K::AutonomyViolation
        | K::AutonomyCapped
        | K::AutonomyComputed
        | K::HumanTaskCreated
        | K::ToolViolation
        | K::EscalationPending
        | K::AutonomyDemotion
        | K::ConfidenceViolation => "reasoning".into(),
        _ => "facts".into(),
    }
}

fn build_instance_snapshot(
    evaluator: &Evaluator,
    prior: &CaseInstance,
    instance_id: &str,
) -> serde_json::Value {
    // Build a fresh CaseInstance carrying the evaluator's updated state plus
    // the bookkeeping fields that the evaluator doesn't surface directly.
    let configuration = evaluator.configuration().active.clone();
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
        timers: prior.timers.clone(), // timer lifecycle wired in Step 10
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

/// Shallow diff of top-level case-state keys. Values new or changed in the
/// after-state are included; deletions are represented as `null`.
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

fn row_to_view(r: &ProvenanceRow) -> ProvenanceRecordView {
    // Re-use the projection in provenance_service by going through the
    // existing stored-row path.
    use crate::domain::{ActorRef, FactsView, IntegrityView};
    fn s(v: &serde_json::Value, k: &str) -> Option<String> {
        v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
    }
    let actor = r
        .payload
        .get("actor")
        .map(|a| ActorRef {
            id: s(a, "id").unwrap_or_else(|| "system".into()),
            actor_type: s(a, "type").unwrap_or_else(|| "system".into()),
            name: s(a, "name").unwrap_or_else(|| "system".into()),
        })
        .unwrap_or(ActorRef {
            id: "system".into(),
            actor_type: "system".into(),
            name: "System".into(),
        });
    let facts = r
        .payload
        .get("facts")
        .map(|f| FactsView {
            inputs: f.get("inputs").cloned().unwrap_or(serde_json::json!({})),
            outputs: f.get("outputs").cloned().unwrap_or(serde_json::json!({})),
            metadata: f.get("metadata").cloned().unwrap_or(serde_json::json!({})),
        })
        .unwrap_or(FactsView {
            inputs: serde_json::json!({}),
            outputs: serde_json::json!({}),
            metadata: serde_json::json!({}),
        });
    ProvenanceRecordView {
        id: r.id.clone(),
        instance_id: r.instance_id.clone(),
        timestamp: r.timestamp.to_rfc3339(),
        tier: r.tier.clone(),
        actor,
        event: s(&r.payload, "event").unwrap_or_default(),
        source_state: s(&r.payload, "sourceState").unwrap_or_default(),
        target_state: s(&r.payload, "targetState").unwrap_or_default(),
        facts,
        reasoning: None,
        ai_narrative: None,
        counterfactual: None,
        authority_chain: None,
        integrity: IntegrityView {
            hash: r.hash.clone(),
            previous_hash: r.previous_hash.clone(),
        },
    }
}
