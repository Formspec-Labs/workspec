import express from 'express';
import { createServer, type Server as HttpServer } from 'http';
import { Server as SocketIOServer } from 'socket.io';
import cors from 'cors';
import { createServer as createViteServer } from 'vite';
import path from 'path';
import { fileURLToPath } from 'url';
import { readFileSync, readdirSync, writeFileSync, existsSync } from 'fs';
import { validateKernelDocument } from './src/services/wos-kernel-validator';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface WosDocumentBundle {
  kernel: any;
  [key: string]: any;
}

interface KernelRegistryEntry {
  bundle: WosDocumentBundle;
  sourcePath: string;
}

export interface StartServerOptions {
  port?: number;
  fixturesDir?: string;
  apiToken?: string;
  corsOrigin?: string;
  geminiApiKey?: string;
  attachVite?: boolean;
  serveStatic?: boolean;
}

export interface StartedServer {
  app: express.Express;
  httpServer: HttpServer;
  io: SocketIOServer;
  close: () => Promise<void>;
  kernelRegistry: Map<string, KernelRegistryEntry>;
  addressPort: number;
}

function tryReadJson(filePath: string): Record<string, unknown> | null {
  try {
    return JSON.parse(readFileSync(filePath, 'utf-8'));
  } catch {
    return null;
  }
}

function makeAuthMiddleware(token: string | undefined) {
  return (req: express.Request, res: express.Response, next: express.NextFunction) => {
    if (!token) return next();
    const supplied = req.headers.authorization?.replace('Bearer ', '');
    if (supplied !== token) {
      return res.status(401).json({ error: 'Unauthorized' });
    }
    next();
  };
}

function requireTokenMiddleware(token: string | undefined, corsOrigin: string) {
  return (req: express.Request, res: express.Response, next: express.NextFunction) => {
    if (!token) {
      if (process.env.NODE_ENV === 'production' || corsOrigin === '*') {
        return res.status(503).json({ error: 'Endpoint requires API_TOKEN to be configured' });
      }
      return next();
    }
    const supplied = req.headers.authorization?.replace('Bearer ', '');
    if (supplied !== token) {
      return res.status(401).json({ error: 'Unauthorized' });
    }
    next();
  };
}

function loadKernelRegistry(fixturesDir: string): Map<string, KernelRegistryEntry> {
  const registry = new Map<string, KernelRegistryEntry>();
  const kernelDir = path.join(fixturesDir, 'kernel');
  for (const file of readdirSync(kernelDir).filter(f => f.endsWith('.json'))) {
    const sourcePath = path.join(kernelDir, file);
    try {
      const kernel = JSON.parse(readFileSync(sourcePath, 'utf-8'));
      if (kernel.url && kernel.$wosKernel) {
        registry.set(kernel.url, { bundle: { kernel }, sourcePath });
      }
    } catch {
      // ignore files that aren't kernel documents
    }
  }
  return registry;
}

function buildFullBundle(fixturesDir: string, kernelUrl: string, registry: Map<string, KernelRegistryEntry>): WosDocumentBundle | null {
  const existing = registry.get(kernelUrl);
  if (!existing) return null;
  const baseName = kernelUrl.split('/').pop();
  const sidecars: [string, string][] = [
    ['governance', path.join(fixturesDir, 'governance', `${baseName}-governance.json`)],
    ['ai', path.join(fixturesDir, 'ai', `${baseName}-ai.json`)],
    ['policyParameters', path.join(fixturesDir, 'governance', `benefits-policy-parameters.json`)],
    ['notificationTemplates', path.join(fixturesDir, 'sidecars', `benefits-notification-templates.json`)],
    ['businessCalendar', path.join(fixturesDir, 'sidecars', `benefits-business-calendar.json`)],
    ['advanced', path.join(fixturesDir, 'advanced', `benefits-advanced-governance.json`)],
    ['equity', path.join(fixturesDir, 'advanced', `benefits-equity-config.json`)],
    ['driftMonitor', path.join(fixturesDir, 'ai', `benefits-drift-monitor.json`)],
    ['verificationReport', path.join(fixturesDir, 'advanced', `verification-report.json`)],
    ['correspondenceMetadata', path.join(fixturesDir, 'kernel', `benefits-correspondence-metadata.json`)],
    ['semanticProfile', path.join(fixturesDir, 'profiles', `semantic-benefits-adjudication.json`)],
    ['integrationProfile', path.join(fixturesDir, 'profiles', `integration-benefits-adjudication.json`)],
    ['lifecycleDetail', path.join(fixturesDir, 'companions', `benefits-lifecycle-detail.json`)],
  ];
  const bundle: WosDocumentBundle = { kernel: existing.bundle.kernel };
  for (const [key, p] of sidecars) {
    if (existsSync(p)) {
      const data = tryReadJson(p);
      if (data) bundle[key] = data;
    }
  }
  const agentPath = path.join(fixturesDir, 'ai', 'document-extractor-config.json');
  if (existsSync(agentPath)) {
    const agentData = tryReadJson(agentPath);
    if (agentData) bundle.agentConfigs = [agentData];
  }
  return bundle;
}

