//! SCHEMA-OPEN-001 coverage for `x-wos.openStringKind`.

use serde_json::{Value, json};

fn lint(schema: Value) -> Vec<wos_lint::LintDiagnostic> {
    let mut diagnostics = Vec::new();
    wos_lint::rules::schema_doc::check_open_string_kinds(&schema, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    diagnostics
}

fn description_60() -> String {
    "a".repeat(60)
}

#[test]
fn annotated_open_string_leaf_is_clean() {
    let schema = json!({
        "type": "object",
        "properties": {
            "label": {
                "type": "string",
                "description": description_60(),
                "examples": ["alpha"],
                "x-wos": { "openStringKind": "tagLabel" }
            }
        }
    });

    assert!(lint(schema).is_empty());
}

#[test]
fn missing_open_string_kind_fails() {
    let schema = json!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "description": description_60(),
                "examples": ["alpha"]
            }
        }
    });

    let diagnostics = lint(schema);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_id, "SCHEMA-OPEN-001");
    assert_eq!(diagnostics[0].path, "/properties/note");
    assert!(diagnostics[0].message.contains("x-wos.openStringKind"));
    assert!(diagnostics[0].message.contains("allowed values"));
}

#[test]
fn unknown_open_string_kind_fails() {
    let schema = json!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "description": description_60(),
                "examples": ["alpha"],
                "x-wos": { "openStringKind": "not-a-kind" }
            }
        }
    });

    let diagnostics = lint(schema);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_id, "SCHEMA-OPEN-001");
    assert_eq!(diagnostics[0].path, "/properties/note");
    assert!(diagnostics[0].message.contains("invalid"));
    assert!(diagnostics[0].message.contains("not-a-kind"));
}

#[test]
fn enum_leaf_does_not_require_open_string_kind() {
    let schema = json!({
        "type": "object",
        "properties": {
            "mode": {
                "type": "string",
                "enum": ["fast", "slow"],
                "description": description_60(),
                "examples": ["fast"]
            }
        }
    });

    assert!(lint(schema).is_empty());
}
