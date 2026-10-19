// Rust guideline compliant 2026-02-21
//! Behavioral controls for canonical static validation and native dependencies.
use serde_json::{Value, json};
use wos_lint::studio_api::*;

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/shared-validation/{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}
fn run(
    candidate: &Value,
    dependencies: &[ResolvedDependency],
    scope: ValidationScope,
) -> ValidationReport {
    validate(ValidationRequest {
        candidate,
        dependencies,
        scope,
    })
}
fn definition() -> ResolvedDependency {
    ResolvedDependency {
        role: DependencyRole::Definition,
        url: "urn:test:definition".into(),
        version: "1.2.0".into(),
        document_json: json!({"$formspec":"1.0","url":"urn:test:definition","version":"1.2.0","status":"draft","title":"Independent Definition","items":[{"key":"name","type":"field","dataType":"string","label":"Name"}]}),
        definition: None,
    }
}
fn mapping() -> ResolvedDependency {
    ResolvedDependency {
        role: DependencyRole::Mapping,
        url: "urn:test:mapping".into(),
        version: "3.0.0".into(),
        document_json: json!({"$formspecMapping":"1.0","version":"3.0.0","definitionRef":"urn:test:definition","definitionVersion":"^1.0.0","targetSchema":{"format":"json"},"rules":[{"sourcePath":"name","targetPath":"name","transform":"preserve"}]}),
        definition: Some(DefinitionSelection {
            url: "urn:test:definition".into(),
            version: "1.2.0".into(),
        }),
    }
}

#[test]
fn literal_candidates_enforce_schema_and_preserve_inputs() {
    for (name, expected) in [
        ("positive", CheckStatus::Pass),
        ("stored-event", CheckStatus::Fail),
        ("invalid-pattern", CheckStatus::Fail),
        ("invalid-hold", CheckStatus::Fail),
    ] {
        let input = fixture(name);
        let original = input.clone();
        let result = run(&input, &[], ValidationScope::Project);
        assert_eq!(result.stages[0].status, expected, "{name}: {result:?}");
        assert_eq!(input, original);
        if name == "positive" {
            assert_eq!(result.status, CheckStatus::Pass, "{result:?}");
        }
        if name == "stored-event" {
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|d| d.path == "/lifecycle/states/review/transitions/0/event")
            );
            assert!(serde_json::from_value::<wos_core::KernelDocument>(input).is_ok());
        }
    }
}

#[test]
fn root_pointer_and_native_diagnostics_match_owning_schema() {
    let result = run(&json!([]), &[], ValidationScope::Document);
    assert_eq!(result.status, CheckStatus::Fail);
    assert_eq!(result.diagnostics[0].path, "");
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
    let id = tooling["$id"].as_str().unwrap().to_owned();
    let registry = jsonschema::Registry::new()
        .add(id.clone(), tooling)
        .unwrap()
        .prepare()
        .unwrap();
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&json!({"$ref":format!("{id}#/$defs/lintDiagnostic__Root")}))
        .unwrap();
    assert!(validator.is_valid(&serde_json::to_value(&result.diagnostics[0]).unwrap()));
    let mut drift = serde_json::to_value(&result.diagnostics[0]).unwrap();
    drift["path"] = json!("$.root");
    assert!(!validator.is_valid(&drift));
    drift["path"] = json!("\n");
    assert!(
        !validator.is_valid(&drift),
        "newline is not the empty root pointer"
    );
    let candidate = fixture("positive");
    let native = wos_lint::lint_document(&candidate.to_string()).unwrap();
    let checked = run(&candidate, &[], ValidationScope::Document);
    assert_eq!(
        serde_json::to_value(native).unwrap(),
        serde_json::to_value(&checked.stages[1].diagnostics).unwrap()
    );
}

#[test]
fn requested_scope_and_actual_typed_skips_prevent_false_completeness() {
    let mut input = fixture("positive");
    let document = run(&input, &[], ValidationScope::Document);
    assert!(
        document
            .stages
            .iter()
            .any(|s| s.name == "t2" && s.status == CheckStatus::NotRun && !s.required)
    );
    assert!(
        document
            .stages
            .iter()
            .any(|s| s.name == "runtime-replay" && s.status == CheckStatus::NotRun)
    );
    input["maxRelationshipEventDepth"] = json!(4294967296u64);
    let result = run(&input, &[], ValidationScope::Project);
    assert_eq!(result.stages[0].status, CheckStatus::Pass);
    assert_eq!(result.status, CheckStatus::NotRun, "{result:?}");
    for name in ["t1-kernel", "t2-kernel"] {
        assert!(
            result.stages.iter().any(|s| s.name == name
                && s.status == CheckStatus::NotRun
                && !s.reasons.is_empty())
        );
    }
    input["lifecycle"]["states"]["review"]["transitions"][0]["guard"] = json!("@@bad");
    let with_error = run(&input, &[], ValidationScope::Project);
    assert!(with_error.diagnostics.iter().any(|d| d.rule_id == "K-012"));
    assert!(
        with_error
            .stages
            .iter()
            .any(|s| s.name == "t1-kernel" && s.status == CheckStatus::NotRun)
    );
}

