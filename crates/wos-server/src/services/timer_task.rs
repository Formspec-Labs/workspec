//! Background task that fires expired timers by enqueueing their `event`
//! through [`AppRuntime`](crate::runtime::AppRuntime).
//!
//! Pragmatic polling loop (default every 1s in dev; `WOS_TIMER_POLL_MS`).
//! Collects every instance via [`list_instances_all_pages`](crate::storage::list_instances_all_pages),
//! inspects `timers` in the stored `CaseInstance`, and for each timer whose
//! RFC3339 `deadline` has passed, enqueues the timer's event as actor
//! `system:timer` and runs one drain step. The runtime mutates configuration,
//! appends provenance, and clears the timer.
//!
//! Deep timer semantics (pause / resume, business calendar, reschedule
//! on re-entry) live inside `wos-core` and are invoked transparently
//! whenever the runtime re-hydrates timer state; this task is only the
//! "deadline passed → fire" trigger.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::AppState;
use crate::storage::{InstanceQuery, LIST_INSTANCES_PAGE_SIZE_MAX, list_instances_all_pages};

/// Cadence for the session-table sweep (WS-052). Runs once per ticker
/// iteration where `now - last_sweep_at >= SESSION_SWEEP_INTERVAL`, so the
/// fast timer-poll cadence is not used for cleanup work.
const SESSION_SWEEP_INTERVAL: chrono::Duration = chrono::Duration::hours(24);

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    let interval = Duration::from_millis(state.cfg.timer_poll_ms.max(250));
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_sweep_at: Option<DateTime<Utc>> = None;
        loop {
            ticker.tick().await;
            if let Err(e) = tick_once(&state).await {
                tracing::warn!(error = %e, "timer tick failed");
            }
            if state.cfg.session_sweep_enabled {
                let now = Utc::now();
                let due = last_sweep_at
                    .map(|prev| now - prev >= SESSION_SWEEP_INTERVAL)
                    .unwrap_or(true);
                if due {
                    match state.storage.sweep_expired_sessions(now).await {
                        Ok(count) => {
                            tracing::info!(deleted_rows = count, "session sweep");
                            last_sweep_at = Some(now);
                        }
                        Err(e) => tracing::warn!(error = %e, "session sweep failed"),
                    }
                }
            }
        }
    })
}

/// Run a single sweep of the timer-poll loop. Visible for integration tests
/// (WS-010) so a fixture can drive timer firing deterministically without
/// spawning the periodic task.
pub async fn tick_once(state: &AppState) -> anyhow::Result<()> {
    let now = Utc::now();
    let all = list_instances_all_pages(
        &state.storage,
        InstanceQuery::default(),
        LIST_INSTANCES_PAGE_SIZE_MAX,
    )
    .await?;

    for row in &all {
        let Some(timers) = row.timers().as_array() else { continue };
        for t in timers {
            let Some(deadline) = t.get("deadline").and_then(|v| v.as_str()) else { continue };
            let Some(event_name) = t.get("event").and_then(|v| v.as_str()) else { continue };
            let Ok(when) = chrono::DateTime::parse_from_rfc3339(deadline) else {
                continue;
            };
            if when.with_timezone(&Utc) > now {
                continue;
            }
            let envelope = serde_json::json!({
                "event": event_name,
                "actorId": "system:timer",
                "timestamp": now.to_rfc3339(),
                "data": {
                    "timerId": t.get("timerId").cloned(),
                    "firedAt": now.to_rfc3339(),
                },
            });
            if let Err(e) = state
                .runtime
                .enqueue_event(&row.instance_id, envelope)
                .await
            {
                tracing::warn!(
                    instance = %row.instance_id,
                    event = event_name,
                    error = %e,
                    "timer enqueue failed"
                );
                continue;
            }
            match state.runtime.drain_once(&row.instance_id).await {
                Ok(result) => tracing::info!(
                    instance = %row.instance_id,
                    event = event_name,
                    transitions = result.transitions.len(),
                    "fired timer"
                ),
                Err(e) => tracing::warn!(
                    instance = %row.instance_id,
                    event = event_name,
                    error = %e,
                    "timer fire rejected"
                ),
            }
            break;
        }
    }
    Ok(())
}

pub struct TimerTaskHandle(pub Arc<tokio::task::JoinHandle<()>>);
