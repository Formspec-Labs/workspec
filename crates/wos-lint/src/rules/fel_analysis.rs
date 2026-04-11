// Rust guideline compliant 2026-02-21

//! FEL AST analysis rules (T2-ast tier).
//!
//! Parses FEL expression strings found in WOS documents and checks normative
//! constraints that require inspecting the AST: expression validity, cross-case
//! reference prohibition, function catalog conformance, and SMT-subset rules.
//!
//! # Rule coverage
//!
//! | Rule   | Category           | What is checked                                  |
//! |--------|--------------------|--------------------------------------------------|
//! | K-012  | expression-validity | Guard expressions are valid FEL                |
//! | K-013  | expression-validity | Milestone condition fields are valid FEL       |
//! | K-017  | expression-validity | Guards must not reference related-case state   |
//! | K-019  | expression-validity | Only built-in + extension functions used       |
//! | G-042  | expression-validity | Assertion `expression` fields are valid FEL    |
//! | G-043  | expression-validity | Delegation scope `conditions` are valid FEL    |
//! | AI-024 | expression-validity | Escalation conditions are valid FEL + use `@agent` |
//! | AG-010 | smt-compatibility   | Verifiable constraints satisfy all SMT rules   |
//! | AG-011 | smt-compatibility   | `let` bindings are not recursive               |
//! | AG-012 | smt-compatibility   | Quantifiers quantify over finite domains       |
//! | AG-013 | smt-compatibility   | Arithmetic is linear (no variable × variable)  |
//! | AG-014 | smt-compatibility   | No extension function calls in verifiable subset|
//!
//! **AG-010 (finite equality):** warns when both sides of `==` / `!=` are simple
//! field/context accesses and neither side is a literal, unless a path is a known
//! WOS enumeration field or listed in `finiteDomainDeclarations`.

use std::collections::{HashMap, HashSet};

use fel_core::{
    ast::{BinaryOp, Expr, PathSegment},
    builtin_function_catalog, parse,
};
use serde_json::Value;

use crate::diagnostic::Diagnostic;
use crate::document::{DocumentKind, WosDocument, WosProject};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all FEL AST analysis checks across every document in the project.
pub fn check(project: &WosProject, diagnostics: &mut Vec<Diagnostic>) {
    for doc in project.documents() {
        match doc.kind {
            DocumentKind::Kernel => check_kernel_fel(doc, diagnostics),
            DocumentKind::WorkflowGovernance => check_governance_fel(doc, diagnostics),
            DocumentKind::AiIntegration => check_ai_integration_fel(doc, diagnostics),
            DocumentKind::Advanced => check_advanced_governance_fel(doc, diagnostics),
            DocumentKind::AssertionLibrary => check_assertion_library_fel(doc, diagnostics),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Per-document-kind dispatchers
// ---------------------------------------------------------------------------

/// Check FEL in a Kernel document (K-012, K-013, K-017, K-019).
fn check_kernel_fel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(states) = doc
        .value
        .pointer("/lifecycle/states")
        .and_then(Value::as_object)
    {
        check_states_fel(states, "/lifecycle/states", diagnostics);
    }
    check_milestones_fel(&doc.value, diagnostics);
}

/// Check FEL in a WorkflowGovernance document (G-043).
fn check_governance_fel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(delegations) = doc.value.get("delegations").and_then(Value::as_array) {
        for (i, delegation) in delegations.iter().enumerate() {
            let base_path = format!("/delegations/{i}");
            check_delegation_conditions(delegation, &base_path, diagnostics);
        }
    }
}

/// Check FEL in an AI Integration document (AI-024).
fn check_ai_integration_fel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(agents) = doc.value.get("agents").and_then(Value::as_object) {
        for (agent_name, agent) in agents {
            let base_path = format!("/agents/{agent_name}");
            check_escalation_conditions(agent, &base_path, diagnostics);
        }
    }
}

/// Check FEL in an Advanced Governance document (AG-010 through AG-014).
fn check_advanced_governance_fel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(constraints) = doc
        .value
        .get("verifiableConstraints")
        .and_then(Value::as_array)
    {
        for (i, constraint) in constraints.iter().enumerate() {
            let path = format!("/verifiableConstraints/{i}");
            if let Some(expr_str) = constraint.get("expression").and_then(Value::as_str) {
                let decls = parse_finite_domain_declarations(constraint.get("finiteDomainDeclarations"));
                check_smt_expression(expr_str, &path, diagnostics, &decls);
            }
        }
    }
}

