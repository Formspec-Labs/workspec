import type {
  IWosBackend, WosDocumentBundle, KernelSummary, CaseInstanceView,
  ProvenanceRecord, EvaluationResult, AvailableTransition, InstanceFilter, PaginatedResult,
} from './WosBackend';
import type {
  IInboxPort, TaskListItem, ICaseViewerPort, IWorkflowDesignPort, WosValidationResult,
  IGovernancePort, AgentView, DelegationEntry, DeonticConstraintView, QualityControlsView,
  PipelineView, VerificationReportView, EquityConfigView,
  PolicyVersionView, CalendarEventView, ServiceHealthView,
  IDashboardPort, DashboardMetrics, StageMetricView, AlertView, DriftDataPoint, PipelineDataPoint,
  IApplicantPort, ApplicantDeterminationView, IAuthPort, AuthUser,
  ISignatureProfilePort, SignatureProfileSummary,
} from './WosPorts';
import type { WOSSignatureProfileDocument } from '../types/wos/signature-profile';
import type { WOSKernelDocument } from '../types/wos/kernel';
import { validateKernelDocument } from './wos-kernel-validator';
import { authedFetch, storeLogin, storeLogout, type TokenPair } from './authedFetch';

const API_BASE = '/api';

function assertBundle(data: unknown): WosDocumentBundle {
  if (!data || typeof data !== 'object') throw new Error('Invalid bundle response');
  const bundle = data as WosDocumentBundle;
  if (!bundle.kernel) throw new Error('Bundle missing kernel');
  const validation = validateKernelDocument(bundle.kernel);
  if (!validation.isValid) {
    throw new Error(`Bundle kernel failed schema validation: ${validation.issues.slice(0, 3).map(i => i.message).join('; ')}`);
  }
  return bundle;
}

function assertCaseInstance(data: unknown): CaseInstanceView {
  if (!data || typeof data !== 'object') throw new Error('Invalid instance response');
  const inst = data as Record<string, unknown>;
  for (const field of ['instanceId', 'definitionUrl', 'status']) {
    if (!(field in inst)) throw new Error(`Instance missing required field "${field}"`);
  }
  return inst as unknown as CaseInstanceView;
}

export class HttpWosBackend implements IWosBackend {
  async loadBundle(workflowUrl: string): Promise<WosDocumentBundle> {
    const res = await fetch(`${API_BASE}/bundles/${encodeURIComponent(workflowUrl)}`);
    if (!res.ok) throw new Error(`Failed to load bundle: ${res.status}`);
    const data = await res.json();
    return assertBundle(data);
  }

  async listBundles(): Promise<KernelSummary[]> {
    const res = await fetch(`${API_BASE}/bundles`);
    if (!res.ok) throw new Error(`Failed to list bundles: ${res.status}`);
    return res.json();
  }

