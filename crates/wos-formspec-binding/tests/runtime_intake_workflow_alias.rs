//! Integration tests for [`wos_formspec_binding::FormspecBinding`] registered as
//! the WOS intake acceptor together with [`wos_runtime::WosRuntime`].
//!
//! ## Why these tests live in this crate
//!
//! `wos-formspec-binding` depends on `wos-runtime`. A **`wos-runtime` dev-dependency**
//! on `wos-formspec-binding` would pull a second `wos-runtime` into the build graph, so
//! [`wos_runtime::intake::IntakeAcceptanceAdapter`] no longer matches
//! [`wos_formspec_binding::FormspecBinding`] at the trait-object boundary. Register
//! `FormspecBinding` only from this crate's integration tests (or a dedicated
//! glue crate), never from `wos-runtime`'s own test binary via dev-deps.
//!
//! ## Scenarios
//!
//! - Workflow attach with a case-ledger id: handoff `caseRef` string must stay
//!   consistent with [`wos_formspec_binding::FormspecBinding::finalize_intake_acceptance`]
//!   while the runtime stores a distinct process id (`outcome_for_binding_finalize` in
//!   `wos-runtime`).
//! - Public intake with a requested governed-case TypeID: acceptance must create a
//!   process bound to that canonical case ledger.

use std::collections::HashMap;

use serde_json::json;
use wos_core::instance::{FormspecTaskContext, WorkflowProcess};
use wos_core::model::kernel::KernelDocument;
use wos_core::provenance::ProvenanceKind;
use wos_core::traits::{DocumentResolver, ExternalService, TaskPresenter};
use wos_formspec_binding::{FormspecBinding, FormspecProcessor};
use wos_runtime::binding::{BindingError, BindingRegistry, CaseMutationBundle};
use wos_runtime::{
    AutoCreatePublicIntakePolicy, Clock, CreateInstanceRequest, InMemoryStore,
    IntakeAcceptanceOutcome, IntakeAcceptanceRegistry, IntakeAcceptanceRequest,
    IntakeCaseDefinition, IntakeCaseDisposition, WosRuntime,
};

#[derive(Debug, Clone)]
struct FixedClock {
    now_ms: u64,
}

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

#[derive(Debug, Clone)]
struct SingleKernelResolver {
    kernels: HashMap<(String, String), KernelDocument>,
}

impl SingleKernelResolver {
    fn new(kernel: KernelDocument) -> Self {
        let url = kernel.url.clone().expect("kernel url");
        let version = kernel.version.clone().expect("kernel version");
        let mut kernels = HashMap::new();
        kernels.insert((url, version), kernel);
        Self { kernels }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("resolver: {0}")]
struct ResolverError(String);

impl DocumentResolver for SingleKernelResolver {
    type Error = ResolverError;

    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        self.kernels
            .get(&(url.to_string(), version.to_string()))
            .cloned()
            .ok_or_else(|| ResolverError(format!("{url}@{version}")))
    }

    fn resolve_governance(
        &self,
        _url: &str,
        _version: &str,
    ) -> Result<wos_core::GovernanceDocument, Self::Error> {
        Err(ResolverError("unused".into()))
    }

    fn resolve_sidecar(
        &self,
        _url: &str,
        _anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Err(ResolverError("unused".into()))
    }
}

#[derive(Debug, Clone, Default)]
struct NoopPresenter;

#[derive(Debug, thiserror::Error)]
#[error("presenter: {0}")]
struct PresenterError(String);

impl TaskPresenter for NoopPresenter {
    type Error = PresenterError;

    fn present_task(&mut self, _context: &FormspecTaskContext) -> Result<(), Self::Error> {
        Ok(())
    }