/// Check FEL in an Assertion Library document (G-042).
fn check_assertion_library_fel(doc: &WosDocument, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(assertions) = doc.value.get("assertions").and_then(Value::as_array) {
        for (i, assertion) in assertions.iter().enumerate() {
            let path = format!("/assertions/{i}/expression");
            if let Some(expr_str) = assertion.get("expression").and_then(Value::as_str) {
                check_expression_syntax("G-042", expr_str, &path, diagnostics);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// K-012, K-013, K-017, K-019: Kernel FEL checks
// ---------------------------------------------------------------------------

/// Recursively check guard expressions in all states and their substates.
fn check_states_fel(
    states: &serde_json::Map<String, Value>,
    path_prefix: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (state_name, state) in states {
        let state_path = format!("{path_prefix}/{state_name}");

        // K-012: Guards on transitions must be valid FEL.
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for (ti, transition) in transitions.iter().enumerate() {
                let t_path = format!("{state_path}/transitions/{ti}");
                if let Some(guard) = transition.get("guard").and_then(Value::as_str) {
                    check_guard_expression(guard, &format!("{t_path}/guard"), diagnostics);
                }
            }
        }

        // Recurse into compound substates.
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            check_states_fel(substates, &format!("{state_path}/states"), diagnostics);
        }

        // Recurse into parallel regions.
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for (region_name, region) in regions {
                let region_path = format!("{state_path}/regions/{region_name}");
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    check_states_fel(rstates, &format!("{region_path}/states"), diagnostics);
                }
            }
        }
    }
}

/// K-012 + K-017 + K-019: Parse a guard expression and run structural checks.
fn check_guard_expression(expr_str: &str, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let expr = match parse(expr_str) {
        Ok(e) => e,
        Err(err) => {
            diagnostics.push(Diagnostic::error(
                "K-012",
                path,
                format!("guard expression is not valid FEL: {err}"),
            ));
            return;
        }
    };

    // K-017: Guards must not reference related-case state.
    check_no_related_case_refs(&expr, "K-017", path, diagnostics);

    // K-019: Only built-in + extension functions.
    check_only_builtin_functions(&expr, "K-019", path, diagnostics);
}

/// K-013: Milestone condition fields must be valid FEL.
fn check_milestones_fel(root: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let Some(milestones) = root
        .pointer("/lifecycle/milestones")
        .and_then(Value::as_array)
    else {
        return;
    };

    for (i, milestone) in milestones.iter().enumerate() {
        let path = format!("/lifecycle/milestones/{i}/condition");
        if let Some(condition) = milestone.get("condition").and_then(Value::as_str) {
            check_expression_syntax("K-013", condition, &path, diagnostics);
        }
    }
}

// ---------------------------------------------------------------------------
// G-043: Delegation conditions
// ---------------------------------------------------------------------------

/// G-043: `conditions` array entries in a delegation must be valid FEL.
fn check_delegation_conditions(
    delegation: &Value,
    base_path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(conditions) = delegation.get("conditions").and_then(Value::as_array) else {
        return;
    };

    for (i, condition) in conditions.iter().enumerate() {
        let path = format!("{base_path}/conditions/{i}");
        if let Some(expr_str) = condition.as_str() {
            check_expression_syntax("G-043", expr_str, &path, diagnostics);
        }
    }
}

// ---------------------------------------------------------------------------
// AI-024: Escalation conditions
// ---------------------------------------------------------------------------

