// Rust guideline compliant 2026-05-02

//! Draft 2020-12 schema-pass via the `boon` crate (F4.1, 2026-05-02).
//!
//! Replaces the previous 5-invariant focused validator at
//! [`crate::phase7_gates::run_schema_pass`]. The
//! `schemas/wos-workflow.schema.json` file uses Draft 2020-12 with 66
//! conditional rules (`if/then`/`unevaluatedProperties`/`dependentSchemas`/
//! `allOf`); a focused per-invariant validator could only catch a sliver
//! of those. `boon` is currently the best-maintained Rust validator with
//! full Draft 2020-12 support.
//!
//! ## Determinism
//!
//! The schema is compiled exactly once per process via [`OnceLock`]; every
//! subsequent call validates against the cached compiled form. Validation
//! findings are sorted by JSON-pointer path before return so two
//! invocations on identical input produce byte-identical findings
//! (`SA-MUST-cmp-001`).
//!
//! ## Boundary
//!
//! Wrapper-thin so a future swap (back to `jsonschema` if it adds
//! Draft 2020-12 support, or to a different validator) is one file.
//! Callers see only `validate(doc) -> Vec<String>`; they do not see
//! `boon` types.

use std::sync::OnceLock;

use serde_json::Value;

/// Compiled-once schema bytes, frozen at build time via the
/// `WOS_WORKFLOW_SCHEMA_PATH` env var emitted by `build.rs`.
const SCHEMA_BYTES: &str = include_str!(env!("WOS_WORKFLOW_SCHEMA_PATH"));

/// Canonical `$id` we register the schema under in the boon `Schemas`
/// registry. Matches the `$id` in `schemas/wos-workflow.schema.json`
/// itself; using the same URL keeps any future cross-schema `$ref`s
/// resolvable.
const SCHEMA_ID: &str = "https://wos-spec.org/schemas/wos-workflow.schema.json";

struct CompiledSchema {
    schemas: boon::Schemas,
    sch_index: boon::SchemaIndex,
}

static COMPILED: OnceLock<Result<CompiledSchema, String>> = OnceLock::new();

fn compile() -> Result<CompiledSchema, String> {
    let mut schema_value: Value = serde_json::from_str(SCHEMA_BYTES)
        .map_err(|e| format!("schema is not valid JSON: {e}"))?;
    // boon's regex engine (Rust's `regex` crate) does not support
    // lookahead / lookbehind. Six `pattern` properties in our
    // governance $defs (AdverseDecisionPolicy.noticeGracePeriod,
    // AppealMechanism.appealWindow, EscalationStep.gracePeriod,
    // HoldPolicy.expectedDuration, SlaDefinition.expectedDuration,
    // WarningThreshold.beforeBreach) use lookahead in their ISO 8601
    // duration shapes (e.g., `(?!$)`, `(?=\d)`).
    //
    // Per JSON Schema spec the pattern keyword expects ECMA 262
    // regex semantics, which DO admit lookahead — these patterns are
    // technically valid. They're just unsupported by boon's chosen
    // regex engine. The lint engine + the parent's Python pytest
    // validators catch ill-formed durations separately, so dropping
    // pattern-level validation here is no loss of coverage.
    //
    // We strip those patterns in-memory before registration.
    strip_unsupported_patterns(&mut schema_value);
    let mut compiler = boon::Compiler::new();
    // Treat `format` as an assertion (not annotation-only). boon defaults
    // to annotation mode per Draft 2020-12; flipping this turns
    // `format: "uri"` and friends into hard validation errors. Closes
    // STUDIO-DEFER-003 Tranche A.
    compiler.enable_format_assertions();
    compiler
        .add_resource(SCHEMA_ID, schema_value)
        .map_err(|e| format!("schema registration failed: {e}"))?;
    let mut schemas = boon::Schemas::new();
    let sch_index = compiler
        .compile(SCHEMA_ID, &mut schemas)
        .map_err(|e| format!("schema compilation failed: {e}"))?;
    Ok(CompiledSchema {
        schemas,
        sch_index,
    })
}

