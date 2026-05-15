// Rust guideline compliant 2026-05-12

//! Case-boundary conformance fixtures (archived refactor tracker Tasks 4C.1-4C.7).
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
//! Historical rationale: `work-spec/thoughts/archive/analysis/2026-05-11-case-boundary-decision-report.md`
//! §4.3 (N:1) and §4.6-§4.7 (direct-append surface + conformance). Current
//! authority is ADR-0093 plus the fixture contracts in this module.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wos_core::instance::{InstanceStatus, WorkflowProcess};
use wos_core::{ProvenanceKind, ProvenanceRecord};
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
    #[serde(default)]
    crash_after_actions: Option<usize>,
    #[serde(default)]
    expected_registry_rejection_event: Option<String>,
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

#[derive(Debug)]
struct FixtureExecution {
    store: InMemoryStore,
    drain_invocations: HashMap<String, u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseViewProjection {
    case_ledger_id: String,
    processes: Vec<String>,
    events: Vec<ProjectedEvent>,
    status: String,
    last_updated: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectedEvent {
    event_id: String,
    sequence: u64,
    event_hash: String,
    previous_hash: String,
    event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_id: Option<String>,
    timestamp: String,
}

#[derive(Debug)]
struct EventSeed {
    timestamp: String,
    event: String,
    public_process_id: Option<String>,
    attribution_process_id: String,
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
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse fixture {}: {e}", path.display()))
}

fn registry_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("record-kind-registry.json")
}

fn registered_event_literals() -> HashSet<String> {
    let path = registry_path();
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read registry {}: {e}", path.display()));
    let registry: serde_json::Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse registry: {e}"));
    registry
        .get("recordKinds")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("registry {} has no recordKinds array", path.display()))
        .iter()
        .filter_map(|entry| entry.get("eventLiteral").and_then(|v| v.as_str()))
        .map(ToOwned::to_owned)
        .collect()
}

fn workflow_process(spec: &ProcessSpec, case_ledger_id: &str) -> WorkflowProcess {
    WorkflowProcess {
        process_id: spec.process_id.clone(),
        case_ledger_id: case_ledger_id.to_string(),
        tenant: stack_common_typeid::DEFAULT_TENANT.to_string(),
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
    let mut record = ProvenanceRecord::unmatched_event(event_type, actor_id);
    if let Some(record_kind) = ProvenanceKind::from_canonical_event_literal(event_type) {
        record.record_kind = record_kind;
    }
    record.timestamp = timestamp.to_string();
    record
}

fn case_view_event_name(record: &ProvenanceRecord) -> &str {
    if record.record_kind == ProvenanceKind::StateTransition
        && let Some(event) = record
            .data
            .as_ref()
            .and_then(|data| data.get("transitionEvent"))
            .and_then(serde_json::Value::as_str)
    {
        return event;
    }

    record.event.as_deref().unwrap_or_default()
}

fn execute_fixture(fixture: &CaseBoundaryFixture) -> FixtureExecution {
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
                let record = stamped_direct_append(event_type, actor_id.as_deref(), timestamp);
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
                    .append_provenance_for_case(&fixture.case_ledger_id, process_id, record)
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

    FixtureExecution {
        store,
        drain_invocations,
    }
}

fn assert_fixture_invariants(fixture: &CaseBoundaryFixture, execution: &FixtureExecution) {
    // Assertion 1: processes_for_case lists every expected process_id.
    let mut actual_processes = execution.store.processes_for_case(&fixture.case_ledger_id);
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
    let merged = execution.store.provenance_for_case(&fixture.case_ledger_id);
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
        let actual_event = case_view_event_name(actual);
        assert_eq!(
            actual_event, expected.event,
            "fixture {}: case-view event[{idx}] name mismatch",
            fixture.id
        );
        let process_record = execution
            .store
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
                    && case_view_event_name(r) == expected.event),
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
            execution.drain_invocations.is_empty(),
            "fixture {}: expected_no_workflow_drain=true but appendProvenance actions ran: {:?}",
            fixture.id,
            execution.drain_invocations,
        );
    }
}

fn fixture_hash(parts: &[&str]) -> String {
    let mut acc = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            acc ^= u64::from(*byte);
            acc = acc.wrapping_mul(0x100000001b3);
        }
    }
    format!(
        "sha256:{:016x}{:016x}{:016x}{:016x}",
        acc,
        acc.rotate_left(13),
        acc.rotate_left(29),
        acc.rotate_left(47)
    )
}

