//! Hold lifecycle CRUD over the typed [`CaseInstance`] (Gov §3.6).
//!
//! Replaces the raw `Value::as_object_mut()` plumbing that used to live
//! inline in [`crate::http::instances`] handlers (WS-082). Round-trips
//! through `CaseInstance` → `GovernanceState::active_holds`, so any
//! drift between the on-wire JSON shape and the typed struct surfaces
//! as a serde error at one site instead of as an `unwrap()` panic in
//! six places.
//!
//! Storage is still `update_instance_atomic` — this service is pure
//! domain logic over the typed shape, no transaction or locking
//! semantics of its own.

use std::sync::{Arc, Mutex};

use wos_core::instance::{ActiveHold, CaseInstance, GovernanceState};

use crate::storage::{StorageError, StorageHandle};

/// Sentinel string used by [`HoldService::release`] to thread an
/// out-of-range index past `update_instance_atomic`'s `StorageError` →
/// HTTP `404` mapping. The handler matches on this exact text.
pub const HOLD_NOT_FOUND_SENTINEL: &str = "hold-not-found";

pub struct HoldService;

impl HoldService {
    /// Read all active holds. Returns an empty `Vec` when the instance
    /// has no `governanceState` populated yet — matches the pre-WS-082
    /// HTTP wire shape (`GET /api/instances/:id/holds` of an empty
    /// instance returns `[]`, not 404).
    pub async fn list(
        storage: &StorageHandle,
        instance_id: &str,
    ) -> Result<Vec<ActiveHold>, StorageError> {
        let row = storage
            .get_instance(instance_id)
            .await?
            .ok_or(StorageError::NotFound)?;
        let instance: CaseInstance = serde_json::from_value(row.instance_json)
            .map_err(|e| StorageError::Other(format!("instance_json invalid: {e}")))?;
        Ok(instance
            .governance_state
            .map(|g| g.active_holds)
            .unwrap_or_default())
    }

    /// Append `hold` to the instance's `governanceState.activeHolds`.
    /// Returns the index where it was placed so the handler can echo
    /// `holdIndex` back to the caller. If `governanceState` was `None`
    /// it is materialised with a default [`GovernanceState`]; this
    /// preserves the pre-WS-082 behaviour of `entry().or_insert_with`
    /// on the raw JSON object.
    pub async fn append(
        storage: &StorageHandle,
        instance_id: &str,
        hold: ActiveHold,
    ) -> Result<usize, StorageError> {
        let final_index = Arc::new(Mutex::new(0usize));
        let captured = final_index.clone();
        let hold = Arc::new(hold);
        storage
            .update_instance_atomic(instance_id, &move |row| {
                let mut instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
                    .map_err(|e| StorageError::Other(format!("instance_json invalid: {e}")))?;
                let gov = instance
                    .governance_state
                    .get_or_insert_with(GovernanceState::default);
                gov.active_holds.push((*hold).clone());
                let idx = gov.active_holds.len() - 1;
                row.instance_json = serde_json::to_value(&instance)
                    .map_err(|e| StorageError::Other(format!("instance serialize: {e}")))?;
                *captured.lock().unwrap() = idx;
                Ok(Vec::new())
            })
            .await?;
        Ok(*final_index.lock().unwrap())
    }

    /// Remove the hold at `hold_idx`. Returns the removed [`ActiveHold`]
    /// for the handler to optionally echo back; signals out-of-range or
    /// missing `governanceState` via [`HOLD_NOT_FOUND_SENTINEL`] so the
    /// HTTP layer can map it to 404.
    pub async fn release(
        storage: &StorageHandle,
        instance_id: &str,
        hold_idx: usize,
    ) -> Result<ActiveHold, StorageError> {
        let released = Arc::new(Mutex::new(None::<ActiveHold>));
        let captured = released.clone();
        storage
            .update_instance_atomic(instance_id, &move |row| {
                let mut instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
                    .map_err(|e| StorageError::Other(format!("instance_json invalid: {e}")))?;
                let gov = instance.governance_state.as_mut().ok_or_else(|| {
                    StorageError::Other(HOLD_NOT_FOUND_SENTINEL.into())
                })?;
                if hold_idx >= gov.active_holds.len() {
                    return Err(StorageError::Other(HOLD_NOT_FOUND_SENTINEL.into()));
                }
                let removed = gov.active_holds.remove(hold_idx);
                row.instance_json = serde_json::to_value(&instance)
                    .map_err(|e| StorageError::Other(format!("instance serialize: {e}")))?;
                *captured.lock().unwrap() = Some(removed);
                Ok(Vec::new())
            })
            .await?;
        Ok(released.lock().unwrap().take().unwrap())
    }
}
