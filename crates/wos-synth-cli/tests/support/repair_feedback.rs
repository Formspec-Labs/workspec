//! Provider consumption of actual native checks, with frozen independent expectations.

use super::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use wos_synth_core::{
    CacheAnchor, CandidateAssessment, Completion, Prompter, PrompterError, ToolError,
};

/// Records the actual adapter's output without changing any native evidence.
struct RecordingTools {
    native: McpToolContext,
    assessments: Mutex<Vec<CandidateAssessment>>,
}

#[async_trait]
impl ToolContext for RecordingTools {
    async fn check_candidate(&self, document: &str) -> Result<CandidateAssessment, ToolError> {
        let assessment = self.native.check_candidate(document).await?;
        self.assessments.lock().unwrap().push(assessment.clone());
        Ok(assessment)
    }
}

struct InspectingPrompter<F> {
    initial: String,
    tools: Arc<RecordingTools>,
    repair: F,
}

#[async_trait]
impl<F> Prompter for InspectingPrompter<F>
where
    F: Fn(usize, &CandidateAssessment) -> String + Send + Sync,
{
    async fn complete(
        &self,
        system: &str,
        user: &str,
        _anchors: &[CacheAnchor],
    ) -> Result<Completion, PrompterError> {
        let assessments = self.tools.assessments.lock().unwrap();
        let text = if let Some(latest) = assessments.last() {
            let (_, feedback) = user
                .split_once("## Candidate assessment\n\n```json\n")
                .expect("repair provider must receive native assessment feedback");
            let (feedback, _) = feedback.split_once("\n```").unwrap();
            let feedback: Value = serde_json::from_str(feedback).unwrap();
            assert_eq!(
                feedback,
                serde_json::to_value(latest).unwrap(),
                "provider must see the exact latest native reports and frozen arguments"
            );
            assert!(system.contains("replay"));
            assert!(system.contains("expectations"));
            (self.repair)(
                assessments.len(),
                &serde_json::from_value(feedback).unwrap(),
            )
        } else {
            self.initial.clone()
        };
        Ok(Completion {
            text,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
        })
    }
}

fn reviewed_checks() -> Value {
    let mut fixture = scenario();
    fixture["id"] = json!("repair-feedback-control");
    fixture["event_sequence"][0]["data"] = json!({"applicationId":"independent-application"});
    configuration(vec![json!({
        "fixture_json": fixture.to_string(),
        "expected_observations": [{"pointer":"/finalSnapshot/configuration","expected":["done"]}]
    })])
}

fn guarded(guard: &str) -> String {
    let mut candidate = positive();
    candidate["lifecycle"]["states"]["review"]["transitions"][0]["guard"] = json!(guard);
    candidate.to_string()
}

fn recording_tools(checks: &Value) -> Arc<RecordingTools> {
    Arc::new(RecordingTools {
        native: adapter(checks, false),
        assessments: Mutex::new(vec![]),
    })
}

fn replay_input<'a>(report: &'a Value, role: &str) -> &'a Value {
    report["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["role"] == role)
        .unwrap()
}

fn assert_guard_failure(assessment: &CandidateAssessment) {
    assert!(
        assessment
            .findings
            .iter()
            .all(|finding| { matches!(finding.severity, Severity::Warning | Severity::Info) })
    );
    assert_eq!(assessment.disposition, CheckDisposition::Blocked);
    assert_eq!(
        completed(&assessment.validation.result)["validation"]["status"],
        "pass"
    );
    let replay = completed(&assessment.replays[0].result);
    assert_eq!(replay["status"], "fail");
    assert_eq!(replay["passed"], false);
    assert!(!replay["failures"].as_array().unwrap().is_empty());
    assert!(replay["differences"].as_array().unwrap().contains(&json!({
        "pointer":"/finalSnapshot/configuration","expected":["done"],"actual":["review"]
    })));
    assert_eq!(replay["trace"]["fixtureId"], "repair-feedback-control");
    let step = &replay["trace"]["steps"][0];
    assert_eq!(step["event"]["name"], "decide");
    assert_eq!(step["event"]["sourceActor"], "reviewer");
    assert_eq!(
        step["event"]["payload"]["applicationId"],
        "independent-application"
    );
    assert_eq!(step["stateAfter"], "review");
    assert_eq!(step["expectedStateAfter"], "done");
    assert_eq!(step["delta"]["kind"], "guardFalse");
    assert_eq!(step["delta"]["expression"], "false");
    assert!(!replay["identity"].as_str().unwrap().is_empty());
    let candidate = replay_input(replay, "kernel");
    assert_eq!(candidate["url"], positive()["url"]);
    assert_eq!(candidate["version"], positive()["version"]);
    for field in ["rawSha256", "canonicalSha256"] {
        assert_eq!(candidate[field].as_str().unwrap().len(), 64);
    }
}

