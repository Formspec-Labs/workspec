// Rust guideline compliant 2026-02-21
//! Authors an immutable D1 sending-policy successor through the native authoring owner.
//!
//! Run with `ORIGINAL_DIRECTORY NEW_DIRECTORY`. Historical Definitions, mappings,
//! Delivery templates and signatures remain selected at their existing versions.
//! This recipe writes only Workflow, Rules and Integration after their selected
//! structural checks pass. It never supplies provider, delivery or replay proof.

use std::{collections::BTreeSet, io::Write, path::Path};

use serde_json::{Map, Value, json};
use wos_authoring::{WosProject, native_schema};
use wos_core::model::{
    governance_policy::{
        GovernancePolicyRules, GovernanceTransition, NoticeTimerBinding, analyze_governance_policy,
        rules_validator,
    },
    integration::{CallbackPhase, IntegrationProfileDocument},
    kernel::{ActionKind, KernelDocument, visit_actions, visit_states},
    notice_requirements::{NoticeAnchor, NoticeObservation, NoticeRequirement, NoticeScope},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// These are new asset identities, not versions of unchanged Formspec companions.
const ORIGINAL_VERSION: &str = "0.4.6";
const SUCCESSOR_VERSION: &str = "0.4.7";
const WORKFLOW: &str = "passport-governed.workflow.json";
const RULES: &str = "passport-governed.policy.json";
const INTEGRATION: &str = "integration.json";
const CASE_STATES: &str = "/lifecycle/states/caseAndDelivery/regions/case/states";

fn read(directory: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        directory.join(name),
    )?)?)
}

fn review_transitions(original: &Value, outside_pending: bool) -> Result<Value> {
    let transitions = original.as_array().ok_or("review transitions missing")?;
    let mut result = Vec::new();
    let mut denial = None;
    for transition in transitions {
        let mut next = transition.clone();
        match transition["target"].as_str() {
            Some("photoNoticePending") => {
                next["target"] = json!(if outside_pending {
                    "photoNoticeOutsidePending"
                } else {
                    "photoNoticeReady"
                });
            }
            Some("outsideDataMinimumPending" | "denialDecisionRecorded") if !outside_pending => {
                next["target"] = json!("denialDecisionRecorded");
                if let Some(first) = &mut denial {
                    let first: &mut Value = first;
                    let left = first["guard"]
                        .as_str()
                        .ok_or("first denial guard missing")?;
                    let right = next["guard"]
                        .as_str()
                        .ok_or("second denial guard missing")?;
                    let guard = format!("({left}) or ({right})");
                    let mut expected = first.clone();
                    expected["guard"] = next["guard"].clone();
                    if expected != next {
                        return Err(
                            "ready denial alternatives differ beyond their original guards".into(),
                        );
                    }
                    first["guard"] = json!(guard);
                } else {
                    denial = Some(next);
                }
                continue;
            }
            _ => {}
        }
        result.push(next);
    }
    if let Some(denial) = denial {
        result.push(denial);
    }
    Ok(Value::Array(result))
}

fn put_state(project: &mut WosProject, expected: &mut Value, id: &str, state: Value) -> Result<()> {
    let pointer = format!("{CASE_STATES}/{id}");
    project
        .put_state(pointer.clone(), state.clone())
        .map_err(|error| error.message)?;
    *expected
        .pointer_mut(&pointer)
        .ok_or("original case state missing")? = state;
    Ok(())
}

fn preserve_notice_context(project: &mut WosProject, expected: &mut Value) -> Result<()> {
    let original = expected
        .pointer(CASE_STATES)
        .ok_or("original case states missing")?
        .clone();
    let mut review = original["awaitingReview"].clone();
    // This entry is reached either with no outside-data notice or after its
    // original P10D dispute timer expired. Neither needs another minimum wait.
    review["transitions"] = review_transitions(&review["transitions"], false)?;
    put_state(project, expected, "awaitingReview", review)?;
    let mut disputed = original["awaitingDisputeReview"].clone();
    disputed["transitions"] = review_transitions(&disputed["transitions"], true)?;
    put_state(project, expected, "awaitingDisputeReview", disputed)?;

    for (id, ready, pending) in [
        (
            "photoNoticePending",
            "photoNoticeReady",
            "photoNoticeOutsidePending",
        ),
        (
            "awaitingPhoto",
            "awaitingPhotoReady",
            "awaitingPhotoOutsidePending",
        ),
        (
            "returnedReview",
            "returnedReviewReady",
            "returnedReviewOutsidePending",
        ),
    ] {
        let mut parent = original[id].clone();
        if parent["type"] != "atomic" {
            return Err(format!("original {id} must be atomic").into());
        }
        let mut children = Map::new();
        for (child, outside_pending) in [(ready, false), (pending, true)] {
            let transitions = match id {
                "photoNoticePending" => {
                    let mut transitions = original[id]["transitions"].clone();
                    transitions[0]["target"] = json!(if outside_pending {
                        "awaitingPhotoOutsidePending"
                    } else {
                        "awaitingPhotoReady"
                    });
                    transitions
                }
                "awaitingPhoto" => {
                    let mut correction = original[id]["transitions"][0].clone();
                    correction["target"] = json!(if outside_pending {
                        "returnedReviewOutsidePending"
                    } else {
                        "returnedReviewReady"
                    });
                    json!([correction])
                }
                _ => review_transitions(&original[id]["transitions"], outside_pending)?,
            };
            children.insert(
                child.into(),
                json!({"type":"atomic","transitions":transitions}),
            );
        }
        parent["type"] = json!("compound");
        parent["initialState"] = json!(ready);
        parent["states"] = Value::Object(children);
        // The existing state-owned P90D timer and its no-response transition
        // remain on their original parent; child completion routes inherit it.
        parent["transitions"] = if id == "awaitingPhoto" {
            json!([original[id]["transitions"][1].clone()])
        } else {
            json!([])
        };
        if parent["onEntry"] != original[id]["onEntry"] {
            return Err("shared parent entry actions changed".into());
        }
        put_state(project, expected, id, parent)?;
    }
    // Reopening occurs after an effective denial, or after the P90D photo
    // no-response period. The earlier P10D outside-data minimum is already over;
    // its existing returnedReview target selects the Ready initial child.
    Ok(())
}

