// Rust guideline compliant 2026-02-21

//! Exercises complete D1 lifecycle requirements with controlled runtime ports.
//!
//! These tests use canned check/delivery outputs and selected Mapping outputs.
//! They do not qualify production Definitions, delivery, protected response fields,
//! processing-pause accounting, host authorization, or offline export verification.

use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use wos_core::instance::WorkflowProcess;
use wos_core::model::{
    governance::{DelegationScope, GovernanceDocument},
    kernel::KernelDocument,
};
use wos_core::traits::{AccessControl, DefaultRuntime, DocumentResolver, ExternalService};
use wos_formspec_binding::{FormspecBinding, FormspecProcessor};
use wos_runtime::{
    BindingError, BindingRegistry, CaseMutationBundle, Clock, CreateProcessRequest,
    HumanActorBinding, IntegrationProfileDocument, RuntimeStore, SharedInMemoryStore,
    TaskSubmissionResult, WosRuntime,
};

const WORKFLOW: &str = include_str!("../../../examples/d1-passport-complete.workflow.json");
const INTEGRATION: &str = include_str!("../../../examples/d1-passport-complete.integration.json");
const REVIEW: &str = "urn:formspec:demo:d1:passport-review";
const APPLICATION: &str = "urn:formspec:demo:d1:passport-renewal";
const REVIEWER: &str = "actor:human:reviewer";
const APPLICANT: &str = "actor:human:applicant";
const DAY: u64 = 86_400_000;
const START: u64 = 1_710_000_000_000;

struct Resolver(KernelDocument);
impl DocumentResolver for Resolver {
    type Error = std::io::Error;
    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        if self.0.url.as_deref() == Some(url) && self.0.version.as_deref() == Some(version) {
            Ok(self.0.clone())
        } else {
            Err(std::io::Error::other("unselected workflow"))
        }
    }
    fn resolve_governance(&self, _: &str, _: &str) -> Result<GovernanceDocument, Self::Error> {
        Err(std::io::Error::other("no separate governance document"))
    }
    fn resolve_sidecar(&self, _: &str, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("no sidecar"))
    }
}
struct AssignmentOnly;
impl AccessControl for AssignmentOnly {
    fn can_transition(&self, _: &str, _: &str) -> bool {
        true
    }
    fn can_read(&self, _: &str, _: &str) -> bool {
        true
    }
    fn can_delegate(&self, _: &str, _: &str, _: &DelegationScope) -> bool {
        false
    }
}
#[derive(Clone)]
struct ManualClock(Arc<AtomicU64>);
impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn iso(ms: u64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap()
        .format(&Rfc3339)
        .unwrap()
}
#[derive(Clone)]
struct ControlledPorts {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    photo_qualifying: bool,
    sent_at: u64,
    adverse: bool,
    fail_no_response: Arc<AtomicBool>,
}
impl ExternalService for ControlledPorts {
    type Error = std::io::Error;
    fn invoke(
        &self,
        service_ref: &str,
        input: &Value,
        _: Option<&str>,
    ) -> Result<Value, Self::Error> {
        self.calls
            .lock()
            .unwrap()
            .push((service_ref.into(), input.clone()));
        if service_ref == "sendNoResponseNotice" && self.fail_no_response.load(Ordering::SeqCst) {
            return Err(std::io::Error::other(
                "controlled notice transport unavailable",
            ));
        }
        match service_ref {
            "mockPriorPassportCheck" | "mockSsnCheck" => Ok(json!({
                "source":service_ref,"checkedAt":iso(START),"adverse":self.adverse,
                "result":"controlled test result","availability":"controlled","confirmation":"unconfirmed","testDouble":true})),
            "buildOfflineVerificationBundle" => Ok(json!({"qualified":false,
                "reason":"Controlled port does not build or verify an export bundle."})),
            name if name.starts_with("send") => Ok(json!({"qualifying":
                if name == "sendPhotoRequestNotice" {self.photo_qualifying} else {true},
                "sentAt":iso(self.sent_at),"source":name,"testDouble":true})),
            _ => Err(std::io::Error::other("unselected service")),
        }
    }
}
struct SelectedMappings;
impl FormspecProcessor for SelectedMappings {
    fn validate_envelope(&self, _: &Value) -> Result<Vec<Value>, BindingError> {
        Ok(vec![])
    }
    fn validate_definition(
        &self,
        _: &str,
        _: &str,
        _: &Value,
    ) -> Result<Option<Vec<Value>>, BindingError> {
        Ok(None)
    }
    fn compute_prefill(
        &self,
        mapping: Option<&str>,
        case: &Value,
    ) -> Result<Option<Value>, BindingError> {
        match mapping {
            Some("urn:formspec:demo:d1:passport-renewal:mapping:correction-prefill") => {
                let mut fields = case["originalAnswers"]
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                fields.insert(
                    "originalFilingRef".into(),
                    case["originalFilingRef"].clone(),
                );
                Ok(Some(Value::Object(fields)))
            }
            Some("urn:formspec:demo:d1:passport-review:mapping:prefill")
            | Some("urn:formspec:demo:d1:passport-outside-data-dispute:mapping:prefill") => {
                Ok(Some(case.clone()))
            }
            None => Ok(None),
            Some(_) => Err(BindingError::InvalidInput(
                "unselected prefill Mapping".into(),
            )),
        }
    }
    fn map_response(
        &self,
        mapping: &str,
        response: &Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let updates: Vec<(String, Value)> = match mapping {
            "urn:formspec:demo:d1:passport-review:mapping:case" => vec![
                ("reviewChoice".into(), response["data"]["choice"].clone()),
                ("reviewReason".into(), response["data"]["reason"].clone()),
                (
                    "photoRuleReason".into(),
                    response["data"]["photoRuleReason"].clone(),
                ),
                (
                    "requestedItems".into(),
                    response["data"]["requestedItems"].clone(),
                ),
            ],
            "urn:formspec:demo:d1:passport-renewal:mapping:correction-case" => vec![
                ("correctionPhoto".into(), response["data"]["photo"].clone()),
                (
                    "correctionFilingRef".into(),
                    response["data"]["originalFilingRef"].clone(),
                ),
            ],
            "urn:formspec:demo:d1:passport-outside-data-dispute:mapping:case" => [
                ("checkRef", "outsideCheckRef"),
                ("response", "outsideCheckResponse"),
                ("reason", "outsideCheckReason"),
                ("evidence", "outsideCheckEvidence"),
            ]
            .into_iter()
            .map(|(source, target)| (target.into(), response["data"][source].clone()))
            .collect(),
            _ => {
                return Err(BindingError::InvalidInput(
                    "unselected response Mapping".into(),
                ));
            }
        };
        Ok(Some(CaseMutationBundle {
            field_updates: updates.into_iter().collect(),
        }))
    }
}
// This task-specific override selects an immutable diagnostic baseline for a
// filtered red/green run. It is test harness input, not runtime configuration.
fn selected_workflow() -> KernelDocument {
    let authored: Value = serde_json::from_str(WORKFLOW).unwrap();
    let selected = match std::env::var_os("D1_CORE_TEST_WORKFLOW_PATH") {
        Some(path) => {
            let bytes = std::fs::read(path).expect("diagnostic workflow must be readable");
            let candidate: Value =
                serde_json::from_slice(&bytes).expect("diagnostic workflow must be JSON");
            for key in ["url", "version", "actors"] {
                assert_eq!(
                    candidate[key], authored[key],
                    "diagnostic workflow must retain {key}"
                );
            }
            candidate
        }
        None => authored,
    };
    serde_json::from_value(selected).expect("selected workflow must be a native KernelDocument")
}

