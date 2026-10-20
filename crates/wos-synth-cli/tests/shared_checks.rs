//! Actual CLI acceptance for the shared candidate-check path.

use std::process::Command;

use serde_json::{Value, json};

fn positive() -> Value {
    serde_json::from_str(include_str!(
        "../../wos-lint/tests/fixtures/shared-validation/positive.json"
    ))
    .unwrap()
}

fn scenario() -> Value {
    json!({"id":"typed-authoring-positive","rule":"control","description":"Independent typed-event control",
        "documents":{},"event_sequence":[{"event":"decide","actor":"reviewer"}],
        "expected_transitions":[{"from":"review","target":"done","event":"decide"}]})
}

fn binary_check(candidate: &str, checks: &Value) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let candidate_path = directory.path().join("candidate.json");
    let checks_path = directory.path().join("checks.json");
    std::fs::write(&candidate_path, candidate).unwrap();
    std::fs::write(&checks_path, checks.to_string()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args(["check-candidate", "--candidate"])
        .arg(candidate_path)
        .arg("--checks")
        .arg(checks_path)
        .output()
        .unwrap()
}

#[test]
fn actual_binary_passes_schema_valid_typed_event_candidate() {
    let output = binary_check(
        &positive().to_string(),
        &json!({"validation":{"scope":"project"},"replays":[{"id":"typed-control","arguments":{"fixture_json":scenario().to_string()}}]}),
    );
    assert!(
        output.status.success(),
        "{}",
        format!(
            "stderr={} stdout={}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["disposition"], "checks_passed");
    assert_eq!(
        report["validation"]["result"]["report"]["validation"]["status"],
        "pass"
    );
    assert_eq!(report["replays"][0]["result"]["report"]["passed"], true);
}

use clap::Parser;
use wos_synth_cli::{Cli, Command as SynthCommand, McpToolContext, generate_with};
use wos_synth_core::{
    CheckDisposition, Layer, SynthOutcome, SynthTrace, ToolContext, ToolResult, synthesize,
};

fn configuration(arguments: Vec<Value>) -> Value {
    json!({"validation":{"scope":"project"},"replays":arguments.into_iter().enumerate().map(|(index,arguments)|json!({"id":format!("reviewed-{index}"),"arguments":arguments})).collect::<Vec<_>>()})
}
fn adapter(config: &Value, draft: bool) -> McpToolContext {
    McpToolContext::new(serde_json::from_value(config.clone()).unwrap(), draft).unwrap()
}
fn completed(result: &ToolResult) -> &Value {
    match result {
        ToolResult::Completed { report } => report,
        other => panic!("native completed report required: {other:?}"),
    }
}
fn binary_assessment(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
async fn mcp(candidate: &Value, tool: &str, arguments: &Value) -> Result<Value, String> {
    let mut registry = wos_mcp::registry::ProjectRegistry::new();
    let pid = registry
        .insert(wos_authoring::WosProject::from_native_document(candidate.clone()).unwrap())
        .unwrap()
        .to_string();
    let mut args = arguments.clone();
    args["project_id"] = json!(pid);
    wos_mcp::dispatch::dispatch(&mut registry, tool, &pid, args)
        .await
        .map_err(|error| error.to_string())
}
/// Compare the whole native result after normalizing only independently minted IDs.
///
/// Closed locations: /initialization/result/caseLedgerId; /provenance/*/id;
/// /attempts/*/provenance/*/id; /attempts/*/drains/*/provenance/*/id.
/// Process/task IDs, authored payload IDs, timestamps, array order, counts, and
/// every other field remain exact. Provenance tokens preserve repeated-ID
/// relationships across the top-level, per-attempt, and per-drain copies.
fn semantic_native_report(raw: &Value) -> Value {
    let mut semantic = raw.clone();
    if let Some(id) = semantic.pointer_mut("/initialization/result/caseLedgerId") {
        let family = id.as_str().unwrap().rsplit_once('_').unwrap().0;
        *id = json!(format!("{family}_<generated-case-ledger>"));
    }
    let mut provenance_ids = std::collections::BTreeMap::new();
    if let Some(records) = semantic.get_mut("provenance") {
        normalize_provenance_ids(records, &mut provenance_ids);
    }
    if let Some(attempts) = semantic.get_mut("attempts").and_then(Value::as_array_mut) {
        for attempt in attempts {
            if let Some(records) = attempt.get_mut("provenance") {
                normalize_provenance_ids(records, &mut provenance_ids);
            }
            if let Some(drains) = attempt.get_mut("drains").and_then(Value::as_array_mut) {
                for drain in drains {
                    if let Some(records) = drain.get_mut("provenance") {
                        normalize_provenance_ids(records, &mut provenance_ids);
                    }
                }
            }
        }
    }
    semantic
}

fn normalize_provenance_ids(
    records: &mut Value,
    ids: &mut std::collections::BTreeMap<String, String>,
) {
    if let Some(records) = records.as_array_mut() {
        for record in records {
            // Only this direct native-record field changes. An id anywhere in
            // record data or other nested structures remains untouched.
            if let Some(value) = record.get_mut("id") {
                let id = value.as_str().unwrap();
                let next = ids.len();
                let normalized = ids.entry(id.to_string()).or_insert_with(|| {
                    let family = id.rsplit_once('_').unwrap().0;
                    format!("{family}_<generated-provenance-{next}>")
                });
                *value = json!(normalized);
            }
        }
    }
}

fn assert_native_replay(actual: &Value, expected: &Value, id: &Value) {
    assert_eq!(
        semantic_native_report(actual),
        semantic_native_report(expected),
        "{id}: complete native report parity"
    );
}

/// Retain raw, unnormalized evidence when requested by the integration test run.
fn retain_raw_parity(corpus: &str, vector: &Value, reports: &Value) {
    if let Some(directory) = std::env::var_os("WOS_SYNTH_PARITY_EVIDENCE_DIR") {
        let id = vector["id"].as_str().unwrap();
        assert!(
            id.chars().all(
                |character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            )
        );
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{corpus}--{id}.json")),
            serde_json::to_vec_pretty(&json!({"source":vector,"rawReports":reports})).unwrap(),
        )
        .unwrap();
    }
}

fn replay_arguments(vector: &Value) -> Value {
    json!({"fixture_json":vector["fixture"].to_string(),"required_observations":vector.get("requiredObservations").cloned().unwrap_or(json!([])),
        "expected_observations":vector.get("expectations").cloned().unwrap_or(json!([])),
        "resolved_dependencies":vector.get("dependencies").and_then(Value::as_array).into_iter().flatten().map(|dependency|json!({"role":dependency["role"],"url":dependency["url"],"version":dependency["version"],"document_json":dependency["document"].to_string()})).collect::<Vec<_>>()})
}

#[tokio::test]
async fn unchanged_common_static_vectors_match_owner_mcp_adapter_and_actual_binary() {
    use wos_lint::studio_api::{ValidationRequest, ValidationScope, validate};
    for name in [
        "positive",
        "stored-event",
        "invalid-pattern",
        "unresolved-reference",
        "invalid-hold",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../wos-lint/tests/fixtures/shared-validation")
            .join(format!("{name}.json"));
        let candidate: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let original = candidate.clone();
        let expected = serde_json::to_value(validate(ValidationRequest {
            candidate: &candidate,
            dependencies: &[],
            scope: ValidationScope::Project,
        }))
        .unwrap();
        let config = configuration(vec![]);
        let assessment = adapter(&config, false)
            .check_candidate(&candidate.to_string())
            .await
            .unwrap();
        let native = mcp(&candidate, "wos_lint", &json!({"scope":"project"}))
            .await
            .unwrap();
        let cli = binary_assessment(&binary_check(&candidate.to_string(), &config));
        assert_eq!(completed(&assessment.validation.result), &native, "{name}");
        assert_eq!(native["validation"], expected, "{name}");
        assert_eq!(cli["validation"]["result"]["report"], native, "{name}");
        assert_eq!(
            assessment.disposition,
            CheckDisposition::Blocked,
            "missing replay never passes {name}"
        );
        assert_eq!(cli["disposition"], "blocked");
        assert_eq!(candidate, original);
        retain_raw_parity(
            "static-validation",
            &json!({"id":name,"candidate":candidate}),
            &json!({"configuration":config,"owner":expected,"mcp":native,"adapter":assessment,"cli":cli}),
        );
    }
}

fn owner_replay(vector: &Value, arguments: &Value) -> Result<Value, String> {
    use wos_conformance::studio_api::{ReplayRequest, ResolvedDependency, prepare_replay, replay};
    let prepared = prepare_replay(ReplayRequest {
        candidate_bytes: serde_json::to_vec(&vector["candidate"]).unwrap(),
        fixture_json: vector["fixture"].to_string(),
        clock_start_ms: 0,
        required_observations: serde_json::from_value(arguments["required_observations"].clone())
            .unwrap(),
        expectations: serde_json::from_value(arguments["expected_observations"].clone()).unwrap(),
        dependencies: vector
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|dependency| ResolvedDependency {
                role: dependency["role"].as_str().unwrap().into(),
                url: dependency["url"].as_str().unwrap().into(),
                version: dependency["version"].as_str().map(str::to_string),
                bytes: serde_json::to_vec(&dependency["document"]).unwrap(),
            })
            .collect(),
    })
    .map_err(|error| error.to_string())?;
    replay(&prepared)
        .map(|result| serde_json::to_value(result).unwrap())
        .map_err(|error| error.to_string())
}

#[tokio::test]
async fn common_replay_and_native_ai_vectors_keep_independent_owner_results_through_binary() {
    for (corpus_name, corpus_source) in [
        (
            "authoring-replay",
            include_str!("../../wos-conformance/fixtures/authoring-replay/vectors.json"),
        ),
        (
            "native-ai-replay",
            include_str!("../../wos-conformance/fixtures/native-ai-replay/vectors.json"),
        ),
    ] {
        let corpus: Value = serde_json::from_str(corpus_source).unwrap();
        for vector in corpus["vectors"].as_array().unwrap() {
            let candidate = &vector["candidate"];
            let original = vector.clone();
            let arguments = replay_arguments(vector);
            let config = configuration(vec![arguments.clone()]);
            let owner = owner_replay(vector, &arguments);
            let mcp = mcp(candidate, "wos_run_conformance", &arguments).await;
            let output = binary_check(&candidate.to_string(), &config);
            let tools = McpToolContext::new(serde_json::from_value(config.clone()).unwrap(), false);
            let tools = match tools {
                Err(wos_synth_core::ToolError::Execution(error)) => {
                    assert!(
                        owner.is_err(),
                        "{}: catalog must agree with owner refusal: {error}",
                        vector["id"]
                    );
                    assert_eq!(
                        mcp.as_ref().unwrap_err(),
                        &format!("tool 'wos_run_conformance' failed: {error}")
                    );
                    assert!(!output.status.success());
                    assert!(output.stdout.is_empty());
                    assert_eq!(
                        String::from_utf8_lossy(&output.stderr),
                        format!("error: candidate-check host failure: {error}\n")
                    );
                    retain_raw_parity(
                        corpus_name,
                        vector,
                        &json!({
                            "configuration":config,"owner":{"error":owner.as_ref().unwrap_err()},
                            "mcp":{"error":mcp.as_ref().unwrap_err()},"adapter":{"configurationError":error},
                            "cli":{"exitCode":output.status.code(),"stderr":String::from_utf8_lossy(&output.stderr),"stdout":String::from_utf8_lossy(&output.stdout)}
                        }),
                    );
                    assert_eq!(vector, &original, "literal corpus must remain unchanged");
                    continue;
                }
                Ok(tools) => tools,
            };
            let assessment = tools.check_candidate(&candidate.to_string()).await.unwrap();
            let cli = binary_assessment(&output);
            let static_owner = serde_json::to_value(wos_lint::studio_api::validate(
                wos_lint::studio_api::ValidationRequest {
                    candidate,
                    dependencies: &[],
                    scope: wos_lint::studio_api::ValidationScope::Project,
                },
            ))
            .unwrap();
            assert_eq!(
                completed(&assessment.validation.result)["validation"],
                static_owner,
                "{}",
                vector["id"]
            );
            assert_eq!(
                cli["validation"]["result"]["report"],
                *completed(&assessment.validation.result),
                "{}",
                vector["id"]
            );
            retain_raw_parity(
                corpus_name,
                vector,
                &json!({
                    "configuration":config,"owner":owner,"mcp":mcp,"adapter":assessment,"cli":cli,
                    "cliExitCode":output.status.code()
                }),
            );
            match owner {
                Err(error) => {
                    let expected =
                        format!("tool 'wos_run_conformance' failed: invalid arguments: {error}");
                    assert_eq!(mcp.as_ref().unwrap_err(), &expected);
                    let native_error = match &assessment.replays[0].result {
                        ToolResult::Error { error } => error,
                        other => panic!(
                            "{}: expected complete preparation error, got {other:?}",
                            vector["id"]
                        ),
                    };
                    assert_eq!(native_error["message"], expected);
                    assert_eq!(
                        cli["replays"][0]["result"],
                        serde_json::to_value(&assessment.replays[0].result).unwrap()
                    );
                    assert!(!output.status.success());
                    assert!(
                        cli["replays"][0]["result"]["report"].is_null(),
                        "no fabricated identity/report"
                    );
                }
                Ok(owner) => {
                    let native = completed(&assessment.replays[0].result);
                    assert_native_replay(native, &owner, &vector["id"]);
                    assert_native_replay(&mcp.unwrap(), &owner, &vector["id"]);
                    assert_native_replay(
                        &cli["replays"][0]["result"]["report"],
                        &owner,
                        &vector["id"],
                    );
                    if static_owner["status"] != "pass" || owner["passed"] != true {
                        assert_eq!(assessment.disposition, CheckDisposition::Blocked);
                        assert!(!output.status.success());
                    }
                }
            }
            assert_eq!(vector, &original, "literal corpus must remain unchanged");
        }
    }
}

#[tokio::test]
async fn malformed_nonobject_schema_invalid_and_required_not_run_cannot_pass() {
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    for raw in ["{", "[]", "null", "{}"] {
        let assessment = adapter(&config, false).check_candidate(raw).await.unwrap();
        assert_eq!(assessment.disposition, CheckDisposition::Blocked, "{raw}");
        let cli = binary_assessment(&binary_check(raw, &config));
        assert_eq!(cli["disposition"], "blocked");
        if raw != "{}" {
            assert!(assessment.intake_error.is_some());
            assert!(matches!(
                assessment.validation.result,
                ToolResult::NotRun { .. }
            ));
            assert!(matches!(
                assessment.replays[0].result,
                ToolResult::NotRun { .. }
            ));
        } else {
            assert_eq!(
                completed(&assessment.validation.result)["validation"]["status"],
                "fail"
            );
            assert!(
                assessment
                    .findings
                    .iter()
                    .any(|finding| finding.path.as_deref() == Some("")),
                "native empty root pointer survives"
            );
        }
    }
    let mut candidate = positive();
    candidate["maxRelationshipEventDepth"] = json!(4294967296u64);
    let assessment = adapter(&config, false)
        .check_candidate(&candidate.to_string())
        .await
        .unwrap();
    let report = &completed(&assessment.validation.result)["validation"];
    assert_eq!(report["status"], "not-run", "{report}");
    assert!(
        report["stages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|stage| stage["required"] == true && stage["status"] == "not-run")
    );
    assert_eq!(assessment.disposition, CheckDisposition::Blocked);
    assert_eq!(
        binary_assessment(&binary_check(&candidate.to_string(), &config))["disposition"],
        "blocked"
    );
}

#[tokio::test]
async fn every_scenario_survives_error_failure_and_native_refusal() {
    let mut wrong = scenario();
    wrong["expected_transitions"][0]["target"] = json!("wrong");
    let mut substitute = scenario();
    substitute["documents"] = json!({"kernel":"unrelated.json"});
    let arguments = vec![
        json!({"fixture_json":substitute.to_string()}),
        json!({"fixture_json":wrong.to_string()}),
        json!({"fixture_json":scenario().to_string()}),
    ];
    let config = configuration(arguments);
    let assessment = adapter(&config, false)
        .check_candidate(&positive().to_string())
        .await
        .unwrap();
    assert_eq!(assessment.replays.len(), 3);
    assert!(matches!(
        assessment.replays[0].result,
        ToolResult::Error { .. }
    ));
    assert_eq!(completed(&assessment.replays[1].result)["passed"], false);
    assert_eq!(completed(&assessment.replays[2].result)["passed"], true);
    assert_eq!(assessment.disposition, CheckDisposition::Blocked);
    let cli = binary_assessment(&binary_check(&positive().to_string(), &config));
    assert_eq!(cli["replays"].as_array().unwrap().len(), 3);
    for (index, record) in assessment.replays.iter().enumerate() {
        assert_eq!(cli["replays"][index]["id"], record.id);
        assert_eq!(cli["replays"][index]["arguments"], record.arguments);
    }
    // The maintained native AI control reaches an owning runtime refusal report.
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/native-ai-replay/vectors.json"
    ))
    .unwrap();
    let vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "review-refused")
        .unwrap();
    let result = adapter(&configuration(vec![replay_arguments(vector)]), false)
        .check_candidate(&vector["candidate"].to_string())
        .await
        .unwrap();
    let native = completed(&result.replays[0].result);
    assert_eq!(native["status"], "refused");
    assert!(!native["identity"].as_str().unwrap().is_empty());
    assert!(native.get("initialization").is_some());
    assert!(native.get("attempts").is_some());
}

