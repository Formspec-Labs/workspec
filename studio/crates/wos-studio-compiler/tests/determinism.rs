// Rust guideline compliant 2026-05-02

//! Cross-process determinism harness for `SA-MUST-cmp-001`.
//!
//! The in-process determinism check at `pipeline.rs::deterministic_compile_byte_identical_in_process_repeats`
//! re-runs `compile()` 10× in one process, which catches per-call
//! HashSet jitter but NOT `HashMap` `RandomState` per-process drift
//! (the seed is process-stable). This test spawns the compiled binary
//! 5× as separate processes and compares output, defeating that
//! seed-stability.
//!
//! If this test ever fails, it means the compiler has a non-deterministic
//! data structure leaking into output across processes — investigate
//! via the same hashes as the in-process test.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const ITERATIONS: usize = 5;

#[test]
fn cross_process_compile_byte_identical() {
    let workspace = build_fixture_workspace();
    let bin = env!("CARGO_BIN_EXE_wos-studio-compile");

    let mut outputs: Vec<(String, String, String, String)> = Vec::new();
    for i in 0..ITERATIONS {
        let out_dir = workspace.path().join(format!("out-{i}"));
        let result = Command::new(bin)
            .arg(workspace.input_dir())
            .arg("--out")
            .arg(&out_dir)
            .arg("--no-gates")
            .output()
            .expect("spawn wos-studio-compile");
        assert!(
            result.status.success(),
            "iteration {i}: stderr={}",
            String::from_utf8_lossy(&result.stderr)
        );

        let workflow = std::fs::read_to_string(out_dir.join("wos-workflow.json"))
            .expect("workflow.json missing");
        let manifest = std::fs::read_to_string(out_dir.join("compile-manifest.json"))
            .expect("manifest.json missing");
        let scenarios = std::fs::read_to_string(out_dir.join("scenarios.json"))
            .expect("scenarios.json missing");
        let manifest_hash = extract_manifest_hash(&manifest);
        outputs.push((workflow, manifest, scenarios, manifest_hash));
    }

    let baseline = &outputs[0];
    for (i, o) in outputs.iter().enumerate().skip(1) {
        assert_eq!(o.0, baseline.0, "process {i}: wos-workflow.json drift");
        // manifest itself differs in `compiledAt`, so we compare the
        // hash field which is computed over a `compiledAt`-excluded
        // canonicalization (SA-MUST-cmp-073).
        assert_eq!(o.3, baseline.3, "process {i}: manifestHash drift");
        assert_eq!(o.2, baseline.2, "process {i}: scenarios.json drift");
    }
    assert!(
        baseline.3.starts_with("sha256:"),
        "manifest_hash format: {}",
        baseline.3
    );
}

fn extract_manifest_hash(manifest_json: &str) -> String {
    let v: serde_json::Value =
        serde_json::from_str(manifest_json).expect("manifest is valid JSON");
    v.get("manifestHash")
        .and_then(serde_json::Value::as_str)
        .expect("manifestHash field present")
        .to_string()
}

// ---------------------------------------------------------------------------
// Fixture workspace — minimal compile-clean shape
// ---------------------------------------------------------------------------

struct FixtureWorkspace {
    root: PathBuf,
}

impl FixtureWorkspace {
    fn path(&self) -> &PathBuf {
        &self.root
    }
    fn input_dir(&self) -> PathBuf {
        self.root.join("input")
    }
}

impl Drop for FixtureWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn build_fixture_workspace() -> FixtureWorkspace {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("wos-det-{pid}-{nanos}"));
    let input = root.join("input");
    std::fs::create_dir_all(&input).expect("mkdir fixture");

    write(&input, "wfi.json", r#"{
        "$wosStudioWorkflowIntent": "1.0",
        "id": "wfi-1",
        "workspaceId": "ws-1",
        "version": "0.1.0",
        "title": "Determinism fixture",
        "impactLevel": "operational",
        "publicationUrl": "https://example.org/det",
        "actors": [],
        "elements": [
            {"id": "intake", "kind": "step",
             "policyObjectRefs": ["pol-x"],
             "bridge": {"kernelKind": "transition"},
             "derivedFrom": ["pol-x"]}
        ]
    }"#);
    write(&input, "po.json", r#"{
        "$wosStudioPolicyObject": "1.0",
        "policyObjects": [{
            "id": "pol-x", "workspaceId": "ws-1",
            "kind": "DecisionRule", "lifecycleState": "approved",
            "originClass": "source",
            "citations": [{"sourceCitationRef": "c-1"}]
        }]
    }"#);
    write(&input, "map.json", r#"{
        "$wosStudioMapping": "1.0",
        "mappings": [{
            "id": "m-1", "policyObjectRef": "pol-x",
            "mappingState": "mapsToWos",
            "targets": [{
                "wosConceptId": "DecisionRule",
                "wosJsonPath": "$.governance.policyObjects[0]"
            }]
        }]
    }"#);
    write(&input, "ws.json", r#"{
        "$wosStudioWorkspace": "1.0",
        "id": "ws-1", "title": "Determinism fixture",
        "reviewerRoles": []
    }"#);

    FixtureWorkspace { root }
}

fn write(dir: &PathBuf, name: &str, contents: &str) {
    let mut f = std::fs::File::create(dir.join(name)).expect("create fixture file");
    f.write_all(contents.as_bytes()).expect("write fixture");
}