const DEMO_INSTANCES = [
  {
    instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4',
    definitionUrl: 'https://agency.gov/workflows/benefits-adjudication',
    definitionVersion: '1.0.0',
    status: 'active' as const,
    configuration: ['eligibilityReview.reviewerA.pendingReviewA', 'eligibilityReview.reviewerB.pendingReviewB'],
    caseState: {
      application: { isComplete: true, applicantName: 'John Doe', ssn: '***-**-1234' },
      verification: { status: 'verified', income: 34200, householdSize: 3 },
      reviewA: { decision: null, notes: '' },
      reviewB: { decision: null, notes: '' },
    },
    activeTasks: [
      { taskId: 'task-1', taskRef: 'eligibilityDetermination', status: 'claimed', assignedActor: 'caseworkerA', deadline: '2026-04-16T23:59:59Z', impactLevel: 'rights-impacting', createdAt: '2026-04-09T14:30:00Z', updatedAt: '2026-04-09T15:00:00Z' },
      { taskId: 'task-2', taskRef: 'eligibilityDetermination', status: 'assigned', assignedActor: 'caseworkerB', deadline: '2026-04-16T23:59:59Z', impactLevel: 'rights-impacting', createdAt: '2026-04-09T14:30:00Z', updatedAt: '2026-04-09T14:30:00Z' },
    ],
    timers: [{ timerId: 'review-deadline', deadline: '2026-04-16T23:59:59Z', event: '$timeout.task', scopeState: 'eligibilityReview' }],
    governanceState: {
      activeDelegations: [{ delegatorId: 'director-smith', delegateId: 'caseworkerA', scope: 'eligibilityDetermination', authority: 'determination', grantedAt: '2026-01-01T00:00:00Z', expiresAt: '2026-12-31T23:59:59Z' }],
      activeHolds: [],
      reviewState: { protocol: 'dual-blind', reviewerAStatus: 'in-progress', reviewerBStatus: 'pending' },
    },
    impactLevel: 'rights-impacting',
    createdAt: '2026-04-08T09:00:00Z',
    updatedAt: '2026-04-09T15:00:00Z',
  },
  {
    instanceId: 'urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8',
    definitionUrl: 'https://agency.gov/workflows/benefits-adjudication',
    definitionVersion: '1.0.0',
    status: 'active' as const,
    configuration: ['intake'],
    caseState: { application: { isComplete: false, applicantName: 'Jane Smith' } },
    activeTasks: [
      { taskId: 'task-3', taskRef: 'processApplication', status: 'created', assignedActor: 'intakeWorker', impactLevel: 'rights-impacting', createdAt: '2026-04-07T10:00:00Z', updatedAt: '2026-04-07T10:00:00Z' },
    ],
    timers: [],
    governanceState: { activeDelegations: [], activeHolds: [], reviewState: {} },
    impactLevel: 'rights-impacting',
    createdAt: '2026-04-07T10:00:00Z',
    updatedAt: '2026-04-07T10:00:00Z',
  },
  {
    instanceId: 'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2',
    definitionUrl: 'https://agency.gov/workflows/benefits-adjudication',
    definitionVersion: '1.0.0',
    status: 'active' as const,
    configuration: ['adverseNotice'],
    caseState: {
      application: { isComplete: true, applicantName: 'Maria Garcia', ssn: '***-**-5678' },
      verification: { status: 'verified', income: 52000, householdSize: 2 },
      determination: { decision: 'denied', reason: 'Income exceeds threshold' },
    },
    activeTasks: [],
    timers: [{ timerId: 'appealWindow', deadline: '2026-04-19T23:59:59Z', event: 'appealWindowExpired', scopeState: 'adverseNotice' }],
    governanceState: { activeDelegations: [], activeHolds: [], reviewState: {} },
    impactLevel: 'rights-impacting',
    createdAt: '2026-03-20T08:00:00Z',
    updatedAt: '2026-04-05T11:00:00Z',
  },
];

