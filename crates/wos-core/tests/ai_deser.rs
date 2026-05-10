// Rust guideline compliant 2026-04-28

//! Round-trip deserialization tests for the AI integration content embedded in
//! `$wosWorkflow` documents (was a standalone document with `$wosAIIntegration`
//! marker; per ADR 0076 D-1 the marker now lives on the workflow envelope and
//! `AIIntegrationDocument` represents the embedded `aiOversight` block).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use wos_core::AIIntegrationDocument;

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

fn load_fixture(name: &str) -> AIIntegrationDocument {
    let path = workspace_root().join("fixtures/ai").join(name);
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    let envelope: Value = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse fixture {name} envelope: {e}"));
    assert_eq!(
        envelope.get("$wosWorkflow").and_then(Value::as_str),
        Some("1.0"),
        "fixture {name} must carry $wosWorkflow envelope per ADR 0076 D-1"
    );
    let mut block = envelope
        .get("aiOversight")
        .cloned()
        .unwrap_or_else(|| panic!("fixture {name} missing aiOversight embedded block"));
    if let Some(map) = block.as_object_mut() {
        if let Some(target) = envelope.get("url").cloned() {
            map.entry("targetWorkflow".to_string()).or_insert(target);
        }
        if let Some(agents) = map.get("x-transportAgentDetails").cloned() {
            map.entry("agents".to_string()).or_insert(agents);
        }
    }
    serde_json::from_value(block)
        .unwrap_or_else(|e| panic!("failed to deserialize aiOversight from {name}: {e}"))
}

#[test]
fn benefits_adjudication_ai_round_trips() {
    let doc = load_fixture("benefits-adjudication-ai.json");
    assert!(doc.target_workflow.contains("benefits-adjudication"));

    // Agents
    assert_eq!(doc.agents.len(), 2);
    let extractor = &doc.agents[0];
    assert_eq!(extractor.id, "documentExtractor");
    assert_eq!(
        extractor.agent_type,
        wos_core::model::ai::AgentType::Generative
    );

    let screener = &doc.agents[1];
    assert_eq!(screener.id, "eligibilityScreener");
    assert_eq!(
        screener.agent_type,
        wos_core::model::ai::AgentType::Statistical
    );

    // Deontic constraints
    let deontic = doc
        .deontic_constraints
        .as_ref()
        .expect("deontic constraints");
    assert!(!deontic.permissions.is_empty());
    assert!(!deontic.prohibitions.is_empty());
    assert!(!deontic.obligations.is_empty());
    assert!(!deontic.rights.is_empty());

    // Confidence floor
    let floor = doc.confidence_floor.as_ref().expect("confidence floor");
    assert!(floor.threshold > 0.0);

    // Fallback chain
    assert!(!doc.fallback_chain.is_empty());

    // Volume constraints
    let vol = doc.volume_constraints.as_ref().expect("volume constraints");
    assert!(vol.max_autonomous_per_hour.is_some());

    // Agent disclosure
    let disclosure = doc.agent_disclosure.as_ref().expect("disclosure");
    assert!(disclosure.disclose_that_agent_assisted);
}
