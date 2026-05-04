import React from 'react';
import { CheckSquare, UserPlus, PauseCircle, X } from 'lucide-react';

interface BulkActionBarProps {
  selectedCount: number;
  onClear: () => void;
  onBatchApprove: () => void;
  onReassign: () => void;
  onHold: () => void;
}

export function BulkActionBar({ selectedCount, onClear, onBatchApprove, onReassign, onHold }: BulkActionBarProps) {
  if (selectedCount === 0) return null;

  return (
    <div className="fixed bottom-4 sm:bottom-6 left-1/2 transform -translate-x-1/2 bg-gray-900 text-white px-3 sm:px-6 py-3 sm:py-4 rounded-xl shadow-2xl flex items-center gap-3 sm:gap-6 z-50 animate-in slide-in-from-bottom-8 fade-in duration-200 max-w-[95vw] sm:max-w-none">
      <div className="flex items-center gap-2 sm:gap-3 border-r border-gray-700 pr-3 sm:pr-6 shrink-0">
        <span className="bg-blue-600 text-white text-xs sm:text-sm font-bold w-5 h-5 sm:w-6 sm:h-6 rounded-full flex items-center justify-center">
          {selectedCount}
        </span>
        <span className="text-xs sm:text-sm font-medium">Selected Tasks</span>
      </div>
      
      <div className="flex items-center gap-1 sm:gap-2">
        <button 
          onClick={onBatchApprove}
          className="flex items-center gap-2 px-2 sm:px-3 py-2 rounded-lg hover:bg-gray-800 text-sm font-medium transition-colors"
          title="Batch Approve"
        >
          <CheckSquare className="w-4 h-4 text-emerald-400" />
          <span className="hidden md:inline">Batch Approve</span>
        </button>
        <button 
          onClick={onReassign}
          className="flex items-center gap-2 px-2 sm:px-3 py-2 rounded-lg hover:bg-gray-800 text-sm font-medium transition-colors" 
          title="Reassign"
        >
          <UserPlus className="w-4 h-4 text-blue-400" />
          <span className="hidden md:inline">Reassign</span>
        </button>
        <button 
          onClick={onHold}
          className="flex items-center gap-2 px-2 sm:px-3 py-2 rounded-lg hover:bg-gray-800 text-sm font-medium transition-colors" 
          title="Put on Hold"
        >
          <PauseCircle className="w-4 h-4 text-amber-400" />
          <span className="hidden md:inline">Put on Hold</span>
        </button>
      </div>

      <button 
        onClick={onClear}
        className="ml-2 sm:ml-4 p-1.5 hover:bg-gray-800 rounded-full text-gray-400 hover:text-white transition-colors"
        title="Clear selection"
      >
        <X className="w-4 h-4 sm:w-5 h-5" />
      </button>
    </div>
  );
}
