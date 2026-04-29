import type { WOSKernelDocument, State, Transition, Action as WosAction, Region } from '../types/wos/kernel';

/**
 * Prefix for transition triggers on designer connections. The suffix is JSON
 * encoding a facts-tier `Transition.event` object so round-trips preserve
 * message vs signal vs timer/condition/error shapes (Kernel §4.5–§4.10).
 */
const DESIGNER_EVENT_TRIGGER_PREFIX = '__wos_te_v1:';

/** Wire value for transitions with no `event` (guard-only / continuous rescan). */
const DESIGNER_NO_EVENT_TRIGGER = '__wos_no_event__';

/**
 * Placeholder `TransitionEventError.code` when a legacy designer connection used
 * the bare kernel dispatch label `$error` with no typed payload (Kernel §4.10).
 */
const LEGACY_BARE_ERROR_CODE = 'wos.designer.unspecified';

/**
 * Synthetic error code when the `__wos_te_v1:` suffix is not valid JSON — avoids
 * throwing on corrupt or crafted `trigger` strings.
 */
const CORRUPT_DESIGNER_TRIGGER_JSON_CODE = 'wos.designer.invalid_trigger_json';

/** Serialize `Transition.event` for `WorkflowConnection.trigger` (string wire). */
function transitionEventToTriggerString(event: Transition['event'] | undefined): string {
  if (event === undefined) return DESIGNER_NO_EVENT_TRIGGER;
  // Historically the kernel allowed `event` as a plain string (message name).
  // On the wire we always emit the prefixed JSON shape so round-trips preserve
  // the message kind and forbid ambiguous `$`-prefixed names at the facts tier.
  if (typeof event === 'string') {
    const legacy = { kind: 'message' as const, name: event };
    return `${DESIGNER_EVENT_TRIGGER_PREFIX}${JSON.stringify(legacy)}`;
  }
  return `${DESIGNER_EVENT_TRIGGER_PREFIX}${JSON.stringify(event)}`;
}

/** Inverse of {@link transitionEventToTriggerString} with legacy plain-name fallbacks. */
function triggerStringToTransitionEvent(trigger: string): Transition['event'] | undefined {
  if (trigger === DESIGNER_NO_EVENT_TRIGGER) return undefined;
  if (trigger.startsWith(DESIGNER_EVENT_TRIGGER_PREFIX)) {
    const json = trigger.slice(DESIGNER_EVENT_TRIGGER_PREFIX.length);
    try {
      return JSON.parse(json) as Transition['event'];
    } catch {
      return { kind: 'error', code: CORRUPT_DESIGNER_TRIGGER_JSON_CODE };
    }
  }
  // Bare `$error` is the kernel error class dispatch label, not a signal name (Kernel §4.10).
  if (trigger === '$error') {
    return { kind: 'error', code: LEGACY_BARE_ERROR_CODE };
  }
  if (trigger === '$join' || trigger === '$compensation.complete' || trigger.startsWith('$')) {
    return { kind: 'signal', name: trigger, scope: 'instance' };
  }
  return { kind: 'message', name: trigger };
}

function defaultSyntheticTrigger(from: string, to: string): string {
  return transitionEventToTriggerString({ kind: 'message', name: `${from}_to_${to}` });
}

export interface WorkflowStage {
  id: string;
  name: string;
  type: string;
  description?: string;
  parentId?: string;
  position: { x: number; y: number };
  config: {
    assignee?: { type: string; id: string; label: string };
    steps?: string[];
    activities?: string[];
    wosTags?: string[];
    [key: string]: unknown;
  };
}

export interface WorkflowConnection {
  id: string;
  from: string;
  to: string;
  condition?: string;
  trigger?: string;
}

export interface DesignerWorkflow {
  id: string;
  name: string;
  version: string;
  status: 'draft' | 'published' | 'archived';
  stages: WorkflowStage[];
  connections: WorkflowConnection[];
  lastModified: string;
  author: string;
}

const LAYOUT_SPACING_X = 220;
const LAYOUT_SPACING_Y = 160;

type StageType = WorkflowStage['type'];

function wosStateTypeToStageType(state: State): StageType {
  switch (state.type) {
    case 'parallel': return 'parallel';
    case 'compound': return 'split';
    case 'final': return 'final';
    default: {
      if (state.onEntry?.some(a => a.action === 'invokeService')) return 'ai-pipeline';
      if (state.tags?.includes('review') || state.tags?.includes('determination')) return 'decision';
      if (state.onEntry?.some(a => a.action === 'startTimer')) return 'timer';
      return 'simple';
    }
  }
}

