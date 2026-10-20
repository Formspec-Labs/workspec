// Rust guideline compliant 2026-02-21
//! Compares ungoverned replay evidence without discarding generated-ID relations.
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Normalizes only native provenance identities and their closed reference paths.
///
/// Raw evidence, authored data IDs, task IDs, clocks, ordering and all other
/// values remain exact. Unknown or forged references stay visible.
pub fn semantic_native_report(raw: &Value) -> Value {
    let mut result = raw.clone();
    if let Some(id) = result.pointer_mut("/initialization/result/caseLedgerId") {
        let family = id.as_str().unwrap().rsplit_once('_').unwrap().0;
        *id = json!(format!("{family}_<generated-case-ledger>"));
    }
    let mut ids = BTreeMap::new();
    if let Some(records) = result.get_mut("provenance") {
        normalize_records(records, &mut ids);
    }
    if let Some(attempts) = result.get_mut("attempts").and_then(Value::as_array_mut) {
        for attempt in attempts {
            if let Some(records) = attempt.get_mut("provenance") {
                normalize_records(records, &mut ids);
            }
            if let Some(drains) = attempt.get_mut("drains").and_then(Value::as_array_mut) {
                for drain in drains {
                    if let Some(records) = drain.get_mut("provenance") {
                        normalize_records(records, &mut ids);
                    }
                    if let Some(transitions) =
                        drain.get_mut("transitions").and_then(Value::as_array_mut)
                    {
                        for transition in transitions {
                            reference(transition.get_mut("provenance_record_id"), &ids);
                        }
                    }
                    guards(drain.get_mut("guardEvaluations"), &ids);
                }
            }
            for phase in ["before", "after"] {
                if let Some(snapshot) = attempt.get_mut(phase) {
                    snapshot_records(snapshot, &ids);
                }
            }
        }
    }
    if let Some(snapshot) = result.get_mut("finalSnapshot") {
        snapshot_records(snapshot, &ids);
    }
    if let Some(steps) = result
        .pointer_mut("/trace/steps")
        .and_then(Value::as_array_mut)
    {
        for step in steps {
            guards(step.get_mut("guardsEvaluated"), &ids);
        }
    }
    result
}

fn normalize_records(records: &mut Value, ids: &mut BTreeMap<String, String>) {
    if let Some(records) = records.as_array_mut() {
        for record in records {
            if let Some(value) = record.get_mut("id") {
                let id = value.as_str().unwrap();
                let next = ids.len();
                let normalized = ids.entry(id.to_string()).or_insert_with(|| {
                    let family = id.rsplit_once('_').unwrap().0;
                    format!("{family}_<generated-provenance-{next}>")
                });
                *value = json!(normalized);
            }
        }
    }
}

fn reference(value: Option<&mut Value>, ids: &BTreeMap<String, String>) {
    if let Some(value) = value {
        if let Some(normalized) = value.as_str().and_then(|id| ids.get(id)) {
            *value = json!(normalized);
        }
    }
}

fn guards(values: Option<&mut Value>, ids: &BTreeMap<String, String>) {
    if let Some(values) = values.and_then(Value::as_array_mut) {
        for guard in values {
            reference(guard.get_mut("firingRecordId"), ids);
        }
    }
}

fn snapshot_records(snapshot: &mut Value, ids: &BTreeMap<String, String>) {
    if let Some(tasks) = snapshot
        .get_mut("taskHistory")
        .and_then(Value::as_array_mut)
    {
        for task in tasks {
            for kind in ["creation", "completion"] {
                reference(
                    task.get_mut(kind).and_then(|record| record.get_mut("id")),
                    ids,
                );
            }
        }
    }
    if let Some(timers) = snapshot
        .get_mut("timerHistory")
        .and_then(Value::as_array_mut)
    {
        for timer in timers {
            reference(timer.pointer_mut("/creation/id"), ids);
            if let Some(events) = timer.get_mut("events").and_then(Value::as_array_mut) {
                for event in events {
                    reference(event.get_mut("id"), ids);
                }
            }
        }
    }
}
