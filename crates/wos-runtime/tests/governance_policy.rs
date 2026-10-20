// Rust guideline compliant 2026-02-21

//! Actual runtime controls for attributable determination evidence before effects.

use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use wos_core::instance::HumanActorBinding;
use wos_core::traits::{DefaultRuntime, DocumentResolver, ExternalService};
use wos_core::{GovernanceDocument, KernelDocument, ProvenanceKind};
use wos_runtime::governance::{GovernancePolicySelection, QualifiedGovernancePolicy};
use wos_runtime::store::RuntimeStore;
use wos_runtime::{
    Clock, CreateProcessRequest, RuntimeError, SharedInMemoryStore, WosRuntime,
    restate_signature_fixture_bindings,
};

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
struct FixedClock;
impl Clock for FixedClock {
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

fn kernel() -> Value {
    json!({
        "$wosWorkflow":"1.0","url":"urn:test:governance-real-drain","version":"1.0.0",
        "impactLevel":"operational","actors":[{"id":"reviewer","type":"human"}],
        "governance":{"audit":{"reasoningTier":{"requiredForTags":["determination"]},
            "counterfactualTier":{"requiredForTags":["adverse-decision"]}}},
        "contracts":{"review":{"binding":"formspec","ref":"urn:test:review","definitionVersion":"1.0.0"}},
        "lifecycle":{"initialState":"review","states":{
            "review":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"review",
                "assignTo":"reviewer","contractRef":"review","completionEvent":"review.completed"}],
                "transitions":[{"event":{"kind":"message","name":"review.completed"},"target":"decided",
                    "tags":["determination","adverse-decision"],
                    "actions":[{"action":"invokeService","serviceRef":"effect"}]}]},
            "decided":{"type":"final"}
        }}
    })
}
fn rule() -> Value {
    json!({"ruleId":"demo-photo-review","description":"Demo photo criterion",
    "sourceAuthority":"policy","citation":"Demo policy selection; not law"})
}
fn evidence() -> Value {
    json!({
        "reason":"The submitted image obscures this applicant's face.",
        "rulesApplied":[rule()],
        "evidenceConsulted":[{"kind":"caseField","caseFieldPath":"/photo","summary":"Actual retained image"}],
        "criteriaChecked":[{"criterion":"photoClear","satisfied":false,"detail":"This accepted image obscures the face"}],
        "positiveCounterfactuals":[{"polarity":"positive","factor":"photoClear","effect":"A clear replacement photo permits reconsideration of this case."}],
        "negativeCounterfactuals":[{"polarity":"negative","factor":"gender","effect":"This reviewer did not use gender in evaluating the submitted photo."}]
    })
}
fn selected(raw: Value) -> GovernancePolicySelection {
    serde_json::from_value(json!({"version":1,"kernel":raw,
        "delivery":{"$wosDelivery":"1.0","targetWorkflow":"urn:test:governance-real-drain"},
        "humanDecisions":[{"transition":{"from":"review","to":"decided","event":"review.completed"},
            "taskRef":"review","evidencePointer":"/data/governance","rules":[rule()],
            "criteria":["photoClear"],"caseFieldPaths":["/photo"],
            "positiveFactors":["photoClear"],"negativeFactors":["gender"]}]}))
    .unwrap()
}
fn rig(
    policy: bool,
    decision: Value,
) -> (WosRuntime, SharedInMemoryStore, String, Arc<AtomicUsize>) {
    let raw = kernel();
    let selected =
        policy.then(|| QualifiedGovernancePolicy::qualify(selected(raw.clone())).unwrap());
    rig_selected(raw, selected, decision)
}

fn rig_created(
    raw: Value,
    policy: Option<QualifiedGovernancePolicy>,
) -> (WosRuntime, SharedInMemoryStore, String, Arc<AtomicUsize>) {
    let document: KernelDocument = serde_json::from_value(raw.clone()).unwrap();
    let store = SharedInMemoryStore::default();
    let effects = Arc::new(AtomicUsize::new(0));
    let mut runtime = WosRuntime::new(
        store.clone(),
        Resolver(document),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(effects.clone()),
        DefaultRuntime::new(),
        FixedClock,
        restate_signature_fixture_bindings(),
    );
    if let Some(policy) = policy {
        runtime = runtime
            .with_governance_policy(policy.clone(), Default::default())
            .unwrap();
    }
    let actor_bindings = raw["actors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|actor| {
            let declared = actor["id"].as_str().unwrap();
            let assigned = match declared {
                "reviewer" => "actor:human:reviewer-1",
                "appealReviewer" => "actor:human:appeal-2",
                _ => "actor:human:applicant-3",
            };
            HumanActorBinding {
                declared_actor: declared.into(),
                assigned_actor: assigned.into(),
            }
        })
        .collect();
    let process = runtime
        .create_process_with_actor_bindings(
            CreateProcessRequest {
                process_id: stack_common_typeid::testing::fixture_minter().mint_process_id(),
                tenant: None,
                definition_url: "urn:test:governance-real-drain".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: Some(json!({"photo":"retained-original-image"})),
            },
            actor_bindings,
        )
        .unwrap();
    (runtime, store, process.process_id, effects)
}

fn rig_selected(
    raw: Value,
    policy: Option<QualifiedGovernancePolicy>,
    decision: Value,
) -> (WosRuntime, SharedInMemoryStore, String, Arc<AtomicUsize>) {
    let (mut runtime, store, id, effects) = rig_created(raw, policy);
    let process = runtime.load_process(&id).unwrap();
    let task = &process.active_tasks[0].task_id;
    runtime
        .submit_task_response(
            task,
            json!({"status":"completed","definitionUrl":"urn:test:review","definitionVersion":"1.0.0",
        "data":{"approved":true,"governance":decision}}),
            "actor:human:reviewer-1",
            Some("original-review"),
        )
        .unwrap();
    (runtime, store, process.process_id, effects)
}

#[test]
fn actual_drain_emits_dedicated_tiers_linked_to_the_real_facts_before_one_effect() {
    let (mut runtime, store, id, effects) = rig(true, evidence());
    let outcome = runtime.drain_once(&id).unwrap();
    let facts = outcome
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::StateTransition)
        .unwrap();
    let reason = outcome
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::ReasoningTierRecorded)
        .unwrap();
    let cf = outcome
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::CounterfactualTierRecorded)
        .unwrap();
    assert_eq!(reason.audit_layer.as_deref(), Some("reasoning"));
    assert_eq!(cf.audit_layer.as_deref(), Some("counterfactual"));
    assert_eq!(reason.data.as_ref().unwrap()["factsRecordRef"], facts.id);
    assert_eq!(
        cf.data.as_ref().unwrap()["snapshotSha256"],
        facts.case_file_snapshot.as_ref().unwrap().sha256
    );
    let assembled = wos_core::explain::assemble_explanation(
        &outcome.provenance,
        &facts.id,
        &facts.transition_tags,
        &facts.timestamp,
    );
    assert_eq!(assembled.reasoning.len(), 1);
    assert_eq!(assembled.positive_counterfactual.len(), 1);
    assert_eq!(assembled.negative_counterfactual.len(), 1);
    for record in [reason, cf] {
        let event = wos_events::NativeProvenanceEvent::from_record(record).unwrap();
        assert_eq!(event.record(), &serde_json::to_value(record).unwrap());
    }
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    let retained = store.load_record(&id).unwrap();
    // The same non-default scope survives creation, accepted task, later drain
    // and human evidence records; no deployment/default minter can replace it.
    let tenant = stack_common_typeid::Tenant::of(&id).unwrap();
    assert_eq!(tenant.as_str(), "acme");
    assert_eq!(retained.process.tenant, tenant.as_str());
    assert_eq!(
        stack_common_typeid::Tenant::of(&retained.process.case_ledger_id),
        Some(tenant.clone())
    );
    assert!(!retained.provenance_log.is_empty());
    for record in &retained.provenance_log {
        assert_eq!(
            stack_common_typeid::Tenant::of(&record.id),
            Some(tenant.clone()),
            "{}",
            record.id
        );
    }
    assert!(retained.process.pending_events.is_empty());
    assert_eq!(runtime.drain_once(&id).unwrap().processed_event, None);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
}

