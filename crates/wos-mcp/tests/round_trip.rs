//! Round-trip integration test for wos-mcp Tasks 5 + 6.
//!
//! Uses the in-process `dispatch()` path — no subprocess spawn, no JSON-RPC
//! overhead. Exercises all 22 tool handlers via the same dispatch table used
//! by the production stdio server.
//!
//! # What this test covers
//!
//! 1. `wos_create_kernel` — create a project.
//! 2. `wos_set_impact_level` — set governance classification.
//! 3. `wos_add_state` (×3) — add three lifecycle states.
//! 4. `wos_set_initial_state` — designate the first state.
//! 5. `wos_add_transition` (×2) — wire the lifecycle.
//! 6. `wos_add_actor` — declare a human actor.
//! 7. `wos_add_due_process_path` — write a due-process path.
//! 8. `wos_add_assertion_gate` — register a gate.
//! 9. `wos_add_ai_agent` — register an AI agent.
//! 10. `wos_add_deontic_constraint` — append a deontic constraint.
//! 11. `wos_preview_state_graph` — generate Mermaid and DOT graphs.
//! 12. `wos_search` — search states, transitions, actors, and constraints.
//! 13. `wos_describe_document` — verify final counts.
//! 14. `wos_export_document` — round-trip to JSON.
//! 15. `wos_lint` — assert zero errors on the exported document.
//! 16. `wos_run_conformance` — run a known-passing conformance fixture.
//! 17. `wos_list_projects` — assert the project is listed.
//! 18. `wos_close_project` — remove the project.
//! 19. `wos_list_projects` (again) — assert registry is empty.

use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

/// Resolve the path to a conformance fixture file relative to the crate manifest directory.
fn conformance_fixture_path(name: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // wos-mcp is at crates/wos-mcp; fixtures are at crates/wos-conformance/tests/fixtures/
    format!("{manifest}/../wos-conformance/tests/fixtures/{name}")
}

/// Resolve the base_dir for a conformance fixture file.
fn conformance_fixture_base_dir() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/../wos-conformance/tests/fixtures")
}

