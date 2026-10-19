// Rust guideline compliant 2026-02-21

//! Independent native admission and policy behavior controls.
use serde_json::{Value, json};
use wos_core::ai_policy::{
    PolicyExerciseContext, PolicyPhase, load_native_ai_policy, native_rule_sources,
};
use wos_core::deontic::evaluate_native_policy;

fn candidate() -> Value {
    json!({"$wosWorkflow":"1.0","url":"urn:test:native-policy","version":"1.0.0","title":"Native policy controls","impactLevel":"operational","actors":[{"id":"assessor","type":"agent"},{"id":"reviewer","type":"human"}],"agents":[{"id":"assessor","agentType":"deterministic","modelIdentifier":"test-rules","modelVersion":"1.0.0"}],"aiOversight":{},"lifecycle":{"initialState":"review","states":{"review":{"type":"atomic"}}}})
}
#[test]
fn independent_native_admission_vectors() {
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/native-admission-vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 39);
    for vector in vectors["cases"].as_array().unwrap() {
        let before = vector["candidate"].clone();
        let loaded = load_native_ai_policy(&before);
        assert_eq!(
            loaded.is_ok(),
            vector["expectedNativeLoad"] == "accept",
            "{}: {loaded:?}",
            vector["id"]
        );
        assert_eq!(before, vector["candidate"]);
    }
}
#[test]
fn independent_policy_vectors() {
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/native-policy-vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 61);
    for vector in vectors["cases"].as_array().unwrap() {
        let mut workflow = candidate();
        workflow["impactLevel"] = vector
            .get("impactLevel")
            .cloned()
            .unwrap_or(json!("operational"));
        workflow["agents"][0]["deonticConstraints"] =
            vector.get("agentRules").unwrap_or(&vector["rules"]).clone();
        if let Some(rules) = vector.get("workflowRules") {
            workflow["aiOversight"]["deonticConstraints"] = rules.clone();
        }
        if let Some(floor) = vector.get("workflowFloor") {
            workflow["aiOversight"]["confidenceFloor"] = floor.clone();
        }
        let action_pointer = "/lifecycle/states/review/onEntry/0";
        if let Some(floor) = vector.get("actionFloor") {
            workflow["lifecycle"]["states"]["review"]["onEntry"] =
                json!([{"action":"emitEvent","event":"fixture.only"}]);
            workflow["agents"][0]["actionOverrides"] =
                json!([{"actionRef":action_pointer,"confidenceFloor":floor}]);
        }
        let policy = load_native_ai_policy(&workflow)
            .unwrap_or_else(|error| panic!("{}: {error}", vector["id"]))
            .unwrap();
        let empty = json!({});
        let context = PolicyExerciseContext {
            agent_id: "assessor",
            action_ref: (vector["trustedActionContext"] == true).then_some(action_pointer),
            input: vector.get("input"),
            case_file: vector.get("caseFile").unwrap_or(&empty),
            output: &vector["output"],
            confidence_report: vector.get("confidenceReport"),
            waiver: vector.get("waiver"),
            phase: if vector["phase"] == "input" {
                PolicyPhase::InputRights
            } else {
                PolicyPhase::Output
            },
        };
        let observed = evaluate_native_policy(&policy, &context);
        assert_eq!(
            observed.disposition(),
            vector["expected"].as_str().unwrap(),
            "{}: {observed:?}",
            vector["id"]
        );
        assert_eq!(policy.source(), &workflow);
    }
}
#[test]
fn source_navigation_preserves_invalid_drafts_and_exact_scopes() {
    let draft = json!({"aiOversight":{"deonticConstraints":[{"id":"shared","kind":"incomplete"}]},"agents":[{"deonticConstraints":[42],"actionOverrides":[{"deonticConstraints":[{"id":"site"}]}]}],"extensions":{"x-wos-ai":{"deonticConstraints":[{"id":"retired"}]}}});
    let sources = native_rule_sources(&draft);
    assert_eq!(
        sources
            .iter()
            .map(|s| s.pointer.as_str())
            .collect::<Vec<_>>(),
        vec![
            "/aiOversight/deonticConstraints/0",
            "/agents/0/deonticConstraints/0",
            "/agents/0/actionOverrides/0/deonticConstraints/0"
        ]
    );
    assert_eq!(sources[1].value, &json!(42));
}