function flattenStates(
  states: Record<string, State>,
  prefix: string = '',
): { id: string; state: State; depth: number }[] {
  const result: { id: string; state: State; depth: number }[] = [];
  for (const [id, state] of Object.entries(states)) {
    const fullId = prefix ? `${prefix}.${id}` : id;
    result.push({ id: fullId, state, depth: prefix.split('.').length });
    if (state.states) {
      result.push(...flattenStates(state.states, fullId));
    }
    if (state.regions) {
      for (const [regionId, region] of Object.entries(state.regions)) {
        result.push(...flattenStates(region.states, `${fullId}.${regionId}`));
      }
    }
  }
  return result;
}

function collectTransitions(
  states: Record<string, State>,
  prefix: string = '',
): { from: string; transition: Transition }[] {
  const result: { from: string; transition: Transition }[] = [];
  for (const [id, state] of Object.entries(states)) {
    const fullId = prefix ? `${prefix}.${id}` : id;
    if (state.transitions) {
      for (const t of state.transitions) {
        result.push({ from: fullId, transition: t });
      }
    }
    if (state.states) {
      result.push(...collectTransitions(state.states, fullId));
    }
    if (state.regions) {
      for (const [regionId, region] of Object.entries(state.regions)) {
        result.push(...collectTransitions(region.states, `${fullId}.${regionId}`));
      }
    }
  }
  return result;
}

export interface KernelToDesignerResult {
  stages: WorkflowStage[];
  connections: WorkflowConnection[];
}

export function kernelToDesigner(kernel: WOSKernelDocument): KernelToDesignerResult {
  if (!kernel.lifecycle?.states) {
    return { stages: [], connections: [] };
  }
  const flat = flattenStates(kernel.lifecycle.states);
  const allTransitions = collectTransitions(kernel.lifecycle.states);

  const colCounts = new Map<number, number>();
  const stages: WorkflowStage[] = flat.map(({ id, state }) => {
    const depth = id.split('.').length - 1;
    const col = colCounts.get(depth) ?? 0;
    colCounts.set(depth, col + 1);

    const stageType = wosStateTypeToStageType(state);
    const assignee = state.onEntry?.find(a => a.action === 'createTask')?.assignTo;

    return {
      id,
      name: getWosStateDisplayName(id),
      type: stageType,
      description: state.description,
      ...(id.includes('.') ? { parentId: id.substring(0, id.lastIndexOf('.')) } : {}),
      position: {
        x: depth * LAYOUT_SPACING_X + 50,
        y: col * LAYOUT_SPACING_Y + 50,
      },
      config: {
        ...(assignee ? {
          assignee: {
            type: 'individual' as const,
            id: assignee,
            label: assignee,
          },
        } : {}),
        ...(stageType === 'ai-pipeline' ? {
          steps: state.onEntry
            ?.filter(a => a.action === 'invokeService')
            .map(a => a.serviceRef ?? 'Service') ?? [],
        } : {}),
        wosTags: state.tags,
      },
    };
  });

  const stageIds = new Set(stages.map(s => s.id));
  const connections: WorkflowConnection[] = [];
  let connIdx = 0;
  for (const { from, transition } of allTransitions) {
    if (!stageIds.has(from)) continue;
    const targetId = resolveTransitionTarget(stageIds, from, transition.target);
    if (targetId === null) continue;
    connections.push({
      id: `conn-${connIdx++}`,
      from,
      to: targetId,
      condition: transition.guard,
      trigger: transitionEventToTriggerString(transition.event),
    });
  }

  return { stages, connections };
}

/**
 * Resolve a transition's `target` against the scope of its `from` state. WOS
 * targets are scope-local; the designer works in a flat namespace, so we have
 * to walk up the `from` state's ancestor chain to find a matching stage.
 *
 * Returns the fully-qualified stage id that the designer should connect to,
 * or the original target if it's a sentinel (`$join`, `$fork`, etc.), or
 * `null` if no stage matches.
 */
function resolveTransitionTarget(stageIds: Set<string>, from: string, target: string): string | null {
  if (target.startsWith('$')) return target;
  if (stageIds.has(target)) return target;
  const fromParts = from.split('.');
  // Try each ancestor scope, starting from the closest.
  for (let i = fromParts.length - 1; i >= 0; i--) {
    const candidate = [...fromParts.slice(0, i), target].join('.');
    if (stageIds.has(candidate)) return candidate;
  }
  return null;
}

/**
 * Inverse of resolveTransitionTarget: given a fully-qualified connection
 * target, express it as a scope-local string relative to `from`. This keeps
 * the round-tripped kernel textually close to the original (targets that
 * were scope-local stay scope-local).
 */
