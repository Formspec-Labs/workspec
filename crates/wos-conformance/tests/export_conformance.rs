// Rust guideline compliant 2026-02-21

//! End-to-end export conformance tests (Semantic Profile §§5–6).
//!
//! Each test loads a standalone export fixture from `tests/fixtures/`, runs
//! its 3-event workflow (Draft → Submitted → Approved) through the
//! conformance engine to produce a runtime-stamped provenance log, feeds
//! that log through the appropriate exporter (`wos_export::prov_o`, `xes`,
//! or `ocel`), and verifies structural properties declared in the fixture.
//!
//! # Fixture envelope
//!
//! Export fixtures use a shape distinct from `ConformanceFixture`: the
//! existing envelope targets state-transition and provenance-match
//! assertions, but structural export checks (XML parse success, JSON key
//! presence, graph node counts) don't fit there. Rather than bolt unrelated
//! assertion kinds onto the runtime-focused envelope, these tests carry
//! their own `{id, description, kernel, events, export, assertions}` shape
//! and reuse `WorkflowEngine` as the runtime machinery only.
//!
//! # Why we don't snapshot
//!
//! XES indentation, PROV-O graph ordering, and OCEL map insertion order can
//! all shift harmlessly across exporter refactors. Structural assertions
//! (counts, key presence, parse success) catch behavior changes without
//! being brittle to cosmetic changes.

use std::collections::BTreeSet;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use serde::Deserialize;
use serde_json::{Value, json};

use wos_conformance::{ConformanceFixture, WorkflowEngine};
use wos_core::provenance::{ProvenanceLog, ProvenanceRecord};
use wos_export::{ExportConfig, ocel, prov_o, xes};

// ── Fixture shape ────────────────────────────────────────────────

/// Export-conformance fixture envelope.
///
/// Deliberately decoupled from `ConformanceFixture`: the runtime fixture
/// envelope is tuned for state-transition and provenance-match assertions,
/// not structural checks against serialized export output.
#[derive(Debug, Deserialize)]
struct ExportFixture {
    id: String,
    #[allow(dead_code)]
    description: String,
    kernel: Value,
    events: Vec<FixtureEvent>,
    export: ExportSpec,
    assertions: Value,
}

#[derive(Debug, Deserialize)]
struct FixtureEvent {
    event: String,
    actor: String,
}

#[derive(Debug, Deserialize)]
struct ExportSpec {
    format: String,
    config: ExportConfigSpec,
}

#[derive(Debug, Deserialize)]
struct ExportConfigSpec {
    provenance_namespace: String,
    instance_id: String,
}

// ── Test entry points ────────────────────────────────────────────

#[test]
fn sp_export_001_prov_o_graph_shape() {
    run_export_fixture("sp-export-prov-o.json");
}

#[test]
fn sp_export_002_xes_xml_structure() {
    run_export_fixture("sp-export-xes.json");
}

#[test]
fn sp_export_003_ocel_json_structure() {
    run_export_fixture("sp-export-ocel.json");
}

// ── Harness ──────────────────────────────────────────────────────

/// Load a fixture, run its workflow, export, and apply the declared
/// structural assertions.
fn run_export_fixture(fixture_filename: &str) {
    let fixture = load_fixture(fixture_filename);
    let log = run_workflow_to_stamped_log(&fixture);
    let config = ExportConfig {
        provenance_namespace: fixture.export.config.provenance_namespace.clone(),
        instance_id: fixture.export.config.instance_id.clone(),
    };

    match fixture.export.format.as_str() {
        "prov_o" => assert_prov_o(&fixture, &log, &config),
        "xes" => assert_xes(&fixture, &log, &config),
        "ocel" => assert_ocel(&fixture, &log, &config),
        other => panic!("unknown export format in fixture {}: {other}", fixture.id),
    }
}

fn load_fixture(filename: &str) -> ExportFixture {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest}/tests/fixtures/{filename}");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read fixture '{path}': {error}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("fixture '{filename}' failed to parse: {error}"))
}

