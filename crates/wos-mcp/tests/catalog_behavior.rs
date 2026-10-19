// Rust guideline compliant 2026-02-21
//! Independently expected behavior at the catalog, dispatch and process boundaries.

use std::{process::Stdio, time::Duration};

use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};
use wos_mcp::{
    catalog,
    dispatch::dispatch,
    errors::{DispatchError, ToolError},
    registry::ProjectRegistry,
};

/// Bound the real process lifetime and retain both streams on an unsuccessful exit.
async fn run_binary(arguments: &[&str], requests: &[Value]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wos-mcp"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("start Cargo-selected MCP binary");
    let mut input = child.stdin.take().expect("piped stdin");
    for request in requests {
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .expect("write request");
    }
    drop(input);
    let output = timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .expect("MCP process exceeded ten seconds")
        .expect("read MCP process output");
    assert!(
        output.status.success(),
        "MCP exit {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

#[tokio::test]
async fn executable_cli_and_stdio_publish_the_same_catalog() {
    let cli: Value =
        serde_json::from_str(&run_binary(&["--list-tools"], &[]).await).expect("CLI catalog JSON");
    let output = run_binary(
        &[],
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"catalog-test","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ],
    )
    .await;
    let replies: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON-RPC response"))
        .collect();
    assert_eq!(replies.len(), 2, "notification has no response");
    assert_eq!(replies[1]["id"], 2);
    assert_eq!(replies[1]["result"], cli);
    assert_eq!(cli, catalog::descriptors());

    let help: Value =
        serde_json::from_str(&run_binary(&["--tool-help", "wos_edit_workflow"], &[]).await)
            .expect("CLI tool help JSON");
    let listed = cli["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "wos_edit_workflow")
        .expect("native editor is advertised");
    assert_eq!(&help, listed);
    assert_eq!(help, catalog::help("wos_edit_workflow").unwrap());
    assert_eq!(help["x-wos-default-action"], "metadata");
    assert!(catalog::help("wos_add_assertion_gate").is_none());
    assert!(catalog::help("wos_add_due_process_path").is_none());
    assert!(catalog::help("wos_add_deontic_constraint").is_none());
}

#[tokio::test]
async fn omitted_defaults_create_literal_native_values() {
    let mut registry = ProjectRegistry::new();
    let created = dispatch(&mut registry, "wos_create_kernel", "", json!({}))
        .await
        .unwrap();
    assert_eq!(created["draft"], true);
    let project = created["project_id"].as_str().unwrap();
    for (tool, arguments) in [
        (
            "wos_add_state",
            json!({"project_id":project,"state_id":"review"}),
        ),
        (
            "wos_add_actor",
            json!({"project_id":project,"actor_id":"reviewer"}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"fields":{"title":"Review application"}}),
        ),
    ] {
        dispatch(&mut registry, tool, project, arguments)
            .await
            .unwrap();
    }
    let native = registry.get(project).unwrap().native_snapshot();
    assert_eq!(native["lifecycle"]["states"]["review"]["type"], "atomic");
    assert_eq!(native["actors"][0]["type"], "human");
    assert_eq!(native["title"], "Review application");
    assert_eq!(
        catalog::default_argument("wos_add_state", "kind"),
        Some(json!("atomic"))
    );
    assert_eq!(
        catalog::default_argument("wos_add_actor", "kind"),
        Some(json!("human"))
    );

    dispatch(
        &mut registry,
        "wos_edit_workflow",
        project,
        json!({"project_id":project,"action":"metadata","fields":{"title":"Review application"}}),
    )
    .await
    .unwrap();
    assert_eq!(registry.get(project).unwrap().native_snapshot(), native);
}

#[tokio::test]
async fn invalid_inputs_neither_apply_defaults_nor_mutate_the_project() {
    let mut registry = ProjectRegistry::new();
    let created = dispatch(&mut registry, "wos_create_kernel", "", json!({}))
        .await
        .unwrap();
    let project = created["project_id"].as_str().unwrap();
    let before = registry.get(project).unwrap().native_snapshot();
    for (tool, arguments) in [
        (
            "wos_edit_workflow",
            json!({"project_id":project,"action":null,"fields":{"title":"wrong"}}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"action":"unknown","fields":{"title":"wrong"}}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"fields":{"title":"wrong"},"unknown":true}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"fields":{"title":"wrong","unknown":true}}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"fields":[]}),
        ),
        (
            "wos_edit_workflow",
            json!({"project_id":project,"fields":{}}),
        ),
        (
            "wos_add_state",
            json!({"project_id":project,"state_id":"bad","kind":null}),
        ),
        (
            "wos_add_state",
            json!({"project_id":project,"state_id":"bad","kind":"unrecognized"}),
        ),
        (
            "wos_add_actor",
            json!({"project_id":project,"actor_id":"bad","kind":false}),
        ),
        (
            "wos_add_actor",
            json!({"project_id":project,"actor_id":"bad","kind":null}),
        ),
        ("wos_edit_workflow", json!(null)),
    ] {
        let error = dispatch(&mut registry, tool, project, arguments.clone())
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                DispatchError::ToolFailed {
                    source: ToolError::InvalidArguments(_),
                    ..
                }
            ),
            "{tool} accepted an invalid shape or failed elsewhere: {arguments}: {error}"
        );
        assert_eq!(
            registry.get(project).unwrap().native_snapshot(),
            before,
            "mutation after {arguments}"
        );
    }
}

#[tokio::test]
async fn stdio_rejects_unknown_fields_without_creating_a_project_and_keeps_serving() {
    let output = run_binary(
        &[],
        &[
            call(1, "wos_create_kernel", json!({"unexpected":true})),
            call(2, "wos_list_projects", json!({})),
            call(3, "wos_ping", json!(null)),
            call(4, "wos_no_such_tool", json!({})),
            call(5, "wos_ping", json!({})),
        ],
    )
    .await;
    let replies: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 5);
    assert_eq!(replies[0]["result"]["isError"], true);
    assert!(replies[0].get("error").is_none());
    let projects: Value =
        serde_json::from_str(replies[1]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(projects, json!({"projects":[],"count":0}));
    assert_eq!(replies[2]["result"]["isError"], true);
    assert_eq!(replies[3]["error"]["code"], -32602);
    let pong: Value =
        serde_json::from_str(replies[4]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(pong, json!({"pong":true}));
}
