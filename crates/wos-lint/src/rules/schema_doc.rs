// Rust guideline compliant 2026-02-21

//! `SCHEMA-DOC-001` — meta lint over WOS JSON Schema files.
//!
//! Unlike every other rule in `wos-lint`, which lints WOS *documents* that
//! carry a `$wos*` marker, this rule lints the **JSON Schema files** under
//! `work-spec/schemas/**/*.json` themselves. The schemas are load-bearing
//! prompt material for LLM authoring: if a property has no description or
//! no example, an LLM cannot reliably generate a valid instance.
//!
//! String leaves also reject prose-only closed vocabularies: if a
//! description says "Allowed values" / "Valid values" / similar, the schema
//! must back that prose with `enum`, `const`, `oneOf`, `anyOf`, or `pattern`.
//!
//! SCHEMA-OPEN-001 separately audits intentional open string leaves via
//! `x-wos.openStringKind`.
//!
//! **Baseline** (every leaf property):
//!   - `description` exists AND `description.len() >= 60`.
//!   - `examples` is a non-empty array.
//!
//! **Critical** (`x-lm.critical == true`):
//!   - `description.len() >= 140`.
//!   - `examples` has at least 2 entries.
//!
//! A *leaf property* declares a concrete `type` without a structural schema
//! keyword such as `properties`, `items`, or `allOf`. Reference sites and
//! conditional property refinements use the documentation on their declarations.
//! A `then`, `else`, or `dependentSchemas` branch can refine a property
//! declared at the same instance path by the enclosing schema, including across
//! a combinator or nested `properties`. A first declaration still needs its
//! own annotations. Tests such as `if`, `not`, and `contains` and subschemas
//! for other instance locations are checked independently.
//! Placeholder descriptions are rejected at every authored schema node.

use serde_json::{Map, Value};

use crate::diagnostic::LintDiagnostic;

/// Rule identifier.
pub const RULE_ID: &str = "SCHEMA-DOC-001";

/// Minimum description length for a baseline leaf property.
const BASELINE_DESCRIPTION_MIN: usize = 60;

/// Minimum description length for a critical leaf property.
const CRITICAL_DESCRIPTION_MIN: usize = 140;

/// Minimum `examples` count for a critical leaf property.
const CRITICAL_EXAMPLES_MIN: usize = 2;

/// Description markers that imply a closed vocabulary.
const CLOSED_VOCAB_PROSE_MARKERS: [&str; 6] = [
    "must be one of",
    "shall be one of",
    "allowed values",
    "canonical values",
    "valid values",
    "standard values:",
];

/// Allowed `x-wos.openStringKind` values for honest-open string leaves.
const OPEN_STRING_KIND_VALUES: [&str; 8] = [
    "prose",
    "fel",
    "uri",
    "identifier",
    "pathExpression",
    "hash",
    "timestamp",
    "tagLabel",
];

/// Walk a parsed JSON Schema document and collect `SCHEMA-DOC-001`
/// violations for every leaf property.
///
/// The caller is expected to have already parsed the schema file.
pub fn check_schema(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    for_each_subschema(root, &mut |obj, pointer, refines_property| {
        if obj
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|description| {
                description.contains(" value under referenceDueProcess/1")
                    && description.trim_start().starts_with("Retained ")
            })
        {
            diagnostics.push(LintDiagnostic::t1_error(
                RULE_ID,
                pointer.to_string(),
                "`description` contains a retained-value placeholder; describe the field's meaning or omit the redundant annotation",
            ));
        }
        if !refines_property && is_leaf(obj) {
            check_leaf(obj, pointer, diagnostics);
        }
    });
}

/// Walk a parsed JSON Schema document and collect `SCHEMA-OPEN-001`
/// violations for open string leaves.
///
/// Leaves with `enum`, `const`, or `pattern` are exempt. Open leaves must
/// carry `x-wos.openStringKind` with one of the allowed values.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use wos_lint::rules::schema_doc::check_open_string_kinds;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "label": {
///             "type": "string",
///             "x-wos": { "openStringKind": "tagLabel" }
///         }
///     }
/// });
/// let mut diagnostics = Vec::new();
/// check_open_string_kinds(&schema, &mut diagnostics);
/// assert!(diagnostics.is_empty());
/// ```
pub fn check_open_string_kinds(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    for_each_subschema(root, &mut |obj, pointer, refines_property| {
        if !refines_property && is_open_string_leaf(obj) {
            check_open_string_kind(obj, pointer, diagnostics);
        }
    });
}