#[tokio::test]
async fn full_round_trip_with_10_plus_tool_calls() {
    let mut registry = ProjectRegistry::new();

    // ── 1. Create a project ───────────────────────────────────────────────

    let create = dispatch(&mut registry, "wos_create_kernel", "", serde_json::json!({}))
        .await
        .expect("wos_create_kernel must succeed");
    let pid = create["project_id"].as_str().expect("must have project_id").to_string();

    // ── 2. Set impact level ───────────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_set_impact_level",
        &pid,
        serde_json::json!({ "project_id": pid, "level": "operational" }),
    )
    .await
    .expect("wos_set_impact_level must accept 'operational'");

    // ── 3. Add three states ───────────────────────────────────────────────

    for state_id in ["open", "under_review", "closed"] {
        dispatch(
            &mut registry,
            "wos_add_state",
            &pid,
            serde_json::json!({ "project_id": pid, "state_id": state_id }),
        )
        .await
        .unwrap_or_else(|e| panic!("wos_add_state({state_id}) must succeed: {e}"));
    }

    // ── 4. Set initial state ──────────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_set_initial_state",
        &pid,
        serde_json::json!({ "project_id": pid, "state_id": "open" }),
    )
    .await
    .expect("wos_set_initial_state must succeed");

    // ── 5. Add two transitions ────────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_transition",
        &pid,
        serde_json::json!({
            "project_id": pid,
            "from": "open",
            "to": "under_review",
            "trigger": "startReview"
        }),
    )
    .await
    .expect("wos_add_transition(open → under_review) must succeed");

    dispatch(
        &mut registry,
        "wos_add_transition",
        &pid,
        serde_json::json!({
            "project_id": pid,
            "from": "under_review",
            "to": "closed",
            "trigger": "close"
        }),
    )
    .await
    .expect("wos_add_transition(under_review → closed) must succeed");

    // ── 6. Add a human actor ──────────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_actor",
        &pid,
        serde_json::json!({ "project_id": pid, "actor_id": "reviewer", "kind": "human" }),
    )
    .await
    .expect("wos_add_actor must succeed");

    // ── 7. Add a due-process path ─────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_due_process_path",
        &pid,
        serde_json::json!({
            "project_id":  pid,
            "path_id":     "appealPath",
            "description": "Standard appeal process",
            "steps":       ["file_appeal", "committee_review", "final_decision"]
        }),
    )
    .await
    .expect("wos_add_due_process_path must succeed");

    // ── 8. Add an assertion gate ──────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_assertion_gate",
        &pid,
        serde_json::json!({
            "project_id": pid,
            "gate_id":    "reviewerAssigned",
            "assertion":  "caseFile.reviewerId != null",
            "transition": "startReview"
        }),
    )
    .await
    .expect("wos_add_assertion_gate must succeed");

    // ── 9. Register an AI agent ───────────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_ai_agent",
        &pid,
        serde_json::json!({
            "project_id":   pid,
            "agent_id":     "summaryBot",
            "role":         "Summarise case documents",
            "model":        "claude-3-5-sonnet",
            "capabilities": ["read_case_file", "submit_summary"]
        }),
    )
    .await
    .expect("wos_add_ai_agent must succeed");

    // ── 10. Add a deontic constraint ──────────────────────────────────────

    dispatch(
        &mut registry,
        "wos_add_deontic_constraint",
        &pid,
        serde_json::json!({
            "project_id":    pid,
            "constraint_id": "mustNotAutoClose",
            "target":        "summaryBot",
            "modality":      "must_not",
            "action":        "close-case"
        }),
    )
    .await
    .expect("wos_add_deontic_constraint must succeed");

    // ── 11. Preview state graph (Mermaid) ─────────────────────────────────

    let mermaid = dispatch(
        &mut registry,
        "wos_preview_state_graph",
        &pid,
        serde_json::json!({ "project_id": pid, "format": "mermaid" }),
    )
    .await
    .expect("wos_preview_state_graph(mermaid) must succeed");

    let graph_str = mermaid["graph"].as_str().expect("must have graph string");
    assert!(graph_str.contains("open"), "mermaid graph must mention 'open' state");
    assert!(graph_str.contains("closed"), "mermaid graph must mention 'closed' state");
    assert!(graph_str.contains("startReview"), "mermaid graph must mention 'startReview' event");

    // DOT format.
    let dot = dispatch(
        &mut registry,
        "wos_preview_state_graph",
        &pid,
        serde_json::json!({ "project_id": pid, "format": "dot" }),
    )
    .await
    .expect("wos_preview_state_graph(dot) must succeed");

    assert!(dot["graph"].as_str().unwrap().contains("digraph"));

    // ── 12. Search ────────────────────────────────────────────────────────

    let state_search = dispatch(
        &mut registry,
        "wos_search",
        &pid,
        serde_json::json!({ "project_id": pid, "kind": "state", "query": "review" }),
    )
    .await
    .expect("wos_search(state) must succeed");

    let state_matches = state_search["matches"].as_array().unwrap();
    assert!(
        state_matches.len() >= 1,
        "search for 'review' must find at least 'under_review'"
    );

    let transition_search = dispatch(
        &mut registry,
        "wos_search",
        &pid,
        serde_json::json!({ "project_id": pid, "kind": "transition", "query": "close" }),
    )
    .await
    .expect("wos_search(transition) must succeed");

    let t_matches = transition_search["matches"].as_array().unwrap();
    assert!(
        t_matches.len() >= 1,
        "search for 'close' must find at least the close transition"
    );

    let actor_search = dispatch(
        &mut registry,
        "wos_search",
        &pid,
        serde_json::json!({ "project_id": pid, "kind": "actor", "query": "review" }),
    )
    .await
    .expect("wos_search(actor) must succeed");

    assert_eq!(actor_search["matches"].as_array().unwrap().len(), 1);

    let constraint_search = dispatch(
        &mut registry,
        "wos_search",
        &pid,
        serde_json::json!({ "project_id": pid, "kind": "constraint", "query": "close" }),
    )
    .await
    .expect("wos_search(constraint) must succeed");

    assert_eq!(constraint_search["matches"].as_array().unwrap().len(), 1);

    // ── 13. Describe document — verify final counts ───────────────────────

    let describe = dispatch(
        &mut registry,
        "wos_describe_document",
        &pid,
        serde_json::json!({ "project_id": pid }),
    )
    .await
    .expect("wos_describe_document must succeed");

    assert_eq!(describe["state_count"], serde_json::json!(3), "must have 3 states");
    assert_eq!(describe["transition_count"], serde_json::json!(2), "must have 2 transitions");
    assert_eq!(describe["actor_count"], serde_json::json!(1), "must have 1 actor");
    assert_eq!(describe["ai_agent_count"], serde_json::json!(1), "must have 1 AI agent");
    assert_eq!(describe["impact_level"], serde_json::json!("operational"));

    // ── 14. Export document ───────────────────────────────────────────────

    let export = dispatch(
        &mut registry,
        "wos_export_document",
        &pid,
        serde_json::json!({ "project_id": pid }),
    )
    .await
    .expect("wos_export_document must succeed");

    let doc_json = export["document"].as_str().expect("must have document string");
    let doc: serde_json::Value = serde_json::from_str(doc_json).expect("exported doc must be valid JSON");

    // Verify round-trip structural integrity.
    assert_eq!(doc["$wosKernel"], serde_json::json!("1.0"));
    let states = doc["lifecycle"]["states"].as_object().unwrap();
    assert_eq!(states.len(), 3);
    assert_eq!(doc["lifecycle"]["initialState"], serde_json::json!("open"));

    // Governance extensions are stored under doc.extensions (not flattened to top-level).
    // The serialized JSON keeps them as a nested "extensions" object.
    let extensions = &doc["extensions"];

    let gov = &extensions["x-wos-governance"];
    assert!(
        gov["dueProcessPaths"]["appealPath"]["description"].is_string(),
        "governance due-process path must be stored under extensions[x-wos-governance]; \
         extensions: {extensions}"
    );
    assert!(gov["assertionGates"]["reviewerAssigned"]["assertion"].is_string());

    // AI extension must persist.
    let ai = &extensions["x-wos-ai"];
    let agents = ai["agents"].as_array().expect("x-wos-ai.agents must be an array");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["id"], serde_json::json!("summaryBot"));

    let constraints = ai["deonticConstraints"].as_array().unwrap();
    assert_eq!(constraints.len(), 1);
    assert_eq!(constraints[0]["modality"], serde_json::json!("must_not"));

    // ── 15. Lint — assert zero errors ─────────────────────────────────────

    let lint = dispatch(
        &mut registry,
        "wos_lint",
        &pid,
        serde_json::json!({ "project_id": pid }),
    )
    .await
    .expect("wos_lint must succeed");

    let error_count = lint["error_count"].as_u64().expect("must have error_count");
    assert_eq!(
        error_count, 0,
        "round-trip project must lint with zero errors; diagnostics: {}",
        lint["diagnostics"]
    );

    // ── 16. Run conformance fixture ───────────────────────────────────────

    // Load the purchase-order-simple fixture. This fixture exercises
    // kernel lifecycle conformance against a well-known document and
    // is known to pass.
    let fixture_path = conformance_fixture_path("purchase-order-simple.json");
    let fixture_json = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("could not read fixture '{fixture_path}': {e}"));
    let base_dir = conformance_fixture_base_dir();

    let conformance = dispatch(
        &mut registry,
        "wos_run_conformance",
        "",
        serde_json::json!({
            "fixture_json": fixture_json,
            "base_dir": base_dir
        }),
    )
    .await
    .expect("wos_run_conformance must succeed");

    assert_eq!(
        conformance["passed"],
        serde_json::json!(true),
        "purchase-order-simple must pass conformance; failures: {}",
        conformance["failures"]
    );
    assert!(conformance["trace"].is_object(), "trace must be a JSON object");

    // ── 17. List projects — assert our project is listed ──────────────────

    let list = dispatch(&mut registry, "wos_list_projects", "", serde_json::json!({}))
        .await
        .expect("wos_list_projects must succeed");

    let projects = list["projects"].as_array().unwrap();
    assert!(
        projects.iter().any(|v| v.as_str() == Some(pid.as_str())),
        "wos_list_projects must include our project_id"
    );

    // ── 18. Close the project ─────────────────────────────────────────────

    let close = dispatch(
        &mut registry,
        "wos_close_project",
        &pid,
        serde_json::json!({ "project_id": pid }),
    )
    .await
    .expect("wos_close_project must succeed");

    assert_eq!(close["closed"], serde_json::json!(true));

    // ── 19. List projects again — registry should be empty ────────────────

    let list_after = dispatch(&mut registry, "wos_list_projects", "", serde_json::json!({}))
        .await
        .expect("wos_list_projects must succeed after close");

    assert_eq!(
        list_after["count"],
        serde_json::json!(0),
        "registry must be empty after wos_close_project"
    );
}