#[tokio::test]
async fn fixed_scenario_executes_changed_candidate_and_native_only_fields() {
    let base = positive();
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    let tools = adapter(&config, false);
    let first = tools.check_candidate(&base.to_string()).await.unwrap();
    assert_eq!(first.disposition, CheckDisposition::ChecksPassed);
    for (field, value) in [
        ("initial", json!("done")),
        ("native", json!({"source":"reviewed"})),
    ] {
        let mut candidate = base.clone();
        if field == "initial" {
            candidate["lifecycle"]["initialState"] = value;
        } else {
            candidate["x-review-context"] = value;
        }
        let second = tools.check_candidate(&candidate.to_string()).await.unwrap();
        assert_ne!(
            completed(&first.replays[0].result)["identity"],
            completed(&second.replays[0].result)["identity"]
        );
        if field == "initial" {
            assert_eq!(completed(&second.replays[0].result)["passed"], false);
            assert_eq!(second.disposition, CheckDisposition::Blocked);
        }
    }
    for mutation in ["clock", "scenario", "expectation"] {
        let mut changed = config.clone();
        let args = &mut changed["replays"][0]["arguments"];
        match mutation {
            "clock" => args["clock_start_ms"] = json!(99),
            "scenario" => {
                let mut fixture = scenario();
                fixture["id"] = json!("different-reviewed-scenario");
                args["fixture_json"] = json!(fixture.to_string());
            }
            _ => {
                args["expected_observations"] =
                    json!([{"pointer":"/finalSnapshot/configuration","expected":["wrong"]}])
            }
        }
        let second = adapter(&changed, false)
            .check_candidate(&base.to_string())
            .await
            .unwrap();
        assert_ne!(
            completed(&first.replays[0].result)["identity"],
            completed(&second.replays[0].result)["identity"],
            "{mutation}"
        );
    }
}

