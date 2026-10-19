//! Native authoring, query, export and exact selected-project replay integration.

use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

#[tokio::test]
async fn full_round_trip_with_10_plus_tool_calls() {
    let mut registry = ProjectRegistry::new();

    // ── 1. Create a project ───────────────────────────────────────────────

    let create = dispatch(
        &mut registry,
        "wos_create_kernel",
        "",
        serde_json::json!({}),
    )
    .await
    .expect("wos_create_kernel must succeed");
    let pid = create["project_id"]
        .as_str()
        .expect("must have project_id")
        .to_string();

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

    // Native governance replaces the obsolete extension writers.
    dispatch(
        &mut registry,
        "wos_edit_workflow",
        &pid,
        serde_json::json!({
            "project_id":pid,"action":"governance","value":{
                "dueProcess":{"scope":"true"},
                "reviewProtocols":[{"id":"appealPath","description":"Standard appeal process"}]
            }
        }),
    )
    .await
    .unwrap();

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

    dispatch(
        &mut registry,
        "wos_edit_workflow",
        &pid,
        serde_json::json!({
            "project_id":pid,"action":"agent_constraint","agent_id":"summaryBot",
            "constraint":{"kind":"prohibition","expression":"true"}
        }),
    )
    .await
    .unwrap();

    dispatch(&mut registry,"wos_edit_workflow",&pid,serde_json::json!({"project_id":pid,"action":"ai_oversight","value":{"disclosure":{"discloseThatAgentAssisted":true}}})).await.unwrap();

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
    assert!(
        graph_str.contains("open"),
        "mermaid graph must mention 'open' state"
    );
    assert!(
        graph_str.contains("closed"),
        "mermaid graph must mention 'closed' state"
    );
    assert!(
        graph_str.contains("startReview"),
        "mermaid graph must mention 'startReview' event"
    );

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

    assert_eq!(constraint_search["matches"].as_array().unwrap().len(), 0); // Historical extension-only search.

    // ── 13. Describe document — verify final counts ───────────────────────

    let describe = dispatch(
        &mut registry,
        "wos_describe_document",
        &pid,
        serde_json::json!({ "project_id": pid }),
    )
    .await
    .expect("wos_describe_document must succeed");

    assert_eq!(
        describe["state_count"],
        serde_json::json!(3),
        "must have 3 states"
    );
    assert_eq!(
        describe["transition_count"],
        serde_json::json!(2),
        "must have 2 transitions"
    );
    assert_eq!(
        describe["actor_count"],
        serde_json::json!(1),
        "must have 1 actor"
    );
    assert_eq!(
        describe["ai_agent_count"],
        serde_json::json!(1),
        "canonical agent writer declares native agents"
    );
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

    let doc_json = export["document"]
        .as_str()
        .expect("must have document string");
    let doc: serde_json::Value =
        serde_json::from_str(doc_json).expect("exported doc must be valid JSON");

    // Verify round-trip structural integrity.
    assert_eq!(doc["$wosWorkflow"], serde_json::json!("1.0"));
    let states = doc["lifecycle"]["states"].as_object().unwrap();
    assert_eq!(states.len(), 3);
    assert_eq!(doc["lifecycle"]["initialState"], serde_json::json!("open"));

    assert_eq!(doc["governance"]["reviewProtocols"][0]["id"], "appealPath");
    assert!(doc["extensions"].get("x-wos-governance").is_none());
    assert!(doc["extensions"].get("x-wos-ai").is_none());
    assert_eq!(doc["agents"][0]["id"], "summaryBot");
    assert_eq!(
        doc["agents"][0]["deonticConstraints"][0]["kind"],
        "prohibition"
    );

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

    // Replay this exported workflow after assigning its required document identity.
    // The scenario expectations are independent. Native agent declarations remain
    // complete even though their canonical AI shapes currently exceed runtime support.
    let mut replay_document = doc.clone();
    replay_document["url"] = serde_json::json!("urn:test:round-trip-replay");
    replay_document["version"] = serde_json::json!("1.0.0");
    let loaded = dispatch(
        &mut registry,
        "wos_load_document",
        "",
        serde_json::json!({
            "json":replay_document.to_string()
        }),
    )
    .await
    .expect("identified round-trip project must reload");
    let replay_id = loaded["project_id"].as_str().unwrap();
    let fixture_json = serde_json::json!({
        "id":"round-trip-current-workflow", "rule":"control",
        "description":"Reviewed lifecycle expectations for the authored workflow", "documents":{},
        "event_sequence":[{"event":"startReview","actor":"reviewer"},{"event":"close","actor":"reviewer"}],
        "expected_transitions":[{"from":"open","target":"under_review","event":"startReview"},
            {"from":"under_review","target":"closed","event":"close"}]
    }).to_string();
    // Current native agent constraints are arrays; the maintained AI runtime still
    // expects its older object model. Preserve the exact candidate and refuse it.
    // Supported execution is covered independently by exact_candidate_replay.rs.
    let refusal = dispatch(
        &mut registry,
        "wos_run_conformance",
        replay_id,
        serde_json::json!({"project_id":replay_id,"fixture_json":fixture_json}),
    )
    .await
    .unwrap_err();
    assert!(
        refusal
            .to_string()
            .contains("unsupported-runtime-configuration"),
        "{refusal}"
    );
    assert!(
        refusal.to_string().contains("AI doc parse error"),
        "{refusal}"
    );
    assert_eq!(
        registry.get(replay_id).unwrap().native_snapshot(),
        replay_document
    );
    dispatch(
        &mut registry,
        "wos_close_project",
        replay_id,
        serde_json::json!({"project_id":replay_id}),
    )
    .await
    .unwrap();

    // ── 17. List projects — assert our project is listed ──────────────────

    let list = dispatch(
        &mut registry,
        "wos_list_projects",
        "",
        serde_json::json!({}),
    )
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

    let list_after = dispatch(
        &mut registry,
        "wos_list_projects",
        "",
        serde_json::json!({}),
    )
    .await
    .expect("wos_list_projects must succeed after close");

    assert_eq!(
        list_after["count"],
        serde_json::json!(0),
        "registry must be empty after wos_close_project"
    );
}

