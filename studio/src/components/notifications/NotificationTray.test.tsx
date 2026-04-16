import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { NotificationTray } from './NotificationTray';
import { WosProvider } from '../../context/WosContext';

const renderWithContext = (ui: React.ReactElement) => {
  return render(<WosProvider>{ui}</WosProvider>);
};

describe('NotificationTray', () => {
  const mockOnClose = vi.fn();
  const mockOnNavigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders notifications and counts unread', async () => {
    renderWithContext(<NotificationTray onClose={mockOnClose} onNavigate={mockOnNavigate} />);

    await waitFor(() => {
      expect(screen.getByText(/New case assigned/i)).toBeInTheDocument();
      expect(screen.getByText(/SLA approaching/i)).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument();
    });
  });

  it('filters unread notifications', async () => {
    renderWithContext(<NotificationTray onClose={mockOnClose} onNavigate={mockOnNavigate} />);

    await waitFor(() => {
      expect(screen.getByText(/SLA approaching/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText(/Unread/i));

    await waitFor(() => {
      expect(screen.queryByText(/SLA breached/i)).not.toBeInTheDocument();
      expect(screen.queryByText(/System update/i)).not.toBeInTheDocument();
      expect(screen.getByText(/New case assigned/i)).toBeInTheDocument();
    });
  });

  it('marks notification as read and navigates', async () => {
    renderWithContext(<NotificationTray onClose={mockOnClose} onNavigate={mockOnNavigate} />);

    await waitFor(() => {
      fireEvent.click(screen.getByText(/New case assigned/i));
    });

    expect(mockOnNavigate).toHaveBeenCalledWith({ type: 'task', id: 'task-1' });
  });

  it('marks all as read', async () => {
    renderWithContext(<NotificationTray onClose={mockOnClose} onNavigate={mockOnNavigate} />);

    await waitFor(() => {
      fireEvent.click(screen.getByText(/Mark all read/i));
    });

    await waitFor(() => {
      expect(screen.queryByText('2')).not.toBeInTheDocument();
    });
  });
});
