//! Behavioral controls for the maintained checker and shared validation entry.
use serde_json::{Value, json};
use wos_lint::studio_api::{CheckStatus, ValidationRequest, ValidationScope, validate};
use wos_lint::{LintDiagnostic, LintSeverity, lint_document, lint_workflow_with_project};

fn workflow(states: Value, initial: &str) -> Value {
    json!({"$wosWorkflow":"1.0", "url":"urn:test:control-flow", "version":"1.0.0", "title":"Graph control", "impactLevel":"operational", "actors":[], "lifecycle":{"initialState":initial,"states":states}})
}
fn lint(value: &Value) -> Vec<LintDiagnostic> {
    lint_document(&value.to_string())
        .unwrap()
        .into_iter()
        .filter(|d| d.rule_id.starts_with("WOS-FLOW-"))
        .collect()
}
fn rule(diags: &[LintDiagnostic], id: &str) -> bool {
    diags.iter().any(|d| d.rule_id == id)
}
fn clean(value: &Value) {
    let d = lint(value);
    assert!(
        !d.iter().any(|d| d.severity == LintSeverity::Error),
        "{d:?}"
    );
    assert!(!rule(&d, "WOS-FLOW-005"), "{d:?}");
}
fn transition(target: &str) -> Value {
    json!({"event":{"kind":"message","name":"next"},"target":target})
}
fn parallel(policy: &str, right: Value) -> Value {
    workflow(
        json!({"p":{"type":"parallel","cancellationPolicy":policy,"regions":{
        "left":{"initialState":"l","states":{"l":{"type":"atomic","transitions":[transition("lf")]},"lf":{"type":"final"}}},
        "right":{"initialState":"r","states":right}},
        "transitions":[{"event":{"kind":"signal","name":"$join","scope":"instance"},"target":"done"}]},"done":{"type":"final"}}),
        "p",
    )
}
#[test]
fn retained_schema_valid_controls_and_shared_entry_parity() {
    for (source, expected, path) in [
        (
            include_str!("fixtures/control-flow/closed_cycle.json"),
            Some("WOS-FLOW-004"),
            "/lifecycle/states/review",
        ),
        (
            include_str!("fixtures/control-flow/reachable_dead_end.json"),
            Some("WOS-FLOW-003"),
            "/lifecycle/states/review",
        ),
        (
            include_str!("fixtures/control-flow/unreachable_state.json"),
            Some("WOS-FLOW-002"),
            "/lifecycle/states/orphan",
        ),
        (
            include_str!("fixtures/control-flow/complete_path.json"),
            None,
            "",
        ),
    ] {
        let value: Value = serde_json::from_str(source).unwrap();
        let doc = lint(&value);
        let project: Vec<_> = lint_workflow_with_project(source)
            .unwrap()
            .into_iter()
            .filter(|d| d.rule_id.starts_with("WOS-FLOW-"))
            .collect();
        assert_eq!(
            serde_json::to_value(&doc).unwrap(),
            serde_json::to_value(project).unwrap()
        );
        let shared = validate(ValidationRequest {
            candidate: &value,
            dependencies: &[],
            scope: ValidationScope::Document,
        });
        assert_eq!(
            shared.stages[0].status,
            CheckStatus::Pass,
            "schema validity must remain unchanged"
        );
        let native: Vec<_> = shared
            .diagnostics
            .into_iter()
            .filter(|d| d.rule_id.starts_with("WOS-FLOW-"))
            .collect();
        assert_eq!(
            serde_json::to_value(&doc).unwrap(),
            serde_json::to_value(native).unwrap()
        );
        if let Some(expected) = expected {
            assert!(
                doc.iter().any(|d| d.rule_id == expected && d.path == path),
                "{doc:?}"
            );
            assert_eq!(shared.status, CheckStatus::Fail);
        } else {
            clean(&value);
            assert_eq!(shared.status, CheckStatus::Pass);
        }
    }
}
#[test]
fn hierarchy_inherited_exit_scoped_finality_and_global_targets() {
    clean(&workflow(
        json!({"parent":{"type":"compound","initialState":"child","states":{"child":{"type":"atomic"}},"transitions":[transition("done")]},"done":{"type":"final"}}),
        "parent",
    ));
    clean(&workflow(
        json!({"parent":{"type":"compound","initialState":"child","states":{"child":{"type":"atomic","transitions":[transition("nestedFinal")]},"nestedFinal":{"type":"final"}},"transitions":[transition("done")]},"done":{"type":"final"}}),
        "parent",
    ));
    let no_parent_exit = workflow(
        json!({"parent":{"type":"compound","initialState":"nestedFinal","states":{"nestedFinal":{"type":"final"}}}}),
        "parent",
    );
    assert!(rule(&lint(&no_parent_exit), "WOS-FLOW-003"));
    let duplicate = workflow(
        json!({"same":{"type":"atomic","transitions":[transition("parent")]},"parent":{"type":"compound","initialState":"same","states":{"same":{"type":"atomic"}}}}),
        "same",
    );
    let d = lint(&duplicate);
    assert!(rule(&d, "WOS-FLOW-001"));
    assert!(!rule(&d, "WOS-FLOW-002"));
}
#[test]
fn cycles_with_exit_remain_permitted_and_output_is_deterministic() {
    let v = workflow(
        json!({"a":{"type":"atomic","transitions":[transition("b")]},"b":{"type":"atomic","transitions":[transition("a"),transition("done")]},"done":{"type":"final"}}),
        "a",
    );
    clean(&v);
    assert_eq!(
        serde_json::to_value(lint(&v)).unwrap(),
        serde_json::to_value(lint(&v)).unwrap()
    );
    let mut closed = v;
    closed["lifecycle"]["states"]["b"]["transitions"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(rule(&lint(&closed), "WOS-FLOW-004"));
}
#[test]
fn parallel_join_policies_and_impossible_join() {
    let right =
        json!({"r":{"type":"atomic","transitions":[transition("rf")]},"rf":{"type":"final"}});
    clean(&parallel("wait-all", right.clone()));
    clean(&parallel("fail-fast", right));
    let looped = json!({"r":{"type":"atomic","transitions":[transition("r")]}});
    assert!(rule(
        &lint(&parallel("wait-all", looped.clone())),
        "WOS-FLOW-004"
    ));
    clean(&parallel("cancel-siblings", looped));
    let mut failure = parallel(
        "fail-fast",
        json!({"r":{"type":"atomic","transitions":[transition("r")]}}),
    );
    failure["lifecycle"]["states"]["p"]["regions"]["left"]["states"]["lf"]["tags"] =
        json!(["error"]);
    clean(&failure);
}
#[test]
fn persistent_receipt_region_uses_typed_inbound_semantics() {
    let receipt = json!({"r":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"receipt"},"target":"r","actions":[{"action":"invokeService","serviceRef":"urn:test:binding","callbackPhase":"inbound"}]}]}});
    let mut valid = parallel("wait-all", receipt);
    valid["lifecycle"]["states"]
        .as_object_mut()
        .unwrap()
        .remove("done");
    valid["lifecycle"]["states"]["p"]["transitions"] = json!([]);
    clean(&valid);
    assert!(rule(&lint(&valid), "WOS-FLOW-007"));
    let mut ordinary_loop = valid.clone();
    ordinary_loop["lifecycle"]["states"]["p"]["regions"]["right"]["states"]["r"]["transitions"]
        [0]["actions"] = json!([]);
    assert!(rule(&lint(&ordinary_loop), "WOS-FLOW-004"));
    assert!(!rule(&lint(&ordinary_loop), "WOS-FLOW-007"));
    let mut creates_tasks = valid;
    creates_tasks["lifecycle"]["states"]["p"]["regions"]["right"]["states"]["r"]["onEntry"] =
        json!([{"action":"createTask","taskRef":"review","assignTo":"reviewer"}]);
    assert!(rule(&lint(&creates_tasks), "WOS-FLOW-004"));
}
#[test]
fn guards_events_and_ordered_choices_never_claim_feasibility() {
    let v = workflow(
        json!({"a":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"external"},"guard":"false","target":"done"}]},"done":{"type":"final"}}),
        "a",
    );
    clean(&v);
    let d = lint(&v);
    assert!(
        d.iter()
            .any(|d| d.rule_id == "WOS-FLOW-006" && d.message.contains("not proved"))
    );
}
#[test]
fn unsupported_history_and_iteration_refuse_full_static_pass() {
    for mode in ["shallow", "deep"] {
        let v = workflow(
            json!({"h":{"type":"compound","initialState":"done","historyState":mode,"states":{"done":{"type":"final"}}}}),
            "h",
        );
        let d = lint(&v);
        assert!(rule(&d, "WOS-FLOW-005"));
        assert!(!rule(&d, "WOS-FLOW-003"));
        let report = validate(ValidationRequest {
            candidate: &v,
            dependencies: &[],
            scope: ValidationScope::Document,
        });
        assert!(
            report
                .stages
                .iter()
                .any(|s| s.name == "workflow-control-flow"
                    && s.required
                    && s.status == CheckStatus::NotRun)
        );
        // The current canonical schema omits historyState, so it may also
        // reject the input; the graph stage must independently stay not-run.
        assert_ne!(report.status, CheckStatus::Pass);
    }
    let v = workflow(
        json!({"f":{"type":"foreach","collection":"[]","body":{"type":"atomic"}}}),
        "f",
    );
    assert!(rule(&lint(&v), "WOS-FLOW-005"));
}
#[test]
fn pointers_escape_authored_global_ids() {
    let v = workflow(
        json!({"start":{"type":"final"},"orphan/~":{"type":"atomic"}}),
        "start",
    );
    assert!(
        lint(&v)
            .iter()
            .any(|d| d.rule_id == "WOS-FLOW-002" && d.path == "/lifecycle/states/orphan~1~0")
    );
}