/// AI-024: Escalation conditions must be valid FEL that references `@agent` context.
fn check_escalation_conditions(agent: &Value, base_path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let Some(escalation) = agent.get("escalation") else {
        return;
    };

    let Some(conditions) = escalation.get("conditions").and_then(Value::as_array) else {
        return;
    };

    for (i, condition) in conditions.iter().enumerate() {
        let path = format!("{base_path}/escalation/conditions/{i}");
        if let Some(expr_str) = condition.as_str() {
            let expr = match parse(expr_str) {
                Ok(e) => e,
                Err(err) => {
                    diagnostics.push(Diagnostic::error(
                        "AI-024",
                        &path,
                        format!("escalation condition is not valid FEL: {err}"),
                    ));
                    continue;
                }
            };

            if !references_agent_context(&expr) {
                diagnostics.push(Diagnostic::warning(
                    "AI-024",
                    &path,
                    "escalation condition should reference @agent context",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AG-010 through AG-014: SMT verifiable subset
// ---------------------------------------------------------------------------

/// Load `finiteDomainDeclarations` paths from a constraint JSON object.
///
/// Shape: `{ "path.to.field": { "domain": ["v1", "v2", ...] }, ... }`.
/// Entries without a non-empty `domain` array of strings are ignored.
fn parse_finite_domain_declarations(value: Option<&Value>) -> HashMap<String, ()> {
    let mut out = HashMap::new();
    let Some(Value::Object(map)) = value else {
        return out;
    };
    for (key, entry) in map {
        let Some(domain) = entry.get("domain").and_then(Value::as_array) else {
            continue;
        };
        if domain.is_empty() || !domain.iter().all(|v| v.as_str().is_some()) {
            continue;
        }
        out.insert(key.clone(), ());
    }
    out
}

/// AG-010: Entry point for all SMT subset checks on a single expression.
///
/// Applies AG-011, AG-012, AG-013, AG-014, and finite-domain equality (AG-010)
/// in sequence. Each violation is reported with its own rule ID.
fn check_smt_expression(
    expr_str: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
    finite_domain_paths: &HashMap<String, ()>,
) {
    let expr = match parse(expr_str) {
        Ok(e) => e,
        Err(err) => {
            diagnostics.push(Diagnostic::error(
                "AG-010",
                path,
                format!("verifiable constraint is not valid FEL: {err}"),
            ));
            return;
        }
    };

    // AG-011: no recursive let bindings.
    let mut let_names: HashSet<String> = HashSet::new();
    check_no_recursive_let(&expr, &mut let_names, "AG-011", path, diagnostics);

    // AG-012: quantifiers must quantify over finite domains (partial check).
    check_finite_quantifiers(&expr, "AG-012", path, diagnostics);

    // AG-013: arithmetic must be linear.
    check_linear_arithmetic(&expr, "AG-013", path, diagnostics);

    // AG-014: no extension function calls.
    check_no_extension_functions(&expr, "AG-014", path, diagnostics);

    // AG-010 (finite equality): variable-to-variable equality on simple paths.
    check_finite_domain_equality(&expr, path, diagnostics, finite_domain_paths);
}

// ---------------------------------------------------------------------------
// Helpers: syntax-only parse (K-013, G-042, G-043)
// ---------------------------------------------------------------------------

/// Parse `expr_str` and emit a diagnostic with `rule_id` if it fails.
fn check_expression_syntax(
    rule_id: &'static str,
    expr_str: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Err(err) = parse(expr_str) {
        diagnostics.push(Diagnostic::error(
            rule_id,
            path,
            format!("expression is not valid FEL: {err}"),
        ));
    }
}

// ---------------------------------------------------------------------------
// AST walkers
// ---------------------------------------------------------------------------

/// K-017: Detect references to related-case state in an expression.
///
/// "Related case" references are `$` field-refs whose first path segment
/// begins with `relatedCase` or uses a wildcard to dereference it, as well
/// as `@relatedCase` context refs. This covers the explicit patterns the
/// spec prohibits: `$relatedCase.status`, `@relatedCase.field`, etc.
fn check_no_related_case_refs(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_expr(expr, &mut |e| {
        match e {
            Expr::FieldRef { name, .. } => {
                if name.as_deref().is_some_and(is_related_case_name) {
                    diagnostics.push(Diagnostic::error(
                        rule_id,
                        path,
                        format!(
                            "guard references related-case field '{}'; guards must not access \
                             related case state",
                            name.as_deref().unwrap_or_default()
                        ),
                    ));
                }
            }
            Expr::ContextRef { name, .. } => {
                if is_related_case_name(name) {
                    diagnostics.push(Diagnostic::error(
                        rule_id,
                        path,
                        format!(
                            "guard references related-case context '@{name}'; guards must not \
                             access related case state"
                        ),
                    ));
                }
            }
            Expr::PostfixAccess {
                expr: inner,
                path: segments,
            } => {
                // Postfix chains like `$someField.relatedCase` — check the first dot segment.
                if let Some(PathSegment::Dot(first)) = segments.first() {
                    if is_related_case_name(first) {
                        // We only warn if the base is a field ref without its own name,
                        // meaning it could be a bare `$` dereferencing into relatedCase.
                        if matches!(inner.as_ref(), Expr::FieldRef { name: None, .. }) {
                            diagnostics.push(Diagnostic::error(
                                rule_id,
                                path,
                                format!(
                                    "guard accesses '.{first}' on bare '$'; this may reference \
                                     related case state"
                                ),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        false // continue walking
    });
}

/// Return true if an identifier looks like a related-case accessor.
///
/// Matches `relatedCase` and common capitalisation variants. The spec (Kernel S5.5)
/// calls this concept "related case state". We match the canonical camelCase and
/// a few predictable alias patterns.
fn is_related_case_name(name: &str) -> bool {
    name == "relatedCase"
        || name == "relatedCases"
        || name.starts_with("relatedCase.")
        || name.starts_with("relatedCases.")
}

/// K-019: Check that every function call in the expression is a known built-in.
///
/// Extension functions are permitted by K-019 ("built-in and extension functions");
/// this check flags anything not in the built-in catalog. At Tier 2 we have no
/// extension registry to consult, so we emit a warning (not an error) for unknown
/// names to avoid false positives against valid registered extensions.
fn check_only_builtin_functions(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let builtin_names: HashSet<&str> = builtin_function_catalog().iter().map(|e| e.name).collect();

    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, .. } = e {
            if !builtin_names.contains(name.as_str()) {
                diagnostics.push(Diagnostic::warning(
                    rule_id,
                    path,
                    format!(
                        "function '{name}' is not in the built-in catalog; if it is an extension \
                         function it must be declared in the extension registry"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-011: Detect recursive `let` bindings.
///
/// A `let x = ... in body` is recursive if `x` is referenced anywhere inside
/// its own value expression. We track the binding name being defined and scan
/// the value sub-tree for any `FieldRef` or `FunctionCall` with the same name.
fn check_no_recursive_let(
    expr: &Expr,
    outer_names: &mut HashSet<String>,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::LetBinding { name, value, body } => {
            // Check whether the value expression references the name being bound
            // (direct self-recursion) or any name currently being defined in an
            // enclosing let (mutual recursion through shadowing — not actually
            // possible in FEL's single-binding let, but we check for completeness).
            let mut self_set = outer_names.clone();
            self_set.insert(name.clone());

            if let_value_references_name(value, &self_set) {
                diagnostics.push(Diagnostic::error(
                    rule_id,
                    path,
                    format!("let binding '{name}' references itself recursively"),
                ));
            }

            // Add this name to the outer scope and recurse into body.
            outer_names.insert(name.clone());
            check_no_recursive_let(body, outer_names, rule_id, path, diagnostics);
            outer_names.remove(name);
        }
        // For any other expression shape, recurse into children.
        _ => {
            visit_children(expr, &mut |child| {
                check_no_recursive_let(child, outer_names, rule_id, path, diagnostics);
            });
        }
    }
}

/// Return true if `expr` contains a `FieldRef` whose name is in `names`.
fn let_value_references_name(expr: &Expr, names: &HashSet<String>) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if let Expr::FieldRef { name: Some(n), .. } = e {
            if names.contains(n) {
                found = true;
                return true; // short-circuit
            }
        }
        false
    });
    found
}

/// AG-012: Warn when `every` or `some` function calls appear (partial check).
///
/// FEL does not have dedicated quantifier syntax; if `every` or `some` are used
/// as function calls they act as quantifiers. The SMT subset requires that
/// quantified variables range over finite, statically-known domains. At Tier 2
/// we cannot verify domain finiteness, so we flag their presence as a warning
/// indicating manual review is required.
fn check_finite_quantifiers(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, .. } = e {
            // `every` and `some` are standard quantifier names in FEL contexts.
            // They are not in the built-in catalog, so any occurrence is either
            // an extension or an incorrect usage — both warrant review.
            if name == "every" || name == "some" {
                diagnostics.push(Diagnostic::warning(
                    rule_id,
                    path,
                    format!(
                        "quantifier '{name}()' detected; the SMT subset requires quantifiers to \
                         range over finite, statically-known domains — verify this manually"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-013: Detect non-linear arithmetic (variable × variable or variable ÷ variable).
///
/// A multiplication or division is non-linear if both operands contain at
/// least one variable reference (`FieldRef` or `ContextRef`). One constant
/// side is allowed (e.g. `$qty * 2`).
fn check_linear_arithmetic(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_expr(expr, &mut |e| {
        if let Expr::BinaryOp { op, left, right } = e {
            if matches!(op, BinaryOp::Mul | BinaryOp::Div) {
                let left_has_var = contains_variable(left);
                let right_has_var = contains_variable(right);

                if left_has_var && right_has_var {
                    let op_symbol = if *op == BinaryOp::Mul { "*" } else { "/" };
                    diagnostics.push(Diagnostic::error(
                        rule_id,
                        path,
                        format!(
                            "non-linear arithmetic: '{op_symbol}' has variable references on both \
                             sides; the SMT subset requires linear arithmetic"
                        ),
                    ));
                }
            }
        }
        false
    });
}

/// Return true if `expr` contains any `FieldRef` or `ContextRef` node.
fn contains_variable(expr: &Expr) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if matches!(e, Expr::FieldRef { .. } | Expr::ContextRef { .. }) {
            found = true;
            return true; // short-circuit
        }
        false
    });
    found
}

/// AG-014: Extension function calls are forbidden in the SMT verifiable subset.
///
/// Unlike K-019 (which only warns), AG-014 is a hard error: the SMT prover
/// cannot reason about extension semantics, so their presence makes a
/// constraint unverifiable.
fn check_no_extension_functions(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let builtin_names: HashSet<&str> = builtin_function_catalog().iter().map(|e| e.name).collect();

    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, .. } = e {
            if !builtin_names.contains(name.as_str()) {
                diagnostics.push(Diagnostic::error(
                    rule_id,
                    path,
                    format!(
                        "extension function '{name}' is not permitted in the SMT verifiable \
                         subset; only Core S3.5 built-ins may appear in verifiable constraints"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-010 (finite enumerations): warn on simple variable-to-variable `==` / `!=`.
///
/// Passes silently when either side is a literal (including comparisons such as
/// `$instance.impactLevel == "rights-impacting"`) or when either side's dotted path is
/// listed in `finiteDomainDeclarations`.
///
/// When both operands resolve to dotted paths, at most one warning is emitted per
/// unordered path pair (avoids duplicate diagnostics for the same comparison shape).
fn check_finite_domain_equality(
    expr: &Expr,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
    finite_paths: &HashMap<String, ()>,
) {
    let mut warned_path_pairs: HashSet<(String, String)> = HashSet::new();
    walk_expr(expr, &mut |e| {
        if let Expr::BinaryOp {
            op: BinaryOp::Eq | BinaryOp::NotEq,
            left,
            right,
        } = e
        {
            if smt_equality_is_decidable(left, right, finite_paths) {
                return false;
            }
            if is_simple_access_expr(left.as_ref()) && is_simple_access_expr(right.as_ref()) {
                let skip_duplicate = match (
                    simple_access_path_string(left.as_ref()),
                    simple_access_path_string(right.as_ref()),
                ) {
                    (Some(a), Some(b)) => {
                        let pair = if a <= b { (a, b) } else { (b, a) };
                        !warned_path_pairs.insert(pair)
                    }
                    _ => false,
                };
                if skip_duplicate {
                    return false;
                }
                diagnostics.push(Diagnostic::warning(
                    "AG-010",
                    path,
                    "`==` or `!=` compares two non-literal field or context accesses; use a \
                     literal on one side, add `finiteDomainDeclarations` for a path, or avoid \
                     variable-to-variable comparison (AdvGov S8.2)"
                        .to_string(),
                ));
            }
        }
        false
    });
}

/// True when AdvGov S8.2 finite-domain reasoning is obvious from the AST.
fn smt_equality_is_decidable(
    left: &Expr,
    right: &Expr,
    finite_paths: &HashMap<String, ()>,
) -> bool {
    if is_literal_expr(left) || is_literal_expr(right) {
        return true;
    }
    path_declared_finite(left, finite_paths) || path_declared_finite(right, finite_paths)
}

fn path_declared_finite(expr: &Expr, finite_paths: &HashMap<String, ()>) -> bool {
    simple_access_path_string(expr).is_some_and(|p| finite_paths.contains_key(&p))
}

/// Scalar or aggregate of literals only (no `$` / `@`).
fn is_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Null
        | Expr::Boolean(_)
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::DateLiteral(_)
        | Expr::DateTimeLiteral(_) => true,
        Expr::Array(elements) => elements.iter().all(is_literal_expr),
        Expr::Object(pairs) => pairs.iter().all(|(_, v)| is_literal_expr(v)),
        _ => false,
    }
}

fn is_simple_access_expr(expr: &Expr) -> bool {
    match expr {
        Expr::FieldRef { .. } | Expr::ContextRef { .. } => true,
        Expr::PostfixAccess { expr: inner, .. } => is_simple_access_expr(inner.as_ref()),
        _ => false,
    }
}

/// Dotted path for a simple field or context access (`$a.b` → `a.b`). Indices/wildcards excluded.
fn simple_access_path_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::FieldRef { name, path: segments } => {
            let root = name.as_deref()?;
            let mut s = root.to_string();
            for seg in segments {
                let PathSegment::Dot(part) = seg else {
                    return None;
                };
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        Expr::ContextRef { name, tail, .. } => {
            let mut s = name.clone();
            for part in tail {
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        Expr::PostfixAccess {
            expr: inner,
            path: segments,
        } => {
            let mut s = simple_access_path_string(inner.as_ref())?;
            for seg in segments {
                let PathSegment::Dot(part) = seg else {
                    return None;
                };
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        _ => None,
    }
}

/// AI-024: Return true if `expr` contains any `@agent` context reference.
fn references_agent_context(expr: &Expr) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if let Expr::ContextRef { name, .. } = e {
            if name == "agent" {
                found = true;
                return true; // short-circuit
            }
        }
        false
    });
    found
}

// ---------------------------------------------------------------------------
// Generic AST traversal
// ---------------------------------------------------------------------------

/// Walk `expr` depth-first, calling `visitor` on every node.
///
/// If `visitor` returns `true` the traversal of the current subtree is
/// short-circuited (useful for early-exit searches). The visitor is
/// called in pre-order: the parent node is visited before its children.
///
/// Children are iterated inline via `visit_children` to avoid allocating
/// a `Vec` per node (Finding #2).
fn walk_expr(expr: &Expr, visitor: &mut impl FnMut(&Expr) -> bool) {
    if visitor(expr) {
        return;
    }
    visit_children(expr, &mut |child| walk_expr(child, visitor));
}

/// Call `f` once for each direct child expression of `expr`.
///
/// Inlines child iteration without allocating a `Vec`, replacing the
/// previous `children_of` helper (Finding #2).
fn visit_children(expr: &Expr, f: &mut impl FnMut(&Expr)) {
    match expr {
        // Leaves — no children.
        Expr::Null
        | Expr::Boolean(_)
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::DateLiteral(_)
        | Expr::DateTimeLiteral(_)
        | Expr::FieldRef { .. }
        | Expr::ContextRef { .. } => {}

        Expr::Array(elements) => {
            for e in elements {
                f(e);
            }
        }

        Expr::Object(pairs) => {
            for (_, v) in pairs {
                f(v);
            }
        }

        Expr::UnaryOp { operand, .. } => f(operand.as_ref()),

        Expr::BinaryOp { left, right, .. } => {
            f(left.as_ref());
            f(right.as_ref());
        }

        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        }
        | Expr::IfThenElse {
            condition,
            then_branch,
            else_branch,
        } => {
            f(condition.as_ref());
            f(then_branch.as_ref());
            f(else_branch.as_ref());
        }

        Expr::Membership {
            value, container, ..
        } => {
            f(value.as_ref());
            f(container.as_ref());
        }

        Expr::NullCoalesce { left, right } => {
            f(left.as_ref());
            f(right.as_ref());
        }

        Expr::LetBinding { value, body, .. } => {
            f(value.as_ref());
            f(body.as_ref());
        }

        Expr::FunctionCall { args, .. } => {
            for arg in args {
                f(arg);
            }
        }

        Expr::PostfixAccess { expr: inner, .. } => f(inner.as_ref()),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_docs_in_private_items)]

    use std::collections::HashMap;

    use super::*;
    use crate::diagnostic::Severity;
    use crate::document::{DocumentKind, WosDocument};
    use serde_json::json;

    fn make_doc(kind: DocumentKind, value: serde_json::Value) -> WosDocument {
        WosDocument {
            kind,
            value,
            source: None,
        }
    }

    // --- K-012: guard syntax ---

    #[test]
    fn k012_valid_guard_is_clean() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "draft": {
                            "transitions": [{"event": "submit", "target": "review", "guard": "$amount > 0"}]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(diag.is_empty(), "unexpected: {diag:?}");
    }

    #[test]
    fn k012_invalid_guard_emits_error() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "draft": {
                            "transitions": [{"event": "submit", "target": "review", "guard": ">>> broken <<<"}]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-012" && d.severity == Severity::Error),
            "expected K-012 error, got: {diag:?}"
        );
    }

    // --- K-013: milestone condition syntax ---

    #[test]
    fn k013_invalid_milestone_condition() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "milestones": [{"id": "m1", "condition": "((( invalid"}]
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "K-013"),
            "expected K-013 error, got: {diag:?}"
        );
    }

    // --- K-017: no related-case refs ---

    #[test]
    fn k017_guard_with_related_case_ref() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "close",
                                "target": "closed",
                                "guard": "$relatedCase.status = 'done'"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-017" && d.severity == Severity::Error),
            "expected K-017 error, got: {diag:?}"
        );
    }

    #[test]
    fn k017_guard_without_related_case_ref_is_clean() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "close",
                                "target": "closed",
                                "guard": "$status = 'done'"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "K-017"),
            "unexpected K-017: {diag:?}"
        );
    }

    // --- K-019: only built-in functions ---

    #[test]
    fn k019_unknown_function_emits_warning() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "go",
                                "target": "done",
                                "guard": "myCustomFn($amount) > 0"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-019" && d.severity == Severity::Warning),
            "expected K-019 warning, got: {diag:?}"
        );
    }

    #[test]
    fn k019_known_builtin_is_clean() {
        let doc = make_doc(
            DocumentKind::Kernel,
            json!({
                "$wosKernel": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "go",
                                "target": "done",
                                "guard": "sum($items[*].amount) > 100"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "K-019"),
            "unexpected K-019: {diag:?}"
        );
    }

    // --- G-042: assertion library expression syntax ---

    #[test]
    fn g042_invalid_assertion_expression() {
        let doc = make_doc(
            DocumentKind::AssertionLibrary,
            json!({
                "$wosAssertionLibrary": true,
                "assertions": [{"id": "a1", "expression": "not ( valid"}]
            }),
        );
        let mut diag = Vec::new();
        check_assertion_library_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "G-042"),
            "expected G-042 error, got: {diag:?}"
        );
    }

    // --- G-043: delegation condition syntax ---

    #[test]
    fn g043_invalid_delegation_condition() {
        let doc = make_doc(
            DocumentKind::WorkflowGovernance,
            json!({
                "$wosWorkflowGovernance": true,
                "delegations": [{"delegator": "alice", "conditions": ["$x >"]}]
            }),
        );
        let mut diag = Vec::new();
        check_governance_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "G-043"),
            "expected G-043 error, got: {diag:?}"
        );
    }

    #[test]
    fn g043_valid_delegation_condition_is_clean() {
        let doc = make_doc(
            DocumentKind::WorkflowGovernance,
            json!({
                "$wosWorkflowGovernance": true,
                "delegations": [{"delegator": "alice", "conditions": ["$level > 2"]}]
            }),
        );
        let mut diag = Vec::new();
        check_governance_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "G-043"),
            "unexpected G-043: {diag:?}"
        );
    }

    // --- AI-024: escalation condition references @agent ---

    #[test]
    fn ai024_condition_without_agent_ref_warns() {
        let doc = make_doc(
            DocumentKind::AiIntegration,
            json!({
                "$wosAIIntegration": true,
                "agents": {
                    "classifier": {
                        "escalation": {
                            "conditions": ["$score > 0.9"]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-024" && d.severity == Severity::Warning),
            "expected AI-024 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ai024_condition_with_agent_ref_is_clean() {
        let doc = make_doc(
            DocumentKind::AiIntegration,
            json!({
                "$wosAIIntegration": true,
                "agents": {
                    "classifier": {
                        "escalation": {
                            "conditions": ["@agent.confidence < 0.7"]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-024"),
            "unexpected AI-024: {diag:?}"
        );
    }

    // --- AG-011: recursive let binding ---

    #[test]
    fn ag011_self_recursive_let() {
        let expr_str = "let x = x + 1 in x";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            diag.iter().any(|d| d.rule_id == "AG-011"),
            "expected AG-011 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag011_non_recursive_let_is_clean() {
        let expr_str = "let x = $amount * 2 in x > 100";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-011"),
            "unexpected AG-011: {diag:?}"
        );
    }

    // --- AG-013: linear arithmetic ---

    #[test]
    fn ag013_variable_times_variable() {
        let expr_str = "$qty * $price > 0";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-013" && d.severity == Severity::Error),
            "expected AG-013 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag013_variable_times_literal_is_linear() {
        let expr_str = "$qty * 2 > 0";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-013"),
            "unexpected AG-013: {diag:?}"
        );
    }

    // --- AG-014: no extension functions in SMT subset ---

    #[test]
    fn ag014_extension_function_in_verifiable_constraint() {
        let expr_str = "myExtFn($value) > 0";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-014" && d.severity == Severity::Error),
            "expected AG-014 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag014_builtin_function_is_allowed() {
        let expr_str = "abs($delta) < 5";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-014"),
            "unexpected AG-014: {diag:?}"
        );
    }

    // --- AG-010: finite-domain equality (variable-to-variable) ---

    #[test]
    fn ag010_literal_comparison_is_clean() {
        let expr_str = r#"$output.status == "approved""#;
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_boolean_comparison_is_clean() {
        let expr_str = "$output.eligible == true";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_membership_literal_array_is_clean() {
        let expr_str = r#"$tier in ["gold", "silver", "bronze"]"#;
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_known_enum_to_literal_is_clean() {
        let expr_str = r#"$instance.impactLevel == "rights-impacting""#;
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_variable_to_variable_equality_warns() {
        let expr_str = "$output.status == $copy.status";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "expected AG-010 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_variable_to_variable_inequality_warns() {
        let expr_str = "$output.status != $copy.status";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "expected AG-010 warning for !=, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_duplicate_path_pair_emits_single_warning() {
        let expr_str =
            "($output.status == $copy.status) and ($copy.status == $output.status)";
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &HashMap::new());
        let n = diag
            .iter()
            .filter(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning)
            .count();
        assert_eq!(n, 1, "expected one deduped AG-010 warning, got: {diag:?}");
    }

    #[test]
    fn ag010_finite_domain_declaration_suppresses_var_var() {
        let expr_str = "$output.status == $copy.status";
        let mut decls = HashMap::new();
        decls.insert("output.status".to_string(), ());
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &decls);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_invalid_declaration_entry_does_not_suppress() {
        let expr_str = "$output.status == $copy.status";
        let mut decls = HashMap::new();
        decls.insert("other.path".to_string(), ());
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &decls);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "expected AG-010 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_advanced_doc_parses_finite_domain_declarations() {
        let doc = make_doc(
            DocumentKind::Advanced,
            json!({
                "$wosAdvancedGovernance": true,
                "verifiableConstraints": [{
                    "constraintRef": "c1",
                    "verifiable": true,
                    "expression": "$output.status == $copy.status",
                    "finiteDomainDeclarations": {
                        "output.status": { "domain": ["a", "b"] },
                        "bad": { "domain": [] },
                        "alsoBad": "not-an-object"
                    }
                }]
            }),
        );
        let mut diag = Vec::new();
        check_advanced_governance_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-010" && d.severity == Severity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    /// JSONPath-style `[?(...)]` is not FEL; restriction 6 is enforced by the parser.
    #[test]
    fn ag010_filter_bracket_syntax_does_not_parse() {
        assert!(
            parse("$items[?(@.x > 1)]").is_err(),
            "JSONPath filter expressions must not parse as FEL"
        );
    }
}
