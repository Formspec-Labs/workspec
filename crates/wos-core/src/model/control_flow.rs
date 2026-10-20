// Rust guideline compliant 2026-02-21
//! Bounded structural analysis of authored active-state configurations.
//!
//! Guards, event arrival, ordered choices and action effects are possible, not
//! proved. A path to an outcome never proves eventual completion or fairness.

use std::collections::{HashMap, HashSet};

use super::integration::CallbackPhase;
use super::kernel::{ActionKind, CancellationPolicy, KernelDocument, State, StateKind};
use indexmap::IndexMap;

/// Meaning of a structural diagnostic, independent of presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowSeverity {
    /// Refuses structural admission.
    Error,
    /// Identifies incomplete or limited coverage.
    Warning,
    /// Describes performed checks and their limits.
    Info,
}
/// Located structural result, independent of authoring or runtime presentation.
#[derive(Debug, Clone)]
pub struct FlowIssue {
    /// Stable diagnostic identifier.
    pub rule_id: &'static str,
    /// Owning JSON pointer.
    pub path: String,
    /// Structural finding.
    pub message: String,
    /// Severity owned by the shared analysis.
    pub severity: FlowSeverity,
}
impl FlowIssue {
    /// Whether this finding refuses selected admission.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == FlowSeverity::Error
    }

    pub(crate) fn error(id: &'static str, path: &str, message: impl Into<String>) -> Self {
        Self {
            rule_id: id,
            path: path.into(),
            message: message.into(),
            severity: FlowSeverity::Error,
        }
    }
    fn warning(id: &'static str, path: &str, message: impl Into<String>) -> Self {
        Self {
            rule_id: id,
            path: path.into(),
            message: message.into(),
            severity: FlowSeverity::Warning,
        }
    }
    fn info(id: &'static str, path: &str, message: impl Into<String>) -> Self {
        Self {
            rule_id: id,
            path: path.into(),
            message: message.into(),
            severity: FlowSeverity::Info,
        }
    }
}
/// Bounded analysis findings and incomplete coverage.
#[derive(Debug, Default)]
pub struct FlowAnalysis {
    /// Located structural findings.
    pub issues: Vec<FlowIssue>,
    /// Portions that could not be analyzed.
    pub gaps: Vec<CoverageGap>,
}
/// Analyze possible configurations without relying on caller validation.
#[must_use]
pub fn analyze(kernel: &KernelDocument) -> FlowAnalysis {
    let mut result = FlowAnalysis::default();
    check(kernel, &[], &mut result.issues, &mut result.gaps);
    result
}

// Original action locations are derived within core, never editable policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActionLocation {
    pub state: String,
    pub transition: Option<usize>,
}
#[derive(Debug)]
pub(super) struct NoticePath {
    pub producers: Vec<ActionLocation>,
    pub actions: Vec<ActionLocation>,
    pub after_mapping: bool,
    pub path: String,
}
pub(super) fn analyze_with_notice_paths(
    kernel: &KernelDocument,
    paths: &[NoticePath],
) -> FlowAnalysis {
    let mut result = FlowAnalysis::default();
    check(kernel, paths, &mut result.issues, &mut result.gaps);
    result
}
/// A refused portion of structural analysis, adapted by each validation boundary.
#[derive(Debug, Clone)]
pub struct CoverageGap {
    pub path: String,
    pub reason: String,
}

// These refusal bounds keep authoring work bounded, including concurrent state
// combinations. They are resource limits, not workflow validity requirements.
const MAX_STATES: usize = 4096;
const MAX_DEPTH: usize = 128;
const MAX_CONFIGURATIONS: usize = 8192;
const MAX_EDGES: usize = 65536;

