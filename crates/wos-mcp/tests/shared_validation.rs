//! Literal shared candidates exercise the actual owning API and MCP adapter.
use serde_json::{Value, json};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};
#[path = "../../wos-conformance/tests/support/local_schemas.rs"]
mod local_schemas;

fn fixture(name: &str) -> Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../wos-lint/tests/fixtures/shared-validation");
    serde_json::from_str(&std::fs::read_to_string(root.join(format!("{name}.json"))).unwrap())
        .unwrap()
}

#[tokio::test]
async fn actual_mcp_rejects_authored_stored_event_and_canonical_pattern() {
    for name in ["stored-event", "invalid-pattern", "invalid-hold"] {
        let document = fixture(name);
        let mut registry = ProjectRegistry::new();
        let project = wos_authoring::WosProject::from_native_document(document).unwrap();
        let pid = registry.insert(project).unwrap().to_string();
        let result = dispatch(&mut registry, "wos_lint", &pid, json!({"project_id":pid}))
            .await
            .unwrap();
        assert_eq!(
            result["validation"]["stages"][0]["status"], "fail",
            "{name}: {result}"
        );
    }
}

#[test]
fn canonical_workflow_schema_enforces_embedded_hold_duration() {
    let current: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-workflow.schema.json")).unwrap();
    let input = fixture("invalid-hold");
    assert!(!local_schemas::validator(&current).is_valid(&input));
}

#[tokio::test]
async fn shared_api_direct_handler_and_actual_dispatch_have_identical_owning_results() {
    use wos_lint::studio_api::{ValidationRequest, ValidationScope, validate};
    for name in [
        "positive",
        "rights-human",
        "stored-event",
        "invalid-pattern",
        "unresolved-reference",
        "invalid-hold",
    ] {
        let candidate = fixture(name);
        let original = candidate.clone();
        let expected = serde_json::to_value(validate(ValidationRequest {
            candidate: &candidate,
            dependencies: &[],
            scope: ValidationScope::Project,
        }))
        .unwrap();
        let mut registry = ProjectRegistry::new();
        let pid = registry
            .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
            .unwrap()
            .to_string();
        let args = json!({"project_id":pid,"scope":"project","resolved_dependencies":[]});
        let dispatched = dispatch(&mut registry, "wos_lint", &pid, args.clone())
            .await
            .unwrap();
        let direct = wos_mcp::tools::wos_lint(&mut registry, &pid, args)
            .await
            .unwrap();
        assert_eq!(dispatched["validation"], expected, "{name}");
        assert_eq!(direct, dispatched, "{name}");
        assert_eq!(registry.get(&pid).unwrap().native_snapshot(), original);
    }
}

#[tokio::test]
async fn actual_mcp_uses_selected_dependencies_and_refuses_unsupported_roles() {
    let candidate = fixture("unresolved-reference");
    let mut registry = ProjectRegistry::new();
    let pid = registry
        .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
        .unwrap()
        .to_string();
    let dependency = json!({"role":"definition","url":"urn:test:missing-definition","version":"1.0.0","document_json":{"$formspec":"1.0","url":"urn:test:missing-definition","version":"1.0.0","status":"draft","title":"Selected definition","items":[]}});
    let args = json!({"project_id":pid,"scope":"project","resolved_dependencies":[dependency]});
    let result = dispatch(&mut registry, "wos_lint", &pid, args.clone())
        .await
        .unwrap();
    assert_eq!(result["validation"]["status"], "pass", "{result}");
    let mut bad = args;
    bad["resolved_dependencies"][0]["role"] = json!("workflow");
    assert!(
        dispatch(&mut registry, "wos_lint", &pid, bad)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn actual_mcp_retains_real_empty_root_pointer() {
    let mut registry = ProjectRegistry::new();
    let pid = registry
        .insert(wos_authoring::WosProject::from_native_document(json!({})).unwrap())
        .unwrap()
        .to_string();
    let result = dispatch(&mut registry, "wos_lint", &pid, json!({"project_id":pid}))
        .await
        .unwrap();
    assert_eq!(result["validation"]["status"], "fail");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == "")
    );
}

#[tokio::test]
async fn fractional_and_omitted_zero_components_pass_actual_mcp_schema_check() {
    for duration in ["PT0.5S", "PT1.5H", "P1Y1D", "PT1H1S"] {
        let mut candidate = fixture("invalid-pattern");
        candidate["agents"][0]["confidenceDecay"]["halfLife"] = json!(duration);
        let original = candidate.clone();
        let mut registry = ProjectRegistry::new();
        let pid = registry
            .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
            .unwrap()
            .to_string();
        let result = dispatch(
            &mut registry,
            "wos_lint",
            &pid,
            json!({"project_id":pid,"scope":"project"}),
        )
        .await
        .unwrap();
        assert_eq!(
            result["validation"]["stages"][0]["status"], "pass",
            "{duration}: {result}"
        );
        assert_eq!(registry.get(&pid).unwrap().native_snapshot(), original);
    }
}

