// Rust guideline compliant 2026-02-21

//! End-to-end export conformance tests (Semantic Profile §§5–6).
//!
//! Each test loads a standalone export fixture from
//! `tests/fixtures/export/`, runs its 3-event workflow (Draft → Submitted
//! → Approved) through the conformance engine to produce a
//! runtime-stamped provenance log, feeds that log through the appropriate
//! exporter (`wos_export::prov_o`, `xes`, or `ocel`), and verifies
//! structural properties declared in the fixture.
//!
//! Export fixtures live in a dedicated subdirectory so the scanners that
//! walk `tests/fixtures/` for `ConformanceFixture` entries — `meta.rs`
//! and `kernel_conformance.rs` — can exclude them by location rather than
//! by filename prefix. A directory-based boundary is intent-revealing and
//! robust to future naming.
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
///
/// The kernel and event sequence are NOT in the fixture JSON — all three
/// export fixtures drive the identical Draft → Submitted → Approved
/// workflow, so `shared_workflow_kernel()` and `shared_workflow_events()`
/// define them once in Rust and every per-format fixture carries only
/// the assertions specific to its serializer. This eliminates ~40 lines
/// of duplicated JSON per fixture while keeping the per-format
/// contracts (id, description, export config, assertions) inspectable in
/// plain JSON.
#[derive(Debug, Deserialize)]
struct ExportFixture {
    id: String,
    #[allow(dead_code)]
    description: String,
    export: ExportSpec,
    assertions: Value,
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

/// Kernel shared by all export-conformance fixtures.
///
/// A two-transition lifecycle `draft → submitted → approved`, where the
/// `submitted.onEntry` sets `caseFile.status`. This produces the minimum
/// 3-record provenance log (two state transitions plus one case-state
/// mutation) that the three serializers exercise.
fn shared_workflow_kernel() -> Value {
    json!({
        "$wosKernel": "1.0",
        "url": "https://test.wos-spec.org/sp-export/shared-workflow",
        "actors": [
            { "id": "author", "type": "human" },
            { "id": "approver", "type": "human" }
        ],
        "caseFile": {
            "fields": {
                "status": { "type": "string", "default": "draft" }
            }
        },
        "lifecycle": {
            "initialState": "draft",
            "states": {
                "draft": {
                    "type": "atomic",
                    "transitions": [
                        { "event": "submit", "target": "submitted" }
                    ]
                },
                "submitted": {
                    "type": "atomic",
                    "onEntry": [
                        { "action": "setData", "path": "caseFile.status", "value": "submitted" }
                    ],
                    "transitions": [
                        { "event": "approve", "target": "approved" }
                    ]
                },
                "approved": {
                    "type": "final"
                }
            }
        }
    })
}

/// Event sequence shared by all export-conformance fixtures. Each event
/// has a distinct actor so exporters that deduplicate by actor_id
/// produce at least two `prov:Agent` nodes.
fn shared_workflow_events() -> Vec<Value> {
    vec![
        json!({ "event": "submit", "actor": "author" }),
        json!({ "event": "approve", "actor": "approver" }),
    ]
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
    let path = format!("{manifest}/tests/fixtures/export/{filename}");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read fixture '{path}': {error}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("fixture '{filename}' failed to parse: {error}"))
}

/// Minimum number of provenance records every export fixture MUST produce.
///
/// All three export fixtures drive the same Draft → Submitted → Approved
/// workflow (two transitions plus case-state mutations), so any run that
/// emits fewer than two records indicates the workflow did not execute as
/// specified — downstream assertions like `event_count == log.len()` would
/// be vacuously satisfied on a zero- or one-event log.
///
/// Enforcing this floor once in `run_workflow_to_stamped_log` (rather than
/// per-format inside `assert_prov_o` / `assert_xes` / `assert_ocel`) closes
/// the silent-pass hole across every export path at a single site.
const MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT: usize = 2;

