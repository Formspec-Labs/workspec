import React, { useState } from 'react';
import { Save, Send, AlertCircle, PauseCircle, X, CheckCircle, XCircle } from 'lucide-react';
import { ConfirmationModal } from '../ui/ConfirmationModal';
import { useBackend } from '../../context/WosContext';

interface ActionBarProps {
  instanceId?: string;
  aiFieldCount?: number;
}

export function ActionBar({ instanceId, aiFieldCount = 0 }: ActionBarProps) {
  const backend = useBackend();
  const [isDecisionModalOpen, setIsDecisionModalOpen] = useState(false);
  const [decisionType, setDecisionType] = useState<'approve' | 'reject' | null>(null);
  const [reasonCode, setReasonCode] = useState('');
  const [rationale, setRationale] = useState('');

  const handleDecision = (type: 'approve' | 'reject') => {
    setDecisionType(type);
    setIsDecisionModalOpen(true);
  };

  const confirmDecision = async () => {
    if (instanceId && decisionType) {
      try {
        await backend.submitEvent(
          instanceId,
          decisionType === 'approve' ? 'approve' : 'reject',
          'current-user',
          { reasonCode, rationale }
        );
      } catch (err) {
        console.error('Failed to submit decision:', err);
      }
    }
    setIsDecisionModalOpen(false);
  };

  return (
    <div className="bg-white border-t border-gray-200 px-4 sm:px-6 py-3 sm:py-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shrink-0 z-20">
      <div className="flex items-center justify-between sm:justify-start gap-4">
        <div className="flex items-center gap-2 text-[10px] sm:text-sm text-amber-600 bg-amber-50 px-2 sm:px-3 py-1 sm:py-1.5 rounded-md border border-amber-200">
          <AlertCircle className="w-3.5 h-3.5 sm:w-4 h-4" />
          <span className="font-medium">{aiFieldCount > 0 ? `${aiFieldCount} AI fields pending review` : 'No AI fields pending'}</span>
        </div>
        
        <button className="sm:hidden p-2 text-gray-500 hover:bg-gray-100 rounded-md transition-colors" title="Cancel">
          <X className="w-5 h-5" />
        </button>
      </div>
      
      <div className="flex items-center gap-2 sm:gap-3">
        <button className="hidden sm:block px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-md transition-colors">
          Cancel
        </button>
        <button className="flex-1 sm:flex-none flex items-center justify-center gap-2 px-3 sm:px-4 py-2 text-sm font-medium text-gray-700 bg-gray-50 sm:bg-transparent hover:bg-gray-100 rounded-lg sm:rounded-md transition-colors border border-gray-200 sm:border-transparent">
          <PauseCircle className="w-4 h-4" />
          <span className="hidden xs:inline sm:inline">Hold</span>
        </button>
        
        <div className="h-8 w-px bg-gray-200 mx-1 hidden sm:block" />

        <button 
          onClick={() => handleDecision('reject')}
          className="flex-1 sm:flex-none flex items-center justify-center gap-2 px-4 py-2 text-sm font-bold text-rose-600 bg-rose-50 hover:bg-rose-100 rounded-lg border border-rose-200 transition-all"
        >
          <XCircle className="w-4 h-4" />
          <span>Reject</span>
        </button>

        <button 
          onClick={() => handleDecision('approve')}
          className="flex-1 sm:flex-none flex items-center justify-center gap-2 px-4 py-2 text-sm font-bold text-emerald-600 bg-emerald-50 hover:bg-emerald-100 rounded-lg border border-emerald-200 transition-all"
        >
          <CheckCircle className="w-4 h-4" />
          <span>Approve</span>
        </button>

        <button className="flex-[2] sm:flex-none flex items-center justify-center gap-2 px-4 sm:px-6 py-2 text-sm font-bold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors shadow-lg shadow-blue-100">
          <Send className="w-4 h-4" />
          <span>Submit</span>
        </button>
      </div>

      <ConfirmationModal
        isOpen={isDecisionModalOpen}
        onClose={() => setIsDecisionModalOpen(false)}
        onConfirm={confirmDecision}
        title={decisionType === 'approve' ? 'Confirm Approval' : 'Confirm Rejection'}
        confirmLabel={decisionType === 'approve' ? 'Confirm Approval' : 'Confirm Rejection'}
        variant={decisionType === 'approve' ? 'info' : 'danger'}
        disabled={!reasonCode || (decisionType === 'reject' && rationale.length < 10)}
        message={
          <div className="space-y-4">
            <p className="text-sm text-gray-600">
              Please provide a formal rationale for this {decisionType} decision. This will be recorded in the immutable audit trail.
            </p>
            
            <div className="space-y-2">
              <label className="text-[10px] font-black uppercase tracking-widest text-gray-400">Reason Code</label>
              <select 
                value={reasonCode}
                onChange={(e) => setReasonCode(e.target.value)}
                className="w-full p-2.5 bg-gray-50 border border-gray-200 rounded-xl text-sm focus:ring-2 focus:ring-blue-500"
              >
                <option value="">Select a reason...</option>
                {decisionType === 'approve' ? (
                  <>
                    <option value="meets-all">Meets all eligibility criteria</option>
                    <option value="policy-exception">Policy exception granted</option>
                    <option value="verified-docs">Verified documentation complete</option>
                  </>
                ) : (
                  <>
                    <option value="missing-docs">Missing required documentation</option>
                    <option value="ineligible">Ineligible based on income/assets</option>
                    <option value="fraud-suspected">Suspected fraudulent activity</option>
                    <option value="other">Other (specify below)</option>
                  </>
                )}
              </select>
            </div>

            <div className="space-y-2">
              <label className="text-[10px] font-black uppercase tracking-widest text-gray-400">Detailed Rationale</label>
              <textarea 
                value={rationale}
                onChange={(e) => setRationale(e.target.value)}
                placeholder="Provide a detailed explanation for the audit trail..."
                className="w-full p-3 bg-gray-50 border border-gray-200 rounded-xl text-sm focus:ring-2 focus:ring-blue-500 h-32 resize-none"
              />
              {decisionType === 'reject' && rationale.length < 10 && (
                <p className="text-[10px] text-rose-500 font-bold italic">Min. 10 characters required for rejections.</p>
              )}
            </div>
          </div>
        }
      />
    </div>
  );
}
