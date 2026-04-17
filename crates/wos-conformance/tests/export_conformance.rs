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
        // `version` populates ProvenanceRecord.definition_version via the
        // runtime's `populate_provenance_record_fields` pass; export fixtures
        // assert PROV-O `wos:definitionVersion` and XES trace-level
        // `wos:definitionVersion` on the resulting records.
        "version": "1.0.0",
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

/// §6.5 Facts-tier filter: higher-tier records (narrative, reasoning,
/// counterfactual) are excluded from the default PROV-O / XES / OCEL
/// surfaces. The three fixture workflows only emit Facts-tier records so
/// they cannot exercise this path; this test constructs a synthetic log
/// with one Facts record and one Narrative record, stamps them through
/// the runtime helpers, and asserts each exporter surfaces only the
/// Facts record.
///
/// The Narrative record is built by taking a fresh `StateTransition`
/// constructor emission (post-populate it carries `audit_layer="facts"`)
/// and explicitly overwriting `audit_layer = Some("narrative")`. §6.5's
/// filter looks at `audit_layer` — not `record_kind` — so this is the
/// minimum surgery required to exercise it without adding a dedicated
/// `ProvenanceKind::NarrativeTierRecorded` construction path.
#[test]
fn sp_export_004_facts_tier_filter() {
    use wos_core::model::kernel::KernelDocument;
    use wos_runtime::{populate_provenance_record_fields, stamp_provenance};

    // Minimal kernel so `populate_provenance_record_fields` can resolve the
    // Human actor and propagate the definition version. Built via JSON parse
    // to avoid depending on manual struct construction (KernelDocument has
    // many required fields and no Default impl).
    let kernel: KernelDocument = serde_json::from_value(json!({
        "$wosKernel": "1.0",
        "url": "urn:wos-conformance:sp-export-004",
        "version": "1.0.0",
        "actors": [
            { "id": "actor-1", "type": "human" }
        ],
        "caseFile": { "fields": {} },
        "lifecycle": {
            "initialState": "draft",
            "states": {
                "draft": { "type": "atomic" }
            }
        }
    }))
    .expect("synthetic kernel must parse");

    // Two records sharing the same record_kind — §6.5's filter keys off
    // `audit_layer`, not `record_kind`, so swapping the tier discriminator
    // is sufficient to exercise the exclusion path. `populate_...` only
    // fills when `audit_layer` is `None`, so we overwrite AFTER the
    // populate pass to simulate what a Layer-1 tier injector will do once
    // the `NarrativeTierRecorded` construction path lands.
    let mut records = vec![
        ProvenanceRecord::state_transition("draft", "submitted", "submit", Some("actor-1")),
        ProvenanceRecord::state_transition("submitted", "approved", "approve", Some("actor-1")),
    ];
    populate_provenance_record_fields(&mut records, &kernel, "1.0.0");
    stamp_provenance(&mut records, "2026-04-16T00:00:00Z");
    // Pin the populator's idempotency invariant: the tier override below only
    // works because `populate_provenance_record_fields` fills `audit_layer`
    // from `None` and leaves a non-None value untouched. If that ever changes,
    // this assertion makes the silent failure loud — otherwise a future
    // populator that re-computed `audit_layer` would overwrite our override
    // and the "narrative excluded" assertion would pass for the wrong reason.
    debug_assert_eq!(
        records[1].audit_layer.as_deref(),
        Some("facts"),
        "populate should have filled audit_layer to facts before we override"
    );
    records[1].audit_layer = Some("narrative".to_string());

    let mut log = ProvenanceLog::default();
    for record in records {
        log.push(record);
    }

    let config = ExportConfig {
        provenance_namespace: "urn:wos:prov:test:".to_string(),
        instance_id: "sp-export-004".to_string(),
    };

    // ── PROV-O ─────────────────────────────────────────────────
    // The facts activity must appear; the narrative activity must not. We
    // assert via activity IRI (minted from the record's position in the
    // filtered log, so the surviving record lands at index 0 regardless of
    // its pre-filter position).
    let prov_doc = prov_o::export(&log, &config);
    let prov_serialized = serde_json::to_value(&prov_doc).expect("PROV-O must serialize");
    let prov_graph = prov_serialized["@graph"]
        .as_array()
        .expect("PROV-O @graph must be an array");
    let activity_iris: Vec<&str> = prov_graph
        .iter()
        .filter(|node| is_activity_node(node, "prov:Activity"))
        .filter_map(|node| node["@id"].as_str())
        .collect();
    assert_eq!(
        activity_iris,
        vec!["urn:wos:prov:test:0"],
        "PROV-O must emit exactly one activity (the facts record) under the §6.5 filter; got {activity_iris:?}"
    );

    // ── XES ────────────────────────────────────────────────────
    let xml = xes::export(&log, &config);
    let event_count = count_xml_elements(&xml, b"event");
    assert_eq!(
        event_count, 1,
        "XES must emit exactly one <event> under the §6.5 filter (narrative record excluded); xml was: {xml}"
    );

    // ── OCEL ───────────────────────────────────────────────────
    let ocel_doc = ocel::export(&log, &config);
    let ocel_events = ocel_doc["events"].as_array().expect("OCEL events array");
    assert_eq!(
        ocel_events.len(),
        1,
        "OCEL must emit exactly one event under the §6.5 filter; got {ocel_events:?}"
    );
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

/// `true` iff `node["@type"]` names a `prov:Activity` (the type is emitted as
/// a single string today — if a future §5.5 subclass pair lands for activities
/// the same array-aware predicate applied to agents below should be mirrored
/// here).
fn is_activity_node(node: &Value, activity_type: &str) -> bool {
    match &node["@type"] {
        Value::String(s) => s == activity_type,
        Value::Array(items) => items
            .iter()
            .any(|v| v.as_str() == Some(activity_type)),
        _ => false,
    }
}

/// `true` iff `node["@type"]` names any PROV-O Agent class. §5.5 subclass
/// pairs expand `"prov:Agent"` to `["prov:Person", "wos:HumanAgent"]`,
/// `["prov:SoftwareAgent", "wos:SystemAgent"]`, or
/// `["prov:SoftwareAgent", "wos:AIAgent"]`. The plain-string `prov:Agent`
/// form still appears when `actor_type` is `None`. A naive
/// `@type == "prov:Agent"` comparison misses every typed agent — this
/// predicate collapses both shapes.
fn is_agent_node(node: &Value) -> bool {
    match &node["@type"] {
        Value::String(s) => s == "prov:Agent",
        Value::Array(items) => items.iter().any(|v| {
            matches!(
                v.as_str(),
                Some("prov:Agent") | Some("prov:Person") | Some("prov:SoftwareAgent")
            )
        }),
        _ => false,
    }
}

/// `true` iff `node["@type"] == "prov:Entity"`. Entity types are always
/// plain strings today — kept as a helper for symmetry with the Activity /
/// Agent predicates and to localise any future subclass expansion.
fn is_entity_node(node: &Value) -> bool {
    node["@type"] == Value::String("prov:Entity".into())
}

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
        .filter(|node| is_activity_node(node, &activity_type))
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

    // §5.3 `wos:definitionVersion` on every activity (populated from the
    // kernel's `version` field by the runtime's populate pass).
    if assertion_bool(assertions, "prov_o_activities_have_definition_version") {
        let expected_version = assertion_string(assertions, "prov_o_expected_definition_version");
        for (index, activity) in activities.iter().enumerate() {
            let actual = activity
                .get("wos:definitionVersion")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    panic!(
                        "PROV-O activity[{index}] missing wos:definitionVersion for fixture '{}': {activity}",
                        fixture.id
                    )
                });
            assert_eq!(
                actual, expected_version,
                "PROV-O activity[{index}] wos:definitionVersion mismatch for fixture '{}'",
                fixture.id
            );
        }
    }

    // §5.3 `wos:atLifecycleState` on at least N activities (records without
    // `lifecycle_state` — e.g. the second, "unresolved" transition emission
    // — legitimately omit it, so we count rather than require every one).
    let min_with_lifecycle = assertion_u64(
        assertions,
        "prov_o_min_activities_with_at_lifecycle_state",
    );
    let with_lifecycle = activities
        .iter()
        .filter(|activity| activity.get("wos:atLifecycleState").is_some())
        .count();
    assert!(
        with_lifecycle as u64 >= min_with_lifecycle,
        "PROV-O activities with wos:atLifecycleState ({with_lifecycle}) must be ≥ {min_with_lifecycle} for fixture '{}'",
        fixture.id
    );

    // §5.3 prov:Entity emissions for StateTransition records with resolved
    // inputs/outputs. Count entities in the graph and confirm they are the
    // expected sibling IRIs (`{namespace}entity/{input|output}/...`).
    let min_entities_per_resolved_transition = assertion_u64(
        assertions,
        "prov_o_min_state_transition_entities_per_resolved_transition",
    );
    let resolved_transition_count = log
        .records()
        .iter()
        .filter(|record| {
            record.record_kind == wos_core::provenance::ProvenanceKind::StateTransition
                && !record.inputs.is_empty()
                && !record.outputs.is_empty()
        })
        .count() as u64;
    let entities: Vec<&Value> = graph.iter().filter(|node| is_entity_node(node)).collect();
    let min_entities = min_entities_per_resolved_transition * resolved_transition_count;
    assert!(
        entities.len() as u64 >= min_entities,
        "PROV-O prov:Entity count ({}) must be ≥ {min_entities} ({min_entities_per_resolved_transition} per {resolved_transition_count} resolved state-transition record(s)) for fixture '{}'",
        entities.len(),
        fixture.id
    );
    // Check IRI shape: input entities start with `{namespace}entity/input/`;
    // output entities with `{namespace}entity/output/`. Each resolved
    // state-transition contributes at least one input-prefix entity and one
    // output-prefix entity (mirroring the total-entity scaling above), so
    // both prefix counts scale with `resolved_transition_count`. An
    // unconditional `>= 1` would silently accept a regression on fixtures
    // with multiple resolved transitions.
    let input_prefix = format!("{}entity/input/", config.provenance_namespace);
    let output_prefix = format!("{}entity/output/", config.provenance_namespace);
    let min_input_entities = resolved_transition_count;
    let min_output_entities = resolved_transition_count;
    let input_entity_count = entities
        .iter()
        .filter(|entity| {
            entity
                .get("@id")
                .and_then(Value::as_str)
                .is_some_and(|iri| iri.starts_with(&input_prefix))
        })
        .count() as u64;
    assert!(
        input_entity_count >= min_input_entities,
        "PROV-O graph must contain ≥ {min_input_entities} prov:Entity with @id prefix '{input_prefix}' (one per resolved state-transition, got {input_entity_count}) for fixture '{}'",
        fixture.id
    );
    let output_entity_count = entities
        .iter()
        .filter(|entity| {
            entity
                .get("@id")
                .and_then(Value::as_str)
                .is_some_and(|iri| iri.starts_with(&output_prefix))
        })
        .count() as u64;
    assert!(
        output_entity_count >= min_output_entities,
        "PROV-O graph must contain ≥ {min_output_entities} prov:Entity with @id prefix '{output_prefix}' (one per resolved state-transition, got {output_entity_count}) for fixture '{}'",
        fixture.id
    );

    // At least one input entity carries `wos:inputDigest` and at least one
    // output entity carries `wos:outputDigest`, each a hex string of the
    // asserted length (sha256 = 64 chars).
    if assertion_bool(assertions, "prov_o_first_entity_has_digest") {
        let digest_len = assertion_u64(assertions, "prov_o_digest_hex_length") as usize;
        let input_digest_count = entities
            .iter()
            .filter(|entity| is_hex_digest_of_len(entity.get("wos:inputDigest"), digest_len))
            .count();
        assert!(
            input_digest_count >= 1,
            "PROV-O must emit at least one prov:Entity with a {digest_len}-char wos:inputDigest for fixture '{}'",
            fixture.id
        );
        let output_digest_count = entities
            .iter()
            .filter(|entity| is_hex_digest_of_len(entity.get("wos:outputDigest"), digest_len))
            .count();
        assert!(
            output_digest_count >= 1,
            "PROV-O must emit at least one prov:Entity with a {digest_len}-char wos:outputDigest for fixture '{}'",
            fixture.id
        );
    }

    // State-transition activities with resolved outputs link to their
    // entities via `prov:used` / `prov:generated`.
    let transitions_with_entity_links = activities
        .iter()
        .filter(|activity| {
            activity.get("wos:actionType").and_then(Value::as_str)
                == Some("stateTransition")
                && activity.get("prov:used").is_some()
                && activity.get("prov:generated").is_some()
        })
        .count();
    assert!(
        transitions_with_entity_links as u64 >= resolved_transition_count,
        "PROV-O must link at least {resolved_transition_count} stateTransition activities via both prov:used and prov:generated; found {transitions_with_entity_links} (fixture '{}')",
        fixture.id
    );

    // Agent node count. §5.5 introduces typed agent nodes — a single
    // `@type` string match would miss every human/system/agent. See
    // `is_agent_node` for the array-aware predicate.
    let min_agents = assertion_u64(assertions, "prov_o_min_agents");
    let agent_count = graph.iter().filter(|node| is_agent_node(node)).count();
    assert!(
        agent_count as u64 >= min_agents,
        "PROV-O agent count ({agent_count}) must be ≥ {min_agents} for fixture '{}'",
        fixture.id
    );

    // §5.5 Human actor yields an Agent whose @type array contains both
    // `prov:Person` AND `wos:HumanAgent`. The fixture declares every actor
    // as Human, so at least one such agent must exist.
    let required_human_pair = assertion_string_array(assertions, "prov_o_human_agent_type_pair");
    let has_human_pair = graph.iter().any(|node| {
        is_agent_node(node)
            && match &node["@type"] {
                Value::Array(items) => required_human_pair.iter().all(|required| {
                    items.iter().any(|v| v.as_str() == Some(required.as_str()))
                }),
                _ => false,
            }
    });
    assert!(
        has_human_pair,
        "PROV-O graph must contain an agent whose @type array is a superset of {required_human_pair:?} (fixture '{}'); graph was: {graph:?}",
        fixture.id
    );

    // §6.5 Facts-tier filter. The shared workflow only emits Facts-tier
    // records (no NarrativeTierRecorded / reasoning / counterfactual kinds),
    // so after export no activity may carry a higher-tier marker. We assert
    // this by checking that no activity's `wos:actionType` maps to a
    // narrative-tier kind. The dedicated sp_export_004 test exercises the
    // filter directly with synthetic mixed-tier records.
    if assertion_bool(assertions, "facts_tier_filter_applied") {
        for activity in &activities {
            let action = activity
                .get("wos:actionType")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert_ne!(
                action, "narrativeTierRecorded",
                "facts-tier filter should have excluded narrative activity from fixture '{}': {activity}",
                fixture.id
            );
        }
    }
}