function scopeLocalTarget(from: string, target: string): string {
  if (target.startsWith('$')) return target;
  const fromScope = from.split('.').slice(0, -1);
  const targetParts = target.split('.');
  let i = 0;
  while (i < fromScope.length && i < targetParts.length - 1 && fromScope[i] === targetParts[i]) i++;
  if (i === fromScope.length) return targetParts.slice(i).join('.');
  return target;
}

export function getWosStateDisplayName(stateId: string): string {
  const parts = stateId.split('.');
  const raw = parts[parts.length - 1].replace(/([A-Z])/g, ' $1').trim();
  return raw.charAt(0).toUpperCase() + raw.slice(1);
}

const STAGE_TYPE_TO_WOS: Record<StageType, State['type']> = {
  'simple': 'atomic',
  'parallel': 'parallel',
  'adaptive': 'atomic',
  'ai-pipeline': 'atomic',
  'final': 'final',
  'split': 'compound',
  'join': 'atomic',
  'decision': 'atomic',
  'timer': 'atomic',
  'api': 'atomic',
  'sub-workflow': 'compound',
};

interface TopologyLocation {
  state: State;
  path: string[];
  kind: 'state' | 'region-state';
  parentKind: 'root' | 'compound' | 'region' | 'parallel';
}

function indexTopology(states: Record<string, State>): Map<string, TopologyLocation> {
  const index = new Map<string, TopologyLocation>();
  function walk(
    map: Record<string, State>,
    prefix: string[],
    parentKind: TopologyLocation['parentKind'],
    kind: TopologyLocation['kind'],
  ): void {
    for (const [id, state] of Object.entries(map)) {
      const path = [...prefix, id];
      index.set(path.join('.'), { state, path, kind, parentKind });
      if (state.states) {
        walk(state.states, path, 'compound', 'state');
      }
      if (state.regions) {
        for (const [regionId, region] of Object.entries(state.regions)) {
          walk(region.states, [...path, regionId], 'region', 'region-state');
        }
      }
    }
  }
  walk(states, [], 'root', 'state');
  return index;
}

/**
 * Build a state for a designer stage that has no counterpart in the base
 * kernel (a designer-added state). Only used on the "new state" path. For
 * stages that existed in the base kernel, use overlayDesignerEdits instead
 * so we don't discard the original's transitions, actions, or extensions.
 */
function buildStateFromStage(stage: WorkflowStage, connections: WorkflowConnection[]): State {
  const wosType = STAGE_TYPE_TO_WOS[stage.type] ?? 'atomic';
  const transitions: Transition[] = connections
    .filter(c => c.from === stage.id)
    .map(c => {
      const rawTrigger = c.trigger ?? defaultSyntheticTrigger(c.from, c.to);
      const ev = triggerStringToTransitionEvent(rawTrigger);
      const t: Transition = {
        target: scopeLocalTarget(c.from, c.to),
        ...(ev !== undefined ? { event: ev } : {}),
      };
      if (c.condition) t.guard = c.condition;
      return t;
    });

  const onEntry: WosAction[] = [];
  if (stage.config.assignee) {
    onEntry.push({ action: 'createTask', taskRef: stage.name, assignTo: stage.config.assignee.id });
  }
  if (stage.type === 'ai-pipeline' && stage.config.steps) {
    for (const step of stage.config.steps) {
      onEntry.push({ action: 'invokeService', serviceRef: step });
    }
  }

  const state: State = { type: wosType };
  if (stage.description) state.description = stage.description;
  if (onEntry.length > 0) state.onEntry = onEntry;
  if (transitions.length > 0) state.transitions = transitions;
  if (stage.config.wosTags) state.tags = stage.config.wosTags;
  return state;
}

/**
 * Overlay designer edits on top of the original state object from the base
 * kernel. Preserves the original's `onEntry`, `onExit`, `transitions`, tags,
 * extensions, regions, substates, and metadata — anything the designer is
 * not empowered to edit stays byte-for-byte identical. Only the following
 * are rewritten from the designer's view:
 *
 *   - description
 *   - the first createTask's assignTo (assignee changed)
 *   - invokeService actions for ai-pipeline (steps changed)
 *   - transitions (only when the set of outgoing connections has diverged)
 *
 * This is a diff-preserving merge, not a regeneration.
 */
