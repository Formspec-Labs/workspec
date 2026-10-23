// Rust guideline compliant 2026-02-21

//! Conformance tests for AI capability preconditions and agent invocation (AI §3.3.1, ADR 0064, fs-w9dv).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use serde_json::Value;
use wos_agent_stub::StubInvoker;
use wos_core::agent::{AgentTask, InvokerKind};
use wos_core::traits::{DefaultRuntime, DocumentResolver, ExternalService};
use wos_core::{KernelDocument, ProvenanceKind};
use wos_runtime::{
    Clock, CreateProcessRequest, SharedInMemoryStore, WosRuntime,
    restate_signature_fixture_bindings,
};

#[derive(Clone)]
struct StaticResolver(KernelDocument);

impl DocumentResolver for StaticResolver {
    type Error = std::io::Error;
    fn resolve_kernel(&self, _: &str, _: &str) -> Result<KernelDocument, Self::Error> {
        Ok(self.0.clone())
    }
    fn resolve_governance(&self, _: &str, _: &str) -> Result<wos_core::GovernanceDocument, Self::Error> {
        Err(std::io::Error::other("not selected"))
    }
    fn resolve_sidecar(&self, _: &str, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("not selected"))
    }
}

struct FixedClock;
impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        1_800_000_000_000
    }
}

struct ServiceTracker(Arc<AtomicUsize>);
impl ExternalService for ServiceTracker {
    type Error = std::io::Error;
    fn invoke(&self, _: &str, _: &Value, _: Option<&str>) -> Result<Value, Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    }
}

fn load_fixture(name: &str) -> (Value, KernelDocument) {
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/conformance/preconditions");
    let path = base_dir.join(name);
    let raw: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture at {:?}: {}", path, e)),
    )
    .unwrap();
    let kernel: KernelDocument = serde_json::from_value(raw.clone()).unwrap();
    (raw, kernel)
}

fn create_test_runtime(kernel: KernelDocument) -> (WosRuntime, SharedInMemoryStore) {
    let store = SharedInMemoryStore::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let stub = StubInvoker::new();
    let runtime = WosRuntime::new(
        store.clone(),
        StaticResolver(kernel),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        ServiceTracker(calls),
        DefaultRuntime::new(),
        FixedClock,
        restate_signature_fixture_bindings(),
    )
    .with_agent_invoker(InvokerKind::Stub, Box::new(stub));

    (runtime, store)
}

#[test]
fn test_programmatic_invoke_agent_permitted() {
    let (_raw, kernel) = load_fixture("preconditions-permitted.json");
    let (mut runtime, _store) = create_test_runtime(kernel);

    let process_id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "https://example.com/workflows/preconditions-permitted".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({
                "claimAmount": 1500,
                "applicantVerified": true
            })),
        })
        .expect("process creation must succeed");

    let task = AgentTask {
        capability_id: "evaluateClaim".into(),
        prompt: None,
        inputs: Default::default(),
        correlation_key: None,
    };

    let decision = runtime
        .invoke_agent(&process_id, "adjudicator", "evaluateClaim", task)
        .expect("invoke_agent must succeed");

    assert!(!decision.invocation_blocked);
    assert_eq!(decision.outcome.as_deref(), Some("completed"));
    let res = decision.result.expect("result must be present when permitted");
    assert_eq!(res.output["eligible"], true);
    assert_eq!(res.output["benefitTier"], "TierA");

    // Inspect provenance log
    let provenance = runtime.load_provenance_window(&process_id, 0, 100).unwrap();
    let cap_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::CapabilityInvocation)
        .collect();
    assert_eq!(cap_records.len(), 1);
    let rec = cap_records[0];
    assert_eq!(rec.actor_id.as_deref(), Some("adjudicator"));
    assert_eq!(rec.outcome, None);
    let data = rec.data.as_ref().unwrap();
    assert_eq!(data["capabilityId"], "evaluateClaim");
    assert_eq!(data["invocationBlocked"], false);
}

