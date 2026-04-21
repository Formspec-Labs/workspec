// Rust guideline compliant 2026-04-20

//! Continuous-mode isolation invariant (Runtime Companion §10.3).
//!
//! In `continuous` evaluation mode, any `setData` mutation re-triggers the
//! guard loop (Runtime Companion §10.3). A transition T2 whose guard reads a
//! path P that another transition T1 writes via `setData` can be re-fired
//! whenever T1 runs; if the graph of `writes → reads` edges among transitions
//! contains a cycle, the processor can loop until stable or until
//! `wos_core::eval_mode::CONVERGENCE_CAP` (100) stops the re-scan. K-049 surfaces
//! that shape as a warning.
//!
//! This module detects that shape statically and surfaces it as K-049.
//! The rule only runs when the kernel declares `evaluationMode: continuous`
//! — `event-driven` mode (the default) handles the same authoring shape
//! safely because guards are only re-evaluated on explicit events.
//!
//! # Rule coverage
//!
//! | Rule    | Category                | What is checked                                                         |
//! |---------|-------------------------|-------------------------------------------------------------------------|
//! | K-049   | continuous-isolation    | `setData`→guard cycles in `continuous`-mode kernels                     |

use std::collections::{HashMap, HashSet};

use fel_core::{
    ast::{Expr, PathSegment as FelPathSegment},
    parse,
};
use wos_core::model::kernel::{
    Action, ActionKind, EvaluationMode, KernelDocument, Region, State, Transition,
};

use crate::diagnostic::LintDiagnostic;

use super::fel_analysis::walk_expr;

// ---------------------------------------------------------------------------
// Structured case-file paths
// ---------------------------------------------------------------------------

/// One segment of a structured case-file path, normalized from either a
/// raw `setData.action.path` string or a FEL access-chain AST subtree.
///
/// Matches the first-class dep-graph vertex shapes enumerated in Core §3.6.1
/// (lines 1388-1409): named properties, indexed slots, and `[*]` wildcards.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Segment {
    Dot(String),
    Index(usize),
    Wildcard,
}

/// Parse a raw `setData.action.path` (e.g. `"caseFile.items[0].x"`) into the
/// structured segment form used for reachability comparison.
///
/// Unparseable input degrades to a single `Segment::Dot(raw)` so comparison
/// stays conservative rather than dropping the write entirely.
fn normalize_setdata_path(raw: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut chars = raw.chars().peekable();
    let mut current = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '.' => {
                chars.next();
                if !current.is_empty() {
                    segments.push(Segment::Dot(std::mem::take(&mut current)));
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(Segment::Dot(std::mem::take(&mut current)));
                }
                chars.next();
                let mut inside = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ']' {
                        break;
                    }
                    inside.push(next);
                    chars.next();
                }
                if chars.peek() != Some(&']') {
                    return vec![Segment::Dot(raw.to_string())];
                }
                chars.next(); // consume ']'
                if inside == "*" {
                    segments.push(Segment::Wildcard);
                } else if let Ok(n) = inside.parse::<usize>() {
                    segments.push(Segment::Index(n));
                } else {
                    return vec![Segment::Dot(raw.to_string())];
                }
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }
    if !current.is_empty() {
        segments.push(Segment::Dot(current));
    }

    if segments.is_empty() {
        vec![Segment::Dot(raw.to_string())]
    } else {
        segments
    }
}

/// Walk a FEL access-chain expression (`FieldRef` / `ContextRef` /
/// `PostfixAccess`) and produce a structured path. Returns `None` for any
/// non-access shape (literal, binary op, function call, etc.).
fn path_from_fel(expr: &Expr) -> Option<Vec<Segment>> {
    match expr {
        Expr::FieldRef {
            name,
            path: segments,
        } => {
            let root = name.as_deref()?;
            let mut out = vec![Segment::Dot(root.to_string())];
            append_segments(&mut out, segments);
            Some(out)
        }
        Expr::ContextRef { name, tail, .. } => {
            let mut out = vec![Segment::Dot(name.clone())];
            for part in tail {
                out.push(Segment::Dot(part.clone()));
            }
            Some(out)
        }
        Expr::PostfixAccess {
            expr: inner,
            path: segments,
        } => {
            let mut out = path_from_fel(inner)?;
            append_segments(&mut out, segments);
            Some(out)
        }
        _ => None,
    }
}

