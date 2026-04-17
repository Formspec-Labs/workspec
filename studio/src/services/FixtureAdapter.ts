import type {
  IWosBackend, WosDocumentBundle, KernelSummary, CaseInstanceView, ActiveTaskView,
  ProvenanceRecord, EvaluationResult, AvailableTransition, InstanceFilter, PaginatedResult,
} from './WosBackend';
import type {
  IInboxPort, TaskListItem, ICaseViewerPort, IWorkflowDesignPort, WosValidationResult,
  IGovernancePort, IGovernanceReader, IGovernanceWriter, AgentView, DelegationEntry, DeonticConstraintView, QualityControlsView, PipelineView, PipelineStageView,
  VerificationReportView, VerificationResultView, EquityConfigView,
  PolicyVersionView, CalendarEventView, ServiceHealthView,
  IDashboardPort, DashboardMetrics, StageMetricView, AlertView, DriftDataPoint, PipelineDataPoint,
  IApplicantPort, ApplicantDeterminationView, IRealtimePort, Unsubscribe,
  IAuthPort, AuthUser,
} from './WosPorts';
import type { WOSKernelDocument } from '../types/wos/kernel';

import {
  loadBenefitsAdjudicationBundle, loadPurchaseOrderBundle,
} from '../data/fixtures';

export class FixtureBackend implements IWosBackend {
  private bundles = new Map<string, WosDocumentBundle>();
  private instances: CaseInstanceView[];

