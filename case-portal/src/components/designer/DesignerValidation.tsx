import React from 'react';
import { AlertCircle, CheckCircle2, Info } from 'lucide-react';
import { motion } from 'motion/react';
import type { WorkflowValidation } from './designer-utils';

function ValidationStatus({ label, valid }: { label: string; valid: boolean }) {
  return (
    <div className="flex items-center gap-2.5">
      {valid ? <CheckCircle2 className="w-4 h-4 text-emerald-500" /> : <AlertCircle className="w-4 h-4 text-rose-500" />}
      <span className={`text-[10px] font-black uppercase tracking-widest ${valid ? 'text-slate-900' : 'text-rose-500'}`}>{label}</span>
    </div>
  );
}

export interface DesignerValidationDockProps {
  validation: WorkflowValidation | null;
  onFocusElement: (id: string) => void;
}

export function DesignerValidationDock({ validation, onFocusElement }: DesignerValidationDockProps) {
  return (
    <div className="absolute bottom-0 left-0 right-0 bg-white border-t border-slate-200 z-20 shadow-[0_-4px_20px_rgba(0,0,0,0.05)]">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between px-4 sm:px-8 py-3 border-b border-slate-50 gap-3 sm:gap-0">
        <div className="flex flex-wrap items-center gap-4 sm:gap-8">
          <ValidationStatus label="Structure" valid={validation?.status.structure ?? false} />
          <ValidationStatus label="Policy" valid={validation?.status.policy ?? false} />
          <ValidationStatus label="Soundness" valid={validation?.status.soundness ?? false} />
          <ValidationStatus label="Satisfiability" valid={validation?.status.satisfiability ?? false} />
        </div>
        <div className="text-[9px] font-black text-slate-400 uppercase tracking-[0.2em] hidden xs:block">Real-time Engine Active</div>
      </div>
      {validation && validation.issues.length > 0 && (
        <div className="max-h-40 overflow-y-auto p-3 bg-slate-50/50">
          {validation.issues.map(issue => (
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              key={issue.id}
              className="flex items-center gap-4 px-5 py-2.5 hover:bg-white rounded-xl cursor-pointer group transition-all border border-transparent hover:border-slate-200 shadow-sm mb-1"
              onClick={() => issue.targetId && onFocusElement(issue.targetId)}
            >
              {issue.severity === 'error' ? <AlertCircle className="w-4 h-4 text-rose-500" /> : <Info className="w-4 h-4 text-amber-500" />}
              <span className="text-xs font-bold text-slate-700">{issue.message}</span>
              <span className="text-[10px] font-black text-slate-400 ml-auto opacity-0 group-hover:opacity-100 uppercase tracking-widest">Focus Element</span>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