fn controlled_runtime(
    store: SharedInMemoryStore,
    clock: ManualClock,
    ports: ControlledPorts,
) -> WosRuntime {
    let kernel = selected_workflow();
    let profile: IntegrationProfileDocument = serde_json::from_str(INTEGRATION).unwrap();
    controlled_runtime_with_artifacts(store, clock, ports, kernel, profile)
}

fn controlled_runtime_with_artifacts(
    store: SharedInMemoryStore,
    clock: ManualClock,
    ports: ControlledPorts,
    kernel: KernelDocument,
    profile: IntegrationProfileDocument,
) -> WosRuntime {
    let mut bindings = BindingRegistry::new();
    bindings.register(FormspecBinding::new(SelectedMappings));
    WosRuntime::new(
        store.clone(),
        Resolver(kernel),
        DefaultRuntime::new(),
        AssignmentOnly,
        ports.clone(),
        DefaultRuntime::new(),
        clock.clone(),
        bindings,
    )
    .with_integration_profile(profile)
}

// These synthetic inputs passed independent native Definition and Mapping probes.
// SelectedMappings still does not implement native Definition validation or
// enforce protected original-answer admission.
fn native_deferred_inputs() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/d1/deferred-native-valid-inputs.json"
    ))
    .unwrap()
}

struct Fixture {
    runtime: WosRuntime,
    store: SharedInMemoryStore,
    process: WorkflowProcess,
    clock: ManualClock,
    ports: ControlledPorts,
    deferred: bool,
}
impl Fixture {
    fn new(photo_qualifying: bool, sent_at: u64, adverse: bool) -> Self {
        Self::build(photo_qualifying, sent_at, adverse, false)
    }
    fn deferred(sent_at: u64) -> Self {
        Self::build(true, sent_at, false, true)
    }
    fn build(photo_qualifying: bool, sent_at: u64, adverse: bool, deferred: bool) -> Self {
        let store = SharedInMemoryStore::default();
        let clock = ManualClock(Arc::new(AtomicU64::new(START)));
        let ports = ControlledPorts {
            calls: Arc::new(Mutex::new(vec![])),
            photo_qualifying,
            sent_at,
            adverse,
            fail_no_response: Arc::new(AtomicBool::new(false)),
        };
        let mut runtime = if deferred {
            controlled_runtime_with_artifacts(
                store.clone(),
                clock.clone(),
                ports.clone(),
                serde_json::from_str(include_str!(
                    "../../../examples/d1-passport-complete-deferred.workflow.json"
                ))
                .unwrap(),
                serde_json::from_str(include_str!(
                    "../../../examples/d1-passport-complete-deferred.integration.json"
                ))
                .unwrap(),
            )
        } else {
            controlled_runtime(store.clone(), clock.clone(), ports.clone())
        };
        let initial_case_state = if deferred {
            let original_answers = native_deferred_inputs()["initial"]["input"].clone();
            json!({"originalFilingRef":"urn:formspec:fixture:original-filing",
                "originalPhoto":original_answers["photo"], "originalAnswers":original_answers,
                "referenceNumber":"D1-TEST", "preferredLanguage":"en", "caseClosed":false})
        } else {
            json!({"originalFilingRef":"filing-original",
                "originalAnswers":{"fullName":"Alex Rivera","photo":{"digest":"original-photo"}},
                "originalPhoto":{"digest":"original-photo"},
                "referenceNumber":"D1-TEST","preferredLanguage":"en-US","caseClosed":false})
        };
        let process = runtime
            .create_process_with_actor_bindings(
                CreateProcessRequest {
                    process_id: stack_common_typeid::mint_process_id(),
                    tenant: None,
                    definition_url: "urn:wos:demo:d1:passport-complete".into(),
                    definition_version: "0.3.0".into(),
                    initial_case_state: Some(initial_case_state),
                },
                vec![
                    HumanActorBinding {
                        declared_actor: "adjudicator".into(),
                        assigned_actor: REVIEWER.into(),
                    },
                    HumanActorBinding {
                        declared_actor: "applicant".into(),
                        assigned_actor: APPLICANT.into(),
                    },
                ],
            )
            .unwrap();
        runtime.drain_until_idle(&process.process_id).unwrap();
        Self {
            runtime,
            store,
            process,
            clock,
            ports,
            deferred,
        }
    }
    fn current(&self) -> WorkflowProcess {
        self.runtime.load_process(&self.process.process_id).unwrap()
    }
    fn state(&self, expected: &str) {
        assert_eq!(self.current().configuration, vec![expected]);
    }
    fn submit(&mut self, task_ref: &str, actor: &str, url: &str, data: Value, token: &str) {
        let task = self
            .current()
            .active_tasks
            .into_iter()
            .find(|t| t.task_ref == task_ref)
            .unwrap();
        assert_eq!(task.definition_url.as_deref(), Some(url));
        assert_eq!(task.definition_version.as_deref(), Some("0.3.0"));
        let result = self
            .runtime
            .submit_task_response(
                &task.task_id,
                json!({"status":"completed",
            "definitionUrl":url,"definitionVersion":"0.3.0","data":data}),
                actor,
                Some(token),
            )
            .unwrap();
        assert!(matches!(result, TaskSubmissionResult::Completed { .. }));
        self.runtime
            .drain_until_idle(&self.process.process_id)
            .unwrap();
    }
    fn review(&mut self, choice: &str, reason: &str, token: &str) {
        self.submit(
            "reviewApplication",
            REVIEWER,
            REVIEW,
            json!({"choice":choice,"reason":reason}),
            token,
        );
    }
    fn send_back(&mut self) {
        if self.deferred {
            self.submit(
                "reviewApplication",
                REVIEWER,
                REVIEW,
                native_deferred_inputs()["sendBack"]["input"].clone(),
                "sendback",
            );
        } else {
            self.review(
                "sendBackPhoto",
                "Photo does not show a clear face.",
                "sendback",
            );
        }
    }
    fn advance(&mut self, days: u64) {
        self.clock.0.store(START + days * DAY, Ordering::SeqCst);
        self.runtime
            .drain_until_idle(&self.process.process_id)
            .unwrap();
    }
    fn late_reply(&mut self) {
        let data = if self.deferred {
            native_deferred_inputs()["correction"]["input"].clone()
        } else {
            json!({"photo":{"digest":"late-photo"},"originalFilingRef":"filing-original"})
        };
        self.submit("attachLatePhoto", APPLICANT, APPLICATION, data, "late");
    }
}

