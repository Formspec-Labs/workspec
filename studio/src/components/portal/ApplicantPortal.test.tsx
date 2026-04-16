import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ApplicantPortal } from './ApplicantPortal';
import { WosProvider } from '../../context/WosContext';
import type { IApplicantPort } from '../../services/WosPorts';

const mockApplicant: IApplicantPort = {
  getDetermination: vi.fn(),
  submitAppeal: vi.fn(),
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ applicant: mockApplicant }}>
      {ui}
    </WosProvider>
  );
};

const mockDetermination = {
  instanceId: 'CASE-123',
  programName: 'Benefits Adjudication',
  decision: 'denied' as const,
  dateIssued: '2026-04-01T00:00:00Z',
  deadlineDate: '2026-04-15T00:00:00Z',
  benefitsContinue: true,
  summary: 'Your application was denied because we did not receive the required documents.',
  evidenceConsidered: [],
  rulesApplied: [],
  aiDisclosure: { wasUsed: false },
  counterfactuals: { positive: [], negative: [] },
  appealStatus: 'not-filed' as const,
  milestones: [],
};

describe('ApplicantPortal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state initially', () => {
    (mockApplicant.getDetermination as ReturnType<typeof vi.fn>).mockReturnValue(new Promise(() => {}));
    const { container } = renderWithContext(<ApplicantPortal />);
    expect(container.querySelector('.animate-spin')).toBeInTheDocument();
  });

  it('renders determination details when loaded', async () => {
    (mockApplicant.getDetermination as ReturnType<typeof vi.fn>).mockResolvedValue(mockDetermination);

    renderWithContext(<ApplicantPortal caseId="CASE-123" />);

    await waitFor(() => {
      expect(screen.getByText(/Notice of Determination/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/CASE-123/i)).toBeInTheDocument();
    expect(screen.getByText(/Action Required: Appeal Deadline Approaching/i)).toBeInTheDocument();
  });

  it('allows starting an appeal', async () => {
    (mockApplicant.getDetermination as ReturnType<typeof vi.fn>).mockResolvedValue(mockDetermination);

    renderWithContext(<ApplicantPortal caseId="CASE-123" />);

    await waitFor(() => {
      expect(screen.getByText(/Start My Appeal/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/Start My Appeal/i));

    expect(screen.getByText(/File an Appeal/i)).toBeInTheDocument();
  });
});