/// Drive the fixture's event sequence through `WorkflowEngine` and return
/// the resulting stamped provenance records as a `ProvenanceLog`.
///
/// The kernel is written to a tempfile so `WorkflowEngine` can load it via
/// the standard fixture path; using the runtime path (not a hand-rolled log)
/// ensures timestamps come from `wos_runtime::stamp_provenance` the same way
/// a production instance would stamp them.
fn run_workflow_to_stamped_log(fixture: &ExportFixture) -> ProvenanceLog {
    // Write the inline kernel to a temp file so WorkflowEngine can read it
    // via the existing fixture path resolver. The TempDir is held alive for
    // the duration of the function so the path stays valid.
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let kernel_path = tempdir.path().join("kernel.json");
    std::fs::write(
        &kernel_path,
        serde_json::to_string_pretty(&fixture.kernel).expect("kernel must serialize"),
    )
    .expect("failed to write temp kernel");
    let kernel_path_str = kernel_path
        .to_str()
        .expect("temp kernel path is not valid UTF-8")
        .to_string();

    let event_sequence: Vec<Value> = fixture
        .events
        .iter()
        .map(|entry| json!({ "event": entry.event, "actor": entry.actor }))
        .collect();

    let conformance_fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": fixture.id,
        "rule": "SP-EXPORT",
        "description": "export-conformance runtime driver",
        "documents": { "kernel": kernel_path_str },
        "event_sequence": event_sequence,
        "expected_transitions": []
    }))
    .expect("could not build ConformanceFixture");

    let mut engine = WorkflowEngine::new(&conformance_fixture)
        .unwrap_or_else(|error| panic!("engine init failed for '{}': {error}", fixture.id));
    let result = engine
        .execute(&conformance_fixture)
        .unwrap_or_else(|error| panic!("engine execute failed for '{}': {error}", fixture.id));

    // Every runtime-sourced record must already be stamped — the runtime
    // calls stamp_provenance on the append path. Guarding here catches any
    // future regression that bypasses stamping.
    for record in &result.provenance {
        assert!(
            !record.timestamp.is_empty(),
            "runtime must stamp every provenance record; got unstamped: {record:?}"
        );
    }

    let mut log = ProvenanceLog::default();
    for record in result.provenance {
        log.push(record);
    }
    log
}

// ── PROV-O assertions ────────────────────────────────────────────

fn assert_prov_o(fixture: &ExportFixture, log: &ProvenanceLog, config: &ExportConfig) {
    assert!(
        log.len() >= 2,
        "PROV-O fixture '{}' expected at least 2 provenance records (one per transition), got {}",
        fixture.id,
        log.len()
    );

    let document = prov_o::export(log, config);
    let serialized = serde_json::to_value(&document).expect("PROV-O document must serialize");
    let graph = serialized["@graph"]
        .as_array()
        .expect("PROV-O @graph must be an array");

    let assertions = &fixture.assertions;

    // Graph node count.
    let min_nodes = assertion_u64(assertions, "prov_o_graph_min_nodes");
    assert!(
        graph.len() as u64 >= min_nodes,
        "PROV-O graph node count ({}) must be ≥ {min_nodes} for fixture '{}'",
        graph.len(),
        fixture.id
    );

    // @context prefixes.
    let context = &serialized["@context"];
    for prefix in assertion_string_array(assertions, "prov_o_context_prefixes") {
        assert!(
            context.get(&prefix).and_then(Value::as_str).is_some(),
            "PROV-O @context missing prefix '{prefix}' for fixture '{}'; context was: {context}",
            fixture.id
        );
    }

    // Activity node requirements.
    let required_fields = assertion_string_array(assertions, "prov_o_activity_required_fields");
    let activity_type = assertion_string(assertions, "prov_o_activity_type_value");
    let stamped_must_have_at_time =
        assertion_bool(assertions, "prov_o_stamped_activities_have_at_time");

    let activities: Vec<&Value> = graph
        .iter()
        .filter(|node| node.get("@type") == Some(&Value::String(activity_type.clone())))
        .collect();
    assert!(
        !activities.is_empty(),
        "PROV-O graph contains no '{activity_type}' nodes for fixture '{}'",
        fixture.id
    );

    for (index, activity) in activities.iter().enumerate() {
        for field in &required_fields {
            assert!(
                activity.get(field).is_some(),
                "PROV-O activity[{index}] missing '{field}' for fixture '{}': {activity}",
                fixture.id
            );
        }
        if stamped_must_have_at_time {
            assert!(
                activity
                    .get("prov:atTime")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty()),
                "PROV-O activity[{index}] missing non-empty prov:atTime for stamped record (fixture '{}'): {activity}",
                fixture.id
            );
        }
    }

    // Agent node count — PROV-O deduplicates by actor_id. With both an
    // `author` and `approver` driving transitions we expect at least one
    // `prov:Agent` node to be present.
    let min_agents = assertion_u64(assertions, "prov_o_min_agents");
    let agent_count = graph
        .iter()
        .filter(|node| node.get("@type") == Some(&Value::String("prov:Agent".to_string())))
        .count();
    assert!(
        agent_count as u64 >= min_agents,
        "PROV-O agent count ({agent_count}) must be ≥ {min_agents} for fixture '{}'",
        fixture.id
    );
}

