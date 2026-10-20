// Rust guideline compliant 2026-02-21
//! Selected public runtime operations preserve actual causal governance boundaries.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use wos_core::instance::{HumanActorBinding, PendingEvent};
use wos_core::model::due_process::*;
use wos_core::model::{governance::GovernanceDocument, kernel::KernelDocument};
use wos_core::traits::{DefaultRuntime, DocumentResolver, ExternalService};
use wos_runtime::runtime::governance::*;
use wos_runtime::store::{InMemoryStore, RuntimeStore};
use wos_runtime::{
    BindingRegistry, Clock, CreateProcessRequest, RuntimeError,
    SelectedDeterminationSignatureVerifier, WosRuntime,
};

#[derive(Clone)]
struct Time(Arc<AtomicU64>);
impl Clock for Time {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
struct Resolver(KernelDocument);
impl DocumentResolver for Resolver {
    type Error = std::io::Error;
    fn resolve_kernel(&self, _: &str, _: &str) -> Result<KernelDocument, Self::Error> {
        Ok(self.0.clone())
    }
    fn resolve_governance(&self, _: &str, _: &str) -> Result<GovernanceDocument, Self::Error> {
        Err(std::io::Error::other("unselected governance"))
    }
    fn resolve_sidecar(&self, _: &str, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("unselected sidecar"))
    }
}
struct Services(Arc<AtomicUsize>);
impl ExternalService for Services {
    type Error = std::io::Error;
    fn invoke(&self, _: &str, _: &Value, _: Option<&str>) -> Result<Value, Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(std::io::Error::other("unselected provider"))
    }
}
fn bytes_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn source() -> Value {
    json!({"sourceVersion":"source","sectionAnchor":"policy.1","retainedSha256":bytes_digest(b"illustrative selected policy")})
}
fn unconfigured_fixture() -> Value {
    let source = source();
    json!({"$wosWorkflow":"1.0","url":"urn:test:reference-due","version":"1.0.0","impactLevel":"rights-impacting",
    "actors":[{"id":"determiner","type":"human"},{"id":"supervisor","type":"human"},{"id":"household","type":"human"},{"id":"reviewer","type":"human"}],
    "caseFile":{"fields":{"income":{"type":"number","default":150},"group":{"type":"string","default":"a"},"level":{"type":"number","default":100},"recipient":{"type":"string","default":"household"}}},
    "decisionTables":[{"id":"eligibility","hitPolicy":"first","inputs":[{"name":"income","type":"number"}],"outputs":[{"name":"adverse","type":"boolean"}],
    "rows":[{"id":"high","inputCells":["income > 100"],"outputCells":["true"]},{"id":"low","inputCells":["income <= 100"],"outputCells":["false"]}]}],
    "lifecycle":{"initialState":"open","states":{
      "open":{"type":"atomic","transitions":[{"event":"decide","target":"pending","actor":"determiner","tags":["determination","adverse-decision"],"guard":{"kind":"decisionTable","ref":"eligibility","outputColumn":"adverse","inputBindings":{"income":"caseFile.income"}}}]},
      "pending":{"type":"atomic","transitions":[{"event":"issue","target":"noticed","actor":"supervisor"},{"event":"appeal","target":"pending","actor":"household"},{"event":"extend","target":"pending","actor":"supervisor"}]},
      "noticed":{"type":"atomic","transitions":[{"event":"effect","target":"applied","actor":"supervisor","actions":[{"action":"setData","path":"caseFile.level","value":0}]}]},
      "applied":{"type":"final"}
    }},
    "governance":{"dueProcess":{"adverseDecisionPolicy":{"noticeRequired":true,"noticeTiming":"beforeEffective","noticeGracePeriod":"P10D","noticeTemplateKey":"notice","explanationLevel":"individualized","counterfactualRequired":true,"appealMechanism":{"enabled":true,"appealWindow":"P90D","continuationOfServices":true,"continuationScope":"currentLevel"}},
      "execution":{"profile":"referenceDueProcess/1","determinations":[{"id":"adverse","transition":{"statePointer":"/lifecycle/states/open","transitionIndex":0},"pendingStatePointer":"/lifecycle/states/pending","appliedStatePointers":["/lifecycle/states/applied"],"servicePointers":["/level"],"signatureTaskRef":"supervisor-signature","sourceRefs":[source.clone()],
      "explanation":{"tableRef":"eligibility","outputColumn":"adverse","beneficialOutput":{"kind":"boolean","value":false},"reasons":[{"tableRef":"eligibility","rowId":"high","reasonCode":"income","sourceAuthority":"policy","description":"Income exceeds the selected limit.","sourceRefs":[source.clone()]}],
      "positive":[{"id":"lower-income","changes":[{"casePointer":"/income","value":{"kind":"number","value":50}}],"description":"Lower income changes the selected result.","sourceRefs":[source.clone()]}],
      "negative":[{"id":"other-group","changes":[{"casePointer":"/group","value":{"kind":"string","value":"b"}}],"description":"This executed group alternative preserves the result.","sourceRefs":[source.clone()]}]}}],
      "operations":[
      {"id":"issue","kind":"issueNotice","determinationRef":"adverse","actors":["supervisor"],"requiredStatePointers":["/lifecycle/states/pending"],"triggerEvent":"issue","transition":{"statePointer":"/lifecycle/states/pending","transitionIndex":0},"sourceRefs":[source.clone()]},
      {"id":"appeal","kind":"fileAppeal","determinationRef":"adverse","actors":["household"],"requiredStatePointers":["/lifecycle/states/pending"],"triggerEvent":"appeal","transition":{"statePointer":"/lifecycle/states/pending","transitionIndex":1},"sourceRefs":[source.clone()]},
      {"id":"effect","kind":"applyAdverseEffect","determinationRef":"adverse","actors":["supervisor"],"requiredStatePointers":["/lifecycle/states/noticed"],"triggerEvent":"effect","transition":{"statePointer":"/lifecycle/states/noticed","transitionIndex":0},"sourceRefs":[source.clone()]}],
      "notice":{"recipientPointer":"/recipient","deliveryChannel":"portal","templates":[{"locale":"en","templateKey":"notice","review":{"id":"review","sha256":bytes_digest(b"selected template review")}}],"evidenceClass":"fixtureQualifiedReceipt","appealAnchor":"noticeIssued","graceAnchor":"qualifiedReceipt","sourceRefs":[source.clone()]},
      "appeal":{"channels":["phone","in-person","written"],"standingActors":["household"],"reviewerActors":["reviewer"],"excludedDeterminationActors":"determinerAndSupervisor","hearingDeadline":{"calendarDays":30,"purpose":"hearingHeld"},"continuation":{"choice":"defaultUnlessWaived","scope":"currentLevel","endsOn":"dispositionOrHouseholdTermination"},"sourceRefs":[source]},
      "limits":{"maxDeterminations":10,"maxNotices":10,"maxAppeals":10,"maxOperations":50,"maxOutstandingNoticeWork":10,"maxTransportAttempts":3,"maxNoticeBytes":100000}
    }}}})
}
fn fixture() -> Value {
    let mut raw = unconfigured_fixture();
    raw["contracts"] = json!({"sign":{"binding":"formspec","ref":"urn:test:supervisor-form","definitionVersion":"1.0.0"}});
    raw["lifecycle"]["states"]["pending"]["onEntry"] = json!([{"action":"createTask","taskRef":"supervisor-signature","assignTo":"supervisor","contractRef":"sign","extensions":{"x-wos-signature-profile-key":"signatureProfile","x-wos-signature-step-id":"supervisorSigns"}}]);
    raw["signature"] = serde_json::to_value(signature_profile()).unwrap();
    raw["governance"]["audit"] = json!({"reasoningTier":{"requiredForTags":["determination"],"requireDecisionRequirements":false},"counterfactualTier":{"requiredForTags":["adverse-decision"],"requireProtectedCharacteristics":true}});
    raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["protectedCharacteristics"] =
        json!([{"casePointer":"/group","sourceRefs":[source()]}]);
    raw
}
fn capabilities(raw: &Value) -> ReferenceDueProcessCapabilities {
    use ring::signature::KeyPair;
    let public = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[41; 32]).unwrap();
    ReferenceDueProcessCapabilities{workflow_bytes:serde_json::to_vec(raw).unwrap(),
      retained_evidence:vec![EvidenceSelection{id:"source".into(),role:EvidenceRole::Source,bytes_base64:STANDARD.encode(b"illustrative selected policy"),sha256:bytes_digest(b"illustrative selected policy")},
      EvidenceSelection{id:"review".into(),role:EvidenceRole::Review,bytes_base64:STANDARD.encode(b"selected template review"),sha256:bytes_digest(b"selected template review")}],
      templates:vec![serde_json::from_value(json!({"templates":{"notice":{"category":"adverse-decision","localeRef":"en","sections":[{"id":"facts","contentType":"text","content":"{{recipient}} {{appealDeadline}}"}]}}})).unwrap()],
      notice_transport:Box::new(DeterministicNoticeTransport::new([])),signature_verifier:Box::new(SelectedDeterminationSignatureVerifier::new(vec![wos_runtime::SelectedDocumentSigningKey{id:"selected-source".into(),task_ref:"supervisor-signature".into(),signer_human:"actor:human:supervisor".into(),kid_base64:STANDARD.encode(b"source"),public_key_base64:STANDARD.encode(public.public_key().as_ref()),method_uri:"urn:formspec:sig-method:ed25519-cose-sign1@1".into()}]).unwrap())}
}
fn runtime(raw: &Value) -> (WosRuntime, Time, Arc<AtomicUsize>) {
    let time = Time(Arc::new(AtomicU64::new(0)));
    let calls = Arc::new(AtomicUsize::new(0));
    let processor = wos_formspec_binding::selected::ImmutableFormspecProcessor::new(&[
        wos_formspec_binding::selected::SelectedDocument {
            kind: wos_formspec_binding::selected::SelectedDocumentKind::Definition,
            url: "urn:test:supervisor-form".into(),
            version: "1.0.0".into(),
            bytes: serde_json::to_vec(&form()).unwrap(),
            definition: None,
        },
    ])
    .unwrap()
    .with_clock(time.clone());
    let mut bindings = BindingRegistry::new();
    bindings.register(wos_formspec_binding::FormspecBinding::new(processor));
    let mut runtime = WosRuntime::new(
        InMemoryStore::new(),
        Resolver(serde_json::from_value(raw.clone()).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Services(calls.clone()),
        DefaultRuntime::new(),
        time.clone(),
        bindings,
    )
    .with_signature_profile("signatureProfile", signature_profile());
    runtime
        .configure_reference_due_process(capabilities(raw))
        .unwrap();
    (runtime, time, calls)
}
fn create(runtime: &mut WosRuntime) -> Result<String, RuntimeError> {
    create_with_initial_case(runtime, None)
}
fn create_with_initial_case(
    runtime: &mut WosRuntime,
    initial_case_state: Option<Value>,
) -> Result<String, RuntimeError> {
    runtime
        .create_process_with_actor_bindings(
            CreateProcessRequest {
                process_id: String::new(),
                tenant: Some("test".into()),
                definition_url: "urn:test:reference-due".into(),
                definition_version: "1.0.0".into(),
                initial_case_state,
            },
            ["determiner", "supervisor", "household", "reviewer"]
                .iter()
                .map(|id| HumanActorBinding {
                    declared_actor: (*id).into(),
                    assigned_actor: format!("actor:human:{id}"),
                })
                .collect(),
        )
        .map(|p| p.process_id)
}
fn enqueue(runtime: &mut WosRuntime, process: &str, event: &str, actor: &str) {
    runtime
        .enqueue_event(
            process,
            PendingEvent {
                event: event.into(),
                actor_id: Some(actor.into()),
                data: None,
                timestamp: String::new(),
                origin: None,
                idempotency_token: None,
            },
        )
        .unwrap();
}
#[test]
fn actual_table_rows_and_alternatives_create_pending_determination_only() {
    let (mut runtime, _, calls) = runtime(&fixture());
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    let drain = runtime.drain_once(&process).unwrap();
    let record = runtime.load_reference_record(&process).unwrap();
    let state = record
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap();
    assert_eq!(state.determinations.len(), 1);
    assert_eq!(
        state.determinations[0].id,
        drain.transitions[0].provenance_record_id
    );
    assert_eq!(record.process.case_state["level"], 100);
    assert!(state.notices.is_empty());
    assert!(state.operations.is_empty());
    assert_eq!(
        drain
            .provenance
            .iter()
            .filter(|p| p.record_kind == wos_core::ProvenanceKind::DeterminationCounterfactual)
            .count(),
        2
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[test]
fn initial_and_intermediate_service_writes_refuse_before_save_or_provider() {
    for initial in [true, false] {
        let mut raw = fixture();
        let actions = json!([{"action":"setData","path":"caseFile.level","value":0},{"action":"setData","path":"caseFile.level","value":100},{"action":"invokeService","serviceRef":"external"}]);
        if initial {
            raw["lifecycle"]["states"]["open"]["onEntry"] = actions;
        } else {
            raw["lifecycle"]["states"]["open"]["transitions"][0]["actions"] = actions;
        }
        let (mut runtime, _, calls) = runtime(&raw);
        if initial {
            assert!(create(&mut runtime).is_err());
        } else {
            let process = create(&mut runtime).unwrap();
            enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
            let before = runtime.load_reference_record(&process).unwrap();
            assert!(runtime.drain_once(&process).is_err());
            let after = runtime.load_reference_record(&process).unwrap();
            assert_eq!(after.provenance_log.len(), before.provenance_log.len());
            assert_eq!(after.process.case_state, before.process.case_state);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn generic_appeal_and_unsigned_notice_cannot_create_success() {
    let (mut runtime, _, _) = runtime(&fixture());
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    enqueue(&mut runtime, &process, "appeal", "actor:human:household");
    assert!(matches!(
        runtime.drain_once(&process),
        Err(RuntimeError::Governance(RefusalCode::InvocationRequired))
    ));
    let determination = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .determinations[0]
        .id
        .clone();
    let result = runtime
        .governance_operation(
            &process,
            "actor:human:supervisor",
            GovernanceRequest {
                operation_ref: "issue".into(),
                determination_id: Some(determination),
                idempotency_token: "issue1".into(),
                actor: GovernanceActor {
                    declared_actor: "supervisor".into(),
                    acting_actor: "actor:human:supervisor".into(),
                },
                command: GovernanceCommand::IssueNotice,
            },
        )
        .unwrap();
    assert_eq!(
        result,
        GovernanceResult::Refused {
            code: RefusalCode::SignatureRejected
        }
    );
}
#[test]
fn foreign_actual_actor_and_failed_counterfactual_refuse() {
    for wrong_counterfactual in [false, true] {
        let mut raw = fixture();
        if wrong_counterfactual {
            raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["positive"]
                [0]["changes"][0]["value"]["value"] = json!(200);
        }
        let (mut runtime, _, _) = runtime(&raw);
        let process = create(&mut runtime).unwrap();
        enqueue(
            &mut runtime,
            &process,
            "decide",
            if wrong_counterfactual {
                "actor:human:determiner"
            } else {
                "actor:human:foreign"
            },
        );
        assert!(runtime.drain_once(&process).is_err());
        assert_eq!(
            runtime.load_process(&process).unwrap().configuration,
            vec!["open"]
        );
    }
}
#[test]
fn transport_recovers_original_success_and_never_backdates_fresh_success() {
    let transport = DeterministicNoticeTransport::new([1]);
    let work = PendingNoticeWork {
        process_id: "process".into(),
        notice_id: "record".into(),
        determination_id: "determination".into(),
        transport_key: "stable".into(),
        correlation: "same".into(),
        artifact_sha256: bytes_digest(b"bytes"),
        artifact_bytes_base64: STANDARD.encode(b"bytes"),
        recipient: "household".into(),
        channel: DeliveryChannel::Portal,
        issue_at: "1970-01-01T00:00:00.000Z".into(),
    };
    assert!(matches!(
        transport.execute(&work, &work.issue_at).unwrap(),
        TransportResult::Failed { .. }
    ));
    assert!(transport.lookup(&work).unwrap().is_none());
    let later = "1970-01-01T00:00:00.001Z";
    let success = transport.execute(&work, later).unwrap();
    let TransportResult::Qualified { receipt, .. } = &success else {
        panic!("actual qualified result")
    };
    assert_eq!(receipt.qualified_at, later);
    assert_ne!(receipt.qualified_at, work.issue_at);
    let recreated = transport.clone();
    assert_eq!(recreated.lookup(&work).unwrap(), Some(success.clone()));
    assert_eq!(
        recreated
            .execute(&work, "1970-01-02T00:00:00.000Z")
            .unwrap(),
        success
    );
    let mut changed = work.clone();
    changed.recipient = "foreign".into();
    assert_eq!(
        recreated.execute(&changed, later),
        Err(RefusalCode::ReplayConflict)
    );
    assert_eq!(transport.invocations("process", "stable").unwrap(), 2);
}

fn signed_fixture() -> Value {
    let mut raw = fixture();
    raw["contracts"] = json!({"sign":{"binding":"formspec","ref":"urn:test:supervisor-form","definitionVersion":"1.0.0"}});
    raw["lifecycle"]["states"]["pending"]["onEntry"] = json!([{"action":"createTask","taskRef":"supervisor-signature","assignTo":"supervisor","contractRef":"sign","extensions":{"x-wos-signature-profile-key":"signatureProfile","x-wos-signature-step-id":"supervisorSigns"}}]);
    raw["lifecycle"]["states"]["noticed"]["transitions"] = json!([
      {"event":"effect","target":"applied","actor":"supervisor","actions":[{"action":"setData","path":"caseFile.level","value":0}]},
      {"event":"appeal","target":"noticed","actor":"household"},
      {"event":"schedule","target":"noticed","actor":"reviewer"},
      {"event":"hold","target":"noticed","actor":"reviewer"},
      {"event":"disposition","target":"noticed","actor":"reviewer"},
      {"event":"terminate","target":"noticed","actor":"household"}]);
    let ops = raw["governance"]["dueProcess"]["execution"]["operations"]
        .as_array_mut()
        .unwrap();
    ops[1]["requiredStatePointers"] = json!(["/lifecycle/states/noticed"]);
    ops[1]["transition"] = json!({"statePointer":"/lifecycle/states/noticed","transitionIndex":1});
    for (index, id, kind, actor) in [
        (2, "schedule", "scheduleHearing", "reviewer"),
        (3, "hold", "holdHearing", "reviewer"),
        (4, "disposition", "recordDisposition", "reviewer"),
        (5, "terminate", "requestServiceTermination", "household"),
    ] {
        ops.push(json!({"id":id,"kind":kind,"determinationRef":"adverse","actors":[actor],"requiredStatePointers":["/lifecycle/states/noticed"],"triggerEvent":id,"transition":{"statePointer":"/lifecycle/states/noticed","transitionIndex":index},"sourceRefs":[source()]}));
    }
    raw["signature"] = serde_json::to_value(signature_profile()).unwrap();
    raw["governance"]["audit"] = json!({"reasoningTier":{"requiredForTags":["determination"],"requireDecisionRequirements":false},"counterfactualTier":{"requiredForTags":["adverse-decision"],"requireProtectedCharacteristics":true}});
    raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["protectedCharacteristics"] =
        json!([{"casePointer":"/group","sourceRefs":[source()]}]);
    raw
}
fn form() -> Value {
    json!({"$formspec":"1.0","url":"urn:test:supervisor-form","version":"1.0.0","status":"draft","title":"Supervisor approval",
    "items":[{"key":"determinationId","type":"field","dataType":"string","label":"Determination"},
    {"key":"determinationSnapshotSha256","type":"field","dataType":"string","label":"Snapshot"},
    {"key":"determinationFiringRecordId","type":"field","dataType":"string","label":"Firing"},
    {"key":"signatureTaskId","type":"field","dataType":"string","label":"Actual task"},
    {"key":"acceptedAt","type":"field","dataType":"string","label":"Consent time"},
    {"key":"affirmed","type":"field","dataType":"boolean","label":"I approve this determination"}]})
}
fn signature_profile() -> wos_runtime::SignatureProfileDocument {
    serde_json::from_value(json!({"roles":[{"id":"supervisorSigner","role":"signer","actorId":"supervisor","authenticationPolicyKey":"otp"}],
        "documents":[{"id":"determination","documentRef":"urn:test:determination","documentHash":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","documentHashAlgorithm":"sha-256"}],
        "authenticationPolicies":[{"key":"otp","method":"email-otp","assuranceLevel":"standard"}],
        "signingFlow":{"type":"sequential","steps":[{"id":"supervisorSigns","roleId":"supervisorSigner","documentId":"determination","signingIntent":"urn:wos:signing-intent:approval"}],"completion":{"type":"all-required"}},
        "evidence":{"requiredFields":["response.data.acceptedAt","response.data.affirmed"],"consentReference":{"consentTextRef":"urn:test:consent","consentVersion":"1.0.0","acceptedAtPath":"response.data.acceptedAt","affirmationPath":"response.data.affirmed"},"identityBinding":{"method":"email-otp","assuranceLevel":"standard"},"custodyHookEligible":true}})).unwrap()
}
struct Signing {
    source: ring::signature::Ed25519KeyPair,
    verifier: integrity_signature_ring::RingVerifier,
    source_pkcs8: Vec<u8>,
    receipt_pkcs8: Vec<u8>,
    receipt_public: Vec<u8>,
}
fn signed_runtime() -> (WosRuntime, Time, Signing) {
    signed_runtime_with_access(DefaultRuntime::new())
}
fn signed_runtime_with_access(
    access: impl wos_core::traits::AccessControl + Send + Sync + 'static,
) -> (WosRuntime, Time, Signing) {
    signed_runtime_with_policy(signed_fixture(), access)
}
fn signed_runtime_with_policy(
    raw: Value,
    access: impl wos_core::traits::AccessControl + Send + Sync + 'static,
) -> (WosRuntime, Time, Signing) {
    use integrity_signature_ring::{InProcessReceiptSigner, RingVerifier, RingVerifierConfig};
    use ring::signature::KeyPair;
    let source_pkcs8 =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
            .unwrap()
            .as_ref()
            .to_vec();
    let source = ring::signature::Ed25519KeyPair::from_pkcs8(&source_pkcs8).unwrap();
    let receipt_pkcs8 =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
            .unwrap()
            .as_ref()
            .to_vec();
    let receipt_pair = ring::signature::Ed25519KeyPair::from_pkcs8(&receipt_pkcs8).unwrap();
    let public = receipt_pair.public_key().as_ref().to_vec();
    let receipt_signer = InProcessReceiptSigner::new_with_identity(
        receipt_pair,
        Some(b"receipt"),
        "urn:test:receipt-key",
        "urn:formspec:receipt-method:ed25519-cose-sign1@1",
    );
    let trust = wos_formspec_binding::TrustedReceiptSigner::ed25519(
        b"receipt".to_vec(),
        public.as_slice().try_into().unwrap(),
        "urn:formspec:receipt-method:ed25519-cose-sign1@1",
    )
    .unwrap();
    let time = Time(Arc::new(AtomicU64::new(0)));
    let processor = wos_formspec_binding::selected::ImmutableFormspecProcessor::new(&[
        wos_formspec_binding::selected::SelectedDocument {
            kind: wos_formspec_binding::selected::SelectedDocumentKind::Definition,
            url: "urn:test:supervisor-form".into(),
            version: "1.0.0".into(),
            bytes: serde_json::to_vec(&form()).unwrap(),
            definition: None,
        },
    ])
    .unwrap()
    .with_clock(time.clone());
    let mut bindings = BindingRegistry::new();
    bindings.register(
        wos_formspec_binding::FormspecBinding::new_with_trusted_receipt_signers(
            processor,
            vec![trust],
        ),
    );
    let mut runtime = WosRuntime::new(
        InMemoryStore::new(),
        Resolver(serde_json::from_value(raw.clone()).unwrap()),
        DefaultRuntime::new(),
        access,
        Services(Arc::new(AtomicUsize::new(0))),
        DefaultRuntime::new(),
        time.clone(),
        bindings,
    )
    .with_signature_profile("signatureProfile", signature_profile());
    let key = wos_runtime::SelectedDocumentSigningKey {
        id: "source-key".into(),
        task_ref: "supervisor-signature".into(),
        signer_human: "actor:human:supervisor".into(),
        kid_base64: STANDARD.encode(b"source"),
        public_key_base64: STANDARD.encode(source.public_key().as_ref()),
        method_uri: "urn:formspec:sig-method:ed25519-cose-sign1@1".into(),
    };
    let mut cap = capabilities(&raw);
    cap.signature_verifier =
        Box::new(SelectedDeterminationSignatureVerifier::new(vec![key]).unwrap());
    cap.retained_evidence.push(EvidenceSelection {
        id: "hearing".into(),
        role: EvidenceRole::HearingEvidence,
        bytes_base64: STANDARD.encode(b"actual retained hearing exhibit"),
        sha256: bytes_digest(b"actual retained hearing exhibit"),
    });
    runtime.configure_reference_due_process(cap).unwrap();
    let clock = integrity_signature::FixedClock::at_rfc3339("1970-01-01T00:00:00Z").unwrap();
    let verifier = RingVerifier::new_with_clock_config_and_receipt_signer(
        Arc::new(clock),
        RingVerifierConfig::new("urn:formspec:sig-method:")
            .with_adapter_id("urn:formspec:adapter:ring@1")
            .with_adapter_version("0.1.0")
            .with_receipt_payload_domain("formspec.verification.receipt.v1"),
        Arc::new(receipt_signer),
    );
    (
        runtime,
        time,
        Signing {
            source,
            verifier,
            source_pkcs8,
            receipt_pkcs8,
            receipt_public: public,
        },
    )
}
fn signed_response(runtime: &WosRuntime, process: &str, signing: &Signing) -> (String, Value) {
    use integrity_signature::{
        KeyRef, RegistryEntry, SignatureMethodRegistry, Verifier, VerifyRequest,
    };
    use ring::signature::KeyPair;
    let record = runtime.load_reference_record(process).unwrap();
    let d = &record
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap()
        .determinations[0];
    let task = record
        .process
        .active_tasks
        .iter()
        .find(|t| t.task_ref == "supervisor-signature")
        .unwrap();
    let mut response = json!({"$formspecResponse":"1.0","id":"actual-signed-response","definitionUrl":"urn:test:supervisor-form","definitionVersion":"1.0.0","status":"completed","authored":"1970-01-01T00:00:00Z","data":{"determinationId":d.id,"determinationSnapshotSha256":d.snapshot.sha256,"determinationFiringRecordId":d.firing.provenance_record_id,"signatureTaskId":task.task_id,"acceptedAt":"1970-01-01T00:00:00Z","affirmed":true}});
    let payload = integrity_canonical::build_signed_payload(
        response.to_string().as_bytes(),
        integrity_canonical::DigestAlgorithm::Sha256,
    )
    .unwrap();
    let method = "urn:formspec:sig-method:ed25519-cose-sign1@1";
    let protected = integrity_cose::detached_signature_protected_header(-8, b"source", method);
    let signature = signing.source.sign(&integrity_cose::sig_structure_bytes(
        &protected,
        &payload.canonical_bytes,
    ));
    let cose = integrity_cose::encode_cose_sign1(&protected, None, signature.as_ref());
    let receipt = signing
        .verifier
        .verify(
            &VerifyRequest {
                signed_bytes: payload.canonical_bytes,
                signature_bytes: cose.clone(),
                method_uri: method.into(),
                key_ref: KeyRef::RawPublicKey(signing.source.public_key().as_ref().to_vec()),
            },
            &SignatureMethodRegistry {
                version: "1.0.0".into(),
                entries: vec![RegistryEntry {
                    id: method.into(),
                    suite: "ed25519".into(),
                    wire: "cose-sign1".into(),
                    alg: Some(-8),
                    status: "registered".into(),
                    deprecation_notice: None,
                }],
            },
        )
        .unwrap();
    assert_eq!(
        receipt.result,
        integrity_signature::VerificationResult::Verified
    );
    response["authoredSignatures"] = json!([{"signatureId":"actual-source-signature","documentId":"determination","signingIntent":"urn:wos:signing-intent:approval","signatureValue":STANDARD.encode(cose),"signerId":"actor:human:supervisor","signerName":"Fixture supervisor","consentAccepted":true,"consentTextRef":"urn:test:consent","consentVersion":"1.0.0","affirmationText":"I approve this actual determination.","signedPayload":{"canonicalization":integrity_canonical::CANONICALIZATION_PROFILE,"digestAlgorithm":"sha-256","digest":payload.digest,"responseId":response["id"],"definitionUrl":"urn:test:supervisor-form","definitionVersion":"1.0.0","signedAt":"1970-01-01T00:00:00Z","signingIntent":"urn:wos:signing-intent:approval"},"documentHash":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","documentHashAlgorithm":"sha-256","signatureProvider":"urn:test:source-provider","ceremonyId":"actual-governance-approval","verificationReceipt":receipt}]);
    (task.task_id.clone(), response)
}
fn command(
    runtime: &mut WosRuntime,
    process: &str,
    op: &str,
    actor: &str,
    determination: &str,
    command: GovernanceCommand,
) -> GovernanceResult {
    runtime
        .governance_operation(
            process,
            &format!("actor:human:{actor}"),
            GovernanceRequest {
                operation_ref: op.into(),
                determination_id: Some(determination.into()),
                idempotency_token: format!("{op}.1"),
                actor: GovernanceActor {
                    declared_actor: actor.into(),
                    acting_actor: format!("actor:human:{actor}"),
                },
                command,
            },
        )
        .unwrap()
}
fn signed_only() -> (WosRuntime, Time, String, String, Signing) {
    signed_only_with_initial_case(signed_fixture(), None)
}
fn signed_only_with_initial_case(
    raw: Value,
    initial: Option<Value>,
) -> (WosRuntime, Time, String, String, Signing) {
    let (mut runtime, time, signing) = signed_runtime_with_policy(raw, DefaultRuntime::new());
    let process = create_with_initial_case(&mut runtime, initial).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    let (task, response) = signed_response(&runtime, &process, &signing);
    let result = runtime
        .submit_task_response(&task, response, "actor:human:supervisor", Some("signed.1"))
        .unwrap();
    assert!(
        matches!(result, wos_runtime::TaskSubmissionResult::Completed { .. }),
        "{result:?}"
    );
    let determination = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .determinations[0]
        .id
        .clone();
    (runtime, time, process, determination, signing)
}
fn issued() -> (WosRuntime, Time, String, String) {
    let (mut runtime, time, process, determination, _) = signed_only();
    let result = command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    assert!(
        matches!(result, GovernanceResult::Committed { .. }),
        "{result:?}"
    );
    (runtime, time, process, determination)
}

#[test]
fn valid_source_signature_with_deferred_normal_admission_cannot_issue_notice() {
    let (mut runtime, _, signing) = signed_runtime();
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    let (task, mut response) = signed_response(&runtime, &process, &signing);
    // The whole-Response signature excludes authoredSignatures. Removing only
    // its receipt keeps the real source signature cryptographically valid.
    response["authoredSignatures"][0]
        .as_object_mut()
        .unwrap()
        .remove("verificationReceipt");
    let result = runtime
        .submit_task_response(
            &task,
            response,
            "actor:human:supervisor",
            Some("deferred.1"),
        )
        .unwrap();
    assert!(matches!(
        result,
        wos_runtime::TaskSubmissionResult::Completed { .. }
    ));
    let before = runtime.load_reference_record(&process).unwrap();
    let affirmation = before
        .provenance_log
        .iter()
        .find(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation)
        .unwrap();
    assert_eq!(
        affirmation.data.as_ref().unwrap()["primitiveVerification"]["status"],
        "deferredPendingHelper"
    );
    let determination = before
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap()
        .determinations[0]
        .id
        .clone();
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Refused {
            code: RefusalCode::SignatureRejected
        }
    );
    let after = runtime.load_reference_record(&process).unwrap();
    assert_eq!(
        serde_json::to_value(&before.process).unwrap(),
        serde_json::to_value(&after.process).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&before.provenance_log).unwrap(),
        serde_json::to_value(&after.provenance_log).unwrap()
    );
    assert_eq!(
        runtime.load_process(&process).unwrap().case_state["level"],
        100
    );
}

fn record_fixture_disposition(
    runtime: &mut WosRuntime,
    process: &str,
    determination: &str,
    appeal_id: &str,
    outcome: DispositionOutcome,
) {
    let scheduled = command(
        runtime,
        process,
        "schedule",
        "reviewer",
        determination,
        GovernanceCommand::ScheduleHearing {
            appeal_id: appeal_id.to_string(),
            reviewer_actor: "reviewer".into(),
            scheduled_for: "1970-01-11T00:00:00Z".into(),
        },
    );
    assert!(
        matches!(scheduled, GovernanceResult::Committed { .. }),
        "{scheduled:?}"
    );
    let evidence = vec![SourceRef {
        source_version: "hearing".into(),
        section_anchor: "Exhibit 1".into(),
        retained_sha256: bytes_digest(b"actual retained hearing exhibit"),
    }];
    let held = command(
        runtime,
        process,
        "hold",
        "reviewer",
        determination,
        GovernanceCommand::HoldHearing {
            appeal_id: appeal_id.to_string(),
            evidence_refs: evidence.clone(),
        },
    );
    assert!(
        matches!(held, GovernanceResult::Committed { .. }),
        "{held:?}"
    );
    let disposition = command(
        runtime,
        process,
        "disposition",
        "reviewer",
        determination,
        GovernanceCommand::RecordDisposition {
            appeal_id: appeal_id.to_string(),
            outcome,
            rationale: "Reviewed the actual held-hearing exhibit.".into(),
            evidence_refs: evidence,
        },
    );
    assert!(
        matches!(disposition, GovernanceResult::Committed { .. }),
        "{disposition:?}"
    );
}

#[test]
fn real_signed_response_qualified_notice_appeal_hearing_and_disposition_follow_native_ports() {
    let (mut runtime, time, process, determination) = issued();
    let state = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap();
    assert_eq!(state.notices[0].status, NoticeStatus::Qualified);
    assert_eq!(
        state.notices[0].receipt.as_ref().unwrap().qualified_at,
        state.notices[0].issue_at
    );
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ClockRejected
        }
    );
    let filed = command(
        &mut runtime,
        &process,
        "appeal",
        "household",
        &determination,
        GovernanceCommand::FileAppeal {
            channel: Channel::Phone,
            continuation_choice: ContinuationChoice::Elect,
        },
    );
    let GovernanceResult::Committed {
        outcome: GovernanceOutcome::AppealFiled { appeal_id },
        ..
    } = filed
    else {
        panic!("{filed:?}")
    };
    time.0.store(10 * 86_400_000, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ContinuationFrozen
        }
    );
    record_fixture_disposition(
        &mut runtime,
        &process,
        &determination,
        &appeal_id,
        DispositionOutcome::Upheld,
    );
    let applied = command(
        &mut runtime,
        &process,
        "effect",
        "supervisor",
        &determination,
        GovernanceCommand::ApplyAdverseEffect,
    );
    assert!(
        matches!(applied, GovernanceResult::Committed { .. }),
        "{applied:?}"
    );
    assert_eq!(
        runtime.load_process(&process).unwrap().case_state["level"],
        0
    );
}

#[test]
fn save_and_restore_refuse_continuation_release_without_native_authority() {
    let (mut runtime, time, process, determination) = issued();
    assert!(matches!(
        command(
            &mut runtime,
            &process,
            "appeal",
            "household",
            &determination,
            GovernanceCommand::FileAppeal {
                channel: Channel::Phone,
                continuation_choice: ContinuationChoice::Elect
            }
        ),
        GovernanceResult::Committed { .. }
    ));
    let original = runtime.load_reference_record(&process).unwrap();
    for mutation in 0..7 {
        let mut changed = original.clone();
        let state = changed
            .process
            .governance_state
            .as_mut()
            .unwrap()
            .due_process
            .as_mut()
            .unwrap();
        match mutation {
            0 => state.appeals[0].continuation = ContinuationStatus::Released,
            1 => state.appeals[0].continuation = ContinuationStatus::Waived,
            2 => state.appeals.clear(),
            3 => {
                state.appeals[0].continuation_choice = ContinuationChoice::Waive;
                state.appeals[0].continuation = ContinuationStatus::Waived;
            }
            4 => {
                state.appeals[0].continuation = ContinuationStatus::Released;
                state.appeals[0].termination = Some(HouseholdTermination {
                    operation_record_id: state
                        .operations
                        .iter()
                        .find(|op| op.kind == OperationKind::FileAppeal)
                        .unwrap()
                        .record_id
                        .clone(),
                    requested_at: state.appeals[0].filed_at.clone(),
                    requested_by: state.appeals[0].filed_by.clone(),
                    rationale: "Grafted onto the actual filing operation.".into(),
                });
            }
            _ => {
                let filing_id = state
                    .operations
                    .iter()
                    .find(|op| op.kind == OperationKind::FileAppeal)
                    .unwrap()
                    .record_id
                    .clone();
                state.appeals[0].schedule = Some(HearingSchedule {
                    operation_record_id: filing_id.clone(),
                    scheduled_at: state.appeals[0].filed_at.clone(),
                    scheduled_for: state.appeals[0].filed_at.clone(),
                    reviewer_actor: "reviewer".into(),
                    reviewer_human: "actor:human:reviewer".into(),
                });
                if mutation == 6 {
                    state.appeals[0].hearing = Some(HeldHearing {
                        operation_record_id: filing_id,
                        held_at: state.appeals[0].filed_at.clone(),
                        reviewer_human: "actor:human:reviewer".into(),
                        evidence_refs: Vec::new(),
                    });
                }
            }
        }
        let mut store = InMemoryStore::new();
        store.restore_record(original.clone()).unwrap();
        let save_refused = store.save_record(changed.clone()).is_err();
        let restore_refused = InMemoryStore::new().restore_record(changed).is_err();
        assert_eq!(
            [save_refused, restore_refused],
            [true, true],
            "mutation {mutation}"
        );
    }
    time.0.store(91 * 86_400_000, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ContinuationFrozen
        }
    );
}

#[test]
fn notice_restore_refuses_deadlines_that_diverge_from_the_retained_artifact() {
    let (runtime, _, process, _) = issued();
    let original = runtime.load_reference_record(&process).unwrap();
    for mutation in 0..3 {
        let mut changed = original.clone();
        let notice = &mut changed
            .process
            .governance_state
            .as_mut()
            .unwrap()
            .due_process
            .as_mut()
            .unwrap()
            .notices[0];
        match mutation {
            0 => notice.appeal_deadline = notice.issue_at.clone(),
            1 => notice.effect_eligible_at = notice.issue_at.clone(),
            _ => notice.issue_at = "1970-01-02T00:00:00Z".into(),
        }
        let mut store = InMemoryStore::new();
        store.restore_record(original.clone()).unwrap();
        assert!(store.save_record(changed.clone()).is_err());
        assert!(
            InMemoryStore::new().restore_record(changed).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn real_waiver_termination_and_reversal_keep_authority_after_restore() {
    for branch in ["waive", "terminate", "reverse"] {
        let (mut runtime, time, process, determination, signing) = signed_only();
        assert!(matches!(
            command(
                &mut runtime,
                &process,
                "issue",
                "supervisor",
                &determination,
                GovernanceCommand::IssueNotice
            ),
            GovernanceResult::Committed { .. }
        ));
        let filed = command(
            &mut runtime,
            &process,
            "appeal",
            "household",
            &determination,
            GovernanceCommand::FileAppeal {
                channel: Channel::Written,
                continuation_choice: if branch == "waive" {
                    ContinuationChoice::Waive
                } else {
                    ContinuationChoice::Elect
                },
            },
        );
        let GovernanceResult::Committed {
            outcome: GovernanceOutcome::AppealFiled { appeal_id },
            ..
        } = filed
        else {
            panic!("{filed:?}")
        };
        if branch == "terminate" {
            assert!(matches!(
                command(
                    &mut runtime,
                    &process,
                    "terminate",
                    "household",
                    &determination,
                    GovernanceCommand::RequestServiceTermination {
                        appeal_id: appeal_id.clone(),
                        rationale: "The household requests termination of continued services."
                            .into()
                    }
                ),
                GovernanceResult::Committed { .. }
            ));
        }
        time.0.store(10 * 86_400_000, Ordering::SeqCst);
        if branch == "reverse" {
            record_fixture_disposition(
                &mut runtime,
                &process,
                &determination,
                &appeal_id,
                DispositionOutcome::Reversed,
            );
        }
        let original = runtime.load_reference_record(&process).unwrap();
        let continuation = &original
            .process
            .governance_state
            .as_ref()
            .unwrap()
            .due_process
            .as_ref()
            .unwrap()
            .appeals[0]
            .continuation;
        assert_eq!(
            continuation,
            if branch == "waive" {
                &ContinuationStatus::Waived
            } else {
                &ContinuationStatus::Released
            }
        );
        let mut store = InMemoryStore::new();
        store.restore_record(original).unwrap();
        let mut restored = recreated(store, time, &signing, DeterministicNoticeTransport::new([]));
        let effect = command(
            &mut restored,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect,
        );
        if branch == "reverse" {
            assert_eq!(
                effect,
                GovernanceResult::Refused {
                    code: RefusalCode::OperationStateRejected
                }
            );
            assert_eq!(
                restored.load_process(&process).unwrap().case_state["level"],
                100
            );
        } else {
            assert!(
                matches!(effect, GovernanceResult::Committed { .. }),
                "{branch}: {effect:?}"
            );
            assert_eq!(
                restored.load_process(&process).unwrap().case_state["level"],
                0
            );
        }
    }
}

#[test]
fn election_required_unspecified_choice_refuses_actual_filing() {
    let mut raw = signed_fixture();
    raw["governance"]["dueProcess"]["execution"]["appeal"]["continuation"]["choice"] =
        json!("electionRequired");
    let (mut runtime, _, signing) = signed_runtime_with_policy(raw, DefaultRuntime::new());
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    let (task, response) = signed_response(&runtime, &process, &signing);
    assert!(matches!(
        runtime
            .submit_task_response(
                &task,
                response,
                "actor:human:supervisor",
                Some("election-signature")
            )
            .unwrap(),
        wos_runtime::TaskSubmissionResult::Completed { .. }
    ));
    let determination = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .determinations[0]
        .id
        .clone();
    assert!(matches!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Committed { .. }
    ));
    let before = runtime.load_reference_record(&process).unwrap();
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "appeal",
            "household",
            &determination,
            GovernanceCommand::FileAppeal {
                channel: Channel::Phone,
                continuation_choice: ContinuationChoice::Unspecified
            }
        ),
        GovernanceResult::Refused {
            code: RefusalCode::OperationStateRejected
        }
    );
    let after = runtime.load_reference_record(&process).unwrap();
    assert_eq!(
        serde_json::to_value(&before.process).unwrap(),
        serde_json::to_value(&after.process).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&before.provenance_log).unwrap(),
        serde_json::to_value(&after.provenance_log).unwrap()
    );
}

#[test]
fn qualified_companion_requires_capabilities_and_preserves_other_family_refusals() {
    let raw = fixture();
    let kernel: KernelDocument = serde_json::from_value(raw.clone()).unwrap();
    assert!(
        wos_runtime::ReferenceCompanionPolicy::new(&kernel, None, None, Default::default())
            .is_err()
    );
    assert!(
        wos_runtime::ReferenceCompanionPolicy::new_reference_due_process(
            &kernel,
            None,
            None,
            Default::default(),
            &capabilities(&raw)
        )
        .is_ok()
    );
    let mut invalid = capabilities(&raw);
    invalid.retained_evidence.clear();
    assert!(
        wos_runtime::ReferenceCompanionPolicy::new_reference_due_process(
            &kernel,
            None,
            None,
            Default::default(),
            &invalid
        )
        .is_err()
    );
    let mut unsupported = raw.clone();
    unsupported["governance"]["reviewProtocols"] =
        json!([{"tags":["adverse-decision"],"protocols":["independent-first"]}]);
    let kernel: KernelDocument = serde_json::from_value(unsupported.clone()).unwrap();
    assert!(
        wos_runtime::ReferenceCompanionPolicy::new_reference_due_process(
            &kernel,
            None,
            None,
            Default::default(),
            &capabilities(&unsupported)
        )
        .is_err()
    );
}
#[test]
fn restore_refuses_erased_native_state_snapshot_receipt_and_signed_response() {
    let (runtime, _, process, _) = issued();
    let original = runtime.load_reference_record(&process).unwrap();
    let mut good = InMemoryStore::new();
    good.restore_record(original.clone()).unwrap();
    for corruption in 0..5 {
        let mut record = original.clone();
        match corruption {
            0 => {
                record
                    .process
                    .governance_state
                    .as_mut()
                    .unwrap()
                    .due_process = None
            }
            1 => record
                .process
                .governance_state
                .as_mut()
                .unwrap()
                .due_process
                .as_mut()
                .unwrap()
                .determinations
                .clear(),
            2 => {
                record
                    .provenance_log
                    .iter_mut()
                    .find(|p| p.case_file_snapshot.is_some())
                    .unwrap()
                    .case_file_snapshot = None
            }
            3 => {
                record
                    .process
                    .governance_state
                    .as_mut()
                    .unwrap()
                    .due_process
                    .as_mut()
                    .unwrap()
                    .notices[0]
                    .receipt = None
            }
            4 => record.artifacts.clear(),
            _ => unreachable!(),
        }
        assert!(
            InMemoryStore::new().restore_record(record).is_err(),
            "corruption {corruption}"
        );
    }
}

#[test]
fn export_reference_inputs_for_parent() {
    use ring::signature::KeyPair;
    let raw = signed_fixture();
    let (runtime, _, signing) = signed_runtime();
    let cap = capabilities(&raw);
    let adapters = json!({"scope":"softwareFixture","humanActorBindings":[
        {"declaredActor":"determiner","assignedActor":"actor:human:determiner"},
        {"declaredActor":"supervisor","assignedActor":"actor:human:supervisor"},
        {"declaredActor":"household","assignedActor":"actor:human:household"},
        {"declaredActor":"reviewer","assignedActor":"actor:human:reviewer"}],
        "trustedReceiptSigners":[{"id":"receipt-key","suite":"ed25519","kidBase64":STANDARD.encode(b"receipt"),"publicKeyBase64":STANDARD.encode(&signing.receipt_public),"methodUri":"urn:formspec:receipt-method:ed25519-cose-sign1@1"}],
        "noticeTransport":{"id":"fixture-notice","scope":"softwareFixture","mode":"sameInstantQualifiedReceipt","retainedResultsIdentity":bytes_digest(b"isolated native reference fixture"),"failureSchedule":[]},
        "retainedEvidence":cap.retained_evidence.iter().cloned().chain(std::iter::once(EvidenceSelection{id:"hearing".into(),role:EvidenceRole::HearingEvidence,bytes_base64:STANDARD.encode(b"actual retained hearing exhibit"),sha256:bytes_digest(b"actual retained hearing exhibit")})).collect::<Vec<_>>(),
        "templates":cap.templates,
        "documentSigningKeys":[{"id":"source-key","taskRef":"supervisor-signature","signerHuman":"actor:human:supervisor","kidBase64":STANDARD.encode(b"source"),"publicKeyBase64":STANDARD.encode(signing.source.public_key().as_ref()),"methodUri":"urn:formspec:sig-method:ed25519-cose-sign1@1"}],
        "fixtureSigningCapabilities":[{"id":"sign-actual","scope":"softwareFixture","documentKeyRef":"source-key","pkcs8Base64":STANDARD.encode(&signing.source_pkcs8),"receiptSignerRef":"receipt-key","receiptPkcs8Base64":STANDARD.encode(&signing.receipt_pkcs8)}],"invocations":[]});
    let output = json!({"candidate":raw,"definition":form(),"profile":serde_json::to_value(signature_profile()).unwrap(),"adapters":adapters,
        "responseTemplate":{"$formspecResponse":"1.0","id":"actual-signed-response","definitionUrl":"urn:test:supervisor-form","definitionVersion":"1.0.0","status":"completed","authored":"1970-01-01T00:00:00Z","data":{"acceptedAt":"1970-01-01T00:00:00Z","affirmed":true}},
        "signatureTemplate":{"signatureId":"actual-source-signature","documentId":"determination","signingIntent":"urn:wos:signing-intent:approval","signerId":"actor:human:supervisor","signerName":"Fixture supervisor","consentAccepted":true,"consentTextRef":"urn:test:consent","consentVersion":"1.0.0","affirmationText":"I approve this actual determination.","documentHash":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","documentHashAlgorithm":"sha-256","signatureProvider":"urn:test:source-provider","ceremonyId":"actual-governance-approval"}});
    if let Ok(path) = std::env::var("WOS_REFERENCE_INPUTS_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    }
    let _ = runtime;
}

fn selected_capability(
    signing: &Signing,
    transport: impl NoticeTransport + Send + Sync + 'static,
) -> ReferenceDueProcessCapabilities {
    use ring::signature::KeyPair;
    let mut cap = capabilities(&signed_fixture());
    cap.notice_transport = Box::new(transport);
    cap.signature_verifier = Box::new(
        SelectedDeterminationSignatureVerifier::new(vec![
            wos_runtime::SelectedDocumentSigningKey {
                id: "source-key".into(),
                task_ref: "supervisor-signature".into(),
                signer_human: "actor:human:supervisor".into(),
                kid_base64: STANDARD.encode(b"source"),
                public_key_base64: STANDARD.encode(signing.source.public_key().as_ref()),
                method_uri: "urn:formspec:sig-method:ed25519-cose-sign1@1".into(),
            },
        ])
        .unwrap(),
    );
    cap
}
fn recreated<S: RuntimeStore + Send + Sync + 'static>(
    store: S,
    time: Time,
    signing: &Signing,
    transport: impl NoticeTransport + Send + Sync + 'static,
) -> WosRuntime {
    let mut runtime = WosRuntime::new(
        store,
        Resolver(serde_json::from_value(signed_fixture()).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Services(Arc::new(AtomicUsize::new(0))),
        DefaultRuntime::new(),
        time,
        BindingRegistry::new(),
    )
    .with_signature_profile("signatureProfile", signature_profile());
    runtime
        .configure_reference_due_process(selected_capability(signing, transport))
        .unwrap();
    runtime
}
#[test]
fn preparation_stages_exact_notice_without_invoking_transport() {
    use wos_runtime::{
        NativeCommandKey, NativePreparationPolicy, NativeReadOnlyDependency, NativeRuntimeCommand,
        NativeRuntimeOutcome, NativeRuntimeSnapshot,
    };
    let (mut runtime, _, process, determination, signing) = signed_only();
    let original = runtime.load_reference_record(&process).unwrap();
    let transport = DeterministicNoticeTransport::new([]);
    runtime
        .configure_reference_due_process(selected_capability(&signing, transport.clone()))
        .unwrap();
    let request = GovernanceRequest {
        operation_ref: "issue".into(),
        determination_id: Some(determination),
        idempotency_token: "stage.issue".into(),
        actor: GovernanceActor {
            declared_actor: "supervisor".into(),
            acting_actor: "actor:human:supervisor".into(),
        },
        command: GovernanceCommand::IssueNotice,
    };
    let mut policy = NativePreparationPolicy::default();
    for dependency in [
        NativeReadOnlyDependency::Clock,
        NativeReadOnlyDependency::Resolver,
        NativeReadOnlyDependency::AccessControl,
        NativeReadOnlyDependency::CompanionPolicy,
        NativeReadOnlyDependency::DueProcessSignatureVerifier,
    ] {
        policy
            .read_only_dependency_ids
            .insert(dependency, "actual-local-fixture".into());
    }
    let prepared = runtime
        .prepare_native_command(
            NativeCommandKey("stage-governance".into()),
            NativeRuntimeCommand::GovernanceOperation {
                process_id: process.clone(),
                caller_actor_id: "actor:human:supervisor".into(),
                request,
            },
            NativeRuntimeSnapshot {
                process_records: vec![original.clone()],
                complete_case_membership: vec![wos_runtime::CaseProcessMembership {
                    case_ledger_id: original.process.case_ledger_id.clone(),
                    process_ids: vec![process.clone()],
                }],
                ..Default::default()
            },
            policy,
        )
        .unwrap();
    assert!(matches!(
        prepared.outcome(),
        NativeRuntimeOutcome::GovernanceOperation(GovernanceResult::Committed { .. })
    ));
    assert_eq!(prepared.notice_work().len(), 1);
    let work = &prepared.notice_work()[0];
    assert_eq!(
        transport
            .invocations(&process, &work.transport_key)
            .unwrap(),
        0
    );
    let wos_runtime::NativeProcessChange::Replace { after, .. } = &prepared.process_changes()[0]
    else {
        panic!("actual speculative replacement")
    };
    let state = after
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap();
    assert_eq!(state.notices[0].status, NoticeStatus::Pending);
    assert!(state.notices[0].receipt.is_none());
    assert!(
        original
            .process
            .governance_state
            .unwrap()
            .due_process
            .unwrap()
            .notices
            .is_empty()
    );
}
#[test]
fn replay_returns_original_after_state_change_and_changed_command_conflicts() {
    let (mut runtime, _, process, determination) = issued();
    let first = command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    assert!(matches!(first, GovernanceResult::Committed { .. }));
    let before = runtime.load_reference_record(&process).unwrap();
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice
        ),
        first
    );
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ReplayConflict
        }
    );
    let after = runtime.load_reference_record(&process).unwrap();
    assert_eq!(before.provenance_log.len(), after.provenance_log.len());
}
#[test]
fn appeal_deadline_equality_preserves_filing_and_strict_after_window_effect() {
    let (mut runtime, time, process, determination) = issued();
    time.0.store(90 * 86_400_000, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ContinuationFrozen
        }
    );
    let filing = command(
        &mut runtime,
        &process,
        "appeal",
        "household",
        &determination,
        GovernanceCommand::FileAppeal {
            channel: Channel::Written,
            continuation_choice: ContinuationChoice::Elect,
        },
    );
    assert!(
        matches!(filing, GovernanceResult::Committed { .. }),
        "{filing:?}"
    );
    let (mut runtime, time, process, determination) = issued();
    time.0.store(90 * 86_400_000 + 1, Ordering::SeqCst);
    assert!(matches!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Committed { .. }
    ));
}
#[derive(Clone)]
struct FaultStore {
    inner: wos_runtime::SharedInMemoryStore,
    saves: Arc<AtomicUsize>,
    fail_at: Arc<AtomicUsize>,
}
impl RuntimeStore for FaultStore {
    fn create_record(
        &mut self,
        r: wos_runtime::store::RuntimeRecord,
    ) -> Result<(), wos_runtime::store::StoreError> {
        self.inner.create_record(r)
    }
    fn load_record(
        &self,
        id: &str,
    ) -> Result<wos_runtime::store::RuntimeRecord, wos_runtime::store::StoreError> {
        self.inner.load_record(id)
    }
    fn processes_for_case(&self, id: &str) -> Vec<String> {
        self.inner.processes_for_case(id)
    }
    fn append_provenance_for_case(
        &mut self,
        c: &str,
        p: &str,
        r: wos_core::ProvenanceRecord,
    ) -> Result<(), wos_runtime::store::StoreError> {
        self.inner.append_provenance_for_case(c, p, r)
    }
    fn save_record(
        &mut self,
        r: wos_runtime::store::RuntimeRecord,
    ) -> Result<(), wos_runtime::store::StoreError> {
        let n = self.saves.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail_at.load(Ordering::SeqCst) == n {
            return Err(wos_runtime::store::StoreError::Failed(
                "injected atomic save failure".into(),
            ));
        }
        self.inner.save_record(r)
    }
    fn create_intake_record(
        &mut self,
        r: wos_runtime::store::IntakeRecord,
    ) -> Result<(), wos_runtime::store::StoreError> {
        self.inner.create_intake_record(r)
    }
    fn load_intake_record(
        &self,
        b: &str,
        i: &str,
    ) -> Result<wos_runtime::store::IntakeRecord, wos_runtime::store::StoreError> {
        self.inner.load_intake_record(b, i)
    }
    fn save_intake_record(
        &mut self,
        r: wos_runtime::store::IntakeRecord,
    ) -> Result<(), wos_runtime::store::StoreError> {
        self.inner.save_intake_record(r)
    }
}
#[test]
fn persisted_attempt_fence_and_success_before_save_recover_across_recreation() {
    for fail_at in [1, 2, 3] {
        let (runtime, time, process, determination, signing) = signed_only();
        let original = runtime.load_reference_record(&process).unwrap();
        let shared = wos_runtime::SharedInMemoryStore::default();
        shared.0.lock().unwrap().restore_record(original).unwrap();
        let store = FaultStore {
            inner: shared.clone(),
            saves: Arc::new(AtomicUsize::new(0)),
            fail_at: Arc::new(AtomicUsize::new(fail_at)),
        };
        let transport = DeterministicNoticeTransport::new([]);
        let mut runtime = recreated(store.clone(), time.clone(), &signing, transport.clone());
        let result = runtime.governance_operation(
            &process,
            "actor:human:supervisor",
            GovernanceRequest {
                operation_ref: "issue".into(),
                determination_id: Some(determination),
                idempotency_token: "fault.issue".into(),
                actor: GovernanceActor {
                    declared_actor: "supervisor".into(),
                    acting_actor: "actor:human:supervisor".into(),
                },
                command: GovernanceCommand::IssueNotice,
            },
        );
        assert!(result.is_err());
        let record = shared.load_record(&process).unwrap();
        let state = record
            .process
            .governance_state
            .as_ref()
            .unwrap()
            .due_process
            .as_ref()
            .unwrap();
        if fail_at == 1 {
            assert!(state.notices.is_empty());
            continue;
        }
        let notice = &state.notices[0];
        assert!(notice.receipt.is_none());
        assert_eq!(notice.status, NoticeStatus::Pending);
        assert_eq!(
            transport
                .invocations(&process, &notice.transport_key)
                .unwrap(),
            if fail_at == 3 { 1 } else { 0 }
        );
        if fail_at == 3 {
            time.0.store(86_400_000, Ordering::SeqCst);
            store.fail_at.store(0, Ordering::SeqCst);
            let mut recreated = recreated(store, time, &signing, transport.clone());
            recreated.deliver_pending_notices(&process).unwrap();
            let recovered = recreated.load_reference_record(&process).unwrap();
            let recovered = &recovered
                .process
                .governance_state
                .as_ref()
                .unwrap()
                .due_process
                .as_ref()
                .unwrap()
                .notices[0];
            assert_eq!(recovered.attempts, 1);
            assert_eq!(recovered.transport_results.len(), 1);
            assert!(matches!(
                recovered.transport_results[0],
                TransportResult::Qualified { .. }
            ));
            assert_notice_schema(recovered);
            assert_eq!(
                recovered.receipt.as_ref().unwrap().qualified_at,
                notice.issue_at
            );
            assert_eq!(
                transport
                    .invocations(&process, &notice.transport_key)
                    .unwrap(),
                1
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PortClaim {
    Clean,
    AcceptedOnly,
    WrongDigest,
    WrongCorrelation,
    WrongNotice,
    EmptyNotice,
    InvalidToken,
    InvalidDigest,
    NoncanonicalTimestamp,
    FutureTimestamp,
}
#[derive(Clone)]
struct ObservedTransport {
    actual: DeterministicNoticeTransport,
    claim: Arc<std::sync::Mutex<PortClaim>>,
}
impl ObservedTransport {
    fn new(claim: PortClaim, failures: impl IntoIterator<Item = usize>) -> Self {
        Self {
            actual: DeterministicNoticeTransport::new(failures),
            claim: Arc::new(std::sync::Mutex::new(claim)),
        }
    }
    fn observed(&self, result: TransportResult) -> TransportResult {
        let TransportResult::Qualified {
            mut acknowledgement,
            mut receipt,
        } = result
        else {
            return result;
        };
        match *self.claim.lock().unwrap() {
            PortClaim::Clean => {}
            PortClaim::AcceptedOnly => return TransportResult::AcceptedOnly { acknowledgement },
            PortClaim::WrongDigest => {
                receipt.artifact_sha256 = format!("sha256:{}", "b".repeat(64))
            }
            PortClaim::WrongCorrelation => receipt.correlation = "wrong-correlation".into(),
            PortClaim::WrongNotice => receipt.notice_id = "other-saved-notice".into(),
            PortClaim::EmptyNotice => receipt.notice_id.clear(),
            PortClaim::InvalidToken => receipt.correlation = "invalid token".into(),
            PortClaim::InvalidDigest => receipt.artifact_sha256 = "sha256:ABCDEF".into(),
            PortClaim::NoncanonicalTimestamp => {
                receipt.qualified_at = "1970-01-01T00:00:00.000Z".into()
            }
            PortClaim::FutureTimestamp => {
                acknowledgement.accepted_at = "1970-01-02T00:00:00Z".into();
                receipt.qualified_at = "1970-01-02T00:00:00Z".into();
            }
        }
        TransportResult::Qualified {
            acknowledgement,
            receipt,
        }
    }
}
impl NoticeTransport for ObservedTransport {
    fn lookup(&self, work: &PendingNoticeWork) -> Result<Option<TransportResult>, RefusalCode> {
        self.actual
            .lookup(work)
            .map(|result| result.map(|result| self.observed(result)))
    }
    fn execute(&self, work: &PendingNoticeWork, now: &str) -> Result<TransportResult, RefusalCode> {
        self.actual
            .execute(work, now)
            .map(|result| self.observed(result))
    }
}
fn notice(runtime: &WosRuntime, process: &str) -> NoticeState {
    runtime
        .load_reference_record(process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .notices[0]
        .clone()
}
fn assert_notice_schema(notice: &NoticeState) {
    let mut schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/wos-reference-due-process.schema.json"
    ))
    .unwrap();
    schema["$ref"] = json!("#/$defs/NoticeState");
    let workflow: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-workflow.schema.json")).unwrap();
    let replay: Value = serde_json::from_str(include_str!(
        "../../../schemas/wos-reference-replay.schema.json"
    ))
    .unwrap();
    let registry = jsonschema::Registry::new()
        .add(workflow["$id"].as_str().unwrap(), workflow.clone())
        .unwrap()
        .add(
            "https://wos-spec.org/schemas/wos-workflow.schema.json",
            workflow,
        )
        .unwrap()
        .add(
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json",
            schema.clone(),
        )
        .unwrap()
        .add(
            "https://wos-spec.org/schemas/wos-reference-replay.schema.json",
            replay,
        )
        .unwrap()
        .prepare()
        .unwrap();
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&schema)
        .unwrap();
    let value = serde_json::to_value(notice).unwrap();
    let errors = validator
        .iter_errors(&value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
fn notice_sent_count(runtime: &WosRuntime, process: &str) -> usize {
    runtime
        .load_reference_record(process)
        .unwrap()
        .provenance_log
        .iter()
        .filter(|p| p.record_kind == wos_core::ProvenanceKind::NoticeSent)
        .count()
}
#[test]
fn rejected_transport_claims_are_retained_without_qualification_or_effect() {
    for claim in [
        PortClaim::FutureTimestamp,
        PortClaim::WrongDigest,
        PortClaim::WrongCorrelation,
        PortClaim::WrongNotice,
    ] {
        let (mut runtime, time, process, determination, signing) = signed_only();
        let port = ObservedTransport::new(claim, []);
        runtime
            .configure_reference_due_process(selected_capability(&signing, port.clone()))
            .unwrap();
        assert!(matches!(
            command(
                &mut runtime,
                &process,
                "issue",
                "supervisor",
                &determination,
                GovernanceCommand::IssueNotice
            ),
            GovernanceResult::Committed { .. }
        ));
        let saved = notice(&runtime, &process);
        assert_eq!(saved.status, NoticeStatus::Blocked);
        assert!(saved.receipt.is_none());
        assert_eq!(saved.transport_results.len(), 1);
        let work = PendingNoticeWork {
            process_id: process.clone(),
            notice_id: saved.id.clone(),
            determination_id: determination.clone(),
            transport_key: saved.transport_key.clone(),
            correlation: saved.correlation.clone(),
            artifact_sha256: saved.artifact_sha256.clone(),
            artifact_bytes_base64: saved.artifact_bytes_base64.clone(),
            recipient: saved.recipient.clone(),
            channel: saved.channel.clone(),
            issue_at: saved.issue_at.clone(),
        };
        assert_eq!(
            saved.transport_results,
            vec![port.lookup(&work).unwrap().unwrap()]
        );
        if !matches!(claim, PortClaim::FutureTimestamp) {
            assert_eq!(saved.acknowledgements.len(), 1);
        } else {
            assert!(saved.acknowledgements.is_empty());
        }
        assert_eq!(notice_sent_count(&runtime, &process), 0);
        assert_notice_schema(&saved);
        time.0.store(91 * 86_400_000, Ordering::SeqCst);
        assert_eq!(
            command(
                &mut runtime,
                &process,
                "effect",
                "supervisor",
                &determination,
                GovernanceCommand::ApplyAdverseEffect
            ),
            GovernanceResult::Refused {
                code: RefusalCode::NoticePending
            }
        );
        runtime.deliver_pending_notices(&process).unwrap();
        assert_eq!(
            notice(&runtime, &process).transport_results,
            saved.transport_results
        );
        assert_eq!(
            port.actual
                .invocations(&process, &saved.transport_key)
                .unwrap(),
            1
        );
    }
}
#[test]
fn fresh_late_success_preserves_original_anchors_and_failure_then_success_observation_order() {
    let (mut runtime, time, process, determination, signing) = signed_only();
    let transport = DeterministicNoticeTransport::new([1]);
    runtime
        .configure_reference_due_process(selected_capability(&signing, transport.clone()))
        .unwrap();
    assert!(matches!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Committed { .. }
    ));
    let before = notice(&runtime, &process);
    assert_eq!(before.transport_results.len(), 1);
    assert!(matches!(
        before.transport_results[0],
        TransportResult::Failed { .. }
    ));
    time.0.store(86_400_000, Ordering::SeqCst);
    runtime.deliver_pending_notices(&process).unwrap();
    let after = notice(&runtime, &process);
    assert_eq!(after.status, NoticeStatus::Blocked);
    assert!(after.receipt.is_none());
    assert_eq!(after.attempts, 2);
    assert_eq!(
        (
            &after.issue_at,
            &after.appeal_deadline,
            &after.effect_eligible_at
        ),
        (
            &before.issue_at,
            &before.appeal_deadline,
            &before.effect_eligible_at
        )
    );
    assert!(
        after
            .transport_results
            .starts_with(&before.transport_results)
    );
    assert_eq!(after.transport_results.len(), 2);
    let TransportResult::Qualified { receipt, .. } = &after.transport_results[1] else {
        panic!("actual late success")
    };
    assert_eq!(receipt.qualified_at, "1970-01-02T00:00:00Z");
    assert_notice_schema(&after);
    assert_eq!(notice_sent_count(&runtime, &process), 0);
    time.0.store(91 * 86_400_000, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "effect",
            "supervisor",
            &determination,
            GovernanceCommand::ApplyAdverseEffect
        ),
        GovernanceResult::Refused {
            code: RefusalCode::NoticePending
        }
    );
}
#[test]
fn accepted_only_recovers_without_invocation_and_changed_nonfailure_refuses() {
    for initial in [PortClaim::AcceptedOnly, PortClaim::WrongDigest] {
        let (mut runtime, _, process, determination, signing) = signed_only();
        let port = ObservedTransport::new(initial, []);
        runtime
            .configure_reference_due_process(selected_capability(&signing, port.clone()))
            .unwrap();
        assert!(matches!(
            command(
                &mut runtime,
                &process,
                "issue",
                "supervisor",
                &determination,
                GovernanceCommand::IssueNotice
            ),
            GovernanceResult::Committed { .. }
        ));
        let before = notice(&runtime, &process);
        assert_eq!(before.transport_results.len(), 1);
        assert_eq!(before.acknowledgements.len(), 1);
        assert!(before.receipt.is_none());
        if matches!(initial, PortClaim::AcceptedOnly) {
            assert_eq!(before.status, NoticeStatus::Pending);
        }
        runtime.deliver_pending_notices(&process).unwrap();
        assert_eq!(notice(&runtime, &process), before);
        assert_eq!(
            port.actual
                .invocations(&process, &before.transport_key)
                .unwrap(),
            1
        );
        *port.claim.lock().unwrap() = if matches!(initial, PortClaim::AcceptedOnly) {
            PortClaim::Clean
        } else {
            PortClaim::WrongCorrelation
        };
        assert!(matches!(
            runtime.deliver_pending_notices(&process),
            Err(RuntimeError::Governance(RefusalCode::ReplayConflict))
        ));
        assert_eq!(notice(&runtime, &process), before);
        assert_eq!(notice_sent_count(&runtime, &process), 0);
        assert_notice_schema(&before);
    }
}
#[test]
fn malformed_transport_strings_refuse_before_execute_or_lookup_result_admission() {
    for claim in [
        PortClaim::EmptyNotice,
        PortClaim::InvalidToken,
        PortClaim::InvalidDigest,
        PortClaim::NoncanonicalTimestamp,
    ] {
        let (mut runtime, _, process, determination, signing) = signed_only();
        let port = ObservedTransport::new(claim, []);
        runtime
            .configure_reference_due_process(selected_capability(&signing, port.clone()))
            .unwrap();
        assert_eq!(
            command(
                &mut runtime,
                &process,
                "issue",
                "supervisor",
                &determination,
                GovernanceCommand::IssueNotice
            ),
            GovernanceResult::Refused {
                code: RefusalCode::InvalidConfiguration
            }
        );
        let before = notice(&runtime, &process);
        assert_eq!(before.attempts, 1);
        assert!(before.transport_results.is_empty());
        assert!(before.acknowledgements.is_empty());
        assert!(before.receipt.is_none());
        assert!(matches!(
            runtime.deliver_pending_notices(&process),
            Err(RuntimeError::Governance(RefusalCode::InvalidConfiguration))
        ));
        assert_eq!(notice(&runtime, &process), before);
        assert_eq!(
            port.actual
                .invocations(&process, &before.transport_key)
                .unwrap(),
            1
        );
        assert_eq!(notice_sent_count(&runtime, &process), 0);
    }
    assert!(
        serde_json::from_value::<TransportResult>(json!({"kind":"qualified","arbitrary":true}))
            .is_err()
    );
    assert!(serde_json::from_value::<TransportResult>(json!({"kind":"failed","failure":{"kind":"noEffectFailure","code":"injectedFailure","extra":true}})).is_err());
    assert!(
        serde_json::from_value::<TransportResult>(
            json!({"kind":"acceptedOnly","acknowledgement":null})
        )
        .is_err()
    );
}
#[test]
fn repeated_failures_deduplicate_and_exhaustion_adds_no_invented_result() {
    for failures in [vec![1, 2], vec![1, 2, 3]] {
        let (mut runtime, _, process, determination, signing) = signed_only();
        let port = DeterministicNoticeTransport::new(failures.clone());
        runtime
            .configure_reference_due_process(selected_capability(&signing, port.clone()))
            .unwrap();
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice,
        );
        runtime.deliver_pending_notices(&process).unwrap();
        let repeated = notice(&runtime, &process);
        assert_eq!(repeated.attempts, 2);
        assert_eq!(repeated.transport_results.len(), 1);
        runtime.deliver_pending_notices(&process).unwrap();
        let final_notice = notice(&runtime, &process);
        assert_eq!(final_notice.attempts, 3);
        assert_eq!(
            port.invocations(&process, &final_notice.transport_key)
                .unwrap(),
            3
        );
        if failures.len() == 3 {
            assert_eq!(final_notice.status, NoticeStatus::Exhausted);
            assert_eq!(final_notice.transport_results.len(), 1);
            assert_eq!(notice_sent_count(&runtime, &process), 0);
        } else {
            assert_eq!(final_notice.status, NoticeStatus::Qualified);
            assert_eq!(final_notice.transport_results.len(), 2);
            assert!(matches!(
                final_notice.transport_results[0],
                TransportResult::Failed { .. }
            ));
            assert!(matches!(
                final_notice.transport_results[1],
                TransportResult::Qualified { .. }
            ));
            assert_eq!(notice_sent_count(&runtime, &process), 1);
        }
        runtime.deliver_pending_notices(&process).unwrap();
        assert_eq!(notice(&runtime, &process), final_notice);
        assert_eq!(
            port.invocations(&process, &final_notice.transport_key)
                .unwrap(),
            3
        );
        assert_notice_schema(&final_notice);
    }
}

fn blocked_record_with_failure_then_receipt() -> wos_runtime::store::RuntimeRecord {
    let (mut runtime, _, process, determination, signing) = signed_only();
    let port = ObservedTransport::new(PortClaim::WrongDigest, [1]);
    runtime
        .configure_reference_due_process(selected_capability(&signing, port))
        .unwrap();
    command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    runtime.deliver_pending_notices(&process).unwrap();
    let record = runtime.load_reference_record(&process).unwrap();
    assert_eq!(
        record
            .process
            .governance_state
            .as_ref()
            .unwrap()
            .due_process
            .as_ref()
            .unwrap()
            .notices[0]
            .transport_results
            .len(),
        2
    );
    record
}
fn first_notice_mut(record: &mut wos_runtime::store::RuntimeRecord) -> &mut NoticeState {
    &mut record
        .process
        .governance_state
        .as_mut()
        .unwrap()
        .due_process
        .as_mut()
        .unwrap()
        .notices[0]
}
#[test]
fn diagnostic_store_updates_refuse_erasure_reorder_duplicate_replacement_and_bound_violation() {
    let original = blocked_record_with_failure_then_receipt();
    for corruption in 0..6 {
        let mut record = original.clone();
        let n = first_notice_mut(&mut record);
        match corruption {
            0 => {
                n.transport_results.clear();
            }
            1 => {
                n.transport_results.swap(0, 1);
            }
            2 => {
                n.transport_results.push(n.transport_results[0].clone());
            }
            3 => {
                n.attempts = 1;
            }
            4 => {
                let TransportResult::Qualified { receipt, .. } = &mut n.transport_results[1] else {
                    panic!("actual result")
                };
                receipt.artifact_sha256 = "not-a-digest".into();
            }
            5 => {
                n.transport_results.push(TransportResult::Failed {
                    failure: TransportFailure {
                        kind: FailureKind::NoEffectFailure,
                        code: FailureCode::AttemptsExhausted,
                    },
                });
            }
            _ => unreachable!(),
        }
        let mut store = InMemoryStore::new();
        store.restore_record(original.clone()).unwrap();
        assert!(
            store.save_record(record).is_err(),
            "corruption {corruption}"
        );
    }
}
#[test]
fn diagnostic_prefix_blocks_replacement_of_a_prior_failure() {
    let original = blocked_record_with_failure_then_receipt();
    let mut changed = original.clone();
    let TransportResult::Failed { failure } =
        &mut first_notice_mut(&mut changed).transport_results[0]
    else {
        panic!("actual first failure")
    };
    failure.code = FailureCode::AttemptsExhausted;
    let mut store = InMemoryStore::new();
    store.restore_record(original).unwrap();
    assert!(store.save_record(changed).is_err());
}
#[test]
fn native_notice_requires_diagnostics_in_saved_json_and_provenance_prefix_stays_immutable() {
    let (runtime, _, process, _) = issued();
    let original = runtime.load_reference_record(&process).unwrap();
    let notice = original
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap()
        .notices[0]
        .clone();
    assert_notice_schema(&notice);
    for missing in [true, false] {
        let mut value = serde_json::to_value(&notice).unwrap();
        if missing {
            value.as_object_mut().unwrap().remove("transportResults");
        } else {
            value["transportResults"] = Value::Null;
        }
        assert!(serde_json::from_value::<NoticeState>(value).is_err());
    }
    // Initial restore has no predecessor. Qualified facts must independently
    // resolve both the exact retained claim and its original success record.
    for corruption in 0..4 {
        let mut record = original.clone();
        match corruption {
            0 => first_notice_mut(&mut record).transport_results.clear(),
            1 => {
                let TransportResult::Qualified { receipt, .. } =
                    &mut first_notice_mut(&mut record).transport_results[0]
                else {
                    panic!("qualified actual result")
                };
                receipt.correlation = "different-valid-token".into();
            }
            2 => {
                record
                    .provenance_log
                    .iter_mut()
                    .find(|p| p.record_kind == wos_core::ProvenanceKind::NoticeSent)
                    .unwrap()
                    .data
                    .as_mut()
                    .unwrap()["receipt"]["adapterResultId"] = json!("changed-result")
            }
            3 => {
                record
                    .provenance_log
                    .iter_mut()
                    .find(|p| p.record_kind == wos_core::ProvenanceKind::NoticeSent)
                    .unwrap()
                    .timestamp = "1970-01-02T00:00:00Z".into()
            }
            _ => unreachable!(),
        }
        assert!(
            InMemoryStore::new().restore_record(record).is_err(),
            "qualified restore corruption {corruption}"
        );
    }
    for corruption in 0..4 {
        let mut record = original.clone();
        match corruption {
            0 => record.provenance_log[0].actor_type = Some("service".into()),
            1 => record.provenance_log[0].from_state = Some("different-state".into()),
            2 => record.provenance_log[0]
                .inputs
                .push("changed-original-input".into()),
            3 => record.provenance_log.swap(0, 1),
            _ => unreachable!(),
        }
        let mut store = InMemoryStore::new();
        store.restore_record(original.clone()).unwrap();
        assert!(
            store.save_record(record).is_err(),
            "provenance corruption {corruption}"
        );
    }
}
#[derive(Clone)]
struct CurrentAccess(Arc<std::sync::atomic::AtomicBool>);
impl wos_core::traits::AccessControl for CurrentAccess {
    fn can_transition(&self, _: &str, _: &str) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    fn can_read(&self, _: &str, _: &str) -> bool {
        true
    }
    fn can_delegate(
        &self,
        _: &str,
        _: &str,
        _: &wos_core::model::governance::DelegationScope,
    ) -> bool {
        false
    }
}
#[test]
fn current_access_denies_actual_determination_and_notice_before_effects_or_replay() {
    let admitted = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (mut runtime, _, signing) = signed_runtime_with_access(CurrentAccess(admitted.clone()));
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    let before = runtime.load_reference_record(&process).unwrap();
    assert!(matches!(
        runtime.drain_once(&process),
        Err(RuntimeError::Governance(RefusalCode::ActorRejected))
    ));
    assert_eq!(
        runtime
            .load_reference_record(&process)
            .unwrap()
            .provenance_log
            .len(),
        before.provenance_log.len()
    );
    admitted.store(true, Ordering::SeqCst);
    runtime.drain_once(&process).unwrap();
    let (task, response) = signed_response(&runtime, &process, &signing);
    assert!(matches!(
        runtime
            .submit_task_response(&task, response, "actor:human:supervisor", None)
            .unwrap(),
        wos_runtime::TaskSubmissionResult::Completed { .. }
    ));
    let d = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .determinations[0]
        .id
        .clone();
    admitted.store(false, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &d,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ActorRejected
        }
    );
    admitted.store(true, Ordering::SeqCst);
    assert!(matches!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &d,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Committed { .. }
    ));
    admitted.store(false, Ordering::SeqCst);
    assert_eq!(
        command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &d,
            GovernanceCommand::IssueNotice
        ),
        GovernanceResult::Refused {
            code: RefusalCode::ActorRejected
        }
    );
}
#[test]
fn a_review_role_alias_for_the_determining_human_cannot_schedule_independent_hearing() {
    let (mut runtime, _, signing) = signed_runtime();
    let process = runtime
        .create_process_with_actor_bindings(
            CreateProcessRequest {
                process_id: String::new(),
                tenant: Some("test".into()),
                definition_url: "urn:test:reference-due".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            },
            ["determiner", "supervisor", "household", "reviewer"]
                .iter()
                .map(|id| HumanActorBinding {
                    declared_actor: (*id).into(),
                    assigned_actor: format!(
                        "actor:human:{}",
                        if *id == "reviewer" { "determiner" } else { id }
                    ),
                })
                .collect(),
        )
        .unwrap()
        .process_id;
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    let (task, response) = signed_response(&runtime, &process, &signing);
    runtime
        .submit_task_response(&task, response, "actor:human:supervisor", None)
        .unwrap();
    let d = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap()
        .determinations[0]
        .id
        .clone();
    command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &d,
        GovernanceCommand::IssueNotice,
    );
    let filed = command(
        &mut runtime,
        &process,
        "appeal",
        "household",
        &d,
        GovernanceCommand::FileAppeal {
            channel: Channel::Phone,
            continuation_choice: ContinuationChoice::Elect,
        },
    );
    let GovernanceResult::Committed {
        outcome: GovernanceOutcome::AppealFiled { appeal_id },
        ..
    } = filed
    else {
        panic!("{filed:?}")
    };
    let request = GovernanceRequest {
        operation_ref: "schedule".into(),
        determination_id: Some(d),
        idempotency_token: "alias-review".into(),
        actor: GovernanceActor {
            declared_actor: "reviewer".into(),
            acting_actor: "actor:human:determiner".into(),
        },
        command: GovernanceCommand::ScheduleHearing {
            appeal_id,
            reviewer_actor: "reviewer".into(),
            scheduled_for: "1970-01-01T00:00:00Z".into(),
        },
    };
    assert_eq!(
        runtime
            .governance_operation(&process, "actor:human:determiner", request)
            .unwrap(),
        GovernanceResult::Refused {
            code: RefusalCode::ActorRejected
        }
    );
    let state = runtime
        .load_reference_record(&process)
        .unwrap()
        .process
        .governance_state
        .unwrap()
        .due_process
        .unwrap();
    assert!(state.appeals[0].schedule.is_none());
}
#[test]
fn delayed_child_extension_uses_parent_original_deadline_and_cumulative_cap() {
    let mut raw = fixture();
    let mut determining = raw["lifecycle"]["states"]["open"]["transitions"][0].clone();
    determining["target"] = json!("pending");
    raw["lifecycle"]["states"]
        .as_object_mut()
        .unwrap()
        .remove("open");
    raw["lifecycle"]["initialState"] = json!("phase");
    raw["lifecycle"]["states"]["phase"] = json!({"type":"compound","initialState":"waiting","onEntry":[{"action":"startTimer","timerId":"pre-notice","duration":"P10D","event":{"kind":"timer","timerId":"pre-notice","source":"custom","firesAs":"expired"}}],"states":{"waiting":{"type":"atomic","transitions":[{"event":"extend","target":"waiting","actor":"supervisor"},determining]}}});
    let execution = &mut raw["governance"]["dueProcess"]["execution"];
    execution["determinations"][0]["transition"] =
        json!({"statePointer":"/lifecycle/states/phase/states/waiting","transitionIndex":1});
    execution["preNoticeExtension"] = json!({"timerId":"pre-notice","allowedStatePointers":["/lifecycle/states/phase/states/waiting"],"maxCumulativeCalendarDays":5,"sourceRefs":[source()]});
    execution["operations"].as_array_mut().unwrap().push(json!({"id":"extend","kind":"extendPreNoticeWindow","determinationRef":"adverse","actors":["supervisor"],"requiredStatePointers":["/lifecycle/states/phase/states/waiting"],"triggerEvent":"extend","transition":{"statePointer":"/lifecycle/states/phase/states/waiting","transitionIndex":0},"sourceRefs":[source()]}));
    raw["governance"]["qualityControls"] = json!({"overrideAuthority":{"requireStructuredRationale":true,"requireAuthorityVerification":true,"requireSupportingEvidence":true}});
    let (mut runtime, time, _) = runtime(&raw);
    let mut cap = capabilities(&raw);
    cap.retained_evidence.push(EvidenceSelection {
        id: "override".into(),
        role: EvidenceRole::OverrideEvidence,
        bytes_base64: STANDARD.encode(b"retained extension exhibit"),
        sha256: bytes_digest(b"retained extension exhibit"),
    });
    runtime.configure_reference_due_process(cap).unwrap();
    time.0.store(86_400_000, Ordering::SeqCst);
    let process = create(&mut runtime).unwrap();
    let original = runtime.load_process(&process).unwrap().timers[0]
        .deadline
        .clone();
    assert_eq!(original, "1970-01-12T00:00:00Z");
    time.0.store(4 * 86_400_000, Ordering::SeqCst);
    let request = |token: &str, days: u32| GovernanceRequest {
        operation_ref: "extend".into(),
        determination_id: None,
        idempotency_token: token.into(),
        actor: GovernanceActor {
            declared_actor: "supervisor".into(),
            acting_actor: "actor:human:supervisor".into(),
        },
        command: GovernanceCommand::ExtendPreNoticeWindow {
            calendar_days: days,
            rationale: "Reviewed the retained extension exhibit.".into(),
            evidence_refs: vec![SourceRef {
                source_version: "override".into(),
                section_anchor: "exhibit.1".into(),
                retained_sha256: bytes_digest(b"retained extension exhibit"),
            }],
        },
    };
    let first = runtime
        .governance_operation(&process, "actor:human:supervisor", request("extension1", 3))
        .unwrap();
    assert!(
        matches!(first, GovernanceResult::Committed { .. }),
        "{first:?}"
    );
    assert_eq!(
        runtime.load_process(&process).unwrap().timers[0].deadline,
        "1970-01-15T00:00:00Z"
    );
    time.0.store(5 * 86_400_000, Ordering::SeqCst);
    let second = runtime
        .governance_operation(&process, "actor:human:supervisor", request("extension2", 2))
        .unwrap();
    assert!(
        matches!(second, GovernanceResult::Committed { .. }),
        "{second:?}"
    );
    assert_eq!(
        runtime.load_process(&process).unwrap().timers[0].deadline,
        "1970-01-17T00:00:00Z"
    );
    assert_eq!(
        runtime
            .governance_operation(&process, "actor:human:supervisor", request("extension1", 3))
            .unwrap(),
        first
    );
    assert_eq!(
        runtime
            .governance_operation(&process, "actor:human:supervisor", request("extension3", 1))
            .unwrap(),
        GovernanceResult::Refused {
            code: RefusalCode::OverrideRejected
        }
    );
    let record = runtime.load_reference_record(&process).unwrap();
    let state = record
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap();
    assert_eq!(state.extensions.len(), 2);
    assert_eq!(state.extensions[0].prior_deadline, original);
    assert_eq!(state.extensions[1].cumulative_calendar_days, 5);
    for extension in &state.extensions {
        assert_eq!(
            extension.override_record.authority_verification.actor_id,
            "actor:human:supervisor"
        );
        assert!(!extension.override_record.supporting_evidence.is_empty());
        assert!(record.provenance_log.iter().any(|p| p.record_kind
            == wos_core::ProvenanceKind::OverrideRecorded
            && p.data.as_ref()
                == Some(&serde_json::to_value(&extension.override_record).unwrap())));
    }
}

