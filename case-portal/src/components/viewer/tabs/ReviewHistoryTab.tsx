import React, { useState, useEffect } from 'react';
import { History, User, Cpu, MessageSquare } from 'lucide-react';
import { useCaseViewer } from '../../../context/WosContext';
import type { ProvenanceRecord } from '../../../services/WosBackend';

export function ReviewHistoryTab({ caseId }: { caseId: string }) {
  const caseViewer = useCaseViewer();
  const [records, setRecords] = useState<ProvenanceRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    caseViewer.getProvenance(caseId).then(all => {
      const reviewEvents = all.filter(r =>
        r.event.toLowerCase().includes('review') ||
        r.tier === 'reasoning'
      );
      setRecords(reviewEvents);
      setIsLoading(false);
    });
  }, [caseViewer, caseId]);

  if (isLoading) {
    return (
      <div className="p-12 flex items-center justify-center">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto p-8">
      <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
          <History className="w-5 h-5 text-gray-500" />
          <h3 className="text-lg font-medium text-gray-900">Case Review History</h3>
        </div>

        <div className="p-8">
          <div className="space-y-8">
            {records.map((record, idx) => (
              <div key={record.id} className="relative pl-8 group">
                {idx !== records.length - 1 && (
                  <div className="absolute left-3.5 top-8 bottom-[-32px] w-0.5 bg-slate-100 group-hover:bg-blue-100 transition-colors"></div>
                )}
                
                <div className="absolute left-0 top-1 w-7 h-7 bg-white border-2 border-slate-200 rounded-full flex items-center justify-center group-hover:border-blue-500 transition-colors z-10">
                  {record.actor.type === 'agent' ? (
                    <Cpu className="w-3.5 h-3.5 text-slate-400 group-hover:text-blue-600" />
                  ) : (
                    <User className="w-3.5 h-3.5 text-slate-400 group-hover:text-blue-600" />
                  )}
                </div>

                <div className="bg-slate-50 border border-slate-100 rounded-2xl p-5 hover:bg-white hover:border-blue-200 hover:shadow-md transition-all">
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <span className="text-sm font-black text-slate-900 tracking-tight">{record.event}</span>
                      <span className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">by {record.actor.name}</span>
                    </div>
                    <span className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">
                      {new Date(record.timestamp).toLocaleString()}
                    </span>
                  </div>
                  
                  <div className="flex items-start gap-3 bg-white border border-slate-100 rounded-xl p-3">
                    <MessageSquare className="w-4 h-4 text-slate-300 shrink-0 mt-0.5" />
                    <div className="text-xs text-slate-600 leading-relaxed font-medium">
                      <span className="font-bold">{record.sourceState} → {record.targetState}</span>
                      {record.reasoning?.explanation && (
                        <p className="mt-1">{record.reasoning.explanation}</p>
                      )}
                      {record.aiNarrative && (
                        <p className="mt-1 italic text-slate-500">{record.aiNarrative.text}</p>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>

          {records.length === 0 && (
            <div className="py-12 text-center">
              <p className="text-sm text-slate-500">No review history recorded for this case.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