#[test]
fn default_policy_cannot_silently_accept_a_required_determination() {
    let (mut runtime, store, id, effects) = rig(false, evidence());
    let before = serde_json::to_value(store.load_record(&id).unwrap().process).unwrap();
    assert!(matches!(
        runtime.drain_once(&id),
        Err(RuntimeError::GovernancePolicy(_))
    ));
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    assert_eq!(
        serde_json::to_value(store.load_record(&id).unwrap().process).unwrap(),
        before
    );
}

#[test]
fn missing_invalid_foreign_or_unattributable_evidence_refuses_without_effects() {
    for label in [
        "missing",
        "empty-counterfactual",
        "foreign-rule",
        "missing-field",
        "wrong-polarity",
        "unknown-rule-field",
        "wrong-actor",
        "missing-completion",
    ] {
        let mut data = evidence();
        match label {
            "missing" => data = Value::Null,
            "empty-counterfactual" => data["negativeCounterfactuals"] = json!([]),
            "foreign-rule" => data["rulesApplied"][0]["ruleId"] = json!("foreign"),
            "missing-field" => data["evidenceConsulted"][0]["caseFieldPath"] = json!("/absent"),
            "wrong-polarity" => {
                data["positiveCounterfactuals"][0]["polarity"] = json!("unexpected")
            }
            "unknown-rule-field" => data["rulesApplied"][0]["ignored"] = json!(true),
            _ => {}
        }
        let (mut runtime, mut store, id, effects) = rig(true, data);
        let mut prior = store.load_record(&id).unwrap();
        if label == "wrong-actor" {
            prior.process.pending_events[0].actor_id = Some("actor:human:other".into());
        }
        if label == "missing-completion" {
            prior
                .provenance_log
                .retain(|record| record.record_kind != ProvenanceKind::TaskCompleted);
        }
        store.save_record(prior).unwrap();
        let before = serde_json::to_value(store.load_record(&id).unwrap().process).unwrap();
        assert!(
            matches!(
                runtime.drain_once(&id),
                Err(RuntimeError::GovernancePolicy(_))
            ),
            "{label}"
        );
        assert_eq!(effects.load(Ordering::SeqCst), 0, "{label}");
        assert_eq!(
            serde_json::to_value(store.load_record(&id).unwrap().process).unwrap(),
            before,
            "{label}"
        );
    }
}

#[test]
fn selection_refuses_incomplete_coverage_foreign_delivery_and_unimplemented_due_process() {
    for label in [
        "missing-binding",
        "extra-binding",
        "foreign-delivery",
        "rights-impacting",
    ] {
        let mut selection = selected(kernel());
        match label {
            "missing-binding" => selection.human_decisions.clear(),
            "extra-binding" => selection
                .human_decisions
                .push(selection.human_decisions[0].clone()),
            "foreign-delivery" => selection.delivery["targetWorkflow"] = json!("urn:foreign"),
            "rights-impacting" => selection.kernel["impactLevel"] = json!("rights-impacting"),
            _ => unreachable!(),
        }
        assert!(
            QualifiedGovernancePolicy::qualify(selection).is_err(),
            "{label}"
        );
    }
}