// ── XES assertions ───────────────────────────────────────────────

fn assert_xes(fixture: &ExportFixture, log: &ProvenanceLog, config: &ExportConfig) {
    let xml = xes::export(log, config);
    let assertions = &fixture.assertions;

    // Parse the XML end-to-end. Any error is a structural failure.
    if assertion_bool(assertions, "xes_parses_as_xml") {
        parse_xml_or_panic(&xml, &fixture.id);
    }

    // Count elements by name.
    let trace_count = count_xml_elements(&xml, b"trace");
    let expected_traces = assertion_u64(assertions, "xes_trace_count");
    assert_eq!(
        trace_count as u64, expected_traces,
        "XES <trace> count ({trace_count}) must equal {expected_traces} for fixture '{}'",
        fixture.id
    );

    if assertion_bool(assertions, "xes_event_count_matches_log") {
        let event_count = count_xml_elements(&xml, b"event");
        assert_eq!(
            event_count,
            log.len(),
            "XES <event> count ({event_count}) must equal provenance record count ({}) for fixture '{}'",
            log.len(),
            fixture.id
        );
    }

    // Every event block must carry the required string/date keys. We split
    // on <event> boundaries rather than re-parse because the assertions are
    // per-event-block; parse validity is already covered above.
    let event_blocks: Vec<&str> = xml.split("<event>").skip(1).collect();
    assert_eq!(
        event_blocks.len(),
        log.len(),
        "XES event block split count ({}) must equal log length ({}) for fixture '{}'",
        event_blocks.len(),
        log.len(),
        fixture.id
    );
    for key in assertion_string_array(assertions, "xes_event_required_keys") {
        let needle = format!(r#"key="{key}""#);
        for (index, block) in event_blocks.iter().enumerate() {
            assert!(
                block.contains(&needle),
                "XES event[{index}] missing key '{key}' for fixture '{}': {block}",
                fixture.id
            );
        }
    }

    // §6.3 lifecycle choice.
    if assertion_bool(assertions, "xes_uses_custom_lifecycle_state") {
        assert!(
            xml.contains(r#"key="wos:lifecycleState""#),
            "XES output must emit wos:lifecycleState (custom WOS attribute) for fixture '{}'",
            fixture.id
        );
    }
    if assertion_bool(assertions, "xes_rejects_standard_lifecycle_transition") {
        assert!(
            !xml.contains("lifecycle:transition"),
            "XES output must NOT emit lifecycle:transition — WOS states are not XES lifecycle vocab (fixture '{}')",
            fixture.id
        );
    }
}

// ── OCEL assertions ──────────────────────────────────────────────

fn assert_ocel(fixture: &ExportFixture, log: &ProvenanceLog, config: &ExportConfig) {
    let document = ocel::export(log, config);
    let assertions = &fixture.assertions;

    // Top-level keys.
    let object = document
        .as_object()
        .unwrap_or_else(|| panic!("OCEL top-level must be an object for fixture '{}'", fixture.id));
    let expected_keys: BTreeSet<String> =
        assertion_string_array(assertions, "ocel_top_level_keys")
            .into_iter()
            .collect();
    let actual_keys: BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(
        actual_keys, expected_keys,
        "OCEL top-level keys mismatch for fixture '{}'",
        fixture.id
    );

    // Event count.
    let events = document["events"]
        .as_array()
        .unwrap_or_else(|| panic!("OCEL 'events' must be an array for fixture '{}'", fixture.id));
    if assertion_bool(assertions, "ocel_event_count_matches_log") {
        assert_eq!(
            events.len(),
            log.len(),
            "OCEL event count ({}) must equal provenance record count ({}) for fixture '{}'",
            events.len(),
            log.len(),
            fixture.id
        );
    }

    // Every event relates to the instance object.
    if assertion_bool(assertions, "ocel_every_event_relates_to_instance") {
        let instance_id = &config.instance_id;
        for (index, event) in events.iter().enumerate() {
            let relationships = event["relationships"].as_array().unwrap_or_else(|| {
                panic!(
                    "OCEL event[{index}] missing relationships array for fixture '{}': {event}",
                    fixture.id
                )
            });
            let has_instance_relationship = relationships.iter().any(|relationship| {
                relationship.get("objectId").and_then(Value::as_str) == Some(instance_id)
            });
            assert!(
                has_instance_relationship,
                "OCEL event[{index}] does not relate to instance object '{instance_id}' (fixture '{}'): {event}",
                fixture.id
            );
        }
    }

    // Empty-timestamp semantics. The runtime stamps every record it produces,
    // so the workflow log itself cannot exercise this path. We verify the
    // exporter behaviour by feeding an auxiliary, hand-built log containing
    // one unstamped record — the fixture asserts that the exporter's
    // contract (time: "" rather than omission) still holds.
    if assertion_bool(assertions, "ocel_preserves_empty_timestamp_as_string") {
        let mut unstamped_log = ProvenanceLog::default();
        unstamped_log.push(ProvenanceRecord::state_transition(
            "draft",
            "submitted",
            "submit",
            None,
        ));
        let unstamped_document = ocel::export(&unstamped_log, config);
        let unstamped_events = unstamped_document["events"]
            .as_array()
            .expect("OCEL events array");
        assert_eq!(
            unstamped_events.len(),
            1,
            "unstamped-log OCEL export must produce exactly one event (fixture '{}')",
            fixture.id
        );
        let time_field = &unstamped_events[0]["time"];
        assert_eq!(
            time_field,
            &Value::String(String::new()),
            "OCEL must preserve empty timestamp as \"\" rather than omit it (fixture '{}'): {}",
            fixture.id,
            unstamped_events[0]
        );
    }
}

// ── Shared helpers ───────────────────────────────────────────────

/// Read the entire XML stream to confirm it parses cleanly. Any error —
/// malformed tags, unclosed elements, bad attribute quoting — panics with a
/// buffer position so regressions are easy to localize.
fn parse_xml_or_panic(xml: &str, fixture_id: &str) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!(
                "XES output failed to parse as XML at byte {} for fixture '{fixture_id}': {error:?}",
                reader.buffer_position()
            ),
        }
        buffer.clear();
    }
}

