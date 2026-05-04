import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ProcessDashboard } from './ProcessDashboard';
import { WosProvider } from '../../context/WosContext';
import type { IDashboardPort } from '../../services/WosPorts';

vi.mock('recharts', async () => {
  const original = await vi.importActual('recharts');
  return {
    ...original,
    ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
      <div style={{ width: '100%', height: '100%' }}>{children}</div>
    ),
    BarChart: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    Bar: () => <div />,
    XAxis: () => <div />,
    YAxis: () => <div />,
    CartesianGrid: () => <div />,
    Tooltip: () => <div />,
    Legend: () => <div />,
    LineChart: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    Line: () => <div />,
    AreaChart: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
    Area: () => <div />,
  };
});

const mockDashboard: IDashboardPort = {
  getMetrics: vi.fn(),
  getStageMetrics: vi.fn().mockResolvedValue([]),
  getAlerts: vi.fn().mockResolvedValue([]),
  getDriftData: vi.fn().mockResolvedValue([]),
  getPipelineData: vi.fn().mockResolvedValue([]),
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ dashboard: mockDashboard }}>
      {ui}
    </WosProvider>
  );
};

describe('ProcessDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders dashboard with metrics', async () => {
    mockDashboard.getMetrics = vi.fn().mockResolvedValue({
      activeInstances: 450,
      activeInstancesTrend: -5,
      completed7d: 1250,
      completed7dTrend: 8,
      slaCompliance: 94.2,
      slaComplianceTrend: 1.2,
      avgProcessingTimeDays: 3.5,
      avgProcessingTimeTrend: -0.5,
      aiAcceptanceRate: 88.5,
      aiAcceptanceRateTrend: 2.1
    });

    renderWithContext(<ProcessDashboard />);

    await waitFor(() => {
      expect(screen.getByText(/Operations Dashboard/i)).toBeInTheDocument();
      expect(screen.getByText(/1,250/i)).toBeInTheDocument();
      expect(screen.getByText(/94.2%/i)).toBeInTheDocument();
    });
  });

  it('renders alerts panel', async () => {
    mockDashboard.getMetrics = vi.fn().mockResolvedValue({
      activeInstances: 10, activeInstancesTrend: 0, completed7d: 100, completed7dTrend: 0,
      slaCompliance: 90, slaComplianceTrend: 0, avgProcessingTimeDays: 1, avgProcessingTimeTrend: 0,
      aiAcceptanceRate: 95, aiAcceptanceRateTrend: 0
    });
    mockDashboard.getAlerts = vi.fn().mockResolvedValue([
      { id: '1', type: 'sla', severity: 'critical', title: 'SLA Breach Warning', description: 'Case #123 is overdue', timeAgo: '2m ago' }
    ]);

    renderWithContext(<ProcessDashboard />);

    await waitFor(() => {
      expect(screen.getByText(/SLA Breach Warning/i)).toBeInTheDocument();
      expect(screen.getByText(/Case #123 is overdue/i)).toBeInTheDocument();
    });
  });
});
