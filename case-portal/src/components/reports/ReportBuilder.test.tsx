import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ReportBuilder } from './ReportBuilder';
import { WosProvider } from '../../context/WosContext';

vi.mock('recharts', async () => {
  const original = await vi.importActual('recharts');
  return {
    ...original,
    ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
      <div style={{ width: '100%', height: '100%' }}>{children}</div>
    ),
  };
});

const renderWithContext = (ui: React.ReactElement) => {
  return render(<WosProvider>{ui}</WosProvider>);
};

describe('ReportBuilder', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders templates initially', async () => {
    renderWithContext(<ReportBuilder />);

    await waitFor(() => {
      expect(screen.getByText(/Decision Drift Analysis/i)).toBeInTheDocument();
    });
  });

  it('allows selecting a template and generating a report', async () => {
    renderWithContext(<ReportBuilder />);

    await waitFor(() => {
      expect(screen.getAllByText(/Decision Drift Analysis/i).length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getAllByText(/Decision Drift Analysis/i)[0]);
    fireEvent.click(screen.getByText(/Generate Report/i));

    await waitFor(() => {
      expect(screen.getAllByText(/Decision Drift Analysis/i).length).toBeGreaterThan(1);
    });
  });

  it('allows switching to custom builder', async () => {
    renderWithContext(<ReportBuilder />);

    await waitFor(() => {
      expect(screen.getByText(/Templates/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/Custom/i));

    await waitFor(() => {
      expect(screen.getByText(/Select Metrics/i)).toBeInTheDocument();
    });
  });
});
