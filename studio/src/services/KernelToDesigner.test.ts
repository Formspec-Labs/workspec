import { describe, it, expect } from 'vitest';
import { kernelToDesigner, designerToKernel } from './KernelToDesigner';
import { validateKernelDocument } from './wos-kernel-validator';
import type { WOSKernelDocument, State } from '../types/wos/kernel';
import { loadBenefitsAdjudicationBundle } from '../data/fixtures';

/** Strip keys whose value is `undefined` so `toEqual` behaves predictably. */
function stripUndefined<T>(value: T): T {
  if (Array.isArray(value)) return value.map(stripUndefined) as unknown as T;
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (v === undefined) continue;
      out[k] = stripUndefined(v);
    }
    return out as T;
  }
  return value;
}

function makeKernel(states: WOSKernelDocument['lifecycle']['states'], initialState?: string): WOSKernelDocument {
  const firstKey = initialState ?? Object.keys(states)[0] ?? 'start';
  return {
    $wosKernel: '1.0',
    url: 'test://workflow',
    version: '1.0.0',
    title: 'Test Workflow',
    status: 'draft',
    impactLevel: 'operational',
    actors: [{ id: 'worker', type: 'human' }],
    lifecycle: { initialState: firstKey, states },
  };
}

function designerFromKernel(kernel: WOSKernelDocument) {
  const { stages, connections } = kernelToDesigner(kernel);
  return {
    id: kernel.url,
    name: kernel.title,
    version: kernel.version,
    status: 'draft' as const,
    stages,
    connections,
    lastModified: new Date().toISOString(),
    author: 'test',
  };
}

function getState(kernel: WOSKernelDocument, path: string[]): State | undefined {
  let current: Record<string, State> | undefined = kernel.lifecycle?.states;
  let state: State | undefined;
  for (let i = 0; i < path.length; i++) {
    if (!current) return undefined;
    state = current[path[i]];
    if (!state) return undefined;
    if (i < path.length - 1) {
      current = state.states;
    }
  }
  return state;
}