#[test]
fn exact_selected_definition_pins_and_mapping_ranges_are_distinct() {
    let mut candidate = fixture("positive");
    candidate["intake"] = json!({"contractRef":"urn:test:definition","contractVersion":"1.2.0"});
    let deps = vec![definition(), mapping()];
    let saved = serde_json::to_value(&deps).unwrap();
    assert_eq!(
        run(&candidate, &deps, ValidationScope::Project).status,
        CheckStatus::Pass,
        "{:?}",
        run(&candidate, &deps, ValidationScope::Project)
    );
    let mut reversed = deps.clone();
    reversed.reverse();
    assert_eq!(
        serde_json::to_value(run(&candidate, &deps, ValidationScope::Project)).unwrap(),
        serde_json::to_value(run(&candidate, &reversed, ValidationScope::Project)).unwrap()
    );
    assert_eq!(serde_json::to_value(&deps).unwrap(), saved);
    assert_eq!(
        run(&candidate, &[], ValidationScope::Project).status,
        CheckStatus::Fail
    );
    let mut bad = deps.clone();
    bad[0].document_json["version"] = json!("1.3.0");
    assert_eq!(
        run(&candidate, &bad, ValidationScope::Project).status,
        CheckStatus::Fail
    );
    let mut bad = deps.clone();
    bad[0].document_json["url"] = json!("urn:other");
    assert_eq!(
        run(&candidate, &bad, ValidationScope::Project).status,
        CheckStatus::Fail
    );
    let mut bad = deps.clone();
    bad.push(definition());
    assert_eq!(
        run(&candidate, &bad, ValidationScope::Project).status,
        CheckStatus::Fail
    );
    let mut bad = deps.clone();
    bad[1].document_json["definitionVersion"] = json!("^2.0.0");
    assert_eq!(
        run(&candidate, &bad, ValidationScope::Project).status,
        CheckStatus::Fail
    );
    let mut bad = deps.clone();
    bad[1].definition.as_mut().unwrap().url = "urn:other".into();
    assert_eq!(
        run(&candidate, &bad, ValidationScope::Project).status,
        CheckStatus::Fail
    );
    candidate["intake"]
        .as_object_mut()
        .unwrap()
        .remove("contractVersion");
    assert_eq!(
        run(&candidate, &deps, ValidationScope::Project).status,
        CheckStatus::Pass
    );
}

#[test]
fn node_semver_ranges_match_mapping_consumer_prerelease_posture() {
    let candidate = fixture("positive");
    for range in [
        "1.2.0",
        ">=1.0.0 <2.0.0",
        "^1.0.0",
        "1.x",
        "^2.0.0 || ^1.0.0",
        "1.0.0 - 1.9.0",
    ] {
        let mut deps = vec![definition(), mapping()];
        deps[1].document_json["definitionVersion"] = json!(range);
        assert_eq!(
            run(&candidate, &deps, ValidationScope::Project).status,
            CheckStatus::Pass,
            "{range}"
        );
    }
    let mut deps = vec![definition(), mapping()];
    deps[0].version = "1.2.0-beta.1".into();
    deps[0].document_json["version"] = json!("1.2.0-beta.1");
    deps[1].definition.as_mut().unwrap().version = "1.2.0-beta.1".into();
    assert_eq!(
        run(&candidate, &deps, ValidationScope::Project).status,
        CheckStatus::Fail,
        "default node-semver excludes prerelease from broad range"
    );
    deps[1].document_json["definitionVersion"] = json!("^1.2.0-beta.1");
    assert_eq!(
        run(&candidate, &deps, ValidationScope::Project).status,
        CheckStatus::Pass
    );
    deps[1].document_json["definitionVersion"] = json!("definitely not a range");
    let result = run(&candidate, &deps, ValidationScope::Project);
    assert!(result.stages.iter().any(|s| s.status == CheckStatus::NotRun
        && s.reasons.iter().any(|r| r.contains("version association"))));
}

