//! Bounded structural analysis of authored active-state configurations.
//!
//! Guards, event arrival, ordered choices and action effects are possible, not
//! proved. A path to an outcome never proves eventual completion or fairness.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use wos_core::model::integration::CallbackPhase;
use wos_core::model::kernel::{ActionKind, CancellationPolicy, KernelDocument, State, StateKind};

use crate::LintDiagnostic;

/// A refused portion of structural analysis, adapted by each validation boundary.
pub(crate) struct CoverageGap {
    pub path: String,
    pub reason: String,
}

// These refusal bounds keep authoring work bounded, including concurrent state
// combinations. They are resource limits, not workflow validity requirements.
const MAX_STATES: usize = 4096;
const MAX_DEPTH: usize = 128;
const MAX_CONFIGURATIONS: usize = 8192;
const MAX_EDGES: usize = 65536;

struct Node<'a> {
    id: &'a str,
    state: &'a State,
    path: String,
    parent: Option<usize>,
    // Direct children of each parallel region, preserving authored order.
    regions: Vec<Vec<usize>>,
}

type Configuration = Vec<usize>;

fn incomplete(
    diagnostics: &mut Vec<LintDiagnostic>,
    skipped: &mut Vec<CoverageGap>,
    path: &str,
    reason: &str,
) {
    diagnostics.push(LintDiagnostic::t1_warning("WOS-FLOW-005", path,
        format!("Structural control-flow coverage is incomplete: {reason}; no reachability or completion conclusion was made")));
    skipped.push(CoverageGap {
        path: path.to_owned(),
        reason: reason.to_owned(),
    });
}

