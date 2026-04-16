import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { WorkflowDesigner } from './WorkflowDesigner';
import { WosProvider } from '../../context/WosContext';
import type { IWorkflowDesignPort, IRealtimePort } from '../../services/WosPorts';
import type { WOSKernelDocument } from '../../types/wos/kernel';

const mockWorkflowDesign: IWorkflowDesignPort = {
  listWorkflows: vi.fn().mockResolvedValue([]),
  loadKernel: vi.fn().mockResolvedValue(null),
  saveKernel: vi.fn().mockResolvedValue(undefined),
  validateKernel: vi.fn().mockResolvedValue({ isValid: true, issues: [] }),
};

const mockRealtime: IRealtimePort = {
  connect: vi.fn(),
  disconnect: vi.fn(),
  onKernelInit: vi.fn(),
  onKernelChanged: vi.fn(),
  onCollaboratorsUpdate: vi.fn(),
  onCursorUpdate: vi.fn(),
  sendCursorMove: vi.fn(),
  sendKernelUpdate: vi.fn(),
};

const renderWithContext = (ui: React.ReactElement) => {
  return render(
    <WosProvider ports={{ workflowDesign: mockWorkflowDesign, realtime: mockRealtime }}>
      {ui}
    </WosProvider>
  );
};

describe('WorkflowDesigner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state initially', () => {
    (mockWorkflowDesign.loadKernel as ReturnType<typeof vi.fn>).mockReturnValue(new Promise(() => {}));
    renderWithContext(<WorkflowDesigner />);
    expect(screen.getByText(/Loading designer/i)).toBeInTheDocument();
  });

  it('renders kernel workflow when loaded', async () => {
    (mockWorkflowDesign.loadKernel as ReturnType<typeof vi.fn>).mockResolvedValue({
      $wosKernel: '1.0',
      url: 'https://agency.gov/workflows/test',
      version: '1.0.0',
      title: 'Test WOS Workflow',
      status: 'active',
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'atomic', transitions: [{ event: 'go', target: 'end' }] },
          end: { type: 'final' },
        },
      },
    } as unknown as WOSKernelDocument);

    renderWithContext(<WorkflowDesigner />);

    await waitFor(() => {
      expect(screen.getByText(/Test WOS Workflow/i)).toBeInTheDocument();
    });
  });

  it('allows deploying workflow', async () => {
    (mockWorkflowDesign.loadKernel as ReturnType<typeof vi.fn>).mockResolvedValue({
      $wosKernel: '1.0',
      url: 'https://agency.gov/workflows/test',
      version: '1.0.0',
      title: 'Deploy Test',
      status: 'active',
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'atomic', transitions: [{ event: 'go', target: 'end' }] },
          end: { type: 'final' },
        },
      },
    } as unknown as WOSKernelDocument);

    renderWithContext(<WorkflowDesigner />);

    await waitFor(() => {
      expect(screen.getByText(/Deploy Test/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/Deploy Changes/i));

    await waitFor(() => {
      expect(mockWorkflowDesign.saveKernel).toHaveBeenCalled();
    });
  });
});