  async getInstance(instanceId: string): Promise<CaseInstanceView | null> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get instance: ${res.status}`);
    return assertCaseInstance(await res.json());
  }

  async listInstances(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<CaseInstanceView>> {
    const params = new URLSearchParams();
    if (filter?.status) params.set('status', filter.status.join(','));
    if (filter?.impactLevel) params.set('impactLevel', filter.impactLevel.join(','));
    if (filter?.definitionUrl) params.set('definitionUrl', filter.definitionUrl);
    if (page) params.set('page', String(page));
    if (pageSize) params.set('pageSize', String(pageSize));
    const res = await fetch(`${API_BASE}/instances?${params}`);
    if (!res.ok) throw new Error(`Failed to list instances: ${res.status}`);
    return res.json();
  }

  async getProvenance(instanceId: string): Promise<ProvenanceRecord[]> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}/provenance`);
    if (!res.ok) throw new Error(`Failed to get provenance: ${res.status}`);
    return res.json();
  }

  async submitEvent(instanceId: string, event: string, actorId: string, data?: Record<string, unknown>): Promise<EvaluationResult> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ event, actorId, data }),
    });
    if (!res.ok) throw new Error(`Failed to submit event: ${res.status}`);
    const payload = await res.json();
    return payload as EvaluationResult;
  }

  async getAvailableTransitions(instanceId: string): Promise<AvailableTransition[]> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}/transitions`);
    if (!res.ok) throw new Error(`Failed to get transitions: ${res.status}`);
    return res.json();
  }
}

export class HttpInboxPort implements IInboxPort {
  async listTasks(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<TaskListItem>> {
    const params = new URLSearchParams();
    if (filter?.status) params.set('status', filter.status.join(','));
    if (filter?.impactLevel) params.set('impactLevel', filter.impactLevel.join(','));
    if (filter?.definitionUrl) params.set('definitionUrl', filter.definitionUrl);
    if (page) params.set('page', String(page));
    if (pageSize) params.set('pageSize', String(pageSize));
    const res = await fetch(`${API_BASE}/tasks?${params}`);
    if (!res.ok) throw new Error(`Failed to list tasks: ${res.status}`);
    return res.json();
  }

  async getTask(taskId: string): Promise<TaskListItem | null> {
    const res = await fetch(`${API_BASE}/tasks/${encodeURIComponent(taskId)}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get task: ${res.status}`);
    return res.json();
  }
}

export class HttpCaseViewerPort implements ICaseViewerPort {
  async getInstance(instanceId: string): Promise<CaseInstanceView | null> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get instance: ${res.status}`);
    return res.json();
  }

  async getProvenance(instanceId: string): Promise<ProvenanceRecord[]> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}/provenance`);
    if (!res.ok) throw new Error(`Failed to get provenance: ${res.status}`);
    return res.json();
  }

  async getTimeline(instanceId: string): Promise<ProvenanceRecord[]> {
    const res = await fetch(`${API_BASE}/instances/${encodeURIComponent(instanceId)}/provenance`);
    if (!res.ok) throw new Error(`Failed to get timeline: ${res.status}`);
    return res.json();
  }
}

export class HttpWorkflowDesignPort implements IWorkflowDesignPort {
  async listWorkflows(): Promise<KernelSummary[]> {
    const res = await fetch(`${API_BASE}/bundles`);
    if (!res.ok) throw new Error(`Failed to list workflows: ${res.status}`);
    return res.json();
  }

  async loadKernel(workflowUrl: string): Promise<WOSKernelDocument | null> {
    const res = await fetch(`${API_BASE}/bundles/${encodeURIComponent(workflowUrl)}/kernel`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to load kernel: ${res.status}`);
    const kernel = await res.json();
    const validation = validateKernelDocument(kernel);
    if (!validation.isValid) {
      throw new Error(`Kernel response failed schema validation: ${validation.issues.slice(0, 3).map(i => i.message).join('; ')}`);
    }
    return kernel as WOSKernelDocument;
  }

  async saveKernel(kernel: WOSKernelDocument): Promise<void> {
    const local = validateKernelDocument(kernel);
    if (!local.isValid) {
      throw new Error(`Kernel validation failed: ${local.issues.map(i => i.message).join(', ')}`);
    }
    if (!kernel.url) throw new Error('Kernel.url is required to persist');
    const saveRes = await fetch(`${API_BASE}/bundles/${encodeURIComponent(kernel.url)}/kernel`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(kernel),
    });
    if (!saveRes.ok) {
      const body = await saveRes.json().catch(() => ({}));
      const issues = Array.isArray((body as { issues?: unknown }).issues) ? (body as { issues: { message: string }[] }).issues.map(i => i.message).join(', ') : '';
      throw new Error(`Failed to save kernel: ${saveRes.status}${issues ? ` — ${issues}` : ''}`);
    }
  }

  async validateKernel(kernel: WOSKernelDocument): Promise<WosValidationResult> {
    return validateKernelDocument(kernel);
  }
}

