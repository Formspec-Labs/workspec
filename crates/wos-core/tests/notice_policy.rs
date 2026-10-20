// Rust guideline compliant 2026-02-21
//! Shared original inputs exercise actual selected relationships and graph coverage.
use serde_json::{Value, json};
use wos_core::KernelDocument;
use wos_core::model::governance_policy::{
    GovernancePolicyRules, analyze_governance_policy, rules_validator,
};
use wos_core::model::integration::IntegrationProfileDocument;
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/notice_policy.json")).unwrap()
}
fn issues(raw: &Value) -> Vec<String> {
    let kernel: KernelDocument = serde_json::from_value(raw["kernel"].clone()).unwrap();
    let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    let profile: Option<IntegrationProfileDocument> =
        match serde_json::from_value(raw["integration"].clone()) {
            Ok(p) => p,
            Err(e) => return vec![format!("/integration: {e}")],
        };
    let result = analyze_governance_policy(&kernel, &rules, profile.as_ref());
    result
        .flow
        .issues
        .iter()
        .filter(|i| i.is_error())
        .map(|i| format!("{} {} {}", i.rule_id, i.path, i.message))
        .chain(
            result
                .flow
                .gaps
                .iter()
                .map(|g| format!("WOS-FLOW-005 {}", g.path)),
        )
        .collect()
}
#[test]
fn selected_notice_original_inputs_resolve_with_one_graph() {
    for method in [
        "transportAccepted",
        "providerDelivered",
        "openObserved",
        "recipientAcknowledged",
    ] {
        let mut raw = fixture();
        for requirement in raw["rules"]["noticeRequirements"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            requirement["observation"] = json!(method);
        }
        assert!(rules_validator().is_valid(&raw["rules"]), "{method}");
        assert!(issues(&raw).is_empty(), "{method}: {:?}", issues(&raw));
    }
}
#[test]
fn selected_notice_missing_and_foreign_associations_refuse() {
    for mode in [
        "integration",
        "producer",
        "inbound",
        "event",
        "binding",
        "timer",
        "timer-anchor",
        "continuation",
        "requirement",
        "method",
        "version",
        "opaque-timer",
    ] {
        let mut raw = fixture();
        match mode {
            "integration" => raw["integration"] = Value::Null,
            "producer" => {
                raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([])
            }
            "inbound" => {
                raw["kernel"]["lifecycle"]["states"]["sendRequest"]["transitions"][0]["actions"]
                    [0]["callbackPhase"] = json!("outbound")
            }
            "event" => {
                raw["integration"]["bindings"]["request"]
                    .as_object_mut()
                    .unwrap()
                    .remove("callbackEventType");
            }
            "binding" => {
                raw["integration"]["bindings"]
                    .as_object_mut()
                    .unwrap()
                    .remove("request");
            }
            "timer" => raw["rules"]["timeoutDecisions"][0]["timerId"] = json!("foreign"),
            "timer-anchor" => {
                raw["integration"]["bindings"]["request"]["outputBinding"] =
                    json!({"caseFile.foreign":"$.sentAt"})
            }
            "continuation" => {
                raw["rules"]["dueProcess"]["effectiveTransitions"][0]["event"] = json!("unrelated")
            }
            "requirement" => {
                raw["rules"]["noticeRequirements"]
                    .as_object_mut()
                    .unwrap()
                    .remove("request");
            }
            "method" => raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none"),
            "version" => {
                raw["integration"]["targetWorkflow"]["compatibleVersions"] = json!("2.0.0")
            }
            "opaque-timer" => {
                raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
                    json!("if true then caseFile.requestSentAt else now()")
            }
            _ => unreachable!(),
        }
        assert!(!issues(&raw).is_empty(), "{mode}");
    }
}
#[test]
fn selected_notice_graph_refuses_after_wait_producer_dead_end_and_cycle() {
    for mode in [
        "after-wait",
        "dead-end",
        "cycle",
        "malformed-final",
        "missing-target",
        "malformed-parallel",
        "malformed-compound",
    ] {
        let mut raw = fixture();
        let states = &mut raw["kernel"]["lifecycle"]["states"];
        match mode {
            "after-wait" => {
                let action = states["sendRequest"]["onEntry"].take();
                states["sendRequest"]["onEntry"] = json!([]);
                states["correction"]["onEntry"]
                    .as_array_mut()
                    .unwrap()
                    .extend(action.as_array().unwrap().iter().cloned());
            }
            "dead-end" => states["done"] = json!({"type":"atomic"}),
            "cycle" => {
                states["done"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"loop"},"target":"done"}]})
            }
            "malformed-final" => {
                states["done"] = json!({"type":"final","transitions":[{"target":"done"}]})
            }
            "missing-target" => states["sendRequest"]["transitions"][0]["target"] = json!("absent"),
            "malformed-parallel" => states["done"] = json!({"type":"parallel"}),
            "malformed-compound" => {
                states["done"] = json!({"type":"compound","initialState":"missing","states":{"child":{"type":"final"}}})
            }
            _ => unreachable!(),
        }
        assert!(!issues(&raw).is_empty(), "{mode}");
    }
}
#[test]
fn closed_rules_schema_rejects_unknown_partial_and_legacy_original_inputs() {
    for mode in [
        "version",
        "missing-map",
        "extra",
        "partial",
        "unknown-method",
        "wrong-hash",
    ] {
        let mut raw = fixture()["rules"].clone();
        match mode {
            "version" => raw["version"] = json!(1),
            "missing-map" => {
                raw.as_object_mut().unwrap().remove("noticeRequirements");
            }
            "extra" => raw["passingGraph"] = json!(true),
            "partial" => {
                raw["noticeRequirements"]["request"]
                    .as_object_mut()
                    .unwrap()
                    .remove("anchor");
            }
            "unknown-method" => {
                raw["noticeRequirements"]["request"]["observation"] =
                    json!("fixtureQualifiedReceipt")
            }
            "wrong-hash" => {
                raw["humanDecisions"][0]["evidenceMapping"] =
                    json!({"ref":"urn:test:mapping","sha256":"unbound"})
            }
            _ => unreachable!(),
        }
        assert!(!rules_validator().is_valid(&raw), "{mode}");
    }
}
#[test]
fn direct_typed_integration_cannot_bypass_missing_callback_event() {
    let raw = fixture();
    let kernel: KernelDocument = serde_json::from_value(raw["kernel"].clone()).unwrap();
    let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    let mut profile: IntegrationProfileDocument =
        serde_json::from_value(raw["integration"].clone()).unwrap();
    profile
        .bindings
        .get_mut("request")
        .unwrap()
        .extensions
        .remove("callbackEventType");
    let result = analyze_governance_policy(&kernel, &rules, Some(&profile));
    assert!(
        result
            .flow
            .issues
            .iter()
            .any(|i| i.is_error() && i.path.contains("request"))
    );
}
fn disjoint_branches(transition_producer: bool) -> Value {
    let mut raw = fixture();
    let action = raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"].take();
    raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
    raw["kernel"]["lifecycle"]["initialState"] = json!("choice");
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    if transition_producer {
        states["choice"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"send"},"target":"done","actions":action},{"event":{"kind":"message","name":"wait"},"target":"sendRequest"}]});
    } else {
        states["choice"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"send"},"target":"sendBranch"},{"event":{"kind":"message","name":"wait"},"target":"sendRequest"}]});
        states["sendBranch"] = json!({"type":"atomic","onEntry":action,"transitions":[{"event":{"kind":"message","name":"finished"},"target":"done"}]});
    }
    raw
}
#[test]
fn selected_notice_disjoint_entry_producer_and_wait_refuse() {
    let actual = issues(&disjoint_branches(false));
    assert!(
        actual.iter().any(|i| i.contains("NOTICE-GRAPH-006")),
        "{actual:?}"
    );
}
#[test]
fn selected_notice_disjoint_transition_send_and_wait_edges_refuse() {
    let actual = issues(&disjoint_branches(true));
    assert!(
        actual.iter().any(|i| i.contains("NOTICE-GRAPH-006")),
        "{actual:?}"
    );
}
#[test]
fn selected_notice_duplicate_actions_are_not_collapsed_into_one_association() {
    for outbound in [true, false] {
        let mut raw = fixture();
        let actions = if outbound {
            &mut raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"]
        } else {
            &mut raw["kernel"]["lifecycle"]["states"]["sendRequest"]["transitions"][0]["actions"]
        };
        let duplicate = actions[0].clone();
        actions.as_array_mut().unwrap().push(duplicate);
        let actual = issues(&raw);
        assert!(
            actual.iter().any(|i| i.contains("NOTICE-GRAPH-001")),
            "{outbound}: {actual:?}"
        );
    }
}
#[test]
fn actual_notice_clock_cannot_be_hidden_by_omitting_timeout_rules() {
    let mut raw = fixture();
    raw["rules"]["timeoutDecisions"] = json!([]);
    raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
    let actual = issues(&raw);
    assert!(
        !actual.is_empty(),
        "ordinary status hid an actual notice-derived timer"
    );
}
#[test]
fn unrelated_authored_timer_origin_remains_outside_notice_classification() {
    let mut raw = fixture();
    raw["rules"]["timeoutDecisions"] = json!([]);
    raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
    raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
        json!("caseFile.unrelatedIssuedAt");
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
}
#[test]
fn explicit_adverse_grace_timer_and_original_anchor_resolve() {
    let mut raw = fixture();
    raw["kernel"]["governance"]["dueProcess"]["adverseDecisionPolicy"]["noticeGracePeriod"] =
        json!("P1D");
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    states["proposal"]["transitions"][0]["target"] = json!("adverseObserved");
    states["proposal"]["transitions"][0]["actions"] = json!([{"action":"invokeService","serviceRef":"adverse","callbackPhase":"inbound"},{"action":"emitEvent","eventType":"grace.start"}]);
    states["adverseObserved"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"grace.start"},"target":"graceWait"}]});
    states["graceWait"] = json!({"type":"atomic","onEntry":[{"action":"startTimer","timerId":"adverseGrace","duration":"P1D","startAtExpression":"caseFile.adverseSentAt"}],"transitions":[{"event":{"kind":"timer","timerId":"adverseGrace","source":"custom"},"target":"closed","actions":[{"action":"setData","path":"caseFile.caseClosed","value":true}]}]});
    raw["rules"]["dueProcess"]["effectiveTransitions"] =
        json!([{"from":"graceWait","to":"closed","event":"$timeout.adverseGrace"}]);
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
    raw["kernel"]["lifecycle"]["states"]["graceWait"]["onEntry"][0]["duration"] = json!("P2D");
    assert!(!issues(&raw).is_empty());
}
#[test]
fn a_real_outbound_transition_can_lead_to_its_callback_consumer() {
    let mut raw = disjoint_branches(true);
    raw["kernel"]["lifecycle"]["states"]["choice"]["transitions"][0]["target"] =
        json!("sendRequest");
    raw["kernel"]["lifecycle"]["states"]["choice"]["transitions"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
}
#[test]
fn timer_start_and_selected_timeout_on_disjoint_branches_refuse() {
    let mut raw = fixture();
    raw["kernel"]["lifecycle"]["initialState"] = json!("choice");
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    let timer = states["correction"]["transitions"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    states["choice"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"request"},"target":"sendRequest"},{"event":{"kind":"message","name":"skip"},"target":"timeoutElsewhere"}]});
    states["timeoutElsewhere"] = json!({"type":"atomic","transitions":[timer]});
    raw["rules"]["timeoutDecisions"][0]["transition"]["from"] = json!("timeoutElsewhere");
    assert!(
        !issues(&raw).is_empty(),
        "disjoint timer start/consumer was qualified"
    );
}
#[test]
fn callback_emit_and_protected_effect_on_disjoint_branches_refuse() {
    let mut raw = fixture();
    raw["kernel"]["lifecycle"]["initialState"] = json!("choice");
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    states["choice"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"request"},"target":"sendRequest"},{"event":{"kind":"message","name":"skip"},"target":"effectElsewhere"}]});
    states["proposal"]["transitions"][0]["target"] = json!("callbackDone");
    states["proposal"]["transitions"][0]["actions"] = json!([{"action":"invokeService","serviceRef":"adverse","callbackPhase":"inbound"},{"action":"emitEvent","eventType":"effect.ready"}]);
    states["callbackDone"] = json!({"type":"final"});
    states["effectElsewhere"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"effect.ready"},"target":"closed"}]});
    raw["rules"]["dueProcess"]["effectiveTransitions"] =
        json!([{"from":"effectElsewhere","to":"closed","event":"effect.ready"}]);
    assert!(
        !issues(&raw).is_empty(),
        "disjoint emit/protected consumer was qualified"
    );
}
#[test]
fn a_good_send_path_does_not_hide_a_reachable_direct_wait_bypass() {
    let mut raw = disjoint_branches(false);
    raw["kernel"]["lifecycle"]["states"]["sendBranch"]["transitions"][0]["target"] =
        json!("sendRequest");
    assert!(
        !issues(&raw).is_empty(),
        "one good route hid an unproduced callback wait"
    );
}
#[test]
fn a_good_callback_route_does_not_hide_a_stuck_protected_event_branch() {
    let mut raw = fixture();
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    states["proposal"]["transitions"][0]["target"] = json!("effectWait");
    states["proposal"]["transitions"][0]["actions"] = json!([{"action":"invokeService","serviceRef":"adverse","callbackPhase":"inbound"},{"action":"emitEvent","eventType":"effect.ready"}]);
    states["proposal"]["transitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"event":{"kind":"message","name":"skip.callback"},"target":"effectWait"}));
    states["effectWait"] = json!({"type":"atomic","transitions":[{"event":{"kind":"message","name":"effect.ready"},"target":"closed"}]});
    raw["rules"]["dueProcess"]["effectiveTransitions"] =
        json!([{"from":"effectWait","to":"closed","event":"effect.ready"}]);
    assert!(
        !issues(&raw).is_empty(),
        "good callback route hid an event wait that cannot obtain its selected event"
    );
}
fn permanent_receipt_fixture() -> Value {
    let source = fixture();
    let mut decision = source["rules"]["humanDecisions"][0].clone();
    decision["transition"] = json!({"from":"review","to":"sending","event":"review.completed"});
    let raw = json!({"kernel":{"$wosWorkflow":"1.0","url":"urn:test:persistent-notice","version":"1.0.0","actors":[{"id":"reviewer","type":"human"}],"governance":{"audit":{"reasoningTier":{"requiredForTags":["determination"]}}},"lifecycle":{"initialState":"running","states":{"running":{"type":"parallel","regions":{"business":{"initialState":"review","states":{"review":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"review.completed"},"target":"sending","tags":["determination"]}]},"sending":{"type":"atomic","onEntry":[{"action":"invokeService","serviceRef":"request","callbackPhase":"outbound"}],"transitions":[{"event":{"kind":"message","name":"business.done"},"target":"done"}]},"done":{"type":"final"}}},"receipts":{"initialState":"receiptListener","states":{"receiptListener":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"receipt"},"target":"receiptListener","actions":[{"action":"invokeService","serviceRef":"request","callbackPhase":"inbound"},{"action":"emitEvent","eventType":"business.done"}]}]}}}}}}}},"rules":{"version":2,"humanDecisions":[decision],"noticeRequirements":{"request":{"observation":"transportAccepted","scope":"softwareFixture","anchor":"none"}}},"integration":{"targetWorkflow":{"url":"urn:test:persistent-notice"},"bindings":{"request":{"type":"callback","dispatchMode":"deferred","eventType":"urn:test:request","callbackEventType":"urn:test:accepted"}}}});
    raw
}

