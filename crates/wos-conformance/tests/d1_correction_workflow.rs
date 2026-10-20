// Rust guideline compliant 2026-02-21

//! Exercises the authored D1 lifecycle with controlled host ports.
//!
//! Definition validation and Mapping execution remain host responsibilities;
//! the processor double supplies their selected outputs, not production proof.

use serde_json::{Value, json};
use stack_common_typeid::testing::fixture_minter;
use wos_conformance::stubs::StubService;
use wos_core::instance::WorkflowProcess;
use wos_core::model::governance::DelegationScope;
use wos_core::model::{governance::GovernanceDocument, kernel::KernelDocument};
use wos_core::traits::{AccessControl, DefaultRuntime, DocumentResolver};
use wos_formspec_binding::{FormspecBinding, FormspecProcessor};
use wos_runtime::{
    BindingError, BindingRegistry, CaseMutationBundle, CreateProcessRequest, HumanActorBinding,
    RuntimeStore, SharedInMemoryStore, SystemClock, TaskSubmissionResult, WosRuntime,
};

const WORKFLOW: &str = include_str!("../../../examples/d1-passport-correction.workflow.json");
const REVIEW: &str = "urn:formspec:demo:d1:passport-review";
const CORRECTION: &str = "urn:formspec:demo:d1:passport-photo-correction";

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

struct SelectedMappingOutputs;
impl FormspecProcessor for SelectedMappingOutputs {
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
            None => Ok(None),
            Some("urn:formspec:demo:d1:passport-photo-correction:mapping:prefill") => Ok(Some(
                json!({"originalFilingRef": case["originalFilingRef"]}),
            )),
            Some(_) => Err(BindingError::InvalidInput(
                "unselected prefill mapping".into(),
            )),
        }
    }
    fn map_response(
        &self,
        mapping: &str,
        response: &Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let fields: &[(&str, &str)] = match mapping {
            "urn:formspec:demo:d1:passport-review:mapping:case" => {
                &[("choice", "reviewChoice"), ("reason", "reviewReason")]
            }
            "urn:formspec:demo:d1:passport-photo-correction:mapping:case" => &[
                ("replacementPhoto", "correctionPhoto"),
                ("originalFilingRef", "correctionFilingRef"),
            ],
            _ => return Err(BindingError::InvalidInput("unselected mapping".into())),
        };
        let field_updates = fields
            .iter()
            .map(|(source, target)| (target.to_string(), response["data"][source].clone()))
            .collect();
        Ok(Some(CaseMutationBundle { field_updates }))
    }
}

fn runtime() -> (WosRuntime, SharedInMemoryStore, WorkflowProcess) {
    let kernel: KernelDocument = serde_json::from_str(WORKFLOW).unwrap();
    let store = SharedInMemoryStore::default();
    let mut bindings = BindingRegistry::new();
    bindings.register(FormspecBinding::new(SelectedMappingOutputs));
    let mut runtime = WosRuntime::new(
        store.clone(),
        Resolver(kernel),
        DefaultRuntime::new(),
        AssignmentOnly,
        StubService::null_response(),
        DefaultRuntime::new(),
        SystemClock,
        bindings,
    );
    let process = runtime
        .create_process_with_actor_bindings(
            CreateProcessRequest {
                process_id: fixture_minter().mint_process_id(),
                tenant: None,
                definition_url: "urn:wos:demo:d1:passport-renewal".into(),
                definition_version: "0.2.0".into(),
                initial_case_state: Some(json!({"originalFilingRef":"filing-original",
            "originalAnswers":{"name":"Alex Rivera"}, "originalPhoto":{"digest":"old-photo"}})),
            },
            vec![
                HumanActorBinding {
                    declared_actor: "adjudicator".into(),
                    assigned_actor: "actor:human:reviewer".into(),
                },
                HumanActorBinding {
                    declared_actor: "applicant".into(),
                    assigned_actor: "actor:human:applicant".into(),
                },
            ],
        )
        .unwrap();
    (runtime, store, process)
}

fn submit(
    runtime: &mut WosRuntime,
    process: &WorkflowProcess,
    actor: &str,
    url: &str,
    data: Value,
    token: &str,
) -> TaskSubmissionResult {
    let task = &runtime
        .load_process(&process.process_id)
        .unwrap()
        .active_tasks[0];
    assert_eq!(task.definition_url.as_deref(), Some(url));
    assert_eq!(task.definition_version.as_deref(), Some("0.2.0"));
    let result = runtime
        .submit_task_response(
            &task.task_id,
            integrity_canonical::ResponseJson::from_value(
                &(json!({"status":"completed",
        "definitionUrl":url,"definitionVersion":"0.2.0","data":data})),
            )
            .unwrap(),
            actor,
            Some(token),
        )
        .unwrap();
    assert!(matches!(result, TaskSubmissionResult::Completed { .. }));
    runtime.drain_until_idle(&process.process_id).unwrap();
    result
}

