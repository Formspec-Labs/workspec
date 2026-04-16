import { describe, it, expect } from 'vitest';
import { kernelToDesigner, designerToKernel } from './KernelToDesigner';
import { validateKernelDocument } from './wos-kernel-validator';
import type { WOSKernelDocument, State } from '../types/wos/kernel';
import { loadBenefitsAdjudicationBundle } from '../data/fixtures';

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

  it('preserves compound initialState and substate shape through round-trip', () => {
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
    expect(Object.keys(review.states ?? {}).sort()).toEqual(['deliberating', 'gathering']);
    expect(review.regions).toBeUndefined();
    expect(validateKernelDocument(roundTripped).isValid).toBe(true);
  });

  it('preserves parallel regions and cancellationPolicy through round-trip', () => {
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
    expect(Object.keys(parallel.regions ?? {}).sort()).toEqual(['pathA', 'pathB']);
    expect(parallel.regions?.pathA.initialState).toBe('stepA1');
    expect(parallel.regions?.pathA.states.stepA1.type).toBe('atomic');
    expect(parallel.regions?.pathB.initialState).toBe('stepB1');
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
});