/// Count the total number of leaf properties in the parsed schema.
///
/// Mirrors `check_schema`'s walk but only counts leaves; does not produce
/// diagnostics. This supports schema-doc inventory reporting where callers
/// need denominator data rather than per-property findings.
pub fn count_leaves(root: &Value) -> usize {
    let mut count = 0usize;
    for_each_subschema(root, &mut |obj, _, refines_property| {
        if !refines_property && is_leaf(obj) {
            count += 1;
        }
    });
    count
}

/// Counts string-shaped leaf nodes vs those with an explicit value constraint.
///
/// Uses the same leaf definition and refinement rule as [`check_schema`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StringLeafInventory {
    /// `type` includes `string` at a leaf node.
    pub string_leaves: usize,
    /// Subset of [`Self::string_leaves`] with `enum`, `const`, `pattern`, or a listed `x-wos.openStringKind`.
    pub constrained_string_leaves: usize,
}

impl StringLeafInventory {
    /// String leaves with no `enum`/`const`/`pattern` at the leaf node.
    #[must_use]
    pub fn open_string_leaves(self) -> usize {
        self.string_leaves
            .saturating_sub(self.constrained_string_leaves)
    }
}

/// Walk `root` and tally string vocabulary shape for schema hardening passes.
pub fn inventory_string_leaves(root: &Value) -> StringLeafInventory {
    let mut inv = StringLeafInventory::default();
    for_each_subschema(root, &mut |obj, _, refines_property| {
        if !refines_property && is_leaf(obj) && is_string_schema(obj) {
            inv.string_leaves += 1;
            if leaf_string_has_value_constraint(obj) {
                inv.constrained_string_leaves += 1;
            }
        }
    });
    inv
}

/// One open string leaf (no `enum` / `const` / `pattern` at the leaf) for triage exports.
#[derive(Debug, Clone)]
pub struct OpenStringLeafRow {
    /// JSON Pointer to the schema node.
    pub pointer: String,
    /// Name of the nearest `$defs` / `definitions` parent, or empty at document root.
    pub def_context: String,
    /// First line of `description`, truncated for CSV/terminal width.
    pub description_snippet: String,
    pub has_format: bool,
    pub has_min_length: bool,
    pub has_max_length: bool,
}

/// Collect every string leaf without `enum`/`const`/`pattern` or listed `openStringKind` at that node.
#[must_use]
pub fn collect_open_string_leaves(root: &Value) -> Vec<OpenStringLeafRow> {
    let mut rows = Vec::new();
    for_each_subschema(root, &mut |obj, pointer, refines_property| {
        if !refines_property && is_open_string_leaf(obj) {
            let description = obj.get("description").and_then(Value::as_str).unwrap_or("");
            rows.push(OpenStringLeafRow {
                pointer: pointer.to_string(),
                def_context: def_context_from_pointer(pointer),
                description_snippet: description_snippet(description, 160),
                has_format: obj.contains_key("format"),
                has_min_length: obj.contains_key("minLength"),
                has_max_length: obj.contains_key("maxLength"),
            });
        }
    });
    rows.sort_by(|a, b| a.pointer.cmp(&b.pointer));
    rows
}

fn description_snippet(text: &str, max_chars: usize) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let mut s: String = line.chars().take(max_chars).collect();
    if line.chars().count() > max_chars {
        s.push('…');
    }
    s.replace('"', "'")
}

fn def_context_from_pointer(pointer: &str) -> String {
    let segments: Vec<String> = pointer
        .split('/')
        .filter(|s| !s.is_empty())
        .map(unescape_json_pointer_token)
        .collect();
    for w in segments.windows(2) {
        if w[0] == "$defs" || w[0] == "definitions" {
            return w[1].clone();
        }
    }
    String::new()
}

