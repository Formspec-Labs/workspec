// Rust guideline compliant 2026-02-21

//! Exercises complete D1 lifecycle requirements with controlled runtime ports.
//!
//! These tests use canned check outputs, controlled delivery receipts and selected
//! Mapping outputs. They do not qualify production Definitions, delivery, protected
//! response fields, processing-pause accounting, host authorization, or offline
//! export verification.

use serde_json::{Value, json};
use stack_common_typeid::testing::fixture_minter;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
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
const DISPUTE: &str = "urn:formspec:demo:d1:passport-outside-data-dispute";
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
/// Answers only the two mock checks. Notices are deferred callbacks, so a
/// provider call for any `send*` binding is an unselected service.
#[derive(Clone)]
struct ControlledPorts {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    adverse: bool,
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
        match service_ref {
            "mockPriorPassportCheck" | "mockSsnCheck" => Ok(json!({
                "source":service_ref,"checkedAt":iso(START),"adverse":self.adverse,
                "result":"controlled test result","availability":"controlled","confirmation":"unconfirmed","testDouble":true})),
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

fn integration() -> IntegrationProfileDocument {
    serde_json::from_str(INTEGRATION).unwrap()
}

fn controlled_runtime(
    store: SharedInMemoryStore,
    clock: ManualClock,
    ports: ControlledPorts,
    profile: IntegrationProfileDocument,
) -> WosRuntime {
    let mut bindings = BindingRegistry::new();
    bindings.register(FormspecBinding::new(SelectedMappings));
    WosRuntime::new(
        store,
        Resolver(selected_workflow()),
        DefaultRuntime::new(),
        AssignmentOnly,
        ports,
        DefaultRuntime::new(),
        clock,
        bindings,
    )
    .with_integration_profile(profile)
    .with_controlled_governance()
}

// These synthetic inputs passed independent native Definition and Mapping probes.
// SelectedMappings still does not implement native Definition validation or
// enforce protected original-answer admission.
fn native_inputs() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/d1/deferred-native-valid-inputs.json"
    ))
    .unwrap()
}

/// Mirrors the data WOS puts on a notice receipt (`notice_callback.rs`): the
/// host builds one only for an accepted delivery with an observed time, so its
/// arrival already means the notice was sent. It carries no qualification flag.
/// These receipts are date-gated, as a notice that starts a timer must be:
/// `clock.governedSentAt` is the send instant in case time, while `sentAt`
/// stays the physical observation.
fn accepted_receipt_data(intent: &str, sent_at: &str, governed_sent_at: &str) -> Value {
    json!({"sentAt":sent_at,"delivery":{"version":1,"status":"accepted",
        "intentId":intent,"attemptId":format!("{intent}:attempt:1"),
        "messageId":format!("<{intent}@example.invalid>"),
        "requestSha256":"sha256:controlled-request","bodySha256":"sha256:controlled-body",
        "startedAt":sent_at,"observedAt":sent_at,"capability":"smtp_test_capture","smtpCode":250},
        "clock":{"version":1,"governedSentAt":governed_sent_at,
            "originalProfileSha256":"sha256:controlled-profile","dateDecisionSha256":"sha256:controlled-decision"}})
}