#[test]
fn explicit_permanent_receipt_region_can_register_before_business_send() {
    assert!(issues(&permanent_receipt_fixture()).is_empty());
}
#[test]
fn permanent_listener_cannot_start_business_work_before_its_request() {
    let mut raw = permanent_receipt_fixture();
    raw["kernel"]["lifecycle"]["states"]["running"]["regions"]["receipts"]["states"]["receiptListener"]
        ["onEntry"] = json!([{"action":"startTimer","timerId":"business","duration":"P1D"}]);
    assert!(!issues(&raw).is_empty());
}
#[test]
fn selected_recursive_visitors_refuse_resource_bounds_before_walking() {
    let raw = fixture();
    let mut kernel: KernelDocument = serde_json::from_value(raw["kernel"].clone()).unwrap();
    let rules: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    let profile: IntegrationProfileDocument =
        serde_json::from_value(raw["integration"].clone()).unwrap();
    let mut nested =
        serde_json::from_value::<wos_core::model::kernel::State>(json!({"type":"final"})).unwrap();
    for i in 0..130 {
        let name = format!("nested{i}");
        let mut parent = serde_json::from_value::<wos_core::model::kernel::State>(
            json!({"type":"compound","initialState":name}),
        )
        .unwrap();
        parent.states.insert(name, nested);
        nested = parent;
    }
    kernel.lifecycle.states.insert("bounded".into(), nested);
    let result = analyze_governance_policy(&kernel, &rules, Some(&profile));
    assert!(!result.flow.gaps.is_empty());
    assert!(result.notices.is_empty());
}
#[test]
fn callback_output_timer_cannot_run_in_the_mapping_evaluation() {
    for (on_transition, nested) in [(true, false), (false, false), (true, true), (false, true)] {
        let mut raw = if nested {
            nested_clock_fixture()
        } else {
            fixture()
        };
        let states = &mut raw["kernel"]["lifecycle"]["states"];
        states.as_object_mut().unwrap().remove("requestObserved");
        states["sendRequest"]["transitions"][0]["target"] = json!("correction");
        states["sendRequest"]["transitions"][0]["actions"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        if on_transition {
            let timer = states["correction"]["onEntry"]
                .as_array_mut()
                .unwrap()
                .remove(0);
            states["sendRequest"]["transitions"][0]["actions"]
                .as_array_mut()
                .unwrap()
                .push(timer);
        }
        assert!(
            !issues(&raw).is_empty(),
            "same-evaluation clock was qualified: transition={on_transition}, nested={nested}"
        );
    }
}

fn nested_clock_fixture() -> Value {
    let mut raw = fixture();
    raw["integration"]["bindings"]["request"]["outputBinding"] =
        json!({"caseFile.requestNotice":"$"});
    raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
        json!("caseFile.requestNotice.clock.governedSentAt");
    raw
}
#[test]
fn nested_clock_whole_callback_has_the_exact_governed_anchor() {
    let raw = nested_clock_fixture();
    assert!(rules_validator().is_valid(&raw["rules"]));
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
}
#[test]
fn nested_clock_descendants_cannot_hide_missing_timeout_rules() {
    for origin in [
        "caseFile.requestNotice.clock.governedSentAt",
        "caseFile.requestNotice.other",
        "caseFile.requestNotice[0].clock.governedSentAt",
        "caseFile.requestNotice[*].clock.governedSentAt",
    ] {
        assert!(fel_core::parser::parse(origin).is_ok(), "{origin}");
        let mut raw = nested_clock_fixture();
        raw["rules"]["timeoutDecisions"] = json!([]);
        raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
        raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
            json!(origin);
        let errors = issues(&raw);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("/noticeRequirements/request/anchor")),
            "derived clock fell through to ordinary status: {origin}: {errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("no original Rules timeout")),
            "missing original Rules escaped: {origin}: {errors:?}"
        );
    }
}
#[test]
fn nested_clock_mapping_direction_and_exact_child_are_not_inferred() {
    for (source, origin) in [
        ("$.clock", "caseFile.requestNotice.clock.governedSentAt"),
        ("$.other", "caseFile.requestNotice.clock.governedSentAt"),
        ("$", "caseFile.requestNotice.sentAt"),
        ("$", "caseFile.requestNotice.clock.other"),
        ("$", "caseFile.requestNotice[0].clock.governedSentAt"),
        ("$", "caseFile.requestNotice[*].clock.governedSentAt"),
        ("$", "caseFile.requestNoticeSibling.clock.governedSentAt"),
    ] {
        let mut raw = nested_clock_fixture();
        raw["integration"]["bindings"]["request"]["outputBinding"]["caseFile.requestNotice"] =
            json!(source);
        raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
            json!(origin);
        assert!(
            !issues(&raw).is_empty(),
            "inferred invalid anchor: {source}: {origin}"
        );
    }
}
#[test]
fn nested_clock_sibling_prefix_is_an_unrelated_ordinary_timer() {
    let mut raw = nested_clock_fixture();
    raw["rules"]["timeoutDecisions"] = json!([]);
    raw["rules"]["noticeRequirements"]["request"]["anchor"] = json!("none");
    raw["kernel"]["lifecycle"]["states"]["correction"]["onEntry"][0]["startAtExpression"] =
        json!("caseFile.requestNoticeSibling.clock.governedSentAt");
    assert!(
        issues(&raw).is_empty(),
        "unrelated timer changed: {:?}",
        issues(&raw)
    );
}

