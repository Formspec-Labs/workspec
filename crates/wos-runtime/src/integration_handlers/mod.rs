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

pub(crate) mod callback;
pub(crate) mod event_consume;
pub(crate) mod event_emit;
pub(crate) mod request_response;

use wos_core::eval::ObservedAction;
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::ProvenanceRecord;

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

pub(crate) use request_response::{InvocationContext, load_or_invoke_service_result};

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
/// `RequestResponse`, `EventEmit`, `EventConsume`, and `Callback` are
/// implemented. Remaining kinds (`ArazzoSequence`, `Tool`, `PolicyEngine`)
/// return `RuntimeError::UnsupportedBindingKind` (NB.4 work).
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
        IntegrationBindingKind::RequestResponse => {
            request_response::RequestResponseHandler.execute(
                ctx, record, kernel, observed, service_ref, binding, now_iso,
            )
        }
        IntegrationBindingKind::EventEmit => event_emit::EventEmitHandler.execute(
            ctx, record, kernel, observed, service_ref, binding, now_iso,
        ),
        IntegrationBindingKind::EventConsume => event_consume::EventConsumeHandler.execute(
            ctx, record, kernel, observed, service_ref, binding, now_iso,
        ),
        IntegrationBindingKind::Callback => callback::CallbackHandler.execute(
            ctx, record, kernel, observed, service_ref, binding, now_iso,
        ),
        other => Err(RuntimeError::UnsupportedBindingKind(other)),
    }
}