function overlayDesignerEdits(original: State, stage: WorkflowStage, connections: WorkflowConnection[]): State {
  const merged: State = { ...original };

  // description: last-writer-wins from the designer, but preserve `undefined`
  // (don't write an empty string into the output).
  if (stage.description !== undefined) {
    if (stage.description === '') delete merged.description;
    else merged.description = stage.description;
  }

  // tags: designer's wosTags are authoritative if present.
  if (stage.config.wosTags !== undefined) {
    if (stage.config.wosTags.length === 0) delete merged.tags;
    else merged.tags = stage.config.wosTags;
  }

  // assignee: find the first createTask in onEntry and update its assignTo
  // in place. If the designer cleared the assignee, we don't remove the
  // createTask action (the kernel may legitimately assign via other means).
  if (original.onEntry) {
    const newOnEntry: WosAction[] = original.onEntry.map((a) => ({ ...a }));
    if (stage.config.assignee) {
      const createTaskIdx = newOnEntry.findIndex(a => a.action === 'createTask');
      if (createTaskIdx >= 0) {
        newOnEntry[createTaskIdx] = { ...newOnEntry[createTaskIdx], assignTo: stage.config.assignee.id };
      }
    }
    // ai-pipeline steps: rewrite the invokeService actions only if the designer
    // has steps and they differ from the originals.
    if (stage.type === 'ai-pipeline' && stage.config.steps) {
      const existingSteps = newOnEntry
        .filter(a => a.action === 'invokeService')
        .map(a => a.serviceRef ?? 'Service');
      const changed =
        existingSteps.length !== stage.config.steps.length ||
        existingSteps.some((s, i) => s !== stage.config.steps![i]);
      if (changed) {
        const withoutInvoke = newOnEntry.filter(a => a.action !== 'invokeService');
        for (const step of stage.config.steps) {
          withoutInvoke.push({ action: 'invokeService', serviceRef: step });
        }
        merged.onEntry = withoutInvoke;
      } else {
        merged.onEntry = newOnEntry;
      }
    } else {
      merged.onEntry = newOnEntry;
    }
  } else if (stage.config.assignee) {
    // Base had no onEntry but the designer set an assignee — synthesize.
    merged.onEntry = [{ action: 'createTask', taskRef: stage.name, assignTo: stage.config.assignee.id }];
  }

  // transitions: if the designer's connection set from this stage matches the
  // original (by event + resolved target), preserve the original transitions
  // verbatim (including descriptions, compensating actions, extensions).
  // Otherwise rebuild from connections — the user has explicitly edited them.
  const outgoing = connections.filter(c => c.from === stage.id);

  // We can only compare resolved targets if we have a topology context. In
  // overlay mode we match against the connection's `from` id to resolve the
  // original target. For that, we need stageIds — passed down by caller via
  // closure below.
  if (compareConnectionSet(outgoing, original.transitions ?? [], stage.id)) {
    if (original.transitions) merged.transitions = original.transitions.map(t => ({ ...t }));
    else delete merged.transitions;
  } else {
    const rebuilt = outgoing.map(c => {
      const rawTrigger = c.trigger ?? defaultSyntheticTrigger(c.from, c.to);
      const ev = triggerStringToTransitionEvent(rawTrigger);
      const t: Transition = {
        target: scopeLocalTarget(c.from, c.to),
        ...(ev !== undefined ? { event: ev } : {}),
      };
      if (c.condition) t.guard = c.condition;
      return t;
    });
    if (rebuilt.length > 0) merged.transitions = rebuilt;
    else delete merged.transitions;
  }

  return merged;
}

/**
 * Returns true when the designer's outgoing connections describe the same
 * set of transitions as the original state. Uses (event, resolved target)
 * as the identity tuple. When true, the caller should preserve original
 * transitions verbatim rather than rebuild.
 */
function compareConnectionSet(
  outgoing: WorkflowConnection[],
  originalTransitions: Transition[],
  fromId: string,
): boolean {
  if (outgoing.length !== originalTransitions.length) return false;
  const originalKeys = new Set(
    originalTransitions.map(t => `${transitionEventToTriggerString(t.event)}::${t.target}`),
  );
  for (const c of outgoing) {
    const localTarget = scopeLocalTarget(c.from, c.to);
    const wireTrigger = c.trigger ?? defaultSyntheticTrigger(c.from, c.to);
    const key = `${wireTrigger}::${localTarget}`;
    if (!originalKeys.has(key)) return false;
    // Also verify guard equivalence when present.
    const matching = originalTransitions.find(t =>
      transitionEventToTriggerString(t.event) === wireTrigger && t.target === localTarget,
    );
    if (matching && (matching.guard ?? undefined) !== (c.condition ?? undefined)) return false;
  }
  // Also fail if we truncated the fromId check — sanity.
  if (fromId.length === 0) return false;
  return true;
}

