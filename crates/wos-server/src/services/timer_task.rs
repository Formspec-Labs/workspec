//! Background task that fires expired timers by enqueueing their `event`
//! through [`AppRuntime`](crate::runtime::AppRuntime).
//!
//! Pragmatic polling loop (default every 1s in dev; `WOS_TIMER_POLL_MS`).
//! Paginates over instances, inspects `timers` in the stored `CaseInstance`,
//! and for each timer whose RFC3339 `deadline` has passed, enqueues the
//! timer's event as actor `system:timer` and runs one drain step. The
//! runtime mutates configuration, appends provenance, and clears the
//! timer.
//!
//! Deep timer semantics (pause / resume, business calendar, reschedule
//! on re-entry) live inside `wos-core` and are invoked transparently
//! whenever the runtime re-hydrates timer state; this task is only the
//! "deadline passed → fire" trigger.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::AppState;
use crate::storage::InstanceQuery;

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    let interval = Duration::from_millis(state.cfg.timer_poll_ms.max(250));
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(e) = tick_once(&state).await {
                tracing::warn!(error = %e, "timer tick failed");
            }
        }
    })
}

async fn tick_once(state: &AppState) -> anyhow::Result<()> {
    let page = state
        .storage
        .list_instances(InstanceQuery {
            page: 1,
            page_size: 500,
            ..Default::default()
        })
        .await?;

    let now = Utc::now();
    for row in &page.items {
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
                "actor": "system:timer",
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
            // One fire per tick per instance — cascading timers are picked
            // up on the next poll.
            break;
        }
    }
    Ok(())
}

pub struct TimerTaskHandle(pub Arc<tokio::task::JoinHandle<()>>);