#[tokio::test]
async fn actual_dispatch_preserves_mapping_selection_and_native_formspec_report() {
    use wos_lint::studio_api::{ResolvedDependency, ValidationRequest, ValidationScope, validate};
    for (source_path, range, status) in [
        ("name", "^1.0.0", "pass"),
        ("missing", "^1.0.0", "fail"),
        ("name", "^2.0.0", "fail"),
    ] {
        let candidate = fixture("positive");
        let original = candidate.clone();
        let dependencies = json!([
            {"role":"definition","url":"urn:test:definition","version":"1.2.0","document_json":{"$formspec":"1.0","url":"urn:test:definition","version":"1.2.0","status":"draft","title":"Selected Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}},
            {"role":"mapping","url":"urn:test:external-mapping-locator","version":"3.0.0","definition":{"url":"urn:test:definition","version":"1.2.0"},"document_json":{"$formspecMapping":"1.0","version":"3.0.0","definitionRef":"urn:test:definition","definitionVersion":range,"targetSchema":{"format":"json"},"rules":[{"sourcePath":source_path,"targetPath":"name","transform":"preserve"}]}}
        ]);
        let selected: Vec<ResolvedDependency> =
            serde_json::from_value(dependencies.clone()).unwrap();
        let selected_before = serde_json::to_value(&selected).unwrap();
        let expected = serde_json::to_value(validate(ValidationRequest {
            candidate: &candidate,
            dependencies: &selected,
            scope: ValidationScope::Project,
        }))
        .unwrap();
        let mut registry = ProjectRegistry::new();
        let pid = registry
            .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
            .unwrap()
            .to_string();
        let result = dispatch(
            &mut registry,
            "wos_lint",
            &pid,
            json!({"project_id":pid,"scope":"project","resolved_dependencies":dependencies}),
        )
        .await
        .unwrap();
        assert_eq!(
            result["validation"], expected,
            "source={source_path}, range={range}"
        );
        assert_eq!(result["validation"]["status"], status);
        assert_eq!(registry.get(&pid).unwrap().native_snapshot(), original);
        assert_eq!(serde_json::to_value(&selected).unwrap(), selected_before);
        let mapping = result["validation"]["stages"]
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
                    .any(|finding| finding["code"].as_str().unwrap().starts_with('E')
                        && finding["path"].as_str().unwrap().starts_with('$')
                        && finding.get("suggestedFix").is_some()
                        && finding.get("specRef").is_some())
            );
        }
    }
}

#[tokio::test]
async fn actual_dispatch_agrees_on_protected_intake_mapping_context() {
    use wos_lint::studio_api::{ResolvedDependency, ValidationRequest, ValidationScope, validate};
    for (source, version, definition_url, expected_status) in [
        (
            "handoff.responseRef",
            "1.2.0",
            "urn:test:definition",
            "pass",
        ),
        ("response.data.name", "1.2.0", "urn:test:definition", "pass"),
        (
            "caseReservation.caseReference",
            "1.2.0",
            "urn:test:definition",
            "pass",
        ),
        ("handoff.missing", "1.2.0", "urn:test:definition", "fail"),
        (
            "response.data.missing",
            "1.2.0",
            "urn:test:definition",
            "fail",
        ),
        ("response.data.name", "1.3.0", "urn:test:definition", "fail"),
        ("response.data.name", "1.2.0", "urn:foreign", "fail"),
    ] {
        let mut candidate = fixture("positive");
        candidate["intake"] = json!({"contractRef":"urn:test:definition","contractVersion":"1.2.0","initialCaseMappingRef":"urn:test:initial-mapping"});
        let dependencies = json!([
            {"role":"definition","url":"urn:test:definition","version":"1.2.0","document_json":{"$formspec":"1.0","url":"urn:test:definition","version":"1.2.0","status":"draft","title":"Selected Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}},
            {"role":"mapping","url":"urn:test:initial-mapping","version":"3.0.0","definition":{"url":"urn:test:definition","version":"1.2.0"},"document_json":{"$formspecMapping":"1.0","version":"3.0.0","definitionRef":definition_url,"definitionVersion":version,"targetSchema":{"format":"json"},"rules":[{"sourcePath":source,"targetPath":"result","transform":"preserve"}]}}
        ]);
        let selected: Vec<ResolvedDependency> =
            serde_json::from_value(dependencies.clone()).unwrap();
        let expected = serde_json::to_value(validate(ValidationRequest {
            candidate: &candidate,
            dependencies: &selected,
            scope: ValidationScope::Project,
        }))
        .unwrap();
        let original = candidate.clone();
        let mut registry = ProjectRegistry::new();
        let pid = registry
            .insert(wos_authoring::WosProject::from_native_document(candidate).unwrap())
            .unwrap()
            .to_string();
        let result = dispatch(
            &mut registry,
            "wos_lint",
            &pid,
            json!({"project_id":pid,"scope":"project","resolved_dependencies":dependencies}),
        )
        .await
        .unwrap();
        assert_eq!(
            result["validation"], expected,
            "{source}, {version}, {definition_url}"
        );
        assert_eq!(result["validation"]["status"], expected_status);
        let stage = result["validation"]["stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stage| stage["document"] == "urn:test:initial-mapping")
            .unwrap();
        assert_eq!(stage["required"], true);
        if expected_status == "pass" {
            assert_eq!(stage["formspec"]["valid"], true);
        }
        if source.ends_with("missing") {
            assert_eq!(stage["status"], "fail");
            assert!(
                stage["formspec"]["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["code"] == "E1110" && d["path"] == "$.rules[0].sourcePath")
            );
        }
        assert_eq!(registry.get(&pid).unwrap().native_snapshot(), original);
    }
}
