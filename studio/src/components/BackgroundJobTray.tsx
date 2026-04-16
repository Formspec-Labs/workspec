import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { CheckCircle2, AlertCircle, Loader2, X, ChevronDown, ChevronUp } from 'lucide-react';
import { useBackend } from '../context/WosContext';
import { BackgroundJob } from '../types';

const STUB_JOBS: BackgroundJob[] = [
  { id: 'job1', type: 'Batch PDF Export', status: 'processing', progress: 67, message: 'Exporting 124 case files...', startedAt: '2026-04-09T15:00:00Z' },
  { id: 'job2', type: 'AI Model Retrain', status: 'pending', progress: 0, message: 'Queued for processing', startedAt: '2026-04-09T15:10:00Z' },
  { id: 'job3', type: 'Regulatory Migration', status: 'completed', progress: 100, message: 'Successfully migrated 89 cases', startedAt: '2026-04-09T14:00:00Z', completedAt: '2026-04-09T14:45:00Z' },
];

export function BackgroundJobTray() {
  useBackend();
  const [jobs] = useState<BackgroundJob[]>(STUB_JOBS);
  const [isOpen, setIsOpen] = useState(false);

  const activeCount = jobs.filter(j => j.status === 'processing' || j.status === 'pending').length;

  if (jobs.length === 0) return null;

  return (
    <div className="fixed bottom-8 left-8 z-[100]">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`flex items-center gap-3 px-4 py-2 rounded-full shadow-2xl border transition-all ${activeCount > 0 ? 'bg-blue-600 border-blue-500 text-white' : 'bg-white border-slate-200 text-slate-600'}`}
      >
        {activeCount > 0 ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <CheckCircle2 className="w-4 h-4 text-emerald-500" />
        )}
        <span className="text-xs font-black uppercase tracking-widest">
          {activeCount > 0 ? `${activeCount} Active Tasks` : 'All Tasks Complete'}
        </span>
        {isOpen ? <ChevronDown className="w-4 h-4" /> : <ChevronUp className="w-4 h-4" />}
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: 20, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 20, scale: 0.95 }}
            className="absolute bottom-14 left-0 w-80 bg-white border border-slate-200 rounded-2xl shadow-2xl overflow-hidden"
          >
            <div className="p-4 border-b border-slate-50 bg-slate-50/50 flex items-center justify-between">
              <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Background Operations</h3>
              <button onClick={() => setIsOpen(false)} className="text-slate-400 hover:text-slate-600">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="max-h-80 overflow-y-auto p-4 space-y-4">
              {jobs.slice().reverse().map(job => (
                <div key={job.id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold text-slate-900">{job.type}</span>
                    <span className={`text-[10px] font-black uppercase tracking-tighter ${
                      job.status === 'completed' ? 'text-emerald-500' : 
                      job.status === 'failed' ? 'text-rose-500' : 'text-blue-500'
                    }`}>
                      {job.status}
                    </span>
                  </div>
                  <div className="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden">
                    <motion.div 
                      initial={{ width: 0 }}
                      animate={{ width: `${job.progress}%` }}
                      className={`h-full ${
                        job.status === 'completed' ? 'bg-emerald-500' : 
                        job.status === 'failed' ? 'bg-rose-500' : 'bg-blue-500'
                      }`}
                    />
                  </div>
                  <p className="text-[10px] text-slate-500 italic">{job.message}</p>
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