fn append_segments(out: &mut Vec<Segment>, segments: &[FelPathSegment]) {
    for seg in segments {
        match seg {
            FelPathSegment::Dot(part) => out.push(Segment::Dot(part.clone())),
            FelPathSegment::Index(n) => out.push(Segment::Index(*n)),
            FelPathSegment::Wildcard => out.push(Segment::Wildcard),
        }
    }
}

/// §3.6.4 reachability: does a write to `write` invalidate a read of `read`?
///
/// Returns true when:
///   - the paths compare equal segment-by-segment,
///   - at any aligned position one side is `Wildcard` and the other is any
///     non-empty segment (wildcard set-cover),
///   - `write` is a strict prefix of `read` (writing `a` invalidates `a.b.c`),
///   - `read` is a strict prefix of `write` (reading `a` is affected by writing `a.b`).
///
/// `reaches` is symmetric in its truth value — `reaches(a, b) == reaches(b, a)`.
/// The names `write` / `read` are labels for the caller's intent; the
/// direction of the dep-graph edge is encoded by the caller, which always
/// passes writers first when building adjacency. §4.3b Finding 6.
fn reaches(write: &[Segment], read: &[Segment]) -> bool {
    let n = write.len().min(read.len());
    for i in 0..n {
        let w = &write[i];
        let r = &read[i];
        if matches!(w, Segment::Wildcard) || matches!(r, Segment::Wildcard) {
            continue;
        }
        if w != r {
            return false;
        }
    }
    // Prefix-or-equal along the overlapping positions; strict prefix on
    // either side is still a reach (writing a parent invalidates a child
    // read; reading a parent is affected by writing a child).
    true
}