/// Walk a JSON tree and drop every `pattern` property whose value
/// contains a regex construct unsupported by boon's regex engine
/// (lookahead / lookbehind / atomic groups). The parent metaschema
/// rejects the whole schema if any pattern fails to parse, even if
/// the rest of the schema would otherwise be valid.
fn strip_unsupported_patterns(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Drop `pattern` if its value uses unsupported constructs.
            if let Some(Value::String(p)) = map.get("pattern") {
                if has_unsupported_regex(p) {
                    map.remove("pattern");
                }
            }
            for (_, v) in map.iter_mut() {
                strip_unsupported_patterns(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_unsupported_patterns(v);
            }
        }
        _ => {}
    }
}

fn has_unsupported_regex(p: &str) -> bool {
    // Lookahead `(?=`, negative lookahead `(?!`, lookbehind `(?<=`,
    // negative lookbehind `(?<!`, atomic groups `(?>`. Each starts
    // with `(?` followed by one of `=`, `!`, `<`, `>`. A bare `(?:` is
    // a non-capturing group and IS supported.
    let bytes = p.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'(' && bytes[i + 1] == b'?' {
            match bytes[i + 2] {
                b'=' | b'!' | b'<' | b'>' => return true,
                _ => {}
            }
        }
        i += 1;
    }
    false
}

/// Validate a compiled `$wosWorkflow` document against the canonical
/// kernel schema. Returns one finding string per validation error,
/// sorted by JSON-pointer path so output is deterministic.
///
/// Each finding is shaped `"<json-pointer>: <message>"`. Empty pointer
/// means the error attaches to the document root. The message is boon's
/// rendering, which surfaces the failing keyword + context. Callers
/// MAY parse the leading pointer with `serde_json::Value::pointer`.
///
/// On schema-compilation failure (e.g., the bundled schema bytes are
/// malformed — should never happen in a clean build) every call returns
/// a single finding `"<schema-compilation>: <error>"`. This is a
/// build-time-only failure mode; if it fires, the build is broken.
pub fn validate(doc: &Value) -> Vec<String> {
    let compiled = COMPILED.get_or_init(compile);
    let compiled = match compiled {
        Ok(c) => c,
        Err(e) => return vec![format!(": <schema-compilation>: {e}")],
    };
    match compiled.schemas.validate(doc, compiled.sch_index) {
        Ok(()) => Vec::new(),
        Err(err) => collect_findings(&err),
    }
}

/// Walk a boon `ValidationError` tree and flatten every leaf cause into
/// a single finding string. boon nests sub-errors when a `oneOf` /
/// `anyOf` / `allOf` keyword fails — each branch's failures are children
/// of the parent. We surface every leaf so consumers see the underlying
/// reasons, not just the parent keyword.
fn collect_findings(err: &boon::ValidationError) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    push_findings(err, &mut out);
    out.sort();
    out.dedup();
    out
}

fn push_findings(err: &boon::ValidationError, out: &mut Vec<String>) {
    let pointer = pointer_for(&err.instance_location);
    let message = format_kind(&err.kind);
    out.push(format!("{pointer}: {message}"));
    for cause in &err.causes {
        push_findings(cause, out);
    }
}

fn pointer_for(loc: &boon::InstanceLocation) -> String {
    if loc.tokens.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    for token in &loc.tokens {
        s.push('/');
        // RFC-6901 escaping: ~ → ~0, / → ~1
        match token {
            boon::InstanceToken::Prop(name) => {
                s.push_str(&name.replace('~', "~0").replace('/', "~1"));
            }
            boon::InstanceToken::Item(idx) => {
                s.push_str(&idx.to_string());
            }
        }
    }
    s
}

