// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS Kernel Documents.
//!
//! Verifies that [`KernelDocument`] can deserialize every valid kernel
//! fixture without data loss. Each test loads a fixture, deserializes
//! it, and asserts key structural properties.

use std::fs;
use std::path::{Path, PathBuf};
use wos_core::KernelDocument;

fn workspace_root() -> PathBuf {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root is two levels above crates/wos-core")
        .to_path_buf();

    let cwd = std::env::current_dir().ok();
    for candidate in [Some(manifest_root), cwd].into_iter().flatten() {
        for ancestor in candidate.ancestors() {
            if ancestor.join("fixtures").is_dir()
                && ancestor.join("schemas/wos-workflow.schema.json").is_file()
            {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("could not resolve workspace root with fixtures/ and schemas/");
}

/// Loads and deserializes a kernel fixture by filename.
fn load_fixture(name: &str) -> KernelDocument {
    let path = workspace_root().join("fixtures/kernel").join(name);
    let json =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn purchase_order_approval_round_trips() {
    let doc = load_fixture("purchase-order-approval.json");
    assert_eq!(doc.wos_workflow, "1.0");
    assert_eq!(doc.title.as_deref(), Some("Purchase Order Approval"));
    assert_eq!(doc.impact_level, Some(wos_core::ImpactLevel::Operational));
    assert_eq!(doc.actors.len(), 3);
    assert_eq!(doc.lifecycle.initial_state, "submitted");
    assert!(doc.lifecycle.states.contains_key("submitted"));
    assert!(doc.lifecycle.states.contains_key("approved"));
    assert!(doc.lifecycle.states.contains_key("rejected"));
    assert!(doc.contracts.contains_key("purchaseOrderForm"));
    assert_eq!(doc.contracts["purchaseOrderForm"].binding, "formspec");
    assert!(doc.execution.is_some());
    let exec = doc.execution.as_ref().unwrap();
    assert_eq!(exec.workflow_timeout.as_deref(), Some("P90D"));
}

#[test]
fn benefits_adjudication_round_trips() {
    let doc = load_fixture("benefits-adjudication.json");
    assert_eq!(doc.wos_workflow, "1.0");
    assert_eq!(
        doc.impact_level,
        Some(wos_core::ImpactLevel::RightsImpacting)
    );
    assert!(!doc.actors.is_empty());
    assert!(doc.lifecycle.states.contains_key("intake"));
    assert!(doc.contracts.contains_key("applicationForm"));
    assert!(doc.execution.is_some());
}

#[test]
fn medicaid_redetermination_round_trips() {
    let doc = load_fixture("medicaid-redetermination.json");
    assert_eq!(doc.wos_workflow, "1.0");
    assert_eq!(
        doc.impact_level,
        Some(wos_core::ImpactLevel::RightsImpacting)
    );
    assert!(doc.lifecycle.states.len() > 5);
    assert!(doc.contracts.contains_key("applicationForm"));
}

#[test]
fn case_relationship_appeal_round_trips() {
    let doc = load_fixture("case-relationship-appeal.json");
    assert_eq!(doc.wos_workflow, "1.0");
    let case_file = doc.case_file.as_ref().expect("case_file present");
    assert!(
        !case_file.relationships.is_empty(),
        "should have case relationships"
    );
}

#[test]
fn new_phase2_fields_round_trip() {
    // Verify evaluationMode and maxRelationshipEventDepth deserialize correctly.
    let json = r#"{
        "$wosWorkflow": "1.0",
        "evaluationMode": "continuous",
        "maxRelationshipEventDepth": 5,
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": { "type": "atomic" },
                "end": { "type": "final" }
            }
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    assert_eq!(
        doc.evaluation_mode,
        Some(wos_core::EvaluationMode::Continuous)
    );
    assert_eq!(doc.max_relationship_event_depth, Some(5));
}

#[test]
fn evaluation_mode_defaults_absent() {
    // When evaluationMode is absent, it should be None (default event-driven).
    let json = r#"{
        "$wosWorkflow": "1.0",
        "lifecycle": {
            "initialState": "s",
            "states": { "s": { "type": "atomic" } }
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    assert!(doc.evaluation_mode.is_none());
    assert!(doc.max_relationship_event_depth.is_none());
}

#[test]
fn contract_reference_typed() {
    let json = r#"{
        "$wosWorkflow": "1.0",
        "lifecycle": {
            "initialState": "s",
            "states": { "s": { "type": "atomic" } }
        },
        "contracts": {
            "myForm": {
                "binding": "formspec",
                "ref": "urn:formspec:test:1.0",
                "description": "Test contract",
                "prefillMappingRef": "urn:formspec:test-prefill:1.0",
                "responseMappingRef": "urn:formspec:test-response:1.0"
            }
        },
        "execution": {
            "workflowTimeout": "P90D",
            "compensable": true
        }
    }"#;
    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    let contract = &doc.contracts["myForm"];
    assert_eq!(contract.binding, "formspec");
    assert_eq!(contract.reference, "urn:formspec:test:1.0");
    assert_eq!(contract.description.as_deref(), Some("Test contract"));
    assert_eq!(
        contract.prefill_mapping_ref.as_deref(),
        Some("urn:formspec:test-prefill:1.0")
    );
    assert_eq!(
        contract.response_mapping_ref.as_deref(),
        Some("urn:formspec:test-response:1.0")
    );
    let exec = doc.execution.as_ref().unwrap();
    assert!(exec.compensable);
}

#[test]
fn create_task_formspec_coprocessor_fields_round_trip() {
    let json = r#"{
        "$wosWorkflow": "1.0",
        "lifecycle": {
            "initialState": "s",
            "states": {
                "s": {
                    "type": "atomic",
                    "onEntry": [{
                        "action": "createTask",
                        "taskRef": "complete-intake",
                        "assignTo": "applicant-123",
                        "contractRef": "intakeApplication",
                        "prefillMappingRef": "urn:formspec:intake-prefill:1.0",
                        "responseMappingRef": "urn:formspec:intake-response:1.0",
                        "completionEvent": "intake.completed",
                        "failureEvent": "intake.failed"
                    }]
                }
            }
        }
    }"#;

    let doc: wos_core::KernelDocument = serde_json::from_str(json).unwrap();
    let action = &doc.lifecycle.states["s"].on_entry[0];
    assert_eq!(action.contract_ref.as_deref(), Some("intakeApplication"));
    assert_eq!(
        action.prefill_mapping_ref.as_deref(),
        Some("urn:formspec:intake-prefill:1.0")
    );
    assert_eq!(
        action.response_mapping_ref.as_deref(),
        Some("urn:formspec:intake-response:1.0")
    );
    assert_eq!(action.completion_event.as_deref(), Some("intake.completed"));
    assert_eq!(action.failure_event.as_deref(), Some("intake.failed"));
}

// ── ADR 0076 / B-2: WorkflowDocument alias + embedded blocks + KernelView ───

#[test]
fn workflow_document_alias_resolves_to_kernel_document() {
    // WorkflowDocument is the canonical name; KernelDocument is the alias.
    // Both names must resolve to the same type so existing consumers keep
    // working unchanged while new code uses the canonical name.
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/alias",
        "version": "1.0.0",
        "title": "Alias Test",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        }
    });
    let workflow: wos_core::WorkflowDocument =
        serde_json::from_value(json.clone()).expect("WorkflowDocument deserializes");
    let kernel: KernelDocument =
        serde_json::from_value(json).expect("KernelDocument alias deserializes the same JSON");
    assert_eq!(workflow.url, kernel.url);
    assert_eq!(workflow.version, kernel.version);
    assert_eq!(
        workflow.lifecycle.initial_state,
        kernel.lifecycle.initial_state
    );
}