export class HttpGovernancePort implements IGovernancePort {
  async listAgents(workflowUrl: string): Promise<AgentView[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/agents`);
    if (!res.ok) throw new Error(`Failed to list agents: ${res.status}`);
    return res.json();
  }

  async listDeonticConstraints(workflowUrl: string): Promise<DeonticConstraintView[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/deontic-constraints`);
    if (!res.ok) throw new Error(`Failed to list deontic constraints: ${res.status}`);
    return res.json();
  }

  async getQualityControls(workflowUrl: string): Promise<QualityControlsView | null> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/quality-controls`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get quality controls: ${res.status}`);
    return res.json();
  }

  async listPipelines(workflowUrl: string): Promise<PipelineView[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/pipelines`);
    if (!res.ok) throw new Error(`Failed to list pipelines: ${res.status}`);
    return res.json();
  }

  async getVerificationReport(workflowUrl: string): Promise<VerificationReportView | null> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/verification-report`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get verification report: ${res.status}`);
    return res.json();
  }

  async getEquityConfig(workflowUrl: string): Promise<EquityConfigView | null> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/equity-config`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get equity config: ${res.status}`);
    return res.json();
  }

  async listDelegations(workflowUrl: string): Promise<DelegationEntry[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/delegations`);
    if (!res.ok) throw new Error(`Failed to list delegations: ${res.status}`);
    return res.json();
  }

  async revokeDelegation(workflowUrl: string, delegationId: string): Promise<void> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/delegations/${encodeURIComponent(delegationId)}`, {
      method: 'DELETE',
    });
    if (!res.ok) throw new Error(`Failed to revoke delegation: ${res.status}`);
  }

  async listPolicyVersions(workflowUrl: string): Promise<PolicyVersionView[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/policy-versions`);
    if (!res.ok) throw new Error(`Failed to list policy versions: ${res.status}`);
    return res.json();
  }

  async listCalendarEvents(workflowUrl: string): Promise<CalendarEventView[]> {
    const res = await fetch(`${API_BASE}/governance/${encodeURIComponent(workflowUrl)}/calendar-events`);
    if (!res.ok) throw new Error(`Failed to list calendar events: ${res.status}`);
    return res.json();
  }

  async getHealthStatus(): Promise<ServiceHealthView[]> {
    const res = await fetch(`${API_BASE}/health`);
    if (!res.ok) throw new Error(`Failed to get health status: ${res.status}`);
    return res.json();
  }
}

export class HttpDashboardPort implements IDashboardPort {
  async getMetrics(): Promise<DashboardMetrics> {
    const res = await fetch(`${API_BASE}/dashboard/metrics`);
    if (!res.ok) throw new Error(`Failed to get dashboard metrics: ${res.status}`);
    return res.json();
  }

  async getStageMetrics(): Promise<StageMetricView[]> {
    const res = await fetch(`${API_BASE}/dashboard/stage-metrics`);
    if (!res.ok) throw new Error(`Failed to get stage metrics: ${res.status}`);
    return res.json();
  }

  async getAlerts(): Promise<AlertView[]> {
    const res = await fetch(`${API_BASE}/dashboard/alerts`);
    if (!res.ok) throw new Error(`Failed to get alerts: ${res.status}`);
    return res.json();
  }

  async getDriftData(): Promise<DriftDataPoint[]> {
    const res = await fetch(`${API_BASE}/dashboard/drift-data`);
    if (!res.ok) throw new Error(`Failed to get drift data: ${res.status}`);
    return res.json();
  }

  async getPipelineData(): Promise<PipelineDataPoint[]> {
    const res = await fetch(`${API_BASE}/dashboard/pipeline-data`);
    if (!res.ok) throw new Error(`Failed to get pipeline data: ${res.status}`);
    return res.json();
  }
}

