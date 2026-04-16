import type { WOSKernelDocument, State, Transition, Action as WosAction, Region } from '../types/wos/kernel';

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
      name: id.split('.').pop() ?? id,
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
    const targetId = transition.target;
    if (stageIds.has(from) && (stageIds.has(targetId) || targetId.startsWith('$'))) {
      connections.push({
        id: `conn-${connIdx++}`,
        from,
        to: targetId,
        condition: transition.guard,
        trigger: transition.event,
      });
    }
  }

  return { stages, connections };
}

export function getWosStateDisplayName(stateId: string): string {
  const parts = stateId.split('.');
  return parts[parts.length - 1].replace(/([A-Z])/g, ' $1').trim();
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

function buildStateFromStage(
  stage: WorkflowStage,
  connections: WorkflowConnection[],
  preservedShape?: Pick<State, 'type' | 'initialState' | 'states' | 'regions' | 'cancellationPolicy' | 'historyState'>,
): State {
  const wosType = preservedShape?.type ?? STAGE_TYPE_TO_WOS[stage.type] ?? 'atomic';
  const transitions: Transition[] = connections
    .filter(c => c.from === stage.id)
    .map(c => {
      const t: Transition = { event: c.trigger ?? `${c.from}_to_${c.to}`, target: c.to };
      if (c.condition) t.guard = c.condition;
      return t;
    });

  const onEntry: WosAction[] = [];
  if (stage.config.assignee) {
    onEntry.push({
      action: 'createTask',
      taskRef: stage.name,
      assignTo: stage.config.assignee.id,
    });
  }
  if (stage.type === 'ai-pipeline' && stage.config.steps) {
    for (const step of stage.config.steps) {
      onEntry.push({ action: 'invokeService', serviceRef: step });
    }
  }

  const state: State = {
    type: wosType,
    ...(stage.description ? { description: stage.description } : {}),
    ...(onEntry.length > 0 ? { onEntry } : {}),
    ...(transitions.length > 0 ? { transitions } : {}),
    ...(stage.config.wosTags ? { tags: stage.config.wosTags } : {}),
    ...(preservedShape?.initialState !== undefined ? { initialState: preservedShape.initialState } : {}),
    ...(preservedShape?.states !== undefined ? { states: preservedShape.states } : {}),
    ...(preservedShape?.regions !== undefined ? { regions: preservedShape.regions } : {}),
    ...(preservedShape?.cancellationPolicy !== undefined ? { cancellationPolicy: preservedShape.cancellationPolicy } : {}),
    ...(preservedShape?.historyState !== undefined ? { historyState: preservedShape.historyState } : {}),
  };

  return state;
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

        const preserved: Parameters<typeof buildStateFromStage>[2] = {
          type: state.type,
          ...(state.initialState !== undefined ? { initialState: state.initialState } : {}),
          ...(state.states !== undefined ? { states: state.states } : {}),
          ...(state.regions !== undefined ? { regions: state.regions } : {}),
          ...(state.cancellationPolicy !== undefined ? { cancellationPolicy: state.cancellationPolicy } : {}),
          ...(state.historyState !== undefined ? { historyState: state.historyState } : {}),
        };
        map[id] = buildStateFromStage(stage, workflow.connections, preserved);

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
    ...(baseKernel ? { $wosKernel: baseKernel.$wosKernel } : { $wosKernel: '1.0' }),
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
  };

  return kernel;
}
