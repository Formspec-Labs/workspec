// Rust guideline compliant 2026-02-21

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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

    fn can_delegate(&self, _delegator_id: &str, _delegate_id: &str, _scope: &DelegationScope) -> bool {
        true
    }
}

const REVIEW_PREFIX: &str = "review:";

pub struct RoleBasedAccessControl {
    authored: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    delegates: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl RoleBasedAccessControl {
    pub fn new() -> Self {
        Self {
            authored: Arc::new(Mutex::new(HashMap::new())),
            delegates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record_authorship(&self, actor: &str, transition_key: &str) {
        if let Ok(mut map) = self.authored.lock() {
            map.entry(actor.to_string())
                .or_default()
                .insert(transition_key.to_string());
        }
    }

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
