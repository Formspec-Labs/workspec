import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { TaskList } from './TaskList';
import type { TaskListItem } from '../services/WosPorts';
import { WosProvider } from '../context/WosContext';

const mockTasks: TaskListItem[] = [
  {
    taskId: '1',
    instanceId: 'CASE-1',
    taskRef: 'Review Application',
    status: 'created',
    configuration: [],
    caseState: {},
    definitionTitle: 'Benefits Adjudication',
    definitionUrl: 'test',
    createdAt: '2026-04-10',
    impactLevel: 'high',
    deadline: '2026-05-15',
  },
  {
    taskId: '2',
    instanceId: 'CASE-2',
    taskRef: 'Verify Documents',
    status: 'assigned',
    configuration: [],
    caseState: {},
    definitionTitle: 'Benefits Adjudication',
    definitionUrl: 'test',
    createdAt: '2026-04-10',
    impactLevel: 'normal',
    deadline: '2026-04-20',
  }
];

const renderWithContext = (ui: React.ReactElement) => {
  return render(<WosProvider>{ui}</WosProvider>);
};

describe('TaskList', () => {
  const mockOnTaskClick = vi.fn();
  const mockSetFilters = vi.fn();
  const defaultFilters = { status: [] as string[], impactLevel: [] as string[], configuration: [] as string[] };

  it('renders tasks with instance IDs', () => {
    renderWithContext(<TaskList tasks={mockTasks} filters={defaultFilters} setFilters={mockSetFilters} onTaskClick={mockOnTaskClick} />);
    expect(screen.getAllByText(/CASE-/i).length).toBeGreaterThan(0);
  });

  it('filters tasks by search query', () => {
    renderWithContext(<TaskList tasks={mockTasks} filters={defaultFilters} setFilters={mockSetFilters} onTaskClick={mockOnTaskClick} />);
    
    const input = screen.getByPlaceholderText(/Search by case ID/i);
    fireEvent.change(input, { target: { value: 'CASE-1' } });

    expect(screen.getByText(/CASE-1/i)).toBeInTheDocument();
    expect(screen.queryByText(/CASE-2/i)).not.toBeInTheDocument();
  });

  it('shows Impact and Deadline sort buttons', () => {
    renderWithContext(<TaskList tasks={mockTasks} filters={defaultFilters} setFilters={mockSetFilters} onTaskClick={mockOnTaskClick} />);
    expect(screen.getAllByText(/Impact/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Deadline/i).length).toBeGreaterThan(0);
  });
});
