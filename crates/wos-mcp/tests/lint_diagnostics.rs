//! Native lint diagnostics stay stable across schemas and MCP dispatch.

use serde_json::{Value, json};
use wos_authoring::WosProject;
use wos_lint::{LintDiagnostic, LintSeverity, SourceLocation, SuggestedFix, Tier};
use wos_mcp::{dispatch::dispatch, registry::ProjectRegistry};

const TOOLING_ID: &str = "https://wos-spec.org/schemas/tooling/1.0";

fn diagnostic_validators() -> Vec<jsonschema::Validator> {
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json")).unwrap();
    let standalone: Value = serde_json::from_str(include_str!(
        "../../../schemas/lint/wos-lint-diagnostic.schema.json"
    ))
    .unwrap();
    let registry = jsonschema::Registry::new()
        .add(TOOLING_ID, tooling)
        .unwrap()
        .prepare()
        .unwrap();
    [
        standalone,
        json!({"$ref": format!("{TOOLING_ID}#/$defs/lintDiagnostic__Root")}),
    ]
    .iter()
    .map(|schema| {
        jsonschema::options()
            .with_registry(&registry)
            .build(schema)
            .unwrap()
    })
    .collect()
}

fn assert_valid(validator: &jsonschema::Validator, value: &Value) {
    let errors: Vec<_> = validator
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "diagnostic schema rejected {value}: {errors:?}"
    );
}

#[tokio::test]
async fn real_mcp_diagnostics_match_native_output_and_both_schema_views() {
    let mut registry = ProjectRegistry::new();
    let mut document = WosProject::new_kernel().snapshot();
    document
        .agents
        .push(json!({"id": "triage", "fallbackChain": ["reviewer"]}));
    let project = WosProject::from_document(document);
    let snapshot = serde_json::to_string(&project.snapshot()).unwrap();
    let native = wos_lint::lint_document(&snapshot).unwrap();
    assert!(
        !native.is_empty(),
        "negative candidate must emit real findings"
    );
    assert!(native.iter().any(|d| d.severity == LintSeverity::Error));
    assert!(native.iter().any(|d| d.severity == LintSeverity::Warning));
    let expected = serde_json::to_value(&native).unwrap();
    let pid = registry.insert(project).unwrap().to_string();
    let result = dispatch(&mut registry, "wos_lint", &pid, json!({"project_id": pid}))
        .await
        .unwrap();
    assert_eq!(
        result["diagnostics"], expected,
        "MCP must preserve the owning diagnostic model"
    );
    for validator in diagnostic_validators() {
        for diagnostic in expected.as_array().unwrap() {
            assert_valid(&validator, diagnostic);
        }
    }
    assert_eq!(result["block_count"], 0);
    assert_eq!(result["info_count"], 0);
}

#[test]
fn schema_accepts_native_severities_and_fix_variants_and_rejects_drift() {
    let validators = diagnostic_validators();
    let fixes = [
        SuggestedFix::AddProperty {
            path: "/lifecycle/states/review/type".into(),
            value: json!("atomic"),
        },
        SuggestedFix::RemoveProperty {
            path: "/agents/0/targetWorkflow".into(),
        },
        SuggestedFix::ReplaceValue {
            path: "/agents/0/autonomy".into(),
            value: json!("assistive"),
        },
        SuggestedFix::Rename {
            from: "reviewr".into(),
            to: "reviewer".into(),
        },
        SuggestedFix::Custom {
            hint: "Declare a reviewing actor.".into(),
        },
    ];
    for severity in [
        LintSeverity::Block,
        LintSeverity::Error,
        LintSeverity::Warning,
        LintSeverity::Info,
    ] {
        for fix in &fixes {
            let diagnostic = LintDiagnostic {
                rule_id: "WOS-EMBED-TARGET-001",
                severity,
                tier: Tier::T2,
                path: "/agents/0/targetWorkflow".into(),
                message: "Embedded agents do not declare targetWorkflow.".into(),
                suggested_fix: Some(fix.clone()),
                related_docs: vec![
                    "thoughts/adr/0063-embedded-vs-sidecar-identity-boundary.md".into(),
                ],
                source: Some(SourceLocation {
                    document: "workflow.json".into(),
                    line: 2,
                    column: 3,
                }),
            };
            let value = serde_json::to_value(&diagnostic).unwrap();
            for validator in &validators {
                assert_valid(validator, &value);
            }
        }
    }
    // Independent malformed wire inputs: no native serializer creates these.
    let valid = json!({"ruleId":"WOS-EMBED-TARGET-001", "severity":"block", "tier":"T1", "path":"/agents/0", "message":"Invalid embedded identity."});
    for validator in &validators {
        assert_valid(validator, &valid);
        for (field, invalid) in [
            ("path", "$.agents[0]"),
            ("severity", "fatal"),
            ("ruleId", "unregistered code"),
        ] {
            let mut mismatched = valid.clone();
            mismatched[field] = json!(invalid);
            assert!(
                !validator.is_valid(&mismatched),
                "must reject {field} drift: {mismatched}"
            );
        }
        let mut invalid_fix = valid.clone();
        invalid_fix["suggestedFix"] =
            json!({"kind":"replace-value", "path":"$.agents[0]", "value":0});
        assert!(
            !validator.is_valid(&invalid_fix),
            "must reject suggested-fix JSONPath drift"
        );
        invalid_fix["suggestedFix"] = json!({"kind":"replace-value", "path":"/agents/0"});
        assert!(
            !validator.is_valid(&invalid_fix),
            "replace-value requires its replacement value"
        );
    }
}