#[test]
fn automatic_checks_map_distinct_sources_times_and_persist_results() {
    let fixture = Fixture::new(true, START, false);
    fixture.state("awaitingReview");
    let current = fixture.current();
    for (field, service) in [
        ("priorPassportCheck", "mockPriorPassportCheck"),
        ("ssnCheck", "mockSsnCheck"),
    ] {
        assert_eq!(current.case_state[field]["source"], service);
        assert_eq!(current.case_state[field]["checkedAt"], iso(START));
        let record = fixture
            .store
            .0
            .lock()
            .unwrap()
            .load_record(&current.process_id)
            .unwrap();
        assert!(
            record
                .step_results
                .iter()
                .any(|result| result.service_ref == service)
        );
        assert!(record.provenance_log.iter().any(|p| {
            p.record_kind == wos_core::ProvenanceKind::StepResultPersisted
                && p.data
                    .as_ref()
                    .is_some_and(|data| data["serviceRef"] == service)
        }));
    }
    assert_eq!(
        fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name.starts_with("mock"))
            .count(),
        2
    );
}

#[test]
fn qualifying_sendback_same_form_photo_returns_new_review_then_reasoned_approval() {
    let mut fixture = Fixture::new(true, START, false);
    let initial_review_id = fixture.current().active_tasks[0].task_id.clone();
    fixture.send_back();
    fixture.state("awaitingPhoto");
    let waiting = fixture.current();
    assert_eq!(waiting.timers.len(), 1);
    assert_eq!(waiting.timers[0].duration_iso.as_deref(), Some("P90D"));
    assert_eq!(waiting.case_state["photoRequestNotice"]["qualifying"], true);
    let task = waiting
        .active_tasks
        .iter()
        .find(|task| task.task_ref == "replacePhoto")
        .unwrap();
    let prefill = task
        .context
        .as_ref()
        .unwrap()
        .prefill_data
        .as_ref()
        .unwrap();
    assert_eq!(prefill["fullName"], "Alex Rivera");
    assert_eq!(prefill["photo"]["digest"], "original-photo");
    assert_eq!(prefill["originalFilingRef"], "filing-original");
    fixture.submit(
        "replacePhoto",
        APPLICANT,
        APPLICATION,
        json!({"photo":{"digest":"new-photo"},"originalFilingRef":"filing-original",
            "fullName":"Alex Rivera"}),
        "photo",
    );
    fixture.state("returnedReview");
    let returned = fixture.current();
    assert!(returned.timers.is_empty());
    let returned_task = returned
        .active_tasks
        .iter()
        .find(|t| t.task_ref == "reviewApplication")
        .unwrap();
    assert_ne!(returned_task.task_id, initial_review_id);
    assert_eq!(
        returned.case_state["originalAnswers"],
        fixture.process.case_state["originalAnswers"]
    );
    assert_eq!(
        returned.case_state["originalPhoto"],
        fixture.process.case_state["originalPhoto"]
    );
    assert_eq!(
        returned.case_state["correctionPhoto"]["digest"],
        "new-photo"
    );
    fixture.review(
        "approve",
        "The replacement photo and original application satisfy the review.",
        "approve",
    );
    fixture.state("approved");
    assert_eq!(
        fixture.current().case_state["reviewReason"],
        "The replacement photo and original application satisfy the review."
    );
}

#[test]
fn reasoned_denial_uses_qualifying_decision_receipt() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.review(
        "deny",
        "The retained evidence does not establish eligibility.",
        "deny",
    );
    fixture.state("denied");
    assert_eq!(
        fixture.current().case_state["decisionNotice"]["qualifying"],
        true
    );
    assert_eq!(
        fixture.current().case_state["reviewReason"],
        "The retained evidence does not establish eligibility."
    );
}

#[test]
fn queued_nonqualifying_photo_notice_creates_no_response_clock_or_task() {
    let mut fixture = Fixture::new(false, START, false);
    fixture.send_back();
    fixture.state("photoNoticePending");
    assert!(fixture.current().timers.is_empty());
    assert!(
        !fixture
            .current()
            .active_tasks
            .iter()
            .any(|t| t.task_ref == "replacePhoto")
    );
    fixture.advance(91);
    fixture.state("photoNoticePending");
}

#[test]
fn expired_photo_clock_closes_late_reply_stays_closed_and_reasoned_adjudicator_reopens() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    fixture.advance(91);
    fixture.state("closedForNoResponse");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    fixture.late_reply();
    fixture.state("closedWithLateEvidence");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    let task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|t| t.task_ref == "considerReopening")
        .unwrap();
    assert!(fixture.runtime.submit_task_response(&task.task_id, json!({"status":"completed",
        "definitionUrl":REVIEW,"definitionVersion":"0.3.0","data":{"choice":"reopen","reason":"New photo changes the decision."}}),
        APPLICANT, Some("unauthorized-reopen")).is_err());
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":"New photo changes the decision."}),
        "reopen",
    );
    fixture.state("returnedReview");
    assert_eq!(fixture.current().case_state["caseClosed"], false);
}

#[test]
fn empty_reopening_reason_keeps_case_closed() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    fixture.advance(91);
    fixture.late_reply();
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":""}),
        "empty-reason",
    );
    fixture.state("closedWithLateEvidence");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
}