/// Drive the fixture's event sequence through `WorkflowEngine` and return
/// the resulting stamped provenance records as a `ProvenanceLog`.
///
/// The kernel is written to a tempfile so `WorkflowEngine` can load it via
/// the standard fixture path; using the runtime path (not a hand-rolled log)
/// ensures timestamps come from `wos_runtime::stamp_provenance` the same way
/// a production instance would stamp them.
///
/// Post-condition: the returned log carries at least
/// `MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT` stamped records. Callers can
/// therefore trust `event_count == log.len()` assertions to be
/// non-vacuous — a workflow that silently produced zero events would panic
/// here, not slip through as a green test.
fn run_workflow_to_stamped_log(fixture: &ExportFixture) -> ProvenanceLog {
    // Write the shared kernel to a temp file so WorkflowEngine can read it
    // via the existing fixture path resolver. The TempDir is held alive for
    // the duration of the function so the path stays valid.
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let kernel_path = tempdir.path().join("kernel.json");
    std::fs::write(
        &kernel_path,
        serde_json::to_string_pretty(&shared_workflow_kernel())
            .expect("kernel must serialize"),
    )
    .expect("failed to write temp kernel");
    let kernel_path_str = kernel_path
        .to_str()
        .expect("temp kernel path is not valid UTF-8")
        .to_string();

    let conformance_fixture: ConformanceFixture = serde_json::from_value(json!({
        "id": fixture.id,
        "rule": "SP-EXPORT",
        "description": "export-conformance runtime driver",
        "documents": { "kernel": kernel_path_str },
        "event_sequence": shared_workflow_events(),
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

    // Silent-pass guard: every export fixture's workflow MUST emit at least
    // `MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT` records. See the constant's
    // doc comment for the rationale.
    assert!(
        log.len() >= MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT,
        "export fixture '{}' produced only {} provenance record(s); expected ≥ {} (the workflow is a 2-transition Draft → Submitted → Approved sequence). A log this short silently satisfies event_count == log.len() assertions and hides regressions.",
        fixture.id,
        log.len(),
        MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT
    );

    log
}

// ── PROV-O assertions ────────────────────────────────────────────

fn assert_prov_o(fixture: &ExportFixture, log: &ProvenanceLog, config: &ExportConfig) {
    // The non-vacuity floor (log.len() >= 2) is enforced once in
    // `run_workflow_to_stamped_log` via
    // `MIN_EXPECTED_PROVENANCE_RECORDS_PER_EXPORT`, so every per-format
    // `event_count == log.len()` assertion below is non-vacuous.
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

    // Every event block must carry the required string/date keys. Parse the
    // XML with quick_xml::Reader rather than splitting on the literal "<event>"
    // byte sequence: a substring split would silently pass with zero blocks if
    // the exporter ever emits "<event attr=\"…\">" or namespaced variants.
    let event_keys = extract_event_keys(&xml);
    assert_eq!(
        event_keys.len(),
        log.len(),
        "XES parsed <event> count ({}) must equal log length ({}) for fixture '{}'",
        event_keys.len(),
        log.len(),
        fixture.id
    );
    for required in assertion_string_array(assertions, "xes_event_required_keys") {
        for (index, keys) in event_keys.iter().enumerate() {
            assert!(
                keys.iter().any(|k| k == &required),
                "XES event[{index}] missing key '{required}' for fixture '{}' (keys present: {:?})",
                fixture.id,
                keys
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
    //
    // The assertion key reads as an invariant ("asserts … preserved as empty
    // string") rather than a unit-test-style name. The pure unit test in
    // `wos_export::ocel` — `preserves_empty_timestamp_as_string` — covers the
    // same property at the serializer level and stays named for that context.
    if assertion_bool(assertions, "asserts_empty_timestamp_preserved_as_empty_string_in_ocel") {
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

/// For each `<event>...</event>` block in the XML, collect the `key` attribute
/// values of its child `<string>` / `<date>` elements. Using the parser (not
/// string splitting) ensures correctness against any future attribute variants
/// on the `<event>` open tag.
fn extract_event_keys(xml: &str) -> Vec<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut events: Vec<Vec<String>> = Vec::new();
    let mut current: Option<Vec<String>> = None;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Eof) => break,
            Ok(XmlEvent::Start(element)) if element.name().as_ref() == b"event" => {
                current = Some(Vec::new());
            }
            Ok(XmlEvent::End(element)) if element.name().as_ref() == b"event" => {
                if let Some(keys) = current.take() {
                    events.push(keys);
                }
            }
            Ok(XmlEvent::Empty(element)) | Ok(XmlEvent::Start(element)) => {
                if let Some(keys) = current.as_mut() {
                    for attr in element.attributes().flatten() {
                        if attr.key.as_ref() == b"key" {
                            keys.push(String::from_utf8_lossy(&attr.value).into_owned());
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(error) => panic!(
                "XES extract_event_keys hit a parse error at byte {}: {error:?}",
                reader.buffer_position()
            ),
        }
        buffer.clear();
    }
    events
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

// ── extract_event_keys regression tests ─────────────────────────
//
// A prior iteration of this helper used `xml.split("<event>")` to segment
// events, which silently produced zero blocks for `<event attr="…">` or
// any namespaced variant of the open tag (quick-xml pretty-prints with
// indentation, not padding, so the literal byte sequence wasn't always
// present). The parser-based implementation below is robust against
// attributes on the open tag. These tests pin that contract.

/// `<event>` carries attributes on its open tag: the parser-based
/// implementation must still emit one block with the child `<string>`
/// key. A literal-string split on `"<event>"` would see zero matches and
/// return `vec![]`, so a regressed implementation would turn a truthful
/// test into vacuous pass/fail.
#[test]
fn extract_event_keys_handles_event_with_open_tag_attributes() {
    let xml = r#"<log><event id="e1"><string key="concept:name" value="submit"/></event></log>"#;

    let events = extract_event_keys(xml);

    assert_eq!(
        events,
        vec![vec!["concept:name".to_string()]],
        "event with attribute on open tag must still yield one block with its child key"
    );
}

/// An `<event></event>` block with no child `<string>` / `<date>`
/// elements must yield a single empty key list (one block, zero keys) —
/// not be dropped. The block itself is still a real event.
#[test]
fn extract_event_keys_yields_empty_list_for_childless_event() {
    let xml = "<log><event></event></log>";

    let events = extract_event_keys(xml);

    assert_eq!(
        events,
        vec![Vec::<String>::new()],
        "childless <event>...</event> must yield one block with zero keys, not be dropped"
    );
}

/// A self-closing `<event/>` is not a shape the XES exporter currently
/// emits, but the helper must not crash on it. It is a shape quick-xml
/// reports as `Empty(event)`, which the current arm order treats as
/// "child element of the currently-open event" — but no event is open
/// at the top level, so `current` is `None` and the attribute loop is a
/// no-op. Net effect: zero events surfaced. This test pins that
/// behavior so a future refactor cannot accidentally crash on the
/// unusual shape.
#[test]
fn extract_event_keys_does_not_crash_on_self_closing_event() {
    let xml = "<log><event/></log>";

    let events = extract_event_keys(xml);

    assert!(
        events.is_empty(),
        "self-closing <event/> at top level must not surface any event block, got {events:?}"
    );
}