/// Shape check for a `Value` expected to be a hex string of exactly `length`
/// chars (all lowercase hex digits). Returns false for `None`, non-strings,
/// wrong lengths, or non-hex characters.
fn is_hex_digest_of_len(value: Option<&Value>, length: usize) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|s| s.len() == length && s.chars().all(|c| c.is_ascii_hexdigit()))
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
    let event_attributes = extract_event_attributes(&xml);
    assert_eq!(
        event_attributes.len(),
        log.len(),
        "XES parsed <event> count ({}) must equal log length ({}) for fixture '{}'",
        event_attributes.len(),
        log.len(),
        fixture.id
    );
    for required in assertion_string_array(assertions, "xes_event_required_keys") {
        for (index, attrs) in event_attributes.iter().enumerate() {
            assert!(
                attrs.iter().any(|(k, _)| k == &required),
                "XES event[{index}] missing key '{required}' for fixture '{}' (keys present: {:?})",
                fixture.id,
                attrs.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>()
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

    // §5.5 / §6.3 `org:group` — emitted ONLY when `actor_type` is populated,
    // so events without an actor (OnEntry hooks, etc.) legitimately omit it.
    // We therefore count events with the key rather than requiring it on
    // every event, and check the emitted value matches.
    let min_org_group = assertion_u64(assertions, "xes_min_events_with_org_group");
    let expected_org_group = assertion_string(assertions, "xes_expected_org_group_value");
    let org_group_events: Vec<usize> = event_attributes
        .iter()
        .enumerate()
        .filter_map(|(index, attrs)| {
            attrs
                .iter()
                .any(|(k, v)| k == "org:group" && v == &expected_org_group)
                .then_some(index)
        })
        .collect();
    assert!(
        org_group_events.len() as u64 >= min_org_group,
        "XES must emit at least {min_org_group} events with org:group='{expected_org_group}'; found {} (fixture '{}')",
        org_group_events.len(),
        fixture.id
    );

    // §6.3 repeated `wos:input` / `wos:output` keys (singular; NOT comma-
    // joined plural `wos:inputs` / `wos:outputs`). State-transition events
    // with resolved inputs/outputs must carry at least one of each.
    let min_input_events = assertion_u64(assertions, "xes_min_events_with_input_key");
    let min_output_events = assertion_u64(assertions, "xes_min_events_with_output_key");
    let events_with_input = event_attributes
        .iter()
        .filter(|attrs| attrs.iter().any(|(k, _)| k == "wos:input"))
        .count();
    let events_with_output = event_attributes
        .iter()
        .filter(|attrs| attrs.iter().any(|(k, _)| k == "wos:output"))
        .count();
    assert!(
        events_with_input as u64 >= min_input_events,
        "XES must emit at least {min_input_events} events with a wos:input attribute; found {events_with_input} (fixture '{}')",
        fixture.id
    );
    assert!(
        events_with_output as u64 >= min_output_events,
        "XES must emit at least {min_output_events} events with a wos:output attribute; found {events_with_output} (fixture '{}')",
        fixture.id
    );

    // §6.3 digests — sha256 hex strings (64 chars).
    let digest_len = assertion_u64(assertions, "xes_digest_hex_length") as usize;
    let min_input_digest = assertion_u64(assertions, "xes_min_events_with_input_digest");
    let min_output_digest = assertion_u64(assertions, "xes_min_events_with_output_digest");
    let events_with_input_digest = event_attributes
        .iter()
        .filter(|attrs| {
            attrs.iter().any(|(k, v)| {
                k == "wos:inputDigest"
                    && v.len() == digest_len
                    && v.chars().all(|c| c.is_ascii_hexdigit())
            })
        })
        .count();
    let events_with_output_digest = event_attributes
        .iter()
        .filter(|attrs| {
            attrs.iter().any(|(k, v)| {
                k == "wos:outputDigest"
                    && v.len() == digest_len
                    && v.chars().all(|c| c.is_ascii_hexdigit())
            })
        })
        .count();
    assert!(
        events_with_input_digest as u64 >= min_input_digest,
        "XES must emit at least {min_input_digest} events with a {digest_len}-char wos:inputDigest; found {events_with_input_digest} (fixture '{}')",
        fixture.id
    );
    assert!(
        events_with_output_digest as u64 >= min_output_digest,
        "XES must emit at least {min_output_digest} events with a {digest_len}-char wos:outputDigest; found {events_with_output_digest} (fixture '{}')",
        fixture.id
    );

    // Guard against regressing to the legacy comma-joined plural form
    // (`wos:inputs`/`wos:outputs`). Repeated singular keys are the §6.3
    // contract after the Task 5 fix.
    if assertion_bool(assertions, "xes_rejects_joined_inputs_outputs_keys") {
        for (index, attrs) in event_attributes.iter().enumerate() {
            for (k, _) in attrs {
                assert_ne!(
                    k, "wos:inputs",
                    "XES event[{index}] emitted legacy joined key 'wos:inputs' (fixture '{}')",
                    fixture.id
                );
                assert_ne!(
                    k, "wos:outputs",
                    "XES event[{index}] emitted legacy joined key 'wos:outputs' (fixture '{}')",
                    fixture.id
                );
            }
        }
    }

    // §6.3 trace-level `wos:definitionVersion`. The attribute must appear on
    // the <trace> itself (before the first <event>), with the expected value.
    if assertion_bool(assertions, "xes_trace_has_definition_version") {
        let expected_version = assertion_string(assertions, "xes_expected_definition_version");
        let expected_attr = format!(
            r#"<string key="wos:definitionVersion" value="{expected_version}"/>"#
        );
        let trace_header = xml
            .split("<event>")
            .next()
            .expect("at least the pre-event prefix exists");
        assert!(
            trace_header.contains(&expected_attr),
            "XES trace-level wos:definitionVersion missing or misplaced for fixture '{}': expected {expected_attr} in trace header, got: {trace_header}",
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

    // §6.4 objectTypes declarations. The current exporter models only the
    // workflow instance itself as an object, so exactly one type is expected.
    let declared_object_types: Vec<String> = document["objectTypes"]
        .as_array()
        .unwrap_or_else(|| {
            panic!("OCEL 'objectTypes' must be an array for fixture '{}'", fixture.id)
        })
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str).map(String::from))
        .collect();
    let expected_object_types = assertion_string_array(assertions, "ocel_object_type_names");
    assert_eq!(
        declared_object_types, expected_object_types,
        "OCEL objectTypes mismatch for fixture '{}'",
        fixture.id
    );

    // §6.4 eventTypes must declare the uniform static attribute schema
    // (actorId / actorType / lifecycleState / definitionVersion / fromState
    // / toState / event / inputDigest / outputDigest). The dynamic indexed
    // `inputs.{i}` / `outputs.{i}` attributes vary per record and are NOT
    // declared here (OCEL 2.0 tolerates undeclared attributes).
    let event_types = document["eventTypes"]
        .as_array()
        .unwrap_or_else(|| panic!("OCEL 'eventTypes' must be an array for fixture '{}'", fixture.id));
    assert!(
        !event_types.is_empty(),
        "OCEL eventTypes must not be empty for fixture '{}'",
        fixture.id
    );
    let required_schema =
        assertion_string_array(assertions, "ocel_event_type_required_schema_attributes");
    for event_type in event_types {
        let declared_names: Vec<&str> = event_type["attributes"]
            .as_array()
            .unwrap_or_else(|| {
                panic!(
                    "OCEL eventType '{}' missing attributes array (fixture '{}'): {event_type}",
                    event_type["name"], fixture.id
                )
            })
            .iter()
            .filter_map(|attr| attr.get("name").and_then(Value::as_str))
            .collect();
        for required in &required_schema {
            assert!(
                declared_names.iter().any(|n| n == required),
                "OCEL eventType '{}' missing required schema attribute '{required}' for fixture '{}': declared = {declared_names:?}",
                event_type["name"],
                fixture.id
            );
        }
    }

    // §5.3 / §5.5 / §6.3 per-event attributes. Each check counts the events
    // carrying the attribute rather than requiring it on every event —
    // OnEntry records without an actor (for example) legitimately omit
    // actorType, lifecycleState, inputs, etc.
    let min_actor_type = assertion_u64(assertions, "ocel_min_events_with_actor_type");
    let min_lifecycle = assertion_u64(assertions, "ocel_min_events_with_lifecycle_state");
    let min_definition_version =
        assertion_u64(assertions, "ocel_min_events_with_definition_version");
    let min_input_digest = assertion_u64(assertions, "ocel_min_events_with_input_digest");
    let min_output_digest = assertion_u64(assertions, "ocel_min_events_with_output_digest");
    let min_indexed_inputs = assertion_u64(assertions, "ocel_min_events_with_indexed_inputs");
    let min_indexed_outputs = assertion_u64(assertions, "ocel_min_events_with_indexed_outputs");

    let count_events_with_attribute = |name: &str| -> u64 {
        events
            .iter()
            .filter(|event| {
                event["attributes"]
                    .as_array()
                    .is_some_and(|attrs| attrs.iter().any(|attr| attr["name"] == name))
            })
            .count() as u64
    };
    let count_events_with_prefixed_attribute = |prefix: &str| -> u64 {
        events
            .iter()
            .filter(|event| {
                event["attributes"].as_array().is_some_and(|attrs| {
                    attrs.iter().any(|attr| {
                        attr["name"].as_str().is_some_and(|n| n.starts_with(prefix))
                    })
                })
            })
            .count() as u64
    };

    let actor_type_count = count_events_with_attribute("actorType");
    assert!(
        actor_type_count >= min_actor_type,
        "OCEL must emit at least {min_actor_type} events with actorType attribute; found {actor_type_count} (fixture '{}')",
        fixture.id
    );
    let lifecycle_count = count_events_with_attribute("lifecycleState");
    assert!(
        lifecycle_count >= min_lifecycle,
        "OCEL must emit at least {min_lifecycle} events with lifecycleState attribute; found {lifecycle_count} (fixture '{}')",
        fixture.id
    );
    let definition_version_count = count_events_with_attribute("definitionVersion");
    assert!(
        definition_version_count >= min_definition_version,
        "OCEL must emit at least {min_definition_version} events with definitionVersion attribute; found {definition_version_count} (fixture '{}')",
        fixture.id
    );
    let input_digest_count = count_events_with_attribute("inputDigest");
    assert!(
        input_digest_count >= min_input_digest,
        "OCEL must emit at least {min_input_digest} events with inputDigest attribute; found {input_digest_count} (fixture '{}')",
        fixture.id
    );
    let output_digest_count = count_events_with_attribute("outputDigest");
    assert!(
        output_digest_count >= min_output_digest,
        "OCEL must emit at least {min_output_digest} events with outputDigest attribute; found {output_digest_count} (fixture '{}')",
        fixture.id
    );
    let indexed_inputs_count = count_events_with_prefixed_attribute("inputs.");
    assert!(
        indexed_inputs_count >= min_indexed_inputs,
        "OCEL must emit at least {min_indexed_inputs} events with indexed `inputs.{{i}}` attributes; found {indexed_inputs_count} (fixture '{}')",
        fixture.id
    );
    let indexed_outputs_count = count_events_with_prefixed_attribute("outputs.");
    assert!(
        indexed_outputs_count >= min_indexed_outputs,
        "OCEL must emit at least {min_indexed_outputs} events with indexed `outputs.{{i}}` attributes; found {indexed_outputs_count} (fixture '{}')",
        fixture.id
    );

    // OCEL 2.0 requires attribute values to be scalars — guard against the
    // legacy JSON-array form of `inputs` / `outputs`.
    for (index, event) in events.iter().enumerate() {
        if let Some(attrs) = event["attributes"].as_array() {
            for attr in attrs {
                let name = attr["name"].as_str().unwrap_or("");
                assert!(
                    name != "inputs" && name != "outputs",
                    "OCEL event[{index}] emitted legacy array attribute '{name}' (fixture '{}'): {attr}",
                    fixture.id
                );
                assert!(
                    !attr["value"].is_array(),
                    "OCEL event[{index}] attribute '{name}' must be a scalar (OCEL 2.0); got array: {attr}"
                );
            }
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
    extract_event_attributes(xml)
        .into_iter()
        .map(|attrs| attrs.into_iter().map(|(k, _)| k).collect())
        .collect()
}

/// For each `<event>...</event>` block, collect the `(key, value)` pairs of
/// its child `<string>` / `<date>` elements in document order. Preserves
/// duplicate keys (§6.3 emits repeated `wos:input` / `wos:output` keys, one
/// per input/output item). Using the parser (not a `<event>` literal split)
/// stays robust against future attribute variants on the open tag.
fn extract_event_attributes(xml: &str) -> Vec<Vec<(String, String)>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut events: Vec<Vec<(String, String)>> = Vec::new();
    let mut current: Option<Vec<(String, String)>> = None;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Eof) => break,
            Ok(XmlEvent::Start(element)) if element.name().as_ref() == b"event" => {
                current = Some(Vec::new());
            }
            Ok(XmlEvent::End(element)) if element.name().as_ref() == b"event" => {
                if let Some(pairs) = current.take() {
                    events.push(pairs);
                }
            }
            Ok(XmlEvent::Empty(element)) | Ok(XmlEvent::Start(element)) => {
                if let Some(pairs) = current.as_mut() {
                    let mut key: Option<String> = None;
                    let mut value: Option<String> = None;
                    for attr in element.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"key" => {
                                key = Some(String::from_utf8_lossy(&attr.value).into_owned());
                            }
                            b"value" => {
                                value = Some(String::from_utf8_lossy(&attr.value).into_owned());
                            }
                            _ => {}
                        }
                    }
                    if let Some(k) = key {
                        pairs.push((k, value.unwrap_or_default()));
                    }
                }
            }
            Ok(_) => {}
            Err(error) => panic!(
                "XES extract_event_attributes hit a parse error at byte {}: {error:?}",
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
