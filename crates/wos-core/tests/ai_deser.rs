// Rust guideline compliant 2026-02-21

//! Round-trip deserialization tests for WOS AI Integration Documents.

use std::fs;
use wos_core::AIIntegrationDocument;

fn load_fixture(name: &str) -> AIIntegrationDocument {
    let path = format!(
        "{}/fixtures/ai/{name}",
        env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
    );
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn benefits_adjudication_ai_round_trips() {
    let doc = load_fixture("benefits-adjudication-ai.json");
    assert_eq!(doc.wos_ai_integration, "1.0");
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