struct Fixture {
    runtime: WosRuntime,
    store: SharedInMemoryStore,
    process: WorkflowProcess,
    clock: ManualClock,
    ports: ControlledPorts,
    sent_at: u64,
    governed_sent_at: Option<u64>,
}
impl Fixture {
    fn new(sent_at: u64, adverse: bool) -> Self {
        let store = SharedInMemoryStore::default();
        let clock = ManualClock(Arc::new(AtomicU64::new(START)));
        let ports = ControlledPorts {
            calls: Arc::new(Mutex::new(vec![])),
            adverse,
        };
        let mut runtime =
            controlled_runtime(store.clone(), clock.clone(), ports.clone(), integration());
        let original_answers = native_inputs()["initial"]["input"].clone();
        let process = runtime
            .create_process_with_actor_bindings(
                CreateProcessRequest {
                    process_id: fixture_minter().mint_process_id(),
                    tenant: None,
                    definition_url: "urn:wos:demo:d1:passport-complete".into(),
                    definition_version: "0.3.0".into(),
                    initial_case_state: Some(json!({
                        "originalFilingRef":"urn:formspec:fixture:original-filing",
                        "originalPhoto":original_answers["photo"], "originalAnswers":original_answers,
                        "referenceNumber":"D1-TEST", "preferredLanguage":"en", "caseClosed":false})),
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
            sent_at,
            governed_sent_at: None,
        }
    }
    fn current(&self) -> WorkflowProcess {
        self.runtime.load_process(&self.process.process_id).unwrap()
    }
    /// Asserts the main case state beside the receipt region's leaf.
    fn state(&self, expected: &str) {
        let mut actual = self.current().configuration;
        actual.sort();
        let mut wanted = vec![expected.to_string(), "awaitingDeliveryReceipt".to_string()];
        wanted.sort();
        assert_eq!(actual, wanted);
    }
    fn pending_notice(&self, service: &str) -> bool {
        self.current()
            .pending_outbound_events
            .values()
            .any(|pending| pending.binding_id == service)
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
                integrity_canonical::ResponseJson::from_value(
                    &(json!({"status":"completed",
            "definitionUrl":url,"definitionVersion":"0.3.0","data":data})),
                )
                .unwrap(),
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
        self.submit(
            "reviewApplication",
            REVIEWER,
            REVIEW,
            native_inputs()["sendBack"]["input"].clone(),
            "sendback",
        );
    }
    /// Sends back for a photo and delivers its notice receipt.
    fn request_photo(&mut self) {
        self.send_back();
        self.state("photoNoticePending");
        self.receipt("sendPhotoRequestNotice");
        self.state("awaitingPhoto");
    }
    fn advance(&mut self, days: u64) {
        self.clock.0.store(START + days * DAY, Ordering::SeqCst);
        self.runtime
            .drain_until_idle(&self.process.process_id)
            .unwrap();
    }
    fn late_reply(&mut self) {
        self.submit(
            "attachLatePhoto",
            APPLICANT,
            APPLICATION,
            native_inputs()["correction"]["input"].clone(),
            "late",
        );
    }
    fn reopen(&mut self, reason: &str, token: &str) {
        self.submit(
            "considerReopening",
            REVIEWER,
            REVIEW,
            json!({"choice":"reopen","reason":reason}),
            token,
        );
    }
    // These receipts are explicit controlled native ingress, not provider qualification.
    fn receipt(&mut self, service: &str) {
        let outbound = self
            .current()
            .pending_outbound_events
            .into_values()
            .find(|pending| pending.binding_id == service)
            .unwrap();
        let receipt = json!({"id":format!("controlled-receipt-{}",outbound.invocation_id),
            "source":"urn:test:controlled-delivery","specversion":"1.0","type":"com.wos.demo.d1.delivery.receipt",
            "subject":outbound.subject,"data":accepted_receipt_data(&outbound.invocation_id, &iso(self.sent_at),
                &iso(self.governed_sent_at.unwrap_or(self.sent_at)))});
        self.runtime
            .enqueue_event(
                &self.process.process_id,
                wos_core::instance::PendingEvent {
                    origin: None,
                    event: format!("delivery.{service}.receipt"),
                    actor_id: None,
                    data: Some(receipt),
                    timestamp: iso(self.clock.now_ms()),
                    idempotency_token: None,
                },
            )
            .unwrap();
        self.runtime
            .drain_until_idle(&self.process.process_id)
            .unwrap();
    }
    fn timer_deadline_ms(&self, timer_id: &str) -> i128 {
        let timer = self
            .current()
            .timers
            .into_iter()
            .find(|timer| timer.timer_id == timer_id)
            .unwrap();
        OffsetDateTime::parse(&timer.deadline, &Rfc3339)
            .unwrap()
            .unix_timestamp_nanos()
            / 1_000_000
    }
}

#[test]
fn automatic_checks_map_distinct_sources_times_and_persist_results() {
    let fixture = Fixture::new(START, false);
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
    assert_eq!(fixture.ports.calls.lock().unwrap().len(), 2);
}

#[test]
fn sent_photo_notice_opens_same_form_photo_task_then_new_review_and_reasoned_approval() {
    let mut fixture = Fixture::new(START, false);
    let initial_review_id = fixture.current().active_tasks[0].task_id.clone();
    fixture.request_photo();
    let waiting = fixture.current();
    assert_eq!(waiting.timers.len(), 1);
    assert_eq!(waiting.timers[0].duration_iso.as_deref(), Some("P90D"));
    assert_eq!(
        waiting.case_state["photoRequestNotice"]["sentAt"],
        iso(START)
    );
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
    let original = fixture.process.case_state.clone();
    assert_eq!(prefill["fullName"], original["originalAnswers"]["fullName"]);
    assert_eq!(prefill["photo"], original["originalPhoto"]);
    assert_eq!(prefill["originalFilingRef"], original["originalFilingRef"]);
    let correction = native_inputs()["correction"].clone();
    fixture.submit(
        "replacePhoto",
        APPLICANT,
        APPLICATION,
        correction["input"].clone(),
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
        original["originalAnswers"]
    );
    assert_eq!(
        returned.case_state["originalPhoto"],
        original["originalPhoto"]
    );
    assert_eq!(
        returned.case_state["correctionPhoto"],
        correction["mappingOutput"]["correctionPhoto"]
    );
    let reason = "The replacement photo and original application satisfy the review.";
    fixture.review("approve", reason, "approve");
    fixture.state("approvalNoticePending");
    fixture.receipt("sendApprovalNotice");
    fixture.state("approved");
    assert_eq!(fixture.current().case_state["reviewReason"], reason);
}

#[test]
fn photo_notice_without_receipt_creates_no_response_clock_or_task() {
    let mut fixture = Fixture::new(START, false);
    fixture.send_back();
    fixture.state("photoNoticePending");
    assert!(fixture.pending_notice("sendPhotoRequestNotice"));
    assert!(fixture.current().case_state["photoRequestNotice"].is_null());
    fixture.advance(91);
    fixture.state("photoNoticePending");
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
fn expired_photo_clock_closes_late_reply_stays_closed_and_reasoned_adjudicator_reopens() {
    let mut fixture = Fixture::new(START, false);
    fixture.request_photo();
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
    assert!(fixture.runtime.submit_task_response(&task.task_id, integrity_canonical::ResponseJson::from_value(&(json!({"status":"completed",
        "definitionUrl":REVIEW,"definitionVersion":"0.3.0","data":{"choice":"reopen","reason":"New photo changes the decision."}}))).unwrap(),
        APPLICANT, Some("unauthorized-reopen")).is_err());
    fixture.reopen("New photo changes the decision.", "reopen");
    fixture.state("reopened");
    assert_eq!(fixture.current().case_state["caseClosed"], false);
    fixture.receipt("sendReopenedNotice");
    fixture.state("returnedReview");
}

#[test]
fn blank_reopening_reason_keeps_case_closed() {
    for reason in ["", " \t\n "] {
        let mut fixture = Fixture::new(START, false);
        fixture.request_photo();
        fixture.advance(91);
        fixture.late_reply();
        fixture.reopen(reason, "blank-reason");
        fixture.state("closedWithLateEvidence");
        assert_eq!(fixture.current().case_state["caseClosed"], true);
        assert!(!fixture.pending_notice("sendReopenedNotice"));
    }
}

#[test]
fn delayed_receipt_clock_expires_ninety_days_after_sent_at() {
    // Recording a receipt two days after sending must not extend the
    // applicant's deadline by two days.
    let sent_at = START - 2 * DAY;
    let mut fixture = Fixture::new(sent_at, false);
    fixture.request_photo();
    assert_eq!(
        fixture.timer_deadline_ms("photoResponse"),
        i128::from(sent_at + 90 * DAY),
        "D1 clock must start at the receipt's sentAt, not receipt processing time"
    );
}

#[test]
fn authored_workflow_has_no_native_lint_errors() {
    let errors: Vec<_> = wos_lint::lint_document(WORKFLOW)
        .unwrap()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == wos_lint::LintSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "lint errors: {errors:?}");
}

#[test]
fn date_gated_receipt_starts_the_photo_clock_in_case_time() {
    // ADR 0144: the deadline runs in the case-time domain. The receipt maps the
    // physical send into it; the physical sentAt here is even later than the
    // case clock, which a historical timer origin may not be.
    let mut fixture = Fixture::new(START + DAY, false);
    fixture.governed_sent_at = Some(START - 2 * DAY);
    fixture.request_photo();
    assert_eq!(
        fixture.timer_deadline_ms("photoResponse"),
        i128::from(START - 2 * DAY + 90 * DAY)
    );
}

#[test]
fn receipt_anchored_timer_survives_new_runtime_reload_and_expires_at_original_deadline() {
    let sent_at = START - 2 * DAY;
    let mut fixture = Fixture::new(sent_at, false);
    fixture.request_photo();
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
        integration(),
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
fn already_expired_receipt_closes_through_normal_drain() {
    let mut fixture = Fixture::new(START - 91 * DAY, false);
    fixture.send_back();
    fixture.receipt("sendPhotoRequestNotice");
    fixture.state("closedForNoResponse");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(fixture.current().timers.is_empty());
}

#[test]
fn whitespace_only_decision_reason_does_not_advance() {
    for choice in ["approve", "deny", "sendBackPhoto"] {
        let mut fixture = Fixture::new(START, false);
        fixture.review(choice, " \t\n ", "whitespace-decision");
        fixture.state("awaitingReview");
        assert!(fixture.current().pending_outbound_events.is_empty());
    }
}

#[test]
fn retained_expired_photo_task_refuses_before_mutating_correction() {
    let mut fixture = Fixture::new(START, false);
    fixture.request_photo();
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
        integrity_canonical::ResponseJson::from_value(
            &json!({"status":"completed","definitionUrl":APPLICATION,"definitionVersion":"0.3.0",
            "data":native_inputs()["correction"]["input"]}),
        )
        .unwrap(),
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
    let mut fixture = Fixture::new(START, true);
    fixture.receipt("sendOutsideDataNotice");
    let expired_task = fixture
        .current()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "disputeOutsideData")
        .unwrap();
    fixture.advance(11);
    fixture.state("awaitingReview");
    let before = fixture.current();
    let result = fixture.runtime.submit_task_response(
        &expired_task.task_id,
        integrity_canonical::ResponseJson::from_value(
            &json!({"status":"completed","definitionUrl":DISPUTE,"definitionVersion":"0.3.0",
            "data":native_inputs()["dispute"]["input"]}),
        )
        .unwrap(),
        APPLICANT,
        Some("expired-dispute-response"),
    );
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
    assert!(!fixture.pending_notice("sendDenialNotice"));
}

/// Records the denial, then delivers its notice receipt and checks finality.
fn assert_recorded_denial(fixture: &mut Fixture, reason: &str) {
    fixture.state("denialNoticePending");
    fixture.receipt("sendDenialNotice");
    fixture.state("denied");
    let current = fixture.current();
    assert_eq!(current.case_state["decision"], "denied");
    assert_eq!(current.case_state["caseClosed"], true);
    assert_eq!(current.case_state["displayStatus"], "Denied");
    assert_eq!(current.case_state["reviewReason"], reason);
    assert_eq!(
        current.case_state["decisionNotice"]["sentAt"],
        iso(fixture.sent_at)
    );
    // Finality belongs to the main case region; receipt ingress stays active.
    assert_eq!(current.status, wos_core::instance::InstanceStatus::Active);
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
        fixture.process.case_state["originalAnswers"]
    );
}