// Selected association walks use the same finite limits before any recursive
// maintained visitor. This is a resource precheck, not another graph analysis.
pub(super) fn check_selected_resources(kernel: &KernelDocument) -> Result<(), CoverageGap> {
    let mut pending = Vec::new();
    for (id, state) in &kernel.lifecycle.states {
        pending.push((
            state,
            format!("/lifecycle/states/{}", pointer_key(id)),
            0usize,
        ));
    }
    let mut states = 0usize;
    let mut actions = 0usize;
    while let Some((state, path, depth)) = pending.pop() {
        states += 1;
        if depth > MAX_DEPTH || states > MAX_STATES {
            return Err(CoverageGap {
                path,
                reason: "selected state or nesting bound exceeded".into(),
            });
        }
        for (id, child) in &state.states {
            pending.push((
                child,
                format!("{path}/states/{}", pointer_key(id)),
                depth + 1,
            ));
        }
        for (region_id, region) in &state.regions {
            for (id, child) in &region.states {
                pending.push((
                    child,
                    format!(
                        "{path}/regions/{}/states/{}",
                        pointer_key(region_id),
                        pointer_key(id)
                    ),
                    depth + 1,
                ));
            }
        }
        if let Some(body) = &state.body {
            pending.push((body, format!("{path}/body"), depth + 1));
        }
        let mut work = state
            .on_entry
            .iter()
            .chain(state.on_exit.iter())
            .chain(state.transitions.iter().flat_map(|t| t.actions.iter()))
            .map(|a| (a, 0usize))
            .collect::<Vec<_>>();
        while let Some((action, depth)) = work.pop() {
            actions += 1;
            if depth > MAX_DEPTH || actions > MAX_EDGES {
                return Err(CoverageGap {
                    path,
                    reason: "selected action or compensation bound exceeded".into(),
                });
            }
            if let Some(compensation) = &action.compensating_action {
                work.push((compensation, depth + 1));
            }
        }
    }
    Ok(())
}
pub(super) fn coverage_refusal(gap: CoverageGap) -> FlowAnalysis {
    let mut result = FlowAnalysis::default();
    incomplete(&mut result.issues, &mut result.gaps, &gap.path, &gap.reason);
    result
}

struct Node<'a> {
    id: &'a str,
    state: &'a State,
    path: String,
    parent: Option<usize>,
    // Direct children of each parallel region, preserving authored order.
    regions: Vec<Vec<usize>>,
}

type Configuration = Vec<usize>;
// Original transition identity stays attached to the one configuration graph.
// The existing edge bound applies to these actual edges, including distinct
// authored transitions that happen to lead to the same configuration.
#[derive(Debug, Clone, Copy)]
struct Edge {
    target: usize,
    source: usize,
    transition: usize,
}