#[test]
fn formspec_native_results_preserve_exact_owner_codes_paths_and_fixes() {
    let candidate = fixture("positive");
    let mut deps = vec![definition(), mapping()];
    deps[1].document_json["rules"][0]["sourcePath"] = json!("unknown");
    let result = run(&candidate, &deps, ValidationScope::Project);
    assert_eq!(result.status, CheckStatus::Fail);
    let report = result
        .stages
        .iter()
        .find(|s| s.document.as_deref() == Some("urn:test:mapping") && s.formspec.is_some())
        .unwrap()
        .formspec
        .as_ref()
        .unwrap();
    let options = formspec_lint::LintOptions {
        definition_document: Some(deps[0].document_json.clone()),
        ..Default::default()
    };
    let native = formspec_lint::lint_with_options(&deps[1].document_json, &options);
    assert_eq!(
        *report,
        formspec_lint::lint_result_to_json_value(&native, formspec_core::JsonWireStyle::JsCamel)
    );
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"].as_str().unwrap().starts_with('E')
                && d["path"].as_str().unwrap().starts_with('$'))
    );
    deps[0].document_json["items"][0]["dataType"] = json!("invalid-type");
    assert!(
        run(&candidate, &deps, ValidationScope::Project)
            .stages
            .iter()
            .any(|s| s.formspec.as_ref().is_some_and(|f| f["valid"] == false))
    );
}

#[test]
fn external_mapping_source_is_supported_and_unknown_resources_are_explicit() {
    let candidate = fixture("positive");
    let mut dependency = mapping();
    dependency.definition = None;
    dependency.document_json = json!({"$formspecMapping":"1.0","version":"3.0.0","sourceSchema":{"url":"urn:test:source","version":"1.0.0","schema":{"type":"object","properties":{"name":{"type":"string"}}}},"targetSchema":{"format":"json"},"rules":[{"sourcePath":"name","targetPath":"name","transform":"preserve"}]});
    assert_eq!(
        run(&candidate, &[dependency.clone()], ValidationScope::Project).status,
        CheckStatus::Pass,
        "{:?}",
        run(&candidate, &[dependency.clone()], ValidationScope::Project)
    );
    for reference in [
        "https://invalid.example/never-fetch.json",
        "file:///definitely-not-readable-schema.json",
    ] {
        dependency.document_json["sourceSchema"]["schema"] = json!({"$ref":reference});
        let report = run(&candidate, &[dependency.clone()], ValidationScope::Project);
        assert_eq!(report.status, CheckStatus::NotRun);
        assert!(
            report
                .stages
                .iter()
                .any(|s| s.reasons.iter().any(|r| r.contains("offline")))
        );
    }
}

#[test]
fn formats_conditionals_patterns_and_governance_owner_link_are_enforced() {
    let positive = fixture("positive");
    let mut uri = positive.clone();
    uri["url"] = json!("not a uri");
    assert_eq!(
        run(&uri, &[], ValidationScope::Document).stages[0].status,
        CheckStatus::Fail
    );
    let mut extra = positive.clone();
    extra["unknownProperty"] = json!(true);
    assert_eq!(
        run(&extra, &[], ValidationScope::Document).stages[0].status,
        CheckStatus::Fail
    );
    let mut conditional = positive.clone();
    conditional["impactLevel"] = json!("rights-impacting");
    assert_eq!(
        run(&conditional, &[], ValidationScope::Document).stages[0].status,
        CheckStatus::Fail
    );
    for duration in ["P1D", "PT1H"] {
        let mut input = fixture("invalid-pattern");
        input["agents"][0]["confidenceDecay"]["halfLife"] = json!(duration);
        assert_eq!(
            run(&input, &[], ValidationScope::Document).stages[0].status,
            CheckStatus::Pass,
            "{duration}: {:?}",
            run(&input, &[], ValidationScope::Document)
        );
    }
    for duration in ["P", "PT", "PT0S"] {
        let mut input = fixture("invalid-pattern");
        input["agents"][0]["confidenceDecay"]["halfLife"] = json!(duration);
        assert_eq!(
            run(&input, &[], ValidationScope::Document).stages[0].status,
            CheckStatus::Fail,
            "{duration}"
        );
    }
    for duration in ["P1D", "PT1H", "indefinite", "P5BD"] {
        let mut hold = fixture("invalid-hold");
        hold["governance"]["holds"]["policies"][0]["expectedDuration"] = json!(duration);
        assert_eq!(
            run(&hold, &[], ValidationScope::Document).stages[0].status,
            CheckStatus::Pass,
            "{duration}"
        );
    }
}