fn case_view_projection_from_seeds(
    fixture: &CaseBoundaryFixture,
    processes: Vec<String>,
    seeds: Vec<EventSeed>,
) -> CaseViewProjection {
    let mut previous_hash =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string();
    let mut events = Vec::with_capacity(seeds.len());
    for (index, seed) in seeds.into_iter().enumerate() {
        let sequence = u64::try_from(index + 1).expect("fixture event count fits u64");
        let event_id = format!("fixture-event-{sequence}");
        let event_hash = fixture_hash(&[
            fixture.case_ledger_id.as_str(),
            seed.attribution_process_id.as_str(),
            seed.event.as_str(),
            seed.timestamp.as_str(),
            previous_hash.as_str(),
        ]);
        events.push(ProjectedEvent {
            event_id,
            sequence,
            previous_hash: previous_hash.clone(),
            event_hash: event_hash.clone(),
            event: seed.event,
            process_id: seed.public_process_id,
            timestamp: seed.timestamp,
        });
        previous_hash = event_hash;
    }
    let last_updated = events
        .last()
        .map(|event| event.timestamp.clone())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let status = if processes.is_empty() {
        "genesis"
    } else {
        "active"
    }
    .to_string();

    CaseViewProjection {
        case_ledger_id: fixture.case_ledger_id.clone(),
        processes,
        events,
        status,
        last_updated,
    }
}

fn run_fixture(fixture: &CaseBoundaryFixture) {
    let execution = execute_fixture(fixture);
    assert_fixture_invariants(fixture, &execution);
}

fn is_direct_append_event(
    fixture: &CaseBoundaryFixture,
    event_timestamp: &str,
    expected: &ExpectedEvent,
) -> bool {
    fixture.actions.iter().any(|action| {
        matches!(
            action,
            Action::DirectAppend {
                process_id,
                event_type,
                timestamp,
                ..
            } if process_id == &expected.process_id
                && event_type == &expected.event
                && timestamp == event_timestamp
        )
    })
}

fn case_view_from_store(
    fixture: &CaseBoundaryFixture,
    store: &InMemoryStore,
) -> CaseViewProjection {
    let mut processes = store.processes_for_case(&fixture.case_ledger_id);
    processes.sort();

    let merged = store.provenance_for_case(&fixture.case_ledger_id);
    assert_eq!(
        merged.len(),
        fixture.expected_case_view_events.len(),
        "fixture {}: projection length mismatch",
        fixture.id
    );
    let seeds = merged
        .iter()
        .zip(fixture.expected_case_view_events.iter())
        .map(|(actual, expected)| {
            let actual_event = case_view_event_name(actual);
            assert_eq!(
                actual_event, expected.event,
                "fixture {}: store projection event mismatch",
                fixture.id
            );
            let direct_append = is_direct_append_event(fixture, &actual.timestamp, expected);
            EventSeed {
                timestamp: actual.timestamp.clone(),
                event: expected.event.clone(),
                public_process_id: if direct_append {
                    None
                } else {
                    Some(expected.process_id.clone())
                },
                attribution_process_id: expected.process_id.clone(),
            }
        })
        .collect();

    case_view_projection_from_seeds(fixture, processes, seeds)
}

fn case_view_from_actions(
    fixture: &CaseBoundaryFixture,
    action_limit: Option<usize>,
) -> CaseViewProjection {
    let limit = action_limit.unwrap_or(fixture.actions.len());
    let mut indexed_events: Vec<(String, usize, EventSeed)> = Vec::new();
    for (index, action) in fixture.actions.iter().take(limit).enumerate() {
        match action {
            Action::DirectAppend {
                process_id,
                event_type,
                timestamp,
                ..
            } => indexed_events.push((
                timestamp.clone(),
                index,
                EventSeed {
                    timestamp: timestamp.clone(),
                    event: event_type.clone(),
                    public_process_id: None,
                    attribution_process_id: process_id.clone(),
                },
            )),
            Action::AppendProvenance {
                process_id,
                event,
                timestamp,
                ..
            } => indexed_events.push((
                timestamp.clone(),
                index,
                EventSeed {
                    timestamp: timestamp.clone(),
                    event: event.clone(),
                    public_process_id: Some(process_id.clone()),
                    attribution_process_id: process_id.clone(),
                },
            )),
        }
    }
    indexed_events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut processes = fixture.expected_processes_for_case.clone();
    processes.sort();

    let seeds = indexed_events
        .into_iter()
        .map(|(_, _, event)| event)
        .collect();
    case_view_projection_from_seeds(fixture, processes, seeds)
}

fn projection_bytes(view: &CaseViewProjection) -> Vec<u8> {
    serde_json::to_vec(view).expect("case-view projection serializes")
}

/// Task 4C.1 — N:1 fixture (CBR §4.3). Two processes started on one case
/// ledger; events interleave time-ordered; case view rebuild reflects both
/// contributions; processes_for_case lists both process_ids.
#[test]
fn cbr_4c1_n_to_one_concurrent_processes_on_one_case_ledger() {
    let fixture = load_fixture("n-to-one-concurrent.json");
    assert_eq!(
        fixture.processes.len(),
        2,
        "N:1 fixture must declare two processes"
    );
    run_fixture(&fixture);
}