#[test]
fn embedded_governance_block_round_trips_as_raw_value() {
    // The embedded `governance` block is carried as raw serde_json::Value to
    // keep KernelDocument deserialization tolerant of deep governance shapes
    // (pipelines, review protocols, etc.) that the strict typed
    // GovernanceDocument doesn't round-trip cleanly. Consumers that want
    // typed access deserialize on demand.
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/embed-governance",
        "version": "1.0.0",
        "title": "Embedded Governance",
        "impactLevel": "rights-impacting",
        "actors": [{"id": "caseworker", "type": "human"}],
        "lifecycle": {
            "initialState": "review",
            "states": {
                "review": {"type": "atomic", "transitions": [{"target": "decided"}]},
                "decided": {"type": "final"}
            }
        },
        "governance": {
            "dueProcess": {"scope": "true"},
            "maxDelegationDepth": 3
        }
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("deserializes");
    let gov = doc.governance.as_ref().expect("governance present");
    assert_eq!(gov["maxDelegationDepth"], 3);
    assert!(gov["dueProcess"].is_object());

    // On-demand typed deserialization MUST succeed for this minimal shape.
    let typed: wos_core::GovernanceDocument =
        serde_json::from_value(gov.clone()).expect("on-demand typed deserialization succeeds");
    assert_eq!(typed.max_delegation_depth, 3);
    assert!(typed.due_process.is_some());
}

