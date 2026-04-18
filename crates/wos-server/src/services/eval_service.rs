//! Kernel introspection — dry-run enumeration of the transitions whose
//! source state is active. Pure kernel walk, does not touch runtime state.
//!
//! Runtime event submission has moved to the HTTP handler, which calls
//! [`AppRuntime::enqueue_event`] + [`AppRuntime::drain_once`] directly and
//! projects the resulting `DrainOnceResult` into an `EvaluationResultView`.

use std::sync::Arc;

use wos_core::instance::CaseInstance;
use wos_core::KernelDocument;

use crate::domain::AvailableTransitionView;
use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::storage::StorageHandle;

pub struct EvalService {
    storage: StorageHandle,
    bundle: Arc<BundleService>,
}

impl EvalService {
    pub fn new(storage: StorageHandle, bundle: Arc<BundleService>) -> Self {
        Self { storage, bundle }
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
            .ok_or_else(|| {
                ApiError::ServiceUnavailable(format!(
                    "kernel `{}` not loaded",
                    row.definition_url
                ))
            })?;
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
                    // Unguarded transitions are reported satisfied; guarded
                    // transitions leave evaluation to the runtime on event
                    // submission (authoritative).
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
