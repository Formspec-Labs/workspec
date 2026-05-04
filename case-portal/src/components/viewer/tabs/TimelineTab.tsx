import React, { useState, useEffect } from 'react';
import { Filter, User, Sparkles, Phone, FileText, ChevronDown, ChevronUp } from 'lucide-react';
import { useCaseViewer } from '../../../context/WosContext';
import type { ProvenanceRecord } from '../../../services/WosBackend';
import { motion, AnimatePresence } from 'motion/react';

type ActorKind = 'human' | 'system' | 'agent';

function actorKind(record: ProvenanceRecord): ActorKind {
  return record.actor.type;
}

export function TimelineTab({ caseId }: { caseId: string }) {
  const caseViewer = useCaseViewer();
  const [records, setRecords] = useState<ProvenanceRecord[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'human' | 'ai'>('all');
  const [isAllExpanded, setIsAllExpanded] = useState(false);

  useEffect(() => {
    caseViewer.getTimeline(caseId).then(setRecords);
  }, [caseViewer, caseId]);

  const filteredRecords = records.filter(record => {
    if (filter === 'all') return true;
    if (filter === 'human') return actorKind(record) === 'human';
    if (filter === 'ai') return actorKind(record) === 'agent';
    return true;
  });

  const getIcon = (kind: ActorKind) => {
    switch (kind) {
      case 'human': return <User className="w-4 h-4 text-blue-600" />;
      case 'agent': return <Sparkles className="w-4 h-4 text-indigo-600" />;
      case 'system': return <FileText className="w-4 h-4 text-slate-600" />;
      default: return <div className="w-4 h-4" />;
    }
  };

  const getIconBg = (kind: ActorKind) => {
    switch (kind) {
      case 'human': return 'bg-blue-50 border-blue-100';
      case 'agent': return 'bg-indigo-50 border-indigo-100';
      case 'system': return 'bg-slate-50 border-slate-100';
      default: return 'bg-slate-50 border-slate-100';
    }
  };

  return (
    <div className="max-w-5xl mx-auto p-10">
      <div className="flex items-center justify-between mb-12 pb-6 border-b border-slate-100">
        <div className="flex items-center gap-4">
          <div className="p-2 bg-slate-100 rounded-xl text-slate-500">
            <Filter className="w-4 h-4" />
          </div>
          <span className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Filter Timeline</span>
          <div className="flex items-center bg-slate-100/50 border border-slate-200 rounded-xl p-1 shadow-inner ml-2">
            <button 
              onClick={() => setFilter('all')}
              className={`px-4 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-colors ${filter === 'all' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
            >
              All Events
            </button>
            <button 
              onClick={() => setFilter('human')}
              className={`px-4 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-colors ${filter === 'human' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
            >
              Human
            </button>
            <button 
              onClick={() => setFilter('ai')}
              className={`px-4 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-colors ${filter === 'ai' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
            >
              AI
            </button>
          </div>
        </div>
        <button 
          onClick={() => setIsAllExpanded(!isAllExpanded)}
          className="text-[10px] font-black text-blue-600 uppercase tracking-widest hover:text-blue-700 transition-colors"
        >
          {isAllExpanded ? 'Collapse All Details' : 'Expand All Details'}
        </button>
      </div>

      <div className="relative before:absolute before:inset-0 before:ml-6 before:-translate-x-px md:before:mx-auto md:before:translate-x-0 before:h-full before:w-0.5 before:bg-gradient-to-b before:from-transparent before:via-slate-200 before:to-transparent">
        {filteredRecords.map((record, index) => {
          const kind = actorKind(record);
          return (
            <div key={record.id} className="relative flex items-center justify-between md:justify-normal md:odd:flex-row-reverse group is-active mb-12">
              <motion.div 
                whileHover={{ scale: 1.1 }}
                className={`flex items-center justify-center w-14 h-14 rounded-2xl border-4 border-white ${getIconBg(kind)} shrink-0 md:order-1 md:group-odd:-translate-x-1/2 md:group-even:translate-x-1/2 shadow-lg z-10 transition-all`}
              >
                {getIcon(kind)}
              </motion.div>
              
              <motion.div 
                initial={{ opacity: 0, x: index % 2 === 0 ? 20 : -20 }}
                animate={{ opacity: 1, x: 0 }}
                className="w-[calc(100%-4.5rem)] md:w-[calc(50%-4rem)] p-6 rounded-2xl border border-slate-200 bg-white shadow-sm hover:shadow-xl transition-all cursor-pointer group/card relative overflow-hidden" 
                onClick={() => setExpandedId(expandedId === record.id ? null : record.id)}
              >
                <div className="absolute top-0 right-0 w-32 h-32 bg-slate-50 rounded-full -mr-16 -mt-16 opacity-0 group-hover/card:opacity-100 transition-opacity blur-2xl" />
                
                <div className="relative z-10">
                  <div className="flex items-center justify-between mb-3">
                    <time className="font-mono text-[10px] font-black text-slate-400 uppercase tracking-widest">{new Date(record.timestamp).toLocaleString()}</time>
                    <div className="text-[9px] font-black text-slate-400 uppercase tracking-widest flex items-center gap-2 bg-slate-50 px-2 py-1 rounded-lg border border-slate-100">
                      {record.actor.name}
                      {(expandedId === record.id || isAllExpanded) ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
                    </div>
                  </div>
                  <h4 className="text-base font-black text-slate-900 mb-2 tracking-tight group-hover/card:text-blue-700 transition-colors">{record.event}</h4>
                  <p className="text-sm text-slate-500 leading-relaxed font-medium">
                    {record.sourceState} → {record.targetState}
                    {record.reasoning?.explanation && ` — ${record.reasoning.explanation}`}
                  </p>
                  
                  <AnimatePresence>
                    {(expandedId === record.id || isAllExpanded) && (
                      <motion.div 
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        className="mt-6 pt-6 border-t border-slate-100 text-xs text-slate-600 bg-slate-50/50 -mx-6 -mb-6 p-6 rounded-b-2xl overflow-hidden"
                      >
                        <div className="flex items-center gap-2 mb-4">
                          <div className="w-1 h-4 bg-blue-600 rounded-full" />
                          <span className="text-[10px] font-black uppercase tracking-widest">Provenance Record</span>
                        </div>
                        <pre className="font-mono text-[11px] whitespace-pre-wrap leading-relaxed bg-white p-4 rounded-xl border border-slate-200 shadow-inner">
                          {JSON.stringify({
                            tier: record.tier,
                            actor: record.actor,
                            sourceState: record.sourceState,
                            targetState: record.targetState,
                            facts: record.facts,
                            reasoning: record.reasoning,
                            aiNarrative: record.aiNarrative,
                            authorityChain: record.authorityChain,
                          }, null, 2)}
                        </pre>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              </motion.div>
            </div>
          );
        })}
        {filteredRecords.length === 0 && (
          <div className="text-center py-12 text-slate-400 text-sm font-bold uppercase tracking-widest">
            No events found for this filter
          </div>
        )}
      </div>
    </div>
  );
}
