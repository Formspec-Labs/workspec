import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AdminConsole } from './AdminConsole';
import { WosProvider } from '../../context/WosContext';
import type { IGovernancePort } from '../../services/WosPorts';

const mockGovernance: IGovernancePort = {
  listAgents: vi.fn(),
  listDeonticConstraints: vi.fn(),
  getQualityControls: vi.fn(),
  listPipelines: vi.fn(),
  getEquityConfig: vi.fn(),
  listDelegations: vi.fn(),
  revokeDelegation: vi.fn(),
  listPolicyVersions: vi.fn(),
  listCalendarEvents: vi.fn(),
  getHealthStatus: vi.fn(),
};

const resetMocks = () => {
  mockGovernance.listAgents = vi.fn().mockResolvedValue([]);
  mockGovernance.listDeonticConstraints = vi.fn().mockResolvedValue([]);
  mockGovernance.getQualityControls = vi.fn().mockResolvedValue(null);
  mockGovernance.listPipelines = vi.fn().mockResolvedValue([]);
  mockGovernance.getEquityConfig = vi.fn().mockResolvedValue(null);
  mockGovernance.listDelegations = vi.fn().mockResolvedValue([]);
  mockGovernance.listPolicyVersions = vi.fn().mockResolvedValue([]);
  mockGovernance.listCalendarEvents = vi.fn().mockResolvedValue([]);
  mockGovernance.getHealthStatus = vi.fn().mockResolvedValue([]);
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ governance: mockGovernance }}>
      {ui}
    </WosProvider>
  );
};

async function switchToPolicyTab(label: string | RegExp) {
  fireEvent.click(screen.getByRole('button', { name: /Policy/i }));
  await waitFor(() => {
    expect(screen.getByRole('button', { name: label })).toBeInTheDocument();
  });
  fireEvent.click(screen.getByRole('button', { name: label }));
}