    fn dismiss_task(&mut self, _task_id: &str, _reason: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct NullService;

#[derive(Debug, thiserror::Error)]
#[error("service: {0}")]
struct ServiceError(String);

impl ExternalService for NullService {
    type Error = ServiceError;

    fn invoke(
        &self,
        _service_ref: &str,
        _input: &serde_json::Value,
        _idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Ok(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, Default)]
struct IntakeRuntimeProcessor;

impl FormspecProcessor for IntakeRuntimeProcessor {
    fn validate_envelope(
        &self,
        _response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError> {
        Ok(Vec::new())
    }

    fn validate_definition(
        &self,
        _definition_url: &str,
        _definition_version: &str,
        _data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
        Ok(None)
    }

    fn compute_prefill(
        &self,
        _mapping_ref: Option<&str>,
        _case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError> {
        Ok(None)
    }

    fn map_response(
        &self,
        _mapping_ref: &str,
        _response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        Ok(None)
    }
}

fn public_intake_handoff(
    handoff_id: &str,
    definition_url: &str,
    definition_version: &str,
) -> serde_json::Value {
    json!({
        "$formspecIntakeHandoff": "1.0",
        "handoffId": handoff_id,
        "initiationMode": "publicIntake",
        "definitionRef": {
            "url": definition_url,
            "version": definition_version
        },
        "responseRef": "urn:formspec:response:resp-public-it-1",
        "responseHash": "sha256:0123456789abcdef",
        "validationReportRef": "urn:formspec:validation-report:vr-public-it-1",
        "intakeSessionId": "session-public-it-1",
        "ledgerHeadRef": "urn:formspec:respondent-ledger-event:evt-public-it-1",
        "occurredAt": "2026-04-22T17:15:00Z"
    })
}

fn workflow_handoff(case_ref: &str, handoff_id: &str) -> serde_json::Value {
    json!({
        "$formspecIntakeHandoff": "1.0",
        "handoffId": handoff_id,
        "initiationMode": "workflowInitiated",
        "caseRef": case_ref,
        "definitionRef": {
            "url": "urn:test:formspec-intake-runtime-it",
            "version": "1.0.0"
        },
        "responseRef": "urn:formspec:response:resp-it-1",
        "responseHash": "sha256:0123456789abcdef",
        "validationReportRef": "urn:formspec:validation-report:vr-it-1",
        "intakeSessionId": "session-it-1",
        "ledgerHeadRef": "urn:formspec:respondent-ledger-event:evt-it-1",
        "occurredAt": "2026-04-22T17:15:00Z"
    })
}

const TEST_PROCESS_ID: &str = "default_process_01hw7rm71vfay8vvw14d2pf2db";
const TEST_CASE_LEDGER_ID: &str = "default_case_01hw7rm71vfay8vvw14d2pf2db";

#[test]
fn runtime_formspec_intake_workflow_attach_uses_case_ledger_identity() {
    let kernel: KernelDocument = serde_json::from_value(json!({
        "$wosWorkflow": "1.0",
        "url": "urn:test:formspec-intake-runtime-it",
        "version": "1.0.0",
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        }
    }))
    .expect("kernel");

    let mut intake = IntakeAcceptanceRegistry::new();
    intake.register(FormspecBinding::new(IntakeRuntimeProcessor));

    let mut runtime = WosRuntime::new(
        InMemoryStore::new(),
        SingleKernelResolver::new(kernel),
        NoopPresenter::default(),
        wos_core::traits::DefaultRuntime::new(),
        NullService::default(),
        wos_core::traits::DefaultRuntime::new(),
        FixedClock {
            now_ms: 1_710_000_000_000,
        },
        BindingRegistry::new(),
    )
    .with_intake_acceptors(intake);

    let created = runtime
        .create_instance_bound_to_case(
            CreateInstanceRequest {
                process_id: TEST_PROCESS_ID.to_string(),
                tenant: None,
                definition_url: "urn:test:formspec-intake-runtime-it".to_string(),
                definition_version: "1.0.0".to_string(),
                initial_case_state: None,
            },
            TEST_CASE_LEDGER_ID.to_string(),
        )
        .expect("create_instance");

    let decision = runtime
        .accept_intake_handoff(
            "formspec",
            IntakeAcceptanceRequest {
                document: workflow_handoff(TEST_CASE_LEDGER_ID, "ih-runtime-it-1"),
                actor_id: Some("intake-service".to_string()),
                governed_case_ref: None,
                governed_case_definition: None,
                initial_case_state: None,
            },
        )
        .expect("accept_intake_handoff");

    assert_eq!(
        decision.outcome,
        IntakeAcceptanceOutcome::Accepted {
            case_disposition: IntakeCaseDisposition::AttachToExistingCase {
                case_ref: created.case_ledger_id.clone()
            }
        }
    );

    let window = runtime
        .load_provenance_window(&created.process_id, 0, 20)
        .expect("provenance window");
    assert!(
        window
            .iter()
            .any(|r| r.record_kind == ProvenanceKind::IntakeAccepted),
        "intake provenance appended to canonical case"
    );
}

#[test]
fn runtime_formspec_intake_public_create_uses_governed_case_ref() {
    let def_url = "urn:test:formspec-intake-public-it";
    let def_version = "1.0.0";
    let kernel: KernelDocument = serde_json::from_value(json!({
        "$wosWorkflow": "1.0",
        "url": def_url,
        "version": def_version,
        "lifecycle": {
            "initialState": "open",
            "states": { "open": { "type": "atomic" } }
        }
    }))
    .expect("kernel");

    let mut intake = IntakeAcceptanceRegistry::new();
    intake.register(FormspecBinding::new(IntakeRuntimeProcessor));

    let mut runtime = WosRuntime::new(
        InMemoryStore::new(),
        SingleKernelResolver::new(kernel),
        NoopPresenter::default(),
        wos_core::traits::DefaultRuntime::new(),
        NullService::default(),
        wos_core::traits::DefaultRuntime::new(),
        FixedClock {
            now_ms: 1_710_000_000_001,
        },
        BindingRegistry::new(),
    )
    .with_intake_acceptors(intake)
    .with_intake_policy(AutoCreatePublicIntakePolicy);

    let decision = runtime
        .accept_intake_handoff(
            "formspec",
            IntakeAcceptanceRequest {
                document: public_intake_handoff("ih-public-it-legacy", def_url, def_version),
                actor_id: Some("intake-service".to_string()),
                governed_case_ref: Some(TEST_CASE_LEDGER_ID.to_string()),
                governed_case_definition: Some(IntakeCaseDefinition {
                    definition_url: def_url.to_string(),
                    definition_version: def_version.to_string(),
                }),
                initial_case_state: Some(json!({ "source": "publicIntakeIt" })),
            },
        )
        .expect("accept_intake_handoff");

    let canonical = match &decision.outcome {
        IntakeAcceptanceOutcome::Accepted {
            case_disposition: IntakeCaseDisposition::CreateGovernedCase { case_ref, .. },
        } => case_ref.clone(),
        other => panic!("expected accepted create: {other:?}"),
    };

    assert!(
        WorkflowProcess::is_case_id(&canonical),
        "public intake acceptance must return canonical governed case id"
    );
    assert_eq!(canonical, TEST_CASE_LEDGER_ID);

    let by_case = runtime
        .load_instance(&canonical)
        .expect("load by governed_case_ref");
    assert_eq!(by_case.case_ledger_id, canonical);
    assert_ne!(by_case.process_id, by_case.case_ledger_id);

    let window = runtime
        .load_provenance_window(&canonical, 0, 30)
        .expect("provenance window");
    assert!(
        window
            .iter()
            .any(|r| r.record_kind == ProvenanceKind::IntakeAccepted),
        "expected IntakeAccepted on new case"
    );
    assert!(
        window
            .iter()
            .any(|r| r.record_kind == ProvenanceKind::CaseCreated),
        "expected CaseCreated from FormspecBinding finalizer"
    );
}