export class HttpApplicantPort implements IApplicantPort {
  async getDetermination(instanceId: string): Promise<ApplicantDeterminationView | null> {
    const res = await fetch(`${API_BASE}/applicant/${encodeURIComponent(instanceId)}/determination`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get determination: ${res.status}`);
    return res.json();
  }

  async submitAppeal(instanceId: string, reason: string): Promise<void> {
    const res = await fetch(`${API_BASE}/applicant/${encodeURIComponent(instanceId)}/appeal`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason }),
    });
    if (!res.ok) throw new Error(`Failed to submit appeal: ${res.status}`);
  }
}

export class HttpAuthPort implements IAuthPort {
  async getCurrentUser(): Promise<AuthUser | null> {
    const res = await authedFetch(`${API_BASE}/auth/me`);
    if (res.status === 401) return null;
    if (!res.ok) throw new Error(`Failed to get current user: ${res.status}`);
    return res.json();
  }

  async login(credentials: { email: string; password: string }): Promise<AuthUser> {
    const res = await fetch(`${API_BASE}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(credentials),
    });
    if (!res.ok) throw new Error(`Login failed: ${res.status}`);
    const body = await res.json();
    // Rust wos-server returns a TokenPair with the user attached; legacy
    // stubs return the AuthUser directly. Support both.
    if (body && typeof body === 'object' && 'accessToken' in body && 'user' in body) {
      storeLogin(body as TokenPair);
      return (body as TokenPair).user;
    }
    return body as AuthUser;
  }

  async logout(): Promise<void> {
    const res = await authedFetch(`${API_BASE}/auth/logout`, { method: 'POST' });
    storeLogout();
    if (!res.ok) throw new Error(`Logout failed: ${res.status}`);
  }

  async hasRole(role: string): Promise<boolean> {
    const res = await authedFetch(
      `${API_BASE}/auth/has-role/${encodeURIComponent(role)}`,
    );
    if (!res.ok) throw new Error(`Failed to check role: ${res.status}`);
    const data = await res.json();
    return data.hasRole;
  }
}

export class HttpSignatureProfilePort implements ISignatureProfilePort {
  async list(): Promise<SignatureProfileSummary[]> {
    const res = await fetch(`${API_BASE}/signature-profiles`);
    if (res.status === 404) return [];
    if (!res.ok) throw new Error(`Failed to list signature profiles: ${res.status}`);
    return res.json();
  }

  async load(profileId: string): Promise<WOSSignatureProfileDocument | null> {
    const res = await fetch(`${API_BASE}/signature-profiles/${encodeURIComponent(profileId)}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to load signature profile: ${res.status}`);
    return res.json();
  }

  async save(profile: WOSSignatureProfileDocument): Promise<WosValidationResult> {
    const validation = await this.validate(profile);
    if (!validation.isValid) return validation;
    const id = profile.targetWorkflow?.url ?? '';
    const res = await fetch(`${API_BASE}/signature-profiles/${encodeURIComponent(id)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(profile),
    });
    if (!res.ok) throw new Error(`Failed to save signature profile: ${res.status}`);
    return validation;
  }

  async validate(profile: WOSSignatureProfileDocument): Promise<WosValidationResult> {
    const res = await fetch(`${API_BASE}/lint/document`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ document: profile, schemaType: 'signature-profile' }),
    });
    if (!res.ok) throw new Error(`Failed to validate signature profile: ${res.status}`);
    const result = await res.json();
    return {
      isValid: result.isValid ?? true,
      issues: (result.diagnostics ?? []).map((d: { ruleId: string; severity: string; message: string; path: string }) => ({
        severity: d.severity === 'warning' ? 'warning' as const : 'error' as const,
        category: 'policy' as const,
        message: d.message,
        targetId: d.path,
      })),
    };
  }
}

export async function optimisticUpdate<T>(
  optimistic: () => T,
  commit: () => Promise<void>,
  rollback: (state: T) => void,
): Promise<void> {
  const saved = optimistic();
  try {
    await commit();
  } catch (err) {
    rollback(saved);
    throw err;
  }
}
