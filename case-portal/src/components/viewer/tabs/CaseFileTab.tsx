import React, { useState, useEffect } from 'react';
import {
  Clock,
  Layers,
  ListChecks,
  Timer,
  Shield,
  Users,
  AlertTriangle,
  ChevronRight,
  Activity,
} from 'lucide-react';
import { useCaseViewer } from '../../../context/WosContext';
import type { CaseInstanceView } from '../../../services/WosBackend';

interface CaseFileTabProps {
  caseId: string;
}

function formatValue(value: unknown): string {
  if (value === null) return 'null';
  if (value === undefined) return '—';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') return value.toLocaleString();
  if (typeof value === 'string') return value;
  return String(value);
}

function StateValueRenderer({ value, depth = 0 }: { value: unknown; depth?: number }) {
  if (value === null || value === undefined) {
    return <span className="text-gray-400 italic">{value === null ? 'null' : '—'}</span>;
  }

  if (typeof value !== 'object') {
    return <span className="text-gray-900">{formatValue(value)}</span>;
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return <span className="text-gray-400 italic">empty</span>;
    }
    return (
      <div className="space-y-1">
        {value.map((item, i) => (
          <div key={i}>
            <StateValueRenderer value={item} depth={depth + 1} />
          </div>
        ))}
      </div>
    );
  }

  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) {
    return <span className="text-gray-400 italic">empty</span>;
  }

  return (
    <div className={depth > 0 ? 'pl-4 border-l-2 border-gray-100 mt-1' : ''}>
      <table className="w-full">
        <tbody>
          {entries.map(([key, val]) => (
            <tr key={key} className={depth === 0 ? 'border-b border-gray-100 last:border-b-0' : ''}>
              <td className={`py-1.5 ${depth === 0 ? 'px-4' : 'pr-4'} text-sm font-medium text-gray-600 w-1/3 align-top`}>
                {key}
              </td>
              <td className={`py-1.5 ${depth === 0 ? 'px-4' : ''} text-sm align-top`}>
                <StateValueRenderer value={val} depth={depth + 1} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    active: 'bg-emerald-50 text-emerald-700 border-emerald-100',
    suspended: 'bg-amber-50 text-amber-700 border-amber-100',
    completed: 'bg-blue-50 text-blue-700 border-blue-100',
    terminated: 'bg-red-50 text-red-700 border-red-100',
    migrating: 'bg-purple-50 text-purple-700 border-purple-100',
  };
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium border ${styles[status] ?? 'bg-gray-50 text-gray-700 border-gray-200'}`}>
      <Activity className="w-3 h-3" />
      {status}
    </span>
  );
}

function TaskStatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    created: 'bg-gray-50 text-gray-700 border-gray-200',
    assigned: 'bg-blue-50 text-blue-700 border-blue-100',
    claimed: 'bg-indigo-50 text-indigo-700 border-indigo-100',
    delegated: 'bg-purple-50 text-purple-700 border-purple-100',
    escalated: 'bg-red-50 text-red-700 border-red-100',
  };
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium border ${styles[status] ?? 'bg-gray-50 text-gray-700 border-gray-200'}`}>
      {status}
    </span>
  );
}