#[test]
fn project_t2_and_supported_reference_surfaces_execute() {
    let mut input = fixture("positive");
    input["lifecycle"]["states"]["review"]["onEntry"] =
        json!([{"action":"createTask","assignTo":"missing"}]);
    let result = run(&input, &[], ValidationScope::Project);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.tier == wos_lint::Tier::T2)
    );
    let mut input = fixture("positive");
    input["bindings"] = json!([{"contractRef":"urn:missing:output"}]);
    let result = run(&input, &[], ValidationScope::Project);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.path == "/bindings/0/contractRef")
    );
    let mut input = fixture("positive");
    input["contracts"] = json!({"custom":{"binding":"x-vendor","ref":"urn:custom"}});
    assert_eq!(
        run(&input, &[], ValidationScope::Project).status,
        CheckStatus::NotRun
    );
}

#[test]
fn fractional_duration_schema_format_discrepancy_is_observed() {
    let mut input = fixture("invalid-pattern");
    input["agents"][0]["confidenceDecay"]["halfLife"] = json!("PT0.5S");
    let result = run(&input, &[], ValidationScope::Document);
    assert_eq!(
        result.stages[0].status,
        CheckStatus::Pass,
        "native ISO8601 fractional duration and canonical schema must agree: {result:?}"
    );
}

#[test]
fn governance_and_native_ai_coverage_are_reported_at_actual_branches() {
    let mut input = fixture("positive");
    input["governance"] = json!({});
    assert_eq!(
        run(&input, &[], ValidationScope::Project).status,
        CheckStatus::Pass
    );
    input["governance"] = json!({"delegation":{"delegations":[{}]}});
    let report = run(&input, &[], ValidationScope::Project);
    assert_eq!(report.stages[0].status, CheckStatus::Pass);
    assert!(
        report
            .stages
            .iter()
            .any(|s| s.name == "t1-governance" && s.status == CheckStatus::NotRun)
    );
    assert!(
        report
            .stages
            .iter()
            .any(|s| s.name == "t2-governance" && s.status == CheckStatus::NotRun)
    );
    let mut legal_hold = fixture("invalid-hold");
    legal_hold["governance"]["holds"]["policies"][0]["expectedDuration"] = json!("indefinite");
    let report = run(&legal_hold, &[], ValidationScope::Project);
    assert_eq!(report.stages[0].status, CheckStatus::Pass);
    assert_eq!(report.status, CheckStatus::NotRun);
    let mut ai = fixture("invalid-pattern");
    ai["agents"][0]["confidenceDecay"]["halfLife"] = json!("PT1H");
    ai["agents"][0]["id"] = json!("notJoined");
    let report = run(&ai, &[], ValidationScope::Project);
    assert_eq!(report.stages[0].status, CheckStatus::Pass);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "WOS-AI-NATIVE-001")
    );
    assert!(
        report
            .stages
            .iter()
            .any(|s| s.name == "t2-native-ai" && s.status == CheckStatus::NotRun)
    );
}

#[test]
fn canonical_sidecar_schema_runs_offline_and_joins_the_selected_workflow() {
    let input = fixture("positive");
    let mut dependency = ResolvedDependency {
        role: DependencyRole::Delivery,
        url: "urn:test:delivery-locator".into(),
        version: "1.0.0".into(),
        document_json: json!({"$wosDelivery":"1.0","targetWorkflow":"urn:test:shared-validation","version":"1.0.0","calendar":{"timezone":"UTC","workWeek":["monday"]}}),
        definition: None,
    };
    let result = run(&input, &[dependency.clone()], ValidationScope::Project);
    assert_eq!(result.status, CheckStatus::Pass, "{result:?}");
    assert!(
        result
            .stages
            .iter()
            .any(|s| s.schema_id.as_deref() == Some("https://wos-spec.org/schemas/delivery/1.0"))
    );
    dependency.document_json["targetWorkflow"] = json!("urn:wrong:workflow");
    assert_eq!(
        run(&input, &[dependency], ValidationScope::Project).status,
        CheckStatus::Fail
    );
}

#[test]
fn native_hold_diagnostic_uses_the_real_referencing_pointer() {
    let mut input = fixture("invalid-hold");
    input["governance"]["holds"]["policies"][0]["expectedDuration"] = json!("invalid");
    let result = run(&input, &[], ValidationScope::Document);
    let finding = result
        .diagnostics
        .iter()
        .find(|d| d.rule_id == "G-055")
        .unwrap();
    assert_eq!(
        finding.path,
        "/governance/holds/policies/0/expectedDuration"
    );
    assert!(input.pointer(&finding.path).is_some());
}