#[test]
fn test_programmatic_invoke_agent_blocked_by_precondition() {
    let (_raw, kernel) = load_fixture("preconditions-blocked.json");
    let (mut runtime, _store) = create_test_runtime(kernel);

    let process_id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "https://example.com/workflows/preconditions-blocked".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({
                "claimAmount": 0, // Violates: claimAmount > 0
                "applicantVerified": true
            })),
        })
        .expect("process creation must succeed");

    let task = AgentTask {
        capability_id: "evaluateClaim".into(),
        prompt: None,
        inputs: Default::default(),
        correlation_key: None,
    };

    let decision = runtime
        .invoke_agent(&process_id, "adjudicator", "evaluateClaim", task)
        .expect("invoke_agent must succeed with blocked decision");

    assert!(decision.invocation_blocked);
    assert_eq!(
        decision.outcome.as_deref(),
        Some("preconditionNotSatisfied")
    );
    assert!(decision.result.is_none(), "agent must not be invoked when blocked");

    // Inspect provenance log
    let provenance = runtime.load_provenance_window(&process_id, 0, 100).unwrap();
    let cap_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::CapabilityInvocation)
        .collect();
    assert_eq!(cap_records.len(), 1);
    let rec = cap_records[0];
    assert_eq!(rec.actor_id.as_deref(), Some("adjudicator"));
    assert_eq!(rec.outcome.as_deref(), Some("preconditionNotSatisfied"));
    let data = rec.data.as_ref().unwrap();
    assert_eq!(data["capabilityId"], "evaluateClaim");
    assert_eq!(data["invocationBlocked"], true);
    assert_eq!(data["failedPrecondition"], "caseFile.claimAmount > 0");
}

#[test]
fn test_transition_action_service_invocation_permitted_path() {
    let (_raw, kernel) = load_fixture("preconditions-permitted.json");
    let (mut runtime, _store) = create_test_runtime(kernel);

    let process_id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    let _proc = runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "https://example.com/workflows/preconditions-permitted".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({
                "claimAmount": 5000,
                "applicantVerified": true
            })),
        })
        .expect("process creation must succeed");

    // Transition from draft to assess, which has onEntry invokeService action
    let _ = runtime.enqueue_event(
        &process_id,
        wos_core::instance::PendingEvent {
            event: "startAssessment".into(),
            actor_id: None,
            data: None,
            idempotency_token: None,
            timestamp: "2026-02-21T00:00:00Z".into(),
            origin: None,
        },
    );
    runtime.drain_once(&process_id).expect("drain_once must succeed");

    // Check that capability was evaluated as permitted and step result persisted
    let provenance = runtime.load_provenance_window(&process_id, 0, 100).unwrap();
    let cap_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::CapabilityInvocation)
        .collect();
    assert_eq!(cap_records.len(), 1);
    assert_eq!(cap_records[0].outcome, None);
    assert_eq!(
        cap_records[0].data.as_ref().unwrap()["invocationBlocked"],
        false
    );

    let step_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::StepResultPersisted)
        .collect();
    assert_eq!(step_records.len(), 1);
    let step_data = step_records[0].data.as_ref().unwrap();
    assert_eq!(step_data["output"]["eligible"], true);
}

#[test]
fn test_transition_action_service_invocation_blocked_path() {
    let (_raw, kernel) = load_fixture("preconditions-blocked.json");
    let (mut runtime, _store) = create_test_runtime(kernel);

    let process_id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    let _proc = runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "https://example.com/workflows/preconditions-blocked".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({
                "claimAmount": 5000,
                "applicantVerified": false // Violates: applicantVerified == true
            })),
        })
        .expect("process creation must succeed");

    // Transition from draft to assess, which has onEntry invokeService action
    let _ = runtime.enqueue_event(
        &process_id,
        wos_core::instance::PendingEvent {
            event: "startAssessment".into(),
            actor_id: None,
            data: None,
            idempotency_token: None,
            timestamp: "2026-02-21T00:00:00Z".into(),
            origin: None,
        },
    );
    runtime.drain_once(&process_id).expect("drain_once must succeed");

    // Preconditions blocked invocation:
    let provenance = runtime.load_provenance_window(&process_id, 0, 100).unwrap();
    let cap_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::CapabilityInvocation)
        .collect();
    assert_eq!(cap_records.len(), 1);
    assert_eq!(
        cap_records[0].outcome.as_deref(),
        Some("preconditionNotSatisfied")
    );
    assert_eq!(
        cap_records[0].data.as_ref().unwrap()["invocationBlocked"],
        true
    );

    // No StepResultPersisted because agent was not called
    let step_records: Vec<_> = provenance
        .iter()
        .filter(|r| r.record_kind == ProvenanceKind::StepResultPersisted)
        .collect();
    assert_eq!(step_records.len(), 0);
}
