// Rust guideline compliant 2026-02-21
//! Direct native admission shares the graph and refuses changed installed inputs.
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use wos_core::instance::HumanActorBinding;
use wos_core::model::notice_requirements::NoticeCapability;
use wos_core::traits::{DefaultRuntime, DocumentResolver, ExternalService};
use wos_core::{GovernanceDocument, KernelDocument};
use wos_runtime::governance::{GovernancePolicySelection, QualifiedGovernancePolicy};
use wos_runtime::store::RuntimeStore;
use wos_runtime::{
    Clock, CreateProcessRequest, SharedInMemoryStore, WosRuntime,
    restate_signature_fixture_bindings,
};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../wos-core/tests/fixtures/notice_policy.json"
    ))
    .unwrap()
}
fn policy(raw: Value) -> Result<QualifiedGovernancePolicy, wos_runtime::RuntimeError> {
    QualifiedGovernancePolicy::qualify(
        serde_json::from_value::<GovernancePolicySelection>(raw).unwrap(),
    )
}
#[derive(Clone)]
struct Resolver(KernelDocument);
impl DocumentResolver for Resolver {
    type Error = std::io::Error;
    fn resolve_kernel(&self, _: &str, _: &str) -> Result<KernelDocument, Self::Error> {
        Ok(self.0.clone())
    }
    fn resolve_governance(&self, _: &str, _: &str) -> Result<GovernanceDocument, Self::Error> {
        Err(std::io::Error::other("not selected"))
    }
    fn resolve_sidecar(&self, _: &str, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("not selected"))
    }
}
struct Now;
impl Clock for Now {
    fn now_ms(&self) -> u64 {
        1_800_000_000_000
    }
}
struct Effect(Arc<AtomicUsize>);
impl ExternalService for Effect {
    type Error = std::io::Error;
    fn invoke(&self, _: &str, _: &Value, _: Option<&str>) -> Result<Value, Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    }
}
fn rig(raw: &Value) -> (WosRuntime, SharedInMemoryStore, Arc<AtomicUsize>) {
    let store = SharedInMemoryStore::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = WosRuntime::new(
        store.clone(),
        Resolver(serde_json::from_value(raw["kernel"].clone()).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(calls.clone()),
        DefaultRuntime::new(),
        Now,
        restate_signature_fixture_bindings(),
    );
    (runtime, store, calls)
}
fn create(
    runtime: &mut WosRuntime,
    id: &str,
) -> Result<wos_core::instance::WorkflowProcess, wos_runtime::RuntimeError> {
    runtime.create_process_with_actor_bindings(
        CreateProcessRequest {
            process_id: id.into(),
            tenant: None,
            definition_url: "urn:test:notice-policy".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(json!({"photo":"retained"})),
        },
        ["applicant", "reviewer", "appealReviewer"]
            .iter()
            .map(|name| HumanActorBinding {
                declared_actor: (*name).into(),
                assigned_actor: format!("actor:human:{name}"),
            })
            .collect(),
    )
}
#[test]
fn direct_native_qualifies_real_original_inputs_without_lint_preflight() {
    let raw = fixture();
    assert!(policy(raw).is_ok());
}
#[test]
fn direct_native_broken_wait_refuses_without_lint_preflight() {
    let mut raw = fixture();
    raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
    let error = policy(raw).unwrap_err().to_string();
    assert!(error.contains("NOTICE-GRAPH-001"), "{error}");
}
#[test]
fn actual_installed_profile_is_checked_before_initialization_or_persistence() {
    for mode in [
        "missing",
        "changed",
        "foreign",
        "valid-before",
        "valid-after",
    ] {
        let raw = fixture();
        let selected = policy(raw.clone()).unwrap();
        let (mut runtime, store, calls) = rig(&raw);
        let mut integration = raw["integration"].clone();
        if mode == "changed" {
            integration["bindings"]["request"]["outputBinding"] =
                json!({"caseFile.foreign":"$.sentAt"});
        }
        if mode == "foreign" {
            integration["targetWorkflow"]["url"] = json!("urn:foreign");
        }
        if mode == "valid-before" {
            runtime = runtime
                .with_integration_profile(serde_json::from_value(integration.clone()).unwrap());
        }
        runtime = runtime
            .with_governance_policy(
                selected,
                Default::default(),
                &[NoticeCapability::LocalSmtpTestCapture],
            )
            .unwrap();
        if mode != "missing" && mode != "valid-before" {
            runtime =
                runtime.with_integration_profile(serde_json::from_value(integration).unwrap());
        }
        let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
        let result = create(&mut runtime, &id);
        if mode.starts_with("valid") {
            assert!(result.is_ok(), "{mode}: {result:?}");
            assert!(store.load_record(&id).is_ok());
        } else {
            assert!(result.is_err(), "{mode}");
            assert!(store.load_record(&id).is_err(), "{mode} left a case");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{mode}");
    }
}
#[test]
fn installed_capabilities_do_not_downgrade_selected_methods() {
    for method in ["providerDelivered", "openObserved", "recipientAcknowledged"] {
        let mut raw = fixture();
        raw["rules"]["noticeRequirements"]["request"]["observation"] = json!(method);
        let selected = policy(raw.clone()).unwrap();
        let (runtime, _store, calls) = rig(&raw);
        let error = runtime
            .with_governance_policy(
                selected,
                Default::default(),
                &[NoticeCapability::LocalSmtpTestCapture],
            )
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains("NOTICE-004")
                && error.contains("/rules/noticeRequirements/request/observation"),
            "{error}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn direct_native_after_own_wait_producer_requires_the_graph_gate() {
    let mut raw = fixture();
    let action = raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"].take();
    raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
    raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"]
        .as_array_mut()
        .unwrap()
        .extend(action.as_array().unwrap().iter().cloned());
    let error = policy(raw).unwrap_err().to_string();
    assert!(error.contains("NOTICE-GRAPH-006"), "{error}");
}
#[test]
fn replacement_before_drain_or_task_submission_preserves_the_entire_retained_record() {
    let raw = fixture();
    let selected = policy(raw.clone()).unwrap();
    let (runtime, store, calls) = rig(&raw);
    let mut runtime = runtime
        .with_integration_profile(serde_json::from_value(raw["integration"].clone()).unwrap())
        .with_governance_policy(
            selected,
            Default::default(),
            &[NoticeCapability::LocalSmtpTestCapture],
        )
        .unwrap();
    let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    create(&mut runtime, &id).unwrap();
    let prior = store.load_record(&id).unwrap();
    let subject = prior
        .process
        .pending_callbacks
        .keys()
        .next()
        .unwrap()
        .clone();
    let now = time::OffsetDateTime::from_unix_timestamp(1_800_000_000)
        .unwrap()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    runtime.enqueue_event(&id,wos_core::instance::PendingEvent{origin:None,event:"request.accepted".into(),actor_id:None,data:Some(json!({"id":"actual-native-request-receipt","source":"urn:test:notice-provider","specversion":"1.0","type":"urn:test:request.accepted","subject":subject,"data":{"sentAt":now}})),timestamp:now.clone(),idempotency_token:None}).unwrap();
    runtime.drain_once(&id).unwrap();
    runtime.drain_once(&id).unwrap();
    runtime
        .enqueue_event(
            &id,
            wos_core::instance::PendingEvent {
                origin: None,
                event: "correction.completed".into(),
                actor_id: None,
                data: None,
                timestamp: now,
                idempotency_token: None,
            },
        )
        .unwrap();
    let original = store.load_record(&id).unwrap();
    let task = original.process.active_tasks[0].task_id.clone();
    let before = snapshot(&original);
    let mut changed = raw["integration"].clone();
    changed["bindings"]["request"]["outputBinding"] = json!({"caseFile.foreign":"$.sentAt"});
    runtime = runtime.with_integration_profile(serde_json::from_value(changed).unwrap());
    let drain = runtime.drain_once(&id).unwrap_err().to_string();
    assert!(
        drain.contains("selected governance owner differs"),
        "{drain}"
    );
    let submit = runtime
        .submit_task_response(
            &task,
            json!({"status":"completed","data":{}}),
            "actor:human:applicant",
            Some("changed-profile"),
        )
        .unwrap_err()
        .to_string();
    assert!(
        submit.contains("selected governance owner differs"),
        "{submit}"
    );
    assert_eq!(snapshot(&store.load_record(&id).unwrap()), before);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn snapshot(record: &wos_runtime::RuntimeRecord) -> Value {
    json!({"process":record.process,"provenance":record.provenance_log,"aux":wos_runtime::runtime_aux_to_json(record)})
}

#[test]
fn nested_clock_direct_native_checks_whole_callback_and_missing_rules() {
    for missing_rules in [false, true] {
        let mut raw = fixture();
        raw["integration"]["bindings"]["request"]["outputBinding"] =
            json!({"caseFile.requestNotice":"$"});
        raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
            json!("caseFile.requestNotice.clock.governedSentAt");
        if missing_rules {
            raw["rules"]["timeoutDecisions"] = json!([]);
            raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
        }
        let result = policy(raw);
        assert_eq!(
            result.is_err(),
            missing_rules,
            "missing_rules={missing_rules}: {result:?}"
        );
        if let Err(error) = result {
            assert!(
                error
                    .to_string()
                    .contains("/noticeRequirements/request/anchor"),
                "{error}"
            );
        }
    }
}
