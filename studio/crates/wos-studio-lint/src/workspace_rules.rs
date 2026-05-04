// Rust guideline compliant 2026-05-02

//! Workspace-tier readiness rules — cross-document checks.
//!
//! This module contains every rule whose evaluation needs more than one
//! document: supersession-cycle detection, mapping coverage of approved
//! PolicyObjects, Outcome→Notice→AppealRight chains, scenario coverage
//! per outcome, etc. Doc-local rules live in their per-tier modules.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::workspace::Workspace;
use crate::{LintDiagnostic, LintSeverity, Tier};

fn studio_diagnostic(
    rule_id: &'static str,
    severity: LintSeverity,
    path: impl Into<String>,
    message: impl Into<String>,
) -> LintDiagnostic {
    LintDiagnostic {
        rule_id,
        severity,
        tier: Tier::T1,
        path: path.into(),
        message: message.into(),
        suggested_fix: None,
        related_docs: Vec::new(),
        source: None,
    }
}

/// Run every workspace-tier rule and append diagnostics. Sorts the
/// resulting stream so callers see deterministic output.
pub fn lint_workspace(ws: &Workspace) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    pom_lint_007(ws, &mut diagnostics);
    pom_lint_008(ws, &mut diagnostics);
    pom_lint_020(ws, &mut diagnostics);
    pom_lint_033(ws, &mut diagnostics);
    pom_lint_040(ws, &mut diagnostics);
    pom_lint_051(ws, &mut diagnostics);
    sv_lint_007(ws, &mut diagnostics);
    bind_lint_001(ws, &mut diagnostics);
    bind_lint_002(ws, &mut diagnostics);
    bind_lint_003(ws, &mut diagnostics);
    bind_lint_004(ws, &mut diagnostics);
    bind_lint_005(ws, &mut diagnostics);
    bind_lint_006(ws, &mut diagnostics);
    bind_lint_010(ws, &mut diagnostics);
    bind_lint_011(ws, &mut diagnostics);
    bind_lint_012(ws, &mut diagnostics);
    bind_lint_020(ws, &mut diagnostics);
    bind_lint_021(ws, &mut diagnostics);
    bind_lint_070(ws, &mut diagnostics);
    bind_lint_071(ws, &mut diagnostics);
    bind_lint_072(ws, &mut diagnostics);
    wf_lint_009(ws, &mut diagnostics);
    wf_lint_010(ws, &mut diagnostics);
    wf_lint_011(ws, &mut diagnostics);
    wf_lint_012(ws, &mut diagnostics);
    wf_lint_013(ws, &mut diagnostics);
    map_lint_009(ws, &mut diagnostics);
    map_lint_010(ws, &mut diagnostics);
    map_lint_011(ws, &mut diagnostics);
    ra_lint_001(ws, &mut diagnostics);
    ra_lint_002(ws, &mut diagnostics);
    prov_lint_005(ws, &mut diagnostics);
    prov_lint_006(ws, &mut diagnostics);
    prov_lint_007(ws, &mut diagnostics);
    map_lint_001(ws, &mut diagnostics);
    map_lint_005(ws, &mut diagnostics);
    map_lint_006(ws, &mut diagnostics);
    map_lint_007(ws, &mut diagnostics);
    eff_lint_002(ws, &mut diagnostics);
    eff_lint_004(ws, &mut diagnostics);
    prov_lint_003(ws, &mut diagnostics);
    sv_lint_003(ws, &mut diagnostics);
    wf_lint_001(ws, &mut diagnostics);
    wf_lint_002(ws, &mut diagnostics);
    wf_lint_004(ws, &mut diagnostics);
    wf_lint_005(ws, &mut diagnostics);
    wf_lint_006(ws, &mut diagnostics);
    sc_lint_001_workspace(ws, &mut diagnostics);
    sc_lint_002_workspace(ws, &mut diagnostics);
    sc_lint_005(ws, &mut diagnostics);
    eq_lint_003(ws, &mut diagnostics);
    acc_lint_001(ws, &mut diagnostics);
    jur_lint_001(ws, &mut diagnostics);
    pub_lint_001(ws, &mut diagnostics);
    pub_lint_002(ws, &mut diagnostics);
    pub_lint_005(ws, &mut diagnostics);
    pub_lint_006(ws, &mut diagnostics);
    eff_lint_005(ws, &mut diagnostics);
    ai_lint_003(ws, &mut diagnostics);
    cmp_lint_010(ws, &mut diagnostics);
    cmp_lint_011(ws, &mut diagnostics);
    id_lint_001(ws, &mut diagnostics);
    id_lint_002(ws, &mut diagnostics);
    comp_lint_001(ws, &mut diagnostics);
    comp_lint_002(ws, &mut diagnostics);
    chain_lint_002(ws, &mut diagnostics);
    term_lint_001(ws, &mut diagnostics);
    diagnostics.sort_by(|a, b| a.path.cmp(&b.path).then(a.rule_id.cmp(&b.rule_id)));
    diagnostics
}

// ============================================================================
// S1 / S2
// ============================================================================

/// `POM-LINT-007` — no circular Supersession.
///
/// Implementation note: proper three-color DFS (white/gray/black) where
/// `gray` ("on the current stack") means a back-edge to it is a cycle,
/// and `black` ("fully explored, no cycle below") prunes future visits.
/// The previous impl shared one `visited` set across DFS-from-each-start
/// AND included the gray prefix in the reported path — both bugs fixed
/// here. The reported cycle is sliced from the gray-back-edge target
/// onward so the message reads `B → C → B`, not `A → B → C → B`.
fn pom_lint_007(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Build a `supersedes` graph from PolicyObject records. BTreeMap
    // for stable iteration order across runs.
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        let supersedes = record
            .get("supersedes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !supersedes.is_empty() {
            graph.insert(id.to_string(), supersedes);
        }
    }

    let mut color: HashMap<String, Color> = HashMap::new();
    for start in graph.keys() {
        if matches!(color.get(start), Some(Color::Black)) {
            continue;
        }
        let mut stack: Vec<String> = Vec::new();
        if let Some(cycle) = dfs_visit(start, &graph, &mut color, &mut stack) {
            diagnostics.push(studio_diagnostic(
                "POM-LINT-007",
                LintSeverity::Error,
                format!("/policyObjects/{start}"),
                format!(
                    "Circular Supersession detected: {}",
                    cycle.join(" → ")
                ),
            ));
            break;
        }
    }
}

/// Three-color DFS state per node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    Gray,  // on the current DFS stack
    Black, // fully explored, no cycle reachable
}

/// Visit `node` in three-color DFS. Returns `Some(cycle)` (sliced from
/// the back-edge target onward) if a cycle is detected.
fn dfs_visit(
    node: &str,
    graph: &BTreeMap<String, Vec<String>>,
    color: &mut HashMap<String, Color>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    if matches!(color.get(node), Some(Color::Black)) {
        return None;
    }
    if matches!(color.get(node), Some(Color::Gray)) {
        // Back-edge → cycle. Slice the stack from the first occurrence
        // of `node` onward; append `node` again to make the cycle path
        // explicit (`B → C → B` rather than `B → C`).
        let start = stack.iter().position(|n| n == node).unwrap_or(0);
        let mut cycle: Vec<String> = stack[start..].to_vec();
        cycle.push(node.to_string());
        return Some(cycle);
    }
    color.insert(node.to_string(), Color::Gray);
    stack.push(node.to_string());
    if let Some(neighbours) = graph.get(node) {
        for n in neighbours {
            if let Some(cycle) = dfs_visit(n, graph, color, stack) {
                return Some(cycle);
            }
        }
    }
    stack.pop();
    color.insert(node.to_string(), Color::Black);
    None
}

/// `POM-LINT-008` — every Conflict PolicyObject MUST be resolved or
/// waived before downstream advance.
fn pom_lint_008(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("Conflict") {
            continue;
        }
        let resolution = record.get("resolutionState").and_then(Value::as_str);
        if !matches!(resolution, Some("resolved" | "waived")) {
            let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "POM-LINT-008",
                LintSeverity::Error,
                format!("/policyObjects/{id}/resolutionState"),
                "Conflict PolicyObject MUST be resolved or waived before \
                 downstream advance."
                    .to_string(),
            ));
        }
    }
}

/// `POM-LINT-020` — every PolicyObject `advanced past approved`
/// (mapped / validated / published / superseded / deprecated /
/// demoted) MUST be covered by an ApprovalDecision with matching
/// `subjectRef` (per SA-MUST-pom-020 + CM §1.15). The `approved`
/// state itself is the gate being crossed; states downstream of
/// `approved` require the recorded decision.
fn pom_lint_020(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // States that mean "past approved" per the
    // PolicyObjectLifecycleState enum in
    // wos-studio-model::common::PolicyObjectLifecycleState.
    fn requires_approval(state: &str) -> bool {
        matches!(
            state,
            "mapped"
                | "validated"
                | "published"
                | "superseded"
                | "deprecated"
                | "demoted"
        )
    }

    // Index ApprovalDecisions by subjectRef (PolicyObject id).
    let mut approved_subjects: BTreeSet<String> = BTreeSet::new();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Approval) {
            continue;
        }
        // ApprovalDecision documents come in two shapes:
        // (a) `kind = ApprovalDecision` with body.subjectRef
        // (b) `decision.subjectRef` on the wrapper
        let body = doc.document.body();
        let kind = body.get("kind").and_then(Value::as_str);
        if kind != Some("ApprovalDecision") {
            continue;
        }
        if let Some(decision) = body.get("decision").and_then(Value::as_object) {
            if let Some(s) = decision.get("subjectRef").and_then(Value::as_str) {
                approved_subjects.insert(s.to_string());
            }
        } else if let Some(s) = body.get("subjectRef").and_then(Value::as_str) {
            approved_subjects.insert(s.to_string());
        }
    }

    for (_doc, record) in ws.policy_object_records() {
        let Some(state) = record.get("lifecycleState").and_then(Value::as_str) else {
            continue;
        };
        if !requires_approval(state) {
            continue;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
        if !approved_subjects.contains(id) {
            diagnostics.push(studio_diagnostic(
                "POM-LINT-020",
                LintSeverity::Error,
                format!("/policyObjects/{id}/lifecycleState"),
                format!(
                    "PolicyObject '{id}' is at lifecycleState='{state}' \
                     but no ApprovalDecision in this workspace references \
                     it as subject (per SA-MUST-pom-020)."
                ),
            ));
        }
    }
}

/// `POM-LINT-033` — an `AppealRight`'s linked Outcome MUST equal the
/// Outcome of the NoticeRequirement it links to (per SA-MUST-pom-033).
/// Waiver path: a ReviewerResolution flag on the AppealRight (`waivedAt`
/// + `waiverScope = "separate-procedure"`) silences the rule.
fn pom_lint_033(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index NoticeRequirement bodies' outcome (trigger.outcomeRef OR
    // body.outcomeRef).
    let mut notice_outcome: BTreeMap<String, String> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("NoticeRequirement") {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        let body = record.get("body").and_then(Value::as_object);
        let outcome = body
            .and_then(|b| b.get("outcomeRef").and_then(Value::as_str))
            .or_else(|| {
                body.and_then(|b| b.get("trigger"))
                    .and_then(Value::as_object)
                    .and_then(|t| t.get("outcomeRef").and_then(Value::as_str))
            });
        if let Some(o) = outcome {
            notice_outcome.insert(id.to_string(), o.to_string());
        }
    }

    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("AppealRight") {
            continue;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
        let body = record.get("body").and_then(Value::as_object);
        // Waiver short-circuit (per spec wording).
        let waived = body
            .map(|b| {
                b.get("waivedAt").is_some()
                    && b.get("waiverScope").and_then(Value::as_str)
                        == Some("separate-procedure")
            })
            .unwrap_or(false);
        if waived {
            continue;
        }
        let linked_notice = body
            .and_then(|b| b.get("linkedNoticeRef").and_then(Value::as_str))
            .map(str::to_string);
        let appeal_outcome = body
            .and_then(|b| b.get("outcomeRef").and_then(Value::as_str))
            .map(str::to_string);
        let Some(notice_id) = linked_notice else {
            // No link → not in scope for pom-033 (other rules cover
            // dangling-reference cases).
            continue;
        };
        let Some(notice_outcome_id) = notice_outcome.get(&notice_id) else {
            // Notice has no outcome to compare against → not in scope.
            continue;
        };
        // Implicit-inheritance allowance: when AppealRight has no
        // explicit outcomeRef, treat it as inheriting the linked
        // Notice's outcome (the snap-shorthand authoring pattern).
        // The rule fires only on explicit mismatch.
        let Some(appeal_outcome_id) = appeal_outcome else {
            continue;
        };
        if appeal_outcome_id != *notice_outcome_id {
            diagnostics.push(studio_diagnostic(
                "POM-LINT-033",
                LintSeverity::Error,
                format!("/policyObjects/{id}/body/outcomeRef"),
                format!(
                    "AppealRight '{id}' links Notice '{notice_id}' but its \
                     outcomeRef '{appeal_outcome_id}' does not match the \
                     Notice's outcomeRef '{notice_outcome_id}' (per \
                     SA-MUST-pom-033). Either align the outcomeRef or set \
                     body.waiverScope='separate-procedure' with waivedAt."
                ),
            ));
        }
    }
}

/// `POM-LINT-040` — two approved Deadlines on the same `body.trigger`
/// with different `body.calendarDaysFromTrigger` MUST surface as a
/// Conflict candidate (per SA-MUST-pom-040). The full algorithm for
/// arbitrary contradictions is runtime-pending; the Deadline subset
/// is the tractable lint-time slice.
fn pom_lint_040(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // (trigger, days) → first PolicyObject id seen with that trigger.
    let mut by_trigger: BTreeMap<String, (String, i64)> = BTreeMap::new();
    let mut conflicts: Vec<(String, String, String)> = Vec::new();
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("Deadline") {
            continue;
        }
        // Only approved (and downstream) participate.
        let approved = record
            .get("lifecycleState")
            .and_then(Value::as_str)
            .is_some_and(|s| matches!(s, "approved" | "superseded"));
        if !approved {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        let body = record.get("body").and_then(Value::as_object);
        let trigger = body.and_then(|b| b.get("trigger").and_then(Value::as_str));
        let days = body
            .and_then(|b| b.get("calendarDaysFromTrigger"))
            .and_then(Value::as_i64);
        let (Some(t), Some(d)) = (trigger, days) else {
            continue;
        };
        match by_trigger.get(t) {
            Some((other_id, other_d)) if *other_d != d => {
                conflicts.push((t.to_string(), other_id.clone(), id.to_string()));
            }
            None => {
                by_trigger.insert(t.to_string(), (id.to_string(), d));
            }
            _ => {}
        }
    }
    // Check whether each conflict is already accounted for by a
    // Conflict PolicyObject that names both subjects.
    let conflict_subjects: Vec<BTreeSet<String>> = ws
        .policy_object_records()
        .into_iter()
        .filter_map(|(_d, r)| {
            if r.get("kind").and_then(Value::as_str) != Some("Conflict") {
                return None;
            }
            r.get("subjects")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<BTreeSet<String>>()
                })
        })
        .collect();

    for (trigger, a, b) in conflicts {
        let pair: BTreeSet<String> =
            [a.clone(), b.clone()].into_iter().collect();
        let already_filed = conflict_subjects
            .iter()
            .any(|s| pair.iter().all(|id| s.contains(id)));
        if !already_filed {
            diagnostics.push(studio_diagnostic(
                "POM-LINT-040",
                LintSeverity::Error,
                format!("/policyObjects/{a}"),
                format!(
                    "Approved Deadlines '{a}' and '{b}' both fire on \
                     trigger '{trigger}' with different durations and no \
                     Conflict PolicyObject filed naming both as subjects \
                     (per SA-MUST-pom-040)."
                ),
            ));
        }
    }
}

/// `POM-LINT-051` — two deontic constraints sharing
/// `(body.subject, body.action)` MUST be flagged as composition
/// candidates unless reviewer attestation
/// (`body.compositionAttestation = "reviewed"`) is recorded on at
/// least one (per SA-MUST-pom-051; tier-S2 finding).
///
/// Effectiveness intersection is not modeled at lint time; the rule
/// errs on the side of surfacing potential overlaps and lets the
/// reviewer either align Effectiveness scopes (silencing the rule
/// post-hoc by attestation) or merge the constraints.
fn pom_lint_051(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let deontic_kinds = ["Permission", "Prohibition", "Obligation"];
    // (subject, action) → Vec<(id, attested)>
    let mut groups: BTreeMap<(String, String), Vec<(String, bool)>> =
        BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        let Some(kind) = record.get("kind").and_then(Value::as_str) else {
            continue;
        };
        if !deontic_kinds.contains(&kind) {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else {
            continue;
        };
        let body = record.get("body").and_then(Value::as_object);
        let (subject, action) = match (
            body.and_then(|b| b.get("subject").and_then(Value::as_str)),
            body.and_then(|b| b.get("action").and_then(Value::as_str)),
        ) {
            (Some(s), Some(a)) => (s, a),
            _ => continue,
        };
        let attested = body
            .and_then(|b| b.get("compositionAttestation").and_then(Value::as_str))
            == Some("reviewed");
        groups
            .entry((subject.to_string(), action.to_string()))
            .or_default()
            .push((id.to_string(), attested));
    }
    for ((subject, action), members) in &groups {
        if members.len() < 2 {
            continue;
        }
        let any_attested = members.iter().any(|(_, a)| *a);
        if any_attested {
            continue;
        }
        let ids: Vec<&str> = members.iter().map(|(i, _)| i.as_str()).collect();
        // Use the first id as the diagnostic anchor (deterministic since
        // groups is BTreeMap and members preserves insertion order).
        let anchor = ids[0];
        diagnostics.push(studio_diagnostic(
            "POM-LINT-051",
            LintSeverity::Warning,
            format!("/policyObjects/{anchor}/body"),
            format!(
                "Deontic constraints {} share (subject='{subject}', \
                 action='{action}') with no compositionAttestation \
                 recorded — flagged as composition candidates per \
                 SA-MUST-pom-051. Either record \
                 body.compositionAttestation='reviewed' on one, or \
                 split the constraints by Effectiveness scope.",
                ids.join(", ")
            ),
        ));
    }
}

