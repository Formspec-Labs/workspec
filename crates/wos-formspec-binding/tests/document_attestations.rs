// Rust guideline compliant 2026-02-21
//! Genuine cross-owner fixtures produced by the maintained Formspec signing helper.
use serde_json::Value;
use wos_formspec_binding::selected::{
    ImmutableFormspecProcessor, SelectedDocument, SelectedDocumentKind,
};
use wos_formspec_binding::{FormspecProcessor, parse_authored_signatures};

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/document-attestations")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn processor() -> ImmutableFormspecProcessor {
    let definition = fixture("definition.json");
    ImmutableFormspecProcessor::new(&[SelectedDocument {
        kind: SelectedDocumentKind::Definition,
        url: definition["url"].as_str().unwrap().into(),
        version: definition["version"].as_str().unwrap().into(),
        bytes: serde_json::to_vec(&definition).unwrap(),
        definition: None,
    }])
    .unwrap()
    .with_clock(Clock)
}
#[test]
fn declared_document_claims_survive_the_wos_parser() {
    let response = fixture("opaque-principal.response.json");
    let parsed = parse_authored_signatures(&response).unwrap();
    let copied = serde_json::to_value(&parsed[0]).unwrap();
    for field in [
        "signingActId",
        "documentRef",
        "presentationHash",
        "witnessedSignatureRef",
    ] {
        assert_eq!(
            copied.get(field),
            response["authoredSignatures"][0].get(field),
            "lost {field}"
        );
    }
}
#[test]
fn selected_native_definition_qualifies_declared_document_response() {
    let errors = processor()
        .validate_envelope(&fixture("opaque-principal.response.json"))
        .unwrap();
    assert!(
        errors.is_empty(),
        "actual selected native Definition required: {errors:?}"
    );
}

use base64::{Engine as _, engine::general_purpose::STANDARD};
use integrity_signature::{KeyRef, MethodRegistry};
use integrity_signature_ring::RingVerifier;
use std::collections::BTreeMap;
use wos_formspec_binding::{DocumentSignatureVerification, FormspecBinding};
use wos_runtime::{ContractBindingAdapter, RuntimeStore, TaskSubmissionResult};