#[test]
fn actual_initial_determination_refuses_before_entry_effects_or_creation() {
    // ForEach completion really fires an anonymous transition during initialization.
    // A plain Atomic initial state does not fire a transition on creation.
    let raw = json!({"$wosWorkflow":"1.0","url":"urn:test:initial-governance","version":"1.0.0",
        "lifecycle":{"initialState":"first","states":{
            "first":{"type":"foreach","collection":"[]","body":{"type":"atomic"},
                "transitions":[{"target":"done","tags":["determination","adverse-decision"],
                    "actions":[{"action":"invokeService","serviceRef":"effect"}]}]},
            "done":{"type":"final"}}}});
    let store = SharedInMemoryStore::default();
    let effects = Arc::new(AtomicUsize::new(0));
    let mut runtime = WosRuntime::new(
        store.clone(),
        Resolver(serde_json::from_value(raw).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(effects.clone()),
        DefaultRuntime::new(),
        FixedClock,
        restate_signature_fixture_bindings(),
    );
    let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    let result = runtime.create_process(CreateProcessRequest {
        process_id: id.clone(),
        tenant: None,
        definition_url: "urn:test:initial-governance".into(),
        definition_version: "1.0.0".into(),
        initial_case_state: Some(json!({})),
    });
    assert!(
        matches!(result, Err(RuntimeError::GovernancePolicy(_))),
        "{result:?}"
    );
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    assert!(store.load_record(&id).is_err());
}

fn due_selection() -> GovernancePolicySelection {
    let mut raw = kernel();
    raw["impactLevel"] = json!("rights-impacting");
    raw["actors"] = json!([{"id":"reviewer","type":"human"},{"id":"appealReviewer","type":"human"},{"id":"applicant","type":"human"}]);
    raw["governance"]["dueProcess"] = json!({"adverseDecisionPolicy":{"noticeRequired":true,
        "noticeTiming":"beforeEffective","explanationLevel":"individualized","noticeTemplateKey":"denial",
        "counterfactualRequired":true,"appealMechanism":{"enabled":true,"appealWindow":"P30D",
            "reviewerRoles":["appealReviewer"],"reviewerConstraint":"independentFromOriginal",
            "continuationOfServices":false}}});
    raw["lifecycle"]["states"]["decided"] = json!({"type":"atomic","transitions":[{
        "event":{"kind":"message","name":"denial.noticeSent"},"target":"closed",
        "actions":[{"action":"setData","path":"caseFile.caseClosed","value":true},
            {"action":"invokeService","serviceRef":"effect"}]}]});
    raw["lifecycle"]["states"]["closed"] = json!({"type":"atomic","onEntry":[{
        "action":"createTask","taskRef":"appealRequest","assignTo":"applicant","contractRef":"review","completionEvent":"appeal.filed"}],
        "transitions":[{"event":{"kind":"message","name":"appeal.filed"},"target":"appealReview"}]});
    raw["lifecycle"]["states"]["appealReview"] = json!({"type":"atomic","onEntry":[{
        "action":"createTask","taskRef":"appealReview","assignTo":"appealReviewer","contractRef":"review","completionEvent":"appeal.reviewed"}],
        "transitions":[{"event":{"kind":"message","name":"appeal.reviewed"},"target":"reopened",
            "tags":["determination"],"actions":[{"action":"setData","path":"caseFile.caseClosed","value":false}]}]});
    raw["lifecycle"]["states"]["reopened"] = json!({"type":"final"});
    let mut selection = serde_json::to_value(selected(raw)).unwrap();
    let mut review = selection["humanDecisions"][0].clone();
    review["transition"] = json!({"from":"appealReview","to":"reopened","event":"appeal.reviewed"});
    review["taskRef"] = json!("appealReview");
    selection["humanDecisions"]
        .as_array_mut()
        .unwrap()
        .push(review);
    selection["delivery"]["$wosDelivery"] = json!("1.0");
    selection["delivery"]["notifications"] = json!({"templates":{"denial":{"category":"adverse-decision",
        "subject":"Individualized demo decision for {{caseId}}","sections":[
            {"id":"determination","contentType":"text","content":"{{determination}}"},
            {"id":"reasons","contentType":"text","content":"{{reason}}\n{{positiveCounterfactuals}}\n{{negativeCounterfactuals}}"},
            {"id":"appealRights","contentType":"appeal-rights","content":"Administrative reconsideration: {{appealWindow}}. This demo policy does not create a statutory hearing."},
            {"id":"appealInstructions","contentType":"action-required","content":"File through the applicant appeal task for {{appealBody}}."}]}}});
    selection["dueProcess"] = json!({"noticeBindingId":"sendDenialNotice",
        "effectiveTransitions":[{"from":"decided","to":"closed","event":"denial.noticeSent"}],
        "protectedEffects":[{"path":"/caseClosed","adverseValues":[true]}]});
    selection["appeal"] = json!({"policyLabel":"demo-administrative-reconsideration",
        "reviewBody":"the independent administrative reconsideration reviewer",
        "filingInstructions":"File through the applicant appeal task.","filedTransition":{"from":"closed","to":"appealReview","event":"appeal.filed"},
        "filedTaskRef":"appealRequest","proposalReferencePointer":"/data/proposalFactsRef",
        "reviewTaskRef":"appealReview","reviewerActor":"appealReviewer",
        "reviewTransitions":[{"from":"appealReview","to":"reopened","event":"appeal.reviewed"}]});
    serde_json::from_value(selection).unwrap()
}

fn timestamp(ms: u64) -> String {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}
fn receipt_for_proposal(
    store: &mut SharedInMemoryStore,
    id: &str,
    proposal: &wos_events::governance::AdverseNoticeProposal,
) -> wos_runtime::governance::CapturedGovernanceNotice {
    let mut prior = store.load_record(id).unwrap();
    let mut record = wos_core::ProvenanceRecord::blank(
        &stack_common_typeid::testing::fixture_minter(),
        ProvenanceKind::CallbackReceived,
    );
    record.timestamp = timestamp(1_800_000_000_000);
    record.actor_id = Some("actor:service:notice".into());
    let sent = timestamp(1_799_999_999_000);
    let date =
        stack_common_hash::canonical_json_hash(&json!({"controlledOwnerFixture":"original-clock"}))
            .unwrap();
    record.data = Some(json!({"bindingId":"sendDenialNotice","data":{
        "delivery":{"status":"accepted","intentId":"original-intent","attemptId":"original-attempt"},
        "clock":{"governedSentAt":sent,"dateDecisionSha256":date}}}));
    // Controlled projection of the existing notice owner, not a live receipt qualification claim.
    let receipt = wos_runtime::governance::CapturedGovernanceNotice {
        process_id: id.into(),
        case_ledger_id: prior.process.case_ledger_id.clone(),
        binding_id: "sendDenialNotice".into(),
        intent_id: "original-intent".into(),
        attempt_id: "original-attempt".into(),
        callback_record_ref: record.id.clone(),
        callback_data_sha256: stack_common_hash::canonical_json_hash(record.data.as_ref().unwrap())
            .unwrap(),
        date_decision_sha256: date,
        governed_sent_at: sent,
        human_readable: proposal.human_readable.clone(),
        proposal_content_sha256: Some(proposal.content_sha256.clone()),
    };
    prior.process.provenance_position += 1;
    prior.provenance_log.push(record);
    store.save_record(prior).unwrap();
    receipt
}
fn enqueue_receipt(runtime: &mut WosRuntime, id: &str) {
    runtime
        .enqueue_event(
            id,
            wos_core::instance::PendingEvent {
                event: "denial.noticeSent".into(),
                actor_id: Some("actor:service:notice".into()),
                data: None,
                timestamp: timestamp(1_800_000_000_000),
                origin: None,
                idempotency_token: None,
            },
        )
        .unwrap();
}
fn proposal(
    outcome: &wos_runtime::DrainOnceResult,
) -> wos_events::governance::AdverseNoticeProposal {
    outcome
        .provenance
        .iter()
        .find_map(|record| {
            if record.record_kind != ProvenanceKind::ReasoningTierRecorded {
                return None;
            }
            serde_json::from_value::<wos_events::governance::ReasoningEvidence>(
                record.data.clone()?,
            )
            .ok()?
            .adverse_notice
        })
        .unwrap()
}

#[test]
fn actual_adverse_proposal_is_deterministic_and_only_matching_retained_receipt_permits_effect() {
    let selection = due_selection();
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let (mut runtime, mut store, id, effects) =
        rig_selected(selection.kernel.clone(), Some(policy.clone()), evidence());
    let original = runtime.drain_once(&id).unwrap();
    let proposed = proposal(&original);
    assert!(proposed.human_readable.contains("submitted image obscures"));
    assert!(
        proposed
            .human_readable
            .contains("30 calendar days from qualifying original delivery")
    );
    assert_ne!(
        store.load_record(&id).unwrap().process.case_state["caseClosed"],
        true
    );
    for row in original
        .provenance
        .iter()
        .filter(|row| row.record_kind == ProvenanceKind::ReasoningTierRecorded)
    {
        wos_events::NativeProvenanceEvent::from_record(row).unwrap();
    }
    let receipt = receipt_for_proposal(&mut store, &id, &proposed);
    enqueue_receipt(&mut runtime, &id);
    let before = effects.load(Ordering::SeqCst);
    assert!(matches!(
        runtime.drain_once(&id),
        Err(RuntimeError::GovernancePolicy(_))
    ));
    assert_eq!(effects.load(Ordering::SeqCst), before);
    runtime = runtime
        .with_governance_policy(
            policy.clone(),
            wos_runtime::governance::CapturedGovernanceEvidence {
                decisions: vec![],
                notices: vec![receipt],
            },
        )
        .unwrap();
    runtime.drain_once(&id).unwrap();
    assert_eq!(
        store.load_record(&id).unwrap().process.case_state["caseClosed"],
        true
    );
    assert_eq!(effects.load(Ordering::SeqCst), before + 1);
    assert_eq!(runtime.drain_once(&id).unwrap().processed_event, None);
    assert_eq!(effects.load(Ordering::SeqCst), before + 1);
}

#[test]
fn mismatched_or_unretained_receipt_cannot_make_an_adverse_proposal_effective() {
    for label in [
        "case", "intent", "attempt", "body", "content", "clock", "callback", "binding", "future",
    ] {
        let selection = due_selection();
        let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
        let (mut runtime, mut store, id, effects) =
            rig_selected(selection.kernel.clone(), Some(policy.clone()), evidence());
        let proposed = proposal(&runtime.drain_once(&id).unwrap());
        let mut receipt = receipt_for_proposal(&mut store, &id, &proposed);
        match label {
            "case" => receipt.case_ledger_id = "another-case".into(),
            "intent" => receipt.intent_id = "another-intent".into(),
            "attempt" => receipt.attempt_id = "another-attempt".into(),
            "body" => receipt.human_readable = "changed".into(),
            "content" => receipt.proposal_content_sha256 = None,
            "clock" => receipt.date_decision_sha256 = "changed".into(),
            "callback" => receipt.callback_record_ref = "unretained".into(),
            "binding" => receipt.binding_id = "another-binding".into(),
            "future" => receipt.governed_sent_at = timestamp(1_800_000_000_001),
            _ => unreachable!(),
        }
        enqueue_receipt(&mut runtime, &id);
        let before = effects.load(Ordering::SeqCst);
        runtime = runtime
            .with_governance_policy(
                policy.clone(),
                wos_runtime::governance::CapturedGovernanceEvidence {
                    decisions: vec![],
                    notices: vec![receipt],
                },
            )
            .unwrap();
        assert!(
            matches!(
                runtime.drain_once(&id),
                Err(RuntimeError::GovernancePolicy(_))
            ),
            "{label}"
        );
        assert_eq!(effects.load(Ordering::SeqCst), before, "{label}");
        assert_ne!(
            store.load_record(&id).unwrap().process.case_state["caseClosed"],
            true,
            "{label}"
        );
    }
}

#[test]
fn actual_mapping_cannot_close_before_a_receipt_and_initial_adverse_values_refuse() {
    let mut selection = due_selection();
    selection.kernel["contracts"]["review"]["responseMappingRef"] =
        json!("urn:test:actual-response-mapping");
    let mut selected = serde_json::to_value(selection).unwrap();
    selected["dueProcess"]["protectedEffects"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"/decision","adverseValues":[true]}));
    let selection: GovernancePolicySelection = serde_json::from_value(selected).unwrap();
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let (mut runtime, store, id, effects) =
        rig_created(selection.kernel.clone(), Some(policy.clone()));
    let before = store.load_record(&id).unwrap();
    let task = before.process.active_tasks[0].task_id.clone();
    let result = runtime.submit_task_response(
        &task,
        json!({"status":"completed","definitionUrl":"urn:test:review",
        "definitionVersion":"1.0.0","data":{"approved":true,"governance":evidence()}}),
        "actor:human:reviewer-1",
        Some("mapping-bypass"),
    );
    assert!(
        matches!(result, Err(RuntimeError::GovernancePolicy(_))),
        "{result:?}"
    );
    let after = store.load_record(&id).unwrap();
    assert_eq!(
        serde_json::to_value(&after.process).unwrap(),
        serde_json::to_value(&before.process).unwrap()
    );
    assert_eq!(after.artifacts.len(), before.artifacts.len());
    assert_eq!(effects.load(Ordering::SeqCst), 0);

    let mut raw = selection.kernel.clone();
    raw["lifecycle"]["states"]["review"]["onEntry"]
        .as_array_mut()
        .unwrap()
        .push(json!({"action":"setData","path":"caseFile.caseClosed","value":true}));
    let mut selected = selection.clone();
    selected.kernel = raw.clone();
    let qualified = QualifiedGovernancePolicy::qualify(selected).unwrap();
    let store = SharedInMemoryStore::default();
    let effects = Arc::new(AtomicUsize::new(0));
    let mut runtime = WosRuntime::new(
        store.clone(),
        Resolver(serde_json::from_value(raw).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(effects.clone()),
        DefaultRuntime::new(),
        FixedClock,
        restate_signature_fixture_bindings(),
    )
    .with_governance_policy(qualified.clone(), Default::default())
    .unwrap();
    let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
    let result = runtime.create_process_with_actor_bindings(
        CreateProcessRequest {
            process_id: id.clone(),
            tenant: None,
            definition_url: "urn:test:governance-real-drain".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(json!({"photo":"original"})),
        },
        vec![
            HumanActorBinding {
                declared_actor: "reviewer".into(),
                assigned_actor: "actor:human:reviewer-1".into(),
            },
            HumanActorBinding {
                declared_actor: "appealReviewer".into(),
                assigned_actor: "actor:human:appeal-2".into(),
            },
            HumanActorBinding {
                declared_actor: "applicant".into(),
                assigned_actor: "actor:human:applicant-3".into(),
            },
        ],
    );
    assert!(
        matches!(result, Err(RuntimeError::GovernancePolicy(_))),
        "{result:?}"
    );
    assert!(store.load_record(&id).is_err());
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

fn closed_appeal_rig() -> (
    WosRuntime,
    SharedInMemoryStore,
    String,
    QualifiedGovernancePolicy,
    wos_events::governance::AdverseNoticeProposal,
) {
    closed_appeal_rig_with_period("P30D")
}
fn closed_appeal_rig_with_period(
    period: &str,
) -> (
    WosRuntime,
    SharedInMemoryStore,
    String,
    QualifiedGovernancePolicy,
    wos_events::governance::AdverseNoticeProposal,
) {
    let mut selection = due_selection();
    selection.kernel["governance"]["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"]["appealWindow"] =
        json!(period);
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let (mut runtime, mut store, id, _) =
        rig_selected(selection.kernel.clone(), Some(policy.clone()), evidence());
    let proposed = proposal(&runtime.drain_once(&id).unwrap());
    let receipt = receipt_for_proposal(&mut store, &id, &proposed);
    enqueue_receipt(&mut runtime, &id);
    runtime = runtime
        .with_governance_policy(
            policy.clone(),
            wos_runtime::governance::CapturedGovernanceEvidence {
                decisions: vec![],
                notices: vec![receipt],
            },
        )
        .unwrap();
    runtime.drain_once(&id).unwrap();
    (runtime, store, id, policy, proposed)
}
fn submit_appeal(
    runtime: &mut WosRuntime,
    store: &SharedInMemoryStore,
    id: &str,
    task_ref: &str,
    actor: &str,
    original: &str,
) {
    let record = store.load_record(id).unwrap();
    let task = record
        .process
        .active_tasks
        .iter()
        .find(|task| task.task_ref == task_ref)
        .unwrap();
    runtime.submit_task_response(&task.task_id,json!({"status":"completed","definitionUrl":"urn:test:review","definitionVersion":"1.0.0",
        "data":{"approved":true,"proposalFactsRef":original,"governance":evidence()}}),actor,Some(task_ref)).unwrap();
}

#[test]
fn actual_filed_appeal_preserves_original_notice_window_and_requires_independent_human_review() {
    let (mut runtime, store, id, _, proposed) = closed_appeal_rig();
    submit_appeal(
        &mut runtime,
        &store,
        &id,
        "appealRequest",
        "actor:human:applicant-3",
        &proposed.facts_record_ref,
    );
    let filed = runtime.drain_once(&id).unwrap();
    let evidence = filed
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::AppealFiled)
        .unwrap();
    assert_eq!(
        evidence.data.as_ref().unwrap()["originalProposalFactsRef"],
        proposed.facts_record_ref
    );
    assert_eq!(evidence.data.as_ref().unwrap()["windowRule"], "P30D");
    assert_eq!(
        evidence.data.as_ref().unwrap()["windowDeadline"],
        timestamp(1_799_999_999_000 + 30 * 24 * 60 * 60 * 1000)
    );
    submit_appeal(
        &mut runtime,
        &store,
        &id,
        "appealReview",
        "actor:human:appeal-2",
        &proposed.facts_record_ref,
    );
    let reviewed = runtime.drain_once(&id).unwrap();
    assert_eq!(
        reviewed
            .provenance
            .iter()
            .filter(|record| record.record_kind == ProvenanceKind::ReasoningTierRecorded)
            .count(),
        1
    );
    assert_eq!(
        store.load_record(&id).unwrap().process.case_state["caseClosed"],
        false
    );
    assert_eq!(runtime.drain_once(&id).unwrap().processed_event, None);
}