#[test]
fn embedded_block_absent_when_not_in_envelope() {
    // A workflow without governance/agents/aiOversight MUST deserialize
    // cleanly with each embedded block at its serde default.
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/no-blocks",
        "version": "1.0.0",
        "title": "No Embedded Blocks",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        }
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("deserializes");
    assert!(doc.governance.is_none());
    assert!(doc.agents.is_empty());
    assert!(doc.ai_oversight.is_none());
    assert!(doc.signature.is_none());
    assert!(doc.custody.is_none());
    assert!(doc.advanced.is_none());
    assert!(doc.assurance.is_none());
    assert!(doc.intake.is_none());
    assert!(doc.bindings.is_empty());
}

#[test]
fn kernel_view_borrows_kernel_relevant_slice() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/kernel-view",
        "version": "2.0.0",
        "title": "KernelView Borrow",
        "impactLevel": "operational",
        "actors": [
            {"id": "extractor", "type": "agent"},
            {"id": "caseworker", "type": "human"}
        ],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        },
        "evaluationMode": "event-driven",
        "maxRelationshipEventDepth": 5,
        "governance": {
            "dueProcess": {"scope": "true"}
        }
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("deserializes");
    let view = doc.kernel_view();

    // The kernel view exposes the kernel-relevant slice directly.
    assert_eq!(
        view.url(),
        Some("https://test.wos-spec.org/workflows/kernel-view")
    );
    assert_eq!(view.version(), Some("2.0.0"));
    assert_eq!(
        view.impact_level(),
        Some(wos_core::ImpactLevel::Operational)
    );
    assert_eq!(view.actors().len(), 2);
    assert_eq!(view.lifecycle().initial_state, "start");
    assert_eq!(
        view.evaluation_mode(),
        Some(wos_core::model::kernel::EvaluationMode::EventDriven)
    );
    assert_eq!(view.max_relationship_event_depth(), Some(5));

    // The view does NOT expose embedded blocks (that's by design — use the
    // document directly when you need both kernel + governance).
    // But document() lets you escape back to the full envelope.
    assert!(view.document().governance.is_some());
}

#[test]
fn kernel_view_is_zero_cost_borrow() {
    // Borrow-check sanity: building a view does not move the document, and
    // the view's lifetime ties to the source.
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "lifecycle": {"initialState": "x", "states": {"x": {"type": "final"}}}
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("deserializes");
    let view_a = doc.kernel_view();
    let view_b = doc.kernel_view();
    // Both views borrow the same document.
    assert_eq!(
        view_a.lifecycle().initial_state,
        view_b.lifecycle().initial_state
    );
}

// ── Sub-PR D: ForEach state ─────────────────────────────────────────────────

#[test]
fn foreach_state_round_trips_with_required_fields() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/foreach-deser",
        "version": "1.0.0",
        "title": "ForEach Round-Trip",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "iter",
            "states": {
                "iter": {
                    "type": "foreach",
                    "collection": "caseFile.items",
                    "body": {"type": "atomic"},
                    "transitions": [{"target": "done"}]
                },
                "done": {"type": "final"}
            }
        }
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("foreach state deserializes");
    let iter_state = &doc.lifecycle.states["iter"];
    assert_eq!(iter_state.kind, wos_core::model::kernel::StateKind::ForEach);
    assert_eq!(iter_state.collection.as_deref(), Some("caseFile.items"));
    assert!(iter_state.body.is_some(), "body MUST round-trip");
    assert!(
        iter_state.item_variable.is_none(),
        "itemVariable defaults to $item (None means 'use default')"
    );
}