fn incomplete(
    diagnostics: &mut Vec<FlowIssue>,
    skipped: &mut Vec<CoverageGap>,
    path: &str,
    reason: &str,
) {
    diagnostics.push(FlowIssue::warning("WOS-FLOW-005", path,
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
fn check(
    kernel: &KernelDocument,
    paths: &[NoticePath],
    diagnostics: &mut Vec<FlowIssue>,
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
            diagnostics.push(FlowIssue::error(
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
    check_topology(kernel, &nodes, diagnostics);
    if diagnostics.iter().any(|issue| issue.is_error()) {
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
        (s.kind == StateKind::Final && !s.transitions.is_empty())
            || (s.cancellation_policy.is_some() && s.kind != StateKind::Parallel)
            || (s.history_state.is_some() && s.kind != StateKind::Compound)
            || (s.kind == StateKind::Parallel
                && s.transitions
                    .iter()
                    .any(|t| !t.is_parallel_join_transition()))
            || (s.kind == StateKind::Compound
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
            for (transition_index, transition) in nodes[source].state.transitions.iter().enumerate()
            {
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
                edges.push(Edge {
                    target: next_index,
                    source,
                    transition: transition_index,
                });
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
    // Each original required use must have one complete ordered run. Also
    // reject exposed waits with no prerequisite request/timer, even if another
    // branch has a valid run. Permanent independent receipt regions merely
    // register listeners; native pending-request correlation still gates them.
    let mut association_work = 0usize;
    for path in paths {
        let locate = |action: &ActionLocation| {
            nodes
                .iter()
                .position(|node| node.path == action.state)
                .map(|index| (index, action.transition))
        };
        let mut locations = path
            .actions
            .iter()
            .map(|action| locate(action).map(|v| vec![v]))
            .collect::<Option<Vec<_>>>();
        if let Some(steps) = &mut locations {
            match path
                .producers
                .iter()
                .map(locate)
                .collect::<Option<Vec<_>>>()
            {
                Some(first) if !first.is_empty() => steps.insert(0, first),
                _ => locations = None,
            }
        }
        let mut possible = false;
        let mut bypass = false;
        let mut witness = String::new();
        if let Some(locations) = locations.as_mut() {
            let initial = usize::from(locations[0].iter().any(|(node, transition)| {
                transition.is_none() && configurations[0].contains(node)
            }));
            let mut states = vec![(0, initial)];
            let mut known = HashMap::from([((0, initial), 0)]);
            let mut reverse = vec![Vec::new()];
            let mut outcomes = vec![false];
            let mut parent = vec![None];
            let mut seed_source = vec![None];
            let mut bad_state = None;
            let mut producer_occurrences = Vec::new();
            if initial == 1 {
                producer_occurrences.push(0);
            }
            // A completed earlier pair must not hide a later fresh request.
            // Seed each real reachable occurrence using this same graph's entry
            // semantics; converging identical configurations safely share work.
            for (source_index, edges) in graph.iter().enumerate() {
                for edge in edges {
                    association_work += 1;
                    if association_work > MAX_EDGES * MAX_DEPTH {
                        incomplete(
                            diagnostics,
                            skipped,
                            &path.path,
                            "selected notice association work bound exceeded",
                        );
                        return;
                    }
                    let target = ids[nodes[edge.source].state.transitions[edge.transition]
                        .target
                        .as_str()];
                    let exit = exit_root(edge.source, target, &nodes);
                    if locations[0].iter().any(|&(node, phase)| match phase {
                        Some(index) => node == edge.source && index == edge.transition,
                        None => {
                            configurations[edge.target].contains(&node)
                                && (!configurations[source_index].contains(&node)
                                    || descendant(node, exit, &nodes))
                        }
                    }) {
                        let seed = (edge.target, 1);
                        let index = if let Some(&index) = known.get(&seed) {
                            index
                        } else {
                            let index = states.len();
                            known.insert(seed, index);
                            states.push(seed);
                            reverse.push(Vec::new());
                            outcomes.push(false);
                            parent.push(None);
                            seed_source.push(Some(source_index));
                            index
                        };
                        producer_occurrences.push(index);
                    }
                }
            }
            let mut cursor = 0;
            while cursor < states.len() {
                let (i, progress) = states[cursor];
                if progress == locations.len() {
                    possible = true;
                    outcomes[cursor] = true;
                    cursor += 1;
                    continue;
                }
                if process_final(&configurations[i], &nodes)
                    || business_final_with_receipts(&configurations[i], &nodes)
                {
                    outcomes[cursor] = true;
                    cursor += 1;
                    continue;
                }
                for (step, alternatives) in locations.iter().enumerate().skip(1) {
                    for &(node, transition) in alternatives {
                        let Some(index) = transition else {
                            continue;
                        };
                        let independent = nodes.iter().any(|parent| {
                            parent.regions.iter().any(|region| {
                                region.as_slice() == [node] && receipt_region(region, &nodes)
                            })
                        });
                        let timer = matches!(
                            nodes[node].state.transitions[index].event,
                            Some(super::kernel::TransitionEvent::Timer { .. })
                        );
                        let prerequisite = if timer { step } else { 1 };
                        if configurations[i].contains(&node)
                            && progress < prerequisite
                            && !independent
                        {
                            bypass = true;
                            bad_state = Some(cursor);
                            break;
                        }
                    }
                }
                if bypass {
                    break;
                }
                for edge in &graph[i] {
                    association_work += 1;
                    if association_work > MAX_EDGES * MAX_DEPTH {
                        incomplete(
                            diagnostics,
                            skipped,
                            &path.path,
                            "selected notice association work bound exceeded",
                        );
                        return;
                    }
                    let matches = |(node, transition): &(usize, Option<usize>)| {
                        *node == edge.source && *transition == Some(edge.transition)
                    };
                    // A selected callback's explicit emitted continuation is
                    // available after that callback, including across regions.
                    // Unrelated external edges still expose ordinary bypasses.
                    if progress < 2
                        && locations[1].iter().any(|&(callback, phase)| {
                            phase.is_some_and(|index| {
                                nodes[callback].state.transitions[index]
                                    .actions
                                    .iter()
                                    .filter(|action| action.action == ActionKind::EmitEvent)
                                    .filter_map(|action| action.event_type.as_deref())
                                    .any(|emitted| {
                                        nodes[edge.source].state.transitions[edge.transition]
                                            .event
                                            .as_ref()
                                            .is_some_and(|event| {
                                                event.matches_runtime_dispatch(emitted)
                                            })
                                    })
                            })
                        })
                    {
                        continue;
                    }
                    if locations
                        .iter()
                        .skip(progress + 1)
                        .any(|stage| stage.iter().any(matches))
                        && !locations[progress].iter().any(matches)
                    {
                        continue;
                    }
                    let mut next_progress =
                        progress + usize::from(locations[progress].iter().any(matches));
                    let transition = &nodes[edge.source].state.transitions[edge.transition];
                    let target = ids[transition.target.as_str()];
                    let exit = exit_root(edge.source, target, &nodes);
                    if path.after_mapping
                        && progress == 1
                        && next_progress == 2
                        && locations.get(2).is_some_and(|starts| {
                            starts.iter().any(|&(node, phase)| {
                                phase.is_none()
                                    && configurations[edge.target].contains(&node)
                                    && (!configurations[i].contains(&node)
                                        || descendant(node, exit, &nodes))
                            })
                        })
                    {
                        // Includes newly entered ancestors and their initial children,
                        // using the exact graph/evaluator entry semantics above.
                        bypass = true;
                        bad_state = Some(cursor);
                        break;
                    }
                    while let Some(alternatives) = locations.get(next_progress) {
                        // Explicit elapsed starts must execute after callback output Mapping,
                        // not on the inbound transition's own target entry.
                        if path.after_mapping && progress == 1 && next_progress == 2 {
                            break;
                        }
                        let entered = alternatives.iter().any(|&(node, phase)| {
                            phase.is_none()
                                && configurations[edge.target].contains(&node)
                                && (!configurations[i].contains(&node)
                                    || descendant(node, exit, &nodes))
                        });
                        if !entered {
                            break;
                        }
                        next_progress += 1;
                    }
                    let next = (edge.target, next_progress);
                    let next_index = if let Some(&index) = known.get(&next) {
                        index
                    } else {
                        let index = states.len();
                        known.insert(next, index);
                        states.push(next);
                        reverse.push(Vec::new());
                        outcomes.push(false);
                        parent.push(Some(cursor));
                        seed_source.push(None);
                        index
                    };
                    reverse[next_index].push(cursor);
                }
                cursor += 1;
            }
            // A valid route cannot hide another reachable stage with no
            // possible ordered continuation. Reverse reachability catches
            // both dead ends and closed cycles after ineligible event edges
            // have been removed, without rebuilding the Workflow graph.
            let mut good = vec![false; states.len()];
            let mut pending = outcomes
                .iter()
                .enumerate()
                .filter_map(|(i, done)| done.then_some(i))
                .collect::<Vec<_>>();
            while let Some(index) = pending.pop() {
                if good[index] {
                    continue;
                }
                good[index] = true;
                pending.extend(reverse[index].iter().copied());
            }
            if let Some(index) = good.iter().position(|good| !*good) {
                bypass = true;
                bad_state = bad_state.or(Some(index));
            }
            let mut callback_reachable = vec![false; states.len()];
            let mut pending = states
                .iter()
                .enumerate()
                .filter_map(|(index, (_, progress))| (*progress >= 2).then_some(index))
                .collect::<Vec<_>>();
            while let Some(index) = pending.pop() {
                if callback_reachable[index] {
                    continue;
                }
                callback_reachable[index] = true;
                pending.extend(reverse[index].iter().copied());
            }
            if let Some(&index) = producer_occurrences
                .iter()
                .find(|&&index| !callback_reachable[index])
            {
                bypass = true;
                bad_state = bad_state.or(Some(index));
            }
            if let Some(index) = bad_state {
                let mut trace = vec![index];
                while let Some(previous) = parent[*trace.last().expect("trace has a state")] {
                    trace.push(previous);
                }
                trace.reverse();
                let prefix = seed_source[trace[0]].unwrap_or(0);
                let continuation = trace
                    .iter()
                    .rev()
                    .take(16)
                    .rev()
                    .map(|&index| names(&configurations[states[index].0], &nodes))
                    .collect::<Vec<_>>();
                witness = format!(
                    "; stage {}; structural prefix {}; selected continuation {:?}",
                    states[index].1,
                    route(prefix, &predecessor, &configurations, &nodes),
                    continuation
                );
            }
        }
        if bypass || !possible {
            let message = if bypass {
                "reachable mandatory wait lacks its required original request or timer on that path"
            } else {
                "no possible run executes the complete required notice action sequence"
            };
            diagnostics.push(FlowIssue::error(
                "NOTICE-GRAPH-006",
                &path.path,
                format!("{message}{witness}"),
            ));
        }
    }
    diagnostics.push(FlowIssue::info("WOS-FLOW-006", "/lifecycle",
        format!("Structural may-reach analysis explored {} configurations; guards, ordered choices, data/action effects, external event delivery, fairness and eventual completion are not proved; administration/migration are outside this analysis", configurations.len())));
    let mut reached = vec![false; nodes.len()];
    for active in &configurations {
        for &index in active {
            reached[index] = true;
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        if !reached[index] {
            diagnostics.push(FlowIssue::error("WOS-FLOW-002", &node.path,
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
        diagnostics.push(FlowIssue::info("WOS-FLOW-007", "/lifecycle",
            "A regional business final can coexist with an independent inbound callback receipt loop (Kernel deferred callbacks); this is a business outcome, not whole-process completion or terminal-cleanup qualification"));
    }
    for (index, active) in configurations.iter().enumerate() {
        if graph[index].is_empty() && !outcomes[index] {
            let located = finding_location(active, &nodes);
            diagnostics.push(FlowIssue::error("WOS-FLOW-003", &nodes[located].path,
                format!("Reachable nonfinal configuration {:?} has no possible local/inherited exit or enabled parallel join; shortest structural route {}", names(active, &nodes), route(index, &predecessor, &configurations, &nodes))));
        }
    }
    for component in components(&graph) {
        let first = component[0];
        let members: HashSet<usize> = component.iter().copied().collect();
        let cyclic = component.len() > 1 || graph[first].iter().any(|edge| edge.target == first);
        if cyclic
            && component.iter().all(|&i| {
                !outcomes[i] && graph[i].iter().all(|edge| members.contains(&edge.target))
            })
        {
            let active = &configurations[first];
            let located = finding_location(active, &nodes);
            diagnostics.push(FlowIssue::error("WOS-FLOW-004", &nodes[located].path,
                format!("Reachable closed configuration cycle containing {:?} has no structural route to process finality or supported regional business finality; shortest structural route {}", names(active, &nodes), route(first, &predecessor, &configurations, &nodes))));
        }
    }
}

fn finding_location(active: &[usize], nodes: &[Node<'_>]) -> usize {
    // Prefer the stranded business state over an independent receipt loop when
    // locating a closed configuration. The message still names all active states.
    active
        .iter()
        .copied()
        .find(|&i| {
            nodes[i].state.kind == StateKind::Atomic && nodes[i].state.transitions.is_empty()
        })
        .or_else(|| {
            active
                .iter()
                .copied()
                .find(|&i| nodes[i].state.kind == StateKind::Atomic)
        })
        .unwrap_or(active[0])
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
fn components(graph: &[Vec<Edge>]) -> Vec<Vec<usize>> {
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
            let target = graph[node][edge].target;
            if !seen[target] {
                seen[target] = true;
                stack.push((target, 0));
            }
        }
    }
    let mut reverse = vec![Vec::new(); graph.len()];
    for (source, edges) in graph.iter().enumerate() {
        for edge in edges {
            reverse[edge.target].push(source);
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

fn check_topology(kernel: &KernelDocument, nodes: &[Node<'_>], diagnostics: &mut Vec<FlowIssue>) {
    check_lifecycle_initial_state_resolves_typed(kernel, diagnostics);
    check_initial_state_keys_into_states_typed(kernel, diagnostics);
    let all_state_ids: HashSet<String> = nodes.iter().map(|n| n.id.to_owned()).collect();
    for node in nodes {
        let state = node.state;
        let path = node.path.as_str();
        match state.kind {
            StateKind::Final => {
                // K-001
                if !state.transitions.is_empty() {
                    diagnostics.push(FlowIssue::error(
                        "K-001",
                        path,
                        "final state must not have outgoing transitions",
                    ));
                }
            }
            StateKind::Compound => {
                // K-002
                if state.initial_state.is_none() {
                    diagnostics.push(FlowIssue::error(
                        "K-002",
                        path,
                        "compound state must declare initialState",
                    ));
                }
                if state.states.is_empty() {
                    diagnostics.push(FlowIssue::error(
                        "K-002",
                        path,
                        "compound state must declare substates in states map",
                    ));
                }
                // K-032 (same rule id as root lifecycle check): compound `initialState`
                // MUST resolve within this state's `states` map. Diagnostic `path`
                // distinguishes root `/lifecycle/initialState` vs compound state path.
                if let Some(init) = state.initial_state.as_deref() {
                    if !state.states.contains_key(init) {
                        diagnostics.push(FlowIssue::error(
                        "K-032",
                        path,
                        format!(
                            "compound initialState '{init}' is not a key in this state's `states` map"
                        ),
                    ));
                    }
                }
            }
            StateKind::Parallel => {
                // K-003
                if state.regions.is_empty() {
                    diagnostics.push(FlowIssue::error(
                        "K-003",
                        path,
                        "parallel state must declare regions",
                    ));
                }
            }
            StateKind::Atomic | StateKind::ForEach => {}
        }
        // K-004: cancellationPolicy only on parallel
        if state.cancellation_policy.is_some() && state.kind != StateKind::Parallel {
            diagnostics.push(FlowIssue::error(
                "K-004",
                path,
                "cancellationPolicy is only valid on parallel states",
            ));
        }

        // K-005: historyState only on compound
        if state.history_state.is_some() && state.kind != StateKind::Compound {
            diagnostics.push(FlowIssue::error(
                "K-005",
                path,
                "historyState is only valid on compound states",
            ));
        }

        for (i, transition) in state.transitions.iter().enumerate() {
            let t_path = format!("{path}/transitions/{i}");
            // K-006
            if !all_state_ids.contains(&transition.target) {
                diagnostics.push(FlowIssue::error(
                    "K-006",
                    &t_path,
                    format!(
                        "transition target '{}' does not exist in states map",
                        transition.target
                    ),
                ));
            }

            // K-008
            if state.kind == StateKind::Parallel && !transition.is_parallel_join_transition() {
                diagnostics.push(FlowIssue::error(
                "K-008",
                &t_path,
                format!(
                    "parallel state outgoing transition must use signal '$join' (instance scope), found {:?}",
                    transition.event
                ),
            ));
            }
        }
    }
}
fn check_lifecycle_initial_state_resolves_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<FlowIssue>,
) {
    let init = kernel.lifecycle.initial_state.as_str();
    if !kernel.lifecycle.states.contains_key(init) {
        diagnostics.push(FlowIssue::error(
            "K-032",
            "/lifecycle/initialState",
            format!(
                "lifecycle initialState '{init}' is not a key in lifecycle.states (Kernel S4.1)"
            ),
        ));
    }
}

fn check_initial_state_keys_into_states_typed(
    kernel: &KernelDocument,
    diagnostics: &mut Vec<FlowIssue>,
) {
    let init = &kernel.lifecycle.initial_state;
    if !kernel.lifecycle.states.contains_key(init) {
        diagnostics.push(FlowIssue::error(
            "K-016",
            "/lifecycle/initialState",
            format!(
                "initialState '{init}' does not exist in lifecycle.states (keys: {})",
                kernel
                    .lifecycle
                    .states
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    for (name, state) in &kernel.lifecycle.states {
        check_compound_initial_state_typed(
            state,
            &format!("/lifecycle/states/{name}"),
            diagnostics,
        );
    }
}

fn check_compound_initial_state_typed(state: &State, path: &str, diagnostics: &mut Vec<FlowIssue>) {
    if let Some(init) = &state.initial_state {
        if !state.states.contains_key(init) {
            diagnostics.push(FlowIssue::error(
                "K-016",
                &format!("{path}/initialState"),
                format!(
                    "compound initialState '{init}' does not exist in this state's substates (keys: {})",
                    state.states.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
    }
    for (name, sub) in &state.states {
        check_compound_initial_state_typed(sub, &format!("{path}/states/{name}"), diagnostics);
    }
    for (rname, region) in &state.regions {
        // Region carries its own value-references-key obligation:
        // `region.initialState` MUST key into `region.states`. Schema
        // is silent for the same reason it's silent for the compound
        // case, so K-016 covers it here.
        if !region.states.contains_key(&region.initial_state) {
            let init = &region.initial_state;
            let keys = region.states.keys().cloned().collect::<Vec<_>>().join(", ");
            diagnostics.push(FlowIssue::error(
                "K-016",
                &format!("{path}/regions/{rname}/initialState"),
                format!(
                    "region initialState '{init}' does not exist in this region's states (keys: {keys})"
                ),
            ));
        }
        for (sname, sstate) in &region.states {
            check_compound_initial_state_typed(
                sstate,
                &format!("{path}/regions/{rname}/states/{sname}"),
                diagnostics,
            );
        }
    }
    // ForEach-body recursion: `state.body` is itself a `State` that
    // may be Compound / Parallel with further nesting, including its
    // own `initialState`. Walk it like any other sub-state.
    if let Some(body) = &state.body {
        check_compound_initial_state_typed(body, &format!("{path}/body"), diagnostics);
    }
}
