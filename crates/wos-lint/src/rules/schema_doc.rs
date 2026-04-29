// Rust guideline compliant 2026-02-21

//! `SCHEMA-DOC-001` — meta lint over WOS JSON Schema files.
//!
//! Unlike every other rule in `wos-lint`, which lints WOS *documents* that
//! carry a `$wos*` marker, this rule lints the **JSON Schema files** under
//! `wos-spec/schemas/**/*.json` themselves. The schemas are load-bearing
//! prompt material for LLM authoring: if a property has no description or
//! no example, an LLM cannot reliably generate a valid instance.
//!
//! **Baseline** (every leaf property):
//!   - `description` exists AND `description.len() >= 60`.
//!   - `examples` is a non-empty array.
//!
//! **Critical** (`x-lm.critical == true`):
//!   - `description.len() >= 140`.
//!   - `examples` has at least 2 entries.
//!
//! A *leaf property* is a schema node that declares a concrete `type` and
//! has no `properties`, `items`, `oneOf`, `anyOf`, or `allOf` direct
//! children (those are composite nodes, not leaves). Nodes with `$ref` are
//! skipped — the check applies at the ref target.

use serde_json::Value;

use crate::diagnostic::LintDiagnostic;

/// Rule identifier.
pub const RULE_ID: &str = "SCHEMA-DOC-001";

/// Minimum description length for a baseline leaf property.
const BASELINE_DESCRIPTION_MIN: usize = 60;

/// Minimum description length for a critical leaf property.
const CRITICAL_DESCRIPTION_MIN: usize = 140;

/// Minimum `examples` count for a critical leaf property.
const CRITICAL_EXAMPLES_MIN: usize = 2;

/// Walk a parsed JSON Schema document and collect `SCHEMA-DOC-001`
/// violations for every leaf property.
///
/// The caller is expected to have already parsed the schema file.
pub fn check_schema(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    walk(root, "", diagnostics);
}

/// Count the total number of leaf properties in the parsed schema.
///
/// Mirrors `check_schema`'s walk but only counts leaves; does not produce
/// diagnostics. The companion ratchet for `EXCLUDED_SCHEMAS_CEILINGS` uses
/// this to detect "fill 1, sketch 1" gaming where violation count stays
/// flat but total leaf count grows.
pub fn count_leaves(root: &Value) -> usize {
    let mut count = 0usize;
    walk_count(root, &mut count);
    count
}

fn walk_count(node: &Value, count: &mut usize) {
    let Some(obj) = node.as_object() else { return };
    if obj.contains_key("$ref") {
        return;
    }
    if is_leaf(obj) {
        *count += 1;
    }
    if let Some(Value::Object(props)) = obj.get("properties") {
        for (_, child) in props {
            walk_count(child, count);
        }
    }
    if let Some(Value::Object(pp)) = obj.get("patternProperties") {
        for (_, child) in pp {
            walk_count(child, count);
        }
    }
    if let Some(defs) = obj.get("$defs").and_then(Value::as_object) {
        for (_, child) in defs {
            walk_count(child, count);
        }
    }
    if let Some(defs) = obj.get("definitions").and_then(Value::as_object) {
        for (_, child) in defs {
            walk_count(child, count);
        }
    }
    if let Some(items) = obj.get("items") {
        match items {
            Value::Object(_) => walk_count(items, count),
            Value::Array(arr) => {
                for child in arr {
                    walk_count(child, count);
                }
            }
            _ => {}
        }
    }
    if let Some(additional) = obj.get("additionalProperties")
        && additional.is_object()
    {
        walk_count(additional, count);
    }
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(Value::Array(arr)) = obj.get(combinator) {
            for child in arr {
                walk_count(child, count);
            }
        }
    }
}