const DEMO_PROVENANCE: Record<string, unknown[]> = {
  'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4': [
    { id: 'prov-1', instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4', timestamp: '2026-04-09T14:30:00Z', tier: 'facts', actor: { id: 'verificationSystem', type: 'system', name: 'Income Verification System' }, event: 'verificationComplete', sourceState: 'incomeVerification', targetState: 'eligibilityReview', facts: { inputs: { income: 34200, householdSize: 3 }, outputs: { status: 'verified' }, metadata: { source: 'IRS Data Bridge', confidence: 0.98 } }, reasoning: { rulesApplied: ['Income Verification Rule v4'], criteriaChecked: [{ label: 'Income < $45,000', passed: true }, { label: 'Household size verified', passed: true }], explanation: 'Income $34,200 verified against IRS records.', sourceAuthority: 'regulation' }, integrity: { hash: 'sha256:7f83b1de...', previousHash: 'sha256:a3b2c1...' } },
    { id: 'prov-2', instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4', timestamp: '2026-04-09T15:00:00Z', tier: 'ai-narrative', actor: { id: 'extractionAgent', type: 'agent', name: 'ExtractionAgent v2.1' }, event: 'ai-extraction', sourceState: 'eligibilityReview', targetState: 'eligibilityReview', facts: { inputs: {}, outputs: {}, metadata: {} }, aiNarrative: { text: 'I analyzed the uploaded tax returns and utility bills. The income was calculated as $34,200 based on the 2025 IRS Form 1040, Line 11.', model: 'ExtractionAgent', version: '2.1.0', confidence: 0.94 }, counterfactual: { positive: ['If household size were 2, benefit would be $1,000/month'], negative: ['Even if residency were different, determination would be Pending for further proof.'] }, integrity: { hash: 'sha256:e5d4f3...', previousHash: 'sha256:7f83b1de...' } },
  ],
  'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2': [
    { id: 'prov-3', instanceId: 'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2', timestamp: '2026-04-05T11:00:00Z', tier: 'reasoning', actor: { id: 'caseworkerA', type: 'human', name: 'Sarah Jenkins' }, event: 'denied', sourceState: 'determination', targetState: 'adverseNotice', facts: { inputs: { income: 52000, householdSize: 2 }, outputs: { determination: 'denied' }, metadata: { policyVersion: 'FY2026-Q2', reviewProtocol: 'dual-blind' } }, reasoning: { rulesApplied: ['Income Eligibility Rule v4', 'Household Size Threshold v2'], criteriaChecked: [{ label: 'Income < $45,000 (household 2)', passed: false }, { label: 'Valid State Residency', passed: true }], explanation: 'Applicant income $52,000 exceeds $45,000 threshold for household of 2.', sourceAuthority: 'statute' }, aiNarrative: { text: 'The AI model suggested denial because income of $52,000 exceeded the $45,000 threshold.', model: 'DecisionSupport', version: '1.0.5', confidence: 0.96 }, counterfactual: { positive: ['If household size were 4, threshold would be $55,000 and applicant would qualify.'], negative: ['Even if income were below threshold, missing residency proof would require further verification.'] }, authorityChain: [{ actor: 'Sarah Jenkins', delegatedBy: 'Director M. Smith', legalInstrument: 'DOA-2025-001', isValid: true }], integrity: { hash: 'sha256:b2c3d4...', previousHash: 'sha256:a1b2c3...' } },
  ],
};

export async function startServer(options: StartServerOptions = {}): Promise<StartedServer> {
  const port = options.port ?? parseInt(process.env.PORT ?? '3000', 10);
  const fixturesDir = options.fixturesDir ?? path.resolve(__dirname, '../fixtures');
  const apiToken = options.apiToken ?? process.env.API_TOKEN;
  const corsOrigin = options.corsOrigin ?? process.env.CORS_ORIGIN ?? '*';
  const geminiApiKey = options.geminiApiKey ?? process.env.GEMINI_API_KEY;
  const attachVite = options.attachVite ?? (process.env.NODE_ENV !== 'production');
  const serveStatic = options.serveStatic ?? (process.env.NODE_ENV === 'production');

  const app = express();
  const httpServer = createServer(app);
  const io = new SocketIOServer(httpServer, {
    cors: { origin: corsOrigin, methods: ['GET', 'POST'] },
  });

  const kernelRegistry = loadKernelRegistry(fixturesDir);
  if (kernelRegistry.size === 0) {
    throw new Error(`No fixture kernels found in ${fixturesDir}`);
  }

  const instances = DEMO_INSTANCES.map(i => ({ ...i, caseState: { ...i.caseState } }));
  const provenance = { ...DEMO_PROVENANCE };

  app.use(cors({ origin: corsOrigin, methods: ['GET', 'POST', 'PUT', 'DELETE'] }));
  app.use(express.json({ limit: '1mb' }));
  app.use('/api/', makeAuthMiddleware(apiToken));

  // ---------- Kernel bundle endpoints ----------

  app.get('/api/bundles', (_req, res) => {
    const summaries = Array.from(kernelRegistry.entries()).map(([url, entry]) => ({
      url,
      title: entry.bundle.kernel?.title ?? 'Untitled',
      version: entry.bundle.kernel?.version ?? '0.0.0',
      status: entry.bundle.kernel?.status ?? 'draft',
      impactLevel: entry.bundle.kernel?.impactLevel ?? 'operational',
    }));
    res.json(summaries);
  });

  app.get('/api/bundles/:url', (req, res) => {
    const url = decodeURIComponent(req.params.url);
    const bundle = buildFullBundle(fixturesDir, url, kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json(bundle);
  });

  app.get('/api/bundles/:url/kernel', (req, res) => {
    const url = decodeURIComponent(req.params.url);
    const entry = kernelRegistry.get(url);
    if (!entry) return res.status(404).json({ error: 'Kernel not found' });
    res.json(entry.bundle.kernel);
  });

  app.put('/api/bundles/:url/kernel', (req, res) => {
    const url = decodeURIComponent(req.params.url);
    const entry = kernelRegistry.get(url);
    if (!entry) return res.status(404).json({ error: 'Kernel not found' });

    const body = req.body;
    if (body?.url && body.url !== url) {
      return res.status(400).json({ error: 'Kernel body URL does not match route' });
    }

    const validation = validateKernelDocument(body);
    if (!validation.isValid) {
      return res.status(400).json({ error: 'Invalid kernel', issues: validation.issues });
    }

    entry.bundle.kernel = body;
    try {
      writeFileSync(entry.sourcePath, JSON.stringify(body, null, 2));
    } catch (err) {
      console.error('Failed to persist kernel:', err);
      return res.status(500).json({ error: 'Failed to persist kernel' });
    }
    io.emit('kernel:changed', { url, kernel: body });
    res.json({ ok: true });
  });

  app.post('/api/kernel/validate', (req, res) => {
    const result = validateKernelDocument(req.body);
    res.json(result);
  });

  // Back-compat single-kernel endpoints (resolve to the first registered kernel).
  const primaryEntry = kernelRegistry.values().next().value as KernelRegistryEntry;
  app.get('/api/kernel', (_req, res) => {
    res.json(primaryEntry.bundle.kernel);
  });

  // ---------- Instances / tasks ----------

  app.get('/api/instances', (req, res) => {
    let items = [...instances];
    const status = req.query.status as string | undefined;
    const impactLevel = req.query.impactLevel as string | undefined;
    const definitionUrl = req.query.definitionUrl as string | undefined;
    if (status) items = items.filter(i => status.split(',').includes(i.status));
    if (impactLevel) items = items.filter(i => impactLevel.split(',').includes(i.impactLevel));
    if (definitionUrl) items = items.filter(i => i.definitionUrl === definitionUrl);

    const page = Math.max(1, Number(req.query.page) || 1);
    const pageSize = Math.min(100, Math.max(1, Number(req.query.pageSize) || 50));
    const total = items.length;
    const totalPages = Math.max(1, Math.ceil(total / pageSize));
    const start = (page - 1) * pageSize;
    items = items.slice(start, start + pageSize);

    res.json({ items, total, page, pageSize, totalPages });
  });

  app.get('/api/instances/:id', (req, res) => {
    const inst = instances.find(i => i.instanceId === decodeURIComponent(req.params.id));
    if (!inst) return res.status(404).json({ error: 'Instance not found' });
    res.json(inst);
  });

  app.get('/api/instances/:id/provenance', (req, res) => {
    const records = provenance[decodeURIComponent(req.params.id)] ?? [];
    res.json(records);
  });

  app.get('/api/instances/:id/transitions', (req, res) => {
    const inst = instances.find(i => i.instanceId === decodeURIComponent(req.params.id));
    if (!inst) return res.json([]);
    const appData = inst.caseState.application as { isComplete?: boolean } | undefined;
    res.json([
      { event: 'applicationComplete', target: 'incomeVerification', guard: 'caseFile.application.isComplete = true', guardSatisfied: Boolean(appData?.isComplete), tags: ['intake'], description: 'Complete application proceeds to income verification' },
      { event: 'applicationIncomplete', target: 'returnedToApplicant', guard: 'caseFile.application.isComplete = false', guardSatisfied: !appData?.isComplete, tags: ['intake'], description: 'Incomplete application returned to applicant' },
    ]);
  });

  app.post('/api/instances/:id/events', (req, res) => {
    const inst = instances.find(i => i.instanceId === decodeURIComponent(req.params.id));
    if (!inst) return res.status(404).json({ error: 'Instance not found' });
    const { event, actorId, data } = req.body ?? {};
    if (!event || typeof event !== 'string') {
      return res.status(400).json({ error: 'Missing or invalid event' });
    }
    const prevConfig = [...inst.configuration];
    inst.configuration = [event];
    if (data && typeof data === 'object') Object.assign(inst.caseState, data);
    inst.updatedAt = new Date().toISOString();
    res.json({
      previousConfiguration: prevConfig,
      newConfiguration: inst.configuration,
      eventsFired: [event],
      actorId: typeof actorId === 'string' ? actorId : null,
      caseStateMutations: data && typeof data === 'object' ? data : {},
    });
  });

  app.get('/api/tasks', (req, res) => {
    let filtered = [...instances];
    const status = req.query.status as string | undefined;
    const impactLevel = req.query.impactLevel as string | undefined;
    const definitionUrl = req.query.definitionUrl as string | undefined;
    if (status) filtered = filtered.filter(i => status.split(',').includes(i.status));
    if (impactLevel) filtered = filtered.filter(i => impactLevel.split(',').includes(i.impactLevel));
    if (definitionUrl) filtered = filtered.filter(i => i.definitionUrl === definitionUrl);

    const items = filtered.flatMap(inst =>
      inst.activeTasks.map(task => ({
        taskId: task.taskId,
        instanceId: inst.instanceId,
        taskRef: task.taskRef,
        status: task.status,
        assignedActor: task.assignedActor,
        deadline: task.deadline,
        impactLevel: task.impactLevel ?? inst.impactLevel,
        configuration: inst.configuration,
        caseState: inst.caseState,
        definitionTitle: inst.definitionUrl.split('/').pop() ?? inst.definitionUrl,
        definitionUrl: inst.definitionUrl,
        createdAt: task.createdAt,
      })),
    );

    const page = Math.max(1, Number(req.query.page) || 1);
    const pageSize = Math.min(100, Math.max(1, Number(req.query.pageSize) || 50));
    const total = items.length;
    const totalPages = Math.max(1, Math.ceil(total / pageSize));
    const start = (page - 1) * pageSize;
    const paged = items.slice(start, start + pageSize);

    res.json({ items: paged, total, page, pageSize, totalPages });
  });

  app.get('/api/tasks/:taskId', (req, res) => {
    const taskId = decodeURIComponent(req.params.taskId);
    for (const inst of instances) {
      const task = inst.activeTasks.find(t => t.taskId === taskId);
      if (task) {
        return res.json({
          taskId: task.taskId,
          instanceId: inst.instanceId,
          taskRef: task.taskRef,
          status: task.status,
          assignedActor: task.assignedActor,
          deadline: task.deadline,
          impactLevel: task.impactLevel ?? inst.impactLevel,
          configuration: inst.configuration,
          caseState: inst.caseState,
          definitionTitle: inst.definitionUrl.split('/').pop() ?? '',
          definitionUrl: inst.definitionUrl,
          createdAt: task.createdAt,
        });
      }
    }
    res.status(404).json({ error: 'Task not found' });
  });

  // ---------- Governance & dashboard ----------

  app.get('/api/governance/:url/agents', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    const ai = bundle.ai as { agents?: { id: string; type?: string; capabilities?: { name: string; autonomy?: string }[] }[] } | undefined;
    const agents = (ai?.agents ?? []).map(a => ({
      id: a.id, name: a.id, type: a.type ?? 'llm', version: '1.0', status: 'active',
      capabilities: (a.capabilities ?? []).map(c => ({ name: c.name, autonomy: c.autonomy ?? 'assistive' })),
    }));
    res.json(agents);
  });

  app.get('/api/governance/:url/deontic-constraints', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json([]);
  });

  app.get('/api/governance/:url/quality-controls', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    const govs = bundle.governance as { qualityControls?: Record<string, unknown> } | undefined;
    res.json(govs?.qualityControls ?? null);
  });

  app.get('/api/governance/:url/pipelines', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json([]);
  });

  app.get('/api/governance/:url/verification-report', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json(bundle.verificationReport ?? null);
  });

  app.get('/api/governance/:url/equity-config', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json(bundle.equity ?? null);
  });

  app.get('/api/governance/:url/delegations', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    res.json([{ id: 'del-1', delegator: 'Director M. Smith', delegate: 'Sarah Jenkins', scope: 'Eligibility Determination', authority: 'determination', legalInstrument: 'DOA-2025-001', startDate: '2026-01-01', endDate: '2026-12-31', status: 'active' }]);
  });

  app.delete('/api/governance/:url/delegations/:delegationId', (_req, res) => {
    res.json({ ok: true });
  });

  app.get('/api/governance/:url/policy-versions', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    const pp = bundle.policyParameters as { title?: string; parameters?: Record<string, unknown> } | undefined;
    res.json([{ id: 'v1', label: pp?.title ?? 'Current', effectiveDate: '2026-04-01', parameterCount: Object.keys(pp?.parameters ?? {}).length, status: 'active' }]);
  });

  app.get('/api/governance/:url/calendar-events', (req, res) => {
    const bundle = buildFullBundle(fixturesDir, decodeURIComponent(req.params.url), kernelRegistry);
    if (!bundle) return res.status(404).json({ error: 'Bundle not found' });
    const cal = bundle.businessCalendar as { holidays?: { name: string; date: string }[] } | undefined;
    res.json((cal?.holidays ?? []).map((h, i) => ({ id: `hol-${i}`, name: h.name, date: h.date, type: 'federal', impactsDeadlines: true })));
  });

  app.get('/api/health', (_req, res) => {
    res.json([
      { id: 'irs', name: 'IRS Data Bridge', status: 'healthy', latency: '120ms', errorRate: '0.1%', lastCheck: new Date().toISOString() },
      { id: 'notif', name: 'Notification Service', status: 'healthy', latency: '45ms', errorRate: '0%', lastCheck: new Date().toISOString() },
      { id: 'ai', name: 'AI Extraction Service', status: 'degraded', latency: '890ms', errorRate: '2.3%', lastCheck: new Date().toISOString() },
    ]);
  });

  app.get('/api/dashboard/metrics', (_req, res) => {
    res.json({
      activeInstances: instances.length, completed7d: 12, slaCompliance: 94, avgProcessingTimeDays: 4.2, aiAcceptanceRate: 87,
      activeInstancesTrend: 5, completed7dTrend: -2, slaComplianceTrend: 1, avgProcessingTimeTrend: -0.3, aiAcceptanceRateTrend: 3,
    });
  });

  app.get('/api/dashboard/stage-metrics', (_req, res) => {
    const primaryKernel = primaryEntry.bundle.kernel as { lifecycle?: { states?: Record<string, unknown> } };
    const states = (primaryKernel.lifecycle?.states ?? {}) as Record<string, unknown>;
    res.json(Object.keys(states).slice(0, 6).map((name, i) => ({ name, count: (i + 1) * 2, avgWait: `${(i % 3) + 1}d`, status: 'normal' })));
  });

  app.get('/api/dashboard/alerts', (_req, res) => {
    res.json([
      { id: 'a1', type: 'drift', title: 'Decision drift detected', description: 'Override rate for caseworkerA increased 15% this week', timeAgo: '2h ago', severity: 'warning' },
      { id: 'a2', type: 'sla', title: 'SLA breach risk', description: '3 cases approaching 30-day determination deadline', timeAgo: '30m ago', severity: 'critical' },
    ]);
  });

  app.get('/api/dashboard/drift-data', (_req, res) => {
    res.json(['Week 1', 'Week 2', 'Week 3', 'Week 4', 'Week 5', 'Week 6'].map((week, i) => ({ week, overrideRate: 5 + i * 2, timeOnTask: 20 + i * 2 })));
  });

  app.get('/api/dashboard/pipeline-data', (_req, res) => {
    res.json([
      { name: 'Intake', volume: 45, capacity: 50 },
      { name: 'Review', volume: 32, capacity: 40 },
      { name: 'Determination', volume: 18, capacity: 20 },
    ]);
  });

  // ---------- Applicant / auth ----------

  app.get('/api/applicant/:instanceId/determination', (req, res) => {
    const inst = instances.find(i => i.instanceId === decodeURIComponent(req.params.instanceId));
    if (!inst) return res.status(404).json({ error: 'Instance not found' });
    const cs = inst.caseState as Record<string, unknown>;
    const determination = cs.determination as { decision?: string; reason?: string } | undefined;
    const timers = inst.timers as { event: string; deadline: string }[];
    const configuration = inst.configuration as string[];
    res.json({
      instanceId: inst.instanceId,
      programName: 'Housing Benefits',
      decision: determination?.decision ?? 'pending',
      dateIssued: inst.updatedAt,
      deadlineDate: timers.find(t => t.event === 'appealWindowExpired')?.deadline ?? '',
      benefitsContinue: false,
      summary: determination?.reason ?? 'Under review',
      evidenceConsidered: ['Tax Return 2025', 'Utility Bill March 2026', 'ID Verification'],
      rulesApplied: ['Income Eligibility Rule v4'],
      aiDisclosure: { wasUsed: true, description: 'AI assisted in document extraction and income verification.' },
      counterfactuals: { positive: [], negative: [] },
      appealStatus: 'not-filed',
      milestones: configuration.map((s, i) => ({ id: `m-${i}`, label: s, status: i < configuration.length - 1 ? 'completed' : 'current', description: `State: ${s}` })),
    });
  });

  app.post('/api/applicant/:instanceId/appeal', (_req, res) => {
    res.json({ ok: true });
  });

  app.get('/api/auth/me', (_req, res) => {
    res.json({ id: 'user-1', name: 'Jane Doe', email: 'jane.doe@agency.gov', role: 'Supervisor' });
  });

  app.post('/api/auth/login', (_req, res) => {
    res.json({ id: 'user-1', name: 'Jane Doe', email: 'jane.doe@agency.gov', role: 'Supervisor' });
  });

  app.post('/api/auth/logout', (_req, res) => {
    res.json({ ok: true });
  });

  app.get('/api/auth/has-role/:role', (req, res) => {
    res.json({ hasRole: req.params.role === 'Supervisor' });
  });

  // /api/ai/chat is hardened: always requires API_TOKEN; refuses wildcard CORS in prod;
  // validates payload shape; caps size at 64kb.
  app.post(
    '/api/ai/chat',
    express.json({ limit: '64kb' }),
    requireTokenMiddleware(apiToken, corsOrigin),
    async (req, res) => {
      if (!geminiApiKey) {
        return res.status(503).json({ error: 'AI service not configured' });
      }
      const body = req.body;
      if (!body || typeof body !== 'object' || !Array.isArray((body as { contents?: unknown }).contents)) {
        return res.status(400).json({ error: 'Invalid payload: expected { contents: [...] }' });
      }
      try {
        const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=${geminiApiKey}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        const data = await response.json();
        res.status(response.status).json(data);
      } catch {
        res.status(500).json({ error: 'AI service error' });
      }
    },
  );

  // ---------- Socket.IO ----------

  const activeUsers = new Map<string, { id: string; name?: string; cursor: { x: number; y: number } }>();

  io.on('connection', (socket) => {
    socket.emit('kernel:init', primaryEntry.bundle.kernel);

    socket.on('user:join', (userData: Record<string, unknown>) => {
      activeUsers.set(socket.id, { ...userData, id: socket.id, cursor: { x: 0, y: 0 } });
      io.emit('users:update', Array.from(activeUsers.values()));
    });

    socket.on('cursor:move', (pos: { x: number; y: number }) => {
      const user = activeUsers.get(socket.id);
      if (user) {
        user.cursor = pos;
        socket.broadcast.emit('cursor:update', { userId: socket.id, cursor: pos });
      }
    });

    socket.on('kernel:update', (message: unknown) => {
      if (!message || typeof message !== 'object') return;
      const { url, kernel } = message as { url?: unknown; kernel?: unknown };
      if (typeof url !== 'string' || !kernel || typeof kernel !== 'object') return;
      const entry = kernelRegistry.get(url);
      if (!entry) return;
      const jsonSize = JSON.stringify(kernel).length;
      if (jsonSize > 1_000_000) return;
      const validation = validateKernelDocument(kernel);
      if (!validation.isValid) return;
      entry.bundle.kernel = kernel;
      try {
        writeFileSync(entry.sourcePath, JSON.stringify(kernel, null, 2));
      } catch (err) {
        console.error('Failed to persist kernel via socket:', err);
      }
      socket.broadcast.emit('kernel:changed', { url, kernel });
    });

    socket.on('disconnect', () => {
      activeUsers.delete(socket.id);
      io.emit('users:update', Array.from(activeUsers.values()));
    });
  });

  // ---------- Static / dev middleware ----------

  if (attachVite) {
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: 'spa',
    });
    app.use(vite.middlewares);
  } else if (serveStatic) {
    const distPath = path.join(process.cwd(), 'dist');
    app.use(express.static(distPath));
    app.get('*', (_req, res) => {
      res.sendFile(path.join(distPath, 'index.html'));
    });
  }

  const bound = httpServer.listen(port, '0.0.0.0');
  await new Promise<void>((resolve, reject) => {
    bound.once('listening', () => resolve());
    bound.once('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'EADDRINUSE') {
        reject(new Error(`Port ${port} is already in use. Set PORT environment variable to use a different port.`));
      } else {
        reject(err);
      }
    });
  });

  const address = httpServer.address();
  const addressPort = typeof address === 'object' && address ? address.port : port;

  const close = () => new Promise<void>((resolve) => {
    io.close();
    httpServer.close(() => resolve());
  });

  return { app, httpServer, io, close, kernelRegistry, addressPort };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  startServer().then(({ addressPort, kernelRegistry }) => {
    console.log(`WOS Studio server running on http://localhost:${addressPort}`);
    console.log(`Loaded ${kernelRegistry.size} kernel fixture(s): ${Array.from(kernelRegistry.keys()).join(', ')}`);

    const shutdown = () => {
      console.log('Shutting down...');
      process.exit(0);
    };
    process.on('SIGTERM', shutdown);
    process.on('SIGINT', shutdown);
  }).catch((err) => {
    console.error(err.message ?? err);
    process.exit(1);
  });
}