#[test]
fn delayed_qualifying_receipt_clock_expires_ninety_days_after_sent_at() {
    // Requirement assertion: recording a receipt two days after sending must not
    // extend the applicant's deadline by two days. Current duration-only timers
    // may fail this assertion; preserve the failure rather than changing it.
    let sent_at = START - 2 * DAY;
    let mut fixture = Fixture::new(true, sent_at, false);
    fixture.send_back();
    let timer = fixture
        .current()
        .timers
        .into_iter()
        .find(|t| t.timer_id == "photoResponse")
        .unwrap();
    let deadline = OffsetDateTime::parse(&timer.deadline, &Rfc3339)
        .unwrap()
        .unix_timestamp_nanos()
        / 1_000_000;
    assert_eq!(
        deadline,
        i128::from(sent_at + 90 * DAY),
        "D1 clock must start at qualifying sentAt, not receipt processing time"
    );
}

#[test]
fn adverse_mock_checks_allow_ten_day_dispute_before_assigned_reasoned_review() {
    let mut fixture = Fixture::new(true, START, true);
    fixture.state("outsideDataDisputeWindow");
    let current = fixture.current();
    assert_eq!(current.case_state["outsideDataNotice"]["qualifying"], true);
    assert_eq!(current.timers.len(), 1);
    assert_eq!(current.timers[0].duration_iso.as_deref(), Some("P10D"));
    assert!(
        current
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "disputeOutsideData")
    );
    assert!(
        !current
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "reviewApplication")
    );
    assert_eq!(
        fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name.starts_with("mock"))
            .count(),
        2
    );
    fixture.advance(9);
    fixture.state("outsideDataDisputeWindow");
    fixture.submit(
        "disputeOutsideData",
        APPLICANT,
        "urn:formspec:demo:d1:passport-outside-data-dispute",
        json!({"checkRef":"mockPriorPassportCheck","response":"dispute","reason":"The mock prior passport result refers to a different person.","evidence":[]}),
        "dispute",
    );
    fixture.state("awaitingDisputeReview");
    assert!(fixture.current().timers.is_empty());
    let task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "reviewApplication")
        .unwrap();
    assert_eq!(task.assigned_actor.as_deref(), Some(REVIEWER));
    assert!(
        fixture
            .runtime
            .submit_task_response(
                &task.task_id,
                json!({"status":"completed",
        "definitionUrl":REVIEW,"definitionVersion":"0.3.0",
        "data":{"choice":"deny","reason":"Applicant cannot issue an adjudication."}}),
                APPLICANT,
                Some("wrong-dispute-reviewer")
            )
            .is_err()
    );
    // Acceptance ends the applicant response clock, but adverse use still
    // waits until the complete ten-day period from the qualifying notice.
    fixture.advance(10);
    fixture.review(
        "deny",
        "The retained dispute and application evidence do not establish eligibility.",
        "dispute-denial",
    );
    assert_recorded_denial(
        &fixture,
        "The retained dispute and application evidence do not establish eligibility.",
    );
}

#[test]
fn expiry_closes_case_even_when_no_response_notice_transport_fails() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    fixture.ports.fail_no_response.store(true, Ordering::SeqCst);
    fixture.clock.0.store(START + 91 * DAY, Ordering::SeqCst);
    let outcome = fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id);
    assert!(
        fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name == "sendNoResponseNotice")
    );
    // Inspect actual saved state despite the port error; do not replace delivery
    // with a fabricated receipt or inject a second transition manually.
    let persisted = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&fixture.process.process_id)
        .unwrap();
    assert_eq!(
        persisted.process.case_state["caseClosed"], true,
        "Expiry must persist business closure independently of notice transport; observed drain outcome: {outcome:?}"
    );
    assert!(persisted.process.configuration.iter().any(|state|
        state == "closedForNoResponse" || state == "noResponseClosedNoticePending"));
    assert!(
        persisted
            .process
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "attachLatePhoto")
    );
    assert!(
        !persisted.process.case_state["decisionNotice"]["qualifying"]
            .as_bool()
            .unwrap_or(false)
    );
}

#[test]
fn receipt_anchored_timer_survives_new_runtime_reload_and_expires_at_original_deadline() {
    let sent_at = START - 2 * DAY;
    let mut fixture = Fixture::new(true, sent_at, false);
    fixture.send_back();
    let before = fixture
        .current()
        .timers
        .into_iter()
        .find(|timer| timer.timer_id == "photoResponse")
        .unwrap();
    assert_eq!(before.created_at_ms, Some(sent_at));
    fixture.runtime = controlled_runtime(
        fixture.store.clone(),
        fixture.clock.clone(),
        fixture.ports.clone(),
    );
    let restored = fixture
        .current()
        .timers
        .into_iter()
        .find(|timer| timer.timer_id == "photoResponse")
        .unwrap();
    assert_eq!(restored.deadline, before.deadline);
    assert_eq!(restored.created_at_ms, before.created_at_ms);
    fixture
        .clock
        .0
        .store(sent_at + 90 * DAY - 1, Ordering::SeqCst);
    fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id)
        .unwrap();
    fixture.state("awaitingPhoto");
    fixture.clock.0.store(sent_at + 90 * DAY, Ordering::SeqCst);
    fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id)
        .unwrap();
    fixture.state("closedForNoResponse");
}

#[test]
fn already_expired_qualifying_receipt_closes_through_normal_drain() {
    let mut fixture = Fixture::new(true, START - 91 * DAY, false);
    fixture.send_back();
    fixture.state("closedForNoResponse");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(fixture.current().timers.is_empty());
}

#[test]
#[ignore = "Known notice queue-head outage blocks late evidence; diagnostic acceptance test, run explicitly"]
fn lasting_notice_outage_still_allows_actual_late_photo_and_reopening_task() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    fixture.ports.fail_no_response.store(true, Ordering::SeqCst);
    fixture.clock.0.store(START + 91 * DAY, Ordering::SeqCst);
    assert!(
        fixture
            .runtime
            .drain_until_idle(&fixture.process.process_id)
            .is_err()
    );
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    let task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "attachLatePhoto")
        .unwrap();
    let accepted = fixture
        .runtime
        .submit_task_response(
            &task.task_id,
            json!({"status":"completed","definitionUrl":APPLICATION,"definitionVersion":"0.3.0",
            "data":{"photo":{"digest":"late-photo"},"originalFilingRef":"filing-original"}}),
            APPLICANT,
            Some("late-during-permanent-notice-outage"),
        )
        .unwrap();
    assert!(matches!(accepted, TaskSubmissionResult::Completed { .. }));
    let drained = fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id);
    let persisted = fixture.current();
    assert_eq!(
        persisted.configuration,
        vec!["closedWithLateEvidence"],
        "Accepted late evidence must progress during an unrelated notice outage; drain={drained:?}, pending={:?}",
        persisted.pending_events
    );
    assert_eq!(persisted.case_state["caseClosed"], true);
    assert!(
        persisted
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "considerReopening"
                && task.assigned_actor.as_deref() == Some(REVIEWER))
    );
}