describe('AdminConsole', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMocks();
  });

  it('renders agent list after loading', async () => {
    mockGovernance.listAgents = vi.fn().mockResolvedValue([
      { id: 'a1', name: 'DocExtractor', type: 'llm', version: '2.1.0', status: 'active', capabilities: [{ name: 'extract', autonomy: 'supervised' }], confidenceFloor: 0.92 },
      { id: 'a2', name: 'RulesEngine', type: 'rules-engine', version: '1.0.0', status: 'active', capabilities: [{ name: 'evaluate', autonomy: 'autonomous' }] },
    ]);

    renderWithContext(<AdminConsole />);

    await waitFor(() => {
      expect(screen.getAllByText(/DocExtractor/i).length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText(/RulesEngine/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/92%/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/extract/i).length).toBeGreaterThan(0);
  });

  it('switches between persona tabs', async () => {
    mockGovernance.listAgents = vi.fn().mockResolvedValue([
      { id: 'a1', name: 'Agent 1', type: 'llm', version: '1.0', status: 'active', capabilities: [] },
    ]);

    renderWithContext(<AdminConsole />);

    await waitFor(() => {
      expect(screen.getAllByText(/Agent 1/i).length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getByRole('button', { name: /Policy/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Constraints/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Ops/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Calendar/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /IT Admin/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Agents/i })).toBeInTheDocument();
    });
  });

  it('shows agent registration modal when button clicked', async () => {
    renderWithContext(<AdminConsole />);

    await waitFor(() => {
      expect(screen.getByText(/Registered AI Agents/i)).toBeInTheDocument();
    });

    const registerButtons = screen.getAllByText(/Register New Agent/i);
    fireEvent.click(registerButtons[0]);

    await waitFor(() => {
      expect(screen.getAllByText(/Agent Name/i).length).toBeGreaterThan(1);
    });
    expect(screen.getAllByText(/Register Agent/i).length).toBeGreaterThan(0);
  });

  it('allows revoking a delegation', async () => {
    mockGovernance.listDelegations = vi.fn()
      .mockResolvedValueOnce([{ id: 'd1', delegator: 'John Doe', delegate: 'Jane Smith', status: 'active' as const, scope: 'full', authority: 'Administrative', legalInstrument: 'I', startDate: '2026-01-01', endDate: '2026-12-31' }])
      .mockResolvedValueOnce([]);
    mockGovernance.revokeDelegation = vi.fn().mockResolvedValue(undefined);

    renderWithContext(<AdminConsole />);

    await waitFor(() => {
      expect(screen.getByText(/System Administration/i)).toBeInTheDocument();
    });

    await switchToPolicyTab(/Delegations/i);

    await waitFor(() => {
      expect(screen.getByText(/John Doe/i)).toBeInTheDocument();
    });

    const revokeBtn = screen.getByTitle(/Revoke Delegation/i);
    fireEvent.click(revokeBtn);

    await waitFor(() => {
      expect(mockGovernance.revokeDelegation).toHaveBeenCalledWith('https://agency.gov/workflows/benefits-adjudication', 'd1');
    });
  });

  it('renders deontic constraints panel with data', async () => {
    mockGovernance.listDeonticConstraints = vi.fn().mockResolvedValue([
      { kind: 'prohibition' as const, id: 'noFinalDenial', summary: 'output.eligible = false and ...', onViolation: 'escalateToHuman' },
      { kind: 'permission' as const, id: 'outputScope', summary: 'Allowed fields: eligible, reason' },
    ]);

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });
    await switchToPolicyTab(/Constraints/i);

    await waitFor(() => {
      expect(screen.getByText(/noFinalDenial/i)).toBeInTheDocument();
    });
    expect(screen.getAllByText(/prohibition/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/permission/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/escalateToHuman/i)).toBeInTheDocument();
  });

  it('renders quality controls panel with data', async () => {
    mockGovernance.getQualityControls = vi.fn().mockResolvedValue({
      reviewSampling: { rate: 0.15, method: 'random', scope: 'workflow' },
      separationOfDuties: { scope: 'sameInstance', excludeRoles: ['intakeWorker'] },
      overrideAuthority: { requireStructuredRationale: true, requireAuthorityVerification: true, requireSupportingEvidence: false },
    });

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });
    await switchToPolicyTab(/Quality/i);

    await waitFor(() => {
      expect(screen.getByText(/15%/)).toBeInTheDocument();
    });
    expect(screen.getByText(/intakeWorker/i)).toBeInTheDocument();
  });

  it('renders pipeline viewer with stages', async () => {
    mockGovernance.listPipelines = vi.fn().mockResolvedValue([
      { id: 'income-verification', stages: [{ id: 'validate-contract', type: 'contract-validation', description: 'Validate income data' }, { id: 'arithmetic-check', type: 'assertion-gate', description: 'Verify arithmetic' }], description: 'Income verification pipeline' },
    ]);

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });
    await switchToPolicyTab(/Pipelines/i);

    await waitFor(() => {
      expect(screen.getByText(/income-verification/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/Validate income data/i)).toBeInTheDocument();
    expect(screen.getByText(/assertion-gate/i)).toBeInTheDocument();
  });

  it('renders equity guardrails panel with categories', async () => {
    mockGovernance.getEquityConfig = vi.fn().mockResolvedValue({
      protectedCategories: [
        { id: 'geographicRegion', groupByPath: 'caseFile.application.geographicRegion', groups: ['northeast', 'southeast', 'west'] },
      ],
      disparityMethods: [{ id: 'approvalRateDifference', method: 'rateDifference' }],
      remediationTriggers: [{ condition: 'disparity > 0.15', action: 'review', notifyRoles: ['equityOfficer'] }],
    });

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });
    await switchToPolicyTab(/Equity/i);

    await waitFor(() => {
      expect(screen.getAllByText(/geographicRegion/i).length).toBeGreaterThan(0);
    });
    expect(screen.getByText(/northeast/i)).toBeInTheDocument();
    expect(screen.getByText(/disparity > 0\.15/i)).toBeInTheDocument();
  });

  it('renders health panel with service status', async () => {
    mockGovernance.getHealthStatus = vi.fn().mockResolvedValue([
      { id: 'svc-1', name: 'Case Engine', status: 'healthy', latency: '45ms', errorRate: '0.1%', lastCheck: '2026-04-16T12:00:00Z' },
      { id: 'svc-2', name: 'AI Gateway', status: 'degraded', latency: '320ms', errorRate: '2.4%', lastCheck: '2026-04-16T12:00:00Z' },
    ]);

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });

    fireEvent.click(screen.getByRole('button', { name: /IT Admin/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Health/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Health/i }));

    await waitFor(() => {
      expect(screen.getByText(/Case Engine/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/AI Gateway/i)).toBeInTheDocument();
    expect(screen.getAllByText(/ms/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/healthy/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/degraded/i).length).toBeGreaterThan(0);
  });

  it('renders calendar panel with events', async () => {
    mockGovernance.listCalendarEvents = vi.fn().mockResolvedValue([
      { id: 'evt-1', name: 'Independence Day', date: '2026-07-04', type: 'federal' as const, impactsDeadlines: true },
    ]);

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });

    fireEvent.click(screen.getByRole('button', { name: /Ops/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Calendar/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Calendar/i }));

    await waitFor(() => {
      expect(screen.getByText(/Independence Day/i)).toBeInTheDocument();
    });
  });

  it('renders regulatory versions timeline', async () => {
    mockGovernance.listPolicyVersions = vi.fn().mockResolvedValue([
      { id: 'v1', label: 'FY2026-Q1', effectiveDate: '2026-01-01', parameterCount: 12, status: 'active' as const },
      { id: 'v2', label: 'FY2026-Q2', effectiveDate: '2026-04-01', parameterCount: 15, status: 'upcoming' as const },
    ]);

    renderWithContext(<AdminConsole />);
    await waitFor(() => { expect(screen.getByText(/System Administration/i)).toBeInTheDocument(); });
    await switchToPolicyTab(/Regulatory/i);

    await waitFor(() => {
      expect(screen.getAllByText(/FY2026-Q1/i).length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText(/FY2026-Q2/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Currently Active/i).length).toBeGreaterThan(0);
  });
});
