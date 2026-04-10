// Rust guideline compliant 2026-02-21

//! Timer management for workflow execution (Kernel S9.7, Lifecycle Detail S6).
//!
//! Timers are created by `startTimer` actions, cancelled by `cancelTimer`
//! actions or state reentry, and fire as `$timeout.*` events.

use std::collections::HashMap;

/// A pending timer.
#[derive(Debug, Clone)]
pub struct Timer {
    /// Timer identifier.
    pub id: String,

    /// Absolute deadline in simulated milliseconds.
    pub deadline_ms: u64,

    /// Event to fire when the timer expires.
    pub fires_event: String,

    /// State that created this timer (for region scoping).
    pub created_in_state: String,
}

/// Timer tracking.
#[derive(Debug, Clone, Default)]
pub struct Timers {
    pending: HashMap<String, Timer>,
}

impl Timers {
    /// Create a new timer. Replaces any existing timer with the same ID.
    pub fn create(&mut self, timer: Timer) {
        self.pending.insert(timer.id.clone(), timer);
    }

    /// Cancel a timer by ID. Returns the cancelled timer if it existed.
    pub fn cancel(&mut self, id: &str) -> Option<Timer> {
        self.pending.remove(id)
    }

    /// Cancel all timers created in a specific state (region scoping).
    pub fn cancel_in_state(&mut self, state_id: &str) -> Vec<Timer> {
        let ids: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, t)| t.created_in_state == state_id)
            .map(|(id, _)| id.clone())
            .collect();

        ids.iter()
            .filter_map(|id| self.pending.remove(id))
            .collect()
    }

    /// Collect all timers that have expired by the given time.
    ///
    /// Returns expired timers in deadline order (earliest first).
    pub fn collect_expired(&mut self, current_time_ms: u64) -> Vec<Timer> {
        let expired_ids: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, t)| t.deadline_ms <= current_time_ms)
            .map(|(id, _)| id.clone())
            .collect();

        let mut expired: Vec<Timer> = expired_ids
            .iter()
            .filter_map(|id| self.pending.remove(id))
            .collect();

        expired.sort_by_key(|t| t.deadline_ms);
        expired
    }

    /// Number of pending timers.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether there are no pending timers.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}
