// Rust guideline compliant 2026-05-02

//! Phase 2 — Resolve mapping records → bind every PolicyObject to a target.
//!
//! `SA-MUST-cmp-011`: every PolicyObject MUST have exactly one mapping.
//! Missing mapping halts with `unmapped-input`. Multiple mappings on the
//! same subject halt with `unmapped-input` plus `mapping-collision` detail.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{CompileError, FailureKind};
use wos_studio_lint::Workspace;

#[derive(Debug)]
pub struct MappingResult<'a> {
    /// Map from PolicyObject id → its single resolved Mapping record.
    pub by_subject: BTreeMap<String, &'a Value>,
}

pub fn run<'a>(
    ws: &'a Workspace,
    referenced_policy_objects: &[String],
) -> Result<MappingResult<'a>, CompileError> {
    let mut by_subject: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for (_doc, record) in ws.mapping_records() {
        let Some(subject) = record.get("policyObjectRef").and_then(Value::as_str) else {
            continue;
        };
        by_subject.entry(subject.to_string()).or_default().push(record);
    }

    let mut missing: Vec<String> = Vec::new();
    let mut multiple: Vec<String> = Vec::new();
    let mut resolved: BTreeMap<String, &Value> = BTreeMap::new();
    for id in referenced_policy_objects {
        match by_subject.get(id) {
            None => missing.push(id.clone()),
            Some(v) if v.len() == 1 => {
                resolved.insert(id.clone(), v[0]);
            }
            Some(v) => multiple.push(format!("{id} ({} mappings)", v.len())),
        }
    }

    if !missing.is_empty() {
        return Err(CompileError::halt_with(
            2,
            FailureKind::UnmappedInput,
            format!(
                "{} approved PolicyObject(s) lack a Mapping record",
                missing.len()
            ),
            missing,
        ));
    }
    if !multiple.is_empty() {
        return Err(CompileError::halt_with(
            2,
            FailureKind::UnmappedInput,
            format!(
                "{} PolicyObject(s) have multiple Mapping records; pick \
                 exactly one or scope by Effectiveness",
                multiple.len()
            ),
            multiple,
        ));
    }

    Ok(MappingResult { by_subject: resolved })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    #[test]
    fn halts_on_missing_mapping() {
        let ws = ws_from(vec![(
            "m.json",
            json!({"$wosStudioMapping": "1.0", "mappings": []}),
        )]);
        let err = run(&ws, &["pol-x".to_string()]).expect_err("halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::UnmappedInput);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn halts_on_multiple_mappings_for_same_subject() {
        let ws = ws_from(vec![(
            "m.json",
            json!({
                "$wosStudioMapping": "1.0",
                "mappings": [
                    {"id": "m1", "policyObjectRef": "pol-x"},
                    {"id": "m2", "policyObjectRef": "pol-x"}
                ]
            }),
        )]);
        let err = run(&ws, &["pol-x".to_string()]).expect_err("halt");
        match err {
            CompileError::Halt { kind, .. } => {
                assert_eq!(kind, FailureKind::UnmappedInput);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn resolves_single_mapping_per_subject() {
        let ws = ws_from(vec![(
            "m.json",
            json!({
                "$wosStudioMapping": "1.0",
                "mappings": [{"id": "m1", "policyObjectRef": "pol-x", "mappingState": "mapsToWos"}]
            }),
        )]);
        let result = run(&ws, &["pol-x".to_string()]).expect("resolves");
        assert!(result.by_subject.contains_key("pol-x"));
    }
}