#[test]
fn appeal_wrong_original_expired_filing_or_original_reviewer_refuses_before_result_persistence() {
    for label in ["foreign-proposal", "expired", "original-reviewer"] {
        let (mut runtime, mut store, id, _, proposed) = closed_appeal_rig();
        let reference = if label == "foreign-proposal" {
            "foreign-facts"
        } else {
            &proposed.facts_record_ref
        };
        submit_appeal(
            &mut runtime,
            &store,
            &id,
            "appealRequest",
            "actor:human:applicant-3",
            reference,
        );
        if label == "expired" {
            let mut prior = store.load_record(&id).unwrap();
            let queue = prior.process.pending_events.first_mut().unwrap();
            let artifact = prior
                .artifacts
                .values_mut()
                .find(|artifact| {
                    artifact.kind == wos_runtime::store::TaskArtifactKind::Accepted
                        && artifact.actor_id == "actor:human:applicant-3"
                })
                .unwrap();
            artifact.recorded_at = timestamp(1_799_999_999_000 + 30 * 24 * 60 * 60 * 1000 + 1);
            queue.timestamp = artifact.recorded_at.clone();
            for record in &mut prior.provenance_log {
                if record.record_kind == ProvenanceKind::TaskCompleted
                    && record.actor_id.as_deref() == Some("actor:human:applicant-3")
                {
                    record.timestamp = artifact.recorded_at.clone();
                }
            }
            store.save_record(prior).unwrap();
        }
        if label != "original-reviewer" {
            assert!(
                matches!(
                    runtime.drain_once(&id),
                    Err(RuntimeError::GovernancePolicy(_))
                ),
                "{label}"
            );
        } else {
            runtime.drain_once(&id).unwrap();
            let mut prior = store.load_record(&id).unwrap();
            let task = prior
                .process
                .active_tasks
                .iter_mut()
                .find(|task| task.task_ref == "appealReview")
                .unwrap();
            task.assigned_actor = Some("actor:human:reviewer-1".into());
            if let Some(context) = task.context.as_mut() {
                context.assigned_actor = "actor:human:reviewer-1".into();
            }
            store.save_record(prior).unwrap();
            submit_appeal(
                &mut runtime,
                &store,
                &id,
                "appealReview",
                "actor:human:reviewer-1",
                &proposed.facts_record_ref,
            );
            assert!(
                matches!(
                    runtime.drain_once(&id),
                    Err(RuntimeError::GovernancePolicy(_))
                ),
                "{label}"
            );
        }
        assert_eq!(
            store.load_record(&id).unwrap().process.case_state["caseClosed"],
            true,
            "{label}"
        );
    }
}

