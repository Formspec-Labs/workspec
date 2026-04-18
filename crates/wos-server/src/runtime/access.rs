//! Permissive `AccessControl` impl for Phase 1 — allows every action.
//!
//! Real policy decisions (role-based transition gating, impact-level
//! autonomy caps, delegation chain validation) are layered on in
//! subsequent phases via `GovernanceService` and `AgentService`.

use wos_core::model::governance::DelegationScope;
use wos_core::traits::AccessControl;

#[derive(Debug, Default)]
pub struct PermissiveAccessControl;

impl AccessControl for PermissiveAccessControl {
    fn can_transition(&self, _actor_id: &str, _transition_event: &str) -> bool {
        true
    }

    fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
        true
    }

    fn can_delegate(
        &self,
        _delegator_id: &str,
        _delegate_id: &str,
        _scope: &DelegationScope,
    ) -> bool {
        true
    }
}
