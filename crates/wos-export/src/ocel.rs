// Rust guideline compliant 2026-04-16

//! OCEL 2.0 JSON serializer (WOS Semantic Profile §6.4).
//!
//! Serializes a [`ProvenanceLog`] into an Object-Centric Event Log shaped per
//! the OCEL 2.0 JSON specification.
//!
//! # Scope filter (§6.5)
//!
//! Higher-tier records (Reasoning, Counterfactual, Narrative) are excluded
//! from the default export. The `wf-instance` object is emitted regardless
//! of filter outcome — it anchors the log even when no events survive.
//!
//! # Object modelling gap
//!
//! This implementation models only the workflow instance itself as an OCEL
//! Object (type `wf-instance`). Every event relates back to that single
//! object with qualifier `relates-to`. The spec (§6.4) mandates that each
//! **Case File Item** is also an OCEL Object, with E2O relationships
//! connecting mutating events to the items they touch and O2O relationships
//! representing cross-references between items. That richer shape is an
//! upstream-architectural gap: `ProvenanceRecord` does not today carry the
//! per-record read/write sets that E2O materialization requires. The gap is
//! tracked in a follow-up ticket and is NOT addressable from inside this
//! crate.
//!
//! # Empty timestamp handling
//!
//! Unstamped records (see [`ProvenanceRecord::timestamp`] docs) emit `time: ""`
//! verbatim. OCEL 2.0 requires `time` on every event, so omitting the field
//! would produce an invalid document; emitting an empty string surfaces the
//! missed stamping site to downstream consumers. This is consistent with the
//! push-stamped design's "surface, don't paper over" philosophy.

use serde_json::{Map, Value, json};

use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};

use crate::{ExportConfig, camel_case_record_kind};

/// Serialize a provenance log as an OCEL 2.0 JSON document (§6.4).
///
/// The returned value is shaped exactly per the OCEL 2.0 spec: four top-level
/// arrays (`objectTypes`, `eventTypes`, `objects`, `events`). Every event
/// relates to a single `wf-instance` object whose id is `config.instance_id`;
/// per-case-file-item E2O relationships are deferred — see module docs for
/// the object modelling gap.
#[must_use]
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> Value {
    // §6.5 scope filter: Facts-tier records only. `None` is Facts for
    // backward compatibility with pre-extension runtimes.
    let facts_records: Vec<&ProvenanceRecord> = log
        .records()
        .iter()
        .filter(|record| is_facts_tier(record))
        .collect();

    let events: Vec<Value> = facts_records
        .iter()
        .enumerate()
        .map(|(index, record)| event_node(index, record, &config.instance_id))
        .collect();

    json!({
        "objectTypes": object_types(),
        "eventTypes": event_types(&facts_records),
        "objects": objects(config, &facts_records),
        "events": events,
    })
}

/// §6.5 scope predicate. Shared-semantics copy of the PROV-O / XES filter.
fn is_facts_tier(record: &ProvenanceRecord) -> bool {
    matches!(record.audit_layer.as_deref(), None | Some("facts"))
}

/// Fixed `objectTypes` array. OCEL 2.0 requires typed objects; this phase
/// exposes exactly one type (`wf-instance`) since only the instance itself is
/// modelled as an object.
fn object_types() -> Value {
    json!([{
        "name": "wf-instance",
        "attributes": [
            { "name": "instanceId", "type": "string" }
        ],
    }])
}