#[derive(Clone)]
struct MovingClock(Arc<AtomicU64>);
impl Clock for MovingClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn timeout_rig() -> (
    WosRuntime,
    SharedInMemoryStore,
    String,
    Arc<AtomicU64>,
    Arc<AtomicUsize>,
) {
    let mut selection = due_selection();
    let original = 1_800_000_000_000_u64;
    selection.kernel["lifecycle"]["initialState"] = json!("awaitingPhoto");
    selection.kernel["lifecycle"]["states"]["awaitingPhoto"] = json!({"type":"atomic","onEntry":[
        {"action":"startTimer","timerId":"photoResponse","duration":"P90D",
            "event":{"kind":"timer","timerId":"photoResponse","source":"state"},"startAtExpression":"caseFile.requestSentAt"},
        {"action":"createTask","taskRef":"replacePhoto","assignTo":"applicant","contractRef":"review",
            "completionEvent":"photoCorrection.completed","expiresWithTimer":"photoResponse"}],
        "transitions":[{"event":{"kind":"message","name":"photoCorrection.completed"},"target":"review",
            "actions":[{"action":"cancelTimer","timerId":"photoResponse"}]},
            {"event":{"kind":"timer","timerId":"photoResponse","source":"state"},"target":"decided",
                "tags":["determination","adverse-decision"],"actions":[{"action":"invokeService","serviceRef":"effect"}]}]});
    let event = serde_json::from_value::<KernelDocument>(selection.kernel.clone())
        .unwrap()
        .lifecycle
        .states["awaitingPhoto"]
        .transitions[1]
        .event
        .as_ref()
        .unwrap()
        .runtime_dispatch_label();
    let mut selected = serde_json::to_value(selection).unwrap();
    selected["timeoutDecisions"] = json!([{"transition":{"from":"awaitingPhoto","to":"decided","event":event},
        "timerId":"photoResponse","responsePeriod":"P90D","requestNoticeBindingId":"sendPhotoRequest","correctionTaskRefs":["replacePhoto"],
        "rule":{"ruleId":"demo-original-ninety-day-photo-rule","description":"Explicit sample no-response rule; not law",
            "sourceAuthority":"policy","citation":"D1 selected demo policy P90D"},
        "reconsiderationRoute":"the retained late-photo task and authorized human reconsideration",
        "excludedFactors":["gender"]}]);
    let selection: GovernancePolicySelection = serde_json::from_value(selected).unwrap();
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let clock = Arc::new(AtomicU64::new(original));
    let effects = Arc::new(AtomicUsize::new(0));
    let store = SharedInMemoryStore::default();
    let mut runtime = WosRuntime::new(
        store.clone(),
        Resolver(serde_json::from_value(selection.kernel.clone()).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(effects.clone()),
        DefaultRuntime::new(),
        MovingClock(clock.clone()),
        restate_signature_fixture_bindings(),
    )
    .with_governance_policy(policy.clone(), Default::default())
    .unwrap();
    let process = runtime
        .create_process_with_actor_bindings(
            CreateProcessRequest {
                process_id: stack_common_typeid::testing::fixture_minter().mint_process_id(),
                tenant: None,
                definition_url: "urn:test:governance-real-drain".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: Some(
                    json!({"photo":"original","requestSentAt":timestamp(original)}),
                ),
            },
            vec![
                HumanActorBinding {
                    declared_actor: "reviewer".into(),
                    assigned_actor: "actor:human:reviewer-1".into(),
                },
                HumanActorBinding {
                    declared_actor: "appealReviewer".into(),
                    assigned_actor: "actor:human:appeal-2".into(),
                },
                HumanActorBinding {
                    declared_actor: "applicant".into(),
                    assigned_actor: "actor:human:applicant-3".into(),
                },
            ],
        )
        .unwrap();
    let id = process.process_id;
    let mut prior = store.load_record(&id).unwrap();
    let mut callback = wos_core::ProvenanceRecord::blank(
        &stack_common_typeid::testing::fixture_minter(),
        ProvenanceKind::CallbackReceived,
    );
    callback.timestamp = timestamp(original);
    callback.actor_id = Some("actor:service:notice".into());
    let date = stack_common_hash::canonical_json_hash(
        &json!({"controlledOwnerFixture":"original-photo-clock"}),
    )
    .unwrap();
    callback.data = Some(
        json!({"bindingId":"sendPhotoRequest","data":{"delivery":{"status":"accepted",
        "intentId":"original-photo-intent","attemptId":"original-photo-attempt"},"clock":{"governedSentAt":timestamp(original),"dateDecisionSha256":date}}}),
    );
    let receipt = wos_runtime::governance::CapturedGovernanceNotice {
        process_id: id.clone(),
        case_ledger_id: prior.process.case_ledger_id.clone(),
        binding_id: "sendPhotoRequest".into(),
        intent_id: "original-photo-intent".into(),
        attempt_id: "original-photo-attempt".into(),
        callback_record_ref: callback.id.clone(),
        callback_data_sha256: stack_common_hash::canonical_json_hash(
            callback.data.as_ref().unwrap(),
        )
        .unwrap(),
        date_decision_sha256: date,
        governed_sent_at: timestamp(original),
        human_readable: "Actual retained request to replace the photo within ninety calendar days"
            .into(),
        proposal_content_sha256: None,
    };
    prior.provenance_log.push(callback);
    prior.process.provenance_position += 1;
    let mut store = store;
    store.save_record(prior).unwrap();
    runtime = runtime
        .with_governance_policy(
            policy.clone(),
            wos_runtime::governance::CapturedGovernanceEvidence {
                decisions: vec![],
                notices: vec![receipt],
            },
        )
        .unwrap();
    (runtime, store, id, clock, effects)
}

#[test]
fn actual_ninety_day_timer_uses_original_qualified_request_and_emits_rule_evidence_without_a_human()
{
    let (mut runtime, store, id, clock, effects) = timeout_rig();
    clock.store(
        1_800_000_000_000 + 90 * 24 * 60 * 60 * 1000 - 1,
        Ordering::SeqCst,
    );
    assert_eq!(runtime.drain_once(&id).unwrap().processed_event, None);
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    clock.store(
        1_800_000_000_000 + 90 * 24 * 60 * 60 * 1000,
        Ordering::SeqCst,
    );
    let outcome = runtime.drain_once(&id).unwrap();
    let proposed = proposal(&outcome);
    assert!(proposed.human_readable.contains("90-calendar-day period"));
    assert!(proposed.human_readable.contains("Later evidence can use"));
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    assert_ne!(
        store.load_record(&id).unwrap().process.case_state["caseClosed"],
        true
    );
    for record in outcome.provenance.iter().filter(|record| {
        matches!(
            record.record_kind,
            ProvenanceKind::ReasoningTierRecorded | ProvenanceKind::CounterfactualTierRecorded
        )
    }) {
        assert_eq!(record.actor_id, None);
        wos_events::NativeProvenanceEvent::from_record(record).unwrap();
    }
}

#[test]
fn actual_timely_correction_uses_its_retained_artifact_and_prevents_timeout_proposal() {
    let (mut runtime, store, id, clock, effects) = timeout_rig();
    clock.store(
        1_800_000_000_000 + 89 * 24 * 60 * 60 * 1000,
        Ordering::SeqCst,
    );
    let task = store
        .load_record(&id)
        .unwrap()
        .process
        .active_tasks
        .iter()
        .find(|task| task.task_ref == "replacePhoto")
        .unwrap()
        .task_id
        .clone();
    runtime.submit_task_response(&task,json!({"status":"completed","definitionUrl":"urn:test:review","definitionVersion":"1.0.0",
        "data":{"approved":true,"photo":"accepted-correction"}}),"actor:human:applicant-3",Some("timely-photo")).unwrap();
    let outcome = runtime.drain_once(&id).unwrap();
    assert_eq!(
        outcome.processed_event.as_deref(),
        Some("photoCorrection.completed")
    );
    assert!(store.load_record(&id).unwrap().process.timers.is_empty());
    clock.store(
        1_800_000_000_000 + 91 * 24 * 60 * 60 * 1000,
        Ordering::SeqCst,
    );
    assert_eq!(runtime.drain_once(&id).unwrap().processed_event, None);
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    assert!(
        !store
            .load_record(&id)
            .unwrap()
            .provenance_log
            .iter()
            .any(|record| record.record_kind == ProvenanceKind::ReasoningTierRecorded)
    );
}

#[test]
fn selected_calendar_period_changes_actual_original_receipt_deadline_and_invalid_periods_refuse() {
    for period in [
        "P0D",
        "P-1D",
        "P1M",
        "PT24H",
        "P18446744073709551615D",
        "invalid",
    ] {
        let mut selection = due_selection();
        selection.kernel["governance"]["dueProcess"]["adverseDecisionPolicy"]["appealMechanism"]
            ["appealWindow"] = json!(period);
        assert!(
            QualifiedGovernancePolicy::qualify(selection).is_err(),
            "{period}"
        );
    }
    let (mut runtime, store, id, _, proposed) = closed_appeal_rig_with_period("P2D");
    assert!(
        proposed
            .human_readable
            .contains("2 calendar days from qualifying original delivery")
    );
    submit_appeal(
        &mut runtime,
        &store,
        &id,
        "appealRequest",
        "actor:human:applicant-3",
        &proposed.facts_record_ref,
    );
    let filed = runtime.drain_once(&id).unwrap();
    let audit = filed
        .provenance
        .iter()
        .find(|record| record.record_kind == ProvenanceKind::AppealFiled)
        .unwrap();
    let data = audit.data.as_ref().unwrap();
    assert_eq!(data["windowRule"], "P2D");
    assert_eq!(
        data["windowDeadline"],
        timestamp(1_799_999_999_000 + 2 * 86_400_000)
    );
}

#[test]
fn selected_mapping_requires_exact_original_artifact_response_and_pin_without_raw_fallback() {
    use wos_runtime::governance::{CapturedGovernanceDecision, CapturedGovernanceEvidence};
    let mapping = json!({"ref":"urn:test:review:mapping:decision-evidence","sha256":format!("sha256:{}", "a".repeat(64))});
    for variant in [
        "actual",
        "missing",
        "duplicate",
        "foreign-artifact",
        "foreign-response",
        "foreign-mapping",
        "foreign-pin",
        "invalid-output",
        "raw-fallback",
    ] {
        let mut selection = serde_json::to_value(selected(kernel())).unwrap();
        selection["humanDecisions"][0]["evidenceMapping"] = mapping.clone();
        let selection: GovernancePolicySelection = serde_json::from_value(selection).unwrap();
        let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
        // Real retained task acceptance remains authoritative. The producer's
        // unique source fields need not have the normalized evidence shape.
        let source = if variant == "raw-fallback" {
            evidence()
        } else {
            json!({"uniqueReason":"An actual human input"})
        };
        let (mut runtime, store, id, effects) =
            rig_selected(selection.kernel.clone(), Some(policy.clone()), source);
        let prior = store.load_record(&id).unwrap();
        let artifact = prior
            .artifacts
            .values()
            .find(|item| item.kind == wos_runtime::TaskArtifactKind::Accepted)
            .unwrap();
        let mut decision = CapturedGovernanceDecision {
            artifact_id: artifact.artifact_id.clone(),
            response_sha256: stack_common_hash::canonical_json_hash(&artifact.response).unwrap(),
            mapping_ref: mapping["ref"].as_str().unwrap().into(),
            mapping_sha256: mapping["sha256"].as_str().unwrap().into(),
            evidence: evidence(),
        };
        match variant {
            "foreign-artifact" => decision.artifact_id.push_str("-foreign"),
            "foreign-response" => decision.response_sha256 = format!("sha256:{}", "b".repeat(64)),
            "foreign-mapping" => decision.mapping_ref.push_str("-foreign"),
            "foreign-pin" => decision.mapping_sha256 = format!("sha256:{}", "b".repeat(64)),
            "invalid-output" => decision.evidence = json!({"reason":"Incomplete mapped output"}),
            _ => (),
        }
        let decisions = match variant {
            "missing" | "raw-fallback" => vec![],
            "duplicate" => vec![decision.clone(), decision],
            _ => vec![decision],
        };
        runtime = runtime
            .with_governance_policy(
                policy.clone(),
                CapturedGovernanceEvidence {
                    notices: vec![],
                    decisions,
                },
            )
            .unwrap();
        let result = runtime.drain_once(&id);
        if variant == "actual" {
            let outcome = result.unwrap();
            assert!(
                outcome
                    .provenance
                    .iter()
                    .any(|record| record.record_kind == ProvenanceKind::ReasoningTierRecorded)
            );
            assert_eq!(effects.load(Ordering::SeqCst), 1);
        } else {
            assert!(
                matches!(result, Err(RuntimeError::GovernancePolicy(_))),
                "{variant}: {result:?}"
            );
            assert_eq!(effects.load(Ordering::SeqCst), 0, "{variant}");
            assert!(
                store
                    .load_record(&id)
                    .unwrap()
                    .provenance_log
                    .iter()
                    .all(|record| record.record_kind != ProvenanceKind::ReasoningTierRecorded),
                "{variant}"
            );
        }
        assert_eq!(
            store.load_record(&id).unwrap().artifacts[&artifact.artifact_id].response,
            artifact.response,
            "{variant}: original Response changed"
        );
    }
}

#[test]
#[ignore = "requires explicit WOS_GOVERNANCE_ARTIFACT_DIR authored D1 selection"]
fn generated_d1_native_policy_qualifies_exact_workflow_delivery_and_mapping_selection() {
    let directory = std::path::PathBuf::from(
        std::env::var("WOS_GOVERNANCE_ARTIFACT_DIR").expect("select exact authored D1 directory"),
    );
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(directory.join(name)).unwrap()).unwrap()
    };
    let kernel = read("passport-governed.workflow.json");
    let delivery = read("passport-governed.delivery.json");
    let mut selected = read("passport-governed.policy.json");
    selected["kernel"] = kernel;
    selected["delivery"] = delivery;
    let policy = QualifiedGovernancePolicy::qualify(serde_json::from_value(selected).unwrap())
        .expect("actual authored selection must qualify");
    assert_eq!(policy.selection().kernel["version"], "0.4.6");
    assert!(
        policy
            .selection()
            .human_decisions
            .iter()
            .all(|binding| binding.evidence_mapping.is_some())
    );
}

