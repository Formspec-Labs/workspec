// Rust guideline compliant 2026-04-20

//! Continuous-mode isolation invariant (Runtime Companion §10.3).
//!
//! In `continuous` evaluation mode, any `setData` mutation re-triggers the
//! guard loop. A transition T2 whose guard reads a path P that another
//! transition T1 writes via `setData` can be re-fired whenever T1 runs; if
//! the graph of `writes → reads` edges among fireable transitions contains
//! a cycle, the processor can thrash against its 100-cycle convergence cap
//! (`CONVERGENCE_CAP` in `wos_core::eval_mode`).
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

use fel_core::parse;
use wos_core::model::kernel::{
    Action, ActionKind, EvaluationMode, KernelDocument, Region, State, Transition,
};

use crate::diagnostic::LintDiagnostic;

use super::fel_analysis::{simple_access_path_string, walk_expr};

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

    // Build per-path write-index so cycle edges are O(writes × reads) not O(n²).
    let mut writers_by_path: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        for path in &node.writes {
            writers_by_path.entry(path.clone()).or_default().push(i);
        }
    }

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (j, node) in nodes.iter().enumerate() {
        let mut edges: HashSet<usize> = HashSet::new();
        for path in &node.reads {
            if let Some(writers) = writers_by_path.get(path) {
                for &i in writers {
                    edges.insert(i);
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
             runtime will trip the 100-cycle convergence cap (Runtime Companion §10.3)"
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
    reads: HashSet<String>,
    /// Case-file paths the `setData` actions write.
    writes: HashSet<String>,
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
    on_entry: HashSet<String>,
    on_exit: HashSet<String>,
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

fn collect_setdata_paths(actions: &[Action], out: &mut HashSet<String>) {
    for action in actions {
        if action.action == ActionKind::SetData {
            if let Some(p) = &action.path {
                out.insert(p.clone());
            }
        }
    }
}

/// Parse a guard expression and collect every simple dotted field/context path
/// it references. Returns an empty set on parse failure — K-012 already reports
/// unparseable guards.
fn extract_read_paths(guard: &str) -> HashSet<String> {
    let Ok(expr) = parse(guard) else {
        return HashSet::new();
    };
    let mut paths: HashSet<String> = HashSet::new();
    walk_expr(&expr, &mut |node| {
        if let Some(p) = simple_access_path_string(node) {
            paths.insert(p);
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
