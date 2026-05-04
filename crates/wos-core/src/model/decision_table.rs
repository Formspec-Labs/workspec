// Rust guideline compliant 2026-02-21

//! Typed model for `decisionTable` constructs per Kernel §4.5.1.
//!
//! The `decisionTable` first-class kernel construct landed 2026-05-01:
//! - Schema definitions live in `schemas/wos-workflow.schema.json` under
//!   `$defs/{DecisionTable, DecisionTableRow, DecisionTableGuard}`.
//! - Spec normative content lives in `specs/kernel/spec.md` §4.5.1.
//!
//! This module exports the matching Rust types so wos-runtime can evaluate
//! decision-table guards, wos-lint can validate their structural integrity
//! (rules K-051/K-052/K-053), and wos-conformance can drive both via the
//! K-05X conformance fixtures.
//!
//! ## The polymorphic `Guard` shape
//!
//! Per Kernel §4.5.1.1, a transition's `guard` is `oneOf [FEL string,
//! DecisionTableGuard]`. The Rust [`Guard`] enum models this with
//! `#[serde(untagged)]` so `serde_json` deserializes either form
//! transparently. Callers that only care about the FEL-string variant
//! (most existing wos-runtime / wos-lint paths) use [`Guard::as_fel_str`]
//! which returns `Some(s)` for FEL guards and `None` for decision-table
//! guards.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A first-class decision table per Kernel §4.5.1.
///
/// Inputs and outputs are typed; rows carry FEL boolean predicates per
/// input cell and FEL value expressions per output cell. Document order is
/// significant for `first` and `priority` hit policies. Cell expressions
/// evaluate in a row-only scope (the declared inputs, bound to the values
/// the referencing [`DecisionTableGuard`]'s `input_bindings` produced) —
/// they MUST NOT reference `caseFile`, `$event`, or any outer namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTable {
    /// Unique identifier within the document.
    pub id: String,

    /// Human-readable description (free-form; not interpreted by the
    /// processor).
    #[serde(default)]
    pub description: Option<String>,

    /// Typed input parameters of the table. Each input is bound by the
    /// referencing [`DecisionTableGuard::input_bindings`] and exposed inside
    /// row cell expressions under its declared `name`.
    pub inputs: Vec<DecisionTableInput>,

    /// Typed output columns. A [`DecisionTableGuard`] selects exactly one
    /// column by name (`output_column`); for transition-guard tables that
    /// column MUST be `boolean`-typed.
    pub outputs: Vec<DecisionTableOutput>,

    /// Decision rows in document order. Each row carries one FEL boolean
    /// predicate per declared input and one FEL value expression per
    /// declared output.
    pub rows: Vec<DecisionTableRow>,

    /// Row-selection rule when zero, one, or many rows match.
    pub hit_policy: HitPolicy,
}

/// A typed input column declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTableInput {
    /// Input identifier; in-scope inside row cell FEL expressions.
    pub name: String,

    /// FEL type of the input value.
    #[serde(rename = "type")]
    pub kind: FelType,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A typed output column declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTableOutput {
    /// Output column identifier; referenced by
    /// [`DecisionTableGuard::output_column`].
    pub name: String,

    /// FEL type of the produced output value. Output cell expressions
    /// MUST evaluate to this type.
    #[serde(rename = "type")]
    pub kind: FelType,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// One row of a [`DecisionTable`]. Matches when every entry of `input_cells`
/// evaluates to `true` against the row scope (the table's declared inputs
/// bound to the referencing guard's `input_bindings`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTableRow {
    /// Stable row identifier (unique within the table). Preserved through
    /// projection so provenance, scenario coverage, and reviewer comments
    /// remain row-anchored across edits.
    pub id: String,

    /// FEL boolean predicates, one per declared input in declaration order.
    /// Each evaluates in row scope (inputs bound to guard `input_bindings`-
    /// resolved values). MUST evaluate to a boolean. An empty array means
    /// 'matches always' — the conventional default-row pattern.
    pub input_cells: Vec<String>,

    /// FEL value expressions, one per declared output in declaration order.
    /// Each evaluates in row scope (same bindings as input cells) and
    /// produces a value of the type declared in the corresponding
    /// `outputs[*].type`.
    pub output_cells: Vec<String>,

    /// Used only when the enclosing [`DecisionTable`]'s `hit_policy =
    /// Priority`. Lower number = higher priority. Ties among matched rows
    /// are a K-052 violation.
    #[serde(default)]
    pub priority: Option<i64>,

    /// Reviewer-facing rationale for this row, preserved as authoring
    /// provenance. Not interpreted by the processor.
    #[serde(default)]
    pub rationale: Option<String>,
}