fn saved_snapshot(record: &wos_runtime::RuntimeRecord) -> Value {
    json!({"process":record.process,"provenance":record.provenance_log,
        "aux":wos_runtime::runtime_aux_to_json(record)})
}
struct Presentations(Arc<AtomicUsize>);
impl wos_core::traits::TaskPresenter for Presentations {
    type Error = std::io::Error;
    fn present_task(
        &mut self,
        _: &wos_core::instance::FormspecTaskContext,
    ) -> Result<(), Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn dismiss_task(&mut self, _: &str, _: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn selected_output_overlap_refuses_the_entire_action_list_before_any_effect() {
    for (target, protected, refuses) in [
        ("caseFile.caseClosed", "/caseClosed", true),
        ("caseFile.caseClosed.detail", "/caseClosed", true),
        ("caseFile.person", "/person/decision", true),
        ("caseFile.a/b", "/a~1b", true),
        ("caseFile.a~b", "/a~0b", true),
        ("caseFile.caseClosedExtra", "/caseClosed", false),
    ] {
        let mut selection = due_selection();
        selection.kernel["lifecycle"]["states"]["review"]["transitions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
            "event":"probe","target":"probeDone","actions":[
                {"action":"invokeService","serviceRef":"earlier"},
                {"action":"createTask","taskRef":"must-not-present","assignTo":"reviewer"},
                {"action":"invokeService","serviceRef":"mapped"}]}));
        selection.kernel["lifecycle"]["states"]["probeDone"] = json!({"type":"atomic"});
        if protected != "/caseClosed" {
            selection
                .due_process
                .as_mut()
                .unwrap()
                .protected_effects
                .push(wos_runtime::governance::ProtectedGovernanceEffect {
                    path: protected.into(),
                    adverse_values: vec![json!(true)],
                });
        }
        let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
        let store = SharedInMemoryStore::default();
        let effects = Arc::new(AtomicUsize::new(0));
        let presentations = Arc::new(AtomicUsize::new(0));
        let mut runtime=WosRuntime::new(store.clone(),Resolver(serde_json::from_value(selection.kernel).unwrap()),Presentations(presentations.clone()),DefaultRuntime::new(),OutputEffect(effects.clone()),DefaultRuntime::new(),FixedClock,restate_signature_fixture_bindings())
            .with_governance_policy(policy,Default::default()).unwrap()
            .with_integration_profile(serde_json::from_value(json!({"$wosIntegration":"1.0","targetWorkflow":{"url":"urn:test:governance-real-drain","compatibleVersions":"1.0.0"},"bindings":{"mapped":{"type":"request-response","interface":{"$ref":"urn:test:api"},"operation":"update","outputBinding":{target:"$.value"}}}})).unwrap());
        let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
        runtime
            .create_process_with_actor_bindings(
                CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:governance-real-drain".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: Some(json!({"photo":"retained"})),
                },
                vec![
                    HumanActorBinding {
                        declared_actor: "reviewer".into(),
                        assigned_actor: "actor:human:reviewer-1".into(),
                    },
                    HumanActorBinding {
                        declared_actor: "appealReviewer".into(),
                        assigned_actor: "actor:human:appeal-2".into(),
                    },
                    HumanActorBinding {
                        declared_actor: "applicant".into(),
                        assigned_actor: "actor:human:applicant-3".into(),
                    },
                ],
            )
            .unwrap();
        runtime
            .enqueue_event(
                &id,
                wos_core::instance::PendingEvent {
                    event: "probe".into(),
                    actor_id: None,
                    data: None,
                    timestamp: "2027-01-15T08:00:00Z".into(),
                    origin: None,
                    idempotency_token: None,
                },
            )
            .unwrap();
        let before = saved_snapshot(&store.load_record(&id).unwrap());
        let shown = presentations.load(Ordering::SeqCst);
        let result = runtime.drain_once(&id);
        if refuses {
            assert!(
                matches!(result, Err(RuntimeError::GovernancePolicy(_))),
                "{target}: {result:?}"
            );
            assert_eq!(effects.load(Ordering::SeqCst), 0, "{target}");
            assert_eq!(presentations.load(Ordering::SeqCst), shown, "{target}");
            assert_eq!(
                saved_snapshot(&store.load_record(&id).unwrap()),
                before,
                "{target}"
            );
        } else {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(effects.load(Ordering::SeqCst), 2);
        }
    }
}