export function CaseFileTab({ caseId }: CaseFileTabProps) {
  const caseViewer = useCaseViewer();
  const [instance, setInstance] = useState<CaseInstanceView | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    setIsLoading(true);
    caseViewer.getInstance(caseId).then(result => {
      setInstance(result);
      setIsLoading(false);
    });
  }, [caseViewer, caseId]);

  if (isLoading || !instance) {
    return (
      <div className="p-12 flex items-center justify-center">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto p-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">Case File</h2>
          <p className="text-sm text-gray-500 mt-1 font-mono">{instance.instanceId}</p>
        </div>
        <div className="flex items-center gap-3">
          <StatusBadge status={instance.status} />
          <span className="inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium bg-orange-50 text-orange-700 border border-orange-100">
            <AlertTriangle className="w-3 h-3" />
            {instance.impactLevel}
          </span>
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
          <Layers className="w-4 h-4 text-gray-500" />
          <h3 className="text-sm font-semibold text-gray-900">Active Configuration</h3>
        </div>
        <div className="p-4 flex flex-wrap gap-2">
          {instance.configuration.map((state, i) => (
            <span key={i} className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-mono bg-blue-50 text-blue-800 border border-blue-100">
              {state}
            </span>
          ))}
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
          <ListChecks className="w-4 h-4 text-gray-500" />
          <h3 className="text-sm font-semibold text-gray-900">Case State</h3>
        </div>
        <StateValueRenderer value={instance.caseState} />
      </div>

      {instance.activeTasks.length > 0 && (
        <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
            <Users className="w-4 h-4 text-gray-500" />
            <h3 className="text-sm font-semibold text-gray-900">Active Tasks ({instance.activeTasks.length})</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 bg-gray-50/50">
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Task ID</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Task Ref</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Status</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Assigned Actor</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Deadline</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {instance.activeTasks.map(task => (
                  <tr key={task.taskId} className="hover:bg-gray-50 transition-colors">
                    <td className="px-4 py-3 font-mono text-xs text-gray-700">{task.taskId}</td>
                    <td className="px-4 py-3 text-gray-900">{task.taskRef}</td>
                    <td className="px-4 py-3">
                      <TaskStatusBadge status={task.status} />
                    </td>
                    <td className="px-4 py-3 text-gray-700">{task.assignedActor ?? '—'}</td>
                    <td className="px-4 py-3 text-gray-600">
                      {task.deadline ? (
                        <span className="flex items-center gap-1">
                          <Clock className="w-3 h-3" />
                          {new Date(task.deadline).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                        </span>
                      ) : '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {instance.timers.length > 0 && (
        <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
            <Timer className="w-4 h-4 text-gray-500" />
            <h3 className="text-sm font-semibold text-gray-900">Timers ({instance.timers.length})</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 bg-gray-50/50">
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Timer ID</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Deadline</th>
                  <th className="px-4 py-3 text-left font-medium text-gray-600">Event</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {instance.timers.map(timer => (
                  <tr key={timer.timerId} className="hover:bg-gray-50 transition-colors">
                    <td className="px-4 py-3 font-mono text-xs text-gray-700">{timer.timerId}</td>
                    <td className="px-4 py-3 text-gray-600">
                      <span className="flex items-center gap-1">
                        <Clock className="w-3 h-3" />
                        {new Date(timer.deadline).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                      </span>
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-gray-900">{timer.event}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {instance.governanceState && (
        <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
            <Shield className="w-4 h-4 text-gray-500" />
            <h3 className="text-sm font-semibold text-gray-900">Governance State</h3>
          </div>
          <div className="divide-y divide-gray-200">
            {instance.governanceState.activeDelegations.length > 0 && (
              <div>
                <div className="px-6 py-3 bg-gray-50/50 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                  Active Delegations
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-gray-100">
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Delegator</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Delegate</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Scope</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Authority</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Granted</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Expires</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-50">
                      {instance.governanceState.activeDelegations.map((del, i) => (
                        <tr key={i} className="hover:bg-gray-50 transition-colors">
                          <td className="px-4 py-2 text-gray-700">{del.delegatorId}</td>
                          <td className="px-4 py-2 text-gray-900">{del.delegateId}</td>
                          <td className="px-4 py-2 font-mono text-xs text-gray-700">{del.scope}</td>
                          <td className="px-4 py-2">
                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-purple-50 text-purple-700 border border-purple-100">
                              {del.authority ?? '—'}
                            </span>
                          </td>
                          <td className="px-4 py-2 text-gray-600 text-xs">
                            {new Date(del.grantedAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                          </td>
                          <td className="px-4 py-2 text-gray-600 text-xs">
                            {del.expiresAt
                              ? new Date(del.expiresAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
                              : '—'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {instance.governanceState.activeHolds.length > 0 && (
              <div>
                <div className="px-6 py-3 bg-gray-50/50 text-xs font-semibold text-gray-500 uppercase tracking-wider">
                  Active Holds
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-gray-100">
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Hold Type</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Started</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Expected End</th>
                        <th className="px-4 py-2 text-left font-medium text-gray-600">Resume Trigger</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-50">
                      {instance.governanceState.activeHolds.map((hold, i) => (
                        <tr key={i} className="hover:bg-gray-50 transition-colors">
                          <td className="px-4 py-2">
                            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-amber-50 text-amber-700 border border-amber-100">
                              <AlertTriangle className="w-3 h-3" />
                              {hold.holdType}
                            </span>
                          </td>
                          <td className="px-4 py-2 text-gray-600 text-xs">
                            {new Date(hold.startedAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                          </td>
                          <td className="px-4 py-2 text-gray-600 text-xs">
                            {hold.expectedEnd
                              ? new Date(hold.expectedEnd).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
                              : '—'}
                          </td>
                          <td className="px-4 py-2 font-mono text-xs text-gray-700">{hold.resumeTrigger}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {instance.governanceState.activeDelegations.length === 0 &&
              instance.governanceState.activeHolds.length === 0 && (
                <div className="p-6 text-sm text-gray-500 text-center">
                  No active delegations or holds
                </div>
              )}
          </div>
        </div>
      )}
    </div>
  );
}