#[test]
fn whitespace_only_decision_reason_does_not_advance() {
    for choice in ["approve", "deny", "sendBackPhoto"] {
        let mut fixture = Fixture::new(true, START, false);
        fixture.review(choice, " \t\n ", "whitespace-decision");
        fixture.state("awaitingReview");
        assert!(
            !fixture
                .ports
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|(name, _)| matches!(
                    name.as_str(),
                    "sendApprovalNotice" | "sendDenialNotice" | "sendPhotoRequestNotice"
                ))
        );
    }
}

#[test]
fn whitespace_only_reopening_reason_keeps_case_closed() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    fixture.advance(91);
    fixture.late_reply();
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":" \t\n "}),
        "whitespace-reopening",
    );
    fixture.state("closedWithLateEvidence");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(
        !fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name == "sendReopenedNotice")
    );
}

#[test]
fn retained_expired_photo_task_refuses_before_mutating_correction() {
    let mut fixture = Fixture::new(true, START, false);
    fixture.send_back();
    let expired_task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "replacePhoto")
        .unwrap();
    fixture.advance(91);
    fixture.state("closedForNoResponse");
    let before = fixture.current();
    let result = fixture.runtime.submit_task_response(
        &expired_task.task_id,
        json!({"status":"completed","definitionUrl":APPLICATION,"definitionVersion":"0.3.0",
            "data":{"photo":{"digest":"stale-task-photo"},"originalFilingRef":"filing-original"}}),
        APPLICANT,
        Some("expired-photo-response"),
    );
    assert!(
        result.is_err(),
        "Expired requested-photo task must refuse before mutation; observed {result:?}"
    );
    assert_eq!(fixture.current().case_state, before.case_state);
    assert_eq!(
        serde_json::to_value(fixture.current().pending_events).unwrap(),
        serde_json::to_value(before.pending_events).unwrap()
    );
}

#[test]
fn retained_expired_dispute_task_refuses_before_mutating_evidence() {
    let mut fixture = Fixture::new(true, START, true);
    let expired_task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "disputeOutsideData")
        .unwrap();
    fixture.advance(11);
    fixture.state("awaitingReview");
    let before = fixture.current();
    let result = fixture.runtime.submit_task_response(&expired_task.task_id,
        json!({"status":"completed","definitionUrl":"urn:formspec:demo:d1:passport-outside-data-dispute",
            "definitionVersion":"0.3.0","data":{"checkRef":"mockPriorPassportCheck","response":"dispute",
            "reason":"Stale task response","evidence":[]}}), APPLICANT, Some("expired-dispute-response"));
    assert!(
        result.is_err(),
        "Expired dispute task must refuse before mutation; observed {result:?}"
    );
    assert_eq!(fixture.current().case_state, before.case_state);
    assert_eq!(
        serde_json::to_value(fixture.current().pending_events).unwrap(),
        serde_json::to_value(before.pending_events).unwrap()
    );
}

fn accept_early_dispute(fixture: &mut Fixture) {
    fixture.advance(9);
    fixture.submit(
        "disputeOutsideData",
        APPLICANT,
        "urn:formspec:demo:d1:passport-outside-data-dispute",
        json!({"checkRef":"mockPriorPassportCheck","response":"dispute",
            "reason":"The mock check concerns someone else.","evidence":[]}),
        "early-dispute",
    );
    fixture.state("awaitingDisputeReview");
}

fn assert_no_effective_adverse_decision(fixture: &Fixture) {
    let current = fixture.current();
    assert_eq!(current.case_state["caseClosed"], false);
    assert!(current.case_state["decision"].is_null());
    assert!(!current.configuration.iter().any(|state| state == "denied"
        || state == "denialNoticePending"
        || state == "denialDecisionRecorded"));
    let record = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&current.process_id)
        .unwrap();
    assert!(!record.provenance_log.iter().any(|entry| {
        entry
            .transition_tags
            .iter()
            .any(|tag| tag == "determination")
    }));
    assert!(
        !fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name == "sendDenialNotice")
    );
}

fn assert_recorded_denial(fixture: &Fixture, reason: &str) {
    fixture.state("denied");
    let current = fixture.current();
    assert_eq!(current.case_state["decision"], "denied");
    assert_eq!(current.case_state["caseClosed"], true);
    assert_eq!(current.case_state["displayStatus"], "Denied");
    assert_eq!(current.case_state["reviewReason"], reason);
    let record = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&current.process_id)
        .unwrap();
    let determination = record
        .provenance_log
        .iter()
        .filter(|entry| {
            entry
                .transition_tags
                .iter()
                .any(|tag| tag == "determination")
        })
        .collect::<Vec<_>>();
    assert_eq!(determination.len(), 1);
    let snapshot: Value = serde_json::from_str(
        &determination[0]
            .case_file_snapshot
            .as_ref()
            .unwrap()
            .jcs_canonical,
    )
    .unwrap();
    assert_eq!(snapshot["decision"], "denied");
    assert_eq!(snapshot["caseClosed"], true);
    assert_eq!(snapshot["reviewReason"], reason);
    assert_eq!(
        snapshot["originalAnswers"],
        current.case_state["originalAnswers"]
    );
    assert_eq!(
        fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == "sendDenialNotice")
            .count(),
        1
    );
}

#[test]
fn early_dispute_response_does_not_allow_adverse_denial_before_full_minimum_period() {
    let mut fixture = Fixture::new(true, START, true);
    accept_early_dispute(&mut fixture);
    let reason = "The outside-data result weighs against eligibility.";
    fixture.review("deny", reason, "early-adverse-denial");
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    assert_eq!(fixture.current().case_state["reviewReason"], reason);
    let current = fixture.current();
    assert_eq!(current.timers.len(), 1);
    assert_eq!(current.timers[0].timer_id, "outsideDataMinimumPeriod");
    let deadline = OffsetDateTime::parse(&current.timers[0].deadline, &Rfc3339)
        .unwrap()
        .unix_timestamp_nanos()
        / 1_000_000;
    assert_eq!(deadline, i128::from(START + 10 * DAY));
    fixture
        .clock
        .0
        .store(START + 10 * DAY - 1, Ordering::SeqCst);
    fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id)
        .unwrap();
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    fixture.advance(10);
    assert_recorded_denial(&fixture, reason);
}

#[test]
fn early_outside_data_review_can_approve_without_waiting_for_adverse_use() {
    let mut fixture = Fixture::new(true, START, true);
    accept_early_dispute(&mut fixture);
    fixture.review(
        "approve",
        "The dispute establishes eligibility.",
        "early-approval",
    );
    fixture.state("approved");
    assert!(fixture.current().timers.is_empty());
    assert!(
        !fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name == "sendDenialNotice")
    );
}