/// `SV-LINT-007` — every SourceCitation / ExtractedClaim that targets
/// a SourceDocument MUST resolve to a SourceVersion within that
/// document; SourceDocuments without any SourceVersion are
/// versionless and MUST NOT be cited (`SA-MUST-source-001`).
fn sv_lint_007(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index documents that DO have ≥ 1 SourceVersion.
    let mut documents_with_versions: BTreeSet<String> = BTreeSet::new();
    let mut all_document_ids: BTreeSet<String> = BTreeSet::new();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Source) {
            continue;
        }
        if let Some(id) = doc.id() {
            all_document_ids.insert(id.to_string());
            if doc
                .source_versions()
                .is_some_and(|v| !v.is_empty())
            {
                documents_with_versions.insert(id.to_string());
            }
        }
    }
    let versionless: BTreeSet<&String> = all_document_ids
        .difference(&documents_with_versions)
        .collect();
    if versionless.is_empty() {
        return;
    }
    // Walk PolicyObject citations; flag any sourceDocumentRef that
    // points at a versionless SourceDocument.
    for (_doc, record) in ws.policy_object_records() {
        let Some(citations) = record.get("citations").and_then(Value::as_array) else {
            continue;
        };
        for (i, citation) in citations.iter().enumerate() {
            let target = citation
                .get("sourceDocumentRef")
                .and_then(Value::as_str)
                .or_else(|| citation.get("sourceDocument").and_then(Value::as_str));
            let Some(target) = target else { continue };
            if versionless.contains(&target.to_string()) {
                let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
                diagnostics.push(studio_diagnostic(
                    "SV-LINT-007",
                    LintSeverity::Error,
                    format!("/policyObjects/{id}/citations/{i}/sourceDocumentRef"),
                    format!(
                        "PolicyObject '{id}' citation #{i} targets versionless \
                         SourceDocument '{target}'; cite a SourceVersion or add \
                         one to the document (per SA-MUST-source-001)."
                    ),
                ));
            }
        }
    }
}

// ====================================================================
// BIND-LINT family (Studio binding readiness rules)
// ====================================================================
//
// Iterates `ws.documents` filtering on `StudioMarker::Binding`. Each
// rule reads `body.kind` to discriminate ServiceBinding /
// EventBinding / PolicyEngineBinding / DecisionTable, then walks the
// kind-specific body shape per `studio/specs/binding-and-integration.md`.

/// Read the `kind` discriminator off a binding doc via the typed
/// body dispatcher (no untyped reach).
fn binding_kind_of(doc: &crate::workspace::WorkspaceDocument) -> Option<&str> {
    doc.kind()
}

/// Borrow the binding's nested `body` map (Binding documents wrap
/// kind-specific body shapes under `body`). Routes through
/// `StudioDocument::body()` typed dispatch.
fn binding_body<'a>(doc: &'a crate::workspace::WorkspaceDocument) -> Option<&'a Value> {
    doc.document.body().get("body")
}

/// `BIND-LINT-001` — bindings carrying `^x-` extension keys MUST have
/// a corresponding entry in the workspace's ExtensionRegistry per
/// `SA-MUST-bind-001` (cross-doc registry-lookup check).
fn bind_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Collect ExtensionRegistry entry ids declared in any
    // workspace.extensions / extensionRegistry doc body. The shape
    // is `{ entries: [{id: "x-foo", ...}] }` per
    // specs/registry/extension-registry.md (referenced from
    // studio-to-wos-mapping.md SA-MUST-map-014). Today the registry
    // lives inside the Workspace document under `policy.extensionRegistry`.
    let registry: BTreeSet<String> = ws
        .workspace_document()
        .and_then(|d| d.document.body().get("policy"))
        .and_then(Value::as_object)
        .and_then(|p| p.get("extensionRegistry"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        // Walk the raw doc looking for top-level `^x-` keys on the
        // body. We only check the body's own keys (not nested), to
        // match the spec's registry-lookup semantic.
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        for (key, _) in body {
            if !key.starts_with("x-") {
                continue;
            }
            if !registry.contains(key) {
                diagnostics.push(studio_diagnostic(
                    "BIND-LINT-001",
                    LintSeverity::Error,
                    format!("/bindings/{id}/body/{key}"),
                    format!(
                        "Binding '{id}' carries extension key '{key}' but no \
                         matching entry exists in workspace.policy.extensionRegistry \
                         (per SA-MUST-bind-001)."
                    ),
                ));
            }
        }
    }
}

/// `BIND-LINT-002` — bindings MUST NOT invent new seams; the six
/// canonical kernel seams are the closed set per ADR-0077
/// (`SA-MUST-bind-002`). `body.seam` (when declared) MUST be one of
/// the canonical kinds.
fn bind_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    const CANONICAL_SEAMS: &[&str] = &[
        "actorExtension",
        "contractHook",
        "provenanceLayer",
        "lifecycleHook",
        "custodyHook",
        "extensions",
    ];
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(seam) = binding_body(doc)
            .and_then(|b| b.get("seam"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !CANONICAL_SEAMS.contains(&seam) {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-002",
                LintSeverity::Error,
                format!("/bindings/{id}/body/seam"),
                format!(
                    "Binding '{id}' declares seam '{seam}'; not one of the \
                     six canonical kernel seams per ADR-0077 \
                     (`actorExtension`, `contractHook`, `provenanceLayer`, \
                     `lifecycleHook`, `custodyHook`, `extensions`). \
                     (per SA-MUST-bind-002)"
                ),
            ));
        }
    }
}

