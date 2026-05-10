// Rust guideline compliant 2026-02-21

//! K-051 / K-052 / K-053 — DecisionTable lint rules per Kernel §4.5.1.
//!
//! These rules validate the structural integrity of the parent kernel's
//! `decisionTable` first-class construct (landed 2026-05-01). The
//! corresponding schema definitions live in
//! `schemas/wos-workflow.schema.json` (`$defs/{DecisionTable,
//! DecisionTableRow, DecisionTableGuard}`); the typed Rust model lives
//! in [`wos_core::model::decision_table`].
//!
//! ## Rule catalogue
//!
//! | Rule  | Tier | Summary                                                                 |
//! |-------|------|-------------------------------------------------------------------------|
//! | K-051 | T1   | DecisionTableGuard `ref`/`outputColumn`/`inputBindings` MUST resolve.   |
//! | K-052 | T2   | `unique`/`priority` rows MUST be pairwise disjoint (priority ties → K-052). |
//! | K-053 | T1   | Transition-guard outputColumn MUST be boolean; `collect` rejected on guards. |
//!
//! ## Diagnostic shape
//!
//! Each rule emits zero or more [`LintDiagnostic`] records via
//! [`LintDiagnostic::t1_error`] / [`LintDiagnostic::t2_error`] /
//! [`LintDiagnostic::t2_warning`]. The `path` field is the
//! JSON-Pointer-shaped path to the offending location, matching the
//! K-049 exemplar in `continuous_mode.rs` and documented in
//! `diagnostic.rs:104`.
//!
//! ## K-052 satisfiability
//!
//! Detecting row-overlap in the general case requires SMT. The Stage-4
//! pragmatic implementation handles the common case: single-variable
//! linear inequalities (`<`, `<=`, `>`, `>=`, `==`, `!=`) over a numeric
//! input. Cases that the analyzer cannot prove disjoint emit a
//! `LintSeverity::Warning` ("potential overlap; manual review");
//! provably-overlapping cases emit `LintSeverity::Error`.

use std::collections::HashMap;

use wos_core::model::decision_table::{
    DecisionTable, DecisionTableGuard, FelType, Guard, HitPolicy,
};
use wos_core::model::kernel::{KernelDocument, State, Transition};

use crate::diagnostic::LintDiagnostic;

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Run K-051 / K-052 / K-053 against a typed kernel document, appending
/// to `diagnostics`.
///
/// Composes the three rules in the order their dependency relationships
/// require: K-051 (resolution) first because K-052 / K-053 reuse the
/// resolution work to walk into table internals.
pub fn check(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Build a lookup table: id -> &DecisionTable. Reused by all three rules.
    let table_index: HashMap<&str, &DecisionTable> = kernel
        .decision_tables
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    // Walk every transition guard once, dispatching to per-rule checks
    // on each DecisionTableGuard.
    for_each_decision_table_guard(
        &kernel.lifecycle.states,
        "/lifecycle/states",
        &mut |path, guard| {
            check_k051_guard(path, guard, &table_index, diagnostics);
            check_k053_on_guard(path, guard, &table_index, diagnostics);
        },
    );

    // K-052 walks the tables themselves (not the guards), once.
    for table in &kernel.decision_tables {
        check_k052_table(table, diagnostics);
    }
}

// ---------------------------------------------------------------------------
// Guard traversal helper
// ---------------------------------------------------------------------------

fn for_each_decision_table_guard<F>(
    states: &indexmap::IndexMap<String, State>,
    parent_path: &str,
    callback: &mut F,
) where
    F: FnMut(&str, &DecisionTableGuard),
{
    for (name, state) in states {
        let state_path = format!("{parent_path}/{name}");
        for (idx, transition) in state.transitions.iter().enumerate() {
            let transition_path = format!("{state_path}/transitions/{idx}");
            if let Some(g) = transition_guard_dt(transition) {
                let guard_path = format!("{transition_path}/guard");
                callback(&guard_path, g);
            }
        }
        // Recurse into compound substates and parallel regions.
        for_each_decision_table_guard(&state.states, &state_path, callback);
        for (region_name, region) in &state.regions {
            let region_path = format!("{state_path}/regions/{region_name}");
            for_each_decision_table_guard(&region.states, &region_path, callback);
        }
    }
}

fn transition_guard_dt(transition: &Transition) -> Option<&DecisionTableGuard> {
    transition.guard.as_ref().and_then(Guard::as_decision_table)
}