/// Hit policy controlling row-selection semantics per Kernel §4.5.1.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HitPolicy {
    /// At most one row may match. Multiple matches → K-052 (overlap).
    /// Zero matches → `on_no_match` behavior.
    Unique,

    /// First matching row in document order wins.
    First,

    /// Among matching rows, the one with the lowest `priority` integer
    /// wins. Ties among overlapping rows → K-052 (priority tie).
    Priority,

    /// Returns all matching rows. The composing layer (caller) MUST
    /// aggregate. **Disallowed for transition-guard usage** (K-053).
    Collect,
}

/// FEL value types accepted on declared inputs/outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FelType {
    String,
    Number,
    Integer,
    Boolean,
    Date,
    Datetime,
    Duration,
}

/// Structured guard form referencing a top-level `decisionTables[*]` entry.
///
/// Composes with [`crate::model::kernel::Transition::guard`] (Kernel §4.5.1).
/// Evaluates by binding the table's declared inputs to the values produced
/// by `input_bindings[*]` (FEL expressions in the full transition context),
/// running the table per its hit policy, and returning the selected row's
/// `output_column` value (which MUST be boolean for transition-guard usage).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTableGuard {
    /// Discriminant. Always [`DecisionTableGuardKind::DecisionTable`].
    pub kind: DecisionTableGuardKind,

    /// Identifier of an entry in the document's top-level
    /// `decision_tables[]`. MUST resolve (K-051).
    #[serde(rename = "ref")]
    pub table_ref: String,

    /// Name of the output column to read for the guard's boolean result.
    /// MUST exist on the referenced table; for transition-guard usage
    /// MUST be `Boolean`-typed (K-053).
    pub output_column: String,

    /// Map from each table input name to a FEL expression evaluated in the
    /// full transition context. The result is bound into the row scope
    /// under the input's name. Every declared input of the referenced
    /// table MUST have a binding (K-051).
    pub input_bindings: IndexMap<String, String>,

    /// Behavior when zero rows match. Defaults to
    /// [`OnNoMatch::False`].
    #[serde(default)]
    pub on_no_match: Option<OnNoMatch>,
}

/// Discriminant for [`DecisionTableGuard`]. Always serializes as
/// `"decisionTable"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionTableGuardKind {
    /// The literal `"decisionTable"` discriminator.
    #[serde(rename = "decisionTable")]
    DecisionTable,
}

/// Behavior when zero rows match (and after `unique`/`first`/`priority`
/// short-circuits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnNoMatch {
    /// Treat the guard as `false` — the transition is skipped per
    /// Kernel §4.6.
    False,
    /// Reject per Kernel §4.6/§4.7 transition resolution. Use when the
    /// table is intended to be exhaustive and a no-match represents an
    /// authoring error worth halting.
    Fail,
}

/// The polymorphic transition-guard shape per Kernel §4.5.1.1.
///
/// `oneOf [FEL string, DecisionTableGuard]`. Deserializes via
/// `#[serde(untagged)]` so `serde_json` accepts either a JSON string
/// (FEL) or a JSON object (DecisionTableGuard) in the same field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Guard {
    /// FEL expression evaluated against the transition's full evaluation
    /// context per Kernel §4.6/§4.7.
    Fel(String),

    /// Structured decision-table reference per Kernel §4.5.1.
    DecisionTable(DecisionTableGuard),
}

