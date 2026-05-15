// Rust guideline compliant 2026-02-21

//! Timer materialization helpers for the reference runtime.
//!
//! This module holds the business-calendar and timer-provenance helpers used
//! by the in-memory adapter. Moving them out of `runtime.rs` makes the event
//! loop easier to read while preserving the current lazy-deadline semantics.

use chrono::DateTime;
use wos_core::business_calendar::{
    BusinessCalendarDocument, BusinessCalendarError, next_business_moment,
};
use wos_core::instance::{PendingEvent, WorkflowProcess};
use wos_core::timer::{Timer, max_tolerance_ms, tolerance_to_iso};
use wos_core::{ProvenanceKind, ProvenanceRecord};

use super::{RuntimeError, format_timestamp, parse_timestamp};

/// Materialize due timers into pending events and provenance records.
///
/// # Errors
/// Returns an error when a stored timer timestamp cannot be parsed.
pub(super) fn materialize_due_timers(
    instance: &mut WorkflowProcess,
    now_ms: u64,
    now_iso: &str,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    let mut due = Vec::new();
    let mut remaining = Vec::new();

    for timer in instance.timers.drain(..) {
        if parse_timestamp(&timer.deadline)? <= now_ms {
            due.push(timer);
        } else {
            remaining.push(timer);
        }
    }
    instance.timers = remaining;

    let mut provenance = Vec::new();
    for timer in due {
        provenance.push(ProvenanceRecord::timer_fired(&timer.timer_id, &timer.event));
        let deadline_ms = parse_timestamp(&timer.deadline)?;
        let lateness_ms = now_ms.saturating_sub(deadline_ms);
        let max_tolerance = max_tolerance_ms(timer.duration_ms.unwrap_or(0));
        if lateness_ms > max_tolerance {
            let tolerance_iso = tolerance_to_iso(max_tolerance);
            provenance.push(ProvenanceRecord::tolerance_violation(
                &timer.timer_id,
                timer.duration_iso.as_deref().unwrap_or("P0D"),
                &tolerance_iso,
            ));
        }
        instance.pending_events.push(PendingEvent {
            event: timer.event.clone(),
            actor_id: None,
            data: Some(serde_json::json!({ "timerId": timer.timer_id })),
            timestamp: now_iso.to_string(),
            idempotency_token: None,
        });
    }

    Ok(provenance)
}

/// Convert all timers to `TimerState`, computing calendar-adjusted deadlines lazily.
///
/// Returns `(states, convergence_error_timer_ids)`. The second element lists
/// timer IDs whose deadline fell back to naive wall-clock time because the
/// business calendar evaluator did not converge.
///
/// # Errors
/// Returns an error when deadline materialization fails.
pub(super) fn timers_to_state(
    timers: &wos_core::timer::Timers,
    calendar: Option<&BusinessCalendarDocument>,
) -> Result<(Vec<wos_core::instance::TimerState>, Vec<String>), RuntimeError> {
    let mut states = Vec::with_capacity(timers.len());
    let mut convergence_error_ids = Vec::new();
    for timer in timers.iter() {
        let (state, had_error) = timer_to_state(timer, calendar)?;
        if had_error {
            convergence_error_ids.push(state.timer_id.clone());
        }
        states.push(state);
    }
    Ok((states, convergence_error_ids))
}

/// Convert a `Timer` to a lazily evaluated `TimerState`.
///
/// # Errors
/// Returns an error when timestamp conversion fails.
fn timer_to_state(
    timer: &Timer,
    calendar: Option<&BusinessCalendarDocument>,
) -> Result<(wos_core::instance::TimerState, bool), RuntimeError> {
    let (deadline_ms, had_convergence_error) = match calendar {
        Some(cal) => business_deadline_ms(timer, cal)?,
        None => (timer.deadline_ms, false),
    };

    let state = wos_core::instance::TimerState {
        timer_id: timer.id.clone(),
        deadline: format_timestamp(deadline_ms)?,
        event: timer.fires_event.clone(),
        scope_state: if timer.created_in_state.is_empty() {
            None
        } else {
            Some(timer.created_in_state.clone())
        },
        duration_iso: Some(timer.duration_iso.clone()),
        duration_ms: Some(timer.duration_ms),
        created_at_ms: Some(timer.created_at_ms),
    };
    Ok((state, had_convergence_error))
}

/// Compute a business-calendar-adjusted deadline for `timer`.
///
/// `Timer.created_at_ms` remains the authoritative start time so repeated
/// recomputation across drains stays deterministic.
///
/// # Errors
/// Returns an error when the timer timestamps or duration are out of range.
fn business_deadline_ms(
    timer: &Timer,
    calendar: &BusinessCalendarDocument,
) -> Result<(u64, bool), RuntimeError> {
    let start_ms = timer.created_at_ms;
    let start_secs = i64::try_from(start_ms / 1000)
        .map_err(|_| RuntimeError::Clock("timer start timestamp out of range".to_string()))?;
    let start_utc = DateTime::from_timestamp(start_secs, 0)
        .ok_or_else(|| RuntimeError::Clock("invalid timer start timestamp".to_string()))?;

    let duration = chrono::Duration::milliseconds(
        i64::try_from(timer.duration_ms)
            .map_err(|_| RuntimeError::Clock("timer duration out of range".to_string()))?,
    );

    match next_business_moment(start_utc, duration, calendar) {
        Ok(result) => {
            let result_ms = u64::try_from(result.timestamp_millis())
                .map_err(|_| RuntimeError::Clock("business deadline out of range".to_string()))?;
            Ok((result_ms, false))
        }
        Err(BusinessCalendarError::DidNotConverge { .. }) => {
            // Degenerate calendar falls back to naive wall-clock time so the
            // timer still fires and provenance can record the convergence issue.
            Ok((timer.deadline_ms, true))
        }
    }
}

/// Annotate `TimerCreated` records with `calendarVersion`.
pub(super) fn annotate_timer_created_with_calendar_version(
    records: &mut [ProvenanceRecord],
    calendar: &BusinessCalendarDocument,
) {
    let version = calendar
        .version
        .as_deref()
        .map(serde_json::Value::from)
        .unwrap_or(serde_json::Value::Null);

    for record in records.iter_mut() {
        if record.record_kind != ProvenanceKind::TimerCreated {
            continue;
        }
        match &mut record.data {
            Some(serde_json::Value::Object(map)) => {
                map.insert("calendarVersion".to_string(), version.clone());
            }
            other => {
                panic!("TimerCreated.data must be an Object; got {other:?}");
            }
        }
    }
}

/// Annotate `TimerCreated` records with convergence fallback metadata.
pub(super) fn annotate_timer_created_with_convergence_error(
    records: &mut [ProvenanceRecord],
    timer_ids: &[String],
) {
    if timer_ids.is_empty() {
        return;
    }
    for record in records.iter_mut() {
        if record.record_kind != ProvenanceKind::TimerCreated {
            continue;
        }
        let record_timer_id = record
            .data
            .as_ref()
            .and_then(|d| d.get("timerId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !timer_ids.contains(&record_timer_id) {
            continue;
        }
        match &mut record.data {
            Some(serde_json::Value::Object(map)) => {
                map.insert(
                    "calendarVersionConvergenceError".to_string(),
                    serde_json::Value::Bool(true),
                );
            }
            other => {
                panic!("TimerCreated.data must be an Object; got {other:?}");
            }
        }
    }
}