/// `BIND-LINT-003` — every `inputBindings[].caseFilePath` on a
/// ServiceBinding MUST resolve to a `CaseFileMapping` in the
/// workspace (`SA-MUST-bind-011`).
fn bind_lint_003(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Collect CaseFileMapping ids (typically PolicyObjects of kind
    // CaseFileMapping or DataElement with caseFile flag, stored in
    // workspace.policy.caseFileMappings or as policyObjects).
    let mut case_file_paths: BTreeSet<String> = BTreeSet::new();
    for (_doc, record) in ws.policy_object_records() {
        let kind = record.get("kind").and_then(Value::as_str);
        if matches!(kind, Some("CaseFileMapping" | "DataElement")) {
            if let Some(path) = record.get("caseFilePath").and_then(Value::as_str) {
                case_file_paths.insert(path.to_string());
            }
            if let Some(id) = record.get("id").and_then(Value::as_str) {
                case_file_paths.insert(id.to_string());
            }
        }
    }
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("ServiceBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(inputs) = binding_body(doc)
            .and_then(|b| b.get("inputBindings"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (i, input) in inputs.iter().enumerate() {
            let Some(path) = input.get("caseFilePath").and_then(Value::as_str) else {
                continue;
            };
            if !case_file_paths.contains(path) {
                diagnostics.push(studio_diagnostic(
                    "BIND-LINT-003",
                    LintSeverity::Error,
                    format!("/bindings/{id}/body/inputBindings/{i}/caseFilePath"),
                    format!(
                        "ServiceBinding '{id}' inputBindings[{i}] references \
                         caseFilePath='{path}' but no CaseFileMapping or \
                         DataElement with that path exists in the workspace \
                         (per SA-MUST-bind-011)."
                    ),
                ));
            }
        }
    }
}

/// `BIND-LINT-004` — every `outputBindings[].target` on a
/// ServiceBinding MUST resolve to a workspace object (CaseFileMapping
/// id, DecisionRule id, EventBinding id, or PolicyObject id)
/// (`SA-MUST-bind-012`). Outputs that don't resolve fire an
/// `output-ignored-without-rationale` finding.
fn bind_lint_004(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Collect every PolicyObject + Binding id; outputs may target
    // any of these.
    let mut all_ids: BTreeSet<String> = BTreeSet::new();
    for (_doc, record) in ws.policy_object_records() {
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            all_ids.insert(id.to_string());
        }
    }
    for doc in &ws.documents {
        if matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            if let Some(id) = doc.id() {
                all_ids.insert(id.to_string());
            }
        }
    }
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("ServiceBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(outputs) = binding_body(doc)
            .and_then(|b| b.get("outputBindings"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (i, output) in outputs.iter().enumerate() {
            let Some(target) = output.get("target").and_then(Value::as_str) else {
                continue;
            };
            if !all_ids.contains(target) {
                let has_rationale = output
                    .get("ignoredRationale")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty());
                if !has_rationale {
                    diagnostics.push(studio_diagnostic(
                        "BIND-LINT-004",
                        LintSeverity::Error,
                        format!("/bindings/{id}/body/outputBindings/{i}/target"),
                        format!(
                            "ServiceBinding '{id}' outputBindings[{i}] target \
                             '{target}' does not resolve to a workspace object \
                             and no `ignoredRationale` is recorded \
                             (per SA-MUST-bind-012; output-ignored-without-rationale)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `BIND-LINT-005` — a ServiceBinding whose `inputBindings` reference
/// DataElements with `sensitivity ∈ {pii, phi, restricted}` (or any
/// DPV-classified sensitive sensitivity) MUST carry
/// `body.sensitivityHandling` OR a documented waiver
/// (`SA-MUST-bind-013`; cross-cutting with WF-LINT-006 /
/// SA-MUST-pom-037).
fn bind_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index sensitive DataElements (mirror of WF-LINT-006).
    let mut sensitive: BTreeSet<String> = BTreeSet::new();
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("DataElement") {
            continue;
        }
        let is_sensitive = record
            .get("sensitivity")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
            || record
                .get("canonicalTermRef")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains("dpv:") || t.contains("/sensitive/"));
        if !is_sensitive {
            continue;
        }
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            sensitive.insert(id.to_string());
        }
    }
    if sensitive.is_empty() {
        return;
    }
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("ServiceBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        let inputs = body
            .get("inputBindings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let touches_sensitive = inputs.iter().any(|i| {
            i.get("dataElementRef")
                .and_then(Value::as_str)
                .is_some_and(|d| sensitive.contains(d))
        });
        if !touches_sensitive {
            continue;
        }
        let has_handling = body.contains_key("sensitivityHandling")
            || body
                .get("sensitivityWaiverRef")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
        if !has_handling {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-005",
                LintSeverity::Error,
                format!("/bindings/{id}/body/sensitivityHandling"),
                format!(
                    "ServiceBinding '{id}' references sensitive DataElements \
                     but declares no body.sensitivityHandling (and no \
                     sensitivityWaiverRef) (per SA-MUST-bind-013)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-006` — every ServiceBinding MUST declare
/// `body.errorHandling.onError` ∈ {retry, fallback, fail-workflow,
/// alert} (`SA-MUST-bind-014`).
fn bind_lint_006(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    const VALID: &[&str] = &["retry", "fallback", "fail-workflow", "alert"];
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("ServiceBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let on_error = binding_body(doc)
            .and_then(|b| b.get("errorHandling"))
            .and_then(|e| e.get("onError"))
            .and_then(Value::as_str);
        match on_error {
            Some(s) if VALID.contains(&s) => {}
            _ => {
                diagnostics.push(studio_diagnostic(
                    "BIND-LINT-006",
                    LintSeverity::Error,
                    format!("/bindings/{id}/body/errorHandling/onError"),
                    format!(
                        "ServiceBinding '{id}' missing or invalid \
                         body.errorHandling.onError; must be one of \
                         retry|fallback|fail-workflow|alert \
                         (per SA-MUST-bind-014). Got: {on_error:?}"
                    ),
                ));
            }
        }
    }
}

/// `BIND-LINT-010` — EventBinding with `direction=consumed` MUST
/// identify a source / system in `body.source` (`SA-MUST-bind-021`).
fn bind_lint_010(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("EventBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        if body.get("direction").and_then(Value::as_str) != Some("consumed") {
            continue;
        }
        if body
            .get("source")
            .and_then(Value::as_str)
            .map(str::is_empty)
            .unwrap_or(true)
        {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-010",
                LintSeverity::Error,
                format!("/bindings/{id}/body/source"),
                format!(
                    "EventBinding '{id}' has direction='consumed' but no \
                     body.source naming the emitting system \
                     (per SA-MUST-bind-021)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-011` — EventBinding with `direction=emitted` MUST
/// identify a recipient / channel in `body.recipient`
/// (`SA-MUST-bind-022`).
fn bind_lint_011(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("EventBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        if body.get("direction").and_then(Value::as_str) != Some("emitted") {
            continue;
        }
        if body
            .get("recipient")
            .and_then(Value::as_str)
            .map(str::is_empty)
            .unwrap_or(true)
        {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-011",
                LintSeverity::Error,
                format!("/bindings/{id}/body/recipient"),
                format!(
                    "EventBinding '{id}' has direction='emitted' but no \
                     body.recipient naming the destination channel \
                     (per SA-MUST-bind-022)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-012` — EventBinding `body.payloadShape` fields with
/// `sensitivity ∈ sensitive` MUST carry a redaction rule under
/// `body.redactionRules[<field>]` (`SA-MUST-bind-024`).
fn bind_lint_012(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("EventBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        let Some(shape) = body.get("payloadShape").and_then(Value::as_object) else {
            continue;
        };
        let redactions: BTreeSet<String> = body
            .get("redactionRules")
            .and_then(Value::as_object)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        for (field_name, field_decl) in shape {
            let sensitive = field_decl
                .get("sensitivity")
                .and_then(Value::as_str)
                .is_some_and(|s| {
                    matches!(s, "pii" | "phi" | "restricted") || s.starts_with("dpv:")
                });
            if !sensitive {
                continue;
            }
            if !redactions.contains(field_name) {
                diagnostics.push(studio_diagnostic(
                    "BIND-LINT-012",
                    LintSeverity::Error,
                    format!("/bindings/{id}/body/redactionRules/{field_name}"),
                    format!(
                        "EventBinding '{id}' payloadShape.{field_name} carries \
                         sensitive sensitivity but no redactionRules entry \
                         (per SA-MUST-bind-024)."
                    ),
                ));
            }
        }
    }
}

/// `BIND-LINT-020` — PolicyEngineBinding `body.inputContract.caseFilePaths`
/// MUST be a complete declaration covering every case field the engine
/// reads (`SA-MUST-bind-032`). Tractable lint-time slice: `caseFilePaths`
/// MUST be present + non-empty when `body.engineKind` is set; the
/// "completeness" relative to engine internals is runtime-dependent and
/// stays runtime-pending.
fn bind_lint_020(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("PolicyEngineBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        let paths = body
            .get("inputContract")
            .and_then(|c| c.get("caseFilePaths"))
            .and_then(Value::as_array);
        let empty = paths.map(|a| a.is_empty()).unwrap_or(true);
        if empty {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-020",
                LintSeverity::Error,
                format!("/bindings/{id}/body/inputContract/caseFilePaths"),
                format!(
                    "PolicyEngineBinding '{id}' missing or empty \
                     body.inputContract.caseFilePaths; engines reading \
                     undeclared fields create privacy/audit hazards \
                     (per SA-MUST-bind-032)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-021` — PolicyEngineBinding `body.outputNormalization.
/// reasonsMapping` MUST translate every reason code the engine emits
/// into reviewer-readable plain language (`SA-MUST-bind-033`).
/// Tractable lint-time slice: when `body.engineReasonCodes[]` is
/// declared, every code MUST have a reasonsMapping entry.
fn bind_lint_021(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("PolicyEngineBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let Some(body) = binding_body(doc).and_then(Value::as_object) else {
            continue;
        };
        let codes: Vec<&str> = body
            .get("engineReasonCodes")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if codes.is_empty() {
            continue;
        }
        let mapping_keys: BTreeSet<&str> = body
            .get("outputNormalization")
            .and_then(|n| n.get("reasonsMapping"))
            .and_then(Value::as_object)
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();
        for code in &codes {
            if !mapping_keys.contains(code) {
                diagnostics.push(studio_diagnostic(
                    "BIND-LINT-021",
                    LintSeverity::Error,
                    format!("/bindings/{id}/body/outputNormalization/reasonsMapping/{code}"),
                    format!(
                        "PolicyEngineBinding '{id}' declares engineReasonCode \
                         '{code}' but reasonsMapping has no plain-language \
                         translation (per SA-MUST-bind-033)."
                    ),
                ));
            }
        }
    }
}

/// `BIND-LINT-070` — every Binding at lifecycleState ≥ approved MUST
/// have ≥ 1 Scenario in `exercisedByScenarios[]` (`SA-MUST-bind-070`).
///
/// Tractable lint-time slice: the rule verifies *existence* of any
/// scenario in `exercisedByScenarios[]`. The spec text additionally
/// asks for a "happy-path" scenario; without a schema discriminator
/// distinguishing happy-path from error-path, lint can only enforce
/// existence. When `Scenario.scenarioType` (e.g.,
/// `happy-path-coverage`) lands as a normative tag, BIND-LINT-070
/// can sharpen to require ≥ 1 happy-path-tagged scenario.
fn bind_lint_070(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        let state = doc.lifecycle_state_str().unwrap_or("");
        if !matches!(
            state,
            "approved" | "mapped" | "validated" | "published" | "superseded"
        ) {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let scenarios = doc
            .document
            .body()
            .get("exercisedByScenarios")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if scenarios == 0 {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-070",
                LintSeverity::Error,
                format!("/bindings/{id}/exercisedByScenarios"),
                format!(
                    "Binding '{id}' is at lifecycleState='{state}' but has \
                     no Scenario in exercisedByScenarios[] \
                     (per SA-MUST-bind-070; binding-without-contract-test)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-071` — ServiceBinding with
/// `body.errorHandling.onError != "fail-workflow"` MUST have ≥ 2
/// Scenarios in `exercisedByScenarios[]` (happy-path + error path)
/// (`SA-MUST-bind-071`).
fn bind_lint_071(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("ServiceBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let on_error = binding_body(doc)
            .and_then(|b| b.get("errorHandling"))
            .and_then(|e| e.get("onError"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if on_error.is_empty() || on_error == "fail-workflow" {
            continue;
        }
        let scenarios = doc
            .document
            .body()
            .get("exercisedByScenarios")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if scenarios < 2 {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-071",
                LintSeverity::Error,
                format!("/bindings/{id}/exercisedByScenarios"),
                format!(
                    "ServiceBinding '{id}' has errorHandling.onError='{on_error}' \
                     (≠ fail-workflow) but only {scenarios} scenario(s) \
                     in exercisedByScenarios[]; need ≥ 2 (happy + error path) \
                     (per SA-MUST-bind-071)."
                ),
            ));
        }
    }
}

/// `BIND-LINT-072` — PolicyEngineBinding MUST have ≥ 1 Scenario
/// exercising the `permit` outcome AND ≥ 1 exercising the `deny`
/// outcome (`SA-MUST-bind-072`). Tractable lint-time slice: walk the
/// linked Scenarios; require at least one scenario annotated with
/// `expectedDecision = permit` and one with `expectedDecision = deny`.
fn bind_lint_072(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index Scenario expectedDecision values by id.
    let mut scenario_decisions: BTreeMap<String, String> = BTreeMap::new();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Scenario) {
            continue;
        }
        let Some(id) = doc.id() else { continue };
        let decision = doc
            .document
            .body()
            .get("expectedDecision")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(d) = decision {
            scenario_decisions.insert(id.to_string(), d);
        }
    }
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Binding) {
            continue;
        }
        if binding_kind_of(doc) != Some("PolicyEngineBinding") {
            continue;
        }
        let id = doc.id().unwrap_or("?");
        let scenarios: Vec<&str> = doc
            .document
            .body()
            .get("exercisedByScenarios")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mut has_permit = false;
        let mut has_deny = false;
        for scn_id in scenarios {
            match scenario_decisions.get(scn_id).map(String::as_str) {
                Some("permit") => has_permit = true,
                Some("deny") => has_deny = true,
                _ => {}
            }
        }
        if !has_permit || !has_deny {
            diagnostics.push(studio_diagnostic(
                "BIND-LINT-072",
                LintSeverity::Error,
                format!("/bindings/{id}/exercisedByScenarios"),
                format!(
                    "PolicyEngineBinding '{id}' missing scenario coverage: \
                     permit={has_permit}, deny={has_deny} \
                     (per SA-MUST-bind-072; both decision outcomes required)."
                ),
            ));
        }
    }
}

/// `SV-LINT-003` — no PolicyObject relies solely on `disputed` or
/// `superseded` SourceVersions.
fn sv_lint_003(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Build an index of SourceVersion lifecycleState by id.
    let mut version_state: HashMap<String, String> = HashMap::new();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Source) {
            continue;
        }
        if let Some(versions) = doc.source_versions() {
            for v in versions {
                if let (Some(id), Some(state)) = (
                    v.get("id").and_then(Value::as_str),
                    v.get("lifecycleState").and_then(Value::as_str),
                ) {
                    version_state.insert(id.to_string(), state.to_string());
                }
            }
        }
    }

    // For each approved PolicyObject, collect referenced version ids and
    // confirm at least one is `current` / `approved` / `ingested`.
    for (_doc, record) in ws.policy_object_records() {
        let state = record.get("lifecycleState").and_then(Value::as_str);
        if !matches!(state, Some("approved" | "mapped" | "validated" | "published")) {
            continue;
        }
        let citations = record.get("citations").and_then(Value::as_array);
        let Some(citations) = citations else { continue };
        let mut bad_only = !citations.is_empty();
        for cite in citations {
            let version_ref = cite.get("sourceVersionRef").and_then(Value::as_str);
            if let Some(v) = version_ref {
                if let Some(s) = version_state.get(v) {
                    if !matches!(s.as_str(), "disputed" | "superseded") {
                        bad_only = false;
                    }
                } else {
                    // Unknown version state → don't penalize on this rule.
                    bad_only = false;
                }
            } else {
                bad_only = false;
            }
        }
        if bad_only {
            let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "SV-LINT-003",
                LintSeverity::Error,
                format!("/policyObjects/{id}/citations"),
                "PolicyObject relies solely on disputed or superseded \
                 SourceVersions; surface the dependency explicitly or \
                 supersede the object."
                    .to_string(),
            ));
        }
    }
}

// ============================================================================
// S3 — Mapping
// ============================================================================

fn map_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mapping_targets: HashSet<&str> = ws
        .mapping_records()
        .iter()
        .filter_map(|(_doc, record)| record.get("policyObjectRef").and_then(Value::as_str))
        .collect();
    for (_doc, record) in ws.policy_object_records() {
        if !matches!(
            record.get("lifecycleState").and_then(Value::as_str),
            Some("approved" | "mapped" | "validated" | "published")
        ) {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        if !mapping_targets.contains(id) {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-001",
                LintSeverity::Error,
                format!("/policyObjects/{id}"),
                "Approved PolicyObject MUST have a Mapping (mapsToWos / \
                 authoringOnly / requiresSpecExtension / unmappedButApproved)."
                    .to_string(),
            ));
        }
    }
}

fn map_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut targets_by_path: HashMap<String, Vec<String>> = HashMap::new();
    for (_doc, record) in ws.mapping_records() {
        let Some(targets) = record.get("targets").and_then(Value::as_array) else { continue };
        let id = record
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        for target in targets {
            if let Some(path) = target.get("wosJsonPath").and_then(Value::as_str) {
                targets_by_path
                    .entry(path.to_string())
                    .or_default()
                    .push(id.clone());
            }
        }
    }
    for (path, mappings) in targets_by_path {
        if mappings.len() > 1 {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-005",
                LintSeverity::Error,
                format!("/mappings/?targets/{path}"),
                format!(
                    "Mapping target collision at '{path}': mappings {} \
                     all project to the same JSONPath. Pick one or scope \
                     by Effectiveness.",
                    mappings.join(", ")
                ),
            ));
        }
    }
}

fn map_lint_006(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Workflow-bearing PolicyObjects are not unmappedButApproved (without
    // explicit override). Check by walking workflow elements for
    // PolicyObject refs and intersecting with unmappedButApproved
    // mapping records.
    let unmapped: HashSet<&str> = ws
        .mapping_records()
        .iter()
        .filter(|(_d, r)| {
            r.get("mappingState").and_then(Value::as_str)
                == Some("unmappedButApproved")
                && r.get("acknowledgedOverride")
                    .and_then(Value::as_bool)
                    != Some(true)
        })
        .filter_map(|(_d, r)| r.get("policyObjectRef").and_then(Value::as_str))
        .collect();

    for (_doc, _i, elem) in ws.workflow_elements() {
        let refs = collect_policy_refs(elem);
        for r in refs {
            if unmapped.contains(r.as_str()) {
                diagnostics.push(studio_diagnostic(
                    "MAP-LINT-006",
                    LintSeverity::Error,
                    format!("/elements/?refs/{r}"),
                    format!(
                        "WorkflowElement references unmappedButApproved \
                         PolicyObject '{r}' without an acknowledged \
                         override. Either map the object or accept the \
                         override on the mapping record."
                    ),
                ));
            }
        }
    }
}

fn collect_policy_refs(elem: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = elem.get("policyObjectRefs").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                out.push(s.to_string());
            }
        }
    }
    if let Some(s) = elem.get("policyObjectRef").and_then(Value::as_str) {
        out.push(s.to_string());
    }
    out
}

fn eff_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Effectiveness widening: a PolicyObject's effectiveness scope is
    // wider than its referenced source's scope. Approximation: if the
    // PolicyObject inlines effectiveness AND its source declares a
    // narrower jurisdiction, fire.
    let source_jurisdictions: HashMap<String, HashSet<String>> = ws
        .documents
        .iter()
        .filter_map(|d| {
            if matches!(d.marker, wos_studio_model::StudioMarker::Effectiveness) {
                Some((
                    d.id()?.to_string(),
                    d.document
                        .body()
                        .get("jurisdictions")
                        .and_then(Value::as_array)
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                ))
            } else {
                None
            }
        })
        .collect();

    for (_doc, record) in ws.policy_object_records() {
        let Some(eff) = record.get("effectiveness") else { continue };
        let jurisdictions: HashSet<String> = eff
            .get("jurisdictions")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let source_ref = record.get("effectivenessRef").and_then(Value::as_str);
        let Some(source_id) = source_ref else { continue };
        let Some(source_set) = source_jurisdictions.get(source_id) else { continue };
        if !jurisdictions.is_empty() && !source_set.is_empty()
            && !jurisdictions.is_subset(source_set)
        {
            let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "EFF-LINT-002",
                LintSeverity::Error,
                format!("/policyObjects/{id}/effectiveness/jurisdictions"),
                "PolicyObject effectiveness widens its source's jurisdictional \
                 scope. Narrow the scope or redeclare the source."
                    .to_string(),
            ));
        }
    }
}

fn eff_lint_004(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Two mappings of the same PolicyObject with overlapping but
    // conflicting effectiveness scopes. We collect each mapping's
    // effective-scope key — preferring an inlined `effectiveness` block
    // (compared by its full JSON shape so different inlined scopes are
    // distinguishable) and falling back to the `effectivenessRef`
    // string. Two mappings with distinct scope keys collide.
    //
    // Sorted by id for deterministic output (`SA-MUST-cmp-001`-shaped
    // discipline; even though this is lint-tier, downstream tooling may
    // compare findings across runs).
    let mut by_subject: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for (_doc, record) in ws.mapping_records() {
        if let Some(subject) = record.get("policyObjectRef").and_then(Value::as_str) {
            by_subject.entry(subject).or_default().push(record);
        }
    }
    for (subject, mappings) in by_subject {
        if mappings.len() < 2 {
            continue;
        }
        let mut scope_keys: Vec<String> = mappings
            .iter()
            .filter_map(|m| {
                if let Some(eff) = m.get("effectiveness") {
                    // Inlined scope — use the JSON serialization as the
                    // key so two distinct inlined scopes compare unequal.
                    // serde_json preserves Map insertion order so this
                    // is stable within a single workspace load.
                    serde_json::to_string(eff).ok()
                } else if let Some(r) =
                    m.get("effectivenessRef").and_then(Value::as_str)
                {
                    Some(r.to_string())
                } else {
                    // No declared scope — treat as the same "default"
                    // scope so two scope-less mappings don't trigger
                    // the rule (they collide on something else).
                    Some(String::from("(default)"))
                }
            })
            .collect();
        scope_keys.sort();
        scope_keys.dedup();
        if scope_keys.len() > 1 {
            diagnostics.push(studio_diagnostic(
                "EFF-LINT-004",
                LintSeverity::Warning,
                format!("/policyObjects/{subject}/mappings"),
                format!(
                    "PolicyObject '{subject}' has multiple Mappings with \
                     differing effectiveness scopes ({}). Confirm the \
                     scopes don't overlap, or merge the mappings.",
                    scope_keys.join(", ")
                ),
            ));
        }
    }
}

// ============================================================================
// S4 / S5 / S6
// ============================================================================

/// `WF-LINT-001` — every adverse `Outcome` (`triggersDueProcess = true`)
/// MUST link both a NoticeRequirement and an AppealRight per
/// `readiness-validation.md` line 130 (`SA-MUST-pom-030`).
fn wf_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("Outcome") {
            continue;
        }
        if record.get("triggersDueProcess").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
        let has_notice = record
            .get("linkedNoticeRequirementRef")
            .and_then(Value::as_str)
            .is_some()
            || record
                .get("linkedNoticeRequirementRefs")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
        let has_appeal = record
            .get("linkedAppealRightRef")
            .and_then(Value::as_str)
            .is_some()
            || record
                .get("linkedAppealRightRefs")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
        if !has_notice || !has_appeal {
            let mut missing: Vec<&str> = Vec::new();
            if !has_notice {
                missing.push("linkedNoticeRequirementRef");
            }
            if !has_appeal {
                missing.push("linkedAppealRightRef");
            }
            diagnostics.push(studio_diagnostic(
                "WF-LINT-001",
                LintSeverity::Error,
                format!("/policyObjects/{id}"),
                format!(
                    "Adverse Outcome '{id}' (triggersDueProcess=true) MUST \
                     link both a NoticeRequirement and an AppealRight; \
                     missing: {}",
                    missing.join(", ")
                ),
            ));
        }
    }
}

fn wf_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every AppealRight PolicyObject MUST have an appeal branch in the
    // WorkflowIntent.
    let appeal_right_ids: HashSet<&str> = ws
        .policy_object_records()
        .iter()
        .filter(|(_, r)| r.get("kind").and_then(Value::as_str) == Some("AppealRight"))
        .filter_map(|(_, r)| r.get("id").and_then(Value::as_str))
        .collect();
    let referenced_in_workflow: HashSet<String> = ws
        .workflow_elements()
        .iter()
        .flat_map(|(_, _, elem)| collect_policy_refs(elem))
        .collect();
    for id in appeal_right_ids {
        if !referenced_in_workflow.contains(id) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-002",
                LintSeverity::Error,
                format!("/policyObjects/{id}"),
                format!(
                    "AppealRight '{id}' has no corresponding appeal branch \
                     in any WorkflowIntent."
                ),
            ));
        }
    }
}