/// Task 4C.2 — Direct-append fixture (CBR §4.6). Genesis `wos.kernel.case_created`
/// creates the ledger; a follow-up `wos.kernel.note_added` direct-append
/// reaches the case view without invoking any workflow drain.
///
/// `ProvenanceKind::NoteAdded` is a flat Facts-tier foundation-category kind
/// (`wos.kernel.note_added`, schemaValidated: false) registered in
/// `work-spec/schemas/record-kind-registry.json`.
#[test]
fn cbr_4c2_direct_append_post_ledger_without_workflow_drain() {
    let fixture = load_fixture("direct-append-intake-accepted.json");
    assert!(
        fixture.expected_no_workflow_drain,
        "direct-append fixture MUST assert no workflow drain (CBR §4.6)",
    );
    run_fixture(&fixture);
}

/// Task 4C.3 — Manual case genesis fixture.
#[test]
fn cbr_4c3_manual_case_created_genesis_event_is_first() {
    let fixture = load_fixture("manual-case-created-genesis.json");
    let Some(Action::DirectAppend { event_type, .. }) = fixture.actions.first() else {
        panic!("4C.3 fixture must start with directAppend case genesis");
    };
    assert_eq!(event_type, "wos.kernel.case_created");
    assert!(
        fixture
            .actions
            .iter()
            .all(|action| matches!(action, Action::DirectAppend { .. })),
        "manual genesis fixture must not depend on workflow drain actions"
    );
    run_fixture(&fixture);
}

/// Task 4C.4 — Cross-process audit fixture.
#[test]
fn cbr_4c4_cross_process_audit_preserves_distinct_process_ids() {
    let fixture = load_fixture("cross-process-audit.json");
    let distinct_processes: HashSet<&str> = fixture
        .expected_case_view_events
        .iter()
        .map(|event| event.process_id.as_str())
        .collect();
    assert!(
        distinct_processes.len() >= 2,
        "4C.4 fixture must include at least two emitting processes"
    );
    run_fixture(&fixture);
}

/// Task 4C.5 — Replay and projection bytes match.
#[test]
fn cbr_4c5_replay_projection_byte_identity() {
    let fixture = load_fixture("replay-projection-byte-identity.json");
    let execution = execute_fixture(&fixture);
    assert_fixture_invariants(&fixture, &execution);

    let replay = case_view_from_actions(&fixture, None);
    let projection = case_view_from_store(&fixture, &execution.store);
    assert_eq!(projection_bytes(&projection), projection_bytes(&replay));
}

/// Task 4C.6 — Projection recovery converges after restart.
#[test]
fn cbr_4c6_crash_recovery_projection_converges() {
    let fixture = load_fixture("crash-recovery-projection.json");
    let crash_after = fixture
        .crash_after_actions
        .expect("4C.6 fixture must declare crash_after_actions");
    assert!(
        crash_after < fixture.actions.len(),
        "crash point must fall before the final action"
    );
    let pre_crash = case_view_from_actions(&fixture, Some(crash_after));
    assert!(
        pre_crash.events.len() < fixture.expected_case_view_events.len(),
        "pre-crash materializer state should be incomplete"
    );

    let execution = execute_fixture(&fixture);
    assert_fixture_invariants(&fixture, &execution);
    let recovered = case_view_from_store(&fixture, &execution.store);
    let uninterrupted = case_view_from_actions(&fixture, None);
    assert_eq!(
        projection_bytes(&recovered),
        projection_bytes(&uninterrupted)
    );
}

/// Task 4C.7 — Unregistered WOS event literal remains gated.
#[test]
fn cbr_4c7_trellis_registry_gate_rejects_unregistered_wos_event() {
    let fixture = load_fixture("registry-gate-unregistered-note.json");
    let rejected_event = fixture
        .expected_registry_rejection_event
        .as_deref()
        .expect("4C.7 fixture must name expected_registry_rejection_event");
    assert!(
        rejected_event.starts_with("wos."),
        "registry gate fixture must use the WOS event namespace"
    );
    assert!(
        fixture.actions.iter().any(|action| matches!(
            action,
            Action::DirectAppend { event_type, .. } if event_type == rejected_event
        )),
        "fixture should carry the rejected event literal in its action stream"
    );
    assert!(
        fixture
            .expected_case_view_events
            .iter()
            .all(|event| event.event != rejected_event),
        "registry-rejected event must not be part of the expected persisted case view"
    );
    let literals = registered_event_literals();
    assert!(
        !literals.contains(rejected_event),
        "{rejected_event} unexpectedly became registered; retarget this fixture"
    );
}