#[test]
fn nonadverse_denial_records_reason_without_outside_data_minimum_timer() {
    let mut fixture = Fixture::new(START, false);
    let reason = "The original application does not meet the eligibility criteria.";
    fixture.review("deny", reason, "nonadverse-denial");
    assert!(fixture.current().timers.is_empty());
    assert_recorded_denial(&mut fixture, reason);
}

/// Adverse checks wait for the outside-data notice receipt, then hold a
/// ten-day dispute window; an early dispute reaches only the assigned reviewer.
fn adverse_dispute_fixture() -> Fixture {
    let mut fixture = Fixture::new(START, true);
    fixture.state("outsideDataNoticePending");
    assert!(fixture.current().timers.is_empty());
    assert!(
        !fixture
            .current()
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "disputeOutsideData")
    );
    fixture.receipt("sendOutsideDataNotice");
    fixture.state("outsideDataDisputeWindow");
    let current = fixture.current();
    assert_eq!(current.timers.len(), 1);
    assert_eq!(current.timers[0].duration_iso.as_deref(), Some("P10D"));
    assert!(
        !current
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "reviewApplication")
    );
    assert_eq!(fixture.ports.calls.lock().unwrap().len(), 2);
    fixture.advance(9);
    fixture.state("outsideDataDisputeWindow");
    let dispute = native_inputs()["dispute"].clone();
    fixture.submit(
        "disputeOutsideData",
        APPLICANT,
        DISPUTE,
        dispute["input"].clone(),
        "early-dispute",
    );
    fixture.state("awaitingDisputeReview");
    assert!(fixture.current().timers.is_empty());
    for key in [
        "outsideCheckRef",
        "outsideCheckResponse",
        "outsideCheckReason",
        "outsideCheckEvidence",
    ] {
        assert_eq!(
            fixture.current().case_state[key],
            dispute["mappingOutput"][key]
        );
    }
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
                integrity_canonical::ResponseJson::from_value(&json!({"status":"completed",
        "definitionUrl":REVIEW,"definitionVersion":"0.3.0",
        "data":{"choice":"deny","reason":"Applicant cannot issue an adjudication."}}))
                .unwrap(),
                APPLICANT,
                Some("wrong-dispute-reviewer")
            )
            .is_err()
    );
    fixture
}

