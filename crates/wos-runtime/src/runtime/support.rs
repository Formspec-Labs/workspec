// Rust guideline compliant 2026-02-21

//! Small runtime support helpers.
//!
//! These helpers are shared by event draining, action execution, task commands,
//! and timer materialization. They are kept out of `runtime.rs` so the drain
//! loop can focus on orchestration.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wos_core::model::kernel::ImpactLevel;

use super::RuntimeError;

pub(super) fn format_timestamp(timestamp_ms: u64) -> Result<String, RuntimeError> {
    let nanos = i128::from(timestamp_ms) * 1_000_000;
    let nanos_i64 = i64::try_from(nanos)
        .map_err(|_| RuntimeError::Clock("timestamp exceeds supported range".to_string()))?;
    let timestamp = OffsetDateTime::from_unix_timestamp_nanos(nanos_i64.into())
        .map_err(|error| RuntimeError::Clock(error.to_string()))?;
    timestamp
        .format(&Rfc3339)
        .map_err(|error| RuntimeError::Clock(error.to_string()))
}

pub(super) fn parse_timestamp(timestamp: &str) -> Result<u64, RuntimeError> {
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339)
        .map_err(|error| RuntimeError::Clock(error.to_string()))?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    u64::try_from(millis).map_err(|_| RuntimeError::Clock("negative timestamp".to_string()))
}

pub(super) fn merge_case_state(target: &mut serde_json::Value, updates: &serde_json::Value) {
    if let (Some(target_object), Some(update_object)) =
        (target.as_object_mut(), updates.as_object())
    {
        for (key, value) in update_object {
            target_object.insert(key.clone(), value.clone());
        }
    }
}

pub(super) fn normalize_semver_range_expression(expression: &str) -> String {
    expression
        .split("||")
        .map(|clause| {
            let clause = clause.trim();
            if clause.contains(',') {
                clause.to_string()
            } else {
                clause.split_whitespace().collect::<Vec<_>>().join(", ")
            }
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

pub(super) fn impact_level_label(level: ImpactLevel) -> String {
    match level {
        ImpactLevel::RightsImpacting => "rights-impacting",
        ImpactLevel::SafetyImpacting => "safety-impacting",
        ImpactLevel::Operational => "operational",
        ImpactLevel::Informational => "informational",
    }
    .to_string()
}

pub(super) fn make_task_id(instance_id: &str, ordinal: u64, task_ref: &str) -> String {
    let encoded_instance_id = URL_SAFE_NO_PAD.encode(instance_id);
    format!("wos-task:{encoded_instance_id}:{ordinal}:{task_ref}")
}
