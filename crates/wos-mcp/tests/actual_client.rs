//! An official SDK exercises the Cargo-selected executable and native dispatch.

use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};
use tokio::{process::Command, time::timeout};
use wos_mcp::{dispatch::dispatch, errors::DispatchError, registry::ProjectRegistry};

#[path = "../../wos-conformance/tests/support/replay_semantics.rs"]
mod replay_semantics;

fn inputs() -> Value {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../wos-conformance/fixtures/authoring-replay/vectors.json"
    ))
    .unwrap();
    let passing = vectors["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "guard-true")
        .unwrap();
    assert_eq!(passing["expected"], "pass");
    let mut candidate = passing["candidate"].clone();
    candidate["x-client-preserved"] = json!({"authored":[1,true,"unknown"]});
    candidate["lifecycle"]["states"]["open"]["x-state-preserved"] = json!({"source":"reviewed"});
    let mut state = candidate["lifecycle"]["states"]["open"].clone();
    state["transitions"][0]["event"] = json!({"kind":"message","name":"decide"});
    let mut failed_state = state.clone();
    failed_state["transitions"][0]["guard"] = json!("false");
    // Same selected native Definition/Mapping shape as shared_validation.rs.
    let dependencies = json!([
        {"role":"definition","url":"urn:test:missing-definition","version":"1.0.0","document_json":{"$formspec":"1.0","url":"urn:test:missing-definition","version":"1.0.0","status":"draft","title":"Selected Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}},
        {"role":"mapping","url":"urn:test:external-mapping-locator","version":"3.0.0","definition":{"url":"urn:test:missing-definition","version":"1.0.0"},"document_json":{"$formspecMapping":"1.0","version":"3.0.0","definitionRef":"urn:test:missing-definition","definitionVersion":"^1.0.0","targetSchema":{"format":"json"},"rules":[{"sourcePath":"name","targetPath":"name","transform":"preserve"}]}}
    ]);
    let mut replay_dependencies = dependencies.clone();
    for dependency in replay_dependencies.as_array_mut().unwrap() {
        dependency["document_json"] = json!(dependency["document_json"].to_string());
    }
    let validation = json!({"scope":"project","resolved_dependencies":dependencies});
    let checks = json!({"validation":validation,"replays":[{"id":"reviewed","arguments":{
        "fixture_json":passing["fixture"].to_string(),"resolved_dependencies":replay_dependencies,
        "clock_start_ms":12345,"expected_observations":[{"pointer":"/finalSnapshot/configuration","expected":["done"]}]
    }}]});
    json!({"catalog":wos_mcp::catalog::descriptors(),"candidate":candidate.to_string(),
        "passing_state":state,"failing_state":failed_state,"checks":checks,"validation":validation,
        "reference_candidate":include_str!("../../wos-lint/tests/fixtures/shared-validation/unresolved-reference.json")})
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
    .unwrap();
}

fn semantic_result(tool: &str, value: &Value) -> Value {
    match tool {
        "wos_run_conformance" => replay_semantics::semantic_native_report(value),
        "wos_check_authoring" => {
            let mut normalized = value.clone();
            for replay in normalized["replays"].as_array_mut().unwrap() {
                if let Some(report) = replay["result"].get_mut("report") {
                    *report = replay_semantics::semantic_native_report(report);
                }
            }
            normalized
        }
        _ => value.clone(),
    }
}

/// Replay the same SDK calls through the actual Rust entry, preserving fault kind.
async fn compare_dispatch(evidence: &Path) {
    let transcript = fs::read_to_string(evidence.join("calls.jsonl")).unwrap();
    let mut registry = ProjectRegistry::new();
    let mut process = 0;
    let mut projects = BTreeMap::<String, String>::new();
    let mut compared = vec![];
    for line in transcript.lines() {
        let entry: Value = serde_json::from_str(line).unwrap();
        let Some(name) = entry["name"].as_str() else {
            continue;
        };
        if entry["process"] != process {
            registry = ProjectRegistry::new();
            projects.clear();
            process += 1;
            assert_eq!(entry["process"], process);
        }
        let mut args = entry["arguments"].clone();
        if let Some(pid) = args["project_id"].as_str() {
            args["project_id"] = json!(
                projects
                    .get(pid)
                    .expect("SDK project maps to native project")
            );
        }
        let pid = args["project_id"].as_str().unwrap_or("").to_string();
        let actual = dispatch(&mut registry, name, &pid, args).await;
        if entry.get("protocol_error").is_some() {
            assert!(
                matches!(actual, Err(DispatchError::UnknownTool(_))),
                "{entry}"
            );
            assert_eq!(entry["protocol_error"]["code"], -32602);
        } else if entry["result"]["isError"] == true {
            let error = actual.unwrap_err();
            assert!(matches!(error, DispatchError::ToolFailed { .. }));
            assert_eq!(entry["result"]["content"][0]["text"], error.to_string());
        } else {
            let actual = actual.unwrap_or_else(|e| panic!("{name}: {e}"));
            let mut expected = entry["value"].clone();
            if matches!(name, "wos_create_kernel" | "wos_load_document") {
                projects.insert(
                    expected["project_id"].as_str().unwrap().into(),
                    actual["project_id"].as_str().unwrap().into(),
                );
            }
            if let Some(pid) = expected["project_id"].as_str() {
                expected["project_id"] = json!(projects.get(pid).unwrap());
            }
            assert_eq!(
                semantic_result(name, &actual),
                semantic_result(name, &expected),
                "official SDK/native parity: {name}"
            );
        }
        compared.push(json!({"process":process,"tool":name,"matched":true}));
    }
    assert!(compared.len() > 30, "complete journey required");
    write_json(&evidence.join("dispatch-parity.json"), &json!(compared));
}

#[tokio::test]
async fn official_sdk_authoring_failure_repair_and_exact_reopen() {
    let temporary = tempfile::tempdir().unwrap();
    let evidence = std::env::var_os("WOS_MCP_CLIENT_EVIDENCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_path_buf());
    fs::create_dir_all(&evidence).unwrap();
    let manifest = evidence.join("selected-inputs.json");
    write_json(&manifest, &inputs());
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&source)
        .output()
        .await
        .unwrap();
    let diff = Command::new("git")
        .args(["diff", "--", "crates/wos-mcp"])
        .current_dir(&source)
        .output()
        .await
        .unwrap();
    fs::write(evidence.join("source-revision.txt"), revision.stdout).unwrap();
    fs::write(evidence.join("source.patch"), diff.stdout).unwrap();
    // New test/support files are untracked during qualification; retain their
    // exact contents as well as the tracked diff and selected source revision.
    for path in [
        "src/server.rs",
        "src/catalog.rs",
        "src/catalog_schema.rs",
        "tests/actual_client.rs",
        "tests/support/official_client.py",
        "tests/support/official_client.py.lock",
        "tool-catalog.json",
        "README.md",
    ] {
        let destination = evidence.join("sources").join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(path),
            destination,
        )
        .unwrap();
    }
    let mut command = Command::new("uv");
    command
        .args(["run", "--offline", "--locked", "--script"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/official_client.py"))
        .arg("--binary")
        .arg(env!("CARGO_BIN_EXE_wos-mcp"))
        .arg("--inputs")
        .arg(&manifest)
        .arg("--evidence")
        .arg(&evidence)
        .stdout(fs::File::create(evidence.join("client.stdout")).unwrap())
        .stderr(fs::File::create(evidence.join("client-and-server.stderr")).unwrap())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .expect("uv prerequisite: bootstrap the locked official-client script as documented");
    // The driver bounds and cleans its server processes; this bound also covers uv.
    let status = timeout(Duration::from_secs(150), child.wait()).await;
    let status = match status {
        Ok(Ok(status)) => status,
        failure => {
            let kept = temporary.keep();
            panic!(
                "official client process failure {failure:?}; evidence: {} (temporary {})",
                evidence.display(),
                kept.display()
            );
        }
    };
    if !status.success() {
        let kept = temporary.keep();
        panic!(
            "official SDK failed: {}; evidence {} (temporary {}): {}",
            status,
            evidence.display(),
            kept.display(),
            fs::read_to_string(evidence.join("client-and-server.stderr")).unwrap()
        );
    }
    let parity_evidence = evidence.clone();
    if let Err(failure) =
        tokio::spawn(async move { compare_dispatch(&parity_evidence).await }).await
    {
        let kept = temporary.keep();
        panic!(
            "native dispatch parity failure {failure}; evidence {} (temporary {})",
            evidence.display(),
            kept.display()
        );
    }
}
