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

use thiserror::Error;
use wos_core::instance::{ActiveHold, CaseInstance, GovernanceState};

use crate::storage::{StorageError, StorageHandle};

/// Private sentinel string used to thread "hold not found" past
/// [`crate::storage::Storage::update_instance_atomic`]'s `StorageError`
/// channel — the trait owns the closure return type, so the service
/// re-enters with `StorageError::Other(SENTINEL)`, then translates to
/// [`HoldServiceError::NotFound`] before the value escapes the service.
/// No caller outside this module ever sees this string.
//
// TODO(WS-082-followup): drop this once `Storage::update_instance_atomic`
// grows a typed-return variant (e.g. `update_instance_atomic_returns<T>`)
// so the mutator can return a domain `Result<T, E>` directly.
const HOLD_NOT_FOUND_SENTINEL: &str = "hold-not-found";

/// Service-layer errors for [`HoldService`]. Promoted from a brittle
/// `StorageError::Other("hold-not-found")` sentinel match at the HTTP
/// layer (review #3, finding 1) so callers pattern-match on a typed
/// variant; any other call site returning the same string payload no
/// longer collides with the 404 mapping.
#[derive(Debug, Error)]
pub enum HoldServiceError {
    /// `hold_idx` was past the end of `governanceState.activeHolds`,
    /// or `governanceState` was absent entirely. Maps to HTTP `404`.
    #[error("hold not found at index {index}")]
    NotFound { index: usize },

    /// Underlying storage error — pass through to the HTTP layer's
    /// generic `StorageError → ApiError` conversion.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub struct HoldService;

impl HoldService {
    /// Read all active holds. Returns an empty `Vec` when the instance
    /// has no `governanceState` populated yet — matches the pre-WS-082
    /// HTTP wire shape (`GET /api/instances/:id/holds` of an empty
    /// instance returns `[]`, not 404).
    pub async fn list(
        storage: &StorageHandle,
        instance_id: &str,
    ) -> Result<Vec<ActiveHold>, HoldServiceError> {
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
    ) -> Result<usize, HoldServiceError> {
        // safe: `update_instance_atomic` invokes the mutator exactly
        // once (see `crates/wos-server-sqlite/src/lib.rs`). The
        // `Arc<Mutex<…>>` is not for concurrency — it is the smallest
        // shape that lets a `Fn` closure thread a value back out under
        // the current trait signature. If the trait ever grows a retry
        // loop, replace with a typed return on the mutator.
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
    /// for the handler to optionally echo back. Out-of-range or missing
    /// `governanceState` surfaces as [`HoldServiceError::NotFound`] —
    /// the storage trait's closure return type forces a private string
    /// sentinel hop, but it is translated before leaving the service.
    pub async fn release(
        storage: &StorageHandle,
        instance_id: &str,
        hold_idx: usize,
    ) -> Result<ActiveHold, HoldServiceError> {
        // safe: `update_instance_atomic` invokes the mutator exactly
        // once (see `crates/wos-server-sqlite/src/lib.rs`). The
        // `Arc<Mutex<…>>` is not for concurrency — it is the smallest
        // shape that lets a `Fn` closure thread a value back out under
        // the current trait signature. If the trait ever grows a retry
        // loop, replace with a typed return on the mutator.
        let released = Arc::new(Mutex::new(None::<ActiveHold>));
        let captured = released.clone();
        let result = storage
            .update_instance_atomic(instance_id, &move |row| {
                let mut instance: CaseInstance = serde_json::from_value(row.instance_json.clone())
                    .map_err(|e| StorageError::Other(format!("instance_json invalid: {e}")))?;
                let gov = instance
                    .governance_state
                    .as_mut()
                    .ok_or_else(|| StorageError::Other(HOLD_NOT_FOUND_SENTINEL.into()))?;
                if hold_idx >= gov.active_holds.len() {
                    return Err(StorageError::Other(HOLD_NOT_FOUND_SENTINEL.into()));
                }
                let removed = gov.active_holds.remove(hold_idx);
                row.instance_json = serde_json::to_value(&instance)
                    .map_err(|e| StorageError::Other(format!("instance serialize: {e}")))?;
                *captured.lock().unwrap() = Some(removed);
                Ok(Vec::new())
            })
            .await;
        match result {
            Ok(_) => Ok(released.lock().unwrap().take().unwrap()),
            Err(StorageError::Other(msg)) if msg == HOLD_NOT_FOUND_SENTINEL => {
                Err(HoldServiceError::NotFound { index: hold_idx })
            }
            Err(other) => Err(HoldServiceError::Storage(other)),
        }
    }
}