describe('KernelToDesigner round-trip', () => {
  it('round-trips a flat kernel (atomic states only) and validates against the schema', () => {
    const kernel = makeKernel({
      intake: { type: 'atomic', transitions: [{ event: 'submit', target: 'review' }] },
      review: { type: 'atomic', transitions: [{ event: 'approve', target: 'done' }] },
      done: { type: 'final' },
    }, 'intake');

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    expect(Object.keys(roundTripped.lifecycle.states).sort()).toEqual(['done', 'intake', 'review']);
    expect(roundTripped.lifecycle.initialState).toBe('intake');
    expect(validateKernelDocument(roundTripped).isValid).toBe(true);
  });

  it('preserves compound initialState, transitions, and substate shape through round-trip', () => {
    const kernel = makeKernel({
      intake: { type: 'atomic', transitions: [{ event: 'submit', target: 'review' }] },
      review: {
        type: 'compound',
        initialState: 'gathering',
        states: {
          gathering: { type: 'atomic', transitions: [{ event: 'ready', target: 'deliberating' }] },
          deliberating: { type: 'atomic', transitions: [{ event: 'decide', target: 'done' }] },
        },
      },
      done: { type: 'final' },
    }, 'intake');

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    const review = roundTripped.lifecycle.states.review;
    expect(review.type).toBe('compound');
    expect(review.initialState).toBe('gathering');
    expect(review.regions).toBeUndefined();

    const gathering = review.states?.gathering;
    const deliberating = review.states?.deliberating;
    expect(gathering?.transitions).toEqual([{ event: 'ready', target: 'deliberating' }]);
    expect(deliberating?.transitions).toEqual([{ event: 'decide', target: 'done' }]);

    expect(validateKernelDocument(roundTripped).isValid).toBe(true);
  });

  it('preserves parallel regions, cancellationPolicy, and region-local transitions', () => {
    const kernel = makeKernel({
      intake: { type: 'atomic', transitions: [{ event: 'submit', target: 'parallelReview' }] },
      parallelReview: {
        type: 'parallel',
        cancellationPolicy: 'wait-all',
        regions: {
          pathA: {
            initialState: 'stepA1',
            states: { stepA1: { type: 'atomic', transitions: [{ event: 'doneA', target: 'stepADone' }] }, stepADone: { type: 'final' } },
          },
          pathB: {
            initialState: 'stepB1',
            states: { stepB1: { type: 'atomic', transitions: [{ event: 'doneB', target: 'stepBDone' }] }, stepBDone: { type: 'final' } },
          },
        },
        transitions: [{ event: '$join', target: 'done' }],
      },
      done: { type: 'final' },
    }, 'intake');

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    const parallel = roundTripped.lifecycle.states.parallelReview;
    expect(parallel.type).toBe('parallel');
    expect(parallel.cancellationPolicy).toBe('wait-all');
    expect(parallel.states).toBeUndefined();
    expect(parallel.regions).toBeDefined();

    // Region-local transitions must survive verbatim.
    expect(parallel.regions?.pathA.states.stepA1.transitions).toEqual([
      { event: 'doneA', target: 'stepADone' },
    ]);
    expect(parallel.regions?.pathB.states.stepB1.transitions).toEqual([
      { event: 'doneB', target: 'stepBDone' },
    ]);
    // The outer parallel-state transition (`$join`) is preserved.
    expect(parallel.transitions).toEqual([{ event: '$join', target: 'done' }]);

    expect(validateKernelDocument(roundTripped).isValid).toBe(true);
  });

  it('handles kernelToDesigner with empty lifecycle states', () => {
    const kernel = makeKernel({});
    const result = kernelToDesigner(kernel);
    expect(result.stages).toEqual([]);
    expect(result.connections).toEqual([]);
  });

  it('round-trips the benefits adjudication fixture kernel and validates against the schema', () => {
    const bundle = loadBenefitsAdjudicationBundle();
    const kernel = bundle.kernel;

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    const eligibility = getState(roundTripped, ['eligibilityReview']);
    expect(eligibility?.type).toBe('parallel');
    expect(eligibility?.regions).toBeDefined();
    expect(eligibility?.regions?.reviewerA.initialState).toBe('pendingReviewA');
    expect(eligibility?.regions?.reviewerB.initialState).toBe('pendingReviewB');

    const result = validateKernelDocument(roundTripped);
    if (!result.isValid) {
      // Surface issues to aid debugging if the assertion fails.
      console.error('round-trip schema issues:', result.issues.slice(0, 10));
    }
    expect(result.isValid).toBe(true);
  });

  it('preserves region-local transitions, taskRef, and onEntry actions on benefits fixture', () => {
    // This is the regression test for the round-trip's silent data loss:
    // region-scoped transitions and task references must survive without
    // being dropped or rewritten.
    const bundle = loadBenefitsAdjudicationBundle();
    const kernel = bundle.kernel;

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    // Region-local transition — the canonical failure mode.
    const pendingReviewA = kernel.lifecycle.states.eligibilityReview.regions!.reviewerA.states.pendingReviewA;
    const rtPendingReviewA = roundTripped.lifecycle.states.eligibilityReview.regions!.reviewerA.states.pendingReviewA;
    expect(rtPendingReviewA.transitions).toEqual(pendingReviewA.transitions);

    // createTask onEntry must preserve the original taskRef (not be renamed
    // to the stage's leaf name).
    expect(rtPendingReviewA.onEntry).toEqual(pendingReviewA.onEntry);
    const createTask = rtPendingReviewA.onEntry?.find(a => a.action === 'createTask');
    expect(createTask?.taskRef).toBe('eligibilityDetermination');
    expect(createTask?.assignTo).toBe('caseworkerA');
  });

  it('preserves the entire benefits fixture byte-for-byte when the designer makes no edits', () => {
    // Stronger guarantee: a null-edit round-trip should be an identity on
    // every state that appears in the designer view.
    const bundle = loadBenefitsAdjudicationBundle();
    const kernel = bundle.kernel;

    const roundTripped = designerToKernel(designerFromKernel(kernel), kernel);

    // Compare each state that kernelToDesigner emits as a stage. Ignore
    // top-level metadata (url/title/version could legitimately change).
    function walk(originalMap: Record<string, State>, rtMap: Record<string, State>, pathPrefix: string[]) {
      for (const id of Object.keys(originalMap)) {
        const path = [...pathPrefix, id];
        const orig = originalMap[id];
        const rt = rtMap[id];
        expect(rt, `state missing at ${path.join('.')}`).toBeDefined();
        // Deep comparison of editable-preserved fields.
        expect(stripUndefined(rt.transitions), `transitions drift at ${path.join('.')}`).toEqual(stripUndefined(orig.transitions));
        expect(stripUndefined(rt.onEntry), `onEntry drift at ${path.join('.')}`).toEqual(stripUndefined(orig.onEntry));
        expect(stripUndefined(rt.onExit), `onExit drift at ${path.join('.')}`).toEqual(stripUndefined(orig.onExit));
        expect(rt.tags, `tags drift at ${path.join('.')}`).toEqual(orig.tags);
        expect(rt.description, `description drift at ${path.join('.')}`).toEqual(orig.description);
        expect(rt.initialState, `initialState drift at ${path.join('.')}`).toEqual(orig.initialState);
        expect(rt.cancellationPolicy, `cancellationPolicy drift at ${path.join('.')}`).toEqual(orig.cancellationPolicy);
        expect(rt.historyState, `historyState drift at ${path.join('.')}`).toEqual(orig.historyState);
        if (orig.states) walk(orig.states, rt.states ?? {}, path);
        if (orig.regions) {
          for (const regionId of Object.keys(orig.regions)) {
            const origRegion = orig.regions[regionId];
            const rtRegion = rt.regions?.[regionId];
            expect(rtRegion, `region missing at ${path.join('.')}.${regionId}`).toBeDefined();
            expect(rtRegion!.initialState).toBe(origRegion.initialState);
            walk(origRegion.states, rtRegion!.states, [...path, regionId]);
          }
        }
      }
    }
    walk(kernel.lifecycle.states, roundTripped.lifecycle.states, []);
  });
});
