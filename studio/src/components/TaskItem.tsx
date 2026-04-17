import React from 'react';
import type { TaskListItem } from '../services/WosPorts';
import { Clock, ChevronRight, Eye } from 'lucide-react';
import { motion } from 'motion/react';

function mapWosStatus(status: string): string {
  switch (status) {
    case 'created':
    case 'assigned': return 'new';
    case 'claimed': return 'in-progress';
    case 'escalated': return 'escalated';
    case 'delegated': return 'on-hold';
    default: return status;
  }
}

function daysUntilDeadline(deadline?: string): number {
  if (!deadline) return Infinity;
  const now = new Date();
  const dl = new Date(deadline);
  return Math.ceil((dl.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
}

interface TaskItemProps {
  key?: React.Key;
  task: TaskListItem;
  isSelected: boolean;
  onSelect: (id: string) => void;
  onClick: (id: string) => void;
  onPeek: (id: string) => void;
}

export function TaskItem({ task, isSelected, onSelect, onClick, onPeek }: TaskItemProps) {
  const deadlineDays = daysUntilDeadline(task.deadline);
  const displayStatus = mapWosStatus(task.status);

  const getDeadlineColor = (days: number) => {
    if (days < 0) return 'text-rose-600 bg-rose-50 border-rose-100';
    if (days <= 2) return 'text-amber-600 bg-amber-50 border-amber-100';
    return 'text-emerald-600 bg-emerald-50 border-emerald-100';
  };

  const getDeadlineText = (days: number) => {
    if (days < 0) return 'Overdue';
    if (days === 0) return 'Due Today';
    if (days === 1) return 'Due Tomorrow';
    if (!isFinite(days)) return 'No deadline';
    return `Due in ${days} days`;
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'new': return 'bg-blue-50 text-blue-700 border-blue-100';
      case 'in-progress': return 'bg-indigo-50 text-indigo-700 border-indigo-100';
      case 'on-hold': return 'bg-slate-100 text-slate-700 border-slate-200';
      case 'escalated': return 'bg-rose-50 text-rose-700 border-rose-100';
      default: return 'bg-slate-50 text-slate-700 border-slate-100';
    }
  };

  const getImpactColor = (level?: string) => {
    switch (level) {
      case 'critical': return 'text-rose-600';
      case 'high': return 'text-amber-600';
      default: return 'text-slate-900';
    }
  };

  return (
    <motion.div 
      layout
      data-testid="task-item"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95 }}
      whileHover={{ x: 4 }}
      className={`group flex flex-col sm:flex-row items-start sm:items-center gap-4 sm:gap-8 p-4 sm:p-6 sm:px-10 border-b border-slate-100 hover:bg-slate-50/80 transition-all cursor-pointer relative ${isSelected ? 'bg-blue-50/40 hover:bg-blue-50/60' : ''}`}
      onClick={() => onClick(task.taskId)}
    >
      {isSelected && (
        <div className="absolute left-0 top-0 bottom-0 w-1.5 bg-blue-600" />
      )}

      <div className="flex items-center gap-4 w-full sm:w-auto">
        <div className="flex-shrink-0" onClick={(e) => e.stopPropagation()}>
          <input 
            type="checkbox" 
            aria-label={`Select task: ${task.taskRef}`}
            className="rounded-lg border-slate-300 text-blue-600 focus:ring-blue-500 w-5 h-5 cursor-pointer transition-all shadow-sm"
            checked={isSelected}
            onChange={() => onSelect(task.taskId)}
          />
        </div>

        <div className="flex-shrink-0 w-14 text-center hidden xs:block">
          <div className="text-[9px] font-black text-slate-400 uppercase tracking-[0.2em] mb-1.5">Impact</div>
          <div className={`text-sm font-black uppercase tracking-tight ${getImpactColor(task.impactLevel)}`}>
            {task.impactLevel || 'Normal'}
          </div>
        </div>

        <div className="xs:hidden flex-shrink-0">
          <div className={`text-sm font-black uppercase tracking-tight ${getImpactColor(task.impactLevel)}`}>
            {task.impactLevel || 'Normal'}
          </div>
        </div>
      </div>

      <div className="flex-grow min-w-0 w-full sm:w-auto">
        <div className="flex flex-wrap items-center gap-2 sm:gap-4 mb-2">
          <span className="text-[9px] sm:text-[10px] font-black text-slate-400 tracking-widest font-mono bg-slate-100 px-2 py-0.5 rounded-lg uppercase border border-slate-200 shadow-sm">{task.instanceId}</span>
          <span className="text-[9px] sm:text-[10px] font-black text-slate-500 bg-slate-50 px-2 py-0.5 rounded-lg border border-slate-200 uppercase tracking-widest">
            {task.definitionTitle}
          </span>
        </div>
        
        <h3 className="text-base sm:text-lg font-black text-slate-900 truncate group-hover:text-blue-700 transition-colors tracking-tight">
          {task.taskRef}
        </h3>
        
        <div className="flex flex-wrap items-center gap-2 sm:gap-4 mt-3 sm:mt-4">
          <span className={`text-[9px] sm:text-[10px] px-2 sm:px-3 py-0.5 sm:py-1 rounded-xl font-black uppercase tracking-widest border shadow-sm ${getStatusBadge(displayStatus)}`}>
            {displayStatus.replace('-', ' ')}
          </span>
        </div>
      </div>

      <div className="flex-shrink-0 flex flex-row sm:flex-col items-center sm:items-end justify-between sm:justify-start gap-3 w-full sm:w-auto pt-3 sm:pt-0 border-t sm:border-t-0 border-slate-100">
        {task.deadline && (
          <div className={`flex items-center gap-2 px-3 sm:px-4 py-1.5 sm:py-2 rounded-2xl border text-[9px] sm:text-[10px] font-black uppercase tracking-widest shadow-sm ${getDeadlineColor(deadlineDays)}`}>
            <Clock className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
            <span className="whitespace-nowrap">{getDeadlineText(deadlineDays)}</span>
          </div>
        )}
        
        <div className="relative z-20 flex items-center gap-2">
          <button 
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onPeek(task.taskId);
            }}
            className="p-2 bg-slate-100 text-slate-500 hover:bg-blue-600 hover:text-white rounded-xl transition-all active:scale-90 shadow-sm"
            title="Quick Peek"
          >
            <Eye className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="hidden sm:flex flex-shrink-0 ml-6 opacity-0 group-hover:opacity-100 transition-all group-hover:translate-x-1">
        <ChevronRight className="w-6 h-6 text-slate-300" />
      </div>
    </motion.div>
  );
}