#[test]
fn graph_bounds_and_unsupported_constraints_never_silently_pass() {
    let states: serde_json::Map<String, Value> = (0..4097)
        .map(|i| (format!("s{i}"), json!({"type":"final"})))
        .collect();
    let bounded = workflow(Value::Object(states), "s0");
    let d = lint(&bounded);
    assert!(rule(&d, "WOS-FLOW-005"));
    assert!(!rule(&d, "WOS-FLOW-002"));
    let mut advanced = workflow(json!({"done":{"type":"final"}}), "done");
    advanced["advanced"] = json!({});
    let d = lint(&advanced);
    assert!(rule(&d, "WOS-FLOW-005"));
    assert!(!rule(&d, "WOS-FLOW-006"));
}

#[test]
fn dangling_target_refuses_graph_conclusions() {
    let v = workflow(
        json!({"a":{"type":"atomic","transitions":[transition("missing")]}}),
        "a",
    );
    let d = lint(&v);
    assert!(!rule(&d, "WOS-FLOW-002"));
    assert!(!rule(&d, "WOS-FLOW-003"));
    assert!(!rule(&d, "WOS-FLOW-004"));
}

#[test]
fn unreachable_cycle_does_not_generate_reachable_cycle_error() {
    let v = workflow(
        json!({"done":{"type":"final"},"island":{"type":"atomic","transitions":[transition("island")]}}),
        "done",
    );
    let d = lint(&v);
    assert!(rule(&d, "WOS-FLOW-002"));
    assert!(!rule(&d, "WOS-FLOW-004"));
}