const EMBEDDED_BLOCK_KEYS = new Set([
  'governance', 'agents', 'aiOversight', 'signature', 'custody',
  'advanced', 'assurance', 'provenance', 'outputBindings', 'extensions',
]);

function passThroughEmbedded(base?: WOSKernelDocument): Record<string, unknown> {
  if (!base) return {};
  const extra: Record<string, unknown> = {};
  const dyn = base as unknown as Record<string, unknown>;
  for (const key of EMBEDDED_BLOCK_KEYS) {
    if (dyn[key] !== undefined) {
      extra[key] = dyn[key];
    }
  }
  return extra;
}

function cloneStateMap(states: Record<string, State>): Record<string, State> {
  return JSON.parse(JSON.stringify(states));
}

export function designerToKernel(
  workflow: DesignerWorkflow,
  baseKernel?: WOSKernelDocument,
): WOSKernelDocument {
  const stageById = new Map(workflow.stages.map(s => [s.id, s]));

  let rootStates: Record<string, State>;
  let initialState: string;

  if (baseKernel?.lifecycle?.states) {
    const baseStates = cloneStateMap(baseKernel.lifecycle.states);
    const topology = indexTopology(baseStates);

    // Update every existing state in-place with the matching stage's editable fields.
    // Delete states whose path is no longer in the designer.
    function updateAndPrune(
      map: Record<string, State>,
      prefix: string[],
    ): void {
      for (const id of Object.keys(map)) {
        const path = [...prefix, id];
        const fullId = path.join('.');
        const stage = stageById.get(fullId);
        const state = map[id];

        if (!stage) {
          delete map[id];
          continue;
        }

        // Overlay designer edits on top of the original state, keeping
        // original onEntry/onExit/transitions/extensions/metadata intact
        // unless the designer explicitly changed them.
        map[id] = overlayDesignerEdits(state, stage, workflow.connections);

        if (map[id].states) {
          updateAndPrune(map[id].states as Record<string, State>, path);
          if (Object.keys(map[id].states as Record<string, State>).length === 0) {
            delete (map[id] as State).states;
            delete (map[id] as State).initialState;
          }
        }
        if (map[id].regions) {
          const regions = map[id].regions as Record<string, Region>;
          for (const regionId of Object.keys(regions)) {
            updateAndPrune(regions[regionId].states, [...path, regionId]);
            if (Object.keys(regions[regionId].states).length === 0) {
              delete regions[regionId];
            }
          }
          if (Object.keys(regions).length === 0) {
            delete (map[id] as State).regions;
          }
        }
      }
    }

    updateAndPrune(baseStates, []);

    // Add any stages that had no base counterpart (designer-added).
    for (const stage of workflow.stages) {
      if (!topology.has(stage.id) && !stage.id.includes('.')) {
        if (!baseStates[stage.id]) {
          baseStates[stage.id] = buildStateFromStage(stage, workflow.connections);
        }
      }
    }

    rootStates = baseStates;
    initialState = baseKernel.lifecycle.initialState
      ?? workflow.stages[0]?.id?.split('.')[0]
      ?? Object.keys(rootStates)[0]
      ?? 'start';
  } else {
    // No baseKernel: treat all stages as flat top-level states.
    rootStates = {};
    for (const stage of workflow.stages) {
      if (!stage.id.includes('.')) {
        rootStates[stage.id] = buildStateFromStage(stage, workflow.connections);
      }
    }
    initialState = workflow.stages[0]?.id ?? Object.keys(rootStates)[0] ?? 'start';
  }

  const kernel: WOSKernelDocument = {
    ...(baseKernel ? { $wosWorkflow: baseKernel.$wosWorkflow } : { $wosWorkflow: '1.0' }),
    ...(baseKernel?.$schema ? { $schema: baseKernel.$schema } : {}),
    url: workflow.id,
    version: workflow.version,
    title: workflow.name,
    status: workflow.status === 'published' ? 'active' : workflow.status === 'archived' ? 'retired' : 'draft',
    ...(baseKernel?.impactLevel ? { impactLevel: baseKernel.impactLevel } : {}),
    ...(baseKernel?.actors ? { actors: baseKernel.actors } : {}),
    lifecycle: {
      initialState,
      states: rootStates,
    },
    ...(baseKernel?.caseFile ? { caseFile: baseKernel.caseFile } : {}),
    ...(baseKernel?.contracts ? { contracts: baseKernel.contracts } : {}),
    ...passThroughEmbedded(baseKernel),
  };

  return kernel;
}