impl Guard {
    /// Returns the FEL-string form when this guard is a [`Guard::Fel`]
    /// variant; `None` for [`Guard::DecisionTable`].
    ///
    /// Most existing wos-runtime / wos-lint paths walk only FEL guards;
    /// this helper preserves the legacy `Option<&str>` shape those call
    /// sites previously got from `transition.guard.as_deref()`.
    #[must_use]
    pub fn as_fel_str(&self) -> Option<&str> {
        match self {
            Guard::Fel(s) => Some(s.as_str()),
            Guard::DecisionTable(_) => None,
        }
    }

    /// Returns the [`DecisionTableGuard`] form when this guard is a
    /// [`Guard::DecisionTable`] variant; `None` for [`Guard::Fel`].
    #[must_use]
    pub fn as_decision_table(&self) -> Option<&DecisionTableGuard> {
        match self {
            Guard::Fel(_) => None,
            Guard::DecisionTable(g) => Some(g),
        }
    }

    /// Returns true if this guard is a structured decision-table reference.
    #[must_use]
    pub fn is_decision_table(&self) -> bool {
        matches!(self, Guard::DecisionTable(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_fel_string_guard() {
        let json = r#""caseFile.income > 1000""#;
        let g: Guard = serde_json::from_str(json).expect("parse FEL guard");
        assert_eq!(g.as_fel_str(), Some("caseFile.income > 1000"));
        assert!(!g.is_decision_table());
    }

    #[test]
    fn deserializes_decision_table_guard() {
        let json = r#"{
            "kind": "decisionTable",
            "ref": "eligibilityTable",
            "outputColumn": "eligible",
            "inputBindings": {
                "income": "caseFile.income",
                "householdSize": "caseFile.size"
            }
        }"#;
        let g: Guard = serde_json::from_str(json).expect("parse decision-table guard");
        let dt = g.as_decision_table().expect("variant is DecisionTable");
        assert_eq!(dt.table_ref, "eligibilityTable");
        assert_eq!(dt.output_column, "eligible");
        assert_eq!(dt.input_bindings.len(), 2);
        assert!(g.as_fel_str().is_none());
    }

    #[test]
    fn deserializes_decision_table_with_on_no_match() {
        let json = r#"{
            "kind": "decisionTable",
            "ref": "t",
            "outputColumn": "ok",
            "inputBindings": {},
            "onNoMatch": "fail"
        }"#;
        let g: Guard = serde_json::from_str(json).unwrap();
        let dt = g.as_decision_table().unwrap();
        assert_eq!(dt.on_no_match, Some(OnNoMatch::Fail));
    }

    #[test]
    fn round_trips_decision_table() {
        let json = r#"{
            "id": "elig",
            "inputs": [{"name": "income", "type": "number"}],
            "outputs": [{"name": "ok", "type": "boolean"}],
            "rows": [
                {"id": "r1", "inputCells": ["income <= 1473"], "outputCells": ["true"]}
            ],
            "hitPolicy": "first"
        }"#;
        let table: DecisionTable = serde_json::from_str(json).expect("parse DecisionTable");
        assert_eq!(table.id, "elig");
        assert_eq!(table.hit_policy, HitPolicy::First);
        assert_eq!(table.rows.len(), 1);
        let reserialized = serde_json::to_string(&table).expect("serialize");
        let reparsed: DecisionTable = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(reparsed.id, "elig");
    }

    #[test]
    fn rejects_unknown_hit_policy() {
        let json = r#"{
            "id": "t",
            "inputs": [],
            "outputs": [{"name": "ok", "type": "boolean"}],
            "rows": [],
            "hitPolicy": "bogus"
        }"#;
        let result: Result<DecisionTable, _> = serde_json::from_str(json);
        assert!(result.is_err(), "expected hit-policy enum rejection");
    }

    #[test]
    fn rejects_object_with_wrong_kind_discriminator() {
        // Object form with kind: "fooBar" — should NOT match Guard::DecisionTable
        // because DecisionTableGuardKind only accepts "decisionTable".
        // Under untagged, both Fel(String) and DecisionTable(DecisionTableGuard)
        // attempt to deserialize; both fail; result is an error.
        let json = r#"{
            "kind": "fooBar",
            "ref": "t",
            "outputColumn": "ok",
            "inputBindings": {}
        }"#;
        let result: Result<Guard, _> = serde_json::from_str(json);
        assert!(result.is_err(), "expected rejection of unknown kind");
    }
}
