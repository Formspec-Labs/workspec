// @vitest-environment node
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { startServer, type StartedServer } from '../../server';

let server: StartedServer;
let baseUrl: string;

beforeAll(async () => {
  // Port 0 lets the OS pick a free port; attachVite is disabled so tests don't
  // boot Vite; persistMode: 'memory' prevents kernel PUTs from overwriting
  // committed fixture files in the repo.
  server = await startServer({
    port: 0,
    attachVite: false,
    serveStatic: false,
    apiToken: undefined,
    persistMode: 'memory',
  });
  baseUrl = `http://127.0.0.1:${server.addressPort}`;
}, 20_000);

afterAll(async () => {
  await server.close();
});

async function apiGet(path: string): Promise<{ status: number; body: any }> {
  const res = await fetch(`${baseUrl}${path}`);
  const body = res.headers.get('content-type')?.includes('application/json') ? await res.json() : await res.text();
  return { status: res.status, body };
}

async function apiJson(method: 'POST' | 'PUT' | 'DELETE', path: string, body?: unknown): Promise<{ status: number; body: any }> {
  const res = await fetch(`${baseUrl}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const parsed = res.headers.get('content-type')?.includes('application/json') ? await res.json() : await res.text();
  return { status: res.status, body: parsed };
}

const benefitsUrl = 'https://agency.gov/workflows/benefits-adjudication';

describe('real server — kernel bundle endpoints', () => {
  it('lists loaded kernel bundles with benefits-adjudication', async () => {
    const { status, body } = await apiGet('/api/bundles');
    expect(status).toBe(200);
    expect(Array.isArray(body)).toBe(true);
    const benefits = body.find((b: { url: string }) => b.url === benefitsUrl);
    expect(benefits).toBeTruthy();
    expect(benefits.title).toContain('Benefits Adjudication');
  });

  it('returns a full bundle by URL', async () => {
    const { status, body } = await apiGet(`/api/bundles/${encodeURIComponent(benefitsUrl)}`);
    expect(status).toBe(200);
    expect(body.kernel.$wosKernel).toBe('1.0');
    expect(body.kernel.url).toBe(benefitsUrl);
    // Sidecars load (governance exists at that path)
    expect(body.governance).toBeDefined();
  });

  it('returns just the kernel via /bundles/:url/kernel', async () => {
    const { status, body } = await apiGet(`/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`);
    expect(status).toBe(200);
    expect(body.$wosKernel).toBe('1.0');
    expect(body.lifecycle.states.eligibilityReview.type).toBe('parallel');
  });

  it('404s on unknown bundle URL', async () => {
    const { status } = await apiGet(`/api/bundles/${encodeURIComponent('https://does-not-exist.example/x')}`);
    expect(status).toBe(404);
  });

  it('rejects PUT with mismatched URL body', async () => {
    const { status, body } = await apiJson('PUT', `/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`, {
      $wosKernel: '1.0',
      url: 'https://other/',
      version: '1.0.0',
      title: 'wrong',
      status: 'active',
      lifecycle: { initialState: 'x', states: { x: { type: 'atomic' } } },
    });
    expect(status).toBe(400);
    expect(body.error).toContain('URL does not match');
  });

  it('rejects PUT with schema-invalid kernel', async () => {
    const { status, body } = await apiJson('PUT', `/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`, {
      $wosKernel: '1.0',
      url: benefitsUrl,
      // missing version, title, lifecycle — schema-invalid
    });
    expect(status).toBe(400);
    expect(body.error).toBe('Invalid kernel');
    expect(Array.isArray(body.issues)).toBe(true);
    expect(body.issues.length).toBeGreaterThan(0);
  });

  it('accepts PUT with a schema-valid kernel and updates the in-memory registry', async () => {
    // Load the current kernel, edit a harmless field, PUT it back, re-fetch.
    // Requires persistMode: 'memory' so we don't mutate the repo's fixture.
    const originalRes = await apiGet(`/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`);
    expect(originalRes.status).toBe(200);
    const updated = { ...originalRes.body, description: 'integration-test-edit' };

    const putRes = await apiJson('PUT', `/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`, updated);
    expect(putRes.status).toBe(200);
    expect(putRes.body.ok).toBe(true);

    const reRead = await apiGet(`/api/bundles/${encodeURIComponent(benefitsUrl)}/kernel`);
    expect(reRead.status).toBe(200);
    expect(reRead.body.description).toBe('integration-test-edit');
  });
});

describe('real server — kernel validation endpoint', () => {
  it('passes for a valid kernel', async () => {
    const { status, body } = await apiJson('POST', '/api/kernel/validate', {
      $wosKernel: '1.0',
      url: 'test://wf',
      version: '1.0.0',
      title: 'Valid',
      status: 'draft',
      impactLevel: 'operational',
      actors: [{ id: 'a', type: 'human' }],
      lifecycle: {
        initialState: 'start',
        states: {
          start: {
            type: 'atomic',
            transitions: [{ event: { kind: 'message', name: 'go' }, target: 'done' }],
          },
          done: { type: 'final' },
        },
      },
    });
    expect(status).toBe(200);
    expect(body.isValid).toBe(true);
    expect(body.issues).toEqual([]);
  });

  it('reports structural issues for a kernel that violates the schema', async () => {
    // Uses an unknown state 'type' value, which the schema's enum explicitly rejects.
    const { status, body } = await apiJson('POST', '/api/kernel/validate', {
      $wosKernel: '1.0',
      url: 'test://wf',
      version: '1.0.0',
      title: 'Invalid',
      status: 'draft',
      impactLevel: 'operational',
      actors: [{ id: 'a', type: 'human' }],
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'not-a-real-type' as unknown as 'atomic' },
        },
      },
    });
    expect(status).toBe(200);
    expect(body.isValid).toBe(false);
    expect(body.issues.length).toBeGreaterThan(0);
  });

  it('rejects a compound state that omits initialState (Kernel S4.3)', async () => {
    // Exercises the state-type structural invariant added to the kernel schema:
    // a compound state MUST declare initialState and a non-empty states map.
    const { status, body } = await apiJson('POST', '/api/kernel/validate', {
      $wosKernel: '1.0',
      url: 'test://wf',
      version: '1.0.0',
      title: 'Invalid compound',
      status: 'draft',
      impactLevel: 'operational',
      actors: [{ id: 'a', type: 'human' }],
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'compound', states: { inner: { type: 'atomic' } } },
          done: { type: 'final' },
        },
      },
    });
    expect(status).toBe(200);
    expect(body.isValid).toBe(false);
    expect(body.issues.length).toBeGreaterThan(0);
  });

  it('rejects a parallel state that omits regions (Kernel S4.3, S4.4)', async () => {
    const { status, body } = await apiJson('POST', '/api/kernel/validate', {
      $wosKernel: '1.0',
      url: 'test://wf',
      version: '1.0.0',
      title: 'Invalid parallel',
      status: 'draft',
      impactLevel: 'operational',
      actors: [{ id: 'a', type: 'human' }],
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'parallel' },
          done: { type: 'final' },
        },
      },
    });
    expect(status).toBe(200);
    expect(body.isValid).toBe(false);
    expect(body.issues.length).toBeGreaterThan(0);
  });

  it('rejects an atomic state that carries compound/parallel structural fields (Kernel S4.3)', async () => {
    const { status, body } = await apiJson('POST', '/api/kernel/validate', {
      $wosKernel: '1.0',
      url: 'test://wf',
      version: '1.0.0',
      title: 'Invalid atomic',
      status: 'draft',
      impactLevel: 'operational',
      actors: [{ id: 'a', type: 'human' }],
      lifecycle: {
        initialState: 'start',
        states: {
          // atomic MUST NOT carry initialState, states, regions, cancellationPolicy, or historyState.
          start: { type: 'atomic', initialState: 'ghost' } as unknown as { type: 'atomic' },
          done: { type: 'final' },
        },
      },
    });
    expect(status).toBe(200);
    expect(body.isValid).toBe(false);
    expect(body.issues.length).toBeGreaterThan(0);
  });
});

describe('real server — instances and tasks', () => {
  it('lists instances with pagination', async () => {
    const { status, body } = await apiGet('/api/instances?page=1&pageSize=2');
    expect(status).toBe(200);
    expect(body.items).toHaveLength(2);
    expect(body.pageSize).toBe(2);
    expect(body.totalPages).toBeGreaterThanOrEqual(1);
  });

  it('submitEvent records actorId on the response', async () => {
    const instanceId = 'urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8';
    const { status, body } = await apiJson('POST', `/api/instances/${encodeURIComponent(instanceId)}/events`, {
      event: 'applicationComplete',
      actorId: 'caseworkerA',
      data: { verification: { status: 'pending' } },
    });
    expect(status).toBe(200);
    expect(body.actorId).toBe('caseworkerA');
    expect(body.eventsFired).toEqual(['applicationComplete']);
    expect(body.caseStateMutations).toEqual({ verification: { status: 'pending' } });
  });

  it('rejects submitEvent without event', async () => {
    const instanceId = 'urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8';
    const { status, body } = await apiJson('POST', `/api/instances/${encodeURIComponent(instanceId)}/events`, {
      data: { x: 1 },
    });
    expect(status).toBe(400);
    expect(body.error).toContain('event');
  });
});

describe('real server — AI chat hardening', () => {
  it('refuses /api/ai/chat when geminiApiKey is not configured', async () => {
    const { status, body } = await apiJson('POST', '/api/ai/chat', { contents: [{ parts: [{ text: 'hi' }] }] });
    expect(status).toBe(503);
    expect(body.error).toMatch(/AI service not configured/i);
  });

  it('returns 413 when /api/ai/chat body exceeds the 64kb cap', async () => {
    // Build a payload whose JSON encoding is comfortably over 64kb but
    // below the default 1mb global limit.
    const bigText = 'x'.repeat(80 * 1024);
    const res = await fetch(`${baseUrl}/api/ai/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ contents: [{ parts: [{ text: bigText }] }] }),
    });
    expect(res.status).toBe(413);
    const body = await res.json();
    expect(body.error).toMatch(/64kb/i);
  });
});

describe('real server — auth', () => {
  it('no-auth mode allows /api/auth/me', async () => {
    const { status, body } = await apiGet('/api/auth/me');
    expect(status).toBe(200);
    expect(body.id).toBe('user-1');
  });
});