fn pointer_key(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Check the maintained typed lifecycle after structural/reference checks.
pub(crate) fn check(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<LintDiagnostic>,
    skipped: &mut Vec<CoverageGap>,
) {
    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    let mut pending = Vec::new();
    push_states(
        &kernel.lifecycle.states,
        "/lifecycle/states",
        None,
        0,
        &mut pending,
    );
    let mut duplicates = false;
    let mut unsupported = Vec::new();
    while let Some((id, state, path, parent, depth)) = pending.pop() {
        if depth > MAX_DEPTH || nodes.len() >= MAX_STATES {
            incomplete(
                diagnostics,
                skipped,
                &path,
                "state or nesting bound exceeded",
            );
            return;
        }
        if let Some(previous) = ids.insert(id, nodes.len()) {
            let previous: &Node<'_> = &nodes[previous];
            diagnostics.push(LintDiagnostic::t1_error(
                "WOS-FLOW-001",
                &path,
                format!(
                    "State ID '{id}' also occurs at {}; runtime targets use global IDs",
                    previous.path
                ),
            ));
            duplicates = true;
        }
        let index = nodes.len();
        if state.history_state.is_some() {
            unsupported.push((
                path.clone(),
                "saved shallow/deep history configurations are not modeled",
            ));
        }
        match state.kind {
            StateKind::Compound => push_states(
                &state.states,
                &format!("{path}/states"),
                Some(index),
                depth + 1,
                &mut pending,
            ),
            StateKind::Parallel => {
                for (name, region) in state.regions.iter().rev() {
                    push_states(
                        &region.states,
                        &format!("{path}/regions/{}/states", pointer_key(name)),
                        Some(index),
                        depth + 1,
                        &mut pending,
                    );
                }
            }
            StateKind::ForEach => unsupported.push((
                path.clone(),
                "data-dependent ForEach execution is not modeled",
            )),
            StateKind::Atomic | StateKind::Final => {}
        }
        nodes.push(Node {
            id,
            state,
            path,
            parent,
            regions: Vec::new(),
        });
    }
    // Identity ambiguity must be reported even when other structure is invalid.
    if duplicates {
        return;
    }
    if diagnostics.iter().any(|d| {
        matches!(
            d.rule_id,
            "K-001"
                | "K-002"
                | "K-003"
                | "K-004"
                | "K-005"
                | "K-006"
                | "K-008"
                | "K-016"
                | "K-032"
                | "K-FOREACH-001"
                | "K-FOREACH-002"
                | "K-FOREACH-003"
                | "K-FOREACH-004"
        )
    }) {
        return;
    }
    if kernel.advanced.is_some() {
        unsupported.push((
            "/advanced".into(),
            "advanced constraint effects are not modeled",
        ));
    }
    if !unsupported.is_empty() {
        for (path, reason) in unsupported {
            incomplete(diagnostics, skipped, &path, reason);
        }
        return;
    }
    // Typed callers also use this path, so never silently turn a missing target
    // or malformed kind into an absent graph edge.
    let malformed = nodes.iter().any(|n| {
        let s = n.state;
        (s.kind == StateKind::Compound
            && !s
                .initial_state
                .as_ref()
                .is_some_and(|id| s.states.contains_key(id)))
            || (s.kind != StateKind::Compound
                && (!s.states.is_empty() || s.initial_state.is_some()))
            || (s.kind != StateKind::Parallel && !s.regions.is_empty())
            || (s.kind == StateKind::Parallel && s.regions.is_empty())
            || s.regions
                .values()
                .any(|r| !r.states.contains_key(&r.initial_state))
            || s.transitions
                .iter()
                .any(|t| !ids.contains_key(t.target.as_str()))
    });
    if malformed
        || !kernel
            .lifecycle
            .states
            .contains_key(&kernel.lifecycle.initial_state)
    {
        incomplete(
            diagnostics,
            skipped,
            "/lifecycle",
            "invalid topology prevents graph construction",
        );
        return;
    }
    for n in &mut nodes {
        n.regions = n
            .state
            .regions
            .values()
            .map(|r| r.states.keys().map(|id| ids[id.as_str()]).collect())
            .collect();
    }
    if nodes
        .iter()
        .map(|n| n.state.transitions.len())
        .sum::<usize>()
        > MAX_EDGES
    {
        incomplete(
            diagnostics,
            skipped,
            "/lifecycle",
            "authored-transition bound exceeded",
        );
        return;
    }
    let initial = ids[kernel.lifecycle.initial_state.as_str()];
    let mut start = Vec::new();
    enter(initial, &nodes, &ids, &mut start);
    normalize(&mut start);
    let mut configurations = vec![start.clone()];
    let mut known = HashMap::from([(start, 0)]);
    let mut graph = Vec::new();
    let mut predecessor = vec![None];
    let mut edge_count = 0;
    let mut cursor = 0;
    while cursor < configurations.len() {
        let active = configurations[cursor].clone();
        let mut edges = Vec::new();
        // Interleaving all possible region transitions widens synchronized event
        // and guard behavior. Impossibility findings remain conservative.
        for &source in &active {
            for transition in &nodes[source].state.transitions {
                if transition.is_parallel_join_transition()
                    && !join_enabled(source, &active, &nodes)
                {
                    continue;
                }
                let target = ids[transition.target.as_str()];
                let mut next = active.clone();
                // Exit through the least common ancestor. Self transitions
                // reenter the source; cross-scope transitions remove exited
                // containers and concurrent siblings together.
                let exit = exit_root(source, target, &nodes);
                next.retain(|&i| !descendant(i, exit, &nodes));
                enter_with_ancestors(target, &nodes, &ids, &mut next);
                normalize(&mut next);
                let next_index = if let Some(&index) = known.get(&next) {
                    index
                } else {
                    if configurations.len() >= MAX_CONFIGURATIONS {
                        incomplete(
                            diagnostics,
                            skipped,
                            "/lifecycle",
                            "active-configuration bound exceeded",
                        );
                        return;
                    }
                    let index = configurations.len();
                    known.insert(next.clone(), index);
                    configurations.push(next);
                    predecessor.push(Some(cursor));
                    index
                };
                if !edges.contains(&next_index) {
                    edges.push(next_index);
                }
            }
        }
        edge_count += edges.len();
        if edge_count > MAX_EDGES {
            incomplete(
                diagnostics,
                skipped,
                "/lifecycle",
                "configuration-edge bound exceeded",
            );
            return;
        }
        graph.push(edges);
        cursor += 1;
    }
    diagnostics.push(LintDiagnostic::t1_info("WOS-FLOW-006", "/lifecycle",
        format!("Structural may-reach analysis explored {} configurations; guards, ordered choices, data/action effects, external event delivery, fairness and eventual completion are not proved; administration/migration are outside this analysis", configurations.len())));
    let mut reached = vec![false; nodes.len()];
    for active in &configurations {
        for &index in active {
            reached[index] = true;
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        if !reached[index] {
            diagnostics.push(LintDiagnostic::t1_error("WOS-FLOW-002", &node.path,
                format!("State '{}' is structurally unreachable from '{}' even with guards/events treated as possible", node.id, kernel.lifecycle.initial_state)));
        }
    }
    let outcomes: Vec<bool> = configurations
        .iter()
        .map(|active| process_final(active, &nodes) || business_final_with_receipts(active, &nodes))
        .collect();
    let persistent = configurations
        .iter()
        .any(|a| !process_final(a, &nodes) && business_final_with_receipts(a, &nodes));
    if persistent {
        diagnostics.push(LintDiagnostic::t1_info("WOS-FLOW-007", "/lifecycle",
            "A regional business final can coexist with an independent inbound callback receipt loop (Kernel deferred callbacks); this is a business outcome, not whole-process completion or terminal-cleanup qualification"));
    }
    for (index, active) in configurations.iter().enumerate() {
        if graph[index].is_empty() && !outcomes[index] {
            let located = active
                .iter()
                .copied()
                .find(|&i| nodes[i].state.kind == StateKind::Atomic)
                .unwrap_or(active[0]);
            diagnostics.push(LintDiagnostic::t1_error("WOS-FLOW-003", &nodes[located].path,
                format!("Reachable nonfinal configuration {:?} has no possible local/inherited exit or enabled parallel join; shortest structural route {}", names(active, &nodes), route(index, &predecessor, &configurations, &nodes))));
        }
    }
    for component in components(&graph) {
        let first = component[0];
        let members: HashSet<usize> = component.iter().copied().collect();
        let cyclic = component.len() > 1 || graph[first].contains(&first);
        if cyclic
            && component
                .iter()
                .all(|&i| !outcomes[i] && graph[i].iter().all(|j| members.contains(j)))
        {
            let active = &configurations[first];
            let located = active
                .iter()
                .copied()
                .find(|&i| nodes[i].state.kind == StateKind::Atomic)
                .unwrap_or(active[0]);
            diagnostics.push(LintDiagnostic::t1_error("WOS-FLOW-004", &nodes[located].path,
                format!("Reachable closed configuration cycle containing {:?} has no structural route to process finality or supported regional business finality; shortest structural route {}", names(active, &nodes), route(first, &predecessor, &configurations, &nodes))));
        }
    }
}

fn route(
    mut index: usize,
    predecessor: &[Option<usize>],
    configurations: &[Configuration],
    nodes: &[Node<'_>],
) -> String {
    let mut path = vec![index];
    while let Some(parent) = predecessor[index] {
        path.push(parent);
        index = parent;
    }
    path.reverse();
    // Bound diagnostic size independently of analysis size; retain the actual
    // shortest BFS distance and label any omitted middle configurations.
    let steps = path.len() - 1;
    let displayed: Vec<_> = if path.len() > 16 {
        path[..8]
            .iter()
            .chain(path[path.len() - 8..].iter())
            .copied()
            .collect()
    } else {
        path.clone()
    };
    format!(
        "{steps} steps {:?}{}",
        displayed
            .iter()
            .map(|&i| names(&configurations[i], nodes))
            .collect::<Vec<_>>(),
        if path.len() > 16 {
            " (middle configurations omitted)"
        } else {
            ""
        }
    )
}

fn names<'a>(active: &[usize], nodes: &[Node<'a>]) -> Vec<&'a str> {
    active.iter().map(|&i| nodes[i].id).collect()
}
fn normalize(active: &mut Configuration) {
    active.sort_unstable();
    active.dedup();
}
fn descendant(mut state: usize, ancestor: usize, nodes: &[Node<'_>]) -> bool {
    loop {
        if state == ancestor {
            return true;
        }
        match nodes[state].parent {
            Some(p) => state = p,
            None => return false,
        }
    }
}
fn exit_root(source: usize, target: usize, nodes: &[Node<'_>]) -> usize {
    // Match the maintained evaluator's external ancestor reentry and orthogonal
    // region reset (eval::transition_paths; Kernel 4.8.6).
    if descendant(source, target, nodes) {
        return target;
    }
    let mut exit = source;
    while let Some(parent) = nodes[exit].parent {
        if descendant(target, parent, nodes) {
            if nodes[parent].state.kind == StateKind::Parallel
                && nodes[parent].regions.iter().any(|region| {
                    region.iter().any(|&child| descendant(source, child, nodes))
                        && !region.iter().any(|&child| descendant(target, child, nodes))
                })
            {
                return parent;
            }
            break;
        }
        exit = parent;
    }
    exit
}
fn enter(index: usize, nodes: &[Node<'_>], ids: &HashMap<&str, usize>, active: &mut Configuration) {
    if active.contains(&index) {
        return;
    }
    active.push(index);
    match nodes[index].state.kind {
        StateKind::Compound => enter(
            ids[nodes[index]
                .state
                .initial_state
                .as_deref()
                .expect("validated initial")],
            nodes,
            ids,
            active,
        ),
        StateKind::Parallel => {
            for region in nodes[index].state.regions.values() {
                enter(ids[region.initial_state.as_str()], nodes, ids, active);
            }
        }
        _ => {}
    }
}
fn enter_with_ancestors(
    index: usize,
    nodes: &[Node<'_>],
    ids: &HashMap<&str, usize>,
    active: &mut Configuration,
) {
    let mut missing = Vec::new();
    let mut ancestor = nodes[index].parent;
    while let Some(parent) = ancestor {
        if !active.contains(&parent) {
            missing.push(parent);
        }
        ancestor = nodes[parent].parent;
    }
    // Ancestors must be active without entering their default compound child.
    // A newly entered parallel also starts the other regions.
    for &parent in missing.iter().rev() {
        active.push(parent);
        if nodes[parent].state.kind == StateKind::Parallel {
            for region in nodes[parent].state.regions.values() {
                if !region
                    .states
                    .keys()
                    .any(|id| descendant(index, ids[id.as_str()], nodes))
                {
                    enter(ids[region.initial_state.as_str()], nodes, ids, active);
                }
            }
        }
    }
    enter(index, nodes, ids, active);
}
fn join_enabled(index: usize, active: &[usize], nodes: &[Node<'_>]) -> bool {
    let node = &nodes[index];
    if node.state.kind != StateKind::Parallel {
        return false;
    }
    let finals: Vec<_> = node
        .regions
        .iter()
        .map(|region| {
            region
                .iter()
                .any(|i| active.contains(i) && nodes[*i].state.kind == StateKind::Final)
        })
        .collect();
    match node
        .state
        .cancellation_policy
        .unwrap_or(CancellationPolicy::WaitAll)
    {
        CancellationPolicy::WaitAll => finals.iter().all(|done| *done),
        CancellationPolicy::CancelSiblings => finals.iter().any(|done| *done),
        // Detailed Kernel 4.8.3 and runtime require an error final, rather
        // than the broader tagged-state wording in the 4.8 overview.
        CancellationPolicy::FailFast => {
            finals.iter().all(|done| *done)
                || node.regions.iter().flatten().any(|i| {
                    active.contains(i)
                        && nodes[*i].state.kind == StateKind::Final
                        && nodes[*i].state.tags.iter().any(|tag| tag == "error")
                })
        }
    }
}
fn process_final(active: &[usize], nodes: &[Node<'_>]) -> bool {
    active
        .iter()
        .any(|&i| nodes[i].parent.is_none() && nodes[i].state.kind == StateKind::Final)
}
fn receipt_region(region: &[usize], nodes: &[Node<'_>]) -> bool {
    // Kernel deferred callbacks explicitly permit an independent permanent
    // receipt region. Recognize only semantic inbound actions, never names,
    // prose or tags. Reentry must be task-free; no arbitrary loop exemption.
    if region.len() != 1 {
        return false;
    }
    let node = &nodes[region[0]];
    node.state.kind == StateKind::Atomic
        && node.state.on_entry.is_empty()
        && node.state.on_exit.is_empty()
        && !node.state.transitions.is_empty()
        && node.state.transitions.iter().all(|t| {
            t.target == node.id
                && t.actions.iter().any(|a| {
                    a.action == ActionKind::InvokeService
                        && a.callback_phase == Some(CallbackPhase::Inbound)
                })
                && !t.actions.iter().any(|a| a.action == ActionKind::CreateTask)
        })
}
fn business_final_with_receipts(active: &[usize], nodes: &[Node<'_>]) -> bool {
    nodes.iter().enumerate().any(|(index, n)| {
        n.parent.is_none()
            && n.state.kind == StateKind::Parallel
            && active.contains(&index)
            && n.state
                .cancellation_policy
                .unwrap_or(CancellationPolicy::WaitAll)
                == CancellationPolicy::WaitAll
            && n.regions.iter().any(|r| receipt_region(r, nodes))
            && n.regions.iter().any(|r| !receipt_region(r, nodes))
            && n.regions.iter().all(|r| {
                if receipt_region(r, nodes) {
                    active.contains(&r[0])
                } else {
                    r.iter()
                        .any(|i| active.contains(i) && nodes[*i].state.kind == StateKind::Final)
                }
            })
    })
}

type PendingState<'a> = (&'a str, &'a State, String, Option<usize>, usize);
fn push_states<'a>(
    states: &'a IndexMap<String, State>,
    path: &str,
    parent: Option<usize>,
    depth: usize,
    pending: &mut Vec<PendingState<'a>>,
) {
    for (id, state) in states.iter().rev() {
        pending.push((
            id.as_str(),
            state,
            format!("{path}/{}", pointer_key(id)),
            parent,
            depth,
        ));
    }
}

/// Iterative Kosaraju traversal keeps stack use bounded and output deterministic.
fn components(graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; graph.len()];
    let mut order = Vec::new();
    for start in 0..graph.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((node, edge)) = stack.pop() {
            if edge == graph[node].len() {
                order.push(node);
                continue;
            }
            stack.push((node, edge + 1));
            let target = graph[node][edge];
            if !seen[target] {
                seen[target] = true;
                stack.push((target, 0));
            }
        }
    }
    let mut reverse = vec![Vec::new(); graph.len()];
    for (source, edges) in graph.iter().enumerate() {
        for &target in edges {
            reverse[target].push(source);
        }
    }
    seen.fill(false);
    let mut result = Vec::new();
    for start in order.into_iter().rev() {
        if seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        seen[start] = true;
        while let Some(node) = stack.pop() {
            component.push(node);
            for &target in &reverse[node] {
                if !seen[target] {
                    seen[target] = true;
                    stack.push(target);
                }
            }
        }
        component.sort_unstable();
        result.push(component);
    }
    result.sort_by_key(|members| members[0]);
    result
}