#[test]
fn lifecycle_actions_resolve_only_native_reference_fields_and_require_task_pins() {
    let mut input = fixture("positive");
    input["contracts"] = json!({"form":{"binding":"formspec","ref":"urn:test:definition","definitionVersion":"1.2.0"}});
    input["lifecycle"]["states"]["review"]["onEntry"] = json!([{"action":"createTask","contractRef":"form","responseMappingRef":"urn:test:mapping","data":{"action":"createTask","contractRef":"inert"}}]);
    let deps = vec![definition(), mapping()];
    let result = run(&input, &deps, ValidationScope::Project);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "WOS-REF-001"),
        "{result:?}"
    );
    input["contracts"]["form"]
        .as_object_mut()
        .unwrap()
        .remove("definitionVersion");
    let result = run(&input, &deps, ValidationScope::Project);
    assert!(result.diagnostics.iter().any(|d| d.rule_id == "WOS-REF-001"
        && d.path == "/lifecycle/states/review/onEntry/0/contractRef"));
}

#[test]
fn positive_iso_duration_lexical_neighbors_and_native_support_agree() {
    for duration in [
        "PT0.5S", "PT1.5H", "P1DT0.5S", "P1Y1D", "PT1H1S", "P1W", "P0.5W",
    ] {
        let mut input = fixture("invalid-pattern");
        input["agents"][0]["confidenceDecay"]["halfLife"] = json!(duration);
        let result = run(&input, &[], ValidationScope::Project);
        assert_eq!(
            result.stages[0].status,
            CheckStatus::Pass,
            "{duration}: {result:?}"
        );
        assert!(
            wos_core::eval::parse_iso_duration_to_ms(duration).is_ok(),
            "{duration}"
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "WOS-AI-NATIVE-001"),
            "{duration}: {result:?}"
        );
    }
    assert_eq!(
        wos_core::eval::parse_iso_duration_to_ms("PT0.5S").unwrap(),
        500
    );
    assert_eq!(
        wos_core::eval::parse_iso_duration_to_ms("PT1H1S").unwrap(),
        3_601_000
    );
    assert_eq!(
        wos_core::eval::parse_iso_duration_to_ms("P1Y1D").unwrap(),
        31_622_400_000
    );
    for duration in [
        "P",
        "PT",
        "PT0S",
        "P1DT",
        "P1YT",
        "P1WT",
        "P1W1D",
        "P1WT1H",
        "P1Y1W",
        "PT1.5H30M",
        "P1.5DT1H",
        "P1.5Y1M",
        "PT1S1H",
        "PT1H1H",
        "P1D1Y",
        "PT.5S",
        "PT1.S",
        "PT1..5S",
        "PT1S\n",
        "PT1S\r",
        " PT1S",
    ] {
        let mut input = fixture("invalid-pattern");
        input["agents"][0]["confidenceDecay"]["halfLife"] = json!(duration);
        assert_eq!(
            run(&input, &[], ValidationScope::Document).stages[0].status,
            CheckStatus::Fail,
            "{duration:?}"
        );
    }
    let mut tiny = fixture("invalid-pattern");
    tiny["agents"][0]["confidenceDecay"]["halfLife"] = json!("PT0.0001S");
    let result = run(&tiny, &[], ValidationScope::Project);
    assert_eq!(result.stages[0].status, CheckStatus::Pass);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.rule_id == "WOS-AI-NATIVE-001")
    );
    assert_eq!(result.status, CheckStatus::Fail);
}

#[test]
fn unsupported_required_source_format_is_not_a_schema_pass() {
    let candidate = fixture("positive");
    let mut dependency = mapping();
    dependency.definition = None;
    dependency.document_json = json!({"$formspecMapping":"1.0","version":"3.0.0","sourceSchema":{"url":"urn:test:source","version":"1.0.0","schema":{"type":"string","format":"unavailable-required-format"}},"targetSchema":{"format":"json"},"rules":[{"sourcePath":"name","targetPath":"name","transform":"preserve"}]});
    let result = run(&candidate, &[dependency], ValidationScope::Project);
    assert_eq!(result.status, CheckStatus::NotRun);
    assert!(result.stages.iter().any(|s| {
        s.reasons
            .iter()
            .any(|r| r.contains("unavailable-required-format"))
    }));
}