fn unescape_json_pointer_token(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

fn check_open_string_kind(
    obj: &serde_json::Map<String, Value>,
    pointer: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(kind) = open_string_kind(obj) else {
        diagnostics.push(LintDiagnostic::t1_error(
            OPEN_STRING_KIND_RULE_ID,
            pointer.to_string(),
            format!(
                "open string leaf must declare `x-wos.openStringKind`; allowed values: {}",
                OPEN_STRING_KIND_VALUES.join(", ")
            ),
        ));
        return;
    };

    if !OPEN_STRING_KIND_VALUES.contains(&kind) {
        diagnostics.push(LintDiagnostic::t1_error(
            OPEN_STRING_KIND_RULE_ID,
            pointer.to_string(),
            format!(
                "invalid `x-wos.openStringKind` value {kind:?}; allowed values: {}",
                OPEN_STRING_KIND_VALUES.join(", ")
            ),
        ));
    }
}

fn open_string_kind(obj: &serde_json::Map<String, Value>) -> Option<&str> {
    obj.get("x-wos")
        .and_then(Value::as_object)
        .and_then(|extensions| extensions.get("openStringKind"))
        .and_then(Value::as_str)
}

/// Value constraints meaningful for a string leaf (`enum` / `const` / `pattern`),
/// plus an honest-open declaration audited by SCHEMA-OPEN-001.
///
/// [`has_explicit_value_constraint`] covers the same listed `x-wos.openStringKind`
/// case for SCHEMA-DOC-001 closed-vocabulary prose checks. That helper also treats
/// `oneOf` / `anyOf` at the leaf (this inventory helper does not — those shapes are
/// structural, not string leaves).
fn leaf_string_has_value_constraint(obj: &serde_json::Map<String, Value>) -> bool {
    if obj.contains_key("enum") || obj.contains_key("const") || obj.contains_key("pattern") {
        return true;
    }
    has_listed_open_string_kind(obj)
}

/// True when the leaf carries a listed `x-wos.openStringKind` (honest-open marker).
fn has_listed_open_string_kind(obj: &serde_json::Map<String, Value>) -> bool {
    open_string_kind(obj).is_some_and(|kind| OPEN_STRING_KIND_VALUES.contains(&kind))
}

/// Rule identifier for `SCHEMA-OPEN-001`.
const OPEN_STRING_KIND_RULE_ID: &str = "SCHEMA-OPEN-001";

/// Declarations that can apply to the instance at the current schema node.
#[derive(Default)]
struct Scope<'a> {
    declarations: &'a [&'a Map<String, Value>],
    conditional: bool,
    refines_property: bool,
}

/// Visit every authored subschema, including sibling keywords at `$ref` sites.
fn for_each_subschema(root: &Value, visit: &mut impl FnMut(&Map<String, Value>, &str, bool)) {
    walk_subschema(root, "", Scope::default(), visit);
}

fn walk_subschema(
    node: &Value,
    pointer: &str,
    scope: Scope<'_>,
    visit: &mut impl FnMut(&Map<String, Value>, &str, bool),
) {
    let Some(obj) = node.as_object() else {
        return;
    };
    visit(obj, pointer, scope.refines_property);

    let mut same_instance = scope.declarations.to_vec();
    same_instance.push(obj);
    for keyword in [
        "properties",
        "patternProperties",
        "$defs",
        "definitions",
        "dependentSchemas",
    ] {
        if let Some(children) = obj.get(keyword).and_then(Value::as_object) {
            for (name, child) in children {
                let child_pointer = format!("{pointer}/{keyword}/{}", escape_pointer(name));
                let property_declarations;
                let child_scope = match keyword {
                    "properties" => {
                        property_declarations = scope
                            .declarations
                            .iter()
                            .filter_map(|schema| schema.get("properties")?.get(name)?.as_object())
                            .collect::<Vec<_>>();
                        Scope {
                            declarations: &property_declarations,
                            conditional: scope.conditional,
                            refines_property: scope.conditional
                                && property_declarations
                                    .iter()
                                    .any(|owner| documents_property(owner)),
                        }
                    }
                    "dependentSchemas" => Scope {
                        declarations: &same_instance,
                        conditional: true,
                        ..Scope::default()
                    },
                    _ => Scope::default(),
                };
                walk_subschema(child, &child_pointer, child_scope, visit);
            }
        }
    }
    for keyword in [
        "items",
        "additionalProperties",
        "if",
        "then",
        "else",
        "not",
        "propertyNames",
        "contains",
    ] {
        match obj.get(keyword) {
            Some(child @ Value::Object(_)) => {
                let child_scope = match keyword {
                    "then" | "else" => Scope {
                        declarations: &same_instance,
                        conditional: true,
                        ..Scope::default()
                    },
                    _ => Scope::default(),
                };
                walk_subschema(child, &format!("{pointer}/{keyword}"), child_scope, visit);
            }
            // Drafts before 2020-12 also allowed tuple schemas under `items`.
            Some(Value::Array(items)) if keyword == "items" => {
                for (i, child) in items.iter().enumerate() {
                    walk_subschema(
                        child,
                        &format!("{pointer}/items/{i}"),
                        Scope::default(),
                        visit,
                    );
                }
            }
            _ => {}
        }
    }
    for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(Value::Array(members)) = obj.get(keyword) {
            for (i, child) in members.iter().enumerate() {
                let child_scope = if keyword == "prefixItems" {
                    Scope::default()
                } else {
                    Scope {
                        declarations: &same_instance,
                        conditional: scope.conditional,
                        ..Scope::default()
                    }
                };
                walk_subschema(
                    child,
                    &format!("{pointer}/{keyword}/{i}"),
                    child_scope,
                    visit,
                );
            }
        }
    }
}