#[test]
fn photo_reply_returns_review_and_decision_preserves_original_and_replay() {
    let (mut runtime, store, process) = runtime();
    let original = process.case_state.clone();
    submit(
        &mut runtime,
        &process,
        "actor:human:reviewer",
        REVIEW,
        json!({"choice":"sendBackPhoto","reason":"Photo does not show a clear face."}),
        "send-back",
    );
    let waiting = runtime.load_process(&process.process_id).unwrap();
    assert_eq!(waiting.configuration, vec!["awaitingPhoto"]);
    assert_eq!(waiting.active_tasks.len(), 1);
    let photo_task = waiting.active_tasks[0].clone();
    assert_eq!(photo_task.task_ref, "replacePhoto");
    assert_eq!(
        photo_task.context.as_ref().unwrap().prefill_data,
        Some(json!({"originalFilingRef":"filing-original"}))
    );
    assert_eq!(
        photo_task.assigned_actor.as_deref(),
        Some("actor:human:applicant")
    );
    let data =
        json!({"replacementPhoto":{"digest":"new-photo"},"originalFilingRef":"filing-original"});
    assert!(runtime.submit_task_response(&photo_task.task_id,
        integrity_canonical::ResponseJson::from_value(&(json!({"status":"completed","definitionUrl":CORRECTION,"definitionVersion":"0.2.0","data":data}))).unwrap(),
        "actor:human:reviewer", Some("wrong-actor")).is_err());
    let correction = submit(
        &mut runtime,
        &process,
        "actor:human:applicant",
        CORRECTION,
        data.clone(),
        "photo-reply",
    );
    let returned = runtime.load_process(&process.process_id).unwrap();
    assert_eq!(returned.configuration, vec!["awaitingReview"]);
    assert_eq!(returned.active_tasks[0].task_ref, "reviewApplication");
    assert_ne!(
        returned.active_tasks[0].task_id,
        process.active_tasks[0].task_id
    );
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
        data["replacementPhoto"]
    );
    let before = store
        .0
        .lock()
        .unwrap()
        .load_record(&process.process_id)
        .unwrap();
    let replay = runtime.submit_task_response(&photo_task.task_id,
        integrity_canonical::ResponseJson::from_value(&(json!({"status":"completed","definitionUrl":CORRECTION,"definitionVersion":"0.2.0","data":data}))).unwrap(),
        "actor:human:applicant", Some("photo-reply")).unwrap();
    match (replay, correction) {
        (
            TaskSubmissionResult::Completed {
                artifact_id: replay_id,
                case_mutated: replay_mutated,
                emitted_event: replay_event,
            },
            TaskSubmissionResult::Completed {
                artifact_id,
                case_mutated,
                emitted_event,
            },
        ) => {
            assert_eq!(replay_id, artifact_id);
            assert_eq!(replay_mutated, case_mutated);
            assert_eq!(replay_event, emitted_event);
        }
        _ => panic!("correction replay must retain its committed completed outcome"),
    }
    let after = store
        .0
        .lock()
        .unwrap()
        .load_record(&process.process_id)
        .unwrap();
    assert_eq!(before.provenance_log.len(), after.provenance_log.len());
    assert_eq!(before.artifacts.len(), after.artifacts.len());
    submit(
        &mut runtime,
        &process,
        "actor:human:reviewer",
        REVIEW,
        json!({"choice":"approve","reason":"Replacement photo resolves the stated issue."}),
        "decision",
    );
    let decided = runtime.load_process(&process.process_id).unwrap();
    assert_eq!(decided.configuration, vec!["approved"]);
    assert!(decided.active_tasks.is_empty());
    assert_eq!(decided.actor_bindings, process.actor_bindings);
    assert_eq!(
        decided.case_state["originalAnswers"],
        original["originalAnswers"]
    );
    let record = store
        .0
        .lock()
        .unwrap()
        .load_record(&process.process_id)
        .unwrap();
    assert_eq!(record.artifacts.len(), 3);
    record.validate_creation().unwrap();
}

#[test]
fn denial_is_a_completed_reasoned_review_not_a_task_failure() {
    let (mut runtime, _, process) = runtime();
    submit(
        &mut runtime,
        &process,
        "actor:human:reviewer",
        REVIEW,
        json!({"choice":"deny","reason":"The submitted photo remains unsuitable."}),
        "denial",
    );
    let denied = runtime.load_process(&process.process_id).unwrap();
    assert_eq!(denied.configuration, vec!["denied"]);
    assert!(denied.active_tasks.is_empty());
    assert_eq!(
        denied.case_state["reviewReason"],
        "The submitted photo remains unsuitable."
    );
}

#[test]
fn authored_candidate_has_no_native_lint_errors() {
    let diagnostics = wos_lint::lint_document(WORKFLOW).unwrap();
    let errors: Vec<_> = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == wos_lint::LintSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "candidate lint errors: {errors:?}");
}
