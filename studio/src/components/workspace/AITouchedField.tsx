import React, { useState } from 'react';
import { Sparkles, Check, X, Edit2, ChevronDown, ChevronUp, FileText } from 'lucide-react';

interface AITouchedFieldProps {
  id: string;
  label: string;
  value: string;
  confidence: 'high' | 'medium' | 'low';
  verification: 'pass' | 'fail' | 'unverified';
  source: string;
  message?: string;
}

export function AITouchedField({ id, label, value, confidence, verification, source, message }: AITouchedFieldProps) {
  const [status, setStatus] = useState<'pending' | 'accepted' | 'modified' | 'rejected'>('pending');
  const [currentValue, setCurrentValue] = useState(value);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

  const getBorderColor = () => {
    if (status === 'accepted') return 'border-l-emerald-500';
    if (status === 'modified') return 'border-l-blue-500';
    if (status === 'rejected') return 'border-l-gray-400';
    
    if (verification === 'pass') return 'border-l-emerald-400';
    if (verification === 'fail') return 'border-l-red-400';
    return 'border-l-amber-400';
  };

  const getBadgeColor = () => {
    if (verification === 'pass') return 'bg-emerald-100 text-emerald-800';
    if (verification === 'fail') return 'bg-red-100 text-red-800';
    return 'bg-amber-100 text-amber-800';
  };

  return (
    <div className={`relative pl-4 border-l-4 ${getBorderColor()} transition-colors duration-200`}>
      <div className="flex items-start justify-between gap-4">
        <div className="flex-1">
          <label htmlFor={id} className="block text-sm font-medium text-gray-700 mb-1 flex items-center gap-2">
            {label}
            {status === 'pending' && (
              <button 
                onClick={() => setIsExpanded(!isExpanded)}
                className="text-purple-600 hover:text-purple-800 flex items-center gap-1 text-xs font-medium bg-purple-50 px-1.5 py-0.5 rounded"
              >
                <Sparkles className="w-3 h-3" />
                AI
                {isExpanded ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
              </button>
            )}
          </label>
          
          {isEditing ? (
            <input 
              id={id}
              type="text" 
              className="w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" 
              value={currentValue}
              onChange={(e) => setCurrentValue(e.target.value)}
              onBlur={() => {
                setIsEditing(false);
                setStatus('modified');
              }}
              autoFocus
            />
          ) : (
            <div 
              className={`w-full px-3 py-2 border rounded-md shadow-sm text-sm ${status === 'rejected' ? 'text-gray-400 line-through border-gray-200 bg-gray-50' : 'text-gray-900 border-gray-300 bg-white'}`}
              onClick={() => {
                if (status !== 'rejected') setIsEditing(true);
              }}
            >
              {currentValue || <span className="text-gray-400 italic">Empty</span>}
            </div>
          )}
        </div>

        {status === 'pending' && (
          <div className="flex items-center gap-1 mt-6 sm:mt-6">
            <button 
              onClick={() => setStatus('accepted')}
              className="p-1.5 sm:p-2 text-emerald-600 hover:bg-emerald-50 rounded-lg sm:rounded border border-emerald-200 transition-colors"
              title="Accept"
            >
              <Check className="w-4 h-4 sm:w-4.5 sm:h-4.5" />
            </button>
            <button 
              onClick={() => setIsEditing(true)}
              className="p-1.5 sm:p-2 text-blue-600 hover:bg-blue-50 rounded-lg sm:rounded border border-blue-200 transition-colors"
              title="Modify"
            >
              <Edit2 className="w-4 h-4 sm:w-4.5 sm:h-4.5" />
            </button>
            <button 
              onClick={() => {
                setStatus('rejected');
                setCurrentValue('');
              }}
              className="p-1.5 sm:p-2 text-red-600 hover:bg-red-50 rounded-lg sm:rounded border border-red-200 transition-colors"
              title="Reject"
            >
              <X className="w-4 h-4 sm:w-4.5 sm:h-4.5" />
            </button>
          </div>
        )}
        
        {status !== 'pending' && (
          <div className="mt-6 sm:mt-6">
            <button 
              onClick={() => {
                setStatus('pending');
                setCurrentValue(value);
              }}
              className="text-[10px] sm:text-xs text-gray-500 hover:underline font-medium"
            >
              Reset
            </button>
          </div>
        )}
      </div>

      {isExpanded && status === 'pending' && (
        <div className="mt-3 bg-gray-50 border border-gray-200 rounded-xl p-3 sm:p-4 text-sm animate-in slide-in-from-top-2 fade-in">
          <div className="flex flex-col sm:flex-row items-start gap-4">
            <div className="flex-1 space-y-2 w-full">
              <div className="flex items-center justify-between sm:justify-start gap-2">
                <span className="font-semibold text-gray-700 text-xs sm:text-sm">Source:</span>
                <span className="flex items-center gap-1 text-blue-600 hover:underline cursor-pointer text-xs sm:text-sm">
                  <FileText className="w-3.5 h-3.5" />
                  {source}
                </span>
              </div>
              <div className="flex items-center justify-between sm:justify-start gap-2">
                <span className="font-semibold text-gray-700 text-xs sm:text-sm">Confidence:</span>
                <span className="capitalize text-xs sm:text-sm">{confidence}</span>
              </div>
              <div className="flex items-center justify-between sm:justify-start gap-2">
                <span className="font-semibold text-gray-700 text-xs sm:text-sm">Verification:</span>
                <span className={`px-2 py-0.5 rounded text-[10px] sm:text-xs font-medium uppercase ${getBadgeColor()}`}>
                  {verification}
                </span>
              </div>
              {message && (
                <p className="text-gray-600 mt-2 text-[10px] sm:text-xs leading-relaxed">{message}</p>
              )}
            </div>
            <div className="w-full sm:w-32 h-24 sm:h-20 bg-white border border-gray-300 rounded-lg overflow-hidden relative shadow-sm shrink-0">
              {/* Mock thumbnail highlight */}
              <div className="absolute top-4 left-2 right-2 h-3 bg-yellow-200/50 border border-yellow-400"></div>
              <div className="text-[8px] text-gray-400 p-1">Document preview...</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
