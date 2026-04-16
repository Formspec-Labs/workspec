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

export interface SortConfig {
  field: string;
  direction: 'asc' | 'desc';
}

export interface BackgroundJob {
  id: string;
  type: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  progress: number;
  message?: string;
  startedAt: string;
  completedAt?: string;
  error?: string;
}

export interface SavedView {
  id: string;
  name: string;
  filters: unknown;
  icon?: string;
}

export interface BulkActionImpact {
  total: number;
  warnings: string[];
  riskLevel: 'low' | 'medium' | 'high';
}

export interface Notification {
  id: string;
  type: 'assignment' | 'sla-warning' | 'sla-breach' | 'escalation' | 'hold-expired' | 'review-activated' | 'system';
  urgency: 'critical' | 'warning' | 'info';
  title: string;
  message: string;
  timestamp: string;
  read: boolean;
  link?: { type: 'task' | 'case' | 'dashboard'; id: string };
}

export interface OutboundNotification {
  id: string;
  recipient: string;
  type: 'approval-letter' | 'denial-letter' | 'request-for-info' | 'status-update';
  caseId: string;
  status: 'pending' | 'sent' | 'confirmed' | 'failed';
  timestamp: string;
  channel: 'email' | 'mail' | 'sms';
  contentHash: string;
  auditTrail: { event: string; timestamp: string; actor: string }[];
}

export interface ReportTemplate {
  id: string;
  title: string;
  description: string;
  category: 'operational' | 'compliance' | 'ai-performance' | 'caseload';
  icon: string;
}

export interface ReportConfig {
  id?: string;
  name: string;
  type: 'template' | 'custom';
  templateId?: string;
  metrics: string[];
  dimensions: string[];
  visualization: 'table' | 'line' | 'bar' | 'heatmap';
  filters: Record<string, unknown>;
  dateRange: { start: string; end: string };
}
