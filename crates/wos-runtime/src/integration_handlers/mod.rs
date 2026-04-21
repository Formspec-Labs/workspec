// Rust guideline compliant 2026-04-14

//! Integration binding kind handlers.
//!
//! Each variant of `IntegrationBindingKind` has a corresponding handler that
//! implements `IntegrationBindingHandler`. The runtime dispatches to the correct
//! handler via `dispatch_integration_binding`.
//!
//! NB.3 adds three CloudEvents handlers: `event_emit`, `event_consume`, and
//! `callback`. All three use the CloudEvents 1.0 envelope types from
//! `crate::cloudevents`.
//!
//! NB.4 adds three more handlers: `tool` (non-HTTP tool invocations),
//! `arazzo_sequence` (multi-step Arazzo orchestration), and `policy_engine`
//! (external policy evaluation with vendor-neutral normalization).

pub(crate) mod arazzo_sequence;
pub(crate) mod callback;
pub(crate) mod event_consume;
pub(crate) mod event_emit;
pub(crate) mod policy_engine;
pub(crate) mod request_response;
pub(crate) mod tool;

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::ProvenanceRecord;

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

pub(crate) use request_response::{load_or_invoke_service_result, InvocationContext};

/// Generate a unique outbound CloudEvent `id` for a given binding invocation.
///
/// Incorporates the current step-result count for uniqueness within an instance
/// lifetime. The `suffix` distinguishes handler types (e.g., `"emit"`, `"cb"`).
/// This is a CloudEvent identifier, NOT an idempotency key.
pub(crate) fn next_outbound_event_id(
    record: &crate::store::RuntimeRecord,
    service_ref: &str,
    suffix: &str,
) -> String {
    let seq = record.step_results.len();
    format!("{service_ref}-{suffix}-{seq}")
}

/// Convert a FEL-evaluated value to a string idempotency key.
///
/// Shared by `request_response` and `tool` handlers. Both handlers evaluate a
/// FEL expression to produce the key and then call this function to coerce the
/// result into a string. `Null` is an error because an absent key produces
/// non-deterministic deduplication behaviour.
pub(crate) fn value_to_idempotency_key(
    value: serde_json::Value,
) -> Result<String, crate::runtime::RuntimeError> {
    match value {
        serde_json::Value::Null => Err(crate::runtime::RuntimeError::Integration(
            "idempotency expression resolved to no value".to_string(),
        )),
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => Ok(value.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Ok(value.to_string()),
    }
}

/// Trait implemented by each integration binding kind handler.
pub(crate) trait IntegrationBindingHandler {
    /// The binding kind this handler services.
    ///
    /// Used for registry lookups and diagnostics. Currently unused in dispatch
    /// (dispatch is by match arm) but required for future dynamic registries (NB.3/NB.4).
    #[allow(dead_code)]
    fn kind(&self) -> IntegrationBindingKind;

    /// Execute the binding and return the provenance records it produces.
    ///
    /// The handler MAY mutate `record.instance.case_state` (output binding) and
    /// MAY append to `record.step_results` (idempotency replay). All other
    /// runtime state is accessed read-only through `ctx`.
    fn execute(
        &self,
        ctx: &InvocationContext<'_>,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError>;
}

/// Dispatch an integration binding to the correct handler by kind.
///
/// All seven `IntegrationBindingKind` variants have handlers:
/// - `RequestResponse` — synchronous HTTP-style invocation
/// - `EventEmit` — outbound CloudEvent emission
/// - `EventConsume` — inbound CloudEvent consumption
/// - `Callback` — bidirectional CloudEvent callback
/// - `Tool` — non-HTTP tool invocation (NB.4)
/// - `ArazzoSequence` — multi-step Arazzo orchestration (NB.4)
/// - `PolicyEngine` — external policy evaluation (NB.4)
pub(crate) fn dispatch_integration_binding(
    ctx: &InvocationContext<'_>,
    record: &mut RuntimeRecord,
    kernel: &KernelDocument,
    observed: &ObservedAction,
    service_ref: &str,
    binding: &IntegrationBinding,
    now_iso: &str,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    match binding.kind {
        IntegrationBindingKind::RequestResponse => request_response::RequestResponseHandler
            .execute(ctx, record, kernel, observed, service_ref, binding, now_iso),
        IntegrationBindingKind::EventEmit => event_emit::EventEmitHandler.execute(
            ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        ),
        IntegrationBindingKind::EventConsume => event_consume::EventConsumeHandler.execute(
            ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        ),
        IntegrationBindingKind::Callback => callback::CallbackHandler.execute(
            ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        ),
        IntegrationBindingKind::Tool => {
            tool::ToolHandler.execute(ctx, record, kernel, observed, service_ref, binding, now_iso)
        }
        IntegrationBindingKind::ArazzoSequence => arazzo_sequence::ArazzoHandler.execute(
            ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        ),
        IntegrationBindingKind::PolicyEngine => policy_engine::PolicyEngineHandler.execute(
            ctx,
            record,
            kernel,
            observed,
            service_ref,
            binding,
            now_iso,
        ),
    }
}