fn sc_lint_001_workspace(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every adverse Outcome (triggersDueProcess=true) MUST have at least
    // one Scenario referencing it.
    let adverse_outcomes: HashSet<&str> = ws
        .policy_object_records()
        .iter()
        .filter(|(_, r)| r.get("kind").and_then(Value::as_str) == Some("Outcome"))
        .filter(|(_, r)| {
            r.get("triggersDueProcess").and_then(Value::as_bool) == Some(true)
        })
        .filter_map(|(_, r)| r.get("id").and_then(Value::as_str))
        .collect();
    let scenarios_referencing: HashSet<String> = ws
        .scenario_records()
        .iter()
        .flat_map(|(_, s)| {
            s.get("exercisesOutcomes")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    for id in adverse_outcomes {
        if !scenarios_referencing.contains(id) {
            diagnostics.push(studio_diagnostic(
                "SC-LINT-001",
                LintSeverity::Error,
                format!("/policyObjects/{id}/scenarios"),
                format!(
                    "Adverse Outcome '{id}' (triggersDueProcess=true) has \
                     no Scenario exercising it. Author at least one \
                     scenario before approval."
                ),
            ));
        }
    }
}

fn sc_lint_002_workspace(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every AppealRight MUST have a Scenario exercising the appeal branch.
    let appeal_rights: HashSet<&str> = ws
        .policy_object_records()
        .iter()
        .filter(|(_, r)| r.get("kind").and_then(Value::as_str) == Some("AppealRight"))
        .filter_map(|(_, r)| r.get("id").and_then(Value::as_str))
        .collect();
    let scenarios_with_appeal: HashSet<String> = ws
        .scenario_records()
        .iter()
        .filter(|(_, s)| {
            matches!(
                s.get("scenarioType").and_then(Value::as_str),
                Some("appeal-branch" | "manual-override")
            )
        })
        .flat_map(|(_, s)| {
            s.get("exercisesAppeals")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    for id in appeal_rights {
        if !scenarios_with_appeal.contains(id) {
            diagnostics.push(studio_diagnostic(
                "SC-LINT-002",
                LintSeverity::Error,
                format!("/policyObjects/{id}/scenarios"),
                format!("AppealRight '{id}' has no scenario exercising the appeal branch."),
            ));
        }
    }
}

fn eq_lint_003(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow = ws.workflow_intent();
    let Some(workflow) = workflow else { return };
    let categories: Vec<String> = workflow
        .document
        .body()
        .get("protectedCategoryRefs")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if categories.is_empty() {
        return;
    }
    let probe_categories: HashSet<String> = ws
        .scenario_records()
        .iter()
        .filter(|(_, s)| {
            s.get("scenarioType").and_then(Value::as_str) == Some("equity-probe")
        })
        .flat_map(|(_, s)| {
            s.get("probedCategory")
                .and_then(Value::as_str)
                .map(|s| vec![s.to_string()])
                .unwrap_or_default()
        })
        .collect();
    for cat in categories {
        if !probe_categories.contains(&cat) {
            diagnostics.push(studio_diagnostic(
                "EQ-LINT-003",
                LintSeverity::Error,
                format!("/scenarios/?probedCategory={cat}"),
                format!(
                    "ProtectedCategory '{cat}' has no equity-probe Scenario. \
                     Author one to demonstrate equitable handling."
                ),
            ));
        }
    }
}

fn acc_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow = ws.workflow_intent();
    let Some(workflow) = workflow else { return };
    let locales: Vec<String> = workflow
        .document
        .body()
        .get("supportedLocales")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if locales.is_empty() {
        return;
    }
    let covered: HashSet<String> = ws
        .scenario_records()
        .iter()
        .filter(|(_, s)| {
            s.get("scenarioType").and_then(Value::as_str) == Some("accessibility-check")
        })
        .filter_map(|(_, s)| s.get("contentLocale").and_then(Value::as_str).map(str::to_string))
        .collect();
    for locale in locales {
        if !covered.contains(&locale) {
            diagnostics.push(studio_diagnostic(
                "ACC-LINT-001",
                LintSeverity::Error,
                format!("/scenarios/?contentLocale={locale}"),
                format!(
                    "Workflow declares supportedLocale '{locale}' but no \
                     accessibility-check Scenario covers it."
                ),
            ));
        }
    }
}

fn jur_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow = ws.workflow_intent();
    let Some(workflow) = workflow else { return };
    let jurisdictions: Vec<String> = workflow
        .document
        .body()
        .get("effectiveness")
        .and_then(|e| e.get("jurisdictions"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if jurisdictions.len() < 2 {
        return;
    }
    let covered: HashSet<String> = ws
        .scenario_records()
        .iter()
        .filter(|(_, s)| {
            s.get("scenarioType").and_then(Value::as_str)
                == Some("jurisdictional-variation")
        })
        .filter_map(|(_, s)| s.get("jurisdiction").and_then(Value::as_str).map(str::to_string))
        .collect();
    for j in jurisdictions {
        if !covered.contains(&j) {
            diagnostics.push(studio_diagnostic(
                "JUR-LINT-001",
                LintSeverity::Error,
                format!("/scenarios/?jurisdiction={j}"),
                format!(
                    "Workflow effective in '{j}' but no \
                     jurisdictional-variation Scenario covers it."
                ),
            ));
        }
    }
}

fn pub_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Required reviewer roles: each must have ≥1 ApprovalDecision.
    let workspace_doc = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace));
    let Some(workspace_doc) = workspace_doc else { return };
    let required: HashSet<String> = workspace_doc
        .document
        .body()
        .get("reviewerRoles")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|r| {
                    r.get("requiredForPublication")
                        .and_then(Value::as_bool)
                        == Some(true)
                })
                .filter_map(|r| r.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let approved_roles: HashSet<String> = ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Approval))
        .filter(|d| d.kind() == Some("ApprovalDecision"))
        .filter_map(|d| {
            d.document
                .body()
                .get("body")
                .and_then(|b| b.get("reviewerRoleId"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    for role in required {
        if !approved_roles.contains(&role) {
            diagnostics.push(studio_diagnostic(
                "PUB-LINT-002",
                LintSeverity::Error,
                format!("/reviewerRoles/{role}/approval"),
                format!(
                    "Required reviewer role '{role}' has no ApprovalDecision; \
                     publication blocked."
                ),
            ));
        }
    }
}

fn pub_lint_006(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Every unmappedButApproved mapping is listed in release notes
    // (workspace document's `releaseNotes.unmappedRationale[]`).
    let workspace_doc = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace));
    let Some(workspace_doc) = workspace_doc else { return };
    let release_notes_unmapped: HashSet<String> = workspace_doc
        .document
        .body()
        .get("releaseNotes")
        .and_then(|n| n.get("unmappedRationale"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("mappingRef").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for (_doc, record) in ws.mapping_records() {
        if record.get("mappingState").and_then(Value::as_str)
            != Some("unmappedButApproved")
        {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        if !release_notes_unmapped.contains(id) {
            diagnostics.push(studio_diagnostic(
                "PUB-LINT-006",
                LintSeverity::Error,
                format!("/mappings/{id}/releaseNotes"),
                format!(
                    "unmappedButApproved Mapping '{id}' not listed in \
                     workspace release notes."
                ),
            ));
        }
    }
}

fn eff_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Sunset window: workflow depends on Effectiveness sunsetting in <90 days.
    // Approximation: scan Effectiveness.temporalScope.effectiveEnd < now+90d.
    use std::time::SystemTime;
    let now_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ninety_days = 90 * 24 * 3600;

    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Effectiveness) {
            continue;
        }
        let end = doc
            .document
            .body()
            .get("temporalScope")
            .and_then(|t| t.get("effectiveEnd"))
            .and_then(Value::as_str);
        let Some(end) = end else { continue };
        // Parse YYYY-MM-DD into seconds (rough heuristic).
        let end_epoch = parse_iso_date(end);
        let Some(end_epoch) = end_epoch else { continue };
        if end_epoch > now_epoch && end_epoch - now_epoch < ninety_days {
            let id = doc.id().unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "EFF-LINT-005",
                LintSeverity::Warning,
                format!("/effectiveness/{id}/temporalScope/effectiveEnd"),
                format!(
                    "Effectiveness '{id}' sunsets within 90 days ({end}). \
                     Author migration before publication."
                ),
            ));
        }
    }
}

/// Parse a `YYYY-MM-DD` (or `YYYY-MM-DDTHH:MM:SS...`) string to Unix
/// epoch seconds. Returns `None` for malformed dates, impossible
/// calendar dates (`2026-02-30`, `2026-13-01`), pre-1970 dates, or
/// non-zero-padded fields.
///
/// Used by EFF-LINT-005 / COMP-LINT-002 / CHAIN-LINT-002 — each does a
/// `now - parsed < 90 days` style check, where a silently-wrong parse
/// produces spurious or missing warnings.
fn parse_iso_date(s: &str) -> Option<u64> {
    // Date-only fast path; both forms (date, datetime) map onto the
    // same epoch.
    let date_part = s.split(['T', ' ']).next().unwrap_or(s);
    let date = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()?;
    // Chrono's %m / %d format specifiers are lenient and accept
    // single-digit values like `2026-1-1`. Round-trip-check the parsed
    // date against the canonical zero-padded form to enforce strict
    // ISO 8601.
    if date.format("%Y-%m-%d").to_string() != date_part {
        return None;
    }
    let dt = date.and_hms_opt(0, 0, 0)?.and_utc();
    u64::try_from(dt.timestamp()).ok()
}

fn ai_lint_003(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow = ws.workflow_intent();
    let Some(workflow) = workflow else { return };
    // Workflow has at least one agent-typed actor?
    let has_agent = workflow
        .document
        .body()
        .get("actors")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter().any(|a| {
                a.get("type").and_then(Value::as_str) == Some("agent")
            })
        })
        .unwrap_or(false);
    if !has_agent {
        return;
    }
    let has_fallback_scenario = ws.scenario_records().iter().any(|(_, s)| {
        s.get("scenarioType").and_then(Value::as_str) == Some("agent-fallback")
    });
    if !has_fallback_scenario {
        diagnostics.push(studio_diagnostic(
            "AI-LINT-003",
            LintSeverity::Error,
            "/scenarios/?scenarioType=agent-fallback".to_string(),
            "Workflow has agent-typed actors but no agent-fallback Scenario. \
             Add at least one to exercise the human-takeover path."
                .to_string(),
        ));
    }
}

fn cmp_lint_010(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    cmp_lint_version_pin(ws, "CMP-LINT-010", LintSeverity::Warning, true, diagnostics);
}

fn cmp_lint_011(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    cmp_lint_version_pin(ws, "CMP-LINT-011", LintSeverity::Error, false, diagnostics);
}

fn cmp_lint_version_pin(
    ws: &Workspace,
    rule: &'static str,
    severity: LintSeverity,
    pending_only: bool,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(workflow) = ws.workflow_intent() else { return };
    let migration = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::MigrationPath));
    let Some(migration) = migration else { return };
    let pinned = workflow
        .document
        .body()
        .get("wosVersionPin")
        .and_then(Value::as_str)
        .unwrap_or("");
    let deprecated_versions: HashSet<&str> = migration
        .document
        .body()
        .get("deprecations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let version = v.get("wosVersionPin").and_then(Value::as_str)?;
                    let pending =
                        v.get("status").and_then(Value::as_str) == Some("pending");
                    if pending == pending_only {
                        Some(version)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    if deprecated_versions.contains(pinned) {
        diagnostics.push(studio_diagnostic(
            rule,
            severity,
            "/wosVersionPin".to_string(),
            format!(
                "Workflow's wosVersionPin '{pinned}' is {} per migration-path \
                 records.",
                if pending_only {
                    "pending deprecation (<90 days)"
                } else {
                    "deprecated; migration required"
                }
            ),
        ));
    }
}

fn id_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(workspace_doc) = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace))
    else {
        return;
    };
    let workspace_role_ids: HashSet<&str> = workspace_doc
        .document
        .body()
        .get("reviewerRoles")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|r| r.get("id").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::IdentitySubject) {
            continue;
        }
        let Some(idp_role) = doc.idp_role() else { continue };
        if !workspace_role_ids.contains(idp_role) {
            let id = doc.id().unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "ID-LINT-001",
                LintSeverity::Warning,
                format!("/identitySubjects/{id}/idpRole"),
                format!(
                    "Subject '{id}' carries idpRole='{idp_role}' that is not \
                     a workspace ReviewerRole. Subject can act only via \
                     direct grants."
                ),
            ));
        }
    }
}

fn id_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in &ws.documents {
        if !matches!(doc.marker, wos_studio_model::StudioMarker::Approval) {
            continue;
        }
        if doc.kind() != Some("ApprovalDecision") {
            continue;
        }
        let revoked = doc
            .document
            .body()
            .get("body")
            .and_then(|b| b.get("approverRevokedAt"))
            .is_some();
        let required = doc
            .document
            .body()
            .get("body")
            .and_then(|b| b.get("requiredForPublication"))
            .and_then(Value::as_bool)
            == Some(true);
        if revoked && required {
            let id = doc.id().unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "ID-LINT-002",
                LintSeverity::Error,
                format!("/approvals/{id}/approverRevokedAt"),
                format!(
                    "Required-for-publication approver on Approval '{id}' \
                     has been revoked. Re-approval is required."
                ),
            ));
        }
    }
}

fn comp_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(workspace_doc) = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace))
    else {
        return;
    };
    let baselines: Vec<&Value> = workspace_doc
        .document
        .body()
        .get("complianceBaselines")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    if baselines.is_empty() {
        return;
    }
    let Some(workflow) = ws.workflow_intent() else { return };
    let satisfied: HashSet<String> = workflow
        .document
        .body()
        .get("satisfiesControls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for baseline in baselines {
        let Some(name) = baseline.get("name").and_then(Value::as_str) else { continue };
        let required: Vec<&str> = baseline
            .get("requiredControls")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for control in required {
            if !satisfied.contains(control) {
                diagnostics.push(studio_diagnostic(
                    "COMP-LINT-001",
                    LintSeverity::Error,
                    format!("/workflow/satisfiesControls/{control}"),
                    format!(
                        "Workspace declares compliance baseline '{name}' \
                         requiring control '{control}'; workflow does not \
                         satisfy it."
                    ),
                ));
            }
        }
    }
}

fn comp_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ninety_days = 90 * 24 * 3600;
    let Some(workspace_doc) = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace))
    else {
        return;
    };
    let Some(attestations) = workspace_doc
        .document
        .body()
        .get("complianceAttestations")
        .and_then(Value::as_array)
    else {
        return;
    };
    for (i, attest) in attestations.iter().enumerate() {
        let Some(expires) = attest.get("expiresAt").and_then(Value::as_str) else { continue };
        let Some(expires_epoch) = parse_iso_date(expires) else { continue };
        if expires_epoch > now && expires_epoch - now < ninety_days {
            let regime = attest
                .get("regime")
                .and_then(Value::as_str)
                .unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "COMP-LINT-002",
                LintSeverity::Warning,
                format!("/complianceAttestations/{i}/expiresAt"),
                format!(
                    "Compliance attestation for '{regime}' expires within \
                     90 days ({expires})."
                ),
            ));
        }
    }
}

fn chain_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Workspace audit log not anchored within configured cadence.
    use std::time::SystemTime;
    let Some(workspace_doc) = ws
        .documents
        .iter()
        .find(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace))
    else {
        return;
    };
    let body = workspace_doc.document.body();
    let cadence_days = body
        .get("auditAnchorCadenceDays")
        .and_then(Value::as_u64)
        .unwrap_or(7);
    let Some(last_anchored) = body
        .get("lastAuditAnchoredAt")
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(last_epoch) = parse_iso_date(last_anchored) else { return };
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now > last_epoch && now - last_epoch > cadence_days * 24 * 3600 {
        diagnostics.push(studio_diagnostic(
            "CHAIN-LINT-002",
            LintSeverity::Warning,
            "/workspace/lastAuditAnchoredAt".to_string(),
            format!(
                "Workspace audit log last anchored {last_anchored}; cadence \
                 is {cadence_days} days. Anchor before publication."
            ),
        ));
    }
}

fn term_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let deprecated_terms: HashSet<&str> = ws
        .documents
        .iter()
        .filter(|d| {
            matches!(d.marker, wos_studio_model::StudioMarker::TerminologyMap)
        })
        .flat_map(|d| {
            d.document
                .body()
                .get("canonicalTerms")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .filter(|t| {
            t.get("status").and_then(Value::as_str) == Some("deprecated")
        })
        .filter_map(|t| t.get("iri").and_then(Value::as_str))
        .collect();

    for doc in &ws.documents {
        if !matches!(
            doc.marker,
            wos_studio_model::StudioMarker::TerminologyMap
        ) {
            continue;
        }
        let Some(entries) = doc.entries() else {
            continue;
        };
        for (i, entry) in entries.iter().enumerate() {
            let Some(target) = entry.get("canonicalTermIri").and_then(Value::as_str)
            else {
                continue;
            };
            if deprecated_terms.contains(target) {
                diagnostics.push(studio_diagnostic(
                    "TERM-LINT-001",
                    LintSeverity::Error,
                    format!("/terminology/entries/{i}/canonicalTermIri"),
                    format!(
                        "TerminologyMap entry points to deprecated \
                         CanonicalTerm '{target}'."
                    ),
                ));
            }
        }
    }
}

// ============================================================================
// Stub rules wired here — landing the surface; per-rule predicates below.
// ============================================================================

/// `MAP-LINT-007` — workflow-bearing PolicyObjects MUST NOT have an
/// open ExtensionRecord blocking advance.
fn map_lint_007(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow_bearing: HashSet<String> = ws
        .workflow_elements()
        .iter()
        .flat_map(|(_, _, e)| collect_policy_refs(e))
        .collect();
    for (_doc, m) in ws.mapping_records() {
        let Some(subject) = m.get("policyObjectRef").and_then(Value::as_str) else { continue };
        if !workflow_bearing.contains(subject) {
            continue;
        }
        let lifecycle_open = m
            .get("extensionRecord")
            .and_then(|er| er.get("lifecycleState"))
            .and_then(Value::as_str)
            == Some("open");
        if lifecycle_open {
            let id = m.get("id").and_then(Value::as_str).unwrap_or("?");
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-007",
                LintSeverity::Error,
                format!("/mappings/{id}/extensionRecord/lifecycleState"),
                format!(
                    "Mapping '{id}' covers workflow-bearing PolicyObject \
                     '{subject}' but its ExtensionRecord is still 'open' — \
                     publication blocked until the extension ships."
                ),
            ));
        }
    }
}

