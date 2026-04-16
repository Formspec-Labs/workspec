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
  getVerificationReport: vi.fn(),
  getEquityConfig: vi.fn(),
  listDelegations: vi.fn(),
  revokeDelegation: vi.fn(),
  listPolicyVersions: vi.fn(),
  listCalendarEvents: vi.fn(),
  getHealthStatus: vi.fn(),
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ governance: mockGovernance }}>
      {ui}
    </WosProvider>
  );
};

describe('AdminConsole', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGovernance.listAgents = vi.fn().mockResolvedValue([]);
    mockGovernance.listDeonticConstraints = vi.fn().mockResolvedValue([]);
    mockGovernance.getQualityControls = vi.fn().mockResolvedValue(null);
    mockGovernance.listPipelines = vi.fn().mockResolvedValue([]);
    mockGovernance.getVerificationReport = vi.fn().mockResolvedValue(null);
    mockGovernance.getEquityConfig = vi.fn().mockResolvedValue(null);
    mockGovernance.listDelegations = vi.fn().mockResolvedValue([]);
    mockGovernance.listPolicyVersions = vi.fn().mockResolvedValue([]);
    mockGovernance.listCalendarEvents = vi.fn().mockResolvedValue([]);
    mockGovernance.getHealthStatus = vi.fn().mockResolvedValue([]);
  });

  it('renders admin console and switches tabs', async () => {
    mockGovernance.listAgents = vi.fn().mockResolvedValue([
      { id: 'a1', name: 'Agent 1', type: 'llm', version: '1.0', status: 'active', capabilities: [] }
    ]);

    renderWithContext(<AdminConsole />);

    await waitFor(() => {
      expect(screen.getAllByText(/Agent 1/i).length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getByRole('button', { name: /Policy/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Delegations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Delegations/i }));

    fireEvent.click(screen.getByRole('button', { name: /Regulatory/i }));

    fireEvent.click(screen.getByRole('button', { name: /Ops/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Calendar/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Calendar/i }));

    fireEvent.click(screen.getByRole('button', { name: /IT Admin/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Health/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Health/i }));
  });

  it('allows revoking a delegation', async () => {
    mockGovernance.listDelegations = vi.fn()
      .mockResolvedValueOnce([{ id: 'd1', delegator: 'John Doe', delegate: 'Jane Smith', status: 'active' as const, scope: 'full', authority: 'Administrative', legalInstrument: 'I', startDate: '2026-01-01', endDate: '2026-12-31' }])
      .mockResolvedValueOnce([]);
    mockGovernance.revokeDelegation = vi.fn().mockResolvedValue(undefined);

    renderWithContext(<AdminConsole />);

    fireEvent.click(screen.getByRole('button', { name: /Policy/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Delegations/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Delegations/i }));

    await waitFor(() => {
      expect(screen.getByText(/John Doe/i)).toBeInTheDocument();
    });

    const revokeBtn = screen.getByTitle(/Revoke Delegation/i);
    fireEvent.click(revokeBtn);

    await waitFor(() => {
      expect(mockGovernance.revokeDelegation).toHaveBeenCalledWith('https://agency.gov/workflows/benefits-adjudication', 'd1');
    });
  });

  async function switchToPolicyTab(label: string | RegExp) {
    fireEvent.click(screen.getByRole('button', { name: /Policy/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: label })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: label }));
  }

  it('renders deontic constraints panel with data', async () => {
    mockGovernance.listDeonticConstraints = vi.fn().mockResolvedValue([
      { kind: 'prohibition' as const, id: 'noFinalDenial', summary: 'output.eligible = false and ...', onViolation: 'escalateToHuman' },
      { kind: 'permission' as const, id: 'outputScope', summary: 'Allowed fields: eligible, reason' },
    ]);

    renderWithContext(<AdminConsole />);
    await switchToPolicyTab(/Constraints/i);

    await waitFor(() => {
      expect(screen.getByText(/noFinalDenial/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/prohibition/i)).toBeInTheDocument();
    expect(screen.getByText(/permission/i)).toBeInTheDocument();
    expect(screen.getByText(/escalateToHuman/i)).toBeInTheDocument();
  });

  it('renders quality controls panel with data', async () => {
    mockGovernance.getQualityControls = vi.fn().mockResolvedValue({
      reviewSampling: { rate: 0.15, method: 'random', scope: 'workflow' },
      separationOfDuties: { scope: 'sameInstance', excludeRoles: ['intakeWorker'] },
      overrideAuthority: { requireStructuredRationale: true, requireAuthorityVerification: true, requireSupportingEvidence: false },
    });

    renderWithContext(<AdminConsole />);
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
    await switchToPolicyTab(/Pipelines/i);

    await waitFor(() => {
      expect(screen.getByText(/income-verification/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/Validate income data/i)).toBeInTheDocument();
    expect(screen.getByText(/assertion-gate/i)).toBeInTheDocument();
  });

  it('renders verification report panel', async () => {
    mockGovernance.getVerificationReport = vi.fn().mockResolvedValue({
      solver: { name: 'Z3', version: '4.13.0' },
      results: [{ constraintRef: 'noFinalDenial', result: 'proven-safe', solverTimeMs: 142 }],
      summary: { totalConstraints: 1, provenSafe: 1, provenUnsafe: 0, inconclusive: 0, totalSolverTimeMs: 142 },
    });

    renderWithContext(<AdminConsole />);
    fireEvent.click(screen.getByRole('button', { name: /IT Admin/i }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Verification/i })).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /Verification/i }));

    await waitFor(() => {
      expect(screen.getByText(/noFinalDenial/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/proven-safe/i)).toBeInTheDocument();
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
    await switchToPolicyTab(/Equity/i);

    await waitFor(() => {
      expect(screen.getByText(/geographicRegion/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/northeast/i)).toBeInTheDocument();
    expect(screen.getByText(/review/i)).toBeInTheDocument();
  });
});