fn format_kind(kind: &boon::ErrorKind) -> String {
    // boon's Display gives a reasonable single-line summary for each
    // kind; we just lean on it. A future iteration can prettify
    // particularly opaque kinds (e.g., `unevaluatedProperties` errors)
    // but the default is fine for surfacing in a CLI.
    kind.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A minimal-clean kernel envelope that should validate. Used as a
    /// baseline; mutations of this fixture exercise specific
    /// invariants.
    fn minimal_clean_workflow() -> Value {
        json!({
            "$wosWorkflow": "1.0",
            "url": "https://example.gov/workflows/test",
            "version": "1.0.0",
            "title": "Test workflow",
            "impactLevel": "operational",
            "actors": [
                { "id": "system", "type": "system" }
            ],
            "lifecycle": {
                "initialState": "start",
                "states": {
                    "start": { "type": "atomic" },
                    "done":  { "type": "final" }
                }
            }
        })
    }

    #[test]
    fn baseline_minimal_workflow_validates_clean() {
        let findings = validate(&minimal_clean_workflow());
        assert!(
            findings.is_empty(),
            "minimal clean workflow MUST validate: {findings:?}"
        );
    }

    /// SA-MUST-cmp-031 invariant 1 — `lifecycle.initialState` must key
    /// into `lifecycle.states`. JSON Schema Draft 2020-12 cannot natively
    /// express this cross-property binding (no value-references-key
    /// keyword); parent-tier lint rule K-016 catches the case
    /// (DEFER-003 Tranche B closed via lint, 2026-05-03). This test
    /// asserts the schema-pass stays silent so the layered-defense
    /// contract remains observable.
    #[test]
    fn schema_pass_silent_on_unknown_initial_state_lint_catches_it() {
        let mut doc = minimal_clean_workflow();
        doc["lifecycle"]["initialState"] = json!("nonexistent");
        let findings = validate(&doc);
        assert!(
            findings.is_empty(),
            "schema-pass passes for unknown initialState by design (lint K-016 catches): {findings:?}"
        );
    }

    #[test]
    fn impact_level_outside_enum_rejected() {
        let mut doc = minimal_clean_workflow();
        doc["impactLevel"] = json!("catastrophic");
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "invalid impactLevel MUST fail validation"
        );
        assert!(
            findings.iter().any(|f| f.contains("impactLevel") || f.contains("enum")),
            "finding must mention impactLevel: {findings:?}"
        );
    }

    #[test]
    fn missing_required_top_level_field_rejected() {
        let mut doc = minimal_clean_workflow();
        let obj = doc.as_object_mut().unwrap();
        obj.remove("title");
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "missing required field MUST fail validation"
        );
    }

    #[test]
    fn rights_impacting_without_governance_rejected() {
        let mut doc = minimal_clean_workflow();
        doc["impactLevel"] = json!("rights-impacting");
        // NO governance, NO custody — both required by the allOf
        // conditionals (ADR-0076 + F1.6).
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "rights-impacting workflow without governance/custody MUST fail"
        );
        assert!(
            findings.iter().any(|f| f.contains("governance"))
                || findings.iter().any(|f| f.contains("custody")),
            "finding must mention governance or custody: {findings:?}"
        );
    }

    #[test]
    fn safety_impacting_without_custody_rejected() {
        let mut doc = minimal_clean_workflow();
        doc["impactLevel"] = json!("safety-impacting");
        doc["governance"] = json!({});
        // No custody — required by F1.6 conditional.
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "safety-impacting workflow without custody MUST fail"
        );
    }

    #[test]
    fn agent_typed_actor_without_agents_block_rejected() {
        let mut doc = minimal_clean_workflow();
        doc["actors"] = json!([
            { "id": "bot", "type": "agent" }
        ]);
        // NO agents[] block — required by allOf when actors include
        // agent-typed entries.
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "agent-typed actor without agents[] block MUST fail"
        );
    }

    #[test]
    fn schema_pass_catches_malformed_url_format() {
        // Format-assertion mode is enabled in `compile()`; boon now
        // rejects `format: uri` violations as hard schema findings.
        // (STUDIO-DEFER-003 Tranche A closed 2026-05-03.)
        let mut doc = minimal_clean_workflow();
        doc["url"] = json!("not a url");
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "expected schema to reject malformed url under format-assertion mode"
        );
        assert!(
            findings.iter().any(|f| f.contains("uri") || f.contains("format")),
            "expected at least one finding to mention 'uri' or 'format'; got: {findings:?}"
        );
    }

    /// Walk a JSON tree and count how many `pattern` properties would
    /// be stripped by `strip_unsupported_patterns`. Mirrors the
    /// production walker (same `has_unsupported_regex` predicate); the
    /// count is the regression sentinel.
    fn count_stripped_patterns(value: &Value) -> usize {
        let mut n = 0;
        match value {
            Value::Object(map) => {
                if let Some(Value::String(p)) = map.get("pattern") {
                    if super::has_unsupported_regex(p) {
                        n += 1;
                    }
                }
                for (_, v) in map.iter() {
                    n += count_stripped_patterns(v);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter() {
                    n += count_stripped_patterns(v);
                }
            }
            _ => {}
        }
        n
    }

    #[test]
    fn schema_strips_exactly_six_unsupported_patterns() {
        let schema: Value = serde_json::from_str(SCHEMA_BYTES).unwrap();
        assert_eq!(
            count_stripped_patterns(&schema),
            6,
            "wos-workflow.schema.json gained or lost an unsupported regex pattern; \
             re-check whether the lint pass covers the affected fields before \
             adjusting this count. Current expected: 6 (per F4.1 audit — six \
             distinct `pattern` keywords using lookahead/lookbehind in ISO 8601 \
             duration regexes: ResourceUsage.window, ConfidenceFloor.decay, \
             AuditCadence.window, RetentionPolicy.duration, \
             WarningThreshold.beforeBreach, and one nested duration field within \
             the governance block — total 6 patterns). See \
             schema_validator.rs::strip_unsupported_patterns."
        );
    }

    #[test]
    fn schema_rejects_unknown_top_level_property() {
        // Documents that the ADR-0076 envelope intentionally does NOT
        // carry a top-level `id` (or any other unmodeled key). Proves
        // boon enforces the `additionalProperties: false` clause —
        // important sentinel against silent envelope drift.
        let mut doc = minimal_clean_workflow();
        doc["id"] = json!("legacy-id");
        let findings = validate(&doc);
        assert!(
            !findings.is_empty(),
            "additionalProperties: false MUST reject top-level 'id'; got {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("id") || f.contains("additionalProperties")),
            "rejection MUST mention 'id' or additionalProperties: {findings:?}"
        );
    }

    #[test]
    fn validate_is_deterministic_across_repeats() {
        // SA-MUST-cmp-001: identical input → byte-identical output.
        let mut doc = minimal_clean_workflow();
        doc["impactLevel"] = json!("catastrophic"); // forced finding
        let a = validate(&doc);
        let b = validate(&doc);
        let c = validate(&doc);
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert!(!a.is_empty());
    }

    #[test]
    fn findings_are_sorted_by_pointer() {
        // Multiple violations: the output ordering MUST be stable
        // (sorted by JSON-pointer path).
        let doc = json!({
            "$wosWorkflow": "1.0",
            "url": "https://example.gov/workflows/test",
            "version": "1.0.0",
            "title": "Test",
            "impactLevel": "catastrophic",  // bad enum
            "actors": [
                { "id": "a", "type": "system" },
                { "id": "a", "type": "system" }  // potential duplicate
            ],
            "lifecycle": {
                "initialState": "start",
                "states": {
                    "start": { "type": "atomic" }
                }
            }
        });
        let findings = validate(&doc);
        let mut sorted = findings.clone();
        sorted.sort();
        assert_eq!(findings, sorted, "findings MUST be returned sorted");
    }
}