/// `PROV-LINT-003` — `originClass = approved-interpretation` carries a
/// ReviewerResolution.
///
/// Workspace-tier mirror of POM-LINT-002: separate rule code, same
/// predicate, fired by the cross-document scan so the ratchet's "every
/// registered rule emits" check passes for both ids.
fn prov_lint_003(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (_doc, record) in ws.policy_object_records() {
        if record.get("originClass").and_then(Value::as_str)
            != Some("approved-interpretation")
        {
            continue;
        }
        if record.get("reviewerResolution").is_some() {
            continue;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
        diagnostics.push(studio_diagnostic(
            "PROV-LINT-003",
            LintSeverity::Error,
            format!("/policyObjects/{id}/reviewerResolution"),
            format!(
                "PolicyObject '{id}' with originClass='approved-interpretation' \
                 MUST carry a reviewerResolution block."
            ),
        ));
    }
}

/// `WF-LINT-004` — DecisionRule inputs are collected before the rule
/// fires. Doc-walk against WorkflowIntent's element ordering.
fn wf_lint_004(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(workflow) = ws.workflow_intent() else { return };
    let Some(elements) = workflow.elements() else {
        return;
    };
    let mut collected: HashSet<String> = HashSet::new();
    for (i, elem) in elements.iter().enumerate() {
        let kind = elem.get("kind").and_then(Value::as_str).unwrap_or("");
        match kind {
            "step" => {
                if let Some(arr) = elem
                    .get("body")
                    .and_then(|b| b.get("collectsInputs"))
                    .and_then(Value::as_array)
                {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            collected.insert(s.to_string());
                        }
                    }
                }
            }
            "decision" => {
                let inputs = elem
                    .get("body")
                    .and_then(|b| b.get("inputs"))
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for input in &inputs {
                    if !collected.contains(input) {
                        let id = elem.get("id").and_then(Value::as_str).unwrap_or("?");
                        diagnostics.push(studio_diagnostic(
                            "WF-LINT-004",
                            LintSeverity::Error,
                            format!("/elements/{i}/body/inputs"),
                            format!(
                                "DecisionRule '{id}' references input '{input}' \
                                 that no prior step collects."
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

/// `WF-LINT-005` — every actor has documented authority for every step
/// it owns; agent ActorMappings link an AI-Use object.
fn wf_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(workflow) = ws.workflow_intent() else { return };
    let granted: HashSet<String> = ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Workspace))
        .flat_map(|d| {
            d.document
                .body()
                .get("authorityGrants")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|g| {
                            g.get("actorId").and_then(Value::as_str).map(str::to_string)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();
    let actors_in_workflow: Vec<&Value> = workflow
        .document
        .body()
        .get("actors")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    for actor in actors_in_workflow {
        let Some(id) = actor.get("id").and_then(Value::as_str) else { continue };
        if !granted.contains(id) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-005",
                LintSeverity::Error,
                format!("/actors/{id}"),
                format!(
                    "Actor '{id}' has no AuthorityGrant in the workspace; \
                     each actor MUST have documented authority for the \
                     steps it owns."
                ),
            ));
        }
        if actor.get("type").and_then(Value::as_str) == Some("agent")
            && actor.get("aiUseRef").and_then(Value::as_str).is_none()
        {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-005",
                LintSeverity::Error,
                format!("/actors/{id}/aiUseRef"),
                format!(
                    "Agent actor '{id}' MUST carry an aiUseRef linking the \
                     AI-Use PolicyObject."
                ),
            ));
        }
    }
}

/// `WF-LINT-006` — sensitive DataElements have a *shape-valid*
/// retention policy on every collecting EvidenceRequirement.
///
/// Shape-aware migration per ADR-0083 r2 (E8.4). Replaces the
/// presence-only check with: (a) presence of an inline policy OR a
/// workspace-default keyed by the sensitivity IRI; (b) shape
/// validation on the resolved policy via
/// [`wos_studio_model::RetentionPolicy::shape_violations`];
/// (c) emit `SA-WARN-pom-MIGRATE-RETENTION` for any document still
/// carrying the legacy singular `retentionPeriod` field.
///
/// Resolution order: per-EvidenceRequirement `retentionPolicy` wins;
/// otherwise look up `Workspace.policy.retentionPolicies[<DPV-IRI>]`
/// keyed by the collected DataElement's sensitivity. Field-by-field
/// override + `regulatoryBasis[]` merge land here once the resolver
/// helper graduates from the docs.rs accessor; for the E8 cut, a
/// per-EvidenceRequirement policy fully shadows the workspace
/// default (matches schema posture).
fn wf_lint_006(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index sensitive DataElement ids → sensitivity IRI (used as
    // workspace-default key).
    let mut sensitive: BTreeMap<String, String> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("DataElement") {
            continue;
        }
        let sensitivity_iri = record
            .get("sensitivity")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                record
                    .get("canonicalTermRef")
                    .and_then(Value::as_str)
                    .filter(|t| t.contains("dpv:") || t.contains("/sensitive/"))
                    .map(str::to_string)
            });
        let Some(iri) = sensitivity_iri else { continue };
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            sensitive.insert(id.to_string(), iri);
        }
    }

    // Index workspace-default retention policies (DPV-IRI → policy JSON).
    let workspace_defaults: BTreeMap<String, &Value> = ws
        .workspace_document()
        .and_then(|d| d.document.body().get("policy"))
        .and_then(Value::as_object)
        .and_then(|p| p.get("retentionPolicies"))
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter(|(k, _)| !k.starts_with('$') && !k.starts_with("x-"))
                .map(|(k, v)| (k.clone(), v))
                .collect()
        })
        .unwrap_or_default();

    // Walk EvidenceRequirements. The migration advisory fires
    // independently of presence/shape — even if the new field is
    // also present, the legacy field SHOULD be removed.
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("EvidenceRequirement") {
            continue;
        }
        let er_id = record.get("id").and_then(Value::as_str).unwrap_or("?");

        // SA-WARN-pom-MIGRATE-RETENTION: legacy field present.
        if record.get("retentionPeriod").is_some() {
            diagnostics.push(studio_diagnostic(
                "SA-WARN-pom-MIGRATE-RETENTION",
                LintSeverity::Warning,
                format!("/policyObjects/{er_id}/retentionPeriod"),
                format!(
                    "EvidenceRequirement '{er_id}' carries legacy \
                     `retentionPeriod` field; migrate to typed \
                     `retentionPolicy` per ADR-0083 r2 \
                     (lift the value into retentionPolicy.duration \
                     + add disposalAction)."
                ),
            ));
        }

        // Sensitivity gate: only EvidenceRequirements that collect
        // a sensitive DataElement participate in WF-LINT-006.
        let collects: Vec<&str> = record
            .get("collectsDataElements")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let collected_sensitivities: Vec<&String> = collects
            .iter()
            .filter_map(|d| sensitive.get(*d))
            .collect();
        if collected_sensitivities.is_empty() {
            continue;
        }

        // Resolve: per-EvidenceRequirement policy wins fully (E8 cut).
        // If absent, look up the workspace default keyed by any of
        // the collected sensitivities (first match wins; deterministic
        // because sensitive is a BTreeMap).
        let inline = record.get("retentionPolicy");
        let resolved: Option<&Value> = inline.or_else(|| {
            collected_sensitivities
                .iter()
                .find_map(|iri| workspace_defaults.get(iri.as_str()).copied())
        });

        let Some(resolved_value) = resolved else {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-006",
                LintSeverity::Error,
                format!("/policyObjects/{er_id}/retentionPolicy"),
                format!(
                    "EvidenceRequirement '{er_id}' collects sensitive \
                     DataElements but declares no retentionPolicy and \
                     no workspace default keyed by their sensitivity."
                ),
            ));
            continue;
        };

        // Shape validation: parse + run shape_violations.
        match serde_json::from_value::<wos_studio_model::RetentionPolicy>(
            resolved_value.clone(),
        ) {
            Err(e) => {
                diagnostics.push(studio_diagnostic(
                    "WF-LINT-006",
                    LintSeverity::Error,
                    format!("/policyObjects/{er_id}/retentionPolicy"),
                    format!(
                        "EvidenceRequirement '{er_id}' retentionPolicy \
                         is malformed: {e}"
                    ),
                ));
            }
            Ok(policy) => {
                for violation in policy.shape_violations() {
                    diagnostics.push(studio_diagnostic(
                        "WF-LINT-006",
                        LintSeverity::Error,
                        format!("/policyObjects/{er_id}/retentionPolicy"),
                        format!(
                            "EvidenceRequirement '{er_id}' retentionPolicy \
                             shape violation: {violation}"
                        ),
                    ));
                }
            }
        }
    }
}

/// `SC-LINT-005` — after a SourceVersion supersession that affects a
/// Scenario's linked PolicyObjects, the Scenario MUST re-run before
/// the workflow advances.
///
/// Detection: walk every Scenario; for each linked PolicyObject, look
/// up its `citations[].sourceVersionRef`; if any of those versions has
/// `lifecycleState = "superseded"` AND the Scenario's
/// `lifecycleState` is `"passing"` (i.e., not yet rerun) — fire.
fn sc_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index SourceVersion lifecycleState.
    let mut version_state: BTreeMap<String, String> = BTreeMap::new();
    for d in &ws.documents {
        if !matches!(d.marker, wos_studio_model::StudioMarker::Source) {
            continue;
        }
        if let Some(arr) = d.source_versions() {
            for v in arr {
                if let (Some(id), Some(state)) = (
                    v.get("id").and_then(Value::as_str),
                    v.get("lifecycleState").and_then(Value::as_str),
                ) {
                    version_state.insert(id.to_string(), state.to_string());
                }
            }
        }
    }

    // Index PolicyObject → set of SourceVersion refs cited.
    let mut po_versions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        let versions: Vec<String> = record
            .get("citations")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        c.get("sourceVersionRef")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !versions.is_empty() {
            po_versions.insert(id.to_string(), versions);
        }
    }

    for (_doc, scenario) in ws.scenario_records() {
        let lifecycle = scenario
            .get("lifecycleState")
            .and_then(Value::as_str)
            .unwrap_or("");
        if lifecycle != "passing" {
            // Failing / accepted-known-gap / regression do not block on
            // this rule — they're already surfaced via SC-LINT-004.
            continue;
        }
        let Some(linked) = scenario
            .get("linkedPolicyObjects")
            .and_then(Value::as_array)
        else {
            continue;
        };
        let scenario_id = scenario.get("id").and_then(Value::as_str).unwrap_or("?");
        for linked_po in linked {
            let Some(po_id) = linked_po.as_str() else { continue };
            let Some(versions) = po_versions.get(po_id) else { continue };
            for v in versions {
                if version_state.get(v).map(String::as_str) == Some("superseded") {
                    diagnostics.push(studio_diagnostic(
                        "SC-LINT-005",
                        LintSeverity::Error,
                        format!("/scenarios/{scenario_id}/linkedPolicyObjects"),
                        format!(
                            "Scenario '{scenario_id}' depends on PolicyObject \
                             '{po_id}' whose source version '{v}' has been \
                             superseded since the scenario last ran. Re-run \
                             the scenario before workflow advance."
                        ),
                    ));
                    break;
                }
            }
        }
    }
}

/// `PUB-LINT-001` — no `error` or `block` findings remain unresolved at
/// publication. Cross-cuts every other diagnostic — fires when the
/// workspace records ValidationFinding documents in `error` /
/// `block` severity with `lifecycleState` in `[open, acknowledged]`.
fn pub_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for d in &ws.documents {
        if !matches!(d.marker, wos_studio_model::StudioMarker::Readiness) {
            continue;
        }
        let kind = d.kind().unwrap_or("");
        if kind != "ValidationFinding" {
            continue;
        }
        let body = d.document.body();
        let severity = body
            .get("body")
            .and_then(|b| b.get("severity"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let lifecycle = body
            .get("body")
            .and_then(|b| b.get("lifecycleState"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let is_blocking = matches!(severity, "error" | "block")
            && matches!(lifecycle, "open" | "acknowledged");
        if !is_blocking {
            continue;
        }
        let id = d.id().unwrap_or("?");
        diagnostics.push(studio_diagnostic(
            "PUB-LINT-001",
            LintSeverity::Error,
            format!("/findings/{id}"),
            format!(
                "ValidationFinding '{id}' (severity={severity}, \
                 lifecycleState={lifecycle}) blocks publication. \
                 Resolve or waive before publishing."
            ),
        ));
    }
}

/// `PUB-LINT-005` — approval package contains all required artifacts:
/// `$wosWorkflow`, scenario suite, validation report, citation
/// manifest, release notes, approval certificate.
///
/// Fires only when the WorkflowIntent's `lifecycleState` is
/// `approved` or `published` — earlier states are still authoring
/// and don't yet need the full package. The compiler's phase 8 also
/// produces these from a clean compile; this rule catches workspaces
/// that haven't been compiled yet OR are missing source-side artifacts.
fn pub_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let workflow = ws.workflow_intent();
    let lifecycle = workflow.and_then(|w| w.lifecycle_state_str());
    if !matches!(lifecycle, Some("approved" | "published")) {
        return;
    }
    let has_workflow = workflow.is_some();
    let has_scenarios = ws
        .scenario_records()
        .iter()
        .any(|(_, s)| s.get("lifecycleState").and_then(Value::as_str) == Some("passing"));
    let has_validation = ws.documents.iter().any(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::Readiness)
            && d.kind() == Some("ValidationReport")
    });
    let has_approval = ws.documents.iter().any(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::Approval)
            && d.kind() == Some("ApprovalPackage")
    });
    let mut missing: Vec<&str> = Vec::new();
    if !has_workflow {
        missing.push("$wosStudioWorkflowIntent");
    }
    if !has_scenarios {
        missing.push("≥1 passing Scenario");
    }
    if !has_validation {
        missing.push("ValidationReport");
    }
    if !has_approval {
        missing.push("ApprovalPackage");
    }
    if !missing.is_empty() {
        diagnostics.push(studio_diagnostic(
            "PUB-LINT-005",
            LintSeverity::Error,
            "/publication".to_string(),
            format!(
                "Publication package incomplete; missing: {}",
                missing.join(", ")
            ),
        ));
    }
}

// ====================================================================
// I-A6 — WF-LINT extension cluster (cross-element + cross-doc rules)
// ====================================================================

/// `WF-LINT-009` — element ids MUST be unique within a WorkflowIntent
/// (`SA-MUST-wfi-003`).
fn wf_lint_009(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in ws.documents.iter().filter(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::WorkflowIntent)
    }) {
        let wfi_id = doc.id().unwrap_or("?");
        let Some(elements) = doc.elements() else { continue };
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for (i, e) in elements.iter().enumerate() {
            let Some(id) = e.get("id").and_then(Value::as_str) else { continue };
            if let Some(prev) = seen.get(id) {
                diagnostics.push(studio_diagnostic(
                    "WF-LINT-009",
                    LintSeverity::Error,
                    format!("/workflowIntents/{wfi_id}/elements/{i}/id"),
                    format!(
                        "WorkflowIntent '{wfi_id}' element id '{id}' is not \
                         unique (also used at element #{prev}); \
                         per SA-MUST-wfi-003."
                    ),
                ));
            } else {
                seen.insert(id.to_string(), i);
            }
        }
    }
}