/// A declaration can own the documentation for a later typed refinement.
fn documents_property(obj: &Map<String, Value>) -> bool {
    // An empty schema or a title alone can reserve a name without describing it.
    is_leaf(obj)
        || obj.contains_key("$ref")
        || (obj
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|description| description.chars().count() >= BASELINE_DESCRIPTION_MIN)
            && obj
                .get("examples")
                .and_then(Value::as_array)
                .is_some_and(|examples| !examples.is_empty()))
}

/// A string leaf with no `enum` / `const` / `pattern` and no listed
/// `x-wos.openStringKind`: the leaves SCHEMA-OPEN-001 audits.
fn is_open_string_leaf(obj: &serde_json::Map<String, Value>) -> bool {
    is_leaf(obj) && is_string_schema(obj) && !leaf_string_has_value_constraint(obj)
}

/// A node is a leaf if it declares a concrete `type` and has no composite
/// children that would make it a structural node.
fn is_leaf(obj: &serde_json::Map<String, Value>) -> bool {
    // In JSON Schema 2020-12, a reference can have sibling schema keywords.
    // Exclude the reference site from leaf checks while keeping traversal intact.
    if obj.contains_key("$ref") || !obj.contains_key("type") {
        return false;
    }
    // Composite containers disqualify leaf status.
    for key in [
        "properties",
        "patternProperties",
        "items",
        "prefixItems",
        "contains",
        "propertyNames",
        "dependentSchemas",
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

    // Listed `x-wos.openStringKind` counts as an explicit vocabulary declaration for
    // this guard (aligned with [`leaf_string_has_value_constraint`] / open-string ratchet).
    if is_string_schema(obj)
        && has_closed_vocab_prose(description)
        && !has_explicit_value_constraint(obj)
    {
        diagnostics.push(LintDiagnostic::t1_error(
            RULE_ID,
            pointer.to_string(),
            "`description` implies a closed vocabulary; add enum/const/oneOf/anyOf/pattern or a listed x-wos.openStringKind",
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

/// Return true when the schema's `type` includes `string`.
fn is_string_schema(obj: &serde_json::Map<String, Value>) -> bool {
    match obj.get("type") {
        Some(Value::String(type_name)) => type_name == "string",
        Some(Value::Array(types)) => types.iter().any(|value| value.as_str() == Some("string")),
        _ => false,
    }
}

/// Return true when the description reads like a closed vocabulary.
fn has_closed_vocab_prose(description: &str) -> bool {
    let description = description.to_ascii_lowercase();
    CLOSED_VOCAB_PROSE_MARKERS
        .iter()
        .any(|marker| description.contains(marker))
}

/// Return true when the schema has an explicit value constraint keyword, or a listed
/// `x-wos.openStringKind` (same rule as [`leaf_string_has_value_constraint`] for
/// honest-open leaves).
///
/// Broader than [`leaf_string_has_value_constraint`] at the leaf node: includes
/// `oneOf` / `anyOf` (inventory excludes those because such nodes are not leaves).
fn has_explicit_value_constraint(obj: &serde_json::Map<String, Value>) -> bool {
    obj.contains_key("enum")
        || obj.contains_key("const")
        || obj.contains_key("oneOf")
        || obj.contains_key("anyOf")
        || obj.contains_key("pattern")
        || has_listed_open_string_kind(obj)
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
    fn string_leaf_closed_vocab_prose_without_constraints_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Allowed values: 'mail' or 'phone'. This description is long enough to clear the baseline threshold.",
                    "examples": ["mail"]
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, RULE_ID);
        assert!(diagnostics[0].message.contains("closed vocabulary"));
    }

    #[test]
    fn string_leaf_shall_be_one_of_prose_without_constraints_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "description": "The processor shall be one of 'fast' or 'slow' for this benchmark. Padding padding padding padding padding padding padding padding padding padding.",
                    "examples": ["fast"]
                }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("closed vocabulary"));
    }

    #[test]
    fn string_leaf_closed_vocab_prose_with_pattern_is_clean() {
        let schema = json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "pattern": "^(mail|phone)$",
                    "description": "Valid values: 'mail' or 'phone'. This description is long enough to clear the baseline threshold.",
                    "examples": ["mail"]
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
    fn string_leaf_inventory_open_vs_constrained() {
        let schema = json!({
            "type": "object",
            "properties": {
                "free": { "type": "string" },
                "closed": { "type": "string", "enum": ["a"] }
            }
        });
        let inv = inventory_string_leaves(&schema);
        assert_eq!(inv.string_leaves, 2);
        assert_eq!(inv.constrained_string_leaves, 1);
        assert_eq!(inv.open_string_leaves(), 1);
    }

    #[test]
    fn open_string_kind_is_inventory_constraint_when_value_is_allowed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "note": {
                    "type": "string",
                    "description": description_60(),
                    "examples": ["hello"],
                    "x-wos": { "openStringKind": "prose" }
                }
            }
        });
        let inv = inventory_string_leaves(&schema);
        assert_eq!(inv.string_leaves, 1);
        assert_eq!(inv.constrained_string_leaves, 1);
        assert_eq!(inv.open_string_leaves(), 0);
    }

    #[test]
    fn open_string_kind_unknown_value_does_not_close_inventory_leaf() {
        let schema = json!({
            "type": "object",
            "properties": {
                "note": {
                    "type": "string",
                    "description": description_60(),
                    "examples": ["hello"],
                    "x-wos": { "openStringKind": "not-a-listed-kind" }
                }
            }
        });
        let inv = inventory_string_leaves(&schema);
        assert_eq!(inv.open_string_leaves(), 1);
    }

    #[test]
    fn listed_open_string_kind_satisfies_closed_vocab_prose_guard() {
        let prefix = "a".repeat(40);
        let schema = json!({
            "type": "object",
            "properties": {
                "note": {
                    "type": "string",
                    "description": format!("{prefix} must be one of alpha beta gamma for prose guard test."),
                    "examples": ["alpha"],
                    "x-wos": { "openStringKind": "prose" }
                }
            }
        });
        let diagnostics = lint(schema);
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.message.contains("closed vocabulary")),
            "unexpected SCHEMA-DOC closed-vocab diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn collect_open_string_leaves_lists_only_open() {
        let schema = json!({
            "$defs": {
                "Box": {
                    "type": "object",
                    "properties": {
                        "open": { "type": "string", "description": "x".repeat(60) },
                        "pat": { "type": "string", "pattern": "^a$", "description": "y".repeat(60) }
                    }
                }
            }
        });
        let rows = collect_open_string_leaves(&schema);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].pointer.ends_with("/properties/open"));
        assert_eq!(rows[0].def_context, "Box");
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
    fn root_ref_preserves_sibling_definitions() {
        let schema = json!({
            "$ref": "#/$defs/Name",
            "type": "string",
            "$defs": {
                "Name": { "type": "string", "examples": ["sample"] }
            },
            "definitions": {
                "Level": {
                    "type": "integer",
                    "description": description_60(),
                    "examples": [1]
                }
            }
        });

        let mut diagnostics = Vec::new();
        check_schema(&schema, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "expected the missing description");
        assert_eq!(diagnostics[0].path, "/$defs/Name");

        diagnostics.clear();
        check_open_string_kinds(&schema, &mut diagnostics);
        assert_eq!(
            diagnostics.len(),
            1,
            "expected the missing open string kind"
        );
        assert_eq!(diagnostics[0].path, "/$defs/Name");

        assert_eq!(count_leaves(&schema), 2);
        assert_eq!(inventory_string_leaves(&schema).open_string_leaves(), 1);
        let rows = collect_open_string_leaves(&schema);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pointer, "/$defs/Name");
    }

    #[test]
    fn nested_ref_site_children_are_walked() {
        // 2020-12 applies keywords beside a `$ref`; the site is not a leaf,
        // but its own `properties` still are.
        let schema = json!({
            "type": "object",
            "properties": {
                "state": {
                    "$ref": "#/$defs/Base",
                    "type": "object",
                    "properties": { "note": { "type": "string", "examples": ["x"] } }
                }
            },
            "$defs": {
                "Base": { "type": "object", "description": description_60(), "examples": [{}] }
            }
        });
        let diagnostics = lint(schema);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].path, "/properties/state/properties/note");
    }

    fn assert_walk_reaches(schema: Value, pointer: &str) {
        let diagnostics = lint(schema.clone());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].path, pointer);

        let mut diagnostics = Vec::new();
        check_open_string_kinds(&schema, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].path, pointer);
        assert_eq!(count_leaves(&schema), 1);
        assert_eq!(inventory_string_leaves(&schema).open_string_leaves(), 1);
        assert_eq!(collect_open_string_leaves(&schema)[0].pointer, pointer);
    }

    macro_rules! schema_keyword_test {
        ($name:ident, $pointer:literal, |$leaf:ident| $schema:expr) => {
            #[test]
            fn $name() {
                let $leaf = json!({"type": "string", "examples": ["value"]});
                assert_walk_reaches($schema, $pointer);
            }
        };
    }

    schema_keyword_test!(walk_visits_if, "/if", |leaf| json!({"if": leaf}));
    schema_keyword_test!(
        walk_visits_then,
        "/then",
        |leaf| json!({"if": true, "then": leaf})
    );
    schema_keyword_test!(
        walk_visits_else,
        "/else",
        |leaf| json!({"if": false, "else": leaf})
    );
    schema_keyword_test!(walk_visits_not, "/not", |leaf| json!({"not": leaf}));
    schema_keyword_test!(
        walk_visits_property_names,
        "/propertyNames",
        |leaf| json!({"propertyNames": leaf})
    );
    schema_keyword_test!(
        walk_visits_contains,
        "/contains",
        |leaf| json!({"contains": leaf})
    );
    schema_keyword_test!(
        walk_visits_prefix_items,
        "/prefixItems/1",
        |leaf| json!({"prefixItems": [true, leaf]})
    );
    schema_keyword_test!(
        walk_visits_dependent_schemas,
        "/dependentSchemas/a~1b",
        |leaf| json!({"dependentSchemas": {"a/b": leaf}})
    );

    #[test]
    fn conditional_property_refinements_use_the_declaration_at_the_same_instance_path() {
        let schema = json!({
            "properties": {
                "data": { "properties": {
                    "known": {
                        "type": "string", "description": description_60(),
                        "examples": ["baseline"], "x-wos": { "openStringKind": "identifier" }
                    }
                }}
            },
            "allOf": [{
                "if": { "required": ["test"] },
                "then": { "properties": { "data": { "properties": {
                    "known": { "type": "string", "const": "specific" },
                    "new": { "type": "string" }
                }}}},
                "else": { "properties": { "other": { "properties": {
                    "known": { "type": "string" }
                }}}}
            }]
        });
        let diagnostics = lint(schema.clone());
        assert_eq!(diagnostics.len(), 4, "{diagnostics:?}");
        assert_eq!(
            diagnostics[0].path,
            "/allOf/0/else/properties/other/properties/known"
        );
        assert_eq!(
            diagnostics[2].path,
            "/allOf/0/then/properties/data/properties/new"
        );
        assert_eq!(count_leaves(&schema), 3);
    }

    #[test]
    fn untyped_property_sites_do_not_hide_first_typed_declarations() {
        for base in [json!({}), json!({"title": "Status"}), json!(true)] {
            assert_walk_reaches(
                json!({
                    "properties": {"status": base},
                    "if": true,
                    "then": {"properties": {
                        "status": {"type": "string", "examples": ["active"]}
                    }}
                }),
                "/then/properties/status",
            );
        }
    }

    #[test]
    fn placeholder_descriptions_are_rejected_on_reference_sites_and_composites() {
        let diagnostics = lint(json!({
            "properties": {
                "link": {
                    "$ref": "#/$defs/Target",
                    "description": "Retained link value under referenceDueProcess/1."
                },
                "entries": {
                    "type": "array", "items": { "$ref": "#/$defs/Target" },
                    "description": "Retained entries value under referenceDueProcess/1."
                }
            },
            "$defs": { "Target": { "type": "object", "properties": {} } }
        }));
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert_eq!(diagnostics[0].path, "/properties/entries");
        assert_eq!(diagnostics[1].path, "/properties/link");
        assert!(
            diagnostics
                .iter()
                .all(|d| d.message.contains("placeholder"))
        );
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
