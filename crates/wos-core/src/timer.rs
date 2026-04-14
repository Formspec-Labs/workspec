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

    /// Original ISO 8601 duration string (for tolerance violation reporting).
    pub duration_iso: String,

    /// Duration in milliseconds (for tolerance tier calculation).
    pub duration_ms: u64,
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

    /// Iterate over pending timers in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &Timer> {
        self.pending.values()
    }
}

/// Maximum tolerance for a timer based on its duration tier (Runtime S7.2).
///
/// | Duration tier | Max tolerance |
/// |---------------|---------------|
/// | Under 1 hour  | 1 second      |
/// | 1–24 hours    | 1 minute      |
/// | Over 24 hours | 5 minutes     |
pub fn max_tolerance_ms(duration_ms: u64) -> u64 {
    const MS_PER_SECOND: u64 = 1_000;
    const MS_PER_MINUTE: u64 = 60 * MS_PER_SECOND;
    const MS_PER_HOUR: u64 = 60 * MS_PER_MINUTE;

    if duration_ms < MS_PER_HOUR {
        MS_PER_SECOND
    } else if duration_ms <= 24 * MS_PER_HOUR {
        MS_PER_MINUTE
    } else {
        5 * MS_PER_MINUTE
    }
}

/// Format a millisecond tolerance value as an ISO 8601 duration string.
pub fn tolerance_to_iso(ms: u64) -> String {
    const MS_PER_SECOND: u64 = 1_000;
    const MS_PER_MINUTE: u64 = 60 * MS_PER_SECOND;

    if ms % MS_PER_MINUTE == 0 {
        format!("PT{}M", ms / MS_PER_MINUTE)
    } else {
        format!("PT{}S", ms / MS_PER_SECOND)
    }
}