/// Count occurrences of a named element (Start or Empty) by parsing the XML.
/// Using the parser — not string search — avoids false positives from
/// occurrences inside attribute values.
fn count_xml_elements(xml: &str, tag: &[u8]) -> usize {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut count = 0usize;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Eof) => break,
            Ok(XmlEvent::Start(element)) | Ok(XmlEvent::Empty(element)) => {
                if element.name().as_ref() == tag {
                    count += 1;
                }
            }
            Ok(_) => {}
            Err(error) => panic!(
                "XES count_xml_elements hit a parse error at byte {}: {error:?}",
                reader.buffer_position()
            ),
        }
        buffer.clear();
    }
    count
}

fn assertion_u64(assertions: &Value, key: &str) -> u64 {
    assertions
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("missing u64 assertion '{key}' in fixture"))
}

fn assertion_bool(assertions: &Value, key: &str) -> bool {
    assertions
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("missing bool assertion '{key}' in fixture"))
}

fn assertion_string(assertions: &Value, key: &str) -> String {
    assertions
        .get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| panic!("missing string assertion '{key}' in fixture"))
}

fn assertion_string_array(assertions: &Value, key: &str) -> Vec<String> {
    assertions
        .get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("missing string-array assertion '{key}' in fixture"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("assertion '{key}' must be an array of strings"))
                .to_string()
        })
        .collect()
}