#[tokio::test]
async fn native_agents_description_survives_export_and_reopen() {
    let source = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": "urn:test:native-agents-description",
        "version": "1.0.0",
        "title": "Native agents description",
        "description": "Preserve authored content while describing agents.",
        "impactLevel": "informational",
        "actors": [
            {"id": "summaryBot", "type": "agent", "extensions": {}},
            {"id": "triageBot", "type": "agent", "extensions": {}},
            {"id": "reviewer", "type": "human", "extensions": {}}
        ],
        "lifecycle": {
            "initialState": "review",
            "states": {"review": {"type": "atomic", "transitions": [], "tags": [], "onEntry": [], "onExit": [], "extensions": {}}},
            "milestones": {}
        },
        "agents": [
            {"id": "summaryBot", "modelIdentifier": "summary-model", "autonomy": "assistive"},
            {"id": "triageBot", "modelIdentifier": "triage-model", "autonomy": "readOnly"}
        ],
        "aiOversight": {},
        "extensions": {"x-test-context": {"retained": [1, true, "authored"]}}
    });
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/wos-workflow.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        validator.is_valid(&source),
        "canonical source must satisfy workflow schema"
    );
    let mut registry = ProjectRegistry::new();
    let mut current = source.clone();
    for _ in 0..2 {
        let loaded = dispatch(
            &mut registry,
            "wos_load_document",
            "",
            serde_json::json!({"json": current.to_string()}),
        )
        .await
        .unwrap();
        let pid = loaded["project_id"].as_str().unwrap();
        let description = dispatch(
            &mut registry,
            "wos_describe_document",
            pid,
            serde_json::json!({"project_id": pid}),
        )
        .await
        .unwrap();
        assert_eq!(description["ai_agent_count"], 2);
        assert_eq!(description["actor_count"], 3);
        assert_eq!(description["state_count"], 1);
        let exported = dispatch(
            &mut registry,
            "wos_export_document",
            pid,
            serde_json::json!({"project_id": pid}),
        )
        .await
        .unwrap();
        current = serde_json::from_str(exported["document"].as_str().unwrap()).unwrap();
        assert!(
            validator.is_valid(&current),
            "export must satisfy workflow schema"
        );
        for key in [
            "agents",
            "actors",
            "lifecycle",
            "aiOversight",
            "description",
            "extensions",
        ] {
            assert_eq!(
                current[key], source[key],
                "authored {key} must survive export/reopen"
            );
        }
    }
}