// ---------------------------------------------------------------------------
// K-051 — DecisionTableGuard structural resolution (T1)
// ---------------------------------------------------------------------------

fn check_k051_guard(
    guard_path: &str,
    guard: &DecisionTableGuard,
    table_index: &HashMap<&str, &DecisionTable>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(table) = table_index.get(guard.table_ref.as_str()) else {
        // Build "declared tables" hint for the message.
        let mut declared: Vec<&str> = table_index.keys().copied().collect();
        declared.sort();
        let declared_hint = if declared.is_empty() {
            "(no tables declared)".to_string()
        } else {
            declared.join(", ")
        };
        diagnostics.push(LintDiagnostic::t1_error(
            "K-051",
            guard_path.to_string(),
            format!(
                "K-051: DecisionTableGuard at {guard_path} references unknown table '{}' (declared tables: {declared_hint})",
                guard.table_ref,
            ),
        ));
        return;
    };

    // outputColumn MUST resolve to a declared output.
    if !table.outputs.iter().any(|o| o.name == guard.output_column) {
        let declared: Vec<&str> = table.outputs.iter().map(|o| o.name.as_str()).collect();
        diagnostics.push(LintDiagnostic::t1_error(
            "K-051",
            guard_path.to_string(),
            format!(
                "K-051: DecisionTableGuard at {guard_path} references unknown outputColumn '{}' on table '{}' (declared outputs: {})",
                guard.output_column,
                guard.table_ref,
                declared.join(", "),
            ),
        ));
    }

    // Every declared input MUST have an inputBindings entry.
    for input in &table.inputs {
        if !guard.input_bindings.contains_key(&input.name) {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-051",
                guard_path.to_string(),
                format!(
                    "K-051: DecisionTableGuard at {guard_path} is missing inputBindings entry for declared input '{}' (table '{}')",
                    input.name, guard.table_ref,
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// K-053 — cell shape + hit-policy validity (T1)
// ---------------------------------------------------------------------------

fn check_k053_on_guard(
    guard_path: &str,
    guard: &DecisionTableGuard,
    table_index: &HashMap<&str, &DecisionTable>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(table) = table_index.get(guard.table_ref.as_str()) else {
        // K-051 already complained; don't double-report.
        return;
    };

    // 1. Output column MUST be boolean for transition-guard usage.
    if let Some(output) = table.outputs.iter().find(|o| o.name == guard.output_column) {
        if output.kind != FelType::Boolean {
            diagnostics.push(LintDiagnostic::t1_error(
                "K-053",
                guard_path.to_string(),
                format!(
                    "K-053: DecisionTableGuard at {guard_path} selects outputColumn '{}' which is type '{}'; transition-guard tables MUST select a boolean-typed output column",
                    guard.output_column,
                    fel_type_name(output.kind),
                ),
            ));
        }
    }

    // 2. hitPolicy = collect is rejected for transition-guard usage.
    if table.hit_policy == HitPolicy::Collect {
        diagnostics.push(LintDiagnostic::t1_error(
            "K-053",
            guard_path.to_string(),
            format!(
                "K-053: DecisionTable '{}' has hitPolicy='collect' but is referenced by a transition guard at {guard_path}; collect is reserved for non-guard consumers",
                guard.table_ref,
            ),
        ));
    }
}

fn fel_type_name(kind: FelType) -> &'static str {
    match kind {
        FelType::String => "string",
        FelType::Number => "number",
        FelType::Integer => "integer",
        FelType::Boolean => "boolean",
        FelType::Date => "date",
        FelType::Datetime => "datetime",
        FelType::Duration => "duration",
    }
}

// ---------------------------------------------------------------------------
// K-052 — row overlap detection (T2)
// ---------------------------------------------------------------------------

fn check_k052_table(table: &DecisionTable, diagnostics: &mut Vec<LintDiagnostic>) {
    if !matches!(table.hit_policy, HitPolicy::Unique | HitPolicy::Priority) {
        return;
    }

    let table_path = format!("/decisionTables/{}", table.id);

    // Pairwise overlap detection. n^2 over rows; bounded by table size.
    for (i, row_a) in table.rows.iter().enumerate() {
        for row_b in table.rows.iter().skip(i + 1) {
            let verdict = rows_overlap(table, row_a, row_b);
            match verdict {
                OverlapVerdict::ProvablyOverlap => {
                    if table.hit_policy == HitPolicy::Unique {
                        diagnostics.push(LintDiagnostic::t2_error(
                            "K-052",
                            table_path.clone(),
                            format!(
                                "K-052: DecisionTable '{}' has hitPolicy='unique' but rows '{}' and '{}' have overlapping input domains",
                                table.id, row_a.id, row_b.id,
                            ),
                        ));
                    } else if table.hit_policy == HitPolicy::Priority {
                        // Priority ties on overlapping rows are violations.
                        let pa = row_a.priority;
                        let pb = row_b.priority;
                        if pa.is_some() && pa == pb {
                            diagnostics.push(LintDiagnostic::t2_error(
                                "K-052",
                                table_path.clone(),
                                format!(
                                    "K-052: DecisionTable '{}' hitPolicy='priority' rows '{}' and '{}' have overlapping input domains and identical priority",
                                    table.id, row_a.id, row_b.id,
                                ),
                            ));
                        }
                    }
                }
                OverlapVerdict::PotentialOverlap => {
                    diagnostics.push(LintDiagnostic::t2_warning(
                        "K-052",
                        table_path.clone(),
                        format!(
                            "K-052: DecisionTable '{}' rows '{}' and '{}' may have overlapping input domains; static analyzer cannot prove disjointness — manual review",
                            table.id, row_a.id, row_b.id,
                        ),
                    ));
                }
                OverlapVerdict::ProvablyDisjoint => {}
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlapVerdict {
    ProvablyDisjoint,
    PotentialOverlap,
    ProvablyOverlap,
}

/// Pragmatic overlap analyzer.
///
/// For each input column independently, parse both rows' input cells as
/// linear-inequality predicates over that input. Two rows are provably
/// disjoint if AT LEAST ONE input column has provably-disjoint predicates.
/// They are provably overlapping if EVERY column has provably-overlapping
/// predicates AND we successfully parsed every cell (otherwise the verdict
/// is `PotentialOverlap`).
///
/// The Stage-4 implementation handles the FEL subset:
/// - `<input> <op> <number>` for op ∈ {<, <=, >, >=, ==, !=}
/// - `true` / `false` cells (always-match, always-skip)
/// - empty input cells (default rows, always match)
fn rows_overlap(
    table: &DecisionTable,
    row_a: &wos_core::model::decision_table::DecisionTableRow,
    row_b: &wos_core::model::decision_table::DecisionTableRow,
) -> OverlapVerdict {
    // Empty input-cells row matches everything.
    let a_default = row_a.input_cells.is_empty();
    let b_default = row_b.input_cells.is_empty();
    if a_default && b_default {
        return OverlapVerdict::ProvablyOverlap;
    }
    if a_default || b_default {
        // One row matches everything; the other matches some non-empty set.
        // They overlap iff the other row's predicates are satisfiable —
        // assume yes (any non-trivial row matches at least one input
        // assignment) which makes overlap PROVABLE here.
        return OverlapVerdict::ProvablyOverlap;
    }

    // Both rows have explicit input cells.
    if row_a.input_cells.len() != table.inputs.len()
        || row_b.input_cells.len() != table.inputs.len()
    {
        // Malformed cells (length mismatch). Not our problem here; another
        // rule should catch.
        return OverlapVerdict::PotentialOverlap;
    }

    // Per-column analysis.
    let mut all_columns_overlap_provable = true;
    for (idx, input) in table.inputs.iter().enumerate() {
        let cell_a = &row_a.input_cells[idx];
        let cell_b = &row_b.input_cells[idx];
        match column_overlap(&input.name, cell_a, cell_b) {
            ColumnVerdict::Disjoint => return OverlapVerdict::ProvablyDisjoint,
            ColumnVerdict::Overlap => {} // continue — need every column
            ColumnVerdict::Unknown => {
                all_columns_overlap_provable = false;
            }
        }
    }
    if all_columns_overlap_provable {
        OverlapVerdict::ProvablyOverlap
    } else {
        OverlapVerdict::PotentialOverlap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnVerdict {
    Disjoint,
    Overlap,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LinearOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // `var` carries the predicated variable name for
// future debug output; no current reader.
struct LinearPred<'a> {
    var: &'a str,
    op: LinearOp,
    value: f64,
}

/// Determine whether two cell predicates over the same input variable
/// overlap. Handles single-variable linear inequalities + equality.
fn column_overlap(var_name: &str, cell_a: &str, cell_b: &str) -> ColumnVerdict {
    // Special cases: literal "true" / "false" in a cell.
    let trim_a = cell_a.trim();
    let trim_b = cell_b.trim();
    if trim_a == "true" {
        return if trim_b == "false" {
            ColumnVerdict::Disjoint
        } else {
            ColumnVerdict::Overlap
        };
    }
    if trim_b == "true" {
        return if trim_a == "false" {
            ColumnVerdict::Disjoint
        } else {
            ColumnVerdict::Overlap
        };
    }
    if trim_a == "false" || trim_b == "false" {
        // 'false' never matches any input; the row never fires; treat as
        // disjoint from any other predicate over this column.
        return ColumnVerdict::Disjoint;
    }

    // Try to parse both cells as linear predicates.
    let pred_a = parse_linear_pred(var_name, trim_a);
    let pred_b = parse_linear_pred(var_name, trim_b);

    match (pred_a, pred_b) {
        (Some(a), Some(b)) => match overlap_of_linear_preds(a, b) {
            Some(true) => ColumnVerdict::Overlap,
            Some(false) => ColumnVerdict::Disjoint,
            None => ColumnVerdict::Unknown,
        },
        _ => ColumnVerdict::Unknown,
    }
}

/// Parse `<var> <op> <number>` (or reversed). Returns None if the cell
/// doesn't match the simple linear-predicate shape.
fn parse_linear_pred<'a>(var_name: &'a str, cell: &str) -> Option<LinearPred<'a>> {
    // Try in order: '<=', '>=', '==', '!=', '<', '>'.
    let ops: &[(&str, LinearOp)] = &[
        ("<=", LinearOp::Le),
        (">=", LinearOp::Ge),
        ("==", LinearOp::Eq),
        ("!=", LinearOp::Ne),
        ("<", LinearOp::Lt),
        (">", LinearOp::Gt),
    ];
    for (sym, op) in ops {
        if let Some((lhs, rhs)) = cell.split_once(sym) {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            // var on lhs, number on rhs
            if lhs == var_name {
                if let Ok(n) = rhs.parse::<f64>() {
                    return Some(LinearPred {
                        var: var_name,
                        op: *op,
                        value: n,
                    });
                }
            }
            // number on lhs, var on rhs — flip the operator
            if rhs == var_name {
                if let Ok(n) = lhs.parse::<f64>() {
                    let flipped = match op {
                        LinearOp::Lt => LinearOp::Gt,
                        LinearOp::Le => LinearOp::Ge,
                        LinearOp::Gt => LinearOp::Lt,
                        LinearOp::Ge => LinearOp::Le,
                        LinearOp::Eq => LinearOp::Eq,
                        LinearOp::Ne => LinearOp::Ne,
                    };
                    return Some(LinearPred {
                        var: var_name,
                        op: flipped,
                        value: n,
                    });
                }
            }
        }
    }
    None
}

/// Returns Some(true) if the two predicates' satisfiable sets intersect,
/// Some(false) if they are provably disjoint, None if undetermined.
fn overlap_of_linear_preds(a: LinearPred<'_>, b: LinearPred<'_>) -> Option<bool> {
    // Treat each predicate as an interval/half-line over R.
    // Compute intersection; if non-empty, they overlap.
    let a_iv = pred_to_interval(a);
    let b_iv = pred_to_interval(b);
    Some(intervals_overlap(&a_iv, &b_iv))
}

#[derive(Debug, Clone, Copy)]
enum Interval {
    /// (-inf, value) or (-inf, value] etc.
    Bounded {
        min: f64,
        min_inclusive: bool,
        max: f64,
        max_inclusive: bool,
    },
    Equals(f64),
    /// Complement of Equals(value): everything except the point.
    NotEquals(f64),
}

fn pred_to_interval(p: LinearPred<'_>) -> Interval {
    match p.op {
        LinearOp::Lt => Interval::Bounded {
            min: f64::NEG_INFINITY,
            min_inclusive: false,
            max: p.value,
            max_inclusive: false,
        },
        LinearOp::Le => Interval::Bounded {
            min: f64::NEG_INFINITY,
            min_inclusive: false,
            max: p.value,
            max_inclusive: true,
        },
        LinearOp::Gt => Interval::Bounded {
            min: p.value,
            min_inclusive: false,
            max: f64::INFINITY,
            max_inclusive: false,
        },
        LinearOp::Ge => Interval::Bounded {
            min: p.value,
            min_inclusive: true,
            max: f64::INFINITY,
            max_inclusive: false,
        },
        LinearOp::Eq => Interval::Equals(p.value),
        LinearOp::Ne => Interval::NotEquals(p.value),
    }
}

fn intervals_overlap(a: &Interval, b: &Interval) -> bool {
    match (a, b) {
        (Interval::Equals(v1), Interval::Equals(v2)) => (v1 - v2).abs() < f64::EPSILON,
        (Interval::Equals(v), Interval::NotEquals(w))
        | (Interval::NotEquals(w), Interval::Equals(v)) => (v - w).abs() >= f64::EPSILON,
        (Interval::NotEquals(_), Interval::NotEquals(_)) => true,
        (
            Interval::Equals(v),
            Interval::Bounded {
                min,
                min_inclusive,
                max,
                max_inclusive,
            },
        )
        | (
            Interval::Bounded {
                min,
                min_inclusive,
                max,
                max_inclusive,
            },
            Interval::Equals(v),
        ) => {
            let above_min = if *min_inclusive { v >= min } else { v > min };
            let below_max = if *max_inclusive { v <= max } else { v < max };
            above_min && below_max
        }
        (Interval::NotEquals(_), Interval::Bounded { .. })
        | (Interval::Bounded { .. }, Interval::NotEquals(_)) => {
            // Bounded interval has infinitely many points (assuming non-empty);
            // removing one point still overlaps.
            true
        }
        (
            Interval::Bounded {
                min: a_min,
                min_inclusive: a_min_inc,
                max: a_max,
                max_inclusive: a_max_inc,
            },
            Interval::Bounded {
                min: b_min,
                min_inclusive: b_min_inc,
                max: b_max,
                max_inclusive: b_max_inc,
            },
        ) => {
            // Intersection min = max(a_min, b_min); intersection max =
            // min(a_max, b_max). Non-empty iff intersection_min <
            // intersection_max OR equal-with-both-inclusive.
            let (lo, lo_inc) = if a_min > b_min {
                (*a_min, *a_min_inc)
            } else if a_min < b_min {
                (*b_min, *b_min_inc)
            } else {
                (*a_min, *a_min_inc && *b_min_inc)
            };
            let (hi, hi_inc) = if a_max < b_max {
                (*a_max, *a_max_inc)
            } else if a_max > b_max {
                (*b_max, *b_max_inc)
            } else {
                (*a_max, *a_max_inc && *b_max_inc)
            };
            if lo < hi {
                true
            } else if (lo - hi).abs() < f64::EPSILON {
                lo_inc && hi_inc
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use wos_core::model::decision_table::{
        DecisionTable, DecisionTableGuardKind, DecisionTableInput, DecisionTableOutput,
        DecisionTableRow,
    };

    fn fel_guard(g: DecisionTableGuard) -> Guard {
        Guard::DecisionTable(g)
    }

    fn make_dt(id: &str, hit_policy: HitPolicy) -> DecisionTable {
        DecisionTable {
            id: id.to_string(),
            description: None,
            inputs: vec![DecisionTableInput {
                name: "income".to_string(),
                kind: FelType::Number,
                description: None,
            }],
            outputs: vec![DecisionTableOutput {
                name: "eligible".to_string(),
                kind: FelType::Boolean,
                description: None,
            }],
            rows: vec![DecisionTableRow {
                id: "r1".to_string(),
                input_cells: vec!["income <= 1473".to_string()],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: None,
            }],
            hit_policy,
        }
    }

    fn make_guard(table_ref: &str, output: &str) -> DecisionTableGuard {
        let mut bindings = IndexMap::new();
        bindings.insert("income".to_string(), "caseFile.income".to_string());
        DecisionTableGuard {
            kind: DecisionTableGuardKind::DecisionTable,
            table_ref: table_ref.to_string(),
            output_column: output.to_string(),
            input_bindings: bindings,
            on_no_match: None,
        }
    }

    #[test]
    fn k051_resolves_when_table_and_output_exist() {
        let table = make_dt("elig", HitPolicy::First);
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("elig", "eligible");
        let mut diags = Vec::new();
        check_k051_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags.is_empty(),
            "expected no K-051 diagnostics, got {:?}",
            diags
        );
    }

    #[test]
    fn k051_flags_unresolved_table_ref() {
        let table = make_dt("elig", HitPolicy::First);
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("nonExistentTable", "eligible");
        let mut diags = Vec::new();
        check_k051_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-051" && d.message.contains("unknown table")),
            "expected K-051 unknown-table; got {:?}",
            diags
        );
    }

    #[test]
    fn k051_flags_unresolved_output_column() {
        let table = make_dt("elig", HitPolicy::First);
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("elig", "approved"); // typo
        let mut diags = Vec::new();
        check_k051_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-051" && d.message.contains("unknown outputColumn")),
            "expected K-051 unknown-output; got {:?}",
            diags
        );
    }

    #[test]
    fn k051_flags_missing_input_binding() {
        let mut table = make_dt("elig", HitPolicy::First);
        table.inputs.push(DecisionTableInput {
            name: "householdSize".to_string(),
            kind: FelType::Integer,
            description: None,
        });
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("elig", "eligible"); // only binds 'income'
        let mut diags = Vec::new();
        check_k051_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags.iter().any(|d| d.rule_id == "K-051"
                && d.message.contains("missing inputBindings entry")
                && d.message.contains("householdSize")),
            "expected K-051 missing-binding; got {:?}",
            diags
        );
    }

    #[test]
    fn k053_flags_non_boolean_output_column() {
        let mut table = make_dt("elig", HitPolicy::First);
        table.outputs[0].kind = FelType::String;
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("elig", "eligible");
        let mut diags = Vec::new();
        check_k053_on_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-053" && d.message.contains("MUST select a boolean")),
            "expected K-053 non-boolean; got {:?}",
            diags
        );
    }

    #[test]
    fn k053_flags_collect_hit_policy_on_guard() {
        let table = make_dt("elig", HitPolicy::Collect);
        let table_index: HashMap<&str, &DecisionTable> =
            std::iter::once(("elig", &table)).collect();
        let g = make_guard("elig", "eligible");
        let mut diags = Vec::new();
        check_k053_on_guard(
            "/lifecycle/states/x/transitions/0/guard",
            &g,
            &table_index,
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-053" && d.message.contains("hitPolicy='collect'")),
            "expected K-053 collect-on-guard; got {:?}",
            diags
        );
    }

    #[test]
    fn k052_disjoint_unique_rows_no_diagnostic() {
        let mut table = make_dt("elig", HitPolicy::Unique);
        table.rows = vec![
            DecisionTableRow {
                id: "rEligible".to_string(),
                input_cells: vec!["income <= 1473".to_string()],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: None,
            },
            DecisionTableRow {
                id: "rIneligible".to_string(),
                input_cells: vec!["income > 1473".to_string()],
                output_cells: vec!["false".to_string()],
                priority: None,
                rationale: None,
            },
        ];
        let mut diags = Vec::new();
        check_k052_table(&table, &mut diags);
        assert!(
            diags.is_empty(),
            "expected no K-052 on disjoint rows; got {:?}",
            diags
        );
    }

    #[test]
    fn k052_overlapping_unique_rows_flagged() {
        let mut table = make_dt("elig", HitPolicy::Unique);
        table.rows = vec![
            DecisionTableRow {
                id: "rTier1".to_string(),
                input_cells: vec!["income <= 1500".to_string()],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: None,
            },
            DecisionTableRow {
                id: "rTier2".to_string(),
                input_cells: vec!["income <= 2000".to_string()],
                output_cells: vec!["true".to_string()],
                priority: None,
                rationale: None,
            },
        ];
        let mut diags = Vec::new();
        check_k052_table(&table, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-052" && d.message.contains("overlapping")),
            "expected K-052 overlap; got {:?}",
            diags
        );
    }

    #[test]
    fn k052_priority_tie_flagged() {
        let mut table = make_dt("elig", HitPolicy::Priority);
        table.rows = vec![
            DecisionTableRow {
                id: "rA".to_string(),
                input_cells: vec!["income <= 1500".to_string()],
                output_cells: vec!["true".to_string()],
                priority: Some(1),
                rationale: None,
            },
            DecisionTableRow {
                id: "rB".to_string(),
                input_cells: vec!["income <= 2000".to_string()],
                output_cells: vec!["false".to_string()],
                priority: Some(1),
                rationale: None,
            },
        ];
        let mut diags = Vec::new();
        check_k052_table(&table, &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.rule_id == "K-052" && d.message.contains("identical priority")),
            "expected K-052 priority-tie; got {:?}",
            diags
        );
    }

    // Suppress unused-import warning when only some tests reference fel_guard.
    #[allow(dead_code)]
    fn _unused_anchor() -> Guard {
        fel_guard(make_guard("t", "ok"))
    }
}
