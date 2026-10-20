//! Both trace schema views preserve complete checks and reject presentation drift.

use serde_json::{Value, json};
use wos_synth_core::{
    CandidateAssessment, CheckDisposition, CheckRecord, IterationRecord, LintFinding, Severity,
    SynthOutcome, SynthTrace, ToolResult,
};

fn representative_trace() -> SynthTrace {
    let mut trace = SynthTrace::new();
    for (index, result) in [
        ToolResult::Completed { report: json!({"passed":false,"status":"refused","identity":"native-identity","trace":{"steps":[]},"differences":[],"refusals":["native reason"]}) },
        ToolResult::Error { error: json!({"message":"native preparation error","operation":"wos_run_conformance"}) },
        ToolResult::NotRun { reason: "no independent scenario supplied".into() },
    ].into_iter().enumerate() {
        trace.push(IterationRecord {
            index: index as u32,
            attempt: "{}".into(),
            assessment: CandidateAssessment {
                findings: vec![LintFinding { rule_id:"K-001".into(),severity:Severity::Block,
                    message:"repair".into(),path:Some("".into()),suggested_fix:Some("remove property".into()),related_docs:vec!["specs/kernel/spec.md".into()] }],
                intake_error: (index == 2).then(|| json!({"operation":"parse-candidate","message":"unexpected EOF"})),
                validation: CheckRecord { id:"static".into(),arguments:json!({"scope":"project"}),
                    result:ToolResult::Completed { report:json!({"validation":{"scope":"project","status":"not-run","stages":[{"name":"t1-kernel","document":null,"document_version":null,"schema_id":null,"required":true,"status":"not-run","reasons":["native conversion skip"],"diagnostics":[]}],"diagnostics":[]}}) } },
                replays:vec![CheckRecord{id:format!("scenario-{index}"),arguments:json!({}),result}],
                disposition:CheckDisposition::Blocked,
            },
            input_tokens:10,output_tokens:5,cache_read_tokens:3,
        });
    }
    trace
}

fn compile(schema: &Value) -> jsonschema::Validator {
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
    let registry = jsonschema::Registry::new()
        .add("https://wos-spec.org/schemas/tooling/1.0", tooling)
        .unwrap()
        .prepare()
        .unwrap();
    jsonschema::options()
        .with_registry(&registry)
        .with_draft(jsonschema::Draft::Draft202012)
        .build(schema)
        .unwrap()
}

fn schemas() -> [Value; 2] {
    [
        serde_json::from_str(include_str!(
            "../../../schemas/synth/wos-synth-trace.schema.json"
        ))
        .unwrap(),
        json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/synthTrace__SynthTrace"}),
    ]
}

#[test]
fn complete_native_error_not_run_refused_trace_round_trips_in_both_views() {
    let trace = representative_trace();
    let value = serde_json::to_value(&trace).unwrap();
    let roundtrip: SynthTrace = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(roundtrip).unwrap(), value);
    for schema in schemas() {
        let validator = compile(&schema);
        assert!(
            validator.is_valid(&value),
            "{:?}",
            validator
                .iter_errors(&value)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
        let mut invalid = value.clone();
        invalid["iterations"][0]["assessment"]["findings"][0]["severity"] = json!("fatal");
        assert!(!validator.is_valid(&invalid));
        let mut invalid = value.clone();
        invalid["iterations"][0]["assessment"]["findings"][0]["path"] = json!("$.states");
        assert!(!validator.is_valid(&invalid));
        let mut invalid = value.clone();
        invalid["iterations"][0]["assessment"]["validation"]["result"]["report"]["validation"]["status"] =
            json!("invented");
        assert!(
            !validator.is_valid(&invalid),
            "canonical validation schema must remain the report authority"
        );
    }
}

#[test]
fn converged_draft_and_exhausted_outcomes_validate_and_round_trip() {
    let validator = compile(
        &json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/synthTrace__SynthOutcome"}),
    );
    for outcome in [
        SynthOutcome::Converged {
            document: "{}".into(),
            trace: representative_trace(),
        },
        SynthOutcome::Draft {
            document: "{}".into(),
            trace: representative_trace(),
        },
        SynthOutcome::Unconverged {
            last_attempt: "{}".into(),
            last_findings: vec![],
            trace: representative_trace(),
        },
    ] {
        let value = serde_json::to_value(outcome).unwrap();
        assert!(
            validator.is_valid(&value),
            "{:?}",
            validator
                .iter_errors(&value)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
        let roundtrip: SynthOutcome = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(roundtrip).unwrap(), value);
    }
}