#[test]
fn canonical_audit_executes_isolated_protected_change_and_actual_records() {
    let (mut runtime, _, calls) = runtime(&fixture());
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    let drain = runtime.drain_once(&process).unwrap();
    let counterfactual = drain
        .provenance
        .iter()
        .find(|r| {
            r.record_kind == wos_core::ProvenanceKind::DeterminationCounterfactual
                && r.data.as_ref().unwrap()["type"] == "negative"
        })
        .unwrap();
    let data = counterfactual.data.as_ref().unwrap();
    assert_eq!(data["alternative"]["changes"][0]["casePointer"], "/group");
    assert_ne!(
        data["originalSnapshotSha256"],
        data["alternateSnapshotSha256"]
    );
    assert_eq!(
        data["originalEvaluation"]["outputs"],
        data["alternateEvaluation"]["outputs"]
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn canonical_audit_rejects_unsupported_or_unbound_requirements_before_operations() {
    for mutation in 0..12 {
        let mut raw = fixture();
        let explanation = "/governance/dueProcess/execution/determinations/0/explanation";
        match mutation {
            0 => {
                raw["governance"].as_object_mut().unwrap().remove("audit");
            }
            1 => {
                raw["governance"]["audit"]["reasoningTier"]["requiredForTags"] = json!([]);
            }
            2 => {
                raw["governance"]["audit"]["counterfactualTier"]["requiredForTags"] = json!([]);
            }
            3 => {
                raw["governance"]["audit"]["reasoningTier"]["requireDecisionRequirements"] =
                    json!(true);
            }
            4 => {
                raw["governance"]["audit"]["counterfactualTier"]["requireProtectedCharacteristics"] =
                    json!(false);
            }
            5 => {
                raw["governance"]["audit"]["counterfactualTier"]["unknownRequiredBehavior"] =
                    json!(true);
            }
            6 => {
                raw.pointer_mut(explanation)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("protectedCharacteristics");
            }
            7 => {
                raw.pointer_mut(explanation).unwrap()["protectedCharacteristics"][0]["sourceRefs"]
                    [0]["retainedSha256"] = json!("sha256:foreign");
            }
            8 => {
                raw.pointer_mut(explanation).unwrap()["negative"][0]["changes"][0]["casePointer"] =
                    json!("/income");
            }
            9 => {
                raw.pointer_mut(explanation).unwrap()["negative"][0]["changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"casePointer":"/income","value":{"kind":"number","value":150}}));
            }
            10 => {
                raw["lifecycle"]["states"]["extra"] = json!({"type":"compound","initialState":"inner","states":{"inner":{"type":"atomic","transitions":[{"event":"extra","target":"inner","tags":["determination"]}]}}});
            }
            _ => {
                raw["lifecycle"]["states"]["extra"] = json!({"type":"foreach","collection":"[1]","itemVariable":"item","body":{"id":"item-body","type":"atomic","transitions":[{"event":"extra","target":"item-body","tags":["determination"]}]}});
            }
        }
        let (mut runtime, _, calls) = runtime(&fixture());
        let result = runtime.configure_reference_due_process(capabilities(&raw));
        if mutation >= 10 {
            assert!(
                matches!(
                    result,
                    Err(RuntimeError::Governance(RefusalCode::UnboundTransition))
                ),
                "wrong coverage failure for {mutation}: {result:?}"
            );
        } else {
            assert!(result.is_err(), "accepted mutation {mutation}");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn protected_default_is_required_but_transition_shaped_metadata_is_not_a_state() {
    let mut raw = fixture();
    raw["governance"]["audit"]["counterfactualTier"]
        .as_object_mut()
        .unwrap()
        .remove("requireProtectedCharacteristics");
    raw["extensions"] = json!({"x-fixture-metadata":{"transitions":[{"tags":["determination","adverse-decision"],"event":"fake","target":"unbound"}]}});
    let (mut runtime, _, calls) = runtime(&raw);
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    assert!(runtime.drain_once(&process).is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[test]
fn protected_noop_and_changed_outcome_refuse_without_saving_fabricated_evidence() {
    for change in [
        json!({"kind":"string","value":"a"}),
        json!({"kind":"string","value":"b"}),
    ] {
        let mut raw = fixture();
        raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["negative"]
            [0]["changes"][0]["value"] = change.clone();
        if change["value"] == "b" {
            raw["decisionTables"][0]["inputs"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"group","type":"string"}));
            raw["decisionTables"][0]["rows"][0]["inputCells"]
                .as_array_mut()
                .unwrap()
                .push(json!("group = \"a\""));
            raw["decisionTables"][0]["rows"][1]["inputCells"] = json!(["true", "true"]);
            raw["lifecycle"]["states"]["open"]["transitions"][0]["guard"]["inputBindings"]["group"] =
                json!("caseFile.group");
        }
        let (mut runtime, _, calls) = runtime(&raw);
        let process = create(&mut runtime).unwrap();
        enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
        let before = runtime.load_reference_record(&process).unwrap();
        assert!(runtime.drain_once(&process).is_err());
        let after = runtime.load_reference_record(&process).unwrap();
        assert_eq!(before.provenance_log.len(), after.provenance_log.len());
        assert_eq!(before.process.case_state, after.process.case_state);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn shared_static_checks_preserve_policy_versus_evidence_refusals() {
    for (pointer, value, expected) in [
        (
            "/governance/dueProcess/adverseDecisionPolicy/counterfactualRequired",
            json!(false),
            RefusalCode::UnsupportedProfile,
        ),
        (
            "/governance/dueProcess/adverseDecisionPolicy/explanationLevel",
            json!("categorical"),
            RefusalCode::UnsupportedProfile,
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/positive",
            json!([]),
            RefusalCode::ExplanationRejected,
        ),
        (
            "/governance/dueProcess/execution/determinations/0/explanation/negative",
            json!([]),
            RefusalCode::ExplanationRejected,
        ),
    ] {
        let mut raw = fixture();
        *raw.pointer_mut(pointer).unwrap() = value;
        let (mut runtime, _, calls) = runtime(&fixture());
        let result = runtime.configure_reference_due_process(capabilities(&raw));
        assert!(
            matches!(&result,Err(RuntimeError::Governance(actual)) if actual==&expected),
            "wrong refusal for {pointer}: {result:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn multiple_actual_table_inputs_create_explanation_before_any_clock_retry() {
    let mut raw = fixture();
    raw["caseFile"]["fields"]["householdSize"] = json!({"type":"number","default":1});
    raw["decisionTables"][0]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"householdSize","type":"number"}));
    for row in raw["decisionTables"][0]["rows"].as_array_mut().unwrap() {
        row["inputCells"]
            .as_array_mut()
            .unwrap()
            .push(json!("householdSize > 0"));
    }
    raw["lifecycle"]["states"]["open"]["transitions"][0]["guard"]["inputBindings"]["householdSize"] =
        json!("caseFile.householdSize");
    let (mut runtime, time, calls) = runtime(&raw);
    let process = create(&mut runtime).unwrap();
    enqueue(&mut runtime, &process, "decide", "actor:human:determiner");
    runtime.drain_once(&process).unwrap();
    let saved = runtime.load_reference_record(&process).unwrap();
    assert_eq!(time.0.load(Ordering::SeqCst), 0);
    assert_eq!(saved.process.configuration, ["pending"]);
    let due = saved
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap();
    assert_eq!(due.determinations.len(), 1);
    assert_eq!(due.determinations[0].determined_at, "1970-01-01T00:00:00Z");
    let reasoning: Vec<_> = saved
        .provenance_log
        .iter()
        .filter(|p| p.record_kind == wos_core::ProvenanceKind::DeterminationReasoning)
        .collect();
    assert_eq!(reasoning.len(), 1);
    let evaluation = &reasoning[0].data.as_ref().unwrap()["evaluation"];
    assert_eq!(
        evaluation["inputs"],
        json!([{"name":"householdSize","value":{"kind":"number","value":1}},{"name":"income","value":{"kind":"number","value":150}}])
    );
    let alternatives: Vec<_> = saved
        .provenance_log
        .iter()
        .filter(|p| p.record_kind == wos_core::ProvenanceKind::DeterminationCounterfactual)
        .collect();
    assert_eq!(alternatives.len(), 2);
    for alternative in alternatives {
        assert_eq!(
            &alternative.data.as_ref().unwrap()["originalEvaluation"],
            evaluation
        );
    }
    assert_eq!(saved.process.case_state["level"], 100);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn issued_notice_body_contains_only_selected_recipient_content() {
    let (runtime, _, process, _) = issued();
    let saved = notice(&runtime, &process);
    assert_eq!(saved.variants[0].body, "household 1970-04-01T00:00:00Z\n");
    assert_notice_schema(&saved);
    let bytes = STANDARD.decode(&saved.artifact_bytes_base64).unwrap();
    assert_eq!(bytes_digest(&bytes), saved.artifact_sha256);
    let artifact: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(artifact["machineReadable"]["kind"], "adverseDecisionNotice");
    assert_eq!(
        artifact["machineReadable"]["determinationId"],
        saved.determination_id
    );
    assert!(
        !artifact["machineReadable"]["explanation"]["reasoning"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let record = runtime.load_reference_record(&process).unwrap();
    let determination = &record
        .process
        .governance_state
        .as_ref()
        .unwrap()
        .due_process
        .as_ref()
        .unwrap()
        .determinations[0];
    let policy = signed_fixture()["governance"]["dueProcess"]["execution"].clone();
    assert_eq!(
        artifact["machineReadable"],
        json!({
            "kind":"adverseDecisionNotice","determinationId":determination.id,
            "firing":determination.firing,"snapshotSha256":determination.snapshot.sha256,
            "explanation":{"transitionId":determination.id,"determination":["adverse-decision"],
                "reasoning":[{"ruleId":"income","authority":"policy","description":"Income exceeds the selected limit.","timestamp":"1970-01-01T00:00:00Z"}],
                "positiveCounterfactual":[{"factor":"lower-income","description":"Lower income changes the selected result.","type":"positive"}],
                "negativeCounterfactual":[{"factor":"other-group","description":"This executed group alternative preserves the result.","type":"negative"}],
                "assembledAt":"1970-01-01T00:00:00Z"},
            "sourceRefs":policy["notice"]["sourceRefs"],"templates":policy["notice"]["templates"],
            "recipient":"household","appealDeadline":"1970-04-01T00:00:00Z","effectEligibleAt":"1970-01-11T00:00:00Z",
            "appealChannels":["phone","in-person","written"],"continuation":policy["appeal"]["continuation"]
        })
    );
    assert_eq!(artifact["variants"][0]["body"], saved.variants[0].body);
    assert_eq!(
        bytes_digest(saved.variants[0].body.as_bytes()),
        saved.variants[0].body_sha256
    );
}

fn individualized_capability(signing: &Signing, raw: &Value) -> ReferenceDueProcessCapabilities {
    let mut cap = selected_capability(signing, DeterministicNoticeTransport::new([]));
    cap.workflow_bytes = serde_json::to_vec(raw).unwrap();
    cap.templates = vec![serde_json::from_value(json!({"templates":{"notice":{
        "category":"adverse-decision","localeRef":"en",
        "requiredVariables":["reasonCode","inputs.income","counterfactuals.lower-income.changes.0.oldValue","counterfactuals.lower-income.changes.0.newValue","counterfactuals.other-group.changes.0.oldValue","counterfactuals.other-group.changes.0.newValue"],
        "sections":[{"id":"facts","contentType":"text","content":"Reason {{reasonCode}}; income {{inputs.income}}. Tested income {{counterfactuals.lower-income.changes.0.oldValue}} to {{counterfactuals.lower-income.changes.0.newValue}} changes the result. Tested group {{counterfactuals.other-group.changes.0.oldValue}} to {{counterfactuals.other-group.changes.0.newValue}} preserves the result."}]
    }}})).unwrap()];
    cap
}

#[test]
fn issued_notice_binds_actual_individual_inputs_and_tested_alternative_values() {
    for income in [150, 200] {
        let mut raw = signed_fixture();
        raw["decisionTables"][0]["rows"]
            .as_array_mut()
            .unwrap()
            .insert(
                0,
                json!({"id":"higher","inputCells":["income > 175"],"outputCells":["true"]}),
            );
        let mut reason = raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["reasons"][0].clone();
        reason["rowId"] = json!("higher");
        reason["reasonCode"] = json!("higher-income");
        raw["governance"]["dueProcess"]["execution"]["determinations"][0]["explanation"]["reasons"]
            .as_array_mut()
            .unwrap()
            .push(reason);
        let (mut runtime, _, process, determination, signing) = signed_only_with_initial_case(
            raw.clone(),
            Some(json!({"income":income,"group":"a","level":100,"recipient":"household"})),
        );
        runtime
            .configure_reference_due_process(individualized_capability(&signing, &raw))
            .unwrap();
        let result = command(
            &mut runtime,
            &process,
            "issue",
            "supervisor",
            &determination,
            GovernanceCommand::IssueNotice,
        );
        assert!(
            matches!(result, GovernanceResult::Committed { .. }),
            "{result:?}"
        );
        let saved = notice(&runtime, &process);
        let reason = if income == 200 {
            "higher-income"
        } else {
            "income"
        };
        assert_eq!(
            saved.variants[0].body,
            format!(
                "Reason {reason}; income {income}. Tested income {income} to 50 changes the result. Tested group a to b preserves the result.\n"
            )
        );
        assert_eq!(
            bytes_digest(saved.variants[0].body.as_bytes()),
            saved.variants[0].body_sha256
        );
    }
}

#[test]
fn issued_notice_facts_preserve_original_snapshot_after_actual_case_edit() {
    let mut raw = signed_fixture();
    raw["lifecycle"]["states"]["pending"]["transitions"].as_array_mut().unwrap().push(json!({
        "event":"edit","target":"pending","actor":"supervisor",
        "actions":[{"action":"setData","path":"caseFile.income","value":999},{"action":"setData","path":"caseFile.group","value":"z"}]
    }));
    let (mut runtime, _, process, determination, signing) =
        signed_only_with_initial_case(raw.clone(), None);
    enqueue(&mut runtime, &process, "edit", "actor:human:supervisor");
    runtime.drain_once(&process).unwrap();
    let edited = runtime.load_process(&process).unwrap();
    assert_eq!(edited.case_state["income"], 999);
    assert_eq!(edited.case_state["group"], "z");
    runtime
        .configure_reference_due_process(individualized_capability(&signing, &raw))
        .unwrap();
    let result = command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    assert!(
        matches!(result, GovernanceResult::Committed { .. }),
        "{result:?}"
    );
    assert_eq!(
        notice(&runtime, &process).variants[0].body,
        "Reason income; income 150. Tested income 150 to 50 changes the result. Tested group a to b preserves the result.\n"
    );
}

#[test]
fn missing_required_individual_fact_refuses_without_saved_notice_work() {
    for content in [
        "required-absent",
        "{{inputs.absent}}",
        "{{inputs.income",
        "unexpected }}",
        "{{inputs.income{{recipient}}",
    ] {
        let (mut runtime, _, process, determination, signing) = signed_only();
        let mut cap = individualized_capability(&signing, &signed_fixture());
        let template = cap.templates[0].templates.get_mut("notice").unwrap();
        if content == "required-absent" {
            template.required_variables.push("inputs.absent".into());
        } else {
            template.sections[0].content = Some(content.into());
        }
        runtime.configure_reference_due_process(cap).unwrap();
        let before = runtime.load_reference_record(&process).unwrap();
        assert_eq!(
            command(
                &mut runtime,
                &process,
                "issue",
                "supervisor",
                &determination,
                GovernanceCommand::IssueNotice
            ),
            GovernanceResult::Refused {
                code: RefusalCode::UnsupportedProfile
            }
        );
        let after = runtime.load_reference_record(&process).unwrap();
        assert_eq!(
            serde_json::to_value(&before.process).unwrap(),
            serde_json::to_value(&after.process).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&before.provenance_log).unwrap(),
            serde_json::to_value(&after.provenance_log).unwrap()
        );
    }
}

#[test]
fn individual_notice_facts_and_hashes_survive_failed_delivery_and_recreation() {
    let (mut runtime, time, process, determination, signing) = signed_only();
    let transport = DeterministicNoticeTransport::new([1]);
    let mut cap = individualized_capability(&signing, &signed_fixture());
    cap.notice_transport = Box::new(transport.clone());
    runtime.configure_reference_due_process(cap).unwrap();
    let result = command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    assert!(
        matches!(result, GovernanceResult::Committed { .. }),
        "{result:?}"
    );
    let original = notice(&runtime, &process);
    assert!(original.receipt.is_none());
    assert_eq!(original.attempts, 1);
    let mut store = InMemoryStore::new();
    store
        .restore_record(runtime.load_reference_record(&process).unwrap())
        .unwrap();
    // Retry at the admitted issue instant; fresh late receipts have a separate
    // fail-closed anchor test above and are not qualified by this test.
    // Recreated runtime selects the ordinary template: pending work must keep
    // its original individualized body instead of rendering that new selection.
    let mut restored = recreated(store, time, &signing, transport);
    restored.deliver_pending_notices(&process).unwrap();
    let recovered = notice(&restored, &process);
    assert!(recovered.receipt.is_some());
    assert_eq!(
        recovered.artifact_bytes_base64,
        original.artifact_bytes_base64
    );
    assert_eq!(recovered.artifact_sha256, original.artifact_sha256);
    assert_eq!(recovered.variants, original.variants);
    assert_eq!(recovered.transport_key, original.transport_key);
    assert_eq!(recovered.issue_at, original.issue_at);
    assert_eq!(recovered.appeal_deadline, original.appeal_deadline);
    assert_eq!(recovered.effect_eligible_at, original.effect_eligible_at);
}

#[test]
fn malformed_or_ambiguous_retained_notice_facts_refuse_before_saved_work() {
    for mutation in [
        "duplicate-input",
        "snapshot",
        "location",
        "missing-old",
        "duplicate-alternative",
    ] {
        let (runtime, time, process, determination, signing) = signed_only();
        let mut original = runtime.load_reference_record(&process).unwrap();
        if mutation == "duplicate-input" || mutation == "location" {
            let data = original
                .provenance_log
                .iter_mut()
                .find(|record| {
                    record.record_kind == wos_core::ProvenanceKind::DeterminationReasoning
                })
                .unwrap()
                .data
                .as_mut()
                .unwrap();
            if mutation == "duplicate-input" {
                let input = data["evaluation"]["inputs"][0].clone();
                data["evaluation"]["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .push(input);
            } else {
                data["evaluation"]["location"]["transitionIndex"] = json!(99);
            }
        } else {
            let counter = original
                .provenance_log
                .iter_mut()
                .find(|record| {
                    record.record_kind == wos_core::ProvenanceKind::DeterminationCounterfactual
                })
                .unwrap();
            if mutation == "duplicate-alternative" {
                let mut duplicate = counter.clone();
                duplicate.id = stack_common_typeid::TypeIdMinter::for_parent(&process)
                    .unwrap()
                    .mint_provenance_id();
                original.provenance_log.push(duplicate);
            } else {
                let data = counter.data.as_mut().unwrap();
                if mutation == "snapshot" {
                    data["originalSnapshotSha256"] = json!("0".repeat(64));
                } else {
                    data["alternative"]["changes"][0]["casePointer"] = json!("/absent");
                }
            }
        }
        let mut store = InMemoryStore::new();
        store.restore_record(original).unwrap();
        let transport = DeterministicNoticeTransport::new([]);
        let mut restored = recreated(store, time, &signing, transport.clone());
        restored
            .configure_reference_due_process(individualized_capability(&signing, &signed_fixture()))
            .unwrap();
        let before = restored.load_reference_record(&process).unwrap();
        let result = restored.governance_operation(
            &process,
            "actor:human:supervisor",
            GovernanceRequest {
                operation_ref: "issue".into(),
                determination_id: Some(determination),
                idempotency_token: "malformed.issue".into(),
                actor: GovernanceActor {
                    declared_actor: "supervisor".into(),
                    acting_actor: "actor:human:supervisor".into(),
                },
                command: GovernanceCommand::IssueNotice,
            },
        );
        assert_eq!(
            result.unwrap(),
            GovernanceResult::Refused {
                code: RefusalCode::ExplanationRejected
            },
            "{mutation}"
        );
        let after = restored.load_reference_record(&process).unwrap();
        assert_eq!(
            serde_json::to_value(&before.process).unwrap(),
            serde_json::to_value(&after.process).unwrap(),
            "{mutation}"
        );
        assert_eq!(
            serde_json::to_value(&before.provenance_log).unwrap(),
            serde_json::to_value(&after.provenance_log).unwrap(),
            "{mutation}"
        );
    }
}

#[test]
fn issued_notice_treats_placeholder_shaped_factual_values_as_literal_text() {
    let literal = "{{counterfactuals.other-group.changes.0.newValue}}";
    let raw = signed_fixture();
    let (mut runtime, _, process, determination, signing) = signed_only_with_initial_case(
        raw.clone(),
        Some(json!({"income":150,"group":literal,"level":100,"recipient":"household"})),
    );
    let record = runtime.load_reference_record(&process).unwrap();
    let original = record
        .provenance_log
        .iter()
        .find(|record| record.id == determination)
        .unwrap();
    assert_eq!(
        original.case_file_snapshot.as_ref().unwrap().value["group"],
        literal
    );
    runtime
        .configure_reference_due_process(individualized_capability(&signing, &raw))
        .unwrap();
    let result = command(
        &mut runtime,
        &process,
        "issue",
        "supervisor",
        &determination,
        GovernanceCommand::IssueNotice,
    );
    assert!(
        matches!(result, GovernanceResult::Committed { .. }),
        "{result:?}"
    );
    let saved = notice(&runtime, &process);
    assert_eq!(
        saved.variants[0].body,
        format!(
            "Reason income; income 150. Tested income 150 to 50 changes the result. Tested group {literal} to b preserves the result.\n"
        )
    );
    let bytes = STANDARD.decode(&saved.artifact_bytes_base64).unwrap();
    let artifact: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        artifact["machineReadable"]["snapshotSha256"],
        original.case_file_snapshot.as_ref().unwrap().sha256
    );
    assert_eq!(bytes_digest(&bytes), saved.artifact_sha256);
    assert_eq!(
        bytes_digest(saved.variants[0].body.as_bytes()),
        saved.variants[0].body_sha256
    );
}

#[test]
fn generic_and_reference_owners_refuse_both_installation_orders() {
    use wos_runtime::governance::{GovernancePolicySelection, QualifiedGovernancePolicy};
    let raw = json!({"$wosWorkflow":"1.0","url":"urn:test:generic-owner","version":"1.0.0","governance":{},
        "lifecycle":{"initialState":"open","states":{"open":{"type":"atomic","transitions":[{"event":"human.completed","target":"done","tags":["determination"]}]},"done":{"type":"final"}}}});
    let policy=QualifiedGovernancePolicy::qualify(serde_json::from_value::<GovernancePolicySelection>(json!({"version":1,"kernel":raw,"delivery":{"targetWorkflow":"urn:test:generic-owner"},"humanDecisions":[{"transition":{"from":"open","to":"done","event":"human.completed"},"taskRef":"review","evidencePointer":"/data/governance","rules":[{"ruleId":"rule:human"}],"criteria":["retained"],"caseFieldPaths":["/retained"],"positiveFactors":[],"negativeFactors":[]}]})).unwrap()).unwrap();
    let reference = fixture();
    let (reference_runtime, _, _) = runtime(&reference);
    assert!(matches!(
        reference_runtime.with_governance_policy(policy.clone(), Default::default()),
        Err(RuntimeError::GovernancePolicy(_))
    ));
    let mut generic_runtime = WosRuntime::new(
        InMemoryStore::new(),
        Resolver(serde_json::from_value(raw).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Services(Arc::new(AtomicUsize::new(0))),
        DefaultRuntime::new(),
        Time(Arc::new(AtomicU64::new(0))),
        BindingRegistry::new(),
    )
    .with_governance_policy(policy, Default::default())
    .unwrap();
    assert!(matches!(
        generic_runtime.configure_reference_due_process(capabilities(&reference)),
        Err(RuntimeError::Governance(RefusalCode::UnsupportedProfile))
    ));
}
