import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AuditViewer } from './AuditViewer';
import { WosProvider } from '../../context/WosContext';
import type { ICaseViewerPort } from '../../services/WosPorts';

const mockProvenance = [
  {
    id: 'prov-test-1',
    instanceId: 'urn:wos:instance:test',
    timestamp: '2026-04-10T10:00:00Z',
    tier: 'facts' as const,
    actor: { id: 'auditor-1', type: 'human' as const, name: 'John Auditor' },
    event: 'verificationComplete',
    sourceState: 'intake',
    targetState: 'review',
    facts: {
      inputs: { income: 50000 },
      outputs: { eligible: true },
      metadata: { policyVersion: 'v1' },
    },
    reasoning: {
      rulesApplied: ['Rule A'],
      criteriaChecked: [{ label: 'Income Check', passed: true }],
      explanation: 'Meets income requirements.',
      sourceAuthority: 'regulation',
    },
    integrity: { hash: 'sha256:abc', previousHash: 'sha256:prev' },
  },
];

const mockCaseViewer: ICaseViewerPort = {
  getInstance: vi.fn().mockResolvedValue(null),
  getProvenance: vi.fn().mockResolvedValue(mockProvenance),
  getTimeline: vi.fn().mockResolvedValue([]),
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ caseViewer: mockCaseViewer }}>
      {ui}
    </WosProvider>
  );
};

describe('AuditViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (mockCaseViewer.getProvenance as ReturnType<typeof vi.fn>).mockResolvedValue(mockProvenance);
  });

  it('renders provenance records and allows selection', async () => {
    renderWithContext(<AuditViewer />);

    await waitFor(() => {
      expect(screen.getByText(/verificationComplete/i)).toBeInTheDocument();
      expect(screen.getByText(/John Auditor/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/John Auditor/i));

    await waitFor(() => {
      expect(screen.getByText(/Meets income requirements/i)).toBeInTheDocument();
    });
  });

  it('allows filtering by tier', async () => {
    renderWithContext(<AuditViewer />);

    await waitFor(() => {
      expect(screen.getByText(/verificationComplete/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Reasoning/i }));

    await waitFor(() => {
      expect(screen.queryByText(/verificationComplete/i)).not.toBeInTheDocument();
    });
  });

  it('shows integrity information for selected record', async () => {
    renderWithContext(<AuditViewer />);

    await waitFor(() => {
      fireEvent.click(screen.getByText(/John Auditor/i));
    });

    await waitFor(() => {
      expect(screen.getByText(/sha256:abc/i)).toBeInTheDocument();
    });
  });
});