#[test]
fn actual_binary_rejects_invalid_inner_catalog_arguments() {
    let base = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    for (section, field, value) in [
        ("validation", "unknown", json!(true)),
        (
            "validation",
            "resolved_dependencies",
            json!([{"role":"businessCalendar","url":"urn:test:calendar","version":"1.0.0","document_json":{}}]),
        ),
        ("replay", "clock_start_ms", json!(-1)),
        ("replay", "base_dir", json!(".")),
        ("replay", "required_observations", json!(["unimplemented"])),
    ] {
        let mut config = base.clone();
        if section == "validation" {
            config["validation"][field] = value;
        } else {
            config["replays"][0]["arguments"][field] = value;
        }
        let output = binary_check(&positive().to_string(), &config);
        assert!(!output.status.success(), "{section}.{field}");
        assert!(output.stdout.is_empty(), "invalid config cannot run checks");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("invalid arguments"),
            "{section}.{field}: {error}"
        );
    }
}

#[tokio::test]
async fn missing_replay_blocks_while_explicit_draft_stays_separate() {
    let config = configuration(vec![]);
    let candidate = positive().to_string();
    let checked = adapter(&config, false)
        .check_candidate(&candidate)
        .await
        .unwrap();
    assert_eq!(checked.disposition, CheckDisposition::Blocked);
    assert!(matches!(
        checked.replays[0].result,
        ToolResult::NotRun { .. }
    ));
    let tools = adapter(&config, true);
    let draft = tools.check_candidate(&candidate).await.unwrap();
    assert_eq!(draft.disposition, CheckDisposition::DraftOnly);
    assert!(matches!(draft.replays[0].result, ToolResult::NotRun { .. }));
    let provider = wos_synth_mock::MockPrompter::new();
    provider.expect("", candidate);
    assert!(matches!(
        synthesize(&provider, &tools, "draft", Layer::Kernel, 1)
            .await
            .unwrap(),
        SynthOutcome::Draft { .. }
    ));
}