#[test]
fn nonadverse_denial_records_reason_without_outside_data_minimum_timer() {
    let mut fixture = Fixture::new(true, START, false);
    let reason = "The original application does not meet the eligibility criteria.";
    fixture.review("deny", reason, "nonadverse-denial");
    assert_recorded_denial(&fixture, reason);
    assert!(fixture.current().timers.is_empty());
}

#[test]
fn early_photo_return_cannot_bypass_outside_data_minimum_period() {
    let mut fixture = Fixture::new(true, START, true);
    accept_early_dispute(&mut fixture);
    fixture.send_back();
    fixture.state("awaitingPhoto");
    fixture.submit(
        "replacePhoto",
        APPLICANT,
        APPLICATION,
        json!({"photo":{"digest":"corrected-photo"},"originalFilingRef":"filing-original"}),
        "early-photo-return",
    );
    fixture.state("returnedReview");
    let reason = "The corrected photo does not resolve the adverse outside-data evidence.";
    fixture.review("deny", reason, "returned-adverse-denial");
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    fixture.advance(10);
    assert_recorded_denial(&fixture, reason);
}

#[test]
fn unanswered_outside_data_window_allows_reasoned_denial_at_full_boundary() {
    let mut fixture = Fixture::new(true, START, true);
    fixture.advance(10);
    fixture.state("awaitingReview");
    let reason =
        "The retained application and undisputed outside-data result do not establish eligibility.";
    fixture.review("deny", reason, "unanswered-adverse-denial");
    assert_recorded_denial(&fixture, reason);
    assert!(fixture.current().timers.is_empty());
}

// These receipts are explicit controlled native ingress, not provider qualification.
fn controlled_deferred_receipt(fixture: &mut Fixture, service: &str, qualifying: bool) {
    let process = fixture.current();
    let outbound = process
        .pending_outbound_events
        .values()
        .find(|pending| pending.binding_id == service)
        .unwrap();
    let receipt = json!({"id":format!("controlled-receipt-{}",outbound.invocation_id),
        "source":"urn:test:controlled-delivery","specversion":"1.0","type":"com.wos.demo.d1.delivery.receipt",
        "subject":outbound.subject,"data":{"qualifying":qualifying,"sentAt":iso(fixture.ports.sent_at),"testDouble":true}});
    fixture
        .runtime
        .enqueue_event(
            &fixture.process.process_id,
            wos_core::instance::PendingEvent {
                event: format!("delivery.{service}.receipt"),
                actor_id: None,
                data: Some(receipt),
                timestamp: iso(fixture.clock.now_ms()),
                idempotency_token: None,
            },
        )
        .unwrap();
    fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id)
        .unwrap();
}

fn assert_deferred_main_state(fixture: &Fixture, state: &str) {
    let current = fixture.current();
    // Native configuration retains the active parallel parent and both regions.
    let mut actual = current.configuration;
    actual.sort();
    let mut expected = vec![
        "caseAndDelivery".to_string(),
        state.to_string(),
        "awaitingDeliveryReceipt".to_string(),
    ];
    expected.sort();
    assert_eq!(actual, expected);
}

fn deferred_outage_fixture() -> Fixture {
    let mut fixture = Fixture::deferred(START);
    fixture.send_back();
    assert_deferred_main_state(&fixture, "photoNoticePending");
    assert!(fixture.current().timers.is_empty());
    assert!(fixture.current().case_state["photoRequestNotice"].is_null());
    controlled_deferred_receipt(&mut fixture, "sendPhotoRequestNotice", true);
    assert_deferred_main_state(&fixture, "awaitingPhoto");
    fixture.ports.fail_no_response.store(true, Ordering::SeqCst);
    fixture.advance(91);
    assert_deferred_main_state(&fixture, "closedForNoResponse");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(
        fixture
            .current()
            .pending_outbound_events
            .values()
            .any(|pending| pending.binding_id == "sendNoResponseNotice")
    );
    fixture
}

#[test]
fn deferred_delivery_continuing_outage_allows_late_evidence_and_global_receipts_without_duplicate_tasks()
 {
    let mut fixture = deferred_outage_fixture();
    fixture.late_reply();
    let expected = native_deferred_inputs()["correction"]["mappingOutput"].clone();
    assert_eq!(
        fixture.current().case_state["correctionPhoto"],
        expected["correctionPhoto"]
    );
    assert_eq!(
        fixture.current().case_state["correctionFilingRef"],
        expected["correctionFilingRef"]
    );
    assert_eq!(
        fixture.current().case_state["originalAnswers"],
        fixture.process.case_state["originalAnswers"]
    );
    assert_deferred_main_state(&fixture, "closedWithLateEvidence");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendNoResponseNotice", true);
    controlled_deferred_receipt(&mut fixture, "sendLateEvidenceReceivedNotice", true);
    assert_deferred_main_state(&fixture, "closedWithLateEvidence");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(
        fixture
            .current()
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "considerReopening"
                && task.assigned_actor.as_deref() == Some(REVIEWER))
    );
    assert!(
        !fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name.starts_with("send"))
    );
}

#[test]
fn deferred_receipts_remain_routable_after_reopening_and_returned_review_without_task_reentry() {
    let mut fixture = deferred_outage_fixture();
    fixture.late_reply();
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":"Accepted late evidence warrants reconsideration."}),
        "deferred-reopen",
    );
    assert_deferred_main_state(&fixture, "reopened");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendNoResponseNotice", true);
    controlled_deferred_receipt(&mut fixture, "sendLateEvidenceReceivedNotice", true);
    assert_deferred_main_state(&fixture, "reopened");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    controlled_deferred_receipt(&mut fixture, "sendReopenedNotice", true);
    assert_deferred_main_state(&fixture, "returnedReview");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendInformationReceivedNotice", true);
    assert_deferred_main_state(&fixture, "returnedReview");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert_eq!(fixture.current().case_state["caseClosed"], false);
}

#[test]
fn deferred_nonqualifying_receipt_does_not_start_notice_clock() {
    let mut fixture = Fixture::deferred(START);
    fixture.send_back();
    controlled_deferred_receipt(&mut fixture, "sendPhotoRequestNotice", false);
    assert_deferred_main_state(&fixture, "photoNoticePending");
    assert!(fixture.current().timers.is_empty());
    assert!(
        !fixture
            .current()
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "replacePhoto")
    );
}