  constructor() {
    const benefits = loadBenefitsAdjudicationBundle();
    this.bundles.set('https://agency.gov/workflows/benefits-adjudication', benefits);
    const po = loadPurchaseOrderBundle();
    this.bundles.set('https://procurement.example.gov/workflows/purchase-order-approval', po);

    this.instances = [
      {
        instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4',
        definitionUrl: 'https://agency.gov/workflows/benefits-adjudication',
        definitionVersion: '1.0.0',
        status: 'active',
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
        status: 'active',
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
        status: 'active',
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
  }

  async loadBundle(url: string): Promise<WosDocumentBundle> {
    const b = this.bundles.get(url);
    if (!b) throw new Error(`Bundle not found: ${url}`);
    return b;
  }

  async listBundles(): Promise<KernelSummary[]> {
    return Array.from(this.bundles.values()).map(b => ({
      url: b.kernel.url ?? '',
      title: b.kernel.title ?? 'Untitled',
      version: b.kernel.version ?? '0.0.0',
      status: b.kernel.status ?? 'draft',
      impactLevel: b.kernel.impactLevel ?? 'operational',
    }));
  }

  async getInstance(id: string): Promise<CaseInstanceView | null> {
    return this.instances.find(i => i.instanceId === id) ?? null;
  }

  async listInstances(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<CaseInstanceView>> {
    let items = [...this.instances];
    if (filter?.status) items = items.filter(i => filter.status!.includes(i.status));
    if (filter?.impactLevel) items = items.filter(i => filter.impactLevel!.includes(i.impactLevel));
    if (filter?.definitionUrl) items = items.filter(i => i.definitionUrl === filter.definitionUrl);
    const resolvedPage = Math.max(1, page ?? 1);
    const resolvedPageSize = Math.max(1, pageSize ?? 50);
    const total = items.length;
    const totalPages = Math.max(1, Math.ceil(total / resolvedPageSize));
    const start = (resolvedPage - 1) * resolvedPageSize;
    const paged = items.slice(start, start + resolvedPageSize);
    return { items: paged, total, page: resolvedPage, pageSize: resolvedPageSize, totalPages };
  }

  private provenanceData: Record<string, ProvenanceRecord[]> = {
    'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4': [
      {
        id: 'prov-1', instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4', timestamp: '2026-04-09T14:30:00Z', tier: 'facts',
        actor: { id: 'verificationSystem', type: 'system', name: 'Income Verification System' },
        event: 'verificationComplete', sourceState: 'incomeVerification', targetState: 'eligibilityReview',
        facts: { inputs: { income: 34200, householdSize: 3 }, outputs: { status: 'verified' }, metadata: { source: 'IRS Data Bridge', confidence: 0.98 } },
        reasoning: { rulesApplied: ['Income Verification Rule v4'], criteriaChecked: [{ label: 'Income < $45,000', passed: true }, { label: 'Household size verified', passed: true }], explanation: 'Income $34,200 verified against IRS records.', sourceAuthority: 'regulation' },
        integrity: { hash: 'sha256:7f83b1de...', previousHash: 'sha256:a3b2c1...' },
      },
      {
        id: 'prov-2', instanceId: 'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4', timestamp: '2026-04-09T15:00:00Z', tier: 'ai-narrative',
        actor: { id: 'extractionAgent', type: 'agent', name: 'ExtractionAgent v2.1' },
        event: 'ai-extraction', sourceState: 'eligibilityReview', targetState: 'eligibilityReview',
        facts: { inputs: {}, outputs: {}, metadata: {} },
        aiNarrative: { text: 'I analyzed the uploaded tax returns and utility bills. The income was calculated as $34,200 based on the 2025 IRS Form 1040, Line 11.', model: 'ExtractionAgent', version: '2.1.0', confidence: 0.94 },
        counterfactual: { positive: ['If household size were 2, benefit would be $1,000/month'], negative: ['Even if residency were different, determination would be Pending for further proof.'] },
        integrity: { hash: 'sha256:e5d4f3...', previousHash: 'sha256:7f83b1de...' },
      },
    ],
    'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2': [
      {
        id: 'prov-3', instanceId: 'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2', timestamp: '2026-04-05T11:00:00Z', tier: 'reasoning',
        actor: { id: 'caseworkerA', type: 'human', name: 'Sarah Jenkins' },
        event: 'denied', sourceState: 'determination', targetState: 'adverseNotice',
        facts: { inputs: { income: 52000, householdSize: 2 }, outputs: { determination: 'denied' }, metadata: { policyVersion: 'FY2026-Q2', reviewProtocol: 'dual-blind' } },
        reasoning: { rulesApplied: ['Income Eligibility Rule v4', 'Household Size Threshold v2'], criteriaChecked: [{ label: 'Income < $45,000 (household 2)', passed: false }, { label: 'Valid State Residency', passed: true }], explanation: 'Applicant income $52,000 exceeds $45,000 threshold for household of 2.', sourceAuthority: 'statute' },
        aiNarrative: { text: 'The AI model suggested denial because income of $52,000 exceeded the $45,000 threshold.', model: 'DecisionSupport', version: '1.0.5', confidence: 0.96 },
        counterfactual: { positive: ['If household size were 4, threshold would be $55,000 and applicant would qualify.'], negative: ['Even if income were below threshold, missing residency proof would require further verification.'] },
        authorityChain: [{ actor: 'Sarah Jenkins', delegatedBy: 'Director M. Smith', legalInstrument: 'DOA-2025-001', isValid: true }],
        integrity: { hash: 'sha256:b2c3d4...', previousHash: 'sha256:a1b2c3...' },
      },
    ],
  };

  async getProvenance(instanceId: string): Promise<ProvenanceRecord[]> {
    return this.provenanceData[instanceId] ?? [];
  }

  async submitEvent(instanceId: string, event: string, actorId: string, data?: Record<string, unknown>): Promise<EvaluationResult> {
    const inst = this.instances.find(i => i.instanceId === instanceId);
    if (!inst) throw new Error(`Instance not found: ${instanceId}`);
    const prevConfig = [...inst.configuration];
    inst.configuration = [event];
    if (data) {
      Object.assign(inst.caseState, data);
    }
    inst.updatedAt = new Date().toISOString();
    return {
      previousConfiguration: prevConfig,
      newConfiguration: inst.configuration,
      eventsFired: [event],
      caseStateMutations: data ?? {},
    };
  }

  async getAvailableTransitions(instanceId: string): Promise<AvailableTransition[]> {
    const inst = this.instances.find(i => i.instanceId === instanceId);
    if (!inst) return [];
    const appData = inst.caseState.application as { isComplete?: boolean } | undefined;
    return [
      { event: 'applicationComplete', target: 'incomeVerification', guard: 'caseFile.application.isComplete = true', guardSatisfied: Boolean(appData?.isComplete), tags: ['intake'], description: 'Complete application proceeds to income verification' },
      { event: 'applicationIncomplete', target: 'returnedToApplicant', guard: 'caseFile.application.isComplete = false', guardSatisfied: !appData?.isComplete, tags: ['intake'], description: 'Incomplete application returned to applicant' },
    ];
  }
}

export class FixtureInboxPort implements IInboxPort {
  constructor(private backend: IWosBackend) {}
  async listTasks(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<TaskListItem>> {
    const res = await this.backend.listInstances(filter);
    const items: TaskListItem[] = res.items.flatMap(inst =>
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
    const resolvedPage = Math.max(1, page ?? 1);
    const resolvedPageSize = Math.max(1, pageSize ?? 50);
    const total = items.length;
    const totalPages = Math.max(1, Math.ceil(total / resolvedPageSize));
    const start = (resolvedPage - 1) * resolvedPageSize;
    const paged = items.slice(start, start + resolvedPageSize);
    return { items: paged, total, page: resolvedPage, pageSize: resolvedPageSize, totalPages };
  }
  async getTask(taskId: string): Promise<TaskListItem | null> {
    const res = await this.backend.listInstances();
    for (const inst of res.items) {
      const task = inst.activeTasks.find(t => t.taskId === taskId);
      if (task) {
        return {
          taskId: task.taskId, instanceId: inst.instanceId, taskRef: task.taskRef,
          status: task.status, assignedActor: task.assignedActor, deadline: task.deadline,
          impactLevel: task.impactLevel ?? inst.impactLevel, configuration: inst.configuration,
          caseState: inst.caseState, definitionTitle: inst.definitionUrl.split('/').pop() ?? '',
          definitionUrl: inst.definitionUrl, createdAt: task.createdAt,
        };
      }
    }
    return null;
  }
}

export class FixtureCaseViewerPort implements ICaseViewerPort {
  constructor(private backend: IWosBackend) {}
  getInstance(id: string) { return this.backend.getInstance(id); }
  getProvenance(id: string) { return this.backend.getProvenance(id); }
  getTimeline(id: string) { return this.backend.getProvenance(id); }
}

export class FixtureWorkflowDesignPort implements IWorkflowDesignPort {
  constructor(private backend: IWosBackend) {}
  async listWorkflows() { return this.backend.listBundles(); }
  async loadKernel(url: string) {
    try { const b = await this.backend.loadBundle(url); return b.kernel; } catch { return null; }
  }
  async saveKernel(_kernel: WOSKernelDocument) {}
  async validateKernel(kernel: WOSKernelDocument): Promise<WosValidationResult> {
    const issues: import('./WosPorts').WosValidationIssue[] = [];
    if (!kernel.lifecycle?.initialState) issues.push({ severity: 'error', category: 'structure', message: 'Missing initialState' });
    if (!kernel.lifecycle?.states || Object.keys(kernel.lifecycle.states).length === 0) issues.push({ severity: 'error', category: 'structure', message: 'No states defined' });
    return { isValid: issues.filter(i => i.severity === 'error').length === 0, issues };
  }
}

export class FixtureGovernancePort implements IGovernancePort {
  constructor(private backend: IWosBackend) {}
  async listAgents(workflowUrl: string): Promise<AgentView[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    return (bundle.ai?.agents ?? []).map(a => ({
      id: a.id, name: a.id, type: a.type ?? 'llm', version: '1.0', status: 'active',
      capabilities: (a.capabilities ?? []).map(c => ({ name: c.name, autonomy: c.autonomy ?? 'assistive' })),
    }));
  }
  async listDeonticConstraints(workflowUrl: string): Promise<DeonticConstraintView[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const dc = bundle.ai?.deonticConstraints;
    if (!dc) return [];
    const result: DeonticConstraintView[] = [];
    for (const p of dc.permissions ?? []) {
      result.push({ kind: 'permission', id: p.id, summary: p.allowedFields ? `Allowed fields: ${p.allowedFields.join(', ')}` : p.bounds ?? 'Permission', detail: p.field ?? undefined, onViolation: p.onViolation, bypassable: p.bypassable });
    }
    for (const p of dc.prohibitions ?? []) {
      result.push({ kind: 'prohibition', id: p.id, summary: p.condition, detail: p.reason, onViolation: p.onViolation, bypassable: p.bypassable });
    }
    for (const o of dc.obligations ?? []) {
      result.push({ kind: 'obligation', id: o.id, summary: o.requirement, detail: o.reason, onViolation: o.onViolation, bypassable: o.bypassable });
    }
    for (const r of dc.rights ?? []) {
      result.push({ kind: 'right', id: r.id, summary: r.entitlement, detail: r.description });
    }
    return result;
  }
  async getQualityControls(workflowUrl: string): Promise<QualityControlsView | null> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const qc = bundle.governance?.qualityControls;
    if (!qc) return null;
    return {
      reviewSampling: qc.reviewSampling ? { rate: qc.reviewSampling.rate, method: qc.reviewSampling.method, scope: qc.reviewSampling.scope } : undefined,
      separationOfDuties: qc.separationOfDuties ? { scope: qc.separationOfDuties.scope, excludeRoles: qc.separationOfDuties.excludeRoles } : undefined,
      overrideAuthority: qc.overrideAuthority ? { requireStructuredRationale: qc.overrideAuthority.requireStructuredRationale, requireAuthorityVerification: qc.overrideAuthority.requireAuthorityVerification, requireSupportingEvidence: qc.overrideAuthority.requireSupportingEvidence } : undefined,
    };
  }
  async listPipelines(workflowUrl: string): Promise<PipelineView[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const pipelines = bundle.governance?.pipelines;
    if (!pipelines) return [];
    return pipelines.map(p => ({
      id: p.id,
      stages: (p.stages ?? []).map(s => ({
        id: s.id,
        type: s.type as PipelineStageView['type'],
        contractRef: s.contractRef,
        assertions: (s.assertions ?? []).map(a => ({
          type: a.type,
          expression: a.expression,
          fields: a.fields,
          description: a.description,
          rejectionPolicy: a.rejectionPolicy,
        })),
        rejectionPolicy: s.rejectionPolicy as PipelineStageView['rejectionPolicy'],
        description: s.description,
      })),
      description: p.description ?? undefined,
    }));
  }
  async getVerificationReport(workflowUrl: string): Promise<VerificationReportView | null> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const vr = bundle.verificationReport;
    if (!vr) return null;
    return {
      solver: { name: vr.solver.name, version: vr.solver.version, timeout: vr.solver.timeout },
      results: vr.results.map(r => ({
        constraintRef: r.constraintRef,
        result: r.result as VerificationResultView['result'],
        solverTimeMs: r.solverTimeMs,
        notes: r.notes,
        counterexample: r.counterexample ? { inputs: r.counterexample.inputs, explanation: r.counterexample.explanation } : undefined,
      })),
      summary: vr.summary ? { totalConstraints: vr.summary.totalConstraints, provenSafe: vr.summary.provenSafe, provenUnsafe: vr.summary.provenUnsafe, inconclusive: vr.summary.inconclusive, totalSolverTimeMs: vr.summary.totalSolverTimeMs } : undefined,
    };
  }
  async getEquityConfig(workflowUrl: string): Promise<EquityConfigView | null> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const eq = bundle.equity;
    if (!eq) return null;
    return {
      protectedCategories: (eq.protectedCategories ?? []).map(c => ({
        id: c.id,
        groupByPath: c.groupByPath,
        description: c.description ?? undefined,
        groups: c.groups ?? [],
      })),
      disparityMethods: (eq.disparityMethods ?? []).map(m => ({
        id: m.id,
        method: m.method,
        description: m.description ?? undefined,
      })),
      reportingSchedule: eq.reportingSchedule ? { frequency: eq.reportingSchedule.frequency, recipientRoles: eq.reportingSchedule.recipientRoles } : undefined,
      remediationTriggers: (eq.remediationTriggers ?? []).map(t => ({
        condition: t.condition,
        action: t.action,
        notifyRoles: t.notifyRoles ?? [],
        description: t.description ?? undefined,
      })),
    };
  }
  async listDelegations(workflowUrl: string): Promise<DelegationEntry[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const govs = bundle.governance;
    if (!govs) return [];
    const delegations = govs.delegations;
    if (!delegations) return [{ id: 'del-1', delegator: 'Director M. Smith', delegate: 'Sarah Jenkins', scope: 'Eligibility Determination', authority: 'determination', legalInstrument: 'DOA-2025-001', startDate: '2026-01-01', endDate: '2026-12-31', status: 'active' as const }];
    return delegations.map(d => ({
      id: d.id,
      delegator: d.delegator,
      delegate: d.delegate,
      scope: typeof d.scope === 'object' ? (d.scope.caseTypes?.join(', ') ?? 'general') : String(d.scope),
      authority: d.authority,
      legalInstrument: d.legalInstrument,
      startDate: d.effectiveDate ?? '',
      endDate: d.expirationDate,
      status: 'active' as const,
    }));
  }
  async revokeDelegation() {}
  async listPolicyVersions(workflowUrl: string): Promise<PolicyVersionView[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const pp = bundle.policyParameters;
    if (!pp) return [{ id: 'v1', label: 'FY2026-Q2', effectiveDate: '2026-04-01', parameterCount: 5, status: 'active' as const }];
    return [{ id: 'v1', label: pp.title ?? 'Current', effectiveDate: '2026-04-01', parameterCount: Object.keys(pp.parameters ?? {}).length, status: 'active' as const }];
  }
  async listCalendarEvents(workflowUrl: string): Promise<CalendarEventView[]> {
    const bundle = await this.backend.loadBundle(workflowUrl);
    const cal = bundle.businessCalendar;
    if (!cal?.holidays) return [];
    return cal.holidays.map((h, i) => ({ id: `hol-${i}`, name: h.name, date: h.date, type: 'federal' as const, impactsDeadlines: true }));
  }
  async getHealthStatus(): Promise<ServiceHealthView[]> {
    return [
      { id: 'irs', name: 'IRS Data Bridge', status: 'healthy', latency: '120ms', errorRate: '0.1%', lastCheck: new Date().toISOString() },
      { id: 'notif', name: 'Notification Service', status: 'healthy', latency: '45ms', errorRate: '0%', lastCheck: new Date().toISOString() },
      { id: 'ai', name: 'AI Extraction Service', status: 'degraded', latency: '890ms', errorRate: '2.3%', lastCheck: new Date().toISOString() },
    ];
  }
}

export class FixtureDashboardPort implements IDashboardPort {
  constructor(private backend: IWosBackend) {}
  async getMetrics(): Promise<DashboardMetrics> {
    const res = await this.backend.listInstances();
    return {
      activeInstances: res.total, completed7d: 12, slaCompliance: 94, avgProcessingTimeDays: 4.2, aiAcceptanceRate: 87,
      activeInstancesTrend: 5, completed7dTrend: -2, slaComplianceTrend: 1, avgProcessingTimeTrend: -0.3, aiAcceptanceRateTrend: 3,
    };
  }
  async getStageMetrics(): Promise<StageMetricView[]> {
    const kernel = (await this.backend.loadBundle('https://agency.gov/workflows/benefits-adjudication')).kernel;
    const states = kernel.lifecycle?.states ?? {};
    return Object.entries(states).slice(0, 6).map(([name], i) => ({ name, count: (i + 1) * 2, avgWait: `${(i % 3) + 1}d`, status: 'normal' as const }));
  }
  async getAlerts(): Promise<AlertView[]> {
    return [
      { id: 'a1', type: 'drift', title: 'Decision drift detected', description: 'Override rate for caseworkerA increased 15% this week', timeAgo: '2h ago', severity: 'warning' },
      { id: 'a2', type: 'sla', title: 'SLA breach risk', description: '3 cases approaching 30-day determination deadline', timeAgo: '30m ago', severity: 'critical' },
    ];
  }
  async getDriftData(): Promise<DriftDataPoint[]> {
    return ['Week 1', 'Week 2', 'Week 3', 'Week 4', 'Week 5', 'Week 6'].map((week, i) => ({ week, overrideRate: 5 + i * 2, timeOnTask: 20 + i * 2 }));
  }
  async getPipelineData(): Promise<PipelineDataPoint[]> {
    return [{ name: 'Intake', volume: 45, capacity: 50 }, { name: 'Review', volume: 32, capacity: 40 }, { name: 'Determination', volume: 18, capacity: 20 }];
  }
}

export class FixtureApplicantPort implements IApplicantPort {
  constructor(private backend: IWosBackend) {}
  async getDetermination(instanceId: string): Promise<ApplicantDeterminationView | null> {
    const inst = await this.backend.getInstance(instanceId);
    if (!inst) return null;
    const cs = inst.caseState;
    const determination = cs.determination as { decision?: string; reason?: string } | undefined;
    return {
      instanceId,
      programName: 'Housing Benefits',
      decision: (determination?.decision as ApplicantDeterminationView['decision']) ?? 'pending',
      dateIssued: inst.updatedAt,
      deadlineDate: inst.timers.find(t => t.event === 'appealWindowExpired')?.deadline ?? '',
      benefitsContinue: false,
      summary: determination?.reason ?? 'Under review',
      evidenceConsidered: ['Tax Return 2025', 'Utility Bill March 2026', 'ID Verification'],
      rulesApplied: ['Income Eligibility Rule v4'],
      aiDisclosure: { wasUsed: true, description: 'AI assisted in document extraction and income verification.' },
      counterfactuals: { positive: [], negative: [] },
      appealStatus: 'not-filed',
      milestones: inst.configuration.map((s, i) => ({ id: `m-${i}`, label: s, status: i < inst.configuration.length - 1 ? 'completed' as const : 'current' as const, description: `State: ${s}` })),
    };
  }
  async submitAppeal(_instanceId: string, _reason: string): Promise<void> {}
}

export class StubRealtimePort implements IRealtimePort {
  connect() {}
  disconnect() {}
  onKernelInit(): Unsubscribe { return () => {}; }
  onKernelChanged(): Unsubscribe { return () => {}; }
  onCollaboratorsUpdate(): Unsubscribe { return () => {}; }
  onCursorUpdate(): Unsubscribe { return () => {}; }
  sendCursorMove() {}
  sendKernelUpdate() {}
}

export class FixtureAuthPort implements IAuthPort {
  private user: AuthUser = {
    id: 'user-1',
    name: 'Jane Doe',
    email: 'jane.doe@agency.gov',
    role: 'Supervisor',
  };

  async getCurrentUser(): Promise<AuthUser | null> { return this.user; }
  async login(): Promise<AuthUser> { return this.user; }
  async logout(): Promise<void> {}
  async hasRole(role: string): Promise<boolean> { return this.user.role === role; }
}
