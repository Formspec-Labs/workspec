//! Background task that fires expired timers by feeding their `event` back
//! through the [`EvalService`].
//!
//! This is a pragmatic polling loop (default every 1s in dev, overridable
//! via `WOS_TIMER_POLL_MS`). It runs a single paginated scan over every
//! instance, inspects `timers` in the stored `CaseInstance`, and — for each
//! timer whose RFC3339 `deadline` has passed — submits a `SubmitEventRequest`
//! as actor `system:timer`. The eval service takes the rest: mutating
//! configuration, appending provenance, clearing the timer from the
//! `CaseInstance`, and broadcasting live update events.
//!
//! Deep timer semantics (pause / resume, business calendar, reschedule on
//! re-entry) live inside `wos-core` and are invoked transparently whenever
//! `Evaluator::from_instance` re-hydrates the timer state, so this task
//! is *only* responsible for the "deadline passed → fire" step.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::AppState;
use crate::domain::SubmitEventRequest;
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
            let Some(event) = t.get("event").and_then(|v| v.as_str()) else { continue };
            let Ok(when) = chrono::DateTime::parse_from_rfc3339(deadline) else {
                continue;
            };
            if when.with_timezone(&Utc) > now {
                continue;
            }
            let req = SubmitEventRequest {
                event: event.to_string(),
                actor_id: "system:timer".to_string(),
                data: Some(serde_json::json!({
                    "timerId": t.get("timerId").cloned(),
                    "firedAt": now.to_rfc3339(),
                })),
            };
            match state.services.eval.submit_event(&row.instance_id, &req).await {
                Ok(result) => tracing::info!(
                    instance = %row.instance_id,
                    event = %req.event,
                    new_configuration = ?result.new_configuration,
                    "fired timer"
                ),
                Err(e) => tracing::warn!(
                    instance = %row.instance_id,
                    event = %req.event,
                    error = %e,
                    "timer fire rejected"
                ),
            }
            // One fire per tick per instance is enough — the next poll will
            // pick up cascading timers emitted by `process_event`.
            break;
        }
    }
    Ok(())
}

pub struct TimerTaskHandle(pub Arc<tokio::task::JoinHandle<()>>);