#[test]
fn deferred_receipt_region_keeps_global_ingress_active_after_main_case_finality() {
    let mut fixture = deferred_outage_fixture();
    fixture.late_reply();
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":"Reconsideration is warranted."}),
        "deferred-final-reopen",
    );
    controlled_deferred_receipt(&mut fixture, "sendReopenedNotice", true);
    fixture.review(
        "approve",
        "The corrected application satisfies eligibility.",
        "deferred-final-approval",
    );
    controlled_deferred_receipt(&mut fixture, "sendApprovalNotice", true);
    assert_deferred_main_state(&fixture, "approved");
    // Finality belongs to the main case region. No whole-process completion or
    // implicit cancellation is claimed while receipt ingress remains active.
    assert_eq!(
        fixture.current().status,
        wos_core::instance::InstanceStatus::Active
    );
    assert!(
        fixture
            .current()
            .pending_outbound_events
            .values()
            .any(|pending| pending.binding_id == "sendNoResponseNotice")
    );
    controlled_deferred_receipt(&mut fixture, "sendNoResponseNotice", true);
    assert_deferred_main_state(&fixture, "approved");
}

#[test]
fn deferred_receipt_in_closed_case_preserves_late_task_identity() {
    let mut fixture = deferred_outage_fixture();
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendNoResponseNotice", true);
    assert_deferred_main_state(&fixture, "closedForNoResponse");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert!(fixture.current().pending_outbound_events.is_empty());
    assert_eq!(fixture.current().case_state["caseClosed"], true);
}

#[test]
fn invalid_and_duplicate_deferred_receipts_never_wake_case_or_restart_clock() {
    for mode in [
        "unknown",
        "wrong-binding",
        "wrong-type",
        "missing-id",
        "drift",
        "duplicate",
    ] {
        let mut fixture = Fixture::deferred(START);
        fixture.send_back();
        let outbound = fixture
            .current()
            .pending_outbound_events
            .values()
            .find(|pending| pending.binding_id == "sendPhotoRequestNotice")
            .unwrap()
            .clone();
        let mut event = "delivery.sendPhotoRequestNotice.receipt".to_string();
        let mut receipt = json!({"id":"controlled-negative-receipt","source":"urn:test:receipt","specversion":"1.0",
            "type":"com.wos.demo.d1.delivery.receipt","subject":outbound.subject,"data":{"qualifying":true,"sentAt":iso(START)}});
        match mode {
            "unknown" => receipt["subject"] = json!("unknown"),
            "wrong-binding" => event = "delivery.sendOutsideDataNotice.receipt".into(),
            "wrong-type" => receipt["type"] = json!("com.example.unrelated"),
            "missing-id" => {
                receipt.as_object_mut().unwrap().remove("id");
            }
            "drift" => {
                // Rebuild only the host-selected profile, leaving persisted request untouched.
                let mut profile: IntegrationProfileDocument = serde_json::from_str(include_str!(
                    "../../../examples/d1-passport-complete-deferred.integration.json"
                ))
                .unwrap();
                profile
                    .bindings
                    .get_mut("sendPhotoRequestNotice")
                    .unwrap()
                    .output_binding
                    .insert("caseFile.foreignReceipt".into(), "$".into());
                fixture.runtime = controlled_runtime_with_artifacts(
                    fixture.store.clone(),
                    fixture.clock.clone(),
                    fixture.ports.clone(),
                    serde_json::from_str(include_str!(
                        "../../../examples/d1-passport-complete-deferred.workflow.json"
                    ))
                    .unwrap(),
                    profile,
                );
            }
            _ => controlled_deferred_receipt(&mut fixture, "sendPhotoRequestNotice", true),
        }
        fixture
            .runtime
            .enqueue_event(
                &fixture.process.process_id,
                wos_core::instance::PendingEvent {
                    event,
                    actor_id: None,
                    data: Some(receipt),
                    timestamp: iso(START),
                    idempotency_token: None,
                },
            )
            .unwrap();
        let before = fixture
            .store
            .0
            .lock()
            .unwrap()
            .load_record(&fixture.process.process_id)
            .unwrap();
        assert!(
            fixture
                .runtime
                .drain_once(&fixture.process.process_id)
                .is_err(),
            "{mode}"
        );
        let after = fixture
            .store
            .0
            .lock()
            .unwrap()
            .load_record(&fixture.process.process_id)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&after.process).unwrap(),
            serde_json::to_value(&before.process).unwrap(),
            "{mode}"
        );
        assert_eq!(
            serde_json::to_value(&after.provenance_log).unwrap(),
            serde_json::to_value(&before.provenance_log).unwrap(),
            "{mode}"
        );
        assert!(
            !after
                .process
                .pending_events
                .iter()
                .any(|pending| pending.event == "photo.noticeDeliveryAttempted")
        );
    }
}

#[test]
fn deferred_closed_case_idle_and_reload_preserve_exact_pending_work_without_busy_loop() {
    let mut fixture = deferred_outage_fixture();
    let original = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&fixture.process.process_id)
        .unwrap();
    for _ in 0..3 {
        assert!(
            fixture
                .runtime
                .drain_until_idle(&fixture.process.process_id)
                .unwrap()
                .is_empty()
        );
        let current = fixture
            .store
            .0
            .lock()
            .unwrap()
            .load_record(&fixture.process.process_id)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&current.process).unwrap(),
            serde_json::to_value(&original.process).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&current.provenance_log).unwrap(),
            serde_json::to_value(&original.provenance_log).unwrap()
        );
    }
    let mut reconstructed = original.clone();
    reconstructed.process =
        serde_json::from_value(serde_json::to_value(&original.process).unwrap()).unwrap();
    let fresh = SharedInMemoryStore::default();
    fresh
        .0
        .lock()
        .unwrap()
        .restore_record(reconstructed)
        .unwrap();
    fixture.store = fresh.clone();
    fixture.runtime = controlled_runtime_with_artifacts(
        fresh,
        fixture.clock.clone(),
        fixture.ports.clone(),
        serde_json::from_str(include_str!(
            "../../../examples/d1-passport-complete-deferred.workflow.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../examples/d1-passport-complete-deferred.integration.json"
        ))
        .unwrap(),
    );
    assert!(
        fixture
            .runtime
            .drain_until_idle(&fixture.process.process_id)
            .unwrap()
            .is_empty()
    );
    let restored = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&fixture.process.process_id)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&restored.process).unwrap(),
        serde_json::to_value(&original.process).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&restored.provenance_log).unwrap(),
        serde_json::to_value(&original.provenance_log).unwrap()
    );
    assert_deferred_main_state(&fixture, "closedForNoResponse");
}

