//! Permissive `AccessControl` impl — allows every action. Role-based
//! transition gating, impact-level autonomy caps, and delegation chain
//! validation are enforced separately through `GovernanceService` and
//! `AgentService`; once those feed decisions back through a real
//! `AccessControl` impl, swap this out for a policy-backed one.

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