/// Deduplicate `record_kind` occurrences into an OCEL `eventTypes` array.
/// Order is stable: first-seen wins, matching the activity emission order the
/// PROV-O exporter uses so snapshot diffs stay clean for Task 5 fixtures.
fn event_types(records: &[&ProvenanceRecord]) -> Value {
    let mut seen: Vec<String> = Vec::new();
    for record in records {
        let name = camel_case_record_kind(record);
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen.into_iter()
        .map(|name| json!({ "name": name, "attributes": [] }))
        .collect()
}

/// Emit the single `wf-instance` object. Its `instanceId` attribute carries
/// the earliest non-empty record timestamp (falling back to `""` when the log
/// contains no stamped records) so downstream OCEL tools can anchor the
/// object in the event timeline.
fn objects(config: &ExportConfig, records: &[&ProvenanceRecord]) -> Value {
    // Lexicographic `.min()` over timestamps is only correct when every
    // timestamp uses the same UTC form (`...Z`). The WOS runtime emits
    // strict RFC 3339 UTC via `wos_runtime::stamp_provenance` (see
    // `crates/wos-runtime/src/lib.rs`), so this assumption holds for
    // runtime-produced logs. If a future source supplies offset-bearing
    // timestamps (`...+01:00`), normalize to UTC before comparing — a
    // lexicographic min over mixed offsets can silently pick the wrong
    // record.
    let earliest = records
        .iter()
        .map(|record| record.timestamp.as_str())
        .filter(|timestamp| !timestamp.is_empty())
        .min()
        .unwrap_or("");

    json!([{
        "id": config.instance_id,
        "type": "wf-instance",
        "attributes": [{
            "name": "instanceId",
            "time": earliest,
            "value": config.instance_id,
        }],
    }])
}

/// Emit a single OCEL event for a record at position `index`.
fn event_node(index: usize, record: &ProvenanceRecord, instance_id: &str) -> Value {
    let mut node = Map::new();
    node.insert("id".into(), Value::String(format!("e-{index}")));
    node.insert("type".into(), Value::String(camel_case_record_kind(record)));
    // `time` is emitted verbatim even when empty — see module docs for the
    // rationale. OCEL requires the field to be present on every event.
    node.insert("time".into(), Value::String(record.timestamp.clone()));
    node.insert("attributes".into(), Value::Array(event_attributes(record)));
    node.insert(
        "relationships".into(),
        json!([{ "objectId": instance_id, "qualifier": "relates-to" }]),
    );
    Value::Object(node)
}

/// Collect the optional `ProvenanceRecord` fields into OCEL event attributes.
/// Only non-`None` / non-empty fields are included; `record_kind` and
/// `timestamp` are carried on the event envelope and are intentionally
/// excluded here.
fn event_attributes(record: &ProvenanceRecord) -> Vec<Value> {
    let mut attributes = Vec::new();
    if let Some(actor_id) = record.actor_id.as_deref() {
        attributes.push(json!({ "name": "actorId", "value": actor_id }));
    }
    if let Some(actor_type) = record.actor_type.as_deref() {
        attributes.push(json!({ "name": "actorType", "value": actor_type }));
    }
    if let Some(from_state) = record.from_state.as_deref() {
        attributes.push(json!({ "name": "fromState", "value": from_state }));
    }
    if let Some(to_state) = record.to_state.as_deref() {
        attributes.push(json!({ "name": "toState", "value": to_state }));
    }
    if let Some(lifecycle_state) = record.lifecycle_state.as_deref() {
        attributes.push(json!({ "name": "lifecycleState", "value": lifecycle_state }));
    }
    if let Some(version) = record.definition_version.as_deref() {
        attributes.push(json!({ "name": "definitionVersion", "value": version }));
    }
    if let Some(event) = record.event.as_deref() {
        attributes.push(json!({ "name": "event", "value": event }));
    }
    if let Some(data) = record.data.as_ref() {
        // Emit `data` as a single structured attribute rather than flattening;
        // the JSON subtree shape is domain-specific and OCEL consumers can
        // introspect it as nested JSON.
        attributes.push(json!({ "name": "data", "value": data.clone() }));
    }
    if !record.inputs.is_empty() {
        // Emit as a JSON array so OCEL consumers see the structured list
        // rather than a delimited string (cf. the XES joined representation).
        let inputs: Vec<Value> = record.inputs.iter().map(|s| Value::String(s.clone())).collect();
        attributes.push(json!({ "name": "inputs", "value": Value::Array(inputs) }));
    }
    if !record.outputs.is_empty() {
        let outputs: Vec<Value> = record.outputs.iter().map(|s| Value::String(s.clone())).collect();
        attributes.push(json!({ "name": "outputs", "value": Value::Array(outputs) }));
    }
    if let Some(digest) = record.input_digest.as_deref() {
        attributes.push(json!({ "name": "inputDigest", "value": digest }));
    }
    if let Some(digest) = record.output_digest.as_deref() {
        attributes.push(json!({ "name": "outputDigest", "value": digest }));
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::provenance::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            instance_id: "instance-abc".to_string(),
        }
    }

    fn stamped(kind: ProvenanceKind, timestamp: &str) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("user-1"));
        record.record_kind = kind;
        record.timestamp = timestamp.to_string();
        record
    }

    #[test]
    fn exports_log_has_required_top_level_keys() {
        let log = ProvenanceLog::default();
        let document = export(&log, &config());

        let object = document.as_object().expect("top-level must be a JSON object");
        let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["eventTypes", "events", "objectTypes", "objects"]);
    }

    #[test]
    fn event_count_equals_record_count() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));
        log.push(stamped(ProvenanceKind::OnExit, "2026-01-01T00:00:02Z"));

        let document = export(&log, &config());

        assert_eq!(document["events"].as_array().expect("events array").len(), 3);
    }

    #[test]
    fn every_event_has_id_type_time_relationships() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event["id"], format!("e-{index}"));
            assert!(event["type"].is_string(), "type must be string on event {index}");
            assert!(event["time"].is_string(), "time must be string on event {index}");
            assert!(
                event["relationships"].is_array(),
                "relationships must be array on event {index}"
            );
        }
    }

    #[test]
    fn every_event_relates_to_instance_object() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));
        log.push(stamped(ProvenanceKind::TimerFired, "2026-01-01T00:00:10Z"));

        let document = export(&log, &config());

        let expected = json!({ "objectId": "instance-abc", "qualifier": "relates-to" });
        for event in document["events"].as_array().expect("events array") {
            let relationships = event["relationships"].as_array().expect("relationships");
            assert!(
                relationships.iter().any(|r| r == &expected),
                "event missing instance relationship: {event}"
            );
        }
    }

    #[test]
    fn event_types_deduplicated_by_kind() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));
        }
        for _ in 0..2 {
            log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));
        }

        let document = export(&log, &config());

        let event_types = document["eventTypes"].as_array().expect("eventTypes array");
        assert_eq!(event_types.len(), 2);

        let mut names: Vec<_> = event_types
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["onEntry", "stateTransition"]);
    }

    #[test]
    fn preserves_empty_timestamp_as_empty_string() {
        let mut log = ProvenanceLog::default();
        // Unstamped record — constructors leave `timestamp` empty; the runtime
        // would normally push-stamp it, but tests may bypass that path.
        log.push(ProvenanceRecord::state_transition("a", "b", "ev", None));

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["time"], "", "empty timestamp must round-trip as empty string");

        // When every record is unstamped, the instance object's `instanceId`
        // attribute falls back to an empty `time` (documented at module level).
        let objects = document["objects"].as_array().expect("objects array");
        let instance_attrs = objects[0]["attributes"].as_array().expect("attributes array");
        assert_eq!(
            instance_attrs[0]["time"], "",
            "all-unstamped log must produce empty instanceId attribute time"
        );
    }

    #[test]
    fn single_instance_object_emitted() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));

        let document = export(&log, &config());

        let objects = document["objects"].as_array().expect("objects array");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0]["id"], "instance-abc");
        assert_eq!(objects[0]["type"], "wf-instance");

        let object_types = document["objectTypes"].as_array().expect("objectTypes array");
        assert_eq!(object_types.len(), 1);
        assert_eq!(object_types[0]["name"], "wf-instance");
    }

    #[test]
    fn emits_sp_section_6_3_attributes_when_populated() {
        // §6.3 symmetry with XES: actorType, lifecycleState, definitionVersion,
        // inputs/outputs as JSON arrays, and the two digests.
        let mut log = ProvenanceLog::default();
        let mut record = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        record.actor_type = Some("human".into());
        record.lifecycle_state = Some("draft".into());
        record.definition_version = Some("3.2.1".into());
        record.inputs = vec!["case/1".into(), "case/2".into()];
        record.outputs = vec!["case/1#state".into()];
        record.input_digest = Some("sha256:aaaa".into());
        record.output_digest = Some("sha256:bbbb".into());
        log.push(record);

        let document = export(&log, &config());

        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");

        let find = |name: &str| -> Option<&Value> {
            attributes
                .iter()
                .find(|attr| attr["name"] == name)
                .map(|attr| &attr["value"])
        };

        assert_eq!(find("actorType"), Some(&Value::String("human".into())));
        assert_eq!(find("lifecycleState"), Some(&Value::String("draft".into())));
        assert_eq!(
            find("definitionVersion"),
            Some(&Value::String("3.2.1".into()))
        );
        assert_eq!(
            find("inputs"),
            Some(&json!(["case/1", "case/2"])),
            "inputs must be emitted as a JSON array",
        );
        assert_eq!(find("outputs"), Some(&json!(["case/1#state"])));
        assert_eq!(find("inputDigest"), Some(&Value::String("sha256:aaaa".into())));
        assert_eq!(find("outputDigest"), Some(&Value::String("sha256:bbbb".into())));
    }

    #[test]
    fn omits_optional_attributes_when_absent() {
        // Baseline record has actor_id + from/to/event only; the new fields
        // must be absent from the attribute list.
        let mut log = ProvenanceLog::default();
        log.push(stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z"));

        let document = export(&log, &config());
        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");
        let names: Vec<&str> = attributes
            .iter()
            .map(|attr| attr["name"].as_str().expect("name"))
            .collect();

        for omitted in [
            "actorType",
            "lifecycleState",
            "definitionVersion",
            "inputs",
            "outputs",
            "inputDigest",
            "outputDigest",
        ] {
            assert!(
                !names.contains(&omitted),
                "attribute {omitted} must not appear when field is unset: {names:?}"
            );
        }
    }

    #[test]
    fn filters_non_facts_tier_records_per_section_6_5() {
        // §6.5: Narrative-tier records excluded from default OCEL export.
        let mut log = ProvenanceLog::default();

        let mut facts = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        facts.audit_layer = Some("facts".into());
        log.push(facts);

        let mut narrative = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:01Z");
        narrative.audit_layer = Some("narrative".into());
        narrative.actor_id = Some("narrator".into());
        log.push(narrative);

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1, "narrative record must be excluded");

        // The narrator should not appear as an event actor attribute.
        let narrator_present = events.iter().any(|event| {
            event["attributes"]
                .as_array()
                .map(|attrs| {
                    attrs
                        .iter()
                        .any(|attr| attr["name"] == "actorId" && attr["value"] == "narrator")
                })
                .unwrap_or(false)
        });
        assert!(!narrator_present, "narrator must be filtered: {events:?}");
    }
}