// Deferred adverse paths use native-valid synthetic dispute data, while the
// processor and admitted delivery receipts remain explicitly controlled doubles.
fn deferred_adverse_dispute_fixture() -> Fixture {
    let mut fixture = Fixture::build(true, START, true, true);
    assert_deferred_main_state(&fixture, "outsideDataNoticePending");
    assert!(fixture.current().timers.is_empty());
    assert!(
        !fixture
            .current()
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "disputeOutsideData")
    );
    controlled_deferred_receipt(&mut fixture, "sendOutsideDataNotice", true);
    assert_deferred_main_state(&fixture, "outsideDataDisputeWindow");
    assert_eq!(fixture.ports.calls.lock().unwrap().len(), 2);
    fixture.advance(9);
    fixture.submit(
        "disputeOutsideData",
        APPLICANT,
        "urn:formspec:demo:d1:passport-outside-data-dispute",
        native_deferred_inputs()["dispute"]["input"].clone(),
        "deferred-dispute",
    );
    assert_deferred_main_state(&fixture, "awaitingDisputeReview");
    let expected = native_deferred_inputs()["dispute"]["mappingOutput"].clone();
    for key in [
        "outsideCheckRef",
        "outsideCheckResponse",
        "outsideCheckReason",
        "outsideCheckEvidence",
    ] {
        assert_eq!(fixture.current().case_state[key], expected[key]);
    }
    assert!(fixture.current().active_tasks.iter().any(|task| {
        task.task_ref == "reviewApplication" && task.assigned_actor.as_deref() == Some(REVIEWER)
    }));
    fixture
}

fn assert_deferred_recorded_denial(fixture: &Fixture, reason: &str) {
    assert_deferred_main_state(fixture, "denied");
    let current = fixture.current();
    assert_eq!(current.case_state["decision"], "denied");
    assert_eq!(current.case_state["caseClosed"], true);
    assert_eq!(current.case_state["displayStatus"], "Denied");
    assert_eq!(current.case_state["reviewReason"], reason);
    let record = fixture
        .store
        .0
        .lock()
        .unwrap()
        .load_record(&current.process_id)
        .unwrap();
    let determination: Vec<_> = record
        .provenance_log
        .iter()
        .filter(|entry| {
            entry
                .transition_tags
                .iter()
                .any(|tag| tag == "determination")
        })
        .collect();
    assert_eq!(determination.len(), 1);
    let snapshot: Value = serde_json::from_str(
        &determination[0]
            .case_file_snapshot
            .as_ref()
            .unwrap()
            .jcs_canonical,
    )
    .unwrap();
    assert_eq!(snapshot["decision"], "denied");
    assert_eq!(snapshot["caseClosed"], true);
    assert_eq!(snapshot["reviewReason"], reason);
    assert_eq!(
        snapshot["originalAnswers"],
        fixture.process.case_state["originalAnswers"]
    );
    assert!(
        !fixture
            .ports
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|(name, _)| name.starts_with("send"))
    );
}

#[test]
fn deferred_adverse_dispute_denial_waits_exact_minimum_boundary_and_receipt_before_regional_finality()
 {
    let mut fixture = deferred_adverse_dispute_fixture();
    let reason = "The retained dispute and source evidence do not establish eligibility.";
    fixture.review("deny", reason, "deferred-adverse-denial");
    assert_deferred_main_state(&fixture, "outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    let timer = fixture
        .current()
        .timers
        .into_iter()
        .find(|timer| timer.timer_id == "outsideDataMinimumPeriod")
        .unwrap();
    assert_eq!(
        OffsetDateTime::parse(&timer.deadline, &Rfc3339)
            .unwrap()
            .unix_timestamp_nanos()
            / 1_000_000,
        i128::from(START + 10 * DAY)
    );
    fixture
        .clock
        .0
        .store(START + 10 * DAY - 1, Ordering::SeqCst);
    fixture
        .runtime
        .drain_until_idle(&fixture.process.process_id)
        .unwrap();
    assert_deferred_main_state(&fixture, "outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    fixture.advance(10);
    assert_deferred_main_state(&fixture, "denialNoticePending");
    assert_eq!(fixture.current().case_state["decision"], "denied");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    let tasks = serde_json::to_value(fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendDenialNotice", true);
    assert_deferred_recorded_denial(&fixture, reason);
    assert_eq!(
        serde_json::to_value(fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert_eq!(
        fixture.current().status,
        wos_core::instance::InstanceStatus::Active
    );
}

#[test]
fn deferred_nonadverse_denied_final_retains_global_receipt_ingress_without_task_recreation() {
    let mut fixture = deferred_outage_fixture();
    fixture.late_reply();
    fixture.submit(
        "considerReopening",
        REVIEWER,
        REVIEW,
        json!({"choice":"reopen","reason":"Accepted late evidence warrants reconsideration."}),
        "deferred-denial-reopen",
    );
    controlled_deferred_receipt(&mut fixture, "sendReopenedNotice", true);
    let reason = "The retained application does not meet the stated requirement.";
    fixture.review("deny", reason, "deferred-nonadverse-denial");
    assert_deferred_main_state(&fixture, "denialNoticePending");
    controlled_deferred_receipt(&mut fixture, "sendDenialNotice", true);
    assert_deferred_recorded_denial(&fixture, reason);
    let before = serde_json::to_value(fixture.current().active_tasks).unwrap();
    controlled_deferred_receipt(&mut fixture, "sendNoResponseNotice", true);
    controlled_deferred_receipt(&mut fixture, "sendLateEvidenceReceivedNotice", true);
    controlled_deferred_receipt(&mut fixture, "sendInformationReceivedNotice", true);
    assert_deferred_recorded_denial(&fixture, reason);
    assert_eq!(
        serde_json::to_value(fixture.current().active_tasks).unwrap(),
        before
    );
    assert!(fixture.current().pending_outbound_events.is_empty());
}

#[test]
fn deferred_early_dispute_approval_remains_available_before_adverse_use_boundary() {
    let mut fixture = deferred_adverse_dispute_fixture();
    fixture.review(
        "approve",
        "The dispute establishes eligibility.",
        "deferred-early-approve",
    );
    assert_deferred_main_state(&fixture, "approvalNoticePending");
    assert!(fixture.current().timers.is_empty());
    controlled_deferred_receipt(&mut fixture, "sendApprovalNotice", true);
    assert_deferred_main_state(&fixture, "approved");
    assert_eq!(
        fixture.current().status,
        wos_core::instance::InstanceStatus::Active
    );
    assert_eq!(fixture.clock.0.load(Ordering::SeqCst), START + 9 * DAY);
}
