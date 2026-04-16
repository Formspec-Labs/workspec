import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import http from 'http';
import type { Server, IncomingMessage, ServerResponse } from 'http';

type Handler = (req: IncomingMessage, res: ServerResponse) => void;

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => { body += chunk; });
    req.on('end', () => resolve(body));
    req.on('error', reject);
  });
}

function createApiRouter(kernel: any) {
  const handlers: Record<string, Record<string, Handler>> = {
    GET: {
      '/api/kernel': (_req, res) => {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(kernel));
      },
    },
    PUT: {
      '/api/kernel': async (req, res) => {
        const raw = await readBody(req);
        let parsed: any;
        try { parsed = JSON.parse(raw); } catch { res.writeHead(400); res.end(JSON.stringify({ error: 'Invalid JSON' })); return; }
        if (!parsed?.$wosKernel || !parsed?.lifecycle?.states) {
          res.writeHead(400);
          res.end(JSON.stringify({ error: 'Invalid kernel: missing $wosKernel or lifecycle.states' }));
          return;
        }
        Object.assign(kernel, parsed);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true }));
      },
    },
    POST: {
      '/api/kernel/validate': async (req, res) => {
        const raw = await readBody(req);
        let parsed: any;
        try { parsed = JSON.parse(raw); } catch { res.writeHead(400); res.end(JSON.stringify({ error: 'Invalid JSON' })); return; }
        const issues: { severity: string; category: string; message: string }[] = [];
        if (!parsed?.lifecycle?.initialState) {
          issues.push({ severity: 'error', category: 'structure', message: 'Missing lifecycle.initialState' });
        }
        if (!parsed?.lifecycle?.states || Object.keys(parsed.lifecycle.states).length === 0) {
          issues.push({ severity: 'error', category: 'structure', message: 'Missing or empty lifecycle.states' });
        }
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ isValid: issues.length === 0, issues }));
      },
    },
  };

  return (req: IncomingMessage, res: ServerResponse) => {
    const method = req.method ?? 'GET';
    const routeHandlers = handlers[method];
    if (!routeHandlers) {
      res.writeHead(405);
      res.end(JSON.stringify({ error: 'Method not allowed' }));
      return;
    }
    const handler = routeHandlers[req.url ?? '/'];
    if (!handler) {
      res.writeHead(404);
      res.end(JSON.stringify({ error: 'Not found' }));
      return;
    }
    handler(req, res);
  };
}

function request(server: Server, method: string, path: string, body?: any): Promise<{ status: number; body: any }> {
  const addr = server.address() as { port: number };
  return new Promise((resolve, reject) => {
    const opts = {
      hostname: '127.0.0.1',
      port: addr.port,
      path,
      method,
      headers: { 'Content-Type': 'application/json' },
    };
    const req = http.request(opts, (res: IncomingMessage) => {
      let data = '';
      res.on('data', (chunk: string) => { data += chunk; });
      res.on('end', () => {
        resolve({ status: res.statusCode ?? 0, body: JSON.parse(data || '{}') });
      });
    });
    req.on('error', reject);
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

const SAMPLE_KERNEL = {
  $wosKernel: '1.0',
  url: 'https://agency.gov/workflows/benefits-adjudication',
  version: '1.0.0',
  title: 'Benefits Adjudication',
  status: 'active',
  lifecycle: {
    initialState: 'intake',
    states: {
      intake: { type: 'atomic', transitions: [{ event: 'submit', target: 'review' }] },
      review: { type: 'atomic', transitions: [{ event: 'approve', target: 'approved' }] },
      approved: { type: 'final' },
    },
  },
};

describe('API integration', () => {
  let server: Server;
  let kernel: any;

  beforeAll(async () => {
    kernel = { ...SAMPLE_KERNEL, lifecycle: { ...SAMPLE_KERNEL.lifecycle, states: { ...SAMPLE_KERNEL.lifecycle.states } } };
    const handler = createApiRouter(kernel);
    server = http.createServer(handler);
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', () => resolve()));
  });

  afterAll(() => {
    server.close();
  });

  it('GET /api/kernel returns kernel JSON', async () => {
    const res = await request(server, 'GET', '/api/kernel');
    expect(res.status).toBe(200);
    expect(res.body.$wosKernel).toBe('1.0');
    expect(res.body.title).toBe('Benefits Adjudication');
    expect(res.body.lifecycle.states).toBeDefined();
  });

  it('PUT /api/kernel with valid body updates kernel', async () => {
    const updated = {
      ...SAMPLE_KERNEL,
      title: 'Updated Benefits',
      lifecycle: {
        initialState: 'start',
        states: { start: { type: 'atomic', transitions: [{ event: 'go', target: 'done' }] }, done: { type: 'final' } },
      },
    };
    const res = await request(server, 'PUT', '/api/kernel', updated);
    expect(res.status).toBe(200);
    expect(res.body.ok).toBe(true);

    const getRes = await request(server, 'GET', '/api/kernel');
    expect(getRes.body.title).toBe('Updated Benefits');
  });

  it('PUT /api/kernel with invalid body returns 400', async () => {
    const res = await request(server, 'PUT', '/api/kernel', { bad: 'data' });
    expect(res.status).toBe(400);
    expect(res.body.error).toContain('Invalid kernel');
  });

  it('POST /api/kernel/validate validates kernel structure', async () => {
    const res = await request(server, 'POST', '/api/kernel/validate', SAMPLE_KERNEL);
    expect(res.status).toBe(200);
    expect(res.body.isValid).toBe(true);
    expect(res.body.issues).toEqual([]);
  });

  it('POST /api/kernel/validate rejects kernel without initialState', async () => {
    const invalid = { ...SAMPLE_KERNEL, lifecycle: { states: { start: { type: 'atomic' } } } };
    const res = await request(server, 'POST', '/api/kernel/validate', invalid);
    expect(res.status).toBe(200);
    expect(res.body.isValid).toBe(false);
    expect(res.body.issues.length).toBeGreaterThan(0);
  });

  it('GET /api/unknown returns 404', async () => {
    const res = await request(server, 'GET', '/api/unknown');
    expect(res.status).toBe(404);
  });
});