#[tokio::test]
async fn repair_inspects_retained_native_failure_and_reexecutes_corrected_candidate() {
    let checks = reviewed_checks();
    let tools = recording_tools(&checks);
    let repaired = guarded("true");
    let provider = InspectingPrompter {
        initial: guarded("false"),
        tools: tools.clone(),
        repair: |attempt, assessment: &CandidateAssessment| {
            assert_eq!(attempt, 1);
            assert_guard_failure(assessment);
            repaired.clone()
        },
    };
    let outcome = synthesize(
        &provider,
        tools.as_ref(),
        "reviewed request",
        Layer::Kernel,
        2,
    )
    .await
    .unwrap();
    let SynthOutcome::Converged { document, trace } = outcome else {
        panic!("a repair informed by native feedback must converge");
    };
    assert_eq!(document, repaired);
    assert_eq!(trace.iterations.len(), 2);
    assert_eq!(tools.assessments.lock().unwrap().len(), 2);
    let first = &trace.iterations[0].assessment;
    let last = &trace.iterations[1].assessment;
    assert_guard_failure(first);
    let replay = completed(&last.replays[0].result);
    assert_eq!(replay["status"], "pass");
    assert_eq!(replay["finalSnapshot"]["configuration"], json!(["done"]));
    assert_ne!(
        completed(&first.replays[0].result)["identity"],
        replay["identity"]
    );
    assert_ne!(
        replay_input(completed(&first.replays[0].result), "kernel"),
        replay_input(replay, "kernel")
    );
    assert_eq!(
        replay_input(completed(&first.replays[0].result), "fixture"),
        replay_input(replay, "fixture")
    );
    assert_eq!(first.replays[0].arguments, last.replays[0].arguments);
    assert_eq!(
        last.replays[0].arguments["fixture_json"],
        checks["replays"][0]["arguments"]["fixture_json"]
    );
    assert_eq!(
        last.replays[0].arguments["expected_observations"],
        checks["replays"][0]["arguments"]["expected_observations"]
    );
    assert_trace_schema(&serde_json::to_value(trace).unwrap());
}

#[tokio::test]
async fn repair_uses_latest_failure_even_when_an_earlier_replay_passed() {
    let tools = recording_tools(&reviewed_checks());
    let mut initial = positive();
    initial.as_object_mut().unwrap().remove("title");
    let provider = InspectingPrompter {
        initial: initial.to_string(),
        tools: tools.clone(),
        repair: |attempt, assessment: &CandidateAssessment| {
            if attempt == 1 {
                assert_eq!(
                    completed(&assessment.validation.result)["validation"]["status"],
                    "fail"
                );
                assert_eq!(completed(&assessment.replays[0].result)["passed"], true);
            } else {
                assert_eq!(attempt, 2);
                assert_guard_failure(assessment);
            }
            guarded("false")
        },
    };
    let outcome = synthesize(
        &provider,
        tools.as_ref(),
        "reviewed request",
        Layer::Kernel,
        3,
    )
    .await
    .unwrap();
    let SynthOutcome::Unconverged {
        last_attempt,
        trace,
        ..
    } = outcome
    else {
        panic!("an earlier replay pass cannot hide the latest failed replay");
    };
    assert_eq!(last_attempt, guarded("false"));
    assert_eq!(trace.iterations.len(), 3);
    assert_eq!(
        completed(&trace.iterations[0].assessment.replays[0].result)["passed"],
        true
    );
    for iteration in &trace.iterations[1..] {
        assert_guard_failure(&iteration.assessment);
        assert_eq!(
            iteration.assessment.replays[0].arguments,
            trace.iterations[0].assessment.replays[0].arguments
        );
    }
    assert_trace_schema(&serde_json::to_value(trace).unwrap());
}

#[tokio::test]
async fn repair_receives_actual_intake_error_and_not_run_reasons() {
    let tools = recording_tools(&reviewed_checks());
    let provider = InspectingPrompter {
        initial: "{ not JSON".into(),
        tools: tools.clone(),
        repair: |attempt, assessment: &CandidateAssessment| {
            assert_eq!(attempt, 1);
            assert!(assessment.findings.is_empty());
            assert!(assessment.intake_error.is_some());
            assert_eq!(assessment.disposition, CheckDisposition::Blocked);
            for result in [&assessment.validation.result, &assessment.replays[0].result] {
                let ToolResult::NotRun { reason } = result else {
                    panic!("intake failure must not fabricate a native execution");
                };
                assert!(reason.contains("intake failed"));
            }
            positive().to_string()
        },
    };
    let outcome = synthesize(
        &provider,
        tools.as_ref(),
        "reviewed request",
        Layer::Kernel,
        2,
    )
    .await
    .unwrap();
    let SynthOutcome::Converged { trace, .. } = outcome else {
        panic!("repairing malformed JSON must run fresh native checks");
    };
    assert_eq!(trace.iterations.len(), 2);
    assert!(trace.iterations[0].assessment.intake_error.is_some());
    assert!(trace.iterations[1].assessment.intake_error.is_none());
    assert_eq!(
        completed(&trace.iterations[1].assessment.replays[0].result)["passed"],
        true
    );
    assert_trace_schema(&serde_json::to_value(trace).unwrap());
}