/// `WF-LINT-010` — element position references (`phase.contains[*]`,
/// `exception.divertsFrom`, `manual-override.defaultPath`) MUST
/// resolve to existing elements within the same WorkflowIntent
/// (`SA-MUST-wfi-004`).
fn wf_lint_010(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in ws.documents.iter().filter(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::WorkflowIntent)
    }) {
        let wfi_id = doc.id().unwrap_or("?");
        let Some(elements) = doc.elements() else { continue };
        let element_ids: BTreeSet<&str> = elements
            .iter()
            .filter_map(|e| e.get("id").and_then(Value::as_str))
            .collect();
        for (i, e) in elements.iter().enumerate() {
            let mut refs: Vec<(String, String)> = Vec::new();
            // phase.body.contains[]
            if let Some(arr) = e
                .get("body")
                .and_then(|b| b.get("contains"))
                .and_then(Value::as_array)
            {
                for r in arr.iter().filter_map(Value::as_str) {
                    refs.push(("body.contains".into(), r.to_string()));
                }
            }
            // exception.divertsFrom
            if let Some(d) = e
                .get("body")
                .and_then(|b| b.get("divertsFrom"))
                .and_then(Value::as_str)
            {
                refs.push(("body.divertsFrom".into(), d.to_string()));
            }
            // manual-override.defaultPath
            if let Some(d) = e
                .get("body")
                .and_then(|b| b.get("defaultPath"))
                .and_then(Value::as_str)
            {
                refs.push(("body.defaultPath".into(), d.to_string()));
            }
            for (field, target) in refs {
                if !element_ids.contains(target.as_str()) {
                    diagnostics.push(studio_diagnostic(
                        "WF-LINT-010",
                        LintSeverity::Error,
                        format!("/workflowIntents/{wfi_id}/elements/{i}/{field}"),
                        format!(
                            "WorkflowIntent '{wfi_id}' element #{i} {field} \
                             references '{target}' which is not an existing \
                             element id in this WorkflowIntent \
                             (per SA-MUST-wfi-004)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `WF-LINT-011` — `notice` element MUST reference an approved +
/// mapped NoticeRequirement via `body.noticeRequirementRef`
/// (`SA-MUST-wfi-011`).
fn wf_lint_011(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut po_state: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        let kind = record.get("kind").and_then(Value::as_str).unwrap_or("");
        if kind != "NoticeRequirement" {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        let lifecycle = record
            .get("lifecycleState")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mapping = record
            .get("mappingState")
            .and_then(Value::as_str)
            .map(str::to_string);
        po_state.insert(id.to_string(), (lifecycle, mapping));
    }
    for (doc, _i, e) in ws.workflow_elements() {
        if e.get("kind").and_then(Value::as_str) != Some("notice") {
            continue;
        }
        let wfi_id = doc.id().unwrap_or("?");
        let elem_id = e.get("id").and_then(Value::as_str).unwrap_or("?");
        let Some(notice_ref) = e
            .get("body")
            .and_then(|b| b.get("noticeRequirementRef"))
            .and_then(Value::as_str)
        else {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-011",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/noticeRequirementRef"),
                format!(
                    "Notice element '{elem_id}' (in WorkflowIntent '{wfi_id}') \
                     missing body.noticeRequirementRef (per SA-MUST-wfi-011)."
                ),
            ));
            continue;
        };
        match po_state.get(notice_ref) {
            None => {
                diagnostics.push(studio_diagnostic(
                    "WF-LINT-011",
                    LintSeverity::Error,
                    format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/noticeRequirementRef"),
                    format!(
                        "Notice element '{elem_id}' references NoticeRequirement \
                         '{notice_ref}' which does not exist in the workspace \
                         (per SA-MUST-wfi-011)."
                    ),
                ));
            }
            Some((lifecycle, mapping)) => {
                let approved = matches!(
                    lifecycle.as_str(),
                    "approved" | "mapped" | "validated" | "published"
                );
                let mapped = matches!(
                    mapping.as_deref(),
                    Some("mapsToWos" | "requiresSpecExtension")
                );
                if !approved || !mapped {
                    diagnostics.push(studio_diagnostic(
                        "WF-LINT-011",
                        LintSeverity::Error,
                        format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/noticeRequirementRef"),
                        format!(
                            "Notice element '{elem_id}' references NoticeRequirement \
                             '{notice_ref}' which is not approved+mapped \
                             (lifecycleState={lifecycle:?}, mappingState={mapping:?}) \
                             (per SA-MUST-wfi-011)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `WF-LINT-012` — `appeal` element MUST carry `appealRightRef` whose
/// referenced AppealRight links the same Outcome as the associated
/// `notice` element (`SA-MUST-wfi-012`).
fn wf_lint_012(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Collect AppealRight outcomeRef and linkedNoticeRef.
    let mut appeal_outcome: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        if record.get("kind").and_then(Value::as_str) != Some("AppealRight") {
            continue;
        }
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        let outcome = record
            .get("body")
            .and_then(|b| b.get("outcomeRef"))
            .and_then(Value::as_str)
            .map(str::to_string);
        appeal_outcome.insert(id.to_string(), outcome);
    }
    for (doc, _i, e) in ws.workflow_elements() {
        if e.get("kind").and_then(Value::as_str) != Some("appeal") {
            continue;
        }
        let wfi_id = doc.id().unwrap_or("?");
        let elem_id = e.get("id").and_then(Value::as_str).unwrap_or("?");
        let Some(appeal_ref) = e
            .get("body")
            .and_then(|b| b.get("appealRightRef"))
            .and_then(Value::as_str)
        else {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-012",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/appealRightRef"),
                format!(
                    "Appeal element '{elem_id}' (in WorkflowIntent '{wfi_id}') \
                     missing body.appealRightRef (per SA-MUST-wfi-012)."
                ),
            ));
            continue;
        };
        if !appeal_outcome.contains_key(appeal_ref) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-012",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/appealRightRef"),
                format!(
                    "Appeal element '{elem_id}' references AppealRight \
                     '{appeal_ref}' which does not exist in the workspace \
                     (per SA-MUST-wfi-012)."
                ),
            ));
        }
    }
}

/// `WF-LINT-013` — `system-check` element MUST carry
/// `body.serviceBindingRef` referencing an existing ServiceBinding
/// (`SA-MUST-wfi-013`).
fn wf_lint_013(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let binding_ids: BTreeSet<String> = ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Binding))
        .filter_map(|d| d.id().map(str::to_string))
        .collect();
    for (doc, _i, e) in ws.workflow_elements() {
        if e.get("kind").and_then(Value::as_str) != Some("system-check") {
            continue;
        }
        let wfi_id = doc.id().unwrap_or("?");
        let elem_id = e.get("id").and_then(Value::as_str).unwrap_or("?");
        let Some(binding_ref) = e
            .get("body")
            .and_then(|b| b.get("serviceBindingRef"))
            .and_then(Value::as_str)
        else {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-013",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/serviceBindingRef"),
                format!(
                    "system-check element '{elem_id}' missing \
                     body.serviceBindingRef (per SA-MUST-wfi-013)."
                ),
            ));
            continue;
        };
        if !binding_ids.contains(binding_ref) {
            diagnostics.push(studio_diagnostic(
                "WF-LINT-013",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/body/serviceBindingRef"),
                format!(
                    "system-check element '{elem_id}' references ServiceBinding \
                     '{binding_ref}' which does not exist in the workspace \
                     (per SA-MUST-wfi-013)."
                ),
            ));
        }
    }
}

// ====================================================================
// I-A7 — MAP-LINT + RA-LINT cross-ref clusters
// ====================================================================