/// Recurse into a schema node. `pointer` is the JSON Pointer to `node`
/// within the containing schema document.
fn walk(node: &Value, pointer: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(obj) = node.as_object() else {
        return;
    };

    // A `$ref` replaces the schema entirely — the target is the thing to check.
    if obj.contains_key("$ref") {
        return;
    }

    // If this node itself is a leaf property, check it.
    if is_leaf(obj) {
        check_leaf(obj, pointer, diagnostics);
    }

    // Recurse into every known sub-schema container.
    if let Some(Value::Object(props)) = obj.get("properties") {
        for (name, child) in props {
            let child_pointer = format!("{pointer}/properties/{}", escape_pointer(name));
            walk(child, &child_pointer, diagnostics);
        }
    }
    if let Some(Value::Object(pattern_props)) = obj.get("patternProperties") {
        for (name, child) in pattern_props {
            let child_pointer = format!("{pointer}/patternProperties/{}", escape_pointer(name));
            walk(child, &child_pointer, diagnostics);
        }
    }
    if let Some(defs) = obj.get("$defs").and_then(Value::as_object) {
        for (name, child) in defs {
            let child_pointer = format!("{pointer}/$defs/{}", escape_pointer(name));
            walk(child, &child_pointer, diagnostics);
        }
    }
    if let Some(defs) = obj.get("definitions").and_then(Value::as_object) {
        for (name, child) in defs {
            let child_pointer = format!("{pointer}/definitions/{}", escape_pointer(name));
            walk(child, &child_pointer, diagnostics);
        }
    }
    if let Some(items) = obj.get("items") {
        match items {
            Value::Object(_) => walk(items, &format!("{pointer}/items"), diagnostics),
            Value::Array(arr) => {
                for (i, child) in arr.iter().enumerate() {
                    walk(child, &format!("{pointer}/items/{i}"), diagnostics);
                }
            }
            _ => {}
        }
    }
    if let Some(additional) = obj.get("additionalProperties")
        && additional.is_object()
    {
        walk(
            additional,
            &format!("{pointer}/additionalProperties"),
            diagnostics,
        );
    }
    for combinator in ["oneOf", "anyOf", "allOf"] {
        if let Some(Value::Array(arr)) = obj.get(combinator) {
            for (i, child) in arr.iter().enumerate() {
                walk(child, &format!("{pointer}/{combinator}/{i}"), diagnostics);
            }
        }
    }
}

/// A node is a leaf if it declares a concrete `type` and has no composite
/// children that would make it a structural node.
fn is_leaf(obj: &serde_json::Map<String, Value>) -> bool {
    if !obj.contains_key("type") {
        return false;
    }
    // Composite containers disqualify leaf status.
    for key in [
        "properties",
        "patternProperties",
        "items",
        "oneOf",
        "anyOf",
        "allOf",
    ] {
        if obj.contains_key(key) {
            return false;
        }
    }
    true
}