#[test]
fn orthogonal_region_targets_reset_the_common_parallel_scope() {
    let v = workflow(
        json!({"p":{"type":"parallel","regions":{
        "left":{"initialState":"l","states":{"l":{"type":"atomic","transitions":[transition("rf")]},"lf":{"type":"final"}}},
        "right":{"initialState":"r","states":{"r":{"type":"atomic","transitions":[transition("lf")]},"rf":{"type":"final"}}}},
        "transitions":[{"event":{"kind":"signal","name":"$join","scope":"instance"},"target":"done"}]},"done":{"type":"final"}}),
        "p",
    );
    let d = lint(&v);
    assert!(
        d.iter()
            .any(|d| d.rule_id == "WOS-FLOW-002" && d.path == "/lifecycle/states/done"),
        "cross-region reset must never fabricate a compatible join: {d:?}"
    );
    assert!(rule(&d, "WOS-FLOW-004"));
}

#[test]
fn transition_to_active_ancestor_reenters_its_initial_child() {
    let v = workflow(
        json!({"c":{"type":"compound","initialState":"a","states":{
        "a":{"type":"atomic","transitions":[transition("b")]},
        "b":{"type":"atomic","transitions":[transition("c"),transition("done")]} }},"done":{"type":"final"}}),
        "c",
    );
    clean(&v);
}

