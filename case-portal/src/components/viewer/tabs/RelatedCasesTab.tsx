import React, { useState, useEffect } from 'react';
import { Network, Link2, Activity, AlertTriangle, Layers, ListChecks, Clock } from 'lucide-react';
import { useBackend } from '../../../context/WosContext';
import type { CaseInstanceView } from '../../../services/WosBackend';

interface RelatedCasesTabProps {
  caseId: string;
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

function ImpactBadge({ level }: { level: string }) {
  const styles: Record<string, string> = {
    'rights-impacting': 'bg-red-50 text-red-700 border-red-100',
    'safety-impacting': 'bg-orange-50 text-orange-700 border-orange-100',
    'operational': 'bg-gray-50 text-gray-700 border-gray-200',
    'informational': 'bg-blue-50 text-blue-700 border-blue-100',
  };
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium border ${styles[level] ?? 'bg-gray-50 text-gray-700 border-gray-200'}`}>
      <AlertTriangle className="w-3 h-3" />
      {level}
    </span>
  );
}

function extractShortId(instanceId: string): string {
  const segments = instanceId.split(':');
  return segments[segments.length - 1] ?? instanceId;
}

export function RelatedCasesTab({ caseId }: RelatedCasesTabProps) {
  const backend = useBackend();
  const [currentInstance, setCurrentInstance] = useState<CaseInstanceView | null>(null);
  const [related, setRelated] = useState<CaseInstanceView[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    setIsLoading(true);
    Promise.all([
      backend.getInstance(caseId),
      backend.listInstances(),
    ]).then(([inst, result]) => {
      setCurrentInstance(inst);
      setRelated(result.items.filter(i => i.instanceId !== caseId));
      setIsLoading(false);
    });
  }, [backend, caseId]);

  if (isLoading) {
    return (
      <div className="p-12 flex items-center justify-center">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  const grouped: Record<string, CaseInstanceView[]> = {};
  for (const inst of related) {
    const key = inst.definitionUrl;
    if (!grouped[key]) grouped[key] = [];
    grouped[key].push(inst);
  }

  if (related.length === 0) {
    return (
      <div className="max-w-5xl mx-auto p-8">
        <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
            <Network className="w-5 h-5 text-gray-500" />
            <h3 className="text-lg font-medium text-gray-900">Related Cases</h3>
          </div>
          <div className="p-12 text-center">
            <div className="w-16 h-16 bg-slate-50 rounded-2xl flex items-center justify-center mx-auto mb-6 border border-slate-100">
              <Link2 className="w-8 h-8 text-slate-300" />
            </div>
            <h4 className="text-base font-bold text-slate-700 mb-2">No related cases found</h4>
            <p className="text-sm text-slate-500 max-w-md mx-auto leading-relaxed">
              No other case instances exist in the system.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto p-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">Related Cases</h2>
          <p className="text-sm text-gray-500 mt-1">{related.length} case{related.length !== 1 ? 's' : ''} across {Object.keys(grouped).length} workflow{Object.keys(grouped).length !== 1 ? 's' : ''}</p>
        </div>
      </div>

      {Object.entries(grouped).map(([definitionUrl, instances]) => (
        <div key={definitionUrl} className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
            <Network className="w-4 h-4 text-gray-500" />
            <h3 className="text-sm font-semibold text-gray-900">{definitionUrl}</h3>
            <span className="ml-auto text-xs text-gray-500">{instances.length} instance{instances.length !== 1 ? 's' : ''}</span>
          </div>
          <div className="divide-y divide-gray-100">
            {instances.map(inst => (
              <div key={inst.instanceId} className="p-5 hover:bg-gray-50 transition-colors">
                <div className="flex items-start gap-4">
                  <div className="w-10 h-10 bg-slate-50 rounded-lg flex items-center justify-center flex-shrink-0 border border-slate-200">
                    <Link2 className="w-5 h-5 text-slate-500" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-2">
                      <h4 className="text-sm font-semibold text-gray-900 font-mono">{extractShortId(inst.instanceId)}</h4>
                      <StatusBadge status={inst.status} />
                      <ImpactBadge level={inst.impactLevel} />
                    </div>
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                      <div>
                        <span className="text-gray-500 block">Configuration</span>
                        <div className="flex flex-wrap gap-1 mt-0.5">
                          {inst.configuration.length > 0 ? (
                            inst.configuration.map((s, i) => (
                              <span key={i} className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-blue-50 text-blue-800 border border-blue-100 font-mono text-xs">
                                <Layers className="w-2.5 h-2.5" />
                                {s}
                              </span>
                            ))
                          ) : (
                            <span className="text-gray-400">—</span>
                          )}
                        </div>
                      </div>
                      <div>
                        <span className="text-gray-500 block">Active Tasks</span>
                        <span className="flex items-center gap-1 mt-0.5 text-gray-900">
                          <ListChecks className="w-3 h-3" />
                          {inst.activeTasks.length}
                        </span>
                      </div>
                      <div>
                        <span className="text-gray-500 block">Created</span>
                        <span className="flex items-center gap-1 mt-0.5 text-gray-700">
                          <Clock className="w-3 h-3" />
                          {new Date(inst.createdAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                        </span>
                      </div>
                      <div>
                        <span className="text-gray-500 block">Updated</span>
                        <span className="flex items-center gap-1 mt-0.5 text-gray-700">
                          <Clock className="w-3 h-3" />
                          {new Date(inst.updatedAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