#[test]
fn selected_generic_version_change_refuses_and_same_version_retains_noop() {
    let selection = due_selection();
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let (mut runtime, store, id, effects) = rig_created(selection.kernel, Some(policy));
    let before = saved_snapshot(&store.load_record(&id).unwrap());
    runtime
        .migrate(&id, "1.0.0", Default::default(), None)
        .unwrap();
    assert!(matches!(
        runtime.migrate(&id, "2.0.0", Default::default(), None),
        Err(RuntimeError::MigrationRejected(_))
    ));
    assert_eq!(saved_snapshot(&store.load_record(&id).unwrap()), before);
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

#[test]
fn actual_governed_emission_retains_the_accepted_decision_facts_across_restore() {
    let mut raw = kernel();
    raw["lifecycle"]["states"]["review"]["transitions"][0]["actions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"action":"emitEvent","eventType":"governed.followup"}));
    let policy = QualifiedGovernancePolicy::qualify(selected(raw.clone())).unwrap();
    let (mut runtime, mut store, id, effects) = rig_selected(raw, Some(policy), evidence());
    let result = runtime.drain_once(&id).unwrap();
    let record = store.load_record(&id).unwrap();
    let queued = record.process.pending_events[0].clone();
    let origin = queued.origin.as_ref().unwrap();
    let reasoning = result
        .provenance
        .iter()
        .find(|r| r.record_kind == ProvenanceKind::ReasoningTierRecorded)
        .unwrap();
    assert_eq!(
        reasoning.data.as_ref().unwrap()["factsRecordRef"],
        origin.transition_record_id.as_ref().unwrap().as_str()
    );
    assert!(record.provenance_log.iter().any(
        |r| r.id == origin.action_record_id && r.record_kind == ProvenanceKind::ActionExecuted
    ));
    let mut restored = record.clone();
    restored.process =
        serde_json::from_value(serde_json::to_value(&record.process).unwrap()).unwrap();
    assert_eq!(restored.process.pending_events[0].origin, queued.origin);
    store.save_record(restored).unwrap();
    assert_eq!(
        runtime.drain_once(&id).unwrap().processed_event.as_deref(),
        Some("governed.followup")
    );
    assert_eq!(effects.load(Ordering::SeqCst), 1);
}

struct OutputEffect(Arc<AtomicUsize>);
impl ExternalService for OutputEffect {
    type Error = std::io::Error;
    fn invoke(&self, _: &str, _: &Value, _: Option<&str>) -> Result<Value, Self::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"value":true}))
    }
}

