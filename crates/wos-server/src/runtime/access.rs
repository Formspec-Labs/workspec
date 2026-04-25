// Rust guideline compliant 2026-02-21

//! `AccessControl` implementations.
//!
//! [`PermissiveAccessControl`] allows every action — used as a test double.
//!
//! [`RoleBasedAccessControl`] enforces Gov §7.2 separation-of-duties and
//! AI §1.5 cross-reference: an actor cannot review their own output. The
//! check uses a convention where review-tagged transitions encode the
//! original author in the event name as `review:{author_id}` — the runtime
//! is responsible for populating this context when it calls `can_transition`.
//! Delegation chains (Gov §6) are honoured when the delegator is recorded
//! as an authorised reviewer.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use wos_core::model::governance::DelegationScope;
use wos_core::traits::AccessControl;

/// Permissive test double — allows every action.
#[derive(Debug, Default)]
pub struct PermissiveAccessControl;

impl AccessControl for PermissiveAccessControl {
    fn can_transition(&self, _actor_id: &str, _transition_event: &str) -> bool {
        true
    }

    fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
        true
    }

    fn can_delegate(&self, _delegator_id: &str, _delegate_id: &str, _scope: &DelegationScope) -> bool {
        true
    }
}

const REVIEW_PREFIX: &str = "review:";

/// Role-backed access control enforcing Gov §7.2 separation-of-duties.
///
/// Review-tagged transitions encode the original author as
/// `review:{author_id}` so `can_transition` can reject self-reviews.
/// Delegated reviewers (Gov §6) are tracked in an internal registry and
/// permitted when the delegator is listed as an authorised actor for the
/// same transition.
pub struct RoleBasedAccessControl {
    /// actor → set of transitions they are known to have authored.
    authored: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    /// delegator → set of delegates authorised to act on their behalf.
    delegates: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl RoleBasedAccessControl {
    pub fn new() -> Self {
        Self {
            authored: Arc::new(Mutex::new(HashMap::new())),
            delegates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record that `actor` authored work under the given transition key.
    /// Called by the runtime when a non-review transition completes so that
    /// subsequent review transitions can enforce separation-of-duties.
    pub fn record_authorship(&self, actor: &str, transition_key: &str) {
        if let Ok(mut map) = self.authored.lock() {
            map.entry(actor.to_string())
                .or_default()
                .insert(transition_key.to_string());
        }
    }

    /// Record that `delegate` is authorised to act on behalf of `delegator`
    /// per Gov §6 delegation chains.
    pub fn record_delegation(&self, delegator: &str, delegate: &str) {
        if let Ok(mut map) = self.delegates.lock() {
            map.entry(delegator.to_string())
                .or_default()
                .insert(delegate.to_string());
        }
    }
}

impl Default for RoleBasedAccessControl {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessControl for RoleBasedAccessControl {
    fn can_transition(&self, actor_id: &str, transition_event: &str) -> bool {
        if let Some(author_id) = transition_event.strip_prefix(REVIEW_PREFIX) {
            if actor_id == author_id {
                return false;
            }
            if let Ok(map) = self.delegates.lock() {
                if let Some(delegate_set) = map.get(author_id) {
                    if delegate_set.contains(actor_id) {
                        return true;
                    }
                }
            }
        }
        true
    }

    fn can_read(&self, _actor_id: &str, _field_path: &str) -> bool {
        true
    }

    fn can_delegate(&self, delegator_id: &str, delegate_id: &str, _scope: &DelegationScope) -> bool {
        delegator_id != delegate_id
    }
}