/// Run K-049 against a typed kernel document.
pub(super) fn check(kernel: &KernelDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if kernel.evaluation_mode != Some(EvaluationMode::Continuous) {
        return;
    }

    // First pass: index every reachable state by name so transitions can
    // look up their source and target state metadata. Finding 1 from the
    // 2026-04-20 code review: `setData` actions living in `state.on_entry`
    // or `state.on_exit` also drive re-evaluation and are the canonical
    // cycle source cited by Runtime Companion §10.3.
    let mut state_writes: HashMap<String, StateWrites> = HashMap::new();
    collect_state_writes(&kernel.lifecycle.states, &mut state_writes);

    let mut nodes: Vec<TransitionNode> = Vec::new();
    collect_transitions(
        &kernel.lifecycle.states,
        "/lifecycle/states",
        &state_writes,
        &mut nodes,
    );

    // Adjacency edges use §3.6.4 reachability (`reaches`), which is not a
    // hash-equality relation — wildcard and prefix matches mean two distinct
    // segment vectors can still be joined. We iterate writes × reads per
    // transition-pair; `n` is bounded by transition count so O(n²) is fine
    // in practice and keeps the graph-build loop obvious.
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (j, reader) in nodes.iter().enumerate() {
        let mut edges: HashSet<usize> = HashSet::new();
        for read_path in &reader.reads {
            for (i, writer) in nodes.iter().enumerate() {
                for write_path in &writer.writes {
                    if reaches(write_path, read_path) {
                        edges.insert(i);
                    }
                }
            }
        }
        for i in edges {
            adjacency[i].push(j);
        }
    }

    let Some(cycle) = find_cycle(&adjacency) else {
        return;
    };

    let trail = cycle
        .iter()
        .map(|&i| nodes[i].path.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");

    let head = &nodes[cycle[0]];
    diagnostics.push(LintDiagnostic::t2_warning(
        "K-049",
        head.path.clone(),
        format!(
            "continuous-mode isolation invariant: `setData` → guard dependency cycle ({trail}); \
             Runtime Companion §10.3 post-mutation guard re-scan can loop this configuration until stable \
             or until the processor hits `wos_core::eval_mode::CONVERGENCE_CAP` (100 iterations)"
        ),
    ));
}

// ---------------------------------------------------------------------------
// Transition enumeration
// ---------------------------------------------------------------------------

struct TransitionNode {
    /// JSON-pointer-shaped path to the transition (for diagnostics).
    path: String,
    /// Case-file paths the guard reads (empty when no guard).
    reads: HashSet<Vec<Segment>>,
    /// Case-file paths the `setData` actions write.
    writes: HashSet<Vec<Segment>>,
}

fn collect_transitions(
    states: &indexmap::IndexMap<String, State>,
    parent_path: &str,
    state_writes: &HashMap<String, StateWrites>,
    out: &mut Vec<TransitionNode>,
) {
    for (name, state) in states {
        let state_path = format!("{parent_path}/{name}");
        for (idx, transition) in state.transitions.iter().enumerate() {
            out.push(build_node(
                format!("{state_path}/transitions/{idx}"),
                name,
                transition,
                state_writes,
            ));
        }
        collect_transitions(
            &state.states,
            &format!("{state_path}/states"),
            state_writes,
            out,
        );
        for (region_name, region) in &state.regions {
            collect_region(
                region,
                &format!("{state_path}/regions/{region_name}"),
                state_writes,
                out,
            );
        }
    }
}

fn collect_region(
    region: &Region,
    parent_path: &str,
    state_writes: &HashMap<String, StateWrites>,
    out: &mut Vec<TransitionNode>,
) {
    collect_transitions(
        &region.states,
        &format!("{parent_path}/states"),
        state_writes,
        out,
    );
}

fn build_node(
    path: String,
    source_name: &str,
    transition: &Transition,
    state_writes: &HashMap<String, StateWrites>,
) -> TransitionNode {
    let reads = transition
        .guard
        .as_deref()
        .map(extract_read_paths)
        .unwrap_or_default();

    // Effective writes for cycle-detection purposes are everything the
    // transition actually causes to run when it fires:
    //   1. The source state's `onExit` actions (Kernel §4.7 step 1).
    //   2. The transition's own `actions` (Kernel §4.7 step 2).
    //   3. The target state's `onEntry` actions (Kernel §4.7 step 3).
    // The target may live anywhere in the state tree; we look it up by
    // name via the pre-built `state_writes` index. Missing-name lookups
    // are silently ignored — K-006 already reports dangling targets.
    let mut writes = HashSet::new();
    if let Some(source) = state_writes.get(source_name) {
        for path in &source.on_exit {
            writes.insert(path.clone());
        }
    }
    collect_setdata_paths(&transition.actions, &mut writes);
    if let Some(target) = state_writes.get(&transition.target) {
        for path in &target.on_entry {
            writes.insert(path.clone());
        }
    }

    TransitionNode {
        path,
        reads,
        writes,
    }
}

/// Per-state index of `setData` paths written by `onEntry` / `onExit`.
struct StateWrites {
    on_entry: HashSet<Vec<Segment>>,
    on_exit: HashSet<Vec<Segment>>,
}

fn collect_state_writes(
    states: &indexmap::IndexMap<String, State>,
    out: &mut HashMap<String, StateWrites>,
) {
    for (name, state) in states {
        let mut on_entry = HashSet::new();
        collect_setdata_paths(&state.on_entry, &mut on_entry);
        let mut on_exit = HashSet::new();
        collect_setdata_paths(&state.on_exit, &mut on_exit);
        out.insert(name.clone(), StateWrites { on_entry, on_exit });
        collect_state_writes(&state.states, out);
        for region in state.regions.values() {
            collect_state_writes(&region.states, out);
        }
    }
}

fn collect_setdata_paths(actions: &[Action], out: &mut HashSet<Vec<Segment>>) {
    for action in actions {
        if action.action == ActionKind::SetData {
            if let Some(p) = &action.path {
                out.insert(normalize_setdata_path(p));
            }
        }
    }
}

/// Parse a guard expression and collect every structured field/context path
/// it references. Returns an empty set on parse failure — K-012 already reports
/// unparseable guards. Non-access shapes (literals, operator trees, function
/// calls) contribute nothing: only access chains are cycle-relevant.
///
/// When the walker sees a node that resolves to a structured path (the
/// outermost node of an access chain), it emits the full path and stops
/// descending into that subtree. The short-circuit is load-bearing because
/// the FEL parser represents dotted accesses as nested nodes: `caseFile.input`
/// parses as `PostfixAccess(FieldRef("caseFile", []), [Dot("input")])`.
/// Without the short-circuit, `walk_expr` would recurse into the inner
/// `FieldRef("caseFile")` after emitting the full path `[caseFile, input]`
/// and emit a second stem path `[caseFile]`. Under §3.6.4 prefix
/// reachability, `reaches([caseFile, output], [caseFile])` returns true
/// (strict-prefix write invalidates parent read), producing spurious K-049
/// warnings on acyclic kernels. See the
/// `k049_guard_walker_short_circuit_prevents_spurious_cycle` test.
fn extract_read_paths(guard: &str) -> HashSet<Vec<Segment>> {
    let Ok(expr) = parse(guard) else {
        return HashSet::new();
    };
    let mut paths: HashSet<Vec<Segment>> = HashSet::new();
    walk_expr(&expr, &mut |node| {
        if let Some(p) = path_from_fel(node) {
            paths.insert(p);
            return true; // stop descending into this access chain
        }
        false
    });
    paths
}

// ---------------------------------------------------------------------------
// Cycle detection (directed-graph DFS)
// ---------------------------------------------------------------------------

/// Find one cycle in `adjacency` using iterative DFS with a three-color marking.
/// Returns the cycle as an ordered list of node indices (first node repeats at
/// both ends for readability). Returns `None` when the graph is acyclic.
fn find_cycle(adjacency: &[Vec<usize>]) -> Option<Vec<usize>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let n = adjacency.len();
    let mut color = vec![Color::White; n];
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (node, next-child-index)
    let mut path: Vec<usize> = Vec::new();

    for start in 0..n {
        if color[start] != Color::White {
            continue;
        }
        stack.clear();
        path.clear();
        stack.push((start, 0));
        path.push(start);
        color[start] = Color::Gray;

        while let Some(&(node, child_idx)) = stack.last() {
            if child_idx < adjacency[node].len() {
                let next = adjacency[node][child_idx];
                stack.last_mut().unwrap().1 += 1;
                match color[next] {
                    Color::White => {
                        color[next] = Color::Gray;
                        stack.push((next, 0));
                        path.push(next);
                    }
                    Color::Gray => {
                        // Back edge — cycle found. Trim path to the cycle.
                        let cycle_start = path.iter().position(|&n| n == next).unwrap();
                        let mut cycle: Vec<usize> = path[cycle_start..].to_vec();
                        cycle.push(next);
                        return Some(cycle);
                    }
                    Color::Black => {}
                }
            } else {
                color[node] = Color::Black;
                stack.pop();
                path.pop();
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_from_json(value: serde_json::Value) -> KernelDocument {
        serde_json::from_value(value).expect("kernel doc")
    }

    #[test]
    fn k049_skips_event_driven_mode() {
        // Even with a flagrant cycle, event-driven kernels MUST NOT trigger K-049.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "event-driven",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": { "fields": { "value": { "type": "number" } } },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "tick",
                            "target": "idle",
                            "guard": "caseFile.value > 0",
                            "actions": [{ "action": "setData", "path": "caseFile.value", "value": 1 }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
    }

    #[test]
    fn k049_flags_self_write_cycle_in_continuous_mode() {
        // One transition whose guard reads `caseFile.value` and whose own setData
        // writes `caseFile.value` — a one-node cycle that pegs the convergence cap.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": { "fields": { "value": { "type": "number" } } },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "tick",
                            "target": "idle",
                            "guard": "caseFile.value > 0",
                            "actions": [{ "action": "setData", "path": "caseFile.value", "value": 1 }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
        assert!(diagnostics[0].message.contains("cycle"));
    }

    #[test]
    fn k049_flags_multi_node_cycle_in_continuous_mode() {
        // Two transitions in the same state: T1 reads `caseFile.a`, writes
        // `caseFile.b`; T2 reads `caseFile.b`, writes `caseFile.a`. Classic
        // two-node cycle.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                }
            },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [
                            {
                                "event": "t1",
                                "target": "idle",
                                "guard": "caseFile.a > 0",
                                "actions": [{ "action": "setData", "path": "caseFile.b", "value": 1 }]
                            },
                            {
                                "event": "t2",
                                "target": "idle",
                                "guard": "caseFile.b > 0",
                                "actions": [{ "action": "setData", "path": "caseFile.a", "value": 1 }]
                            }
                        ]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
    }

    #[test]
    fn k049_guard_walker_short_circuit_prevents_spurious_cycle() {
        // §4.3b #F2a — direct regression for `extract_read_paths`'s
        // short-circuit on access chains. The parser represents
        // `caseFile.input` as `PostfixAccess(FieldRef("caseFile", []),
        // [Dot("input")])`. Without the short-circuit at
        // `extract_read_paths`, `walk_expr` would visit the inner
        // `FieldRef("caseFile")` AFTER emitting the full path
        // `[caseFile, input]`, emitting an additional stem path
        // `[caseFile]`. Then `reaches([caseFile, output], [caseFile])`
        // returns true under §3.6.4 prefix reachability (strict-prefix
        // write invalidates parent read), producing a spurious K-049
        // warning on this acyclic kernel.
        //
        // This test is deliberately structurally identical to
        // `k049_ignores_acyclic_continuous_kernel` but lives as its own
        // named case so regressions in the short-circuit get attributed
        // directly. Load-bearing: do not collapse into the sibling test.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "input": { "type": "number" },
                    "output": { "type": "number" }
                }
            },
            "lifecycle": {
                "initialState": "compute",
                "states": {
                    "compute": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "step",
                            "target": "compute",
                            "guard": "caseFile.input > 0",
                            "actions": [{
                                "action": "setData",
                                "path": "caseFile.output",
                                "value": 1
                            }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "spurious K-049 from missing guard-walker short-circuit: {diagnostics:?}"
        );
    }

    #[test]
    fn k049_ignores_acyclic_continuous_kernel() {
        // Continuous mode, setData + guard — but the write path and the guard
        // read paths are disjoint. No cycle.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "input": { "type": "number" },
                    "output": { "type": "number" }
                }
            },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "compute",
                            "target": "idle",
                            "guard": "caseFile.input > 0",
                            "actions": [{ "action": "setData", "path": "caseFile.output", "value": 1 }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
    }

    #[test]
    fn k049_detects_cycle_across_nested_compound_states() {
        // Cycle spans two atomic states inside a compound parent — ensures
        // recursion through `state.states` works.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                }
            },
            "lifecycle": {
                "initialState": "wrapper",
                "states": {
                    "wrapper": {
                        "type": "compound",
                        "initialState": "inner1",
                        "states": {
                            "inner1": {
                                "type": "atomic",
                                "transitions": [{
                                    "event": "e1",
                                    "target": "inner2",
                                    "guard": "caseFile.a > 0",
                                    "actions": [{ "action": "setData", "path": "caseFile.b", "value": 1 }]
                                }]
                            },
                            "inner2": {
                                "type": "atomic",
                                "transitions": [{
                                    "event": "e2",
                                    "target": "inner1",
                                    "guard": "caseFile.b > 0",
                                    "actions": [{ "action": "setData", "path": "caseFile.a", "value": 1 }]
                                }]
                            }
                        }
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
    }

    #[test]
    fn k049_flags_cycle_through_on_entry_setdata() {
        // Spec §10.3 names onEntry setData as a canonical cycle source.
        // Transition T1 has no action but targets `phaseB`; phaseB's onEntry
        // writes `caseFile.b`. Transition T2 (on phaseB) reads `caseFile.b`
        // in its guard and writes `caseFile.a` via onExit of phaseA (which
        // T1 itself targets). Closes the loop through entry/exit actions.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                }
            },
            "lifecycle": {
                "initialState": "phaseA",
                "states": {
                    "phaseA": {
                        "type": "atomic",
                        "onExit": [
                            { "action": "setData", "path": "caseFile.a", "value": 1 }
                        ],
                        "transitions": [{
                            "event": "e1",
                            "target": "phaseB",
                            "guard": "caseFile.a > 0"
                        }]
                    },
                    "phaseB": {
                        "type": "atomic",
                        "onEntry": [
                            { "action": "setData", "path": "caseFile.b", "value": 1 }
                        ],
                        "transitions": [{
                            "event": "e2",
                            "target": "phaseA",
                            "guard": "caseFile.b > 0"
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
    }

    #[test]
    fn k049_message_matches_runtime_and_cap_wording() {
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": { "fields": { "value": { "type": "number" } } },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "target": "idle",
                            "guard": "caseFile.value > 0",
                            "actions": [{ "action": "setData", "path": "caseFile.value", "value": 1 }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        let msg = &diagnostics[0].message;
        assert!(
            msg.contains("Runtime Companion §10.3"),
            "message should cite Runtime Companion §10.3, got: {msg}"
        );
        assert!(
            msg.contains("CONVERGENCE_CAP"),
            "message should name CONVERGENCE_CAP, got: {msg}"
        );
    }

    #[test]
    fn k049_flags_cycle_through_indexed_paths() {
        // Guard reads `caseFile.items[0].x`; setData writes the same indexed
        // slot. Pre-structured-path implementation matched these on string
        // equality and missed the relation when indices changed. Now the
        // reachability comparator handles equal-index segments directly.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "items": {
                        "type": "array",
                        "items": { "type": "object", "properties": { "x": { "type": "number" } } }
                    }
                }
            },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "target": "idle",
                            "guard": "caseFile.items[0].x > 0",
                            "actions": [{
                                "action": "setData",
                                "path": "caseFile.items[0].x",
                                "value": 1
                            }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
    }

    #[test]
    fn k049_flags_cycle_through_wildcard() {
        // Guard reads `caseFile.items[*].y` — wildcard must reach the
        // indexed write to `caseFile.items[3].y` (§3.6.4 set-cover).
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": {
                "fields": {
                    "items": {
                        "type": "array",
                        "items": { "type": "object", "properties": { "y": { "type": "number" } } }
                    }
                }
            },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "target": "idle",
                            "guard": "sum(caseFile.items[*].y) > 0",
                            "actions": [{
                                "action": "setData",
                                "path": "caseFile.items[3].y",
                                "value": 1
                            }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "K-049");
    }

    // --- normalize_setdata_path unit tests ---

    #[test]
    fn normalize_plain_dotted_path() {
        assert_eq!(
            normalize_setdata_path("caseFile.value"),
            vec![Segment::Dot("caseFile".into()), Segment::Dot("value".into())]
        );
    }

    #[test]
    fn normalize_indexed_and_wildcard() {
        assert_eq!(
            normalize_setdata_path("caseFile.items[0].x"),
            vec![
                Segment::Dot("caseFile".into()),
                Segment::Dot("items".into()),
                Segment::Index(0),
                Segment::Dot("x".into()),
            ]
        );
        assert_eq!(
            normalize_setdata_path("caseFile.items[*].x"),
            vec![
                Segment::Dot("caseFile".into()),
                Segment::Dot("items".into()),
                Segment::Wildcard,
                Segment::Dot("x".into()),
            ]
        );
    }

    #[test]
    fn normalize_adversarial_inputs_degrade_to_single_dot() {
        // §4.3b Finding 5 — pin the degrade-to-single-Dot contract on the
        // inputs `normalize_setdata_path` treats as unparseable. Each case
        // must return `vec![Segment::Dot(raw.to_string())]` so downstream
        // reachability stays conservative rather than dropping the write.
        //
        // Trimming, numeric normalization, negative indices, and bracket
        // nesting are deliberately NOT supported — they would mask author
        // intent. `[*]` is intentionally NOT in this list: a leading
        // bracket with a wildcard normalizes to `[Wildcard]` and is
        // exercised by the `reaches_wildcard_*` tests.
        let adversarial: &[&str] = &[
            "",             // empty raw input
            ".",            // lone separator
            "foo[]",        // empty bracket contents
            "foo[-1]",      // negative index (not usize-parseable)
            "foo[[0]]",     // nested brackets
            "foo[a]",       // non-numeric, non-wildcard bracket contents
            "foo[ 1 ]",     // whitespace inside brackets
        ];

        for &raw in adversarial {
            let got = normalize_setdata_path(raw);
            assert_eq!(
                got,
                vec![Segment::Dot(raw.to_string())],
                "adversarial input {raw:?} should degrade to a single Dot segment \
                 holding the raw input; got {got:?}"
            );
        }
    }

    // --- reaches() unit tests ---

    #[test]
    fn reaches_wildcard_matches_index() {
        let write = normalize_setdata_path("caseFile.items[3].y");
        let read = normalize_setdata_path("caseFile.items[*].y");
        assert!(reaches(&write, &read));
        assert!(reaches(&read, &write));
    }

    #[test]
    fn reaches_distinct_indices_do_not_match() {
        let write = normalize_setdata_path("caseFile.items[0].y");
        let read = normalize_setdata_path("caseFile.items[1].y");
        assert!(!reaches(&write, &read));
    }

    #[test]
    fn reaches_prefix_write_invalidates_deeper_read() {
        let write = normalize_setdata_path("caseFile.items");
        let read = normalize_setdata_path("caseFile.items[0].y");
        assert!(reaches(&write, &read));
    }

    #[test]
    fn reaches_prefix_read_affected_by_deeper_write() {
        let write = normalize_setdata_path("caseFile.items[0].y");
        let read = normalize_setdata_path("caseFile.items");
        assert!(reaches(&write, &read));
    }

    #[test]
    fn reaches_is_symmetric_in_truth_value() {
        // §4.3b Finding 6 — `reaches(a, b) == reaches(b, a)` for every
        // input pair. Direction is encoded by the CALLER; this function
        // only asks "do these two path shapes overlap?". Pin a few
        // representative pairs so a future asymmetric refactor breaks here.
        let pairs: &[(&str, &str, bool)] = &[
            ("caseFile.a", "caseFile.a", true),                         // equal
            ("caseFile.items[0].x", "caseFile.items[*].x", true),       // wildcard set-cover
            ("caseFile.items", "caseFile.items[0].y", true),            // prefix
            ("caseFile.items[0].y", "caseFile.items[1].y", false),      // distinct indices
            ("caseFile.a", "caseFile.b", false),                        // distinct names
        ];
        for (lhs, rhs, expected) in pairs {
            let a = normalize_setdata_path(lhs);
            let b = normalize_setdata_path(rhs);
            assert_eq!(
                reaches(&a, &b),
                *expected,
                "reaches({lhs:?}, {rhs:?}) mismatched"
            );
            assert_eq!(
                reaches(&a, &b),
                reaches(&b, &a),
                "reaches is asymmetric for ({lhs:?}, {rhs:?})"
            );
        }
    }

    #[test]
    fn k049_gracefully_ignores_unparseable_guards() {
        // An unparseable guard MUST NOT panic; K-012 reports parse errors.
        // Here the bad guard prevents us from inferring reads, so no cycle
        // can be detected even though a naive string match would flag it.
        let kernel = kernel_from_json(serde_json::json!({
            "$wosKernel": "1.0",
            "evaluationMode": "continuous",
            "actors": [{ "id": "operator", "type": "human" }],
            "impactLevel": "operational",
            "caseFile": { "fields": { "value": { "type": "number" } } },
            "lifecycle": {
                "initialState": "idle",
                "states": {
                    "idle": {
                        "type": "atomic",
                        "transitions": [{
                            "event": "tick",
                            "target": "idle",
                            "guard": "!!! not FEL !!!",
                            "actions": [{ "action": "setData", "path": "caseFile.value", "value": 1 }]
                        }]
                    }
                }
            }
        }));

        let mut diagnostics = Vec::new();
        check(&kernel, &mut diagnostics);
        assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
    }
}