#[test]
fn foreach_state_preserves_optional_fields() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/foreach-full",
        "version": "1.0.0",
        "title": "ForEach Full Shape",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "iter",
            "states": {
                "iter": {
                    "type": "foreach",
                    "collection": "caseFile.applicants",
                    "itemVariable": "$applicant",
                    "indexVariable": "$idx",
                    "concurrency": 5,
                    "breakCondition": "$applicant.eligibility == 'denied'",
                    "outputPath": "caseFile.results",
                    "mergeStrategy": "collect",
                    "body": {"type": "atomic"},
                    "transitions": [{"target": "done"}]
                },
                "done": {"type": "final"}
            }
        }
    });
    let doc: KernelDocument = serde_json::from_value(json).expect("full foreach deserializes");
    let s = &doc.lifecycle.states["iter"];
    assert_eq!(s.item_variable.as_deref(), Some("$applicant"));
    assert_eq!(s.index_variable.as_deref(), Some("$idx"));
    assert_eq!(s.concurrency, Some(5));
    assert_eq!(
        s.break_condition.as_deref(),
        Some("$applicant.eligibility == 'denied'")
    );
    assert_eq!(s.output_path.as_deref(), Some("caseFile.results"));
    assert_eq!(
        s.merge_strategy,
        Some(wos_core::model::kernel::MergeStrategy::Collect)
    );
}

// ── ADR 0064: Agent ActorKind variant ───────────────────────────────────────

#[test]
fn agent_actor_kind_round_trips() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/agent-actor-roundtrip",
        "version": "1.0.0",
        "title": "Agent Actor Round-Trip",
        "impactLevel": "operational",
        "actors": [
            {"id": "extractor", "type": "agent"},
            {"id": "caseworker", "type": "human"},
            {"id": "scheduler", "type": "system"}
        ],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        }
    });
    let doc: KernelDocument =
        serde_json::from_value(json).expect("agent-typed actor MUST deserialize per ADR 0064");
    assert_eq!(doc.actors.len(), 3);
    assert_eq!(doc.actors[0].id, "extractor");
    assert_eq!(
        doc.actors[0].kind,
        wos_core::model::kernel::ActorKind::Agent
    );
    assert_eq!(
        doc.actors[1].kind,
        wos_core::model::kernel::ActorKind::Human
    );
    assert_eq!(
        doc.actors[2].kind,
        wos_core::model::kernel::ActorKind::System
    );
}

// ── caseFile.contractRef + FieldDefinition.required ─────────────────────────

#[test]
fn case_file_contract_ref_round_trips() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/case-file-contract-ref",
        "version": "1.0.0",
        "title": "CaseFile contractRef Round-Trip",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        },
        "caseFile": {
            "contractRef": "https://agency.gov/contracts/benefits-applicant.json",
            "contractVersion": "1.0.0"
        }
    });
    let doc: KernelDocument =
        serde_json::from_value(json).expect("caseFile.contractRef MUST deserialize");
    let case_file = doc.case_file.expect("caseFile present");
    assert_eq!(
        case_file.contract_ref.as_deref(),
        Some("https://agency.gov/contracts/benefits-applicant.json")
    );
    assert_eq!(case_file.contract_version.as_deref(), Some("1.0.0"));
    assert!(
        case_file.fields.is_empty(),
        "contractRef shape MUST NOT carry inline fields (oneOf in schema)"
    );
}

#[test]
fn case_file_field_required_round_trips() {
    let json = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "https://test.wos-spec.org/workflows/field-required",
        "version": "1.0.0",
        "title": "FieldDefinition.required Round-Trip",
        "impactLevel": "operational",
        "actors": [{"id": "system", "type": "system"}],
        "lifecycle": {
            "initialState": "start",
            "states": {
                "start": {"type": "atomic", "transitions": [{"target": "done"}]},
                "done": {"type": "final"}
            }
        },
        "caseFile": {
            "fields": {
                "applicantName": {"type": "string", "required": true},
                "monthlyIncome": {"type": "number"}
            }
        }
    });
    let doc: KernelDocument =
        serde_json::from_value(json).expect("fields with required MUST deserialize");
    let case_file = doc.case_file.expect("caseFile present");
    assert!(
        case_file.fields["applicantName"].required,
        "applicantName.required SHOULD be true"
    );
    assert!(
        !case_file.fields["monthlyIncome"].required,
        "monthlyIncome.required SHOULD default to false"
    );
}

#[test]
fn non_kernel_fixtures_do_not_parse() {
    // These files in fixtures/kernel/ are NOT KernelDocuments.
    // Verify they fail to parse as KernelDocument.
    let non_kernel = [
        "invalid-documents.json",
        "benefits-correspondence-metadata.json",
        "purchase-order-provenance.json",
    ];
    for name in non_kernel {
        let path = workspace_root().join("fixtures/kernel").join(name);
        let json = fs::read_to_string(&path).unwrap();
        let result: Result<KernelDocument, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "{name} should not parse as KernelDocument");
    }
}
