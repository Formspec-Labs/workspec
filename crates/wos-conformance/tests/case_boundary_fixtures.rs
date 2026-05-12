// Rust guideline compliant 2026-05-12

//! Case-boundary conformance fixtures (REFACTOR-TODO Tasks 4C.1 + 4C.2).
//!
//! Exercises the N:1 case-to-processes invariant (CBR §4.3) and the post-ledger
//! direct-append surface (CBR §4.6) by driving the `wos-runtime` `InMemoryStore`
//! through the case-scoped primitives (`create_record`, `append_provenance_for_case`,
//! `processes_for_case`, `provenance_for_case`) declared in
//! [`work-spec/crates/wos-runtime/src/store.rs`].
//!
//! These fixtures intentionally bypass the workflow state-machine harness used
//! by `tests/signature_profile.rs`: the case-boundary contract sits beneath
//! workflow drains and MUST be observable independent of any kernel transition.
//!
//! See `work-spec/thoughts/analysis/case-boundary-decision-report.md`
//! §4.3 (N:1) and §4.6-§4.7 (direct-append surface + conformance).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use wos_core::instance::{InstanceStatus, WorkflowProcess};
use wos_core::provenance::ProvenanceRecord;
use wos_runtime::{InMemoryStore, RuntimeRecord, RuntimeStore};

/// Case-boundary fixture format. Distinct from `ConformanceFixture` because
/// these tests drive the case-ledger store directly rather than the workflow
/// engine — the case-boundary surface predates workflow drain semantics.
#[derive(Debug, Deserialize)]
struct CaseBoundaryFixture {
    id: String,
    #[allow(dead_code)]
    rule: String,
    #[allow(dead_code)]
    description: String,
    case_ledger_id: String,
    processes: Vec<ProcessSpec>,
    actions: Vec<Action>,
    expected_processes_for_case: Vec<String>,
    expected_case_view_timestamps: Vec<String>,
    expected_case_view_events: Vec<ExpectedEvent>,
    #[serde(default)]
    expected_no_workflow_drain: bool,
}