fn alternative_producer_fixture() -> Value {
    let mut raw = fixture();
    let action = raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"].take();
    raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"] = json!([]);
    raw["kernel"]["lifecycle"]["initialState"] = json!("noticeChoice");
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    states["noticeChoice"] = json!({"type":"atomic","transitions":[
        {"event":{"kind":"message","name":"first"},"target":"firstSend"},
        {"event":{"kind":"message","name":"second"},"target":"secondSend"}]});
    for name in ["firstSend", "secondSend"] {
        states[name] = json!({"type":"atomic","onEntry":action.clone(),"transitions":[
            {"event":{"kind":"message","name":"wait"},"target":"sendRequest"}]});
    }
    raw
}
#[test]
fn distinct_alternative_producers_share_one_exact_consumer() {
    let raw = alternative_producer_fixture();
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
}
#[test]
fn alternative_producer_cannot_hide_a_send_without_callback_or_an_unproduced_wait() {
    for target in ["done", "sendRequest"] {
        let mut raw = alternative_producer_fixture();
        if target == "done" {
            raw["kernel"]["lifecycle"]["states"]["firstSend"]["transitions"][0]["target"] =
                json!(target);
        } else {
            raw["kernel"]["lifecycle"]["states"]["noticeChoice"]["transitions"]
                .as_array_mut()
                .unwrap()
                .push(json!({"event":{"kind":"message","name":"skip"},"target":target}));
        }
        assert!(
            !issues(&raw).is_empty(),
            "{target}: a good producer hid a bad branch"
        );
    }
}
fn elapsed_notice_fixture() -> Value {
    let mut raw = fixture();
    raw["rules"]["timeoutDecisions"] = json!([]);
    raw["rules"]["noticeTimers"] = json!([{"transition":{"from":"minimumWait","to":"proposal","event":"$timeout.response"},
        "timerId":"response","duration":"P90D","requestNoticeBindingId":"request"}]);
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    let timer = states["correction"]["onEntry"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    states["correction"]["transitions"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    states["review"]["transitions"][0]["target"] = json!("minimumWait");
    states["minimumWait"] = json!({"type":"atomic","onEntry":[timer],"transitions":[
        {"event":{"kind":"timer","timerId":"response","source":"custom"},"target":"proposal"}]});
    raw["rules"]["humanDecisions"][0]["transition"]["to"] = json!("minimumWait");
    raw
}
#[test]
fn explicit_elapsed_period_can_start_after_a_timely_response_and_review() {
    let raw = elapsed_notice_fixture();
    assert!(rules_validator().is_valid(&raw["rules"]));
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
    let typed: GovernancePolicyRules = serde_json::from_value(raw["rules"].clone()).unwrap();
    assert!(typed.timeout_decisions.is_empty());
}
#[test]
fn elapsed_notice_period_requires_exact_unique_original_association() {
    for mode in [
        "missing",
        "duration",
        "duplicate",
        "overlap",
        "foreign-binding",
        "physical-anchor",
        "same-evaluation",
        "guarded",
    ] {
        let mut raw = elapsed_notice_fixture();
        match mode {
            "missing" => raw["rules"]["noticeTimers"] = json!([]),
            "duration" => raw["rules"]["noticeTimers"][0]["duration"] = json!("P10D"),
            "duplicate" => {
                let duplicate = raw["rules"]["noticeTimers"][0].clone();
                raw["rules"]["noticeTimers"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            "overlap" => {
                let mut old = fixture()["rules"]["timeoutDecisions"][0].clone();
                old["transition"]["from"] = json!("minimumWait");
                raw["rules"]["timeoutDecisions"] = json!([old]);
            }
            "foreign-binding" => {
                raw["rules"]["noticeTimers"][0]["requestNoticeBindingId"] = json!("foreign")
            }
            "physical-anchor" => {
                raw["kernel"]["lifecycle"]["states"]["minimumWait"]["onEntry"][0]["startAtExpression"] =
                    json!("caseFile.requestSentAt.physical")
            }
            "same-evaluation" => {
                let timer = raw["kernel"]["lifecycle"]["states"]["minimumWait"]["onEntry"].take();
                raw["kernel"]["lifecycle"]["states"]["minimumWait"]["onEntry"] = json!([]);
                raw["kernel"]["lifecycle"]["states"]["requestObserved"]["onEntry"] = timer;
            }
            "guarded" => {
                raw["kernel"]["lifecycle"]["states"]["minimumWait"]["transitions"][0]["guard"] =
                    json!("true")
            }
            _ => unreachable!(),
        }
        assert!(
            !issues(&raw).is_empty(),
            "{mode}: invalid elapsed association passed"
        );
    }
}

#[test]
fn an_earlier_callback_cannot_hide_a_later_uncompletable_outbound_occurrence() {
    let mut raw = fixture();
    let action = raw["kernel"]["lifecycle"]["states"]["sendRequest"]["onEntry"].clone();
    raw["kernel"]["lifecycle"]["states"]["done"] = json!({"type":"atomic","onEntry":action,
        "transitions":[{"event":{"kind":"message","name":"finish"},"target":"finished"}]});
    raw["kernel"]["lifecycle"]["states"]["finished"] = json!({"type":"final"});
    assert!(
        !issues(&raw).is_empty(),
        "the first completed pair hid the later send with no callback route"
    );
}
#[test]
fn elapsed_timer_on_newly_entered_callback_ancestor_runs_before_mapping() {
    let mut raw = elapsed_notice_fixture();
    let states = &mut raw["kernel"]["lifecycle"]["states"];
    let timer = states["minimumWait"]["onEntry"].take();
    states["minimumWait"]["onEntry"] = json!([]);
    let wait = states
        .as_object_mut()
        .unwrap()
        .remove("minimumWait")
        .unwrap();
    states["minimumScope"] = json!({"type":"compound","initialState":"afterCallback","onEntry":timer,
        "states":{"afterCallback":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"later"},"target":"requestObserved"}]},"minimumWait":wait}});
    states["sendRequest"]["transitions"][0]["target"] = json!("afterCallback");
    // The timer-consuming state is entered only on later reentry, so an
    // inactive-child prerequisite check alone cannot identify the first unsafe start.
    assert!(
        !issues(&raw).is_empty(),
        "newly entered ancestor clock ran before callback Mapping"
    );
}

