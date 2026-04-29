import type { ProvenanceRecord, CaseInstanceView, ActiveTaskView, AvailableTransition, EvaluationResult } from './WosBackend';
import type { WOSKernelDocument } from '../types/wos/kernel';
import type { WOSWorkflowGovernanceDocument } from '../types/wos/workflow-governance';
import type { WOSAIIntegrationDocument } from '../types/wos/ai-integration';
import type { WOSPolicyParameterConfig } from '../types/wos/policy-parameters';
import type { WOSBusinessCalendarConfig } from '../types/wos/business-calendar';
import type { WOSAgentConfig } from '../types/wos/agent-config';
import type { WOSSignatureProfileDocument } from '../types/wos/signature-profile';
import type { KernelSummary, InstanceFilter, PaginatedResult } from './WosBackend';

export interface TaskListItem {
  taskId: string;
  instanceId: string;
  taskRef: string;
  status: ActiveTaskView['status'];
  assignedActor?: string;
  deadline?: string;
  impactLevel?: string;
  configuration: string[];
  caseState: Record<string, unknown>;
  definitionTitle: string;
  definitionUrl: string;
  createdAt: string;
}

export interface IInboxPort {
  listTasks(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<TaskListItem>>;
  getTask(taskId: string): Promise<TaskListItem | null>;
}

export interface ICaseViewerPort {
  getInstance(instanceId: string): Promise<CaseInstanceView | null>;
  getProvenance(instanceId: string): Promise<ProvenanceRecord[]>;
  getTimeline(instanceId: string): Promise<ProvenanceRecord[]>;
}

export interface IWorkflowDesignPort {
  listWorkflows(): Promise<KernelSummary[]>;
  loadKernel(workflowUrl: string): Promise<WOSKernelDocument | null>;
  saveKernel(kernel: WOSKernelDocument): Promise<void>;
  validateKernel(kernel: WOSKernelDocument): Promise<WosValidationResult>;
}

export interface WosValidationResult {
  isValid: boolean;
  issues: WosValidationIssue[];
}

export interface WosValidationIssue {
  severity: 'error' | 'warning';
  category: 'structure' | 'policy' | 'soundness' | 'satisfiability';
  message: string;
  targetId?: string;
}

export interface AgentView {
  id: string;
  name: string;
  type: string;
  version: string;
  status: string;
  capabilities: { name: string; autonomy: string }[];
  confidenceFloor?: number;
}

export interface DelegationEntry {
  id: string;
  delegator: string;
  delegate: string;
  scope: string;
  authority?: string;
  legalInstrument?: string;
  startDate: string;
  endDate?: string;
  status: 'active' | 'expired' | 'revoked';
}

export interface PolicyVersionView {
  id: string;
  label: string;
  effectiveDate: string;
  expiryDate?: string;
  parameterCount: number;
  status: 'active' | 'upcoming' | 'archived';
}

export interface CalendarEventView {
  id: string;
  name: string;
  date: string;
  type: 'federal' | 'state' | 'agency';
  impactsDeadlines: boolean;
}

export interface ServiceHealthView {
  id: string;
  name: string;
  status: 'healthy' | 'degraded' | 'down';
  latency: string;
  errorRate: string;
  lastCheck: string;
}

export interface DeonticConstraintView {
  kind: 'permission' | 'prohibition' | 'obligation' | 'right';
  id: string;
  summary: string;
  detail?: string;
  onViolation?: string;
  bypassable?: boolean;
}

export interface QualityControlsView {
  reviewSampling?: { rate: number; method?: string; scope?: string };
  separationOfDuties?: { scope: string; excludeRoles?: string[] };
  overrideAuthority?: { requireStructuredRationale?: boolean; requireAuthorityVerification?: boolean; requireSupportingEvidence?: boolean };
}

export interface PipelineStageView {
  id: string;
  type: 'contract-validation' | 'assertion-gate' | 'transform' | 'human-review';
  contractRef?: string;
  assertions?: { type: string; expression?: string; fields?: string[]; description?: string; rejectionPolicy?: string }[];
  rejectionPolicy?: string;
  description?: string;
}

export interface PipelineView {
  id: string;
  stages: PipelineStageView[];
  description?: string;
}

export interface EquityCategoryView {
  id: string;
  groupByPath: string;
  description?: string;
  groups: string[];
}

export interface EquityRemediationTriggerView {
  condition: string;
  action: string;
  notifyRoles: string[];
  description?: string;
}

export interface EquityConfigView {
  protectedCategories: EquityCategoryView[];
  disparityMethods: { id: string; method: string; description?: string }[];
  reportingSchedule?: { frequency?: string; recipientRoles?: string[] };
  remediationTriggers?: EquityRemediationTriggerView[];
}

export interface IGovernanceReader {
  listAgents(workflowUrl: string): Promise<AgentView[]>;
  listDeonticConstraints(workflowUrl: string): Promise<DeonticConstraintView[]>;
  getQualityControls(workflowUrl: string): Promise<QualityControlsView | null>;
  listPipelines(workflowUrl: string): Promise<PipelineView[]>;
  getEquityConfig(workflowUrl: string): Promise<EquityConfigView | null>;
  listDelegations(workflowUrl: string): Promise<DelegationEntry[]>;
  listPolicyVersions(workflowUrl: string): Promise<PolicyVersionView[]>;
  listCalendarEvents(workflowUrl: string): Promise<CalendarEventView[]>;
  getHealthStatus(): Promise<ServiceHealthView[]>;
}

export interface IGovernanceWriter {
  revokeDelegation(workflowUrl: string, delegationId: string): Promise<void>;
}

export interface IGovernancePort extends IGovernanceReader, IGovernanceWriter {}

export interface DashboardMetrics {
  activeInstances: number;
  completed7d: number;
  slaCompliance: number;
  avgProcessingTimeDays: number;
  aiAcceptanceRate: number;
  activeInstancesTrend: number;
  completed7dTrend: number;
  slaComplianceTrend: number;
  avgProcessingTimeTrend: number;
  aiAcceptanceRateTrend: number;
}

export interface StageMetricView {
  name: string;
  count: number;
  avgWait: string;
  status: 'normal' | 'warning' | 'bottleneck';
}

export interface AlertView {
  id: string;
  type: 'drift' | 'queue' | 'sla';
  title: string;
  description: string;
  timeAgo: string;
  severity: 'critical' | 'warning' | 'info';
}

export interface DriftDataPoint {
  week: string;
  overrideRate: number;
  timeOnTask: number;
}

export interface PipelineDataPoint {
  name: string;
  volume: number;
  capacity: number;
}

export interface IDashboardPort {
  getMetrics(): Promise<DashboardMetrics>;
  getStageMetrics(): Promise<StageMetricView[]>;
  getAlerts(): Promise<AlertView[]>;
  getDriftData(): Promise<DriftDataPoint[]>;
  getPipelineData(): Promise<PipelineDataPoint[]>;
}

export interface ApplicantDeterminationView {
  instanceId: string;
  programName: string;
  decision: 'approved' | 'denied' | 'reduced' | 'terminated' | 'pending';
  dateIssued: string;
  deadlineDate: string;
  benefitsContinue: boolean;
  summary: string;
  evidenceConsidered: string[];
  rulesApplied: string[];
  aiDisclosure: { wasUsed: boolean; description?: string; humanReviewer?: string };
  counterfactuals: { positive: string[]; negative: string[] };
  appealStatus: 'not-filed' | 'filed' | 'under-review' | 'hearing-scheduled' | 'decided';
  milestones: { id: string; label: string; status: 'completed' | 'current' | 'pending'; description: string; date?: string }[];
}

export interface IApplicantPort {
  getDetermination(instanceId: string): Promise<ApplicantDeterminationView | null>;
  submitAppeal(instanceId: string, reason: string): Promise<void>;
}

export type Unsubscribe = () => void;

export interface IRealtimePort {
  connect(): void;
  disconnect(): void;
  onKernelInit(cb: (kernel: WOSKernelDocument, url?: string) => void): Unsubscribe;
  onKernelChanged(cb: (kernel: WOSKernelDocument, url?: string) => void): Unsubscribe;
  onCollaboratorsUpdate(cb: (users: { id: string; name: string; cursor: { x: number; y: number } }[]) => void): Unsubscribe;
  onCursorUpdate(cb: (cursors: { userId: string; cursor: { x: number; y: number } }) => void): Unsubscribe;
  sendCursorMove(pos: { x: number; y: number }): void;
  sendKernelUpdate(kernel: WOSKernelDocument, url?: string): void;
}

export interface AuthUser {
  id: string;
  name: string;
  email: string;
  role: string;
  avatar?: string;
}

export interface IAuthPort {
  getCurrentUser(): Promise<AuthUser | null>;
  login(credentials: { email: string; password: string }): Promise<AuthUser>;
  logout(): Promise<void>;
  hasRole(role: string): Promise<boolean>;
}

export interface SignatureProfileSummary {
  id: string;
  targetWorkflowUrl: string;
  flowType: 'sequential' | 'parallel' | 'routed' | 'free-for-all';
  roleCount: number;
  documentCount: number;
}

export interface ISignatureProfilePort {
  list(): Promise<SignatureProfileSummary[]>;
  load(profileId: string): Promise<WOSSignatureProfileDocument | null>;
  save(profile: WOSSignatureProfileDocument): Promise<WosValidationResult>;
  validate(profile: WOSSignatureProfileDocument): Promise<WosValidationResult>;
}