#[test]
fn early_adverse_denial_waits_for_the_full_minimum_period_from_the_sent_notice() {
    let mut fixture = adverse_dispute_fixture();
    let reason = "The retained dispute and source evidence do not establish eligibility.";
    fixture.review("deny", reason, "early-adverse-denial");
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    assert_eq!(fixture.current().case_state["reviewReason"], reason);
    assert_eq!(fixture.current().timers.len(), 1);
    assert_eq!(
        fixture.timer_deadline_ms("outsideDataMinimumPeriod"),
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
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    fixture.advance(10);
    assert_recorded_denial(&mut fixture, reason);
}

#[test]
fn early_outside_data_review_can_approve_without_waiting_for_adverse_use() {
    let mut fixture = adverse_dispute_fixture();
    fixture.review(
        "approve",
        "The dispute establishes eligibility.",
        "early-approval",
    );
    fixture.state("approvalNoticePending");
    assert!(fixture.current().timers.is_empty());
    fixture.receipt("sendApprovalNotice");
    fixture.state("approved");
    assert!(!fixture.pending_notice("sendDenialNotice"));
    assert_eq!(fixture.clock.0.load(Ordering::SeqCst), START + 9 * DAY);
}

#[test]
fn early_photo_return_cannot_bypass_outside_data_minimum_period() {
    let mut fixture = adverse_dispute_fixture();
    fixture.request_photo();
    fixture.submit(
        "replacePhoto",
        APPLICANT,
        APPLICATION,
        native_inputs()["correction"]["input"].clone(),
        "early-photo-return",
    );
    fixture.state("returnedReview");
    let reason = "The corrected photo does not resolve the adverse outside-data evidence.";
    fixture.review("deny", reason, "returned-adverse-denial");
    fixture.state("outsideDataMinimumPending");
    assert_no_effective_adverse_decision(&fixture);
    fixture.advance(10);
    assert_recorded_denial(&mut fixture, reason);
}

#[test]
fn unanswered_outside_data_window_allows_reasoned_denial_at_full_boundary() {
    let mut fixture = Fixture::new(START, true);
    fixture.receipt("sendOutsideDataNotice");
    fixture.advance(10);
    fixture.state("awaitingReview");
    let reason =
        "The retained application and undisputed outside-data result do not establish eligibility.";
    fixture.review("deny", reason, "unanswered-adverse-denial");
    assert_recorded_denial(&mut fixture, reason);
    assert!(fixture.current().timers.is_empty());
}

/// Expires the photo clock while the no-response notice stays unsent.
fn unsent_no_response_notice_fixture() -> Fixture {
    let mut fixture = Fixture::new(START, false);
    fixture.request_photo();
    fixture.advance(91);
    fixture.state("closedForNoResponse");
    assert_eq!(fixture.current().case_state["caseClosed"], true);
    assert!(
        fixture
            .current()
            .active_tasks
            .iter()
            .any(|task| task.task_ref == "attachLatePhoto")
    );
    assert!(fixture.pending_notice("sendNoResponseNotice"));
    fixture
}

#[test]
fn unsent_notice_allows_late_evidence_and_global_receipts_without_duplicate_tasks() {
    let mut fixture = unsent_no_response_notice_fixture();
    fixture.late_reply();
    let expected = native_inputs()["correction"]["mappingOutput"].clone();
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
    fixture.state("closedWithLateEvidence");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    fixture.receipt("sendNoResponseNotice");
    fixture.receipt("sendLateEvidenceReceivedNotice");
    fixture.state("closedWithLateEvidence");
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
    // Deferred notices persist a request; no provider call reaches the port.
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
fn receipts_remain_routable_after_reopening_and_returned_review_without_task_reentry() {
    let mut fixture = unsent_no_response_notice_fixture();
    fixture.late_reply();
    fixture.reopen("Accepted late evidence warrants reconsideration.", "reopen");
    fixture.state("reopened");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    fixture.receipt("sendNoResponseNotice");
    fixture.receipt("sendLateEvidenceReceivedNotice");
    fixture.state("reopened");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    fixture.receipt("sendReopenedNotice");
    fixture.state("returnedReview");
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    fixture.receipt("sendInformationReceivedNotice");
    fixture.state("returnedReview");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert_eq!(fixture.current().case_state["caseClosed"], false);
}

#[test]
fn receipt_region_stays_open_after_finality_and_keeps_the_approval_notice() {
    let mut fixture = unsent_no_response_notice_fixture();
    fixture.late_reply();
    fixture.reopen("Reconsideration is warranted.", "final-reopen");
    fixture.receipt("sendReopenedNotice");
    fixture.review(
        "approve",
        "The corrected application satisfies eligibility.",
        "final-approval",
    );
    fixture.receipt("sendApprovalNotice");
    fixture.state("approved");
    // Finality belongs to the main case region. No whole-process completion or
    // implicit cancellation is claimed while receipt ingress remains active.
    assert_eq!(
        fixture.current().status,
        wos_core::instance::InstanceStatus::Active
    );
    assert!(fixture.pending_notice("sendNoResponseNotice"));
    let approval_notice = fixture.current().case_state["decisionNotice"].clone();
    assert!(approval_notice["sentAt"].is_string());
    fixture.receipt("sendNoResponseNotice");
    fixture.state("approved");
    // A late no-response receipt lands in its own field, not over the approval.
    let current = fixture.current();
    assert_eq!(current.case_state["decisionNotice"], approval_notice);
    assert!(current.case_state["noResponseNotice"]["sentAt"].is_string());
}

#[test]
fn nonadverse_denied_final_retains_global_receipt_ingress_without_task_recreation() {
    let mut fixture = unsent_no_response_notice_fixture();
    fixture.late_reply();
    fixture.reopen(
        "Accepted late evidence warrants reconsideration.",
        "denial-reopen",
    );
    fixture.receipt("sendReopenedNotice");
    let reason = "The retained application does not meet the stated requirement.";
    fixture.review("deny", reason, "nonadverse-denial");
    assert_recorded_denial(&mut fixture, reason);
    let before = serde_json::to_value(fixture.current().active_tasks).unwrap();
    fixture.receipt("sendNoResponseNotice");
    fixture.receipt("sendLateEvidenceReceivedNotice");
    fixture.receipt("sendInformationReceivedNotice");
    fixture.state("denied");
    assert_eq!(
        serde_json::to_value(fixture.current().active_tasks).unwrap(),
        before
    );
    assert!(fixture.current().pending_outbound_events.is_empty());
}

#[test]
fn receipt_in_closed_case_preserves_late_task_identity() {
    let mut fixture = unsent_no_response_notice_fixture();
    let tasks = serde_json::to_value(&fixture.current().active_tasks).unwrap();
    fixture.receipt("sendNoResponseNotice");
    fixture.state("closedForNoResponse");
    assert_eq!(
        serde_json::to_value(&fixture.current().active_tasks).unwrap(),
        tasks
    );
    assert!(fixture.current().pending_outbound_events.is_empty());
    assert_eq!(fixture.current().case_state["caseClosed"], true);
}

#[test]
fn invalid_and_duplicate_receipts_never_wake_case_or_restart_clock() {
    for mode in [
        "unknown",
        "wrong-binding",
        "wrong-type",
        "missing-id",
        "drift",
        "duplicate",
    ] {
        let mut fixture = Fixture::new(START, false);
        fixture.send_back();
        let outbound = fixture
            .current()
            .pending_outbound_events
            .into_values()
            .find(|pending| pending.binding_id == "sendPhotoRequestNotice")
            .unwrap();
        let mut event = "delivery.sendPhotoRequestNotice.receipt".to_string();
        let mut receipt = json!({"id":"controlled-negative-receipt","source":"urn:test:receipt","specversion":"1.0",
            "type":"com.wos.demo.d1.delivery.receipt","subject":outbound.subject,
            "data":accepted_receipt_data(&outbound.invocation_id, &iso(START), &iso(START))});
        match mode {
            "unknown" => receipt["subject"] = json!("unknown"),
            "wrong-binding" => event = "delivery.sendOutsideDataNotice.receipt".into(),
            "wrong-type" => receipt["type"] = json!("com.example.unrelated"),
            "missing-id" => {
                receipt.as_object_mut().unwrap().remove("id");
            }
            "drift" => {
                // Rebuild only the host-selected profile, leaving persisted request untouched.
                let mut profile = integration();
                profile
                    .bindings
                    .get_mut("sendPhotoRequestNotice")
                    .unwrap()
                    .output_binding
                    .insert("caseFile.foreignReceipt".into(), "$".into());
                fixture.runtime = controlled_runtime(
                    fixture.store.clone(),
                    fixture.clock.clone(),
                    fixture.ports.clone(),
                    profile,
                );
            }
            _ => fixture.receipt("sendPhotoRequestNotice"),
        }
        fixture
            .runtime
            .enqueue_event(
                &fixture.process.process_id,
                wos_core::instance::PendingEvent {
                    origin: None,
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
                .any(|pending| pending.event == "photo.noticeSent")
        );
    }
}

#[test]
fn closed_case_idle_and_reload_preserve_exact_pending_work_without_busy_loop() {
    let mut fixture = unsent_no_response_notice_fixture();
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
    fixture.runtime = controlled_runtime(
        fresh,
        fixture.clock.clone(),
        fixture.ports.clone(),
        integration(),
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
    fixture.state("closedForNoResponse");
}
