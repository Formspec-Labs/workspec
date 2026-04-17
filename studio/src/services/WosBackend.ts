import type { WOSKernelDocument } from '../types/wos/kernel';
import type { WOSWorkflowGovernanceDocument } from '../types/wos/workflow-governance';
import type { WOSAIIntegrationDocument } from '../types/wos/ai-integration';
import type { WOSPolicyParameterConfig } from '../types/wos/policy-parameters';
import type { WOSBusinessCalendarConfig } from '../types/wos/business-calendar';
import type { WOSNotificationTemplateConfig } from '../types/wos/notification-template';
import type { WOSAdvancedGovernanceDocument } from '../types/wos/advanced';
import type { WOSEquityConfig } from '../types/wos/equity';
import type { WOSDriftMonitorConfig } from '../types/wos/drift-monitor';
import type { WOSAgentConfig } from '../types/wos/agent-config';
import type { WOSDueProcessConfig } from '../types/wos/due-process';
import type { WOSAssertionGateLibrary } from '../types/wos/assertion-gate';
import type { WOSVerificationReport } from '../types/wos/verification-report';
import type { WOSCorrespondenceMetadataConfig } from '../types/wos/correspondence-metadata';
import type { WOSSemanticProfileDocument } from '../types/wos/semantic-profile';
import type { WOSIntegrationProfileDocument } from '../types/wos/integration-profile';
import type { WOSLifecycleDetailConfiguration } from '../types/wos/lifecycle-detail';
import type { WOSCaseInstance } from '../types/wos/case-instance';

export interface ProvenanceRecord {
  id: string;
  instanceId: string;
  timestamp: string;
  tier: 'facts' | 'reasoning' | 'ai-narrative' | 'counterfactual';
  actor: { id: string; type: 'human' | 'system' | 'agent'; name: string };
  event: string;
  sourceState: string;
  targetState: string;
  facts: { inputs: Record<string, unknown>; outputs: Record<string, unknown>; metadata: Record<string, unknown> };
  reasoning?: { rulesApplied: string[]; criteriaChecked: { label: string; passed: boolean }[]; explanation?: string; sourceAuthority?: string };
  aiNarrative?: { text: string; model: string; version: string; confidence?: number };
  counterfactual?: { positive: string[]; negative: string[] };
  authorityChain?: { actor: string; delegatedBy?: string; legalInstrument?: string; isValid: boolean }[];
  integrity: { hash: string; previousHash: string };
}

export interface CaseInstanceView {
  instanceId: string;
  definitionUrl: string;
  definitionVersion: string;
  status: 'active' | 'suspended' | 'migrating' | 'completed' | 'terminated';
  configuration: string[];
  caseState: Record<string, unknown>;
  activeTasks: ActiveTaskView[];
  timers: TimerView[];
  governanceState?: {
    activeDelegations: DelegationView[];
    activeHolds: HoldView[];
    reviewState: Record<string, unknown>;
  };
  impactLevel: string;
  createdAt: string;
  updatedAt: string;
}

export interface ActiveTaskView {
  taskId: string;
  taskRef: string;
  status: 'created' | 'assigned' | 'claimed' | 'delegated' | 'escalated';
  assignedActor?: string;
  contractRef?: string;
  binding?: 'formspec' | 'jsonSchema';
  deadline?: string;
  impactLevel?: string;
  createdAt: string;
  updatedAt: string;
}

export interface TimerView {
  timerId: string;
  deadline: string;
  event: string;
  scopeState?: string;
}

export interface DelegationView {
  delegatorId: string;
  delegateId: string;
  scope: string;
  authority?: 'signing' | 'determination' | 'review' | 'override';
  grantedAt: string;
  expiresAt?: string;
}

export interface HoldView {
  holdType: string;
  startedAt: string;
  expectedEnd?: string;
  resumeTrigger: string;
  holdState?: string;
}

export interface EvaluationResult {
  previousConfiguration: string[];
  newConfiguration: string[];
  eventsFired: string[];
  provenanceRecord?: ProvenanceRecord;
  caseStateMutations: Record<string, unknown>;
}

export interface AvailableTransition {
  event: string;
  target: string;
  guard?: string;
  guardSatisfied: boolean;
  tags: string[];
  description?: string;
}

export interface WosDocumentBundle {
  kernel: WOSKernelDocument;
  governance?: WOSWorkflowGovernanceDocument;
  dueProcess?: WOSDueProcessConfig;
  assertionGates?: WOSAssertionGateLibrary;
  ai?: WOSAIIntegrationDocument;
  policyParameters?: WOSPolicyParameterConfig;
  notificationTemplates?: WOSNotificationTemplateConfig;
  businessCalendar?: WOSBusinessCalendarConfig;
  advanced?: WOSAdvancedGovernanceDocument;
  equity?: WOSEquityConfig;
  driftMonitor?: WOSDriftMonitorConfig;
  agentConfigs?: WOSAgentConfig[];
  verificationReport?: WOSVerificationReport;
  correspondenceMetadata?: WOSCorrespondenceMetadataConfig;
  semanticProfile?: WOSSemanticProfileDocument;
  integrationProfile?: WOSIntegrationProfileDocument;
  lifecycleDetail?: WOSLifecycleDetailConfiguration;
  caseInstances?: WOSCaseInstance[];
}

export interface KernelSummary {
  url: string;
  title: string;
  version: string;
  status: string;
  impactLevel: string;
}

export interface InstanceFilter {
  status?: string[];
  impactLevel?: string[];
  configuration?: string[];
  definitionUrl?: string;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface IWosBackend {
  loadBundle(workflowUrl: string): Promise<WosDocumentBundle>;
  listBundles(): Promise<KernelSummary[]>;
  getInstance(instanceId: string): Promise<CaseInstanceView | null>;
  listInstances(filter?: InstanceFilter, page?: number, pageSize?: number): Promise<PaginatedResult<CaseInstanceView>>;
  getProvenance(instanceId: string): Promise<ProvenanceRecord[]>;
  submitEvent(instanceId: string, event: string, actorId: string, data?: Record<string, unknown>): Promise<EvaluationResult>;
  getAvailableTransitions(instanceId: string): Promise<AvailableTransition[]>;
}