#[derive(Debug, Deserialize)]
struct ProcessSpec {
    process_id: String,
    definition_url: String,
    definition_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Action {
    /// Direct-append surface (CBR §4.6) — `POST /cases/{case_id}/events`.
    /// Models a non-workflow-driven event written straight to the case ledger
    /// via `append_provenance_for_case`. Distinguished from `appendProvenance`
    /// because the event_type is a fully-qualified `wos.*` literal rather than
    /// a transition event name.
    DirectAppend {
        case_ledger_id: String,
        process_id: String,
        event_type: String,
        #[serde(default)]
        actor_id: Option<String>,
        timestamp: String,
        #[allow(dead_code)]
        #[serde(default)]
        label: Option<String>,
    },
    /// Workflow-style transition append. Distinguished from direct-append so
    /// future variants can route through a real drain rather than a synthetic
    /// state_transition record without changing the fixture schema.
    AppendProvenance {
        process_id: String,
        event: String,
        from_state: String,
        to_state: String,
        #[serde(default)]
        actor_id: Option<String>,
        timestamp: String,
        #[allow(dead_code)]
        #[serde(default)]
        label: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct ExpectedEvent {
    event: String,
    process_id: String,
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("case-boundary")
        .join(name)
}

fn load_fixture(name: &str) -> CaseBoundaryFixture {
    let path = fixture_path(name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("parse fixture {}: {e}", path.display()))
}

fn workflow_process(spec: &ProcessSpec, case_ledger_id: &str) -> WorkflowProcess {
    WorkflowProcess {
        process_id: spec.process_id.clone(),
        case_ledger_id: case_ledger_id.to_string(),
        tenant: wos_core::typeid::DEFAULT_TENANT.to_string(),
        definition_url: spec.definition_url.clone(),
        definition_version: spec.definition_version.clone(),
        configuration: Vec::new(),
        case_state: serde_json::Value::Null,
        provenance_position: 0,
        next_task_sequence: 0,
        timers: Vec::new(),
        active_tasks: Vec::new(),
        history_store: Default::default(),
        compensation_logs: Default::default(),
        status: InstanceStatus::Active,
        stalled_since: None,
        decline_reason: None,
        voided_by: None,
        voided_at: None,
        expired_at: None,
        pending_events: Vec::new(),
        governance_state: None,
        volume_counters: None,
        fired_milestones: Default::default(),
        pending_callbacks: Default::default(),
        created_at: "1970-01-01T00:00:00Z".to_string(),
        updated_at: "1970-01-01T00:00:00Z".to_string(),
        extensions: Default::default(),
    }
}

/// Build a stamped state-transition record for the workflow-style action.
fn stamped_transition(
    from: &str,
    to: &str,
    event: &str,
    actor_id: Option<&str>,
    timestamp: &str,
) -> ProvenanceRecord {
    let mut record = ProvenanceRecord::state_transition(from, to, event, actor_id);
    record.timestamp = timestamp.to_string();
    record
}

/// Build a stamped direct-append record. The event-type literal is stored on
/// `event` so the case view exposes the fully-qualified `wos.*` name. Actor and
/// timestamp are honored verbatim — this is the post-ledger direct surface, so
/// the runtime stamper is NOT invoked.
fn stamped_direct_append(
    event_type: &str,
    actor_id: Option<&str>,
    timestamp: &str,
) -> ProvenanceRecord {
    let mut record = ProvenanceRecord::state_transition(
        "(direct-append)",
        "(direct-append)",
        event_type,
        actor_id,
    );
    record.timestamp = timestamp.to_string();
    record
}

fn run_fixture(fixture: &CaseBoundaryFixture) {
    let mut store = InMemoryStore::new();

    // Create one runtime record per process spec, all bound to the same case ledger.
    for spec in &fixture.processes {
        let process = workflow_process(spec, &fixture.case_ledger_id);
        store
            .create_record(RuntimeRecord::new(process))
            .unwrap_or_else(|e| {
                panic!(
                    "fixture {}: create_record for process {} failed: {e:?}",
                    fixture.id, spec.process_id
                )
            });
    }

    // Replay the action sequence against the store.
    let mut drain_invocations: HashMap<String, u32> = HashMap::new();
    for action in &fixture.actions {
        match action {
            Action::DirectAppend {
                case_ledger_id,
                process_id,
                event_type,
                actor_id,
                timestamp,
                ..
            } => {
                let record =
                    stamped_direct_append(event_type, actor_id.as_deref(), timestamp);
                store
                    .append_provenance_for_case(case_ledger_id, process_id, record)
                    .unwrap_or_else(|e| {
                        panic!(
                            "fixture {}: directAppend for case {} via {} failed: {e:?}",
                            fixture.id, case_ledger_id, process_id
                        )
                    });
            }
            Action::AppendProvenance {
                process_id,
                event,
                from_state,
                to_state,
                actor_id,
                timestamp,
                ..
            } => {
                let record =
                    stamped_transition(from_state, to_state, event, actor_id.as_deref(), timestamp);
                store
                    .append_provenance_for_case(
                        &fixture.case_ledger_id,
                        process_id,
                        record,
                    )
                    .unwrap_or_else(|e| {
                        panic!(
                            "fixture {}: appendProvenance for {} failed: {e:?}",
                            fixture.id, process_id
                        )
                    });
                *drain_invocations.entry(process_id.clone()).or_default() += 1;
            }
        }
    }

    // Assertion 1: processes_for_case lists every expected process_id.
    let mut actual_processes = store.processes_for_case(&fixture.case_ledger_id);
    actual_processes.sort();
    let mut expected_processes = fixture.expected_processes_for_case.clone();
    expected_processes.sort();
    assert_eq!(
        actual_processes, expected_processes,
        "fixture {}: processes_for_case mismatch (N:1 binding invariant)",
        fixture.id
    );

    // Assertion 2: provenance_for_case returns events in time order across all
    // processes bound to the case ledger.
    let merged = store.provenance_for_case(&fixture.case_ledger_id);
    let actual_timestamps: Vec<&str> = merged.iter().map(|r| r.timestamp.as_str()).collect();
    assert_eq!(
        actual_timestamps, fixture.expected_case_view_timestamps,
        "fixture {}: case-view timestamp order mismatch (provenance_for_case time-sorted across processes)",
        fixture.id
    );

    // Assertion 3: each event in the case view matches the expected event-name
    // and emitting process. Since `provenance_for_case` doesn't carry the
    // process_id on the record, we cross-check by replaying through
    // load_record for each candidate process and confirming the event lives in
    // its log.
    assert_eq!(
        merged.len(),
        fixture.expected_case_view_events.len(),
        "fixture {}: case-view length mismatch",
        fixture.id
    );
    for (idx, (actual, expected)) in merged
        .iter()
        .zip(fixture.expected_case_view_events.iter())
        .enumerate()
    {
        let actual_event = actual.event.as_deref().unwrap_or_default();
        assert_eq!(
            actual_event, expected.event,
            "fixture {}: case-view event[{idx}] name mismatch",
            fixture.id
        );
        let process_record = store
            .load_record(&expected.process_id)
            .unwrap_or_else(|e| {
                panic!(
                    "fixture {}: load_record for {} failed: {e:?}",
                    fixture.id, expected.process_id
                )
            });
        assert!(
            process_record
                .provenance_log
                .iter()
                .any(|r| r.timestamp == actual.timestamp
                    && r.event.as_deref() == Some(expected.event.as_str())),
            "fixture {}: case-view event[{idx}] ({}) not attributable to process {}",
            fixture.id,
            expected.event,
            expected.process_id,
        );
    }

    // Assertion 4 (direct-append fixtures only): no workflow drain was invoked.
    // For Task 4C.2 acceptance — the appended event reaches the view without
    // going through any workflow drain (CBR §4.6 post-ledger branch).
    if fixture.expected_no_workflow_drain {
        assert!(
            drain_invocations.is_empty(),
            "fixture {}: expected_no_workflow_drain=true but appendProvenance actions ran: {drain_invocations:?}",
            fixture.id
        );
    }
}

/// Task 4C.1 — N:1 fixture (CBR §4.3). Two processes started on one case
/// ledger; events interleave time-ordered; case view rebuild reflects both
/// contributions; processes_for_case lists both process_ids.
#[test]
fn cbr_4c1_n_to_one_concurrent_processes_on_one_case_ledger() {
    let fixture = load_fixture("n-to-one-concurrent.json");
    assert_eq!(fixture.processes.len(), 2, "N:1 fixture must declare two processes");
    run_fixture(&fixture);
}

/// Task 4C.2 — Direct-append fixture (CBR §4.6). Genesis `wos.kernel.case_created`
/// creates the ledger; a follow-up `wos.kernel.intake_accepted` direct-append
/// reaches the case view without invoking any workflow drain.
///
/// CBR §4.6 names `wos.kernel.note_added` but that kind is NOT yet registered
/// in `work-spec/schemas/record-kind-registry.json` (only 14 kinds carry
/// eventLiteral entries at HEAD). The fixture substitutes `IntakeAccepted`
/// (foundation category, post-ledger, eventLiteral `wos.kernel.intake_accepted`)
/// as the closest existing analog. Re-target to `wos.kernel.note_added` once
/// that kind lands in the registry.
#[test]
fn cbr_4c2_direct_append_post_ledger_without_workflow_drain() {
    let fixture = load_fixture("direct-append-intake-accepted.json");
    assert!(
        fixture.expected_no_workflow_drain,
        "direct-append fixture MUST assert no workflow drain (CBR §4.6)",
    );
    run_fixture(&fixture);
}