#[tokio::test]
async fn actual_generate_options_use_real_tools_beyond_session_limit_and_preserve_failed_evidence()
{
    let directory = tempfile::tempdir().unwrap();
    let problem = directory.path().join("problem.md");
    let checks = directory.path().join("checks.json");
    let output = directory.path().join("candidate.json");
    let trace_path = directory.path().join("trace.json");
    std::fs::write(&problem, "Independent reviewed workflow request").unwrap();
    std::fs::write(
        &checks,
        configuration(vec![json!({"fixture_json":scenario().to_string()})]).to_string(),
    )
    .unwrap();
    let args = || {
        let parsed = Cli::parse_from([
            "wos-synth",
            "generate",
            "--problem",
            problem.to_str().unwrap(),
            "--checks",
            checks.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--trace",
            trace_path.to_str().unwrap(),
            "--max-iterations",
            "25",
        ]);
        match parsed.command {
            SynthCommand::Generate(args) => args,
            _ => unreachable!(),
        }
    };
    let provider = wos_synth_mock::MockPrompter::new();
    let mut bad = positive();
    bad["lifecycle"]["initialState"] = json!("done");
    provider.expect_n("", bad.to_string(), 24);
    provider.expect("", positive().to_string());
    assert_eq!(
        generate_with(&provider, args()).await.unwrap(),
        std::process::ExitCode::SUCCESS
    );
    let raw = std::fs::read_to_string(&trace_path).unwrap();
    let trace: SynthTrace = serde_json::from_str(&raw).unwrap();
    assert_eq!(trace.iterations.len(), 25);
    for iteration in &trace.iterations[..24] {
        assert_eq!(
            completed(&iteration.assessment.validation.result)["validation"]["status"],
            "pass"
        );
        assert!(
            iteration.assessment.findings.is_empty(),
            "real behavioral failure is independent of static findings"
        );
        assert_eq!(iteration.assessment.disposition, CheckDisposition::Blocked);
        assert_eq!(
            completed(&iteration.assessment.replays[0].result)["passed"],
            false
        );
    }
    assert_eq!(
        trace.iterations[24].assessment.disposition,
        CheckDisposition::ChecksPassed
    );
    assert_eq!(
        std::fs::read_to_string(&output).unwrap(),
        positive().to_string()
    );
    let rendered = wos_synth_cli::render_trace(&trace);
    assert!(rendered.contains("Blocked"));
    assert!(rendered.contains("ChecksPassed"));
    assert!(rendered.contains("identity="));
    assert_trace_schema(&serde_json::from_str(&raw).unwrap());
    let explained = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args(["explain", trace_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(explained.status.success());
    assert_eq!(String::from_utf8(explained.stdout).unwrap(), rendered);
    let provider = wos_synth_mock::MockPrompter::new();
    provider.expect("", bad.to_string());
    assert_eq!(
        generate_with(&provider, args()).await.unwrap(),
        std::process::ExitCode::from(2)
    );
    let failed: Value =
        serde_json::from_str(&std::fs::read_to_string(&trace_path).unwrap()).unwrap();
    assert_eq!(failed["iterations"].as_array().unwrap().len(), 25);
    assert_trace_schema(&failed);
}

fn assert_trace_schema(value: &Value) {
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
    let registry = jsonschema::Registry::new()
        .add("https://wos-spec.org/schemas/tooling/1.0", tooling)
        .unwrap()
        .prepare()
        .unwrap();
    for schema in [
        serde_json::from_str::<Value>(include_str!(
            "../../../schemas/synth/wos-synth-trace.schema.json"
        ))
        .unwrap(),
        json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/synthTrace__SynthTrace"}),
    ] {
        let validator = jsonschema::options()
            .with_registry(&registry)
            .build(&schema)
            .unwrap();
        assert!(
            validator.is_valid(value),
            "{:?}",
            validator
                .iter_errors(value)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn binary_dry_run_and_configuration_preflight_use_actual_host_path() {
    let directory = tempfile::tempdir().unwrap();
    let problem = directory.path().join("problem.md");
    let output = directory.path().join("candidate.json");
    let trace = directory.path().join("trace.json");
    let checks = directory.path().join("checks.json");
    std::fs::write(&problem, "test problem").unwrap();
    let invoke = |command: &str, extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_wos-synth"))
            .args([
                command,
                "--problem",
                problem.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--trace",
                trace.to_str().unwrap(),
            ])
            .args(extra)
            .env_remove("ANTHROPIC_API_KEY")
            .output()
            .unwrap()
    };
    for command in ["generate", "dry-run"] {
        let result = invoke(command, &[]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("at least one replay scenario"));
        assert!(!output.exists());
    }
    std::fs::write(
        &checks,
        configuration(vec![json!({"fixture_json":scenario().to_string()})]).to_string(),
    )
    .unwrap();
    let result = invoke("dry-run", &["--checks", checks.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("requested synthesis checks passed"));
    let result = invoke("dry-run", &["--draft-only"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("DRAFT ONLY"));
    let draft: Value = serde_json::from_str(&std::fs::read_to_string(&trace).unwrap()).unwrap();
    assert_eq!(
        draft["iterations"][0]["assessment"]["disposition"],
        "draft_only"
    );
    assert_trace_schema(&draft);
    let result = invoke("dry-run", &["--max-iterations", "0", "--draft-only"]);
    assert!(!result.status.success());
    std::fs::write(&checks, "{").unwrap();
    assert!(
        !invoke("dry-run", &["--checks", checks.to_str().unwrap()])
            .status
            .success()
    );
}

#[tokio::test]
async fn selected_static_definition_mapping_and_native_formspec_findings_survive_all_entry_points()
{
    use wos_lint::studio_api::{ResolvedDependency, ValidationRequest, ValidationScope, validate};
    for (source_path, range, status) in [
        ("name", "^1.0.0", "pass"),
        ("missing", "^1.0.0", "fail"),
        ("name", "^2.0.0", "fail"),
    ] {
        let dependencies = json!([
            {"role":"definition","url":"urn:test:definition","version":"1.2.0","document_json":{"$formspec":"1.0","url":"urn:test:definition","version":"1.2.0","status":"draft","title":"Selected Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}},
            {"role":"mapping","url":"urn:test:external-mapping-locator","version":"3.0.0","definition":{"url":"urn:test:definition","version":"1.2.0"},"document_json":{"$formspecMapping":"1.0","version":"3.0.0","definitionRef":"urn:test:definition","definitionVersion":range,"targetSchema":{"format":"json"},"rules":[{"sourcePath":source_path,"targetPath":"name","transform":"preserve"}]}}
        ]);
        let selected: Vec<ResolvedDependency> =
            serde_json::from_value(dependencies.clone()).unwrap();
        let candidate = positive();
        let original = dependencies.clone();
        let owner = serde_json::to_value(validate(ValidationRequest {
            candidate: &candidate,
            dependencies: &selected,
            scope: ValidationScope::Project,
        }))
        .unwrap();
        let mut config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
        config["validation"]["resolved_dependencies"] = dependencies;
        let assessment = adapter(&config, false)
            .check_candidate(&candidate.to_string())
            .await
            .unwrap();
        let native = completed(&assessment.validation.result);
        assert_eq!(native["validation"], owner);
        assert_eq!(owner["status"], status);
        let mcp = mcp(&candidate, "wos_lint", &config["validation"])
            .await
            .unwrap();
        assert_eq!(native, &mcp);
        let cli = binary_assessment(&binary_check(&candidate.to_string(), &config));
        assert_eq!(cli["validation"]["result"]["report"], mcp);
        let mapping = owner["stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stage| {
                stage["document"] == "urn:test:external-mapping-locator"
                    && stage.get("formspec").is_some()
            })
            .unwrap();
        assert_eq!(mapping["document_version"], "3.0.0");
        if source_path == "missing" {
            assert_eq!(mapping["formspec"]["valid"], false);
            assert!(
                mapping["formspec"]["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|finding| finding["path"]
                        .as_str()
                        .is_some_and(|path| path.starts_with('$'))
                        && finding.get("suggestedFix").is_some()
                        && finding.get("specRef").is_some())
            );
            assert_eq!(
                assessment.disposition,
                CheckDisposition::Blocked,
                "empty WOS findings must not bypass native Formspec failures"
            );
        }
        assert_eq!(config["validation"]["resolved_dependencies"], original);
    }
}

#[tokio::test]
async fn runtime_dependency_bytes_are_frozen_and_identity_changes_without_reserializing_them() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/authoring-replay/vectors.json"
    ))
    .unwrap();
    let vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["id"] == "selected-calendar-no-holiday")
        .unwrap();
    let config = configuration(vec![replay_arguments(vector)]);
    let original = config.clone();
    let first = adapter(&config, false)
        .check_candidate(&vector["candidate"].to_string())
        .await
        .unwrap();
    let mut changed = config.clone();
    let document = changed["replays"][0]["arguments"]["resolved_dependencies"][0]["document_json"]
        .as_str()
        .unwrap()
        .to_string();
    changed["replays"][0]["arguments"]["resolved_dependencies"][0]["document_json"] =
        json!(format!("{document}\r\n"));
    let second = adapter(&changed, false)
        .check_candidate(&vector["candidate"].to_string())
        .await
        .unwrap();
    assert_ne!(
        completed(&first.replays[0].result)["identity"],
        completed(&second.replays[0].result)["identity"]
    );
    assert_eq!(
        second.replays[0].arguments["resolved_dependencies"][0]["document_json"],
        format!("{document}\r\n")
    );
    let cli = binary_assessment(&binary_check(&vector["candidate"].to_string(), &changed));
    assert_eq!(
        cli["replays"][0]["result"]["report"]["identity"],
        completed(&second.replays[0].result)["identity"]
    );
    assert_eq!(config, original, "the selected runtime bytes remain frozen");
}

#[tokio::test]
async fn actual_error_not_run_refused_and_draft_evidence_round_trips_and_explains() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/native-ai-replay/vectors.json"
    ))
    .unwrap();
    let refused = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "review-refused")
        .unwrap();
    let mut substitute = scenario();
    substitute["documents"] = json!({"kernel":"unrelated.json"});
    let mut not_run = positive();
    not_run["maxRelationshipEventDepth"] = json!(4294967296u64);
    let normal = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    let cases = vec![
        ("{ not JSON".into(), normal.clone(), false, "unconverged"),
        ("[]".into(), normal.clone(), false, "unconverged"),
        ("{}".into(), normal.clone(), false, "unconverged"),
        (not_run.to_string(), normal.clone(), false, "unconverged"),
        (
            positive().to_string(),
            configuration(vec![]),
            false,
            "unconverged",
        ),
        (
            positive().to_string(),
            configuration(vec![json!({"fixture_json":substitute.to_string()})]),
            false,
            "unconverged",
        ),
        (
            refused["candidate"].to_string(),
            configuration(vec![replay_arguments(refused)]),
            false,
            "unconverged",
        ),
        (positive().to_string(), configuration(vec![]), true, "draft"),
        (positive().to_string(), normal, false, "converged"),
    ];
    let directory = tempfile::tempdir().unwrap();
    for (candidate, config, draft, expected) in cases {
        let tools = adapter(&config, draft);
        let provider = wos_synth_mock::MockPrompter::new();
        provider.expect("", candidate.clone());
        let outcome = synthesize(&provider, &tools, "independent request", Layer::Kernel, 1)
            .await
            .unwrap();
        let serialized = serde_json::to_value(&outcome).unwrap();
        assert_eq!(serialized["outcome"], expected);
        let trace = match outcome {
            SynthOutcome::Converged { trace, .. }
            | SynthOutcome::Draft { trace, .. }
            | SynthOutcome::Unconverged { trace, .. }
            | SynthOutcome::Interrupted { trace, .. } => trace,
        };
        assert_eq!(trace.iterations[0].attempt, candidate);
        let value = serde_json::to_value(&trace).unwrap();
        assert_trace_schema(&value);
        let restored: SynthTrace = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), value);
        let path = directory.path().join("trace.json");
        std::fs::write(&path, serde_json::to_vec(&restored).unwrap()).unwrap();
        let explanation = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
            .args(["explain", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(explanation.status.success());
        assert_eq!(
            String::from_utf8(explanation.stdout).unwrap(),
            wos_synth_cli::render_trace(&trace)
        );
    }
}