#[test]
fn native_invoker_uses_literal_a2a_without_an_alias() {
    use wos_core::agent::InvokerSpec;
    let value = json!({"kind":"a2a","orchestrator":"fixture-orchestrator"});
    let invoker: InvokerSpec = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(invoker).unwrap(), value);
    assert!(
        serde_json::from_value::<InvokerSpec>(
            json!({"kind":"a2A","orchestrator":"fixture-orchestrator"})
        )
        .is_err()
    );
}

#[test]
fn trusted_operational_state_keeps_local_limits_separate_and_does_not_promote_on_clear() {
    use wos_core::autonomy::{AgentOperationalState, CalibrationEvidence, evaluate_autonomy};
    use wos_core::model::ai::AutonomyLevel::{Assistive, Autonomous, Manual};
    let mut source = candidate();
    source["aiOversight"]["defaultAutonomy"] = json!("autonomous");
    source["lifecycle"]["states"]["review"]["onEntry"] = json!([{"action":"emitEvent","event":"task.work"},{"action":"emitEvent","event":"task.other"}]);
    source["agents"][0]["actionOverrides"] =
        json!([{"actionRef":"/lifecycle/states/review/onEntry/0","autonomy":"manual"}]);
    let policy = load_native_ai_policy(&source).unwrap().unwrap();
    for reverse in [false, true] {
        let mut state = AgentOperationalState::new();
        for action in if reverse { [1, 0] } else { [0, 1] } {
            let pointer = format!("/lifecycle/states/review/onEntry/{action}");
            let observed =
                evaluate_autonomy(&policy, "assessor", Some(&pointer), None, &mut state, 10)
                    .unwrap();
            assert_eq!(
                observed.effective_level,
                if action == 0 { Manual } else { Autonomous }
            );
            assert_eq!(observed.retained_level, Autonomous);
        }
        let local = evaluate_autonomy(
            &policy,
            "assessor",
            Some("/lifecycle/states/review/onEntry/1"),
            Some(Assistive),
            &mut state,
            11,
        )
        .unwrap();
        assert_eq!(local.effective_level, Assistive);
        assert_eq!(local.retained_level, Autonomous);
        state.reduce("real-operational-reduction", Assistive, 12);
        state.clear_reduction("real-operational-reduction");
        assert_eq!(
            evaluate_autonomy(
                &policy,
                "assessor",
                Some("/lifecycle/states/review/onEntry/1"),
                None,
                &mut state,
                13
            )
            .unwrap()
            .effective_level,
            Assistive
        );
    }
    source["agents"][0]["calibration"] = json!({"required":true,"frequency":"P1D"});
    let policy = load_native_ai_policy(&source).unwrap().unwrap();
    let mut state = AgentOperationalState::new();
    state.calibration = Some(CalibrationEvidence {
        completed_at_ms: 1,
        expires_at_ms: 20,
    });
    assert_eq!(
        evaluate_autonomy(
            &policy,
            "assessor",
            Some("/lifecycle/states/review/onEntry/1"),
            None,
            &mut state,
            19
        )
        .unwrap()
        .effective_level,
        Autonomous
    );
    assert_eq!(
        evaluate_autonomy(
            &policy,
            "assessor",
            Some("/lifecycle/states/review/onEntry/1"),
            None,
            &mut state,
            20
        )
        .unwrap()
        .effective_level,
        Assistive
    );
    state.calibration = Some(CalibrationEvidence {
        completed_at_ms: 21,
        expires_at_ms: 40,
    });
    assert_eq!(
        evaluate_autonomy(
            &policy,
            "assessor",
            Some("/lifecycle/states/review/onEntry/1"),
            None,
            &mut state,
            21
        )
        .unwrap()
        .effective_level,
        Assistive,
        "fresh calibration alone cannot promote retained state"
    );
}