fn binding() -> FormspecBinding<ImmutableFormspecProcessor> {
    binding_with_registry(serde_json::from_value(fixture("registry.json")).unwrap())
}
fn binding_with_registry(registry: MethodRegistry) -> FormspecBinding<ImmutableFormspecProcessor> {
    let keys = fixture("trusted-test-keys.json")["signers"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(signer, key)| {
            (
                signer.clone(),
                KeyRef::RawPublicKey(
                    STANDARD
                        .decode(key["publicKeyBase64"].as_str().unwrap())
                        .unwrap(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    FormspecBinding::new(processor()).with_document_verification(
        DocumentSignatureVerification::new(
            RingVerifier::new_for_method_prefix("urn:formspec:sig-method:"),
            registry,
            keys,
        ),
    )
}
fn task(actor: &str) -> wos_core::instance::ActiveTask {
    serde_json::from_value(serde_json::json!({
        "taskId":"wos-task:test:1:terms", "taskRef":"terms", "status":"assigned", "assignedActor":actor,
        "binding":"formspec", "definitionUrl":"urn:test:document-attestation", "definitionVersion":"1.0.0",
        "createdAt":"2026-10-20T19:00:00Z", "updatedAt":"2026-10-20T19:00:00Z"
    })).unwrap()
}
#[test]
fn real_ed25519_source_preserves_both_document_records() {
    let source = fixture("opaque-principal.response.json");
    let evidence = binding()
        .signature_evidence(&task("applicant"), &source)
        .unwrap()
        .unwrap();
    assert_eq!(evidence.len(), 2);
    for (index, record) in evidence.iter().enumerate() {
        let claims = record.document_claims.as_ref().unwrap();
        assert_eq!(claims.response_id, "ih9d-principal-response");
        assert_eq!(
            serde_json::to_value(&claims.document_ref).unwrap(),
            source["authoredSignatures"][index]["documentRef"]
        );
        assert_eq!(
            claims.signing_act_id,
            source["authoredSignatures"][index]["signingActId"]
        );
        assert_eq!(
            claims.presentation_hash,
            source["authoredSignatures"][index]["presentationHash"]
        );
        assert_eq!(
            record.primitive_verification,
            wos_runtime::SignaturePrimitiveStatus::Verified
        );
        assert!(
            record.source_response_ref.is_none(),
            "runtime owns actual retained locator"
        );
    }
}
#[test]
fn genuine_root_outer_and_missing_claim_controls_refuse() {
    for name in [
        "negative-root-tamper.response.json",
        "negative-outer-mirror.response.json",
        "negative-wos-missing-presentation.response.json",
        "legacy-principal.response.json",
    ] {
        assert!(
            binding()
                .retained_document_signature_evidence(&fixture(name), "wos-task:test:1:terms")
                .is_err(),
            "accepted {name}"
        );
    }
    let source = fixture("opaque-principal.response.json");
    assert!(
        FormspecBinding::new(processor())
            .signature_evidence(&task("applicant"), &source)
            .is_err()
    );
    let mut signed_task_conflict = source.clone();
    signed_task_conflict["data"]["signatureTaskId"] = "different-task".into();
    assert!(
        binding()
            .signature_evidence(&task("applicant"), &signed_task_conflict)
            .is_err()
    );
}

const PROCESS: &str = "default_process_01hw7rm71vfay8vvw14d2pf2db";
const CASE: &str = "default_case_01hw7rm71vfay8vvw14d2pf2db";

// Actual reference runtime and native Formspec adapter; no mocked verification result.
struct Resolver(wos_core::model::kernel::KernelDocument);
impl wos_core::traits::DocumentResolver for Resolver {
    type Error = std::io::Error;
    fn resolve_kernel(
        &self,
        url: &str,
        version: &str,
    ) -> Result<wos_core::model::kernel::KernelDocument, Self::Error> {
        if self.0.url.as_deref() == Some(url) && self.0.version.as_deref() == Some(version) {
            Ok(self.0.clone())
        } else {
            Err(std::io::Error::other("unselected kernel"))
        }
    }
    fn resolve_governance(
        &self,
        _: &str,
        _: &str,
    ) -> Result<wos_core::GovernanceDocument, Self::Error> {
        Err(std::io::Error::other("unused governance"))
    }
    fn resolve_sidecar(&self, _: &str, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("unused sidecar"))
    }
}
struct NoService;
impl wos_core::traits::ExternalService for NoService {
    type Error = std::io::Error;
    fn invoke(&self, _: &str, _: &Value, _: Option<&str>) -> Result<Value, Self::Error> {
        Err(std::io::Error::other("unexpected external service"))
    }
}
struct Clock;
impl wos_runtime::Clock for Clock {
    fn now_ms(&self) -> u64 {
        1792524000000
    }
}

fn runtime() -> (wos_runtime::WosRuntime, wos_runtime::SharedInMemoryStore) {
    runtime_with_binding(binding())
}
fn runtime_with_binding(
    adapter: FormspecBinding<ImmutableFormspecProcessor>,
) -> (wos_runtime::WosRuntime, wos_runtime::SharedInMemoryStore) {
    use serde_json::json;
    let mut kernel = serde_json::to_value(wos_runtime::signature_runtime_fixture_kernel()).unwrap();
    kernel["contracts"]["signatureContract"]["ref"] = "urn:test:document-attestation".into();
    kernel["lifecycle"]["initialState"] = "signing".into();
    kernel["lifecycle"]["states"]["signing"]["onEntry"] = json!([
        {"action":"createTask","taskRef":"principalTerms","assignTo":"applicant","contractRef":"signatureContract","extensions":{"x-wos-signature-profile-key":"documents","x-wos-signature-step-id":"terms"}},
        {"action":"createTask","taskRef":"principalPrivacy","assignTo":"applicant","contractRef":"signatureContract","extensions":{"x-wos-signature-profile-key":"documents","x-wos-signature-step-id":"privacy"}},
        {"action":"createTask","taskRef":"witnessTerms","assignTo":"witness","contractRef":"signatureContract","extensions":{"x-wos-signature-profile-key":"documents","x-wos-signature-step-id":"witness"}}
    ]);
    let source = fixture("opaque-principal.response.json");
    let documents: Vec<_> = source["resolvedDocuments"].as_array().unwrap().iter().map(|document| json!({
        "id":document["documentId"],"documentRef":"urn:test:selected-document","documentHash":document["documentHash"],"documentHashAlgorithm":"sha-256"
    })).collect();
    let profile = json!({
        "roles":[{"id":"principal","role":"signer","actorId":"applicant"},{"id":"witness","role":"witness","actorId":"witness"}],
        "documents":documents,
        "signingFlow":{"type":"parallel","steps":[
            {"id":"terms","roleId":"principal","documentId":"terms-v2","signingIntent":"urn:wos:signing-intent:applicant-signature"},
            {"id":"privacy","roleId":"principal","documentId":"privacy-v1","signingIntent":"urn:wos:signing-intent:applicant-signature"},
            {"id":"witness","roleId":"witness","documentId":"terms-v2","signingIntent":"urn:wos:signing-intent:witness-attestation"}
        ],"completion":{"type":"all-required"}},
        "evidence":{"requiredFields":[],"consentReference":{"consentTextRef":"urn:example:consent:esign:v1","consentVersion":"1.0","acceptedAtPath":"response.extensions.x-formspec-document-attestations.signedAt","affirmationPath":"response.extensions.x-formspec-document-attestations.consentAccepted"},"identityBinding":{"method":"email-otp","assuranceLevel":"standard"},"custodyHookEligible":true}
    });
    let store = wos_runtime::SharedInMemoryStore::default();
    let mut bindings = wos_runtime::BindingRegistry::new();
    bindings.register(adapter);
    let mut runtime = wos_runtime::WosRuntime::new(
        store.clone(),
        Resolver(serde_json::from_value(kernel).unwrap()),
        wos_core::traits::DefaultRuntime::new(),
        wos_core::traits::DefaultRuntime::new(),
        NoService,
        wos_core::traits::DefaultRuntime::new(),
        Clock,
        bindings,
    )
    .with_signature_profile("documents", serde_json::from_value(profile).unwrap());
    runtime
        .create_process_bound_to_case(
            wos_runtime::CreateProcessRequest {
                process_id: PROCESS.into(),
                tenant: None,
                definition_url: "urn:test:signature-runtime".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            },
            CASE.into(),
        )
        .unwrap();
    (runtime, store)
}
fn submit(
    runtime: &mut wos_runtime::WosRuntime,
    task_ref: &str,
    source: &str,
    actor: &str,
) -> Result<TaskSubmissionResult, wos_runtime::RuntimeError> {
    let task = runtime
        .load_process(PROCESS)
        .unwrap()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == task_ref)
        .unwrap();
    runtime.submit_task_response(&task.task_id, fixture(source), actor, Some(task_ref))
}
fn submit_value(
    runtime: &mut wos_runtime::WosRuntime,
    task_ref: &str,
    source: Value,
    actor: &str,
) -> Result<TaskSubmissionResult, wos_runtime::RuntimeError> {
    let task = runtime
        .load_process(PROCESS)
        .unwrap()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == task_ref)
        .unwrap();
    runtime.submit_task_response(&task.task_id, source, actor, Some(task_ref))
}
fn affirmations(store: &wos_runtime::SharedInMemoryStore) -> Vec<wos_core::ProvenanceRecord> {
    store
        .load_record(PROCESS)
        .unwrap()
        .provenance_log
        .into_iter()
        .filter(|record| record.record_kind == wos_core::ProvenanceKind::SignatureAffirmation)
        .collect()
}
#[test]
fn actual_runtime_admits_two_documents_and_resolves_real_principal_record() {
    let (mut runtime, store) = runtime();
    assert!(matches!(
        submit(
            &mut runtime,
            "principalTerms",
            "opaque-principal.response.json",
            "applicant"
        )
        .unwrap(),
        TaskSubmissionResult::Completed { .. }
    ));
    assert!(matches!(
        submit(
            &mut runtime,
            "principalPrivacy",
            "opaque-principal.response.json",
            "applicant"
        )
        .unwrap(),
        TaskSubmissionResult::Completed { .. }
    ));
    let principals = affirmations(&store);
    assert_eq!(principals.len(), 2);
    let expected = &principals[0];
    assert_eq!(
        expected.data.as_ref().unwrap()["sourceSignatureId"],
        "sig-1"
    );
    assert!(matches!(
        submit(
            &mut runtime,
            "witnessTerms",
            "opaque-witness.response.json",
            "witness"
        )
        .unwrap(),
        TaskSubmissionResult::Completed { .. }
    ));
    let all = affirmations(&store);
    assert_eq!(all.len(), 3);
    let witness = all[2].data.as_ref().unwrap();
    assert_eq!(witness["witnessedSignatureRef"], expected.id);
    assert_ne!(witness["witnessedSignatureRef"], "sig-1");
    let retained = store.load_record(PROCESS).unwrap();
    for (index, record) in all.iter().enumerate() {
        let data = record.data.as_ref().unwrap();
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/wos-provenance-log.schema.json"
        ))
        .unwrap();
        let mut data_schema = schema["$defs"]["SignatureAffirmationData"].clone();
        data_schema["$defs"] = serde_json::json!({
            "SignatureIdentityBinding": schema["$defs"]["SignatureIdentityBinding"],
            "SignatureConsentReference": schema["$defs"]["SignatureConsentReference"],
            "SignatureSignerAuthority": schema["$defs"]["SignatureSignerAuthority"]
        });
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .build(&data_schema)
            .unwrap();
        assert!(
            validator.is_valid(data),
            "full owning affirmation data schema: {:?}",
            validator
                .iter_errors(data)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            data["documentRef"]["locale"],
            if index == 1 { "en" } else { "es-US" }
        );
        let artifact = retained
            .artifacts
            .get(data["sourceResponseRef"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            artifact.response["id"],
            if index == 2 {
                "ih9d-witness-response"
            } else {
                "ih9d-principal-response"
            }
        );
        let link = retained
            .provenance_log
            .iter()
            .find(|item| {
                item.data
                    .as_ref()
                    .is_some_and(|d| d["details"]["signatureAffirmationId"] == record.id)
            })
            .unwrap();
        assert_eq!(
            link.data.as_ref().unwrap()["details"]["artifactId"],
            artifact.artifact_id
        );
    }
}
#[test]
fn real_crypto_witness_with_missing_foreign_or_wrong_principal_is_refused() {
    for source in [
        "opaque-witness.response.json",
        "negative-foreign-witness-response.response.json",
        "negative-wrong-witness-digest.response.json",
        "negative-legacy-principal-target.response.json",
    ] {
        let (mut runtime, store) = runtime();
        if source != "opaque-witness.response.json" {
            submit(
                &mut runtime,
                "principalTerms",
                "opaque-principal.response.json",
                "applicant",
            )
            .unwrap();
        }
        let before = store.load_record(PROCESS).unwrap();
        assert!(
            submit(&mut runtime, "witnessTerms", source, "witness").is_err(),
            "accepted {source}"
        );
        let after = store.load_record(PROCESS).unwrap();
        assert_eq!(after.artifacts.len(), before.artifacts.len());
        assert_eq!(
            after.process.active_tasks.len(),
            before.process.active_tasks.len()
        );
        assert_eq!(after.process.case_state, before.process.case_state);
        assert_eq!(after.provenance_log.len(), before.provenance_log.len());
    }
}

fn resigned(response: Value) -> (Value, FormspecBinding<ImmutableFormspecProcessor>) {
    resigned_with_method(response, "urn:formspec:sig-method:ed25519-cose-sign1@1")
}
fn resigned_with_method(
    mut response: Value,
    method: &str,
) -> (Value, FormspecBinding<ImmutableFormspecProcessor>) {
    use integrity_signature::{VerificationResult, Verifier, VerifyRequest};
    use ring::signature::KeyPair;
    let pkcs8 =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let payload = integrity_canonical::build_signed_payload(
        &response,
        integrity_canonical::DigestAlgorithm::Sha256,
    )
    .unwrap();
    let protected =
        integrity_cose::detached_signature_protected_header(-8, b"genuine-rust-test-key", method);
    let signed = key.sign(&integrity_cose::sig_structure_bytes(
        &protected,
        &payload.canonical_bytes,
    ));
    let cose = integrity_cose::encode_cose_sign1(&protected, None, signed.as_ref());
    let expected = KeyRef::RawPublicKey(key.public_key().as_ref().to_vec());
    let registry: MethodRegistry = serde_json::from_value(fixture("registry.json")).unwrap();
    let prefix = if method.starts_with("urn:formspec:receipt-method:") {
        "urn:formspec:receipt-method:"
    } else {
        "urn:formspec:sig-method:"
    };
    let receipt = RingVerifier::new_for_method_prefix(prefix)
        .verify(
            &VerifyRequest {
                signed_bytes: payload.canonical_bytes,
                signature_bytes: cose.clone(),
                method_uri: method.into(),
                key_ref: expected.clone(),
            },
            &registry,
        )
        .unwrap();
    assert_eq!(
        receipt.result,
        VerificationResult::Verified,
        "negative semantics still has genuine crypto"
    );
    for signature in response["authoredSignatures"].as_array_mut().unwrap() {
        signature["signedPayload"]["digest"] = payload.digest.clone().into();
        signature["signatureValue"] = STANDARD.encode(&cose).into();
    }
    let signer = response["extensions"]["x-formspec-document-attestations"]["signerId"]
        .as_str()
        .unwrap()
        .to_string();
    let adapter = FormspecBinding::new(processor()).with_document_verification(
        DocumentSignatureVerification::new(
            RingVerifier::new_for_method_prefix("urn:formspec:sig-method:"),
            registry,
            BTreeMap::from([(signer, expected)]),
        ),
    );
    (response, adapter)
}
#[test]
fn native_valid_source_with_wrong_key_or_altered_primitive_refuses() {
    let source = fixture("opaque-principal.response.json");
    let (other, wrong_key) = resigned(source.clone());
    assert!(
        wrong_key
            .signature_evidence(&task("applicant"), &other)
            .is_ok()
    );
    let failed = wrong_key
        .signature_evidence(&task("applicant"), &source)
        .unwrap()
        .unwrap();
    assert_eq!(
        failed[0].admission_failure.as_ref().unwrap().reason,
        wos_runtime::SignatureAdmissionFailureReason::PrimitiveVerificationFailed
    );
    assert!(matches!(
        failed[0].primitive_verification,
        wos_runtime::SignaturePrimitiveStatus::Failed { .. }
    ));
    assert!(
        wrong_key
            .retained_document_signature_evidence(&source, "wos-task:test:1:terms")
            .is_err()
    );
    let mut altered = source;
    altered["extensions"]["x-other"] = serde_json::json!({"changedButValid":true});
    let payload = integrity_canonical::build_signed_payload(
        &altered,
        integrity_canonical::DigestAlgorithm::Sha256,
    )
    .unwrap();
    for signature in altered["authoredSignatures"].as_array_mut().unwrap() {
        signature["signedPayload"]["digest"] = payload.digest.clone().into();
    }
    assert!(processor().validate_envelope(&altered).unwrap().is_empty());
    assert!(
        parse_authored_signatures(&altered).is_ok(),
        "canonical pins match; original COSE is now false"
    );
    let failed = binding()
        .signature_evidence(&task("applicant"), &altered)
        .unwrap()
        .unwrap();
    assert_eq!(
        failed[0].admission_failure.as_ref().unwrap().reason,
        wos_runtime::SignatureAdmissionFailureReason::PrimitiveVerificationFailed
    );
}
#[test]
fn genuinely_signed_authority_shape_refuses_unknown_classes_and_fields() {
    for authority in [
        serde_json::json!({"class":"nonsense"}),
        serde_json::json!({"class":"self","unknown":true}),
        serde_json::json!({"class":"witness"}),
    ] {
        let mut source = fixture("opaque-principal.response.json");
        source["extensions"]["x-wos-signer-authority"] = authority;
        let (source, adapter) = resigned(source);
        assert!(processor().validate_envelope(&source).unwrap().is_empty());
        assert!(
            adapter
                .signature_evidence(&task("applicant"), &source)
                .is_err()
        );
    }
}
#[test]
fn registered_intent_authority_floor_refusal_retains_truthful_failure() {
    let mut source = fixture("opaque-witness.response.json");
    source["extensions"]["x-wos-signer-authority"] = serde_json::json!({"class":"self"});
    let (source, adapter) = resigned(source);
    let (mut runtime, store) = runtime_with_binding(adapter);
    assert!(matches!(
        submit_value(&mut runtime, "witnessTerms", source, "witness").unwrap(),
        TaskSubmissionResult::Failed { .. }
    ));
    let retained = store.load_record(PROCESS).unwrap();
    assert!(retained.artifacts.is_empty());
    assert!(affirmations(&store).is_empty());
    assert!(
        retained
            .provenance_log
            .iter()
            .any(|item| item.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
    );
    assert_eq!(retained.process.active_tasks.len(), 3);
}
#[test]
fn corrupted_or_duplicate_principal_artifact_links_cannot_be_witnessed() {
    for corruption in [
        "missing",
        "duplicate",
        "swapped",
        "actor",
        "uncommitted",
        "duplicatePrincipal",
        "intent",
        "signedAt",
        "docAlgorithm",
        "payloadAlgorithm",
        "provider",
        "ceremony",
        "identity",
        "authority",
    ] {
        let (mut runtime, store) = runtime();
        submit(
            &mut runtime,
            "principalTerms",
            "opaque-principal.response.json",
            "applicant",
        )
        .unwrap();
        submit(
            &mut runtime,
            "principalPrivacy",
            "opaque-principal.response.json",
            "applicant",
        )
        .unwrap();
        let mut retained = store.load_record(PROCESS).unwrap();
        let principal = affirmations(&store)[0].clone();
        let link_index = retained
            .provenance_log
            .iter()
            .position(|item| {
                item.data
                    .as_ref()
                    .is_some_and(|d| d["details"]["signatureAffirmationId"] == principal.id)
            })
            .unwrap();
        match corruption {
            "missing" => {
                retained.provenance_log[link_index].data.as_mut().unwrap()["details"]
                    .as_object_mut()
                    .unwrap()
                    .remove("signatureAffirmationId");
            }
            "duplicate" => {
                let mut duplicate = retained.provenance_log[link_index].clone();
                duplicate.id = wos_core::ProvenanceRecord::task_lifecycle(
                    wos_core::ProvenanceKind::TaskCompleted,
                    "unused",
                    Some("applicant"),
                    None,
                )
                .id;
                retained.provenance_log.push(duplicate);
            }
            "swapped" => {
                retained.provenance_log[link_index].data.as_mut().unwrap()["details"]["artifactId"] =
                    affirmations(&store)[1].data.as_ref().unwrap()["sourceResponseRef"].clone();
            }
            "actor" => {
                let locator = principal.data.as_ref().unwrap()["sourceResponseRef"]
                    .as_str()
                    .unwrap();
                retained.artifacts.get_mut(locator).unwrap().actor_id = "witness".into();
            }
            "uncommitted" => {
                let locator = principal.data.as_ref().unwrap()["sourceResponseRef"]
                    .as_str()
                    .unwrap();
                retained.artifacts.get_mut(locator).unwrap().response["extensions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("x-formspec-document-attestations");
            }
            "duplicatePrincipal" => {
                let mut duplicate = principal.clone();
                duplicate.id = wos_core::ProvenanceRecord::task_lifecycle(
                    wos_core::ProvenanceKind::TaskCompleted,
                    "unused",
                    Some("applicant"),
                    None,
                )
                .id;
                let mut link = retained.provenance_log[link_index].clone();
                link.data.as_mut().unwrap()["details"]["signatureAffirmationId"] =
                    duplicate.id.clone().into();
                link.id = wos_core::ProvenanceRecord::task_lifecycle(
                    wos_core::ProvenanceKind::TaskCompleted,
                    "unused",
                    Some("applicant"),
                    None,
                )
                .id;
                retained.provenance_log.push(duplicate);
                retained.provenance_log.push(link);
            }
            "intent" | "signedAt" | "docAlgorithm" | "payloadAlgorithm" | "provider"
            | "ceremony" | "identity" | "authority" => {
                let data = retained
                    .provenance_log
                    .iter_mut()
                    .find(|item| item.id == principal.id)
                    .unwrap()
                    .data
                    .as_mut()
                    .unwrap();
                match corruption {
                    "intent" => {
                        data["signingIntent"] = "urn:wos:signing-intent:counter-signature".into()
                    }
                    "signedAt" => data["signedAt"] = "2026-10-20T19:03:00Z".into(),
                    "docAlgorithm" => data["documentHashAlgorithm"] = "sha-512".into(),
                    "payloadAlgorithm" => data["signedPayloadDigestAlgorithm"] = "sha-512".into(),
                    "provider" => data["signatureProvider"] = "urn:test:other-provider".into(),
                    "ceremony" => data["ceremonyId"] = "other-ceremony".into(),
                    "identity" => {
                        data["identityBinding"]["providerRef"] = "urn:test:other-identity".into()
                    }
                    "authority" => data["signerAuthority"] = fixture(
                        "opaque-witness.response.json",
                    )["extensions"]["x-wos-signer-authority"]
                        .clone(),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        retained.process.provenance_position = retained.provenance_log.len() as u64;
        let mut restored = wos_runtime::InMemoryStore::new();
        restored.restore_record(retained).unwrap();
        *store.0.lock().unwrap() = restored;
        let before = store.load_record(PROCESS).unwrap();
        let error = submit(
            &mut runtime,
            "witnessTerms",
            "opaque-witness.response.json",
            "witness",
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("witness principal")
                || (corruption == "uncommitted" && error.to_string().contains("typed commitment")),
            "wrong refusal for {corruption}: {error}"
        );
        let after = store.load_record(PROCESS).unwrap();
        assert_eq!(after.artifacts.len(), before.artifacts.len());
        assert_eq!(
            after.process.active_tasks.len(),
            before.process.active_tasks.len()
        );
    }
}
#[test]
fn genuine_ambiguous_locales_are_refused_instead_of_picked() {
    let mut source = fixture("opaque-principal.response.json");
    let mut resolved =
        fixture("definition.json")["documents"]["terms-v2"]["variants"]["en-US"].clone();
    resolved["documentId"] = "terms-v2".into();
    resolved["locale"] = "en-US".into();
    resolved["pinnedAt"] = "2026-10-20T19:00:00Z".into();
    let hash =
        integrity_cbor::domain_separated_sha256("trellis-presentation-artifact-v1", b"Hello")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
    resolved["presentationHash"] = hash.clone().into();
    source["resolvedDocuments"]
        .as_array_mut()
        .unwrap()
        .push(resolved);
    let selection = serde_json::json!({"signatureId":"sig-3","documentRef":{"documentId":"terms-v2","locale":"en-US"},"witnessedSignatureRef":null});
    source["extensions"]["x-formspec-document-attestations"]["attestations"]
        .as_array_mut()
        .unwrap()
        .push(selection);
    let mut extra = source["authoredSignatures"][0].clone();
    extra["signatureId"] = "sig-3".into();
    extra["documentRef"]["locale"] = "en-US".into();
    extra["documentHash"] =
        fixture("definition.json")["documents"]["terms-v2"]["variants"]["en-US"]["documentHash"]
            .clone();
    extra["presentationHash"] = hash.into();
    source["authoredSignatures"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    let (source, adapter) = resigned(source);
    assert!(processor().validate_envelope(&source).unwrap().is_empty());
    assert_eq!(
        adapter
            .signature_evidence(&task("applicant"), &source)
            .unwrap()
            .unwrap()
            .len(),
        3
    );
    let (mut runtime, store) = runtime_with_binding(adapter);
    assert!(submit_value(&mut runtime, "principalTerms", source, "applicant").is_err());
    assert!(affirmations(&store).is_empty());
    assert!(store.load_record(PROCESS).unwrap().artifacts.is_empty());
}

#[test]
fn genuine_receipt_domain_signature_cannot_replace_source_signature() {
    let (source, adapter) = resigned_with_method(
        fixture("opaque-principal.response.json"),
        "urn:formspec:receipt-method:ed25519-cose-sign1@1",
    );
    assert!(processor().validate_envelope(&source).unwrap().is_empty());
    assert!(
        adapter
            .signature_evidence(&task("applicant"), &source)
            .is_err()
    );
}
#[test]
fn accepted_principal_in_another_process_is_not_a_local_witness_target() {
    let (mut runtime, store) = runtime();
    let other = "default_process_01hw7rm71vfay8vvw14d2pf2dc";
    runtime
        .create_process_bound_to_case(
            wos_runtime::CreateProcessRequest {
                process_id: other.into(),
                tenant: None,
                definition_url: "urn:test:signature-runtime".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            },
            CASE.into(),
        )
        .unwrap();
    let principal_task = runtime
        .load_process(other)
        .unwrap()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "principalTerms")
        .unwrap();
    assert!(matches!(
        runtime
            .submit_task_response(
                &principal_task.task_id,
                fixture("opaque-principal.response.json"),
                "applicant",
                Some("foreign-principal")
            )
            .unwrap(),
        TaskSubmissionResult::Completed { .. }
    ));
    assert!(
        store
            .load_record(other)
            .unwrap()
            .provenance_log
            .iter()
            .any(|item| item.record_kind == wos_core::ProvenanceKind::SignatureAffirmation)
    );
    assert!(
        submit(
            &mut runtime,
            "witnessTerms",
            "opaque-witness.response.json",
            "witness"
        )
        .is_err()
    );
    assert!(affirmations(&store).is_empty());
}

#[test]
fn signed_legacy_signer_association_is_not_bypassed_by_new_claims() {
    let mut source = fixture("opaque-principal.response.json");
    source["data"]["signerId"] = "different-signer".into();
    let (source, adapter) = resigned(source);
    assert!(processor().validate_envelope(&source).unwrap().is_empty());
    let error = adapter
        .signature_evidence(&task("applicant"), &source)
        .unwrap_err();
    assert!(error.to_string().contains("signerId must match"));
}

#[test]
fn a_transplanted_complete_foreign_principal_triple_is_refused() {
    let (mut runtime, store) = runtime();
    let other = "default_process_01hw7rm71vfay8vvw14d2pf2dc";
    runtime
        .create_process_bound_to_case(
            wos_runtime::CreateProcessRequest {
                process_id: other.into(),
                tenant: None,
                definition_url: "urn:test:signature-runtime".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: None,
            },
            CASE.into(),
        )
        .unwrap();
    let task = runtime
        .load_process(other)
        .unwrap()
        .active_tasks
        .into_iter()
        .find(|task| task.task_ref == "principalTerms")
        .unwrap();
    runtime
        .submit_task_response(
            &task.task_id,
            fixture("opaque-principal.response.json"),
            "applicant",
            Some("foreign-principal"),
        )
        .unwrap();
    let foreign = store.load_record(other).unwrap();
    let mut current = store.load_record(PROCESS).unwrap();
    current.artifacts.extend(foreign.artifacts);
    current
        .provenance_log
        .extend(foreign.provenance_log.into_iter().filter(|item| {
            matches!(
                item.record_kind,
                wos_core::ProvenanceKind::SignatureAffirmation
                    | wos_core::ProvenanceKind::TaskCompleted
            )
        }));
    current.process.provenance_position = current.provenance_log.len() as u64;
    let mut restored = wos_runtime::InMemoryStore::new();
    restored.restore_record(current).unwrap();
    *store.0.lock().unwrap() = restored;
    let error = submit(
        &mut runtime,
        "witnessTerms",
        "opaque-witness.response.json",
        "witness",
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("witness principal"),
        "wrong refusal: {error}"
    );
    assert_eq!(store.load_record(PROCESS).unwrap().artifacts.len(), 1);
}

#[test]
fn reached_crypto_failure_is_a_durable_replayable_admission_failure() {
    let original = fixture("opaque-principal.response.json");
    let (_, wrong_key) = resigned(original.clone());
    let mut altered = original.clone();
    altered["extensions"]["x-other"] = serde_json::json!({"changedButValid":true});
    let digest = integrity_canonical::build_signed_payload(
        &altered,
        integrity_canonical::DigestAlgorithm::Sha256,
    )
    .unwrap()
    .digest;
    for signature in altered["authoredSignatures"].as_array_mut().unwrap() {
        signature["signedPayload"]["digest"] = digest.clone().into();
    }
    let mut unsupported_registry: MethodRegistry =
        serde_json::from_value(fixture("registry.json")).unwrap();
    unsupported_registry.entries[0].status = "deprecated".into();
    for (source, adapter, expected_reason) in [
        (original.clone(), wrong_key, "primitive_verification_failed"),
        (altered, binding(), "primitive_verification_failed"),
        (
            original,
            binding_with_registry(unsupported_registry),
            "method_unsupported",
        ),
    ] {
        let (mut runtime, store) = runtime_with_binding(adapter);
        let before = store.load_record(PROCESS).unwrap();
        let first =
            submit_value(&mut runtime, "principalTerms", source.clone(), "applicant").unwrap();
        assert!(
            matches!(first,TaskSubmissionResult::Failed {ref code,..} if code=="signatureAdmissionFailed")
        );
        let failed = store.load_record(PROCESS).unwrap();
        assert!(failed.artifacts.is_empty());
        assert!(affirmations(&store).is_empty());
        assert_eq!(
            failed.process.active_tasks.len(),
            before.process.active_tasks.len()
        );
        assert_eq!(failed.process.case_state, before.process.case_state);
        assert_eq!(
            failed.process.extensions.get("x-wos-signature-completions"),
            before.process.extensions.get("x-wos-signature-completions")
        );
        let decisions: Vec<_> = failed
            .provenance_log
            .iter()
            .filter(|item| item.record_kind == wos_core::ProvenanceKind::SignatureAdmissionFailed)
            .collect();
        assert_eq!(decisions.len(), 1);
        let data = decisions[0].data.as_ref().unwrap();
        assert_eq!(data["reason"], expected_reason);
        assert_eq!(data["evidenceBindings"]["responseId"], source["id"]);
        assert_eq!(data["evidenceBindings"]["signatureId"], "sig-1");
        assert_eq!(
            data["evidenceBindings"]["signedPayloadDigest"],
            source["authoredSignatures"][0]["signedPayload"]["digest"]
        );
        assert_eq!(
            failed
                .provenance_log
                .iter()
                .filter(|item| item.record_kind == wos_core::ProvenanceKind::TaskFailed)
                .count(),
            1
        );
        let replay = submit_value(&mut runtime, "principalTerms", source, "applicant").unwrap();
        assert_eq!(
            serde_json::to_value(replay).unwrap(),
            serde_json::to_value(first).unwrap()
        );
        let after = store.load_record(PROCESS).unwrap();
        assert_eq!(after.provenance_log.len(), failed.provenance_log.len());
        assert_eq!(after.artifacts.len(), failed.artifacts.len());
    }
}