#[tokio::test]
async fn repair_provider_failure_saves_complete_native_evidence_and_original_error() {
    let directory = tempfile::tempdir().unwrap();
    let problem = directory.path().join("problem.md");
    let checks = directory.path().join("checks.json");
    let output = directory.path().join("candidate.json");
    std::fs::write(&problem, "reviewed workflow").unwrap();
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    std::fs::write(&checks, config.to_string()).unwrap();
    let mut candidate = positive();
    candidate["lifecycle"]["initialState"] = json!("done");
    let raw = candidate.to_string();
    let native = adapter(&config, false).check_candidate(&raw).await.unwrap();
    let provider = wos_synth_mock::MockPrompter::new();
    provider.expect_n("", raw.clone(), 1);
    let parsed = Cli::parse_from([
        "wos-synth",
        "generate",
        "--problem",
        problem.to_str().unwrap(),
        "--checks",
        checks.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--max-iterations",
        "2",
    ]);
    let args = match parsed.command {
        SynthCommand::Generate(args) => args,
        _ => unreachable!(),
    };
    assert_eq!(
        generate_with(&provider, args).await.unwrap(),
        std::process::ExitCode::from(1)
    );
    assert_eq!(std::fs::read_to_string(&output).unwrap(), raw);
    let path = output.with_extension("synth-trace.json");
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        value["iterations"].as_array().unwrap().len(),
        1,
        "failed repair must not fabricate an iteration"
    );
    assert_eq!(
        value["iterations"][0]["assessment"]["validation"]["result"]["report"],
        *completed(&native.validation.result)
    );
    assert_native_replay(
        &value["iterations"][0]["assessment"]["replays"][0]["result"]["report"],
        completed(&native.replays[0].result),
        &json!("interrupted"),
    );
    assert_eq!(value["interruption"]["operation"], "repair_provider");
    assert!(
        value["interruption"]["error"]
            .as_str()
            .unwrap()
            .contains("unexpected prompt")
    );
    assert_trace_schema(&value);
    let explained = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args(["explain", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(explained.status.success());
    assert!(
        String::from_utf8_lossy(&explained.stdout).contains("interrupted during repair_provider")
    );
    let trace: SynthTrace = serde_json::from_value(value).unwrap();
    let outcome = SynthOutcome::Interrupted {
        last_attempt: raw,
        trace,
    };
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
    let registry = jsonschema::Registry::new()
        .add("https://wos-spec.org/schemas/tooling/1.0", tooling)
        .unwrap()
        .prepare()
        .unwrap();
    let validator = jsonschema::options().with_registry(&registry).build(&json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/synthTrace__SynthOutcome"})).unwrap();
    assert!(validator.is_valid(&serde_json::to_value(&outcome).unwrap()));
    let roundtrip: SynthOutcome =
        serde_json::from_value(serde_json::to_value(&outcome).unwrap()).unwrap();
    assert!(matches!(roundtrip, SynthOutcome::Interrupted { .. }));
}

#[tokio::test]
async fn empty_model_output_is_retained_verbatim_and_valid_evidence() {
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    for candidate in ["", "```\n```"] {
        let provider = wos_synth_mock::MockPrompter::new();
        provider.expect("", candidate);
        let outcome = synthesize(
            &provider,
            &adapter(&config, false),
            "request",
            Layer::Kernel,
            1,
        )
        .await
        .unwrap();
        let value = serde_json::to_value(&outcome).unwrap();
        assert_eq!(value["outcome"], "unconverged");
        assert_eq!(value["last_attempt"], "");
        let trace = match outcome {
            SynthOutcome::Unconverged { trace, .. } => trace,
            other => panic!("{other:?}"),
        };
        assert_eq!(trace.iterations[0].attempt, "");
        assert!(trace.iterations[0].assessment.intake_error.is_some());
        assert_trace_schema(&serde_json::to_value(&trace).unwrap());
        let tooling: Value =
            serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
        let registry = jsonschema::Registry::new()
            .add("https://wos-spec.org/schemas/tooling/1.0", tooling)
            .unwrap()
            .prepare()
            .unwrap();
        let validator=jsonschema::options().with_registry(&registry).build(&json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/synthTrace__SynthOutcome"})).unwrap();
        assert!(validator.is_valid(&value));
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("empty.json");
        std::fs::write(&path, serde_json::to_vec(&trace).unwrap()).unwrap();
        let explained = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
            .args(["explain", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(explained.status.success());
        assert!(String::from_utf8_lossy(&explained.stdout).contains("intake error"));
    }
}

#[test]
fn artifact_write_error_never_prints_a_success_claim() {
    let directory = tempfile::tempdir().unwrap();
    let problem = directory.path().join("problem.md");
    let output = directory.path().join("candidate.json");
    std::fs::write(&problem, "request").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args([
            "dry-run",
            "--problem",
            problem.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--draft-only",
            "--trace",
            directory.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("error:"));
    assert!(!error.contains("checks passed"));
    assert!(!error.contains("wrote"));
    let existing = std::fs::read(&output).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args([
            "dry-run",
            "--problem",
            problem.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--draft-only",
            "--trace",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("different paths"));
    assert_eq!(std::fs::read(&output).unwrap(), existing);
}

#[test]
fn filesystem_aliases_cannot_overwrite_candidate_with_its_trace() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let problem = root.join("problem.md");
    std::fs::write(&problem, "request").unwrap();
    let output = root.join("candidate.json");
    let original = b"preserve existing reviewed candidate";
    std::fs::write(&output, original).unwrap();
    std::fs::create_dir(root.join("subdir")).unwrap();
    let mut aliases = vec![
        std::path::PathBuf::from("candidate.json"),
        root.join("subdir/../candidate.json"),
    ];
    let hard = root.join("hard-link.json");
    std::fs::hard_link(&output, &hard).unwrap();
    aliases.push(hard);
    #[cfg(unix)]
    {
        let link = root.join("symbolic-link.json");
        std::os::unix::fs::symlink(&output, &link).unwrap();
        aliases.push(link);
    }
    for trace in aliases {
        let result = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
            .current_dir(root)
            .args([
                "dry-run",
                "--problem",
                problem.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--trace",
                trace.to_str().unwrap(),
                "--draft-only",
            ])
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(1),
            "alias {} bypassed preflight: stderr={}",
            trace.display(),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("different paths"));
        assert_eq!(
            std::fs::read(&output).unwrap(),
            original,
            "preflight must precede provider/artifact writes"
        );
    }
    #[cfg(unix)]
    {
        let output = root.join("not-created.json");
        let trace = root.join("dangling-link.json");
        std::os::unix::fs::symlink(&output, &trace).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
            .current_dir(root)
            .args([
                "dry-run",
                "--problem",
                problem.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
                "--trace",
                trace.to_str().unwrap(),
                "--draft-only",
            ])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("different paths"));
        assert!(!output.exists());
    }
}

#[test]
fn initially_absent_case_alias_preserves_candidate_when_filesystem_is_case_insensitive() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let probe = root.join("case-probe.json");
    std::fs::write(&probe, "probe").unwrap();
    let case_insensitive = root.join("CASE-PROBE.JSON").exists();
    std::fs::remove_file(&probe).unwrap();
    if !case_insensitive {
        return;
    }
    let output = root.join("candidate.json");
    let trace = root.join("CANDIDATE.JSON");
    let problem = root.join("problem.md");
    std::fs::write(&problem, "request").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_wos-synth"))
        .args([
            "dry-run",
            "--problem",
            problem.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--trace",
            trace.to_str().unwrap(),
            "--draft-only",
        ])
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(1),
        "both absent aliases must never report success"
    );
    let candidate: Value = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(
        candidate["$wosWorkflow"], "1.0",
        "trace must never replace the created candidate"
    );
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("different paths"));
    assert!(!error.contains("wrote"));
}

#[tokio::test]
async fn full_native_parity_rejects_semantic_changes_in_previously_uncompared_fields() {
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    let assessment = adapter(&config, false)
        .check_candidate(&positive().to_string())
        .await
        .unwrap();
    let original = completed(&assessment.replays[0].result).clone();
    for (pointer, replacement) in [
        ("/attempts/0/actor", json!("different-actor")),
        (
            "/initialization/request/processId",
            json!("different-fixed-process"),
        ),
        ("/initialization/result/status", json!("unexpected-status")),
        ("/provenance/0/timestamp", json!("2099-01-01T00:00:00Z")),
        ("/provenance/0/data/state", json!("different-state")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            std::panic::catch_unwind(|| assert_native_replay(&changed, &original, &json!(pointer)))
                .is_err(),
            "full report parity must reject semantic mutation at {pointer}"
        );
    }
}

#[tokio::test]
async fn generated_id_normalization_is_closed_and_retains_raw_relationships() {
    let config = configuration(vec![json!({"fixture_json":scenario().to_string()})]);
    let tools = adapter(&config, false);
    let first = tools
        .check_candidate(&positive().to_string())
        .await
        .unwrap();
    let second = tools
        .check_candidate(&positive().to_string())
        .await
        .unwrap();
    let raw = completed(&first.replays[0].result).clone();
    let original = raw.clone();
    let other = completed(&second.replays[0].result);
    assert_ne!(
        raw["initialization"]["result"]["caseLedgerId"],
        other["initialization"]["result"]["caseLedgerId"]
    );
    assert_ne!(raw["provenance"][0]["id"], other["provenance"][0]["id"]);
    assert_eq!(
        semantic_native_report(&raw),
        semantic_native_report(other),
        "independently minted native IDs are the only allowed differences"
    );
    assert_eq!(
        raw, original,
        "normalization must not modify raw retained native evidence"
    );

    let mut broken_copy = raw.clone();
    broken_copy["attempts"][0]["provenance"][0]["id"] =
        json!(wos_conformance::ProvenanceRecord::mint_id());
    assert_ne!(
        semantic_native_report(&broken_copy),
        semantic_native_report(&raw),
        "repeated native provenance references must retain the same relationship"
    );
    let mut added_payload = raw.clone();
    added_payload["provenance"][0]["data"]["id"] = raw["provenance"][0]["id"].clone();
    assert_eq!(
        semantic_native_report(&added_payload)["provenance"][0]["data"]["id"],
        raw["provenance"][0]["id"],
        "an authored/data id is outside the closed normalization paths"
    );
    assert_ne!(
        semantic_native_report(&added_payload),
        semantic_native_report(&raw)
    );
    let mut changed_family = raw.clone();
    changed_family["initialization"]["result"]["caseLedgerId"] =
        json!("different_tenant_case_generated");
    assert_ne!(
        semantic_native_report(&changed_family),
        semantic_native_report(&raw),
        "native ID tenant/family prefixes remain semantic evidence"
    );
    for (pointer, replacement) in [
        ("/attempts/0/clockMs", json!(99)),
        (
            "/attempts/0/drains/0/processedEvent",
            json!("different-event"),
        ),
        (
            "/attempts/0/drains/0/provenance/0/timestamp",
            json!("2099-01-01T00:00:00Z"),
        ),
        (
            "/initialization/result/definitionVersion",
            json!("changed-version"),
        ),
        (
            "/initialization/result/createdAt",
            json!("2099-01-01T00:00:00Z"),
        ),
        ("/provenance/0/data/state", json!("changed-state")),
    ] {
        let mut changed = raw.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert_ne!(
            semantic_native_report(&changed),
            semantic_native_report(&raw),
            "semantic mutation at {pointer} must remain visible"
        );
    }
    let nullable = json!({"provenance":null,"attempts":[{"provenance":null,"drains":[]}]});
    assert_eq!(
        semantic_native_report(&nullable),
        nullable,
        "normalization must not add fields"
    );
    let mut missing = nullable.clone();
    missing.as_object_mut().unwrap().remove("provenance");
    assert_ne!(
        semantic_native_report(&missing),
        semantic_native_report(&nullable),
        "missing evidence is distinct from a native null result"
    );
    let mut missing = nullable.clone();
    missing["attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    assert_ne!(
        semantic_native_report(&missing),
        semantic_native_report(&nullable)
    );
}