#[test]
fn callback_in_parallel_receipt_region_starts_business_clock_on_later_emitted_event() {
    let mut raw = fixture();
    let mut business = raw["kernel"]["lifecycle"]["states"].take();
    let receipt = business["sendRequest"]["transitions"][0].clone();
    let mut inbound = receipt;
    inbound["target"] = json!("requestListener");
    business["sendRequest"]["transitions"] = json!([{
        "event":{"kind":"message","name":"request.periodStarts"},"target":"correction"
    }]);
    business.as_object_mut().unwrap().remove("requestObserved");
    raw["kernel"]["lifecycle"] = json!({"initialState":"running","states":{
        "running":{"type":"parallel","regions":{
            "business":{"initialState":"sendRequest","states":business},
            "receipts":{"initialState":"requestListener","states":{
                "requestListener":{"type":"atomic","transitions":[inbound]}
            }}
        }}
    }});
    assert!(issues(&raw).is_empty(), "{:?}", issues(&raw));
}

#[test]
fn explicit_context_child_preserves_shared_parent_entry_and_inherited_timer_owner() {
    for context in ["Ready", "OutsidePending"] {
        for timeout in [false, true] {
            let kernel: KernelDocument = serde_json::from_value(json!({
                "$wosWorkflow":"1.0","lifecycle":{"initialState":"start","states":{
                    "start":{"type":"atomic","transitions":[
                        {"event":{"kind":"message","name":"ready"},"target":"photoReady"},
                        {"event":{"kind":"message","name":"outside"},"target":"photoOutsidePending"}]},
                    "awaitingPhoto":{"type":"compound","initialState":"photoReady",
                        "onEntry":[{"action":"startTimer","timerId":"photoResponse","duration":"P90D","event":{"kind":"timer","timerId":"photoResponse","source":"state"}}],
                        "transitions":[{"event":{"kind":"timer","timerId":"photoResponse","source":"state"},"target":"closed"}],
                        "states":{
                            "photoReady":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"correction"},"target":"reviewReady","actions":[{"action":"cancelTimer","timerId":"photoResponse"}]}]},
                            "photoOutsidePending":{"type":"atomic","transitions":[{"event":{"kind":"message","name":"correction"},"target":"reviewOutsidePending","actions":[{"action":"cancelTimer","timerId":"photoResponse"}]}]}}},
                    "reviewReady":{"type":"final"},"reviewOutsidePending":{"type":"final"},"closed":{"type":"final"}
                }}
            })).unwrap();
            let mut evaluator =
                wos_core::Evaluator::new(kernel, &stack_common_typeid::testing::fixture_minter())
                    .unwrap();
            evaluator
                .process_event(
                    if context == "Ready" {
                        "ready"
                    } else {
                        "outside"
                    },
                    None,
                    None,
                )
                .unwrap();
            assert!(
                evaluator
                    .configuration()
                    .contains(&format!("photo{context}"))
            );
            assert_eq!(evaluator.timers().len(), 1);
            let timer = evaluator.timers().iter().next().unwrap();
            assert_eq!(timer.created_in_state, "awaitingPhoto");
            assert_eq!(timer.duration_iso, "P90D");
            if timeout {
                evaluator.advance_time(90 * 86_400_000, None).unwrap();
                assert!(evaluator.configuration().contains("closed"));
                assert_eq!(
                    evaluator.transitions().last().unwrap().from,
                    "awaitingPhoto"
                );
            } else {
                evaluator.process_event("correction", None, None).unwrap();
                assert!(
                    evaluator
                        .configuration()
                        .contains(&format!("review{context}"))
                );
                assert!(evaluator.timers().is_empty());
            }
        }
    }
}