fn main() -> Result<()> {
    let paths = std::env::args_os().skip(1).collect::<Vec<_>>();
    if paths.len() != 2 {
        return Err("usage: d1_notice_successor ORIGINAL_DIRECTORY NEW_DIRECTORY".into());
    }
    let original_directory = Path::new(&paths[0]);
    let output_directory = Path::new(&paths[1]);
    if output_directory.exists() {
        return Err(
            "successor directory already exists; historical assets are never overwritten".into(),
        );
    }
    let original = read(original_directory, WORKFLOW)?;
    if original["version"] != ORIGINAL_VERSION {
        return Err("this recipe requires the retained D1 0.4.6 Workflow".into());
    }
    let mut project =
        WosProject::from_native_document(original.clone()).map_err(|error| error.message)?;
    project
        .set_native_fields(Map::from_iter([(
            "version".into(),
            json!(SUCCESSOR_VERSION),
        )]))
        .map_err(|error| error.message)?;
    let mut expected = original;
    expected["version"] = json!(SUCCESSOR_VERSION);
    preserve_notice_context(&mut project, &mut expected)?;
    let workflow = project.native_snapshot();
    if workflow != expected {
        return Err("native authoring changed fields outside the selected version edit".into());
    }
    native_schema::validate_fragment("", &workflow).map_err(|error| error.message)?;
    let kernel: KernelDocument = serde_json::from_value(workflow.clone())?;

    let mut integration = read(original_directory, INTEGRATION)?;
    if integration["targetWorkflow"]["url"] != workflow["url"]
        || integration["targetWorkflow"]["compatibleVersions"] != format!("={ORIGINAL_VERSION}")
    {
        return Err("original Integration does not select the original Workflow exactly".into());
    }
    integration["version"] = json!(SUCCESSOR_VERSION);
    integration["targetWorkflow"]["compatibleVersions"] = json!(format!("={SUCCESSOR_VERSION}"));
    let selected_integration: IntegrationProfileDocument =
        serde_json::from_value(integration.clone())?;

    let mut rules = read(original_directory, RULES)?;
    if rules["version"] != 1 || rules.get("noticeRequirements").is_some() {
        return Err(
            "this recipe requires the retained Rules/1, without replacement requirements".into(),
        );
    }
    // These are the actual original notice-producing actions, not every callback
    // in Integration (which also includes an unused historical notice receiver).
    let mut outbound = BTreeSet::new();
    visit_actions(&kernel, |_, action| {
        if action.action == ActionKind::InvokeService
            && action.callback_phase == Some(CallbackPhase::Outbound)
            && let Some(binding) = action.service_ref.as_deref()
        {
            outbound.insert(binding.to_owned());
        }
    });
    let expected_outbound = BTreeSet::from(
        [
            "sendApprovalNotice",
            "sendDenialNotice",
            "sendInformationReceivedNotice",
            "sendLateEvidenceReceivedNotice",
            "sendOutsideDataNotice",
            "sendPhotoRequestNotice",
            "sendReopenedNotice",
        ]
        .map(str::to_owned),
    );
    if outbound != expected_outbound {
        return Err(
            "original D1 outbound action inventory differs; review the new source selection".into(),
        );
    }
    let requirements = outbound
        .into_iter()
        .map(|binding| {
            // Original photo/outside-data clocks and adverse/appeal consequences use
            // the immutable qualified sending time. Informational notices do not.
            let anchor = match binding.as_str() {
                "sendPhotoRequestNotice" | "sendOutsideDataNotice" | "sendDenialNotice" => {
                    NoticeAnchor::QualifiedObservation
                }
                _ => NoticeAnchor::None,
            };
            let requirement = NoticeRequirement {
                observation: NoticeObservation::TransportAccepted,
                scope: NoticeScope::SoftwareFixture,
                anchor,
            };
            Ok((binding, serde_json::to_value(requirement)?))
        })
        .collect::<Result<Map<String, Value>>>()?;
    rules["version"] = json!(2);
    rules["noticeRequirements"] = Value::Object(requirements);
    let mut decisions = Vec::new();
    for original in rules["humanDecisions"]
        .as_array()
        .ok_or("human decision Rules missing")?
    {
        let from = original["transition"]["from"]
            .as_str()
            .ok_or("decision source missing")?;
        let sources: &[&str] = if from == "returnedReview" {
            &["returnedReviewReady", "returnedReviewOutsidePending"]
        } else {
            &[from]
        };
        for &source in sources {
            let mut decision = original.clone();
            decision["transition"]["from"] = json!(source);
            if (source == "awaitingReview" || source == "returnedReviewReady")
                && decision["transition"]["to"] == "outsideDataMinimumPending"
            {
                decision["transition"]["to"] = json!("denialDecisionRecorded");
            }
            // The original two Ready denial guards are alternatives for the
            // same accepted human judgment, never two capability bindings.
            if !decisions.contains(&decision) {
                decisions.push(decision);
            }
        }
    }
    rules["humanDecisions"] = Value::Array(decisions);
    // Both periods already exist in the original Workflow. The dispute window
    // permits a response; the minimum period still runs after a timely response.
    // Neither is the photo rule's absence-of-response determination.
    let mut elapsed = Vec::new();
    for timer_id in ["outsideDataDispute", "outsideDataMinimumPeriod"] {
        let mut selected = Vec::new();
        visit_states(&kernel, |path, state| {
            for action in &state.on_entry {
                if action.action != ActionKind::StartTimer
                    || action.timer_id.as_deref() != Some(timer_id)
                {
                    continue;
                }
                for transition in &state.transitions {
                    if let Some(
                        event @ wos_core::model::kernel::TransitionEvent::Timer {
                            timer_id: consumed,
                            ..
                        },
                    ) = &transition.event
                        && consumed == timer_id
                        && transition.guard.is_none()
                        && let Some(duration) = &action.duration
                    {
                        selected.push(NoticeTimerBinding {
                            transition: GovernanceTransition {
                                from: path.rsplit('/').next().unwrap_or("").to_owned(),
                                to: transition.target.clone(),
                                event: event.runtime_dispatch_label(),
                            },
                            timer_id: timer_id.into(),
                            duration: duration.clone(),
                            request_notice_binding_id: "sendOutsideDataNotice".into(),
                        });
                    }
                }
            }
        });
        if selected.len() != 1 {
            return Err(
                format!("original elapsed timer {timer_id} is missing or ambiguous").into(),
            );
        }
        elapsed.extend(selected);
    }
    rules["noticeTimers"] = serde_json::to_value(elapsed)?;
    let schema_errors = rules_validator()
        .iter_errors(&rules)
        .map(|error| json!({"path":error.instance_path().to_string(), "message":error.to_string()}))
        .collect::<Vec<_>>();
    if !schema_errors.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"rulesSchemaErrors":schema_errors}))?
        );
        return Err("selected Rules/2 schema refused; no successor files written".into());
    }
    let selected_rules: GovernancePolicyRules = serde_json::from_value(rules.clone())?;
    // This is the same core association/graph owner used by validate_with_governance
    // and native qualification. It is deliberately not a second checker.
    let analysis = analyze_governance_policy(&kernel, &selected_rules, Some(&selected_integration));
    let passed =
        analysis.flow.gaps.is_empty() && !analysis.flow.issues.iter().any(|issue| issue.is_error());
    let report = json!({
        "version":1,
        "scope":"selected-workflow-rules-integration-structure",
        "passed":passed,
        "successorVersion":SUCCESSOR_VERSION,
        "diagnostics":analysis.flow.issues.iter().map(|issue| json!({
            "ruleId":issue.rule_id,"path":issue.path,"message":issue.message,
            "severity":format!("{:?}",issue.severity),
        })).collect::<Vec<_>>(),
        "coverageGaps":analysis.flow.gaps.iter().map(|gap| json!({
            "path":gap.path,"reason":gap.reason,
        })).collect::<Vec<_>>(),
        "limitations":[
            "No provider is installed or authenticated by this authoring recipe.",
            "No callback, recipient receipt, replay or hosted execution is qualified.",
            "Formspec companions and Delivery templates retain their original identities."
        ]
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !passed {
        return Err("selected structural owner refused; no successor files written".into());
    }
    std::fs::create_dir(output_directory)?;
    for (name, document) in [
        (WORKFLOW, workflow),
        (RULES, rules),
        (INTEGRATION, integration),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output_directory.join(name))?;
        file.write_all(serde_json::to_string_pretty(&document)?.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}