fn retain_probe(label: &str, before: &Value, after: &Value, effects: usize, result: &str) {
    if let Some(directory) = std::env::var_os("WOS_RUNTIME_GOVERNANCE_PROBE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&json!({
            "before":before,"after":after,"effectCount":effects,"actualResult":result,
            "claim":"Controlled native test observation, not host/provider qualification"}))
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn normal_drain_missing_actual_evidence_preserves_saved_inputs_before_effect() {
    let (mut runtime, store, id, effects) = rig(true, Value::Null);
    let before = saved_snapshot(&store.load_record(&id).unwrap());
    let result = runtime.drain_once(&id);
    let after = saved_snapshot(&store.load_record(&id).unwrap());
    let count = effects.load(Ordering::SeqCst);
    retain_probe(
        "normal-drain-missing-evidence",
        &before,
        &after,
        count,
        &format!("{result:?}"),
    );
    assert!(
        matches!(result, Err(RuntimeError::GovernancePolicy(_))),
        "actual result {result:?}; effects={count}"
    );
    assert_eq!(after, before);
    assert_eq!(count, 0);
}

#[test]
fn actual_protected_task_mapping_preserves_artifacts_tasks_replay_and_case() {
    let mut selection = due_selection();
    selection.kernel["contracts"]["review"]["responseMappingRef"] =
        json!("urn:test:actual-response-mapping");
    selection
        .due_process
        .as_mut()
        .unwrap()
        .protected_effects
        .push(wos_runtime::governance::ProtectedGovernanceEffect {
            path: "/decision".into(),
            adverse_values: vec![json!(true)],
        });
    let policy = QualifiedGovernancePolicy::qualify(selection.clone()).unwrap();
    let (mut runtime, store, id, effects) = rig_created(selection.kernel, Some(policy));
    let before_record = store.load_record(&id).unwrap();
    let task = before_record.process.active_tasks[0].task_id.clone();
    let before = saved_snapshot(&before_record);
    let result=runtime.submit_task_response(&task,json!({"status":"completed","definitionUrl":"urn:test:review","definitionVersion":"1.0.0","data":{"approved":true,"governance":evidence()}}),"actor:human:reviewer-1",Some("protected-mapping"));
    let after = saved_snapshot(&store.load_record(&id).unwrap());
    let count = effects.load(Ordering::SeqCst);
    retain_probe(
        "protected-task-mapping",
        &before,
        &after,
        count,
        &format!("{result:?}"),
    );
    assert!(
        matches!(result, Err(RuntimeError::GovernancePolicy(_))),
        "actual result {result:?}"
    );
    assert_eq!(after, before);
    assert_eq!(count, 0);
}

#[test]
fn foreign_kernel_policy_installation_refuses_before_creation_or_effects() {
    let policy = QualifiedGovernancePolicy::qualify(selected(kernel())).unwrap();
    let mut foreign = kernel();
    foreign["title"] = json!("Foreign actual source");
    let effects = Arc::new(AtomicUsize::new(0));
    let runtime = WosRuntime::new(
        SharedInMemoryStore::default(),
        Resolver(serde_json::from_value(foreign).unwrap()),
        DefaultRuntime::new(),
        DefaultRuntime::new(),
        Effect(effects.clone()),
        DefaultRuntime::new(),
        FixedClock,
        restate_signature_fixture_bindings(),
    );
    assert!(matches!(
        runtime.with_governance_policy(policy, Default::default()),
        Err(RuntimeError::GovernancePolicy(_))
    ));
    assert_eq!(effects.load(Ordering::SeqCst), 0);
}

#[test]
fn protected_paths_require_actual_json_pointer_escapes_at_qualification() {
    for path in ["caseClosed", "/bad~escape", "/bad~2escape", "/trailing~"] {
        let mut selection = due_selection();
        selection.due_process.as_mut().unwrap().protected_effects[0].path = path.into();
        assert!(
            QualifiedGovernancePolicy::qualify(selection).is_err(),
            "{path}"
        );
    }
}