#[test]
fn report_port_refuses_invalid_scores_without_wrapping_or_defaulting() {
    use wos_core::model::ai::ConfidenceReport;
    for report in [
        json!(0.9),
        json!({"overall":0.9}),
        json!({"overall":0.9,"method":"invented"}),
        json!({"overall":1.1,"method":"declared"}),
        json!({"overall":0.9,"method":"declared","fieldLevel":{"amount":-0.1}}),
    ] {
        assert!(
            serde_json::from_value::<ConfidenceReport>(report.clone()).is_err(),
            "{report}"
        );
    }
    for method in [
        "modelNative",
        "calibrated",
        "heuristic",
        "conformal",
        "declared",
    ] {
        let report = json!({"overall":0.9,"method":method,"fieldLevel":{"amount":0.0}});
        let parsed: ConfidenceReport = serde_json::from_value(report.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), report);
    }
}

#[test]
fn approved_complete_workflow_examples_load_unchanged_as_drafts() {
    let documents = [
        include_str!("../../../specs/ai/ai-integration.md"),
        include_str!("../../../specs/ai/agent-config.md"),
        include_str!("../../../specs/ai/drift-monitor.md"),
    ];
    let mut count = 0;
    for document in documents {
        for block in document.split("```json").skip(1) {
            let source = block.split("```").next().unwrap();
            let value: Value = serde_json::from_str(source).unwrap();
            if value.get("$wosWorkflow").is_none() || value.get("lifecycle").is_none() {
                continue;
            }
            let before = value.clone();
            let policy = load_native_ai_policy(&value)
                .unwrap_or_else(|error| panic!("normative Workflow example: {error}"))
                .unwrap();
            assert_eq!(policy.source(), &before);
            assert_eq!(value, before);
            count += 1;
        }
    }
    assert_eq!(
        count, 2,
        "complete examples across the three expert-approved specifications"
    );
}

#[test]
fn report_omission_and_empty_field_map_differ_from_explicit_null() {
    use wos_core::model::ai::ConfidenceReport;
    for report in [
        json!({"overall":0.9,"method":"declared","explanation":null}),
        json!({"overall":0.9,"method":"declared","fieldLevel":null}),
    ] {
        assert!(
            serde_json::from_value::<ConfidenceReport>(report.clone()).is_err(),
            "{report}"
        );
    }
    for report in [
        json!({"overall":0.9,"method":"declared"}),
        json!({"overall":0.9,"method":"declared","explanation":"","fieldLevel":{}}),
    ] {
        let native: ConfidenceReport = serde_json::from_value(report.clone()).unwrap();
        assert_eq!(serde_json::to_value(native).unwrap(), report);
    }
}

#[test]
fn independent_native_presence_vectors_preserve_omission_and_metadata_null() {
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/native-presence-vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 23);
    for vector in vectors["cases"].as_array().unwrap() {
        let original = vector["candidate"].clone();
        let observed = load_native_ai_policy(&original);
        assert_eq!(
            observed.is_ok(),
            vector["expectedNativeLoad"] == "accept",
            "{}: {observed:?}",
            vector["id"]
        );
        if let Ok(Some(policy)) = observed {
            assert_eq!(policy.source(), &original, "{}", vector["id"]);
        }
        assert_eq!(original, vector["candidate"]);
    }
}