#[test]
fn concurrent_configuration_bound_is_explicit_and_suppresses_findings() {
    let regions: serde_json::Map<String, Value> = (0..14)
        .map(|i| {
            let start = format!("start{i}");
            let done = format!("done{i}");
            (
                format!("region{i}"),
                json!({"initialState":start,"states":{
            start:{"type":"atomic","transitions":[transition(&done)]},done:{"type":"final"}}}),
            )
        })
        .collect();
    let v = workflow(json!({"p":{"type":"parallel","regions":regions}}), "p");
    let d = lint(&v);
    assert!(
        d.iter().any(
            |d| d.rule_id == "WOS-FLOW-005" && d.message.contains("active-configuration bound")
        ),
        "{d:?}"
    );
    assert!(!d.iter().any(|d| matches!(
        d.rule_id,
        "WOS-FLOW-002" | "WOS-FLOW-003" | "WOS-FLOW-004" | "WOS-FLOW-006"
    )));
}

/// The admin appeal branch of the selected D1 workflows is neither a final nor a
/// supported permanent inbound receipt region: lint reports the stranded state
/// and does not turn the supported topology into an analysis refusal. These are
/// retained authored bytes; their refusal is preserved, not exempted.
fn assert_stranded_appeal_branch_reported(source: &str) {
    let diagnostics = lint_document(source).unwrap();
    let graph: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule_id.starts_with("WOS-FLOW-"))
        .collect();
    assert!(
        graph.iter().any(|d| d.rule_id == "WOS-FLOW-004"
            && d.path
                == "/lifecycle/states/caseAndDelivery/regions/administrativeReconsideration/states/appealGrantRecorded"
            && d.message.contains("appealGrantRecorded")),
        "{graph:?}"
    );
    assert!(
        !graph.iter().any(|d| d.rule_id == "WOS-FLOW-005"),
        "supported topology must not become an analysis refusal: {graph:?}"
    );
    assert!(
        graph.iter().any(|d| d.rule_id == "WOS-FLOW-007"),
        "{graph:?}"
    );
}

#[test]
fn selected_d1_0_4_1_workflow_reports_its_stranded_appeal_branch() {
    // formspec-studio demo-authoring/outputs/d1-full/v0.4.1/passport-governed.workflow.json,
    // SHA-256 53c33d2f103c0964f1abab5d163ffeac4dd2eaec64532f5855c8d974e153bba1.
    assert_stranded_appeal_branch_reported(include_str!(
        "fixtures/control-flow/d1-current-0.4.1.workflow.json"
    ));
}

#[test]
#[ignore = "requires WOS_GOVERNANCE_ARTIFACT_DIR=<formspec-studio>/demo-authoring/outputs/d1-full/v0.4.0"]
fn selected_d1_0_4_0_workflow_reports_its_stranded_appeal_branch() {
    // Studio owns the authored bytes, SHA-256
    // 61fc6f275f77f14eb705a9754ee0f4230eab571078036613f701e829d1255133.
    let directory = std::path::PathBuf::from(
        std::env::var("WOS_GOVERNANCE_ARTIFACT_DIR")
            .expect("select the authored D1 v0.4.0 directory"),
    );
    let source =
        std::fs::read_to_string(directory.join("passport-governed.workflow.json")).unwrap();
    let value: Value = serde_json::from_str(&source).unwrap();
    assert_eq!(value["version"], "0.4.0", "select the v0.4.0 directory");
    assert_stranded_appeal_branch_reported(&source);
}
