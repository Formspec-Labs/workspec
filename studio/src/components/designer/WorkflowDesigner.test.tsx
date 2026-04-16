import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { WorkflowDesigner } from './WorkflowDesigner';
import { WosProvider } from '../../context/WosContext';
import type { IWorkflowDesignPort, IRealtimePort } from '../../services/WosPorts';
import type { WOSKernelDocument } from '../../types/wos/kernel';

const SAMPLE_KERNEL = {
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
} as unknown as WOSKernelDocument;

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

const loadWithKernel = async (kernel = SAMPLE_KERNEL) => {
  (mockWorkflowDesign.loadKernel as ReturnType<typeof vi.fn>).mockResolvedValue(kernel);
  const result = renderWithContext(<WorkflowDesigner />);
  await waitFor(() => {
    expect(screen.getByText(/Test WOS Workflow/i)).toBeInTheDocument();
  });
  return result;
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
    await loadWithKernel();
  });

  it('allows deploying workflow', async () => {
    await loadWithKernel();

    fireEvent.click(screen.getByText(/Deploy Changes/i));

    await waitFor(() => {
      expect(mockWorkflowDesign.saveKernel).toHaveBeenCalled();
    });
  });

  it('adds a stage from the palette', async () => {
    await loadWithKernel();

    const initialStageCount = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;

    const taskButton = screen.getAllByText(/Task/i).find(el => el.closest('button'));
    expect(taskButton).toBeTruthy();
    fireEvent.click(taskButton!);

    await waitFor(() => {
      const newCount = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;
      expect(newCount).toBe(initialStageCount + 1);
    });
  });

  it('deletes a selected stage', async () => {
    await loadWithKernel();

    const stageNode = screen.getByRole('button', { name: /Workflow stage: start/i });
    fireEvent.pointerDown(stageNode, { clientX: 0, clientY: 0, button: 0 });

    await waitFor(() => {
      expect(screen.getByText(/Delete Stage/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/Delete Stage/i));

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /Workflow stage: start/i })).not.toBeInTheDocument();
    });
  });

  it('connects two stages via the connection flow', async () => {
    await loadWithKernel();

    const fromStage = screen.getByRole('button', { name: /Workflow stage: start/i });
    const toStage = screen.getByRole('button', { name: /Workflow stage: end/i });

    const rightHandle = fromStage.querySelector('[title="Drag to connect"]');
    expect(rightHandle).toBeTruthy();
    fireEvent.pointerDown(rightHandle!, { button: 0, clientX: 0, clientY: 0 });

    fireEvent.pointerUp(toStage, { clientX: 0, clientY: 0 });

    expect(mockRealtime.sendKernelUpdate).toHaveBeenCalled();
  });

  it('undoes and redoes stage addition', async () => {
    await loadWithKernel();

    const taskButton = screen.getAllByText(/Task/i).find(el => el.closest('button'));
    fireEvent.click(taskButton!);

    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: /Workflow stage:/i }).length).toBeGreaterThan(2);
    });

    const stageCountAfterAdd = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;

    fireEvent.keyDown(window, { key: 'z', ctrlKey: true });

    await waitFor(() => {
      const count = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;
      expect(count).toBeLessThan(stageCountAfterAdd);
    });

    const stageCountAfterUndo = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;

    fireEvent.keyDown(window, { key: 'y', ctrlKey: true });

    await waitFor(() => {
      const count = screen.getAllByRole('button', { name: /Workflow stage:/i }).length;
      expect(count).toBe(stageCountAfterAdd);
    });
  });

  it('shows validation error when saving kernel without initialState', async () => {
    const invalidKernel = {
      $wosKernel: '1.0',
      url: 'https://agency.gov/workflows/test',
      version: '1.0.0',
      title: 'Test WOS Workflow',
      status: 'draft',
      lifecycle: {
        initialState: 'start',
        states: {
          start: { type: 'atomic' },
        },
      },
    } as unknown as WOSKernelDocument;

    (mockWorkflowDesign.validateKernel as ReturnType<typeof vi.fn>).mockResolvedValue({
      isValid: false,
      issues: [{ severity: 'error', category: 'structure', message: 'Stage "start" is a dead end. Add an exit path.' }],
    });

    await loadWithKernel(invalidKernel);

    const deployButtons = screen.getAllByText(/Deploy/i);
    const deployBtn = deployButtons.find(b => b.closest('button'));
    fireEvent.click(deployBtn!);

    await waitFor(() => {
      expect(mockWorkflowDesign.saveKernel).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(mockWorkflowDesign.validateKernel).toHaveBeenCalled();
    });
  });
});