#[test]
fn duplicate_task_labels_select_the_exact_native_human_and_configuration() {
    use wos_core::model::ai::FallbackStep;
    let vectors: Value =
        serde_json::from_str(include_str!("fixtures/native-task-selection.json")).unwrap();
    for choice in vectors["choices"].as_array().unwrap() {
        let mut source = vectors["candidate"].clone();
        source["agents"][0]["fallbackChain"] =
            json!([{"action":"escalateToHuman","taskActionRef":choice["pointer"]}]);
        let before = source.clone();
        let policy = load_native_ai_policy(&source).unwrap().unwrap();
        let selected = match &policy
            .agent("assessor")
            .unwrap()
            .fallback_chain
            .as_ref()
            .unwrap()[0]
        {
            FallbackStep::EscalateToHuman {
                task_action_ref, ..
            } => policy.action(task_action_ref).unwrap(),
            other => panic!("expected literal human fallback selection, got {other:?}"),
        };
        assert_eq!(selected, &choice["expectedAction"], "{}", choice["pointer"]);
        assert_eq!(policy.source(), &before);
        assert_eq!(
            source, before,
            "selection is read-only and executes no lifecycle/task operation"
        );
    }
    assert_ne!(
        vectors["choices"][0]["expectedAction"],
        vectors["choices"][1]["expectedAction"]
    );
}

#[test]
fn trusted_approval_calculation_restores_only_with_matching_unexpired_newer_facts() {
    use wos_core::autonomy::{AgentOperationalState, ApprovalEvidence, evaluate_autonomy};
    use wos_core::model::ai::AutonomyLevel::{Assistive, Autonomous, Manual};
    let mut source = candidate();
    source["aiOversight"]["defaultAutonomy"] = json!("autonomous");
    source["agents"][0]["autonomyPolicy"] = json!({"maxAutonomy":"autonomous","escalation":[{"id":"restore","condition":"true","targetLevel":"autonomous","requiredApproverRef":"reviewer","escalationExpiry":"PT1H"}]});
    let policy = load_native_ai_policy(&source).unwrap().unwrap();
    let mut retained = AgentOperationalState::new();
    assert_eq!(
        evaluate_autonomy(&policy, "assessor", None, None, &mut retained, 0)
            .unwrap()
            .effective_level,
        Autonomous
    );
    retained.reduce("established-reduction", Assistive, 100);
    retained.clear_reduction("established-reduction");
    // These are explicit caller-established facts. This calculation neither
    // authenticates approval nor evaluates predicates/derives its lifetime.
    let approval = ApprovalEvidence {
        workflow_url: "urn:test:native-policy".into(),
        workflow_version: "1.0.0".into(),
        agent_id: "assessor".into(),
        rule_pointer: "/agents/0/autonomyPolicy/escalation/0".into(),
        approver_ref: "reviewer".into(),
        approved_at_ms: 200,
        expires_at_ms: 1200,
    };
    let mut approved = retained.clone();
    approved.approval = Some(approval.clone());
    let observed = evaluate_autonomy(&policy, "assessor", None, None, &mut approved, 201).unwrap();
    assert_eq!(observed.effective_level, Autonomous);
    assert_eq!(observed.retained_level, Assistive);
    assert!(observed.approval_active);
    let expired = evaluate_autonomy(&policy, "assessor", None, None, &mut approved, 1200).unwrap();
    assert_eq!(expired.effective_level, Assistive);
    assert!(!expired.approval_active);
    for invalid in [
        ApprovalEvidence {
            workflow_version: "2.0.0".into(),
            ..approval.clone()
        },
        ApprovalEvidence {
            approver_ref: "forged-reviewer".into(),
            ..approval.clone()
        },
    ] {
        let mut state = retained.clone();
        state.approval = Some(invalid);
        assert!(evaluate_autonomy(&policy, "assessor", None, None, &mut state, 201).is_err());
    }
    let mut demoted = retained;
    demoted.approval = Some(approval);
    demoted.reduce("newer-established-reduction", Manual, 300);
    demoted.clear_reduction("newer-established-reduction");
    let observed = evaluate_autonomy(&policy, "assessor", None, None, &mut demoted, 301).unwrap();
    assert_eq!(observed.effective_level, Manual);
    assert!(!observed.approval_active);
}