/// Apply the baseline + critical thresholds to a leaf property.
fn check_leaf(
    obj: &serde_json::Map<String, Value>,
    pointer: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let is_critical = obj
        .get("x-lm")
        .and_then(Value::as_object)
        .and_then(|m| m.get("critical"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let description = obj.get("description").and_then(Value::as_str).unwrap_or("");
    let description_len = description.chars().count();

    let (description_min, examples_min) = if is_critical {
        (CRITICAL_DESCRIPTION_MIN, CRITICAL_EXAMPLES_MIN)
    } else {
        (BASELINE_DESCRIPTION_MIN, 1)
    };

    if description.is_empty() {
        diagnostics.push(LintDiagnostic::t1_error(
            RULE_ID,
            pointer.to_string(),
            format!(
                "leaf property has no `description` (required: {description_min}+ chars{})",
                if is_critical { ", critical" } else { "" }
            ),
        ));
    } else if description_len < description_min {
        diagnostics.push(LintDiagnostic::t1_error(
            RULE_ID,
            pointer.to_string(),
            format!(
                "`description` is {description_len} chars; need at least {description_min}{}",
                if is_critical {
                    " (x-lm.critical=true)"
                } else {
                    ""
                }
            ),
        ));
    }

    let examples_count = obj
        .get("examples")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    if examples_count < examples_min {
        diagnostics.push(LintDiagnostic::t1_error(
            RULE_ID,
            pointer.to_string(),
            format!(
                "leaf property has {examples_count} `examples`; need at least {examples_min}{}",
                if is_critical {
                    " (x-lm.critical=true)"
                } else {
                    ""
                }
            ),
        ));
    }
}

/// Escape a property name for inclusion in a JSON Pointer (RFC 6901).
fn escape_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lint(schema: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_schema(&schema, &mut diagnostics);
        diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(b.rule_id)));
        diagnostics
    }

    /// 60-char string used to hit the baseline description minimum exactly.
    fn description_60() -> String {
        "a".repeat(60)
    }

    /// 140-char string used to hit the critical description minimum exactly.
    fn description_140() -> String {
        "b".repeat(140)
    }

    #[test]
    fn baseline_leaf_missing_description_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "examples": ["hi"] }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, RULE_ID);
        assert_eq!(diagnostics[0].path, "/properties/title");
        assert!(diagnostics[0].message.contains("description"));
    }

    #[test]
    fn baseline_leaf_empty_examples_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": description_60(),
                    "examples": []
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].path, "/properties/title");
        assert!(diagnostics[0].message.contains("examples"));
    }

    #[test]
    fn baseline_leaf_short_description_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "short",
                    "examples": ["ok"]
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("5 chars"));
    }

    #[test]
    fn baseline_leaf_at_boundary_passes() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": description_60(),
                    "examples": ["ok"]
                }
            }
        });
        assert!(lint(schema).is_empty());
    }

    #[test]
    fn critical_leaf_needs_two_examples() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": description_140(),
                    "examples": ["only-one"],
                    "x-lm": { "critical": true }
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("examples"));
        assert!(diagnostics[0].message.contains("critical"));
    }

    #[test]
    fn critical_leaf_short_description_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "b".repeat(139),
                    "examples": ["a", "b"],
                    "x-lm": { "critical": true }
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("139 chars"));
        assert!(diagnostics[0].message.contains("140"));
    }

    #[test]
    fn non_leaf_without_description_is_ignored() {
        // `container` has `properties`, so it is structural, not a leaf.
        let schema = json!({
            "type": "object",
            "properties": {
                "container": {
                    "type": "object",
                    "properties": {
                        "inner": {
                            "type": "string",
                            "description": description_60(),
                            "examples": ["ok"]
                        }
                    }
                }
            }
        });
        assert!(lint(schema).is_empty());
    }

    #[test]
    fn ref_node_is_skipped() {
        // A `$ref` node defers to its target — we must not flag it for
        // missing description/examples at this site.
        let schema = json!({
            "type": "object",
            "properties": {
                "state": { "$ref": "#/$defs/State" }
            },
            "$defs": {
                "State": {
                    "type": "string",
                    "description": description_60(),
                    "examples": ["ok"]
                }
            }
        });
        assert!(lint(schema).is_empty());
    }

    #[test]
    fn empty_object_has_no_errors() {
        assert!(lint(json!({})).is_empty());
    }

    #[test]
    fn known_good_kernel_excerpt_passes() {
        // Minimal excerpt mirroring wos-workflow.schema.json's `$wosWorkflow`
        // and `url` leaves, which are both critical and documented.
        let schema = json!({
            "type": "object",
            "properties": {
                "$wosWorkflow": {
                    "type": "string",
                    "const": "1.0",
                    "description": "WOS Kernel specification version. MUST be '1.0'. Identifies this document as a WOS Kernel Document and pins the specification version to the 1.0 line.",
                    "examples": ["1.0", "1.0"],
                    "x-lm": { "critical": true }
                }
            }
        });
        assert!(lint(schema).is_empty());
    }

    #[test]
    fn defs_leaves_are_walked() {
        let schema = json!({
            "$defs": {
                "Id": { "type": "string" }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 2); // missing description + missing examples
        assert!(diagnostics.iter().all(|d| d.path == "/$defs/Id"));
    }

    #[test]
    fn items_leaves_are_walked() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        });
        let diagnostics = lint(schema);
        // `tags` itself is not a leaf (has `items`); the inner string is.
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.path == "/properties/tags/items")
        );
    }

    #[test]
    fn json_pointer_escapes_special_chars() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a/b": { "type": "string" }
            }
        });
        let diagnostics = lint(schema);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].path, "/properties/a~1b");
    }
}