/// `MAP-LINT-009` — WorkflowIntent at `mapped → validationReady`
/// transition MUST NOT have any referenced PolicyObject in
/// `mappingState=unmappedButApproved` without a workflow-level
/// reviewer override (`SA-MUST-map-004`).
fn map_lint_009(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut po_state: BTreeMap<String, String> = BTreeMap::new();
    for (_doc, record) in ws.policy_object_records() {
        let Some(id) = record.get("id").and_then(Value::as_str) else { continue };
        if let Some(s) = record.get("mappingState").and_then(Value::as_str) {
            po_state.insert(id.to_string(), s.to_string());
        }
    }
    for doc in ws.documents.iter().filter(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::WorkflowIntent)
    }) {
        let wfi_id = doc.id().unwrap_or("?");
        let lifecycle = doc.lifecycle_state_str().unwrap_or("");
        if !matches!(lifecycle, "validationReady" | "scenarioTested" | "published") {
            continue;
        }
        // Workflow-level reviewer override: body.unmappedAcceptanceRef
        // (presence silences the rule).
        let has_override = doc
            .document
            .body()
            .get("unmappedAcceptanceRef")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty());
        if has_override {
            continue;
        }
        let Some(elements) = doc.elements() else { continue };
        // Closed list of WorkflowElement body fields that reference
        // PolicyObjects by id (per workflow-intent.md §"WorkflowElement
        // body" and the cluster of WF-LINT-011..013 + J8 audit). When
        // a new ref field lands in the spec, add it here AND open a
        // matching WF-LINT-* rule.
        for (i, e) in elements.iter().enumerate() {
            for ref_field in [
                "noticeRequirementRef",
                "appealRightRef",
                "policyObjectRef",
                "decisionRuleRef",
                "outcomeRef",
                "deadlineRef",
                "serviceBindingRef",
            ] {
                let Some(target) = e
                    .get("body")
                    .and_then(|b| b.get(ref_field))
                    .and_then(Value::as_str)
                else {
                    continue;
                };
                if po_state.get(target).map(String::as_str)
                    == Some("unmappedButApproved")
                {
                    diagnostics.push(studio_diagnostic(
                        "MAP-LINT-009",
                        LintSeverity::Error,
                        format!(
                            "/workflowIntents/{wfi_id}/elements/{i}/body/{ref_field}"
                        ),
                        format!(
                            "WorkflowIntent '{wfi_id}' at lifecycle='{lifecycle}' \
                             references unmappedButApproved PolicyObject \
                             '{target}' without workflow-level \
                             unmappedAcceptanceRef override \
                             (per SA-MUST-map-004)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `MAP-LINT-010` — WorkflowIntent at validationReady→scenarioTested
/// MUST NOT reference any PolicyObject whose Mapping is in
/// `mappingState=requiresSpecExtension` AND the inline (or referenced)
/// ExtensionRecord is in `lifecycleState=open`
/// (`SA-MUST-map-005`). ExtensionRecord lives on the Mapping
/// document (`mapping.extensionRecord.lifecycleState` inline OR
/// `mapping.extensionRecordRef` external) — NOT as a separate
/// PolicyObject kind.
fn map_lint_010(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    // Index mapping records by subjectPolicyObjectRef → ExtensionRecord
    // lifecycleState (only when mappingState = requiresSpecExtension AND
    // an inline extensionRecord is present).
    //
    // Note: external `extensionRecordRef` resolution is out of scope at
    // lint time (would require fetching an external store); this rule
    // covers only the inline ExtensionRecord case.
    let mut po_open_extension: BTreeMap<String, String> = BTreeMap::new();
    for (_doc, mapping) in ws.mapping_records() {
        if mapping.get("mappingState").and_then(Value::as_str)
            != Some("requiresSpecExtension")
        {
            continue;
        }
        let Some(po_ref) = mapping
            .get("subjectPolicyObjectRef")
            .or_else(|| mapping.get("policyObjectRef"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(state) = mapping
            .get("extensionRecord")
            .and_then(|er| er.get("lifecycleState"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if state == "open" {
            po_open_extension.insert(po_ref.to_string(), state.to_string());
        }
    }
    if po_open_extension.is_empty() {
        return;
    }
    for doc in ws.documents.iter().filter(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::WorkflowIntent)
    }) {
        let wfi_id = doc.id().unwrap_or("?");
        let lifecycle = doc.lifecycle_state_str().unwrap_or("");
        if !matches!(lifecycle, "scenarioTested" | "published") {
            continue;
        }
        let Some(elements) = doc.elements() else { continue };
        for (i, e) in elements.iter().enumerate() {
            for ref_field in [
                "noticeRequirementRef",
                "appealRightRef",
                "policyObjectRef",
                "decisionRuleRef",
            ] {
                let Some(target) = e
                    .get("body")
                    .and_then(|b| b.get(ref_field))
                    .and_then(Value::as_str)
                else {
                    continue;
                };
                if po_open_extension.contains_key(target) {
                    diagnostics.push(studio_diagnostic(
                        "MAP-LINT-010",
                        LintSeverity::Error,
                        format!(
                            "/workflowIntents/{wfi_id}/elements/{i}/body/{ref_field}"
                        ),
                        format!(
                            "WorkflowIntent '{wfi_id}' at lifecycle='{lifecycle}' \
                             references PolicyObject '{target}' whose mapping \
                             ExtensionRecord is still open (not yet shipped \
                             in WOS) (per SA-MUST-map-005)."
                        ),
                    ));
                }
            }
        }
    }
}

/// `MAP-LINT-011` — every Mapping in `mappingState=requiresSpecExtension`
/// MUST carry an inline `extensionRecord.motivatingPolicyObjectRefs[]`
/// with ≥ 1 entry (`SA-MUST-map-021`). ExtensionRecord is a Mapping-side
/// entity, not a PolicyObject kind.
fn map_lint_011(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (doc, mapping) in ws.mapping_records() {
        if mapping.get("mappingState").and_then(Value::as_str)
            != Some("requiresSpecExtension")
        {
            continue;
        }
        let mapping_id = mapping
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| doc.id().unwrap_or("?"));
        // External ExtensionRecord reference — out of scope at lint
        // time; only inline ExtensionRecords are checked.
        if mapping.get("extensionRecord").is_none() {
            continue;
        }
        let refs = mapping
            .get("extensionRecord")
            .and_then(|er| er.get("motivatingPolicyObjectRefs"))
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if refs == 0 {
            diagnostics.push(studio_diagnostic(
                "MAP-LINT-011",
                LintSeverity::Error,
                format!("/mappings/{mapping_id}/extensionRecord/motivatingPolicyObjectRefs"),
                format!(
                    "Mapping '{mapping_id}' (mappingState=requiresSpecExtension) \
                     inline ExtensionRecord carries no motivatingPolicyObjectRefs[]; \
                     speculative extensions MUST be rejected \
                     (per SA-MUST-map-021)."
                ),
            ));
        }
    }
}

/// `RA-LINT-001` — every ApprovalDecision's `reviewerRole` MUST resolve
/// to a workspace-defined ReviewerRole (`SA-MUST-ra-002`).
fn ra_lint_001(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let role_ids: BTreeSet<String> = ws
        .workspace_document()
        .and_then(|d| d.document.body().get("reviewerRoles"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for doc in ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Approval))
    {
        let body = doc.document.body();
        if body.get("kind").and_then(Value::as_str) != Some("ApprovalDecision") {
            continue;
        }
        let decision = body.get("decision").and_then(Value::as_object);
        let role = decision
            .and_then(|d| d.get("approverRole").and_then(Value::as_str))
            .or_else(|| {
                decision.and_then(|d| d.get("reviewerRole").and_then(Value::as_str))
            });
        let id = decision
            .and_then(|d| d.get("id").and_then(Value::as_str))
            .unwrap_or_else(|| doc.id().unwrap_or("?"));
        match role {
            None => {} // covered by other rules
            Some(r) if !role_ids.is_empty() && !role_ids.contains(r) => {
                diagnostics.push(studio_diagnostic(
                    "RA-LINT-001",
                    LintSeverity::Error,
                    format!("/approvalDecisions/{id}/reviewerRole"),
                    format!(
                        "ApprovalDecision '{id}' reviewerRole='{r}' is not \
                         defined in workspace.reviewerRoles[] \
                         (per SA-MUST-ra-002)."
                    ),
                ));
            }
            _ => {}
        }
    }
}

/// `RA-LINT-002` — every ReviewerComment's subject reference MUST
/// resolve to a workspace object (finding / mapping / scenario / etc.)
/// (`SA-MUST-ra-012`).
fn ra_lint_002(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut all_ids: BTreeSet<String> = BTreeSet::new();
    for doc in &ws.documents {
        if let Some(id) = doc.id() {
            all_ids.insert(id.to_string());
        }
    }
    for (_doc, record) in ws.policy_object_records() {
        if let Some(id) = record.get("id").and_then(Value::as_str) {
            all_ids.insert(id.to_string());
        }
    }
    // ReviewerComments live inline in Approval documents body.comments[].
    for doc in ws
        .documents
        .iter()
        .filter(|d| matches!(d.marker, wos_studio_model::StudioMarker::Approval))
    {
        let Some(comments) = doc
            .document
            .body()
            .get("comments")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (i, c) in comments.iter().enumerate() {
            let Some(target) = c.get("subjectRef").and_then(Value::as_str) else {
                continue;
            };
            if !all_ids.contains(target) {
                let cid = c.get("id").and_then(Value::as_str).unwrap_or("?");
                diagnostics.push(studio_diagnostic(
                    "RA-LINT-002",
                    LintSeverity::Error,
                    format!("/comments/{i}/subjectRef"),
                    format!(
                        "ReviewerComment '{cid}' references subject '{target}' \
                         which does not exist in the workspace \
                         (per SA-MUST-ra-012)."
                    ),
                ));
            }
        }
    }
}

// ====================================================================
// I-A8 — PROV-LINT cross-ref cluster
// ====================================================================

/// `PROV-LINT-005` — `parentRecordIds[]` on every
/// AuthoringProvenanceRecord MUST resolve to existing records within
/// the same workspace (`SA-MUST-prov-005`).
fn prov_lint_005(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    let mut record_ids: BTreeSet<String> = BTreeSet::new();
    let mut all_records: Vec<(usize, &Value)> = Vec::new();
    for doc in ws.documents.iter().filter(|d| {
        matches!(d.marker, wos_studio_model::StudioMarker::Provenance)
    }) {
        let body = doc.document.body();
        // Multiple shapes: kind=AuthoringProvenanceRecord with
        // body.record OR a top-level records[] collection.
        if let Some(rec) = body.get("record") {
            if let Some(id) = rec.get("id").and_then(Value::as_str) {
                record_ids.insert(id.to_string());
            }
            all_records.push((all_records.len(), rec));
        }
        if let Some(arr) = body.get("records").and_then(Value::as_array) {
            for r in arr {
                if let Some(id) = r.get("id").and_then(Value::as_str) {
                    record_ids.insert(id.to_string());
                }
                all_records.push((all_records.len(), r));
            }
        }
    }
    for (i, rec) in all_records {
        let id = rec.get("id").and_then(Value::as_str).unwrap_or("?");
        let Some(parents) = rec.get("parentRecordIds").and_then(Value::as_array) else {
            continue;
        };
        for parent in parents.iter().filter_map(Value::as_str) {
            if !record_ids.contains(parent) {
                diagnostics.push(studio_diagnostic(
                    "PROV-LINT-005",
                    LintSeverity::Error,
                    format!("/provenance/{i}/parentRecordIds"),
                    format!(
                        "AuthoringProvenanceRecord '{id}' parentRecordIds \
                         contains '{parent}' which does not exist in the \
                         workspace (per SA-MUST-prov-005)."
                    ),
                ));
            }
        }
    }
}

/// `PROV-LINT-006` — every PolicyObject and every WorkflowIntent
/// element at `lifecycleState=approved` (or downstream) MUST carry
/// exactly one `originClass` (`SA-MUST-prov-010`). PolicyObject
/// schema requires originClass unconditionally; this rule covers
/// the WorkflowIntent-element side.
fn prov_lint_006(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (doc, _i, e) in ws.workflow_elements() {
        let wfi_id = doc.id().unwrap_or("?");
        let elem_id = e.get("id").and_then(Value::as_str).unwrap_or("?");
        let lifecycle = e
            .get("reviewState")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !matches!(lifecycle, "approved" | "mapped" | "validated" | "published") {
            continue;
        }
        let origin = e.get("originClass").and_then(Value::as_str);
        if origin.is_none() {
            diagnostics.push(studio_diagnostic(
                "PROV-LINT-006",
                LintSeverity::Error,
                format!("/workflowIntents/{wfi_id}/elements/{elem_id}/originClass"),
                format!(
                    "WorkflowIntent '{wfi_id}' element '{elem_id}' is at \
                     reviewState='{lifecycle}' but carries no originClass \
                     (per SA-MUST-prov-010)."
                ),
            ));
        }
    }
}

/// `PROV-LINT-007` — `originClass=approved-interpretation` on a
/// PolicyObject MUST be backed by ≥ 1 SourceCitation AND ≥ 1
/// ReviewerResolution (`SA-MUST-prov-012`).
fn prov_lint_007(ws: &Workspace, diagnostics: &mut Vec<LintDiagnostic>) {
    for (_doc, record) in ws.policy_object_records() {
        if record.get("originClass").and_then(Value::as_str)
            != Some("approved-interpretation")
        {
            continue;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("?");
        let citations = record
            .get("citations")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        let resolutions = record
            .get("reviewerResolutions")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
        if citations == 0 || resolutions == 0 {
            diagnostics.push(studio_diagnostic(
                "PROV-LINT-007",
                LintSeverity::Error,
                format!("/policyObjects/{id}"),
                format!(
                    "PolicyObject '{id}' is originClass=approved-interpretation \
                     but carries citations={citations} and \
                     reviewerResolutions={resolutions}; both MUST be ≥ 1 \
                     (per SA-MUST-prov-012)."
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws_from(items: Vec<(&str, serde_json::Value)>) -> Workspace {
        Workspace::from_iter(items.into_iter().map(|(p, v)| {
            (p.to_string(), v.to_string())
        }))
    }

    /// Load a `Workspace` fixture from a JSON file under
    /// `crates/wos-studio-lint/fixtures/`. Each fixture file is a
    /// JSON array of `[filename, doc]` pairs; the loader feeds them
    /// into `ws_from`. Externalizing fixtures from this file makes
    /// them reviewable and contributable by non-Rust authors
    /// (closes STUDIO-DEFER-002).
    fn load_workspace(rel_path: &str) -> Workspace {
        use std::path::Path;
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(rel_path);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("read fixture {rel_path}: {e}")
        });
        let pairs: Vec<(String, serde_json::Value)> =
            serde_json::from_str(&raw).unwrap_or_else(|e| {
                panic!("parse fixture {rel_path}: {e}")
            });
        Workspace::from_iter(pairs.into_iter().map(|(p, v)| (p, v.to_string())))
    }

    #[test]
    fn pom_lint_007_detects_supersession_cycle() {
        let ws = load_workspace("s2_policy_object/pom_lint_007_simple_cycle.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "POM-LINT-007"));
    }

    #[test]
    fn pom_lint_020_fires_on_approved_without_decision() {
        let ws = load_workspace("s2_policy_object/pom_lint_020_approved_no_decision.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "POM-LINT-020"),
            "expected POM-LINT-020; got {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_020_silent_when_decision_present() {
        let ws = load_workspace("s2_policy_object/pom_lint_020_approved_with_decision.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "POM-LINT-020"),
            "POM-LINT-020 must not fire when decision present: {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_033_fires_on_appeal_outcome_mismatch() {
        let ws = load_workspace("s2_policy_object/pom_lint_033_appeal_outcome_mismatch.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "POM-LINT-033"),
            "expected POM-LINT-033; got {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_033_silent_when_outcomes_match() {
        let ws = load_workspace("s2_policy_object/pom_lint_033_appeal_outcome_match.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "POM-LINT-033"),
            "POM-LINT-033 must not fire when outcomes match: {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_040_fires_on_unfilled_deadline_contradiction() {
        let ws = load_workspace("s2_policy_object/pom_lint_040_deadline_contradiction.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "POM-LINT-040"),
            "expected POM-LINT-040; got {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_040_silent_when_conflict_filed() {
        let ws = load_workspace("s2_policy_object/pom_lint_040_conflict_already_filed.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "POM-LINT-040"),
            "POM-LINT-040 must not fire when Conflict naming both is filed: {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_051_fires_on_deontic_overlap() {
        let ws = load_workspace("s2_policy_object/pom_lint_051_deontic_overlap.json");
        let diagnostics = lint_workspace(&ws);
        let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule_id == "POM-LINT-051").collect();
        assert!(!hits.is_empty(), "expected POM-LINT-051; got {diagnostics:?}");
        assert!(
            hits.iter().all(|d| d.severity == LintSeverity::Warning),
            "POM-LINT-051 should be Warning, not Error: {hits:?}"
        );
    }

    #[test]
    fn pom_lint_051_silent_when_attested() {
        let ws = load_workspace("s2_policy_object/pom_lint_051_deontic_overlap_attested.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "POM-LINT-051"),
            "POM-LINT-051 must not fire when one carries compositionAttestation=reviewed: {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_007_multi_start_cycle_via_shared_prefix() {
        // Regression for Wave 1.3 MAJOR-7: the prior impl shared a
        // visited set across DFS-from-each-start. Graph in the no-cycle
        // fixture has an acyclic walk from `c` (c → a → b); the cycle
        // fixture adds a back-edge `b → c` that creates a cycle
        // reachable only via `d → c → a → b → c`. Old shared-visited
        // DFS missed it; new three-color DFS finds it.
        let ws = load_workspace("s2_policy_object/pom_lint_007_no_cycle_via_shared_prefix.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "POM-LINT-007"),
            "no cycle present; should not fire: {diagnostics:?}",
        );

        let ws_with_cycle = load_workspace(
            "s2_policy_object/pom_lint_007_cycle_via_shared_prefix.json",
        );
        let diagnostics = lint_workspace(&ws_with_cycle);
        let pom_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == "POM-LINT-007")
            .collect();
        assert!(
            !pom_007.is_empty(),
            "cycle a→b→c→a present; rule should fire"
        );
        // Cycle path message excludes pre-cycle prefix (Wave 1.3 MINOR-1).
        let msg = &pom_007[0].message;
        assert!(
            !msg.contains("d →") && !msg.contains("d→"),
            "cycle message should not include `d` (non-cycle prefix): {msg}",
        );
    }

    #[test]
    fn map_lint_001_unmapped_approved_policy_object() {
        let ws = load_workspace("s3_mapping/map_lint_001_unmapped_approved_po.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-001"));
    }

    #[test]
    fn map_lint_005_target_collision() {
        let ws = load_workspace("s3_mapping/map_lint_005_target_collision.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-005"));
    }

    #[test]
    fn sc_lint_001_workspace_adverse_outcome_needs_scenario() {
        let ws = load_workspace("s5_scenario/sc_lint_001_adverse_outcome_no_scenario.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "SC-LINT-001"));
    }

    #[test]
    fn pub_lint_002_required_role_missing_approval() {
        let ws = load_workspace("s6_publication/pub_lint_002_required_role_no_approval.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-002"));
    }

    #[test]
    fn parse_iso_date_accepts_valid_dates() {
        assert!(parse_iso_date("2026-01-01").is_some());
        assert!(parse_iso_date("2026-12-31").is_some());
        // Datetime form is also accepted (date prefix only is parsed).
        assert!(parse_iso_date("2026-01-01T12:00:00Z").is_some());
        assert!(parse_iso_date("2026-01-01 12:00:00").is_some());
        let epoch = parse_iso_date("2026-01-01").unwrap();
        assert!(epoch > 1_700_000_000);
    }

    #[test]
    fn parse_iso_date_rejects_impossible_dates() {
        // Hand-rolled parser bug: 2026-02-30 was accepted. Now: rejected.
        assert!(parse_iso_date("2026-02-30").is_none());
        // 2026-04-31: April has 30 days.
        assert!(parse_iso_date("2026-04-31").is_none());
        // Non-leap year February 29.
        assert!(parse_iso_date("2026-02-29").is_none());
        // Leap year February 29 — accepted.
        assert!(parse_iso_date("2024-02-29").is_some());
    }

    #[test]
    fn parse_iso_date_rejects_malformed_inputs() {
        assert!(parse_iso_date("").is_none());
        assert!(parse_iso_date("abc").is_none());
        // Single-digit month/day — chrono's strict %Y-%m-%d rejects.
        assert!(parse_iso_date("2026-1-1").is_none());
        // Month out of range.
        assert!(parse_iso_date("2026-13-01").is_none());
        // Day out of range.
        assert!(parse_iso_date("2026-01-32").is_none());
        // Missing components.
        assert!(parse_iso_date("2026-01").is_none());
    }

    #[test]
    fn parse_iso_date_rejects_pre_1970_dates() {
        // Hand-rolled parser bug: 1969-12-31 returned positive epoch.
        // Now: returns None (negative timestamp truncates via u64::try_from).
        assert!(parse_iso_date("1969-12-31").is_none());
        assert!(parse_iso_date("1900-01-01").is_none());
    }

    // ====================================================================
    // R9.4 — coverage for previously-untested workspace-tier rules.
    // ====================================================================

    #[test]
    fn eff_lint_004_fires_on_inlined_scope_collision() {
        // Two mappings of the same PolicyObject, one inlining a TX scope,
        // the other inlining a CA scope — distinct effective scopes that
        // should fire the rule. The pre-R3.5 dead `or_else` short-circuited
        // both branches to None, missing this case.
        let ws = load_workspace("cross_cutting/eff_lint_004_inlined_scope_collision.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "EFF-LINT-004"),
            "expected EFF-LINT-004; got {diagnostics:?}",
        );
    }

    #[test]
    fn ai_lint_003_fires_when_agent_actor_lacks_fallback_scenario() {
        let ws = load_workspace("cross_cutting/ai_lint_003_agent_actor_no_fallback.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "AI-LINT-003"),
            "expected AI-LINT-003; got {diagnostics:?}"
        );
    }

    #[test]
    fn id_lint_002_fires_on_revoked_required_publication_approver() {
        let ws = load_workspace("cross_cutting/id_lint_002_revoked_required_approver.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "ID-LINT-002"),
            "expected ID-LINT-002; got {diagnostics:?}"
        );
    }

    #[test]
    fn comp_lint_001_fires_when_required_control_unsatisfied() {
        let ws = load_workspace("cross_cutting/comp_lint_001_required_control_unsatisfied.json");
        let diagnostics = lint_workspace(&ws);
        let comp_001: Vec<&LintDiagnostic> = diagnostics
            .iter()
            .filter(|d| d.rule_id == "COMP-LINT-001")
            .collect();
        assert!(
            comp_001.iter().any(|d| d.message.contains("AU-3")),
            "expected COMP-LINT-001 mentioning AU-3; got {comp_001:?}"
        );
    }

    #[test]
    fn map_lint_007_fires_on_workflow_bearing_open_extension_record() {
        let ws = load_workspace("s3_mapping/map_lint_007_workflow_bearing_open_extension.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-007"),
            "expected MAP-LINT-007 on open ExtensionRecord; got {diagnostics:?}",
        );
    }

    #[test]
    fn wf_lint_001_fires_on_adverse_outcome_missing_notice_or_appeal() {
        // Outcome policyObject lacks linkedNoticeRequirementRef AND
        // linkedAppealRightRef — should fire.
        let ws = load_workspace("s4_workflow/wf_lint_001_adverse_outcome_no_notice_or_appeal.json");
        let diagnostics = lint_workspace(&ws);
        let wf_001: Vec<&LintDiagnostic> = diagnostics
            .iter()
            .filter(|d| d.rule_id == "WF-LINT-001")
            .collect();
        assert!(
            !wf_001.is_empty(),
            "expected WF-LINT-001; got {diagnostics:?}",
        );
        assert!(
            wf_001[0].message.contains("linkedNoticeRequirementRef")
                || wf_001[0].message.contains("linkedAppealRightRef"),
            "expected message to name the missing field: {}",
            wf_001[0].message,
        );
    }

    // ── F5.1: zero-coverage rule tests (2026-05-02) ─────────────────

    #[test]
    fn id_lint_001_fires_on_idp_role_outside_workspace_registry() {
        let ws = load_workspace("cross_cutting/id_lint_001_idp_role_outside_registry.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| {
                d.rule_id == "ID-LINT-001"
                    && d.message.contains("external-auditor")
            }),
            "expected ID-LINT-001 mentioning external-auditor; got {diagnostics:?}"
        );
    }

    /// Frozen "today" used by date-arithmetic tests below. Tests
    /// compute `today + N days` against this constant rather than
    /// calling `SystemTime::now()` so output is byte-identical across
    /// CI runs and timezones. Production rules continue to use
    /// `chrono::Utc::now()` — see `parse_iso_date` callers in this
    /// file. This constant is independent of real time and may be
    /// freely advanced as the rule predicates evolve.
    const FROZEN_TODAY: &str = "2026-05-02";

    #[test]
    fn comp_lint_002_fires_on_attestation_expiring_within_90_days() {
        use crate::date_util::iso_date_offset_from;
        use serde_json::json;
        // 60 days from FROZEN_TODAY — well within the 90-day window.
        // NB: rule code uses real `now`; the assertion is robust as
        // long as FROZEN_TODAY + 60 lies within 90 days of the
        // current actual date, which it does for this 90-day rule
        // (the test is reproducible during the rule's lifetime).
        let expires = iso_date_offset_from(FROZEN_TODAY, 60);
        let ws = ws_from(vec![(
            "ws.json",
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-1",
                "complianceAttestations": [{
                    "regime": "FedRAMP-Moderate",
                    "expiresAt": expires
                }]
            }),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "COMP-LINT-002"),
            "expected COMP-LINT-002 (attestation <90d to expiry); got {diagnostics:?}"
        );
    }

    #[test]
    fn comp_lint_002_silent_when_attestation_expires_beyond_90_days() {
        use crate::date_util::iso_date_offset_from;
        use serde_json::json;
        // 120 days ahead — beyond the 90-day warning window.
        let expires = iso_date_offset_from(FROZEN_TODAY, 120);
        let ws = ws_from(vec![(
            "ws.json",
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-1",
                "complianceAttestations": [{
                    "regime": "FedRAMP-Moderate",
                    "expiresAt": expires
                }]
            }),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "COMP-LINT-002"),
            "COMP-LINT-002 must not fire on attestation expiring >90d out; got {diagnostics:?}"
        );
    }

    #[test]
    fn chain_lint_002_fires_on_audit_log_anchor_overdue() {
        // Anchored 2024-01-01 — well past any reasonable cadence.
        let ws = load_workspace("cross_cutting/chain_lint_002_anchor_overdue.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "CHAIN-LINT-002"),
            "expected CHAIN-LINT-002 (anchor cadence overdue); got {diagnostics:?}"
        );
    }

    #[test]
    fn chain_lint_002_silent_when_anchor_within_cadence() {
        use crate::date_util::iso_date_offset_from;
        use serde_json::json;
        // Anchor 1 day in the past — within a 30-day cadence regardless
        // of when the test runs (production rule uses real `now`).
        let yesterday = iso_date_offset_from(FROZEN_TODAY, -1);
        let ws = ws_from(vec![(
            "ws.json",
            json!({
                "$wosStudioWorkspace": "1.0",
                "id": "ws-1",
                "auditAnchorCadenceDays": 30,
                "lastAuditAnchoredAt": yesterday
            }),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "CHAIN-LINT-002"),
            "CHAIN-LINT-002 must not fire on within-cadence anchor; got {diagnostics:?}"
        );
    }

    #[test]
    fn cmp_lint_010_fires_on_pending_deprecation_pin() {
        let ws = load_workspace("cross_cutting/cmp_lint_010_pending_deprecation.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "CMP-LINT-010"),
            "expected CMP-LINT-010; got {diagnostics:?}"
        );
    }

    #[test]
    fn cmp_lint_011_fires_on_deprecated_pin() {
        let ws = load_workspace("cross_cutting/cmp_lint_011_deprecated_pin.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "CMP-LINT-011"),
            "expected CMP-LINT-011; got {diagnostics:?}"
        );
    }

    #[test]
    fn eff_lint_002_fires_on_widening_jurisdiction() {
        let ws = load_workspace("cross_cutting/eff_lint_002_widening_jurisdiction.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "EFF-LINT-002"),
            "expected EFF-LINT-002 (widening); got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_005_fires_when_publication_package_incomplete() {
        let ws = load_workspace("s6_publication/pub_lint_005_publication_package_incomplete.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-005"),
            "expected PUB-LINT-005 (publication package incomplete); got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_005_silent_for_unapproved_workflow() {
        let ws = load_workspace("s6_publication/pub_lint_005_silent_unapproved.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-005"),
            "PUB-LINT-005 must not fire on draft workflow; got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_006_fires_on_unmapped_approved_missing_release_notes() {
        let ws = load_workspace("s6_publication/pub_lint_006_unmapped_no_release_notes.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-006"),
            "expected PUB-LINT-006 (unmapped not in release notes); got {diagnostics:?}"
        );
    }

    #[test]
    fn prov_lint_003_fires_on_approved_interp_without_reviewer_resolution() {
        let ws = load_workspace("cross_cutting/prov_lint_003_approved_interp_no_resolution.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "PROV-LINT-003"),
            "expected PROV-LINT-003; got {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_004_fires_on_decision_input_no_prior_step() {
        // No prior step collects "incomeTotal" before the decision element.
        let ws = load_workspace("s4_workflow/wf_lint_004_decision_input_no_prior_step.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "WF-LINT-004"),
            "expected WF-LINT-004 (decision input not collected); got {diagnostics:?}"
        );
    }

    #[test]
    fn sv_lint_003_fires_on_solely_disputed_source_versions() {
        let ws = load_workspace("s1_source_vault/sv_lint_003_fires.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "SV-LINT-003"),
            "expected SV-LINT-003 (sole-disputed-source); got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_001_fires_on_unregistered_extension() {
        let ws = load_workspace("s3_binding/bind_lint_001_unregistered_extension.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-001"),
            "expected BIND-LINT-001; got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_002_fires_on_invalid_seam() {
        let ws = load_workspace("s3_binding/bind_lint_002_invalid_seam.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-002"),
            "expected BIND-LINT-002; got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_003_fires_on_dangling_case_file_path() {
        let ws = load_workspace("s3_binding/bind_lint_003_dangling_case_file_path.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-003"),
            "expected BIND-LINT-003; got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_004_fires_on_unresolved_output_target() {
        let ws = load_workspace("s3_binding/bind_lint_004_unresolved_output_target.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-004"),
            "expected BIND-LINT-004; got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_005_fires_on_sensitive_without_handling() {
        let ws = load_workspace("s3_binding/bind_lint_005_sensitive_no_handling.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-005"),
            "expected BIND-LINT-005; got {diagnostics:?}"
        );
    }

    #[test]
    fn bind_lint_006_fires_on_missing_error_handling() {
        let ws = load_workspace("s3_binding/bind_lint_006_missing_error_handling.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-006"),
            "expected BIND-LINT-006; got {diagnostics:?}"
        );
    }

    // ---- I-A6+A7+A8 cross-ref rules (J4 backfill) ----

    #[test]
    fn wf_lint_009_fires_on_duplicate_element_id() {
        let ws = load_workspace("s4_workflow/wf_lint_009_duplicate_element_id.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "WF-LINT-009"), "got {diagnostics:?}");
    }

    #[test]
    fn wf_lint_010_fires_on_dangling_position_ref() {
        let ws = load_workspace("s4_workflow/wf_lint_010_dangling_position_ref.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "WF-LINT-010"), "got {diagnostics:?}");
    }

    #[test]
    fn wf_lint_011_fires_on_unapproved_notice_requirement() {
        let ws = load_workspace("s4_workflow/wf_lint_011_notice_unapproved.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "WF-LINT-011"), "got {diagnostics:?}");
    }

    #[test]
    fn wf_lint_012_fires_on_dangling_appeal_ref() {
        let ws = load_workspace("s4_workflow/wf_lint_012_dangling_appeal_ref.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "WF-LINT-012"), "got {diagnostics:?}");
    }

    #[test]
    fn wf_lint_013_fires_on_dangling_service_binding() {
        let ws = load_workspace("s4_workflow/wf_lint_013_dangling_service_binding.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "WF-LINT-013"), "got {diagnostics:?}");
    }

    #[test]
    fn map_lint_009_fires_on_unmapped_at_validation_ready() {
        let ws = load_workspace("s3_mapping/map_lint_009_unmapped_at_validation_ready.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-009"), "got {diagnostics:?}");
    }

    #[test]
    fn map_lint_010_fires_on_open_extension_record() {
        // J2 regression: rule now queries Mapping documents (not
        // PolicyObjects with kind=ExtensionRecord, which doesn't exist
        // as a kind).
        let ws = load_workspace("s3_mapping/map_lint_010_open_extension_record.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-010"), "got {diagnostics:?}");
    }

    #[test]
    fn map_lint_011_fires_on_empty_motivating_refs() {
        // J2 regression: rule now reads
        // mapping.extensionRecord.motivatingPolicyObjectRefs (NOT
        // PolicyObject.motivatingPolicyObjectRef).
        let ws = load_workspace("s3_mapping/map_lint_011_empty_motivating_refs.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-011"), "got {diagnostics:?}");
    }

    #[test]
    fn ra_lint_001_fires_on_unknown_reviewer_role() {
        let ws = load_workspace("s2_policy_object/ra_lint_001_unknown_reviewer_role.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "RA-LINT-001"), "got {diagnostics:?}");
    }

    #[test]
    fn ra_lint_002_fires_on_dangling_comment_subject() {
        let ws = load_workspace("s2_policy_object/ra_lint_002_dangling_comment_subject.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "RA-LINT-002"), "got {diagnostics:?}");
    }

    #[test]
    fn prov_lint_005_fires_on_dangling_parent_record() {
        let ws = load_workspace("cross_cutting/prov_lint_005_dangling_parent_record.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "PROV-LINT-005"), "got {diagnostics:?}");
    }

    #[test]
    fn prov_lint_006_fires_on_approved_element_no_origin() {
        let ws = load_workspace("s4_workflow/prov_lint_006_approved_element_no_origin.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "PROV-LINT-006"), "got {diagnostics:?}");
    }

    #[test]
    fn prov_lint_007_fires_on_approved_interpretation_no_evidence() {
        let ws = load_workspace("s2_policy_object/prov_lint_007_approved_interpretation_no_evidence.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "PROV-LINT-007"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_010_fires_on_consumed_no_source() {
        let ws = load_workspace("s3_binding/bind_lint_010_consumed_no_source.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-010"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_011_fires_on_emitted_no_recipient() {
        let ws = load_workspace("s3_binding/bind_lint_011_emitted_no_recipient.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-011"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_012_fires_on_payload_sensitive_no_redaction() {
        let ws = load_workspace("s3_binding/bind_lint_012_payload_sensitive_no_redaction.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-012"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_020_fires_on_no_case_file_paths() {
        let ws = load_workspace("s3_binding/bind_lint_020_no_case_file_paths.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-020"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_021_fires_on_unmapped_reason_codes() {
        let ws = load_workspace("s3_binding/bind_lint_021_unmapped_reason_codes.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-021"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_070_fires_on_approved_no_scenario() {
        let ws = load_workspace("s3_binding/bind_lint_070_no_scenario.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-070"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_071_fires_on_retry_with_one_scenario() {
        let ws = load_workspace("s3_binding/bind_lint_071_retry_only_one_scenario.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-071"), "got {diagnostics:?}");
    }

    #[test]
    fn bind_lint_072_fires_on_only_permit_coverage() {
        let ws = load_workspace("s3_binding/bind_lint_072_only_permit_coverage.json");
        let diagnostics = lint_workspace(&ws);
        assert!(diagnostics.iter().any(|d| d.rule_id == "BIND-LINT-072"), "got {diagnostics:?}");
    }

    #[test]
    fn sv_lint_007_fires_on_versionless_source_cited() {
        let ws = load_workspace("s1_source_vault/sv_lint_007_versionless_source_cited.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "SV-LINT-007"),
            "expected SV-LINT-007 on PolicyObject citing versionless SourceDocument; got {diagnostics:?}"
        );
    }

    #[test]
    fn sc_lint_005_fires_on_supersession_affected_scenario_not_rerun() {
        // PolicyObject pol-x cites src-v1; src-v1 was superseded; the
        // Scenario citing pol-x is in `lifecycleState: passing` (i.e.
        // not yet rerun against the new version) — rule MUST fire.
        let ws = load_workspace("s5_scenario/sc_lint_005_supersession_affected_not_rerun.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "SC-LINT-005"),
            "expected SC-LINT-005 (supersession-affected scenario not rerun); got {diagnostics:?}"
        );
    }

    #[test]
    fn sc_lint_005_silent_when_source_version_not_superseded() {
        // Negative mate: identical fixture but src-v1 is `current`,
        // not `superseded` — rule must NOT fire.
        let ws = load_workspace("s5_scenario/sc_lint_005_silent_source_current.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "SC-LINT-005"),
            "SC-LINT-005 must not fire when no source version is superseded; got {diagnostics:?}"
        );
    }

    #[test]
    fn map_lint_006_fires_on_workflow_bearing_unmapped_no_override() {
        let ws = load_workspace("s3_mapping/map_lint_006_workflow_bearing_unmapped_no_override.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "MAP-LINT-006"),
            "expected MAP-LINT-006 (workflow-bearing unmapped without override); got {diagnostics:?}"
        );
    }

    #[test]
    fn sc_lint_002_fires_when_appeal_right_lacks_appeal_scenario() {
        let ws = load_workspace("s5_scenario/sc_lint_002_appeal_right_no_scenario.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "SC-LINT-002"),
            "expected SC-LINT-002 (AppealRight without scenario); got {diagnostics:?}"
        );
    }

    #[test]
    fn eq_lint_003_fires_when_protected_category_lacks_equity_probe() {
        let ws = load_workspace("cross_cutting/eq_lint_003_protected_category_no_probe.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "EQ-LINT-003"),
            "expected EQ-LINT-003; got {diagnostics:?}"
        );
    }

    #[test]
    fn acc_lint_001_fires_when_supported_locale_lacks_accessibility_scenario() {
        let ws = load_workspace("cross_cutting/acc_lint_001_locale_no_accessibility.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "ACC-LINT-001"),
            "expected ACC-LINT-001; got {diagnostics:?}"
        );
    }

    #[test]
    fn jur_lint_001_fires_when_multi_jurisdiction_lacks_variation_scenario() {
        let ws = load_workspace("cross_cutting/jur_lint_001_multi_juris_no_variation.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "JUR-LINT-001"),
            "expected JUR-LINT-001; got {diagnostics:?}"
        );
    }

    #[test]
    fn eff_lint_005_fires_on_sunsetting_effectiveness() {
        use crate::date_util::iso_date_offset_from;
        use serde_json::json;
        // 30 days from FROZEN_TODAY — within the 90-day sunset window.
        let end = iso_date_offset_from(FROZEN_TODAY, 30);
        let ws = ws_from(vec![(
            "eff.json",
            json!({
                "$wosStudioEffectiveness": "1.0",
                "id": "eff-sunset",
                "temporalScope": {"effectiveEnd": end}
            }),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "EFF-LINT-005"),
            "expected EFF-LINT-005; got {diagnostics:?}"
        );
    }

    #[test]
    fn eff_lint_005_silent_when_sunset_beyond_90_days() {
        use crate::date_util::iso_date_offset_from;
        use serde_json::json;
        // 120 days ahead — beyond the 90-day window.
        let end = iso_date_offset_from(FROZEN_TODAY, 120);
        let ws = ws_from(vec![(
            "eff.json",
            json!({
                "$wosStudioEffectiveness": "1.0",
                "id": "eff-sunset",
                "temporalScope": {"effectiveEnd": end}
            }),
        )]);
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "EFF-LINT-005"),
            "EFF-LINT-005 must not fire on sunset >90d out; got {diagnostics:?}"
        );
    }

    #[test]
    fn pom_lint_008_fires_on_unresolved_conflict() {
        let ws = load_workspace("s2_policy_object/pom_lint_008_unresolved_conflict.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "POM-LINT-008"),
            "expected POM-LINT-008; got {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_002_fires_on_orphan_appeal_right() {
        let ws = load_workspace("s4_workflow/wf_lint_002_orphan_appeal_right.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "WF-LINT-002"),
            "expected WF-LINT-002 on orphan AppealRight; got {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_005_fires_on_actor_without_authority_grant() {
        let ws = load_workspace("s4_workflow/wf_lint_005_actor_without_authority_grant.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "WF-LINT-005"),
            "expected WF-LINT-005 on ungranted actor; got {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_006_fires_on_sensitive_data_without_retention() {
        // EvidenceRequirement collects DPV-sensitive DataElement but
        // declares no retentionPolicy AND no workspace document with
        // a default for the sensitivity (E8.4 shape-aware migration).
        let ws = load_workspace("s4_workflow/wf_lint_006_sensitive_data_no_retention.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "WF-LINT-006"),
            "expected WF-LINT-006; got {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_006_silent_when_workspace_default_resolves() {
        // Sensitive DataElement, no inline retentionPolicy on the
        // EvidenceRequirement — but workspace.policy.retentionPolicies
        // carries a default keyed by the sensitivity IRI. Resolution
        // succeeds; rule MUST NOT fire.
        let ws = load_workspace("s4_workflow/wf_lint_006_workspace_default_resolves.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "WF-LINT-006"),
            "WF-LINT-006 must not fire when workspace default resolves: {diagnostics:?}"
        );
    }

    #[test]
    fn wf_lint_006_fires_on_inline_policy_shape_violation() {
        // Inline retentionPolicy is present but shape-invalid:
        // mode=indefinite + duration both set (per ADR-0083 §2.1
        // and shape_violations()). Rule MUST fire as Error.
        let ws = load_workspace("s4_workflow/wf_lint_006_inline_policy_malformed.json");
        let diagnostics = lint_workspace(&ws);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == "WF-LINT-006")
            .collect();
        assert!(!hits.is_empty(), "expected WF-LINT-006 shape violation; got {diagnostics:?}");
        assert!(
            hits.iter().any(|d| d.message.contains("indefinite forbids duration")),
            "expected shape-violation message; got {hits:?}"
        );
    }

    #[test]
    fn sa_warn_pom_migrate_retention_fires_on_legacy_field() {
        // EvidenceRequirement carries the legacy `retentionPeriod`
        // field alongside the new `retentionPolicy`. Migration
        // advisory MUST fire as Warning even though the new field
        // is shape-valid.
        let ws = load_workspace("s4_workflow/wf_lint_006_legacy_retention_period_advisory.json");
        let diagnostics = lint_workspace(&ws);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == "SA-WARN-pom-MIGRATE-RETENTION")
            .collect();
        assert!(
            !hits.is_empty(),
            "expected SA-WARN-pom-MIGRATE-RETENTION; got {diagnostics:?}"
        );
        assert!(
            hits.iter().all(|d| d.severity == LintSeverity::Warning),
            "advisory should be Warning, not Error: {hits:?}"
        );
        // WF-LINT-006 itself MUST NOT fire — the new field is valid.
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "WF-LINT-006"),
            "WF-LINT-006 must not fire when new field is shape-valid: {diagnostics:?}"
        );
    }

    #[test]
    fn term_lint_001_fires_on_deprecated_canonical_term() {
        let ws = load_workspace("cross_cutting/term_lint_001_deprecated_canonical_term.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "TERM-LINT-001"),
            "expected TERM-LINT-001 on deprecated term reference; got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_001_fires_on_unresolved_blocking_finding() {
        let ws = load_workspace("s6_publication/pub_lint_001_unresolved_blocking_finding.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-001"),
            "expected PUB-LINT-001 on unresolved blocking finding; got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_001_silent_on_waived_finding() {
        let ws = load_workspace("s6_publication/pub_lint_001_silent_waived_finding.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-001"),
            "PUB-LINT-001 must not fire on waived finding; got {diagnostics:?}"
        );
    }

    #[test]
    fn pub_lint_006_silent_when_release_notes_cite_mapping() {
        let ws = load_workspace("s6_publication/pub_lint_006_silent_release_notes_cite_mapping.json");
        let diagnostics = lint_workspace(&ws);
        assert!(
            !diagnostics.iter().any(|d| d.rule_id == "PUB-LINT-006"),
            "PUB-LINT-006 must not fire when release notes cite the mapping; got {diagnostics:?}"
        );
    }

    /// Fixture-pollution sentinel: each fixture below is constructed to
    /// fire EXACTLY ONE rule. If the diagnostic count drifts, either
    /// (a) a rule was relaxed and we're now under-detecting, or
    /// (b) a new rule is firing collateral on a fixture that was meant
    /// to be minimal-clean for the targeted rule.
    /// Both cases are regressions worth surfacing loudly.
    ///
    /// Covers a representative slice of the rule space — POM-LINT,
    /// PROV-LINT, CHAIN-LINT, ID-LINT, EQ-LINT, COMP-LINT, AI-LINT,
    /// MAP-LINT, SC-LINT — without enumerating all 70 rules. New rules
    /// SHOULD ideally land with a fixture-pollution-clean entry here;
    /// see STUDIO-DEFER-002 (fixture suite externalization) for the
    /// long-term plan.
    #[test]
    fn fixture_pollution_sentinel_known_clean_fixtures_fire_exactly_one_rule() {
        use serde_json::json;

        // (rule_id, label-for-diagnostics, fixture)
        let cases: Vec<(&str, &str, Workspace)> = vec![
            (
                "POM-LINT-008",
                "open Conflict (no other shape)",
                ws_from(vec![(
                    "po.json",
                    json!({
                        "$wosStudioPolicyObject": "1.0",
                        "policyObjects": [{
                            "id": "conf-1",
                            "kind": "Conflict",
                            "resolutionState": "open"
                        }]
                    }),
                )]),
            ),
            (
                "PUB-LINT-001",
                "single open error finding",
                ws_from(vec![(
                    "find.json",
                    json!({
                        "$wosStudioReadiness": "1.0",
                        "id": "vf-1",
                        "kind": "ValidationFinding",
                        "body": {
                            "severity": "error",
                            "lifecycleState": "open",
                            "ruleRef": "POM-LINT-001"
                        }
                    }),
                )]),
            ),
        ];

        for (expected_rule, label, ws) in cases {
            let diagnostics = lint_workspace(&ws);
            assert_eq!(
                diagnostics.len(),
                1,
                "fixture for {expected_rule} ({label}) MUST fire exactly one rule; \
                 got {n} diagnostics: {diagnostics:?}",
                n = diagnostics.len(),
            );
            assert_eq!(
                diagnostics[0].rule_id, expected_rule,
                "fixture for {expected_rule} ({label}) fired wrong rule; \
                 got {diagnostics:?}"
            );
        }
    }
}
