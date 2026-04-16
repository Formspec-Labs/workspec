import React, { useState, useEffect } from 'react';
import { ArrowLeft, Download, Printer, ShieldAlert, GitCommit, ChevronRight } from 'lucide-react';
import { TabType } from './CaseViewer';
import { useCaseViewer } from '../../context/WosContext';
import type { CaseInstanceView } from '../../services/WosBackend';

interface CaseHeaderProps {
  caseId: string;
  onBack: () => void;
  activeTab: TabType;
  onTabChange: (tab: TabType) => void;
}

export function CaseHeader({ caseId, onBack, activeTab, onTabChange }: CaseHeaderProps) {
  const caseViewer = useCaseViewer();
  const [instance, setInstance] = useState<CaseInstanceView | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    setIsLoading(true);
    caseViewer.getInstance(caseId).then(data => {
      setInstance(data);
      setIsLoading(false);
    });
  }, [caseViewer, caseId]);

  const tabs: { id: TabType; label: string }[] = [
    { id: 'timeline', label: 'Timeline' },
    { id: 'case-file', label: 'Case File' },
    { id: 'related', label: 'Related Cases (2)' },
    { id: 'review-history', label: 'Review History' },
    { id: 'documents', label: 'Docs & Correspondence' },
  ];

  const [isPrinting, setIsPrinting] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  const handlePrint = async () => {
    setIsPrinting(true);
    setIsPrinting(false);
  };

  const handleExport = async () => {
    setIsExporting(true);
    setIsExporting(false);
  };

  if (isLoading || !instance) {
    return (
      <div className="bg-white border-b border-slate-200 px-10 py-8 h-[200px] flex items-center justify-center">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  return (
    <div className="bg-white border-b border-slate-200 shrink-0 flex flex-col shadow-sm relative z-20">
      <div className="px-4 sm:px-10 py-6 sm:py-8 flex flex-col lg:flex-row lg:items-start justify-between gap-6 sm:gap-8">
        <div className="flex items-start gap-4 sm:gap-6">
          <button 
            onClick={onBack} 
            className="mt-1 p-2 sm:p-3 hover:bg-slate-50 rounded-xl sm:rounded-2xl text-slate-400 transition-all active:scale-90 border border-slate-100 hover:border-slate-200 bg-white shadow-sm shrink-0"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div className="space-y-3 sm:space-y-4 min-w-0">
            <div className="flex flex-wrap items-center gap-3 sm:gap-4">
              <h2 className="text-2xl sm:text-4xl font-black text-slate-900 tracking-tight truncate">{(instance.caseState as any)?.application?.applicantName ?? instance.instanceId.split(':').pop()}</h2>
              <span className="text-[9px] sm:text-[10px] font-black font-mono text-slate-400 bg-slate-50 px-2 py-0.5 sm:px-2.5 sm:py-1 rounded-lg border border-slate-200 tracking-widest uppercase">{caseId}</span>
              <div className="flex items-center gap-2 sm:gap-3">
                <span className="text-[8px] sm:text-[9px] font-black bg-blue-50 text-blue-700 px-2 py-1 sm:px-3 sm:py-1.5 rounded-xl border border-blue-100 uppercase tracking-[0.2em] shadow-sm whitespace-nowrap">{instance.status}</span>
                {instance.impactLevel === 'rights-impacting' && (
                  <span className="text-[8px] sm:text-[9px] font-black bg-rose-50 text-rose-700 px-2 py-1 sm:px-3 sm:py-1.5 rounded-xl border border-rose-100 flex items-center gap-1.5 sm:gap-2 uppercase tracking-[0.2em] shadow-sm" title="Rights-Impacting Case">
                    <ShieldAlert className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                    <span className="hidden xs:inline">Rights-Impacting</span>
                  </span>
                )}
              </div>
            </div>
            
            <div className="flex flex-wrap items-center gap-4 sm:gap-6 text-[10px] sm:text-xs text-slate-500">
              <div className="flex items-center gap-2 sm:gap-2.5">
                <GitCommit className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-300" />
                <span className="font-bold uppercase tracking-widest text-[9px] sm:text-[10px]">Workflow: <span className="text-slate-900">{instance.definitionUrl.split('/').pop()}</span></span>
              </div>
              <div className="hidden xs:block w-1 h-1 bg-slate-300 rounded-full"></div>
              <div className="flex items-center gap-2 sm:gap-2.5">
                <span className="font-black text-[8px] sm:text-[9px] bg-slate-100 border border-slate-200 px-2 py-0.5 sm:px-2.5 sm:py-1 rounded-lg uppercase tracking-[0.15em] text-slate-500 shadow-sm">v{instance.definitionVersion}</span>
              </div>
              <div className="hidden xs:block w-1 h-1 bg-slate-300 rounded-full"></div>
              <span className="font-bold uppercase tracking-widest text-[9px] sm:text-[10px]">Active States <span className="text-slate-900 font-black ml-1">{instance.configuration.join(', ')}</span></span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 sm:gap-3 self-end lg:self-auto">
          <button 
            onClick={handlePrint}
            disabled={isPrinting}
            className="flex items-center gap-2 px-3 sm:px-5 py-2 sm:py-3 text-[9px] sm:text-[10px] font-black uppercase tracking-widest text-slate-700 bg-white hover:bg-slate-50 rounded-xl sm:rounded-2xl transition-all border border-slate-200 shadow-sm active:scale-95 disabled:opacity-70"
          >
            {isPrinting ? <div className="w-3.5 h-3.5 sm:w-4 sm:h-4 border-2 border-slate-400 border-t-transparent rounded-full animate-spin" /> : <Printer className="w-3.5 h-3.5 sm:w-4 sm:h-4" />}
            <span className="hidden xs:inline">{isPrinting ? 'Printing...' : 'Print Summary'}</span>
            <span className="xs:hidden">Print</span>
          </button>
          <button 
            onClick={handleExport}
            disabled={isExporting}
            className="flex items-center gap-2 px-4 sm:px-6 py-2 sm:py-3 text-[9px] sm:text-[10px] font-black uppercase tracking-widest text-white bg-slate-900 hover:bg-slate-800 rounded-xl sm:rounded-2xl transition-all shadow-xl shadow-slate-200 active:scale-95 border border-slate-800 disabled:opacity-70"
          >
            {isExporting ? <div className="w-3.5 h-3.5 sm:w-4 sm:h-4 border-2 border-white border-t-transparent rounded-full animate-spin" /> : <Download className="w-3.5 h-3.5 sm:w-4 sm:h-4" />}
            <span className="hidden xs:inline">{isExporting ? 'Exporting...' : 'Export Record'}</span>
            <span className="xs:hidden">Export</span>
          </button>
        </div>
      </div>

      <div className="px-4 sm:px-28 pb-6 sm:pb-8">
        <div className="flex items-center gap-y-3 gap-x-2 text-[8px] sm:text-[9px] font-black text-slate-400 uppercase tracking-[0.2em] overflow-x-auto no-scrollbar pb-2">
          {instance.configuration.map((state, i) => {
            const isLast = i === instance.configuration.length - 1;
            return (
              <span key={state} className={`flex items-center gap-2 px-2.5 py-1 rounded-lg border whitespace-nowrap ${isLast ? 'text-blue-600 bg-blue-50 border-blue-200 shadow-sm' : 'text-emerald-600 bg-emerald-50/50 border-emerald-100'}`}>
                {state}
                {!isLast && <ChevronRight className="w-3 h-3 text-emerald-300" />}
              </span>
            );
          })}
        </div>
      </div>

      <div className="px-4 sm:px-10 flex gap-6 sm:gap-10 border-t border-slate-100 pt-1 overflow-x-auto no-scrollbar bg-slate-50/30">
        {tabs.map(tab => (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className={`pb-4 sm:pb-5 text-[9px] sm:text-[10px] font-black uppercase tracking-[0.2em] sm:tracking-[0.25em] border-b-2 transition-all whitespace-nowrap pt-3 ${
              activeTab === tab.id 
                ? 'border-blue-600 text-blue-700' 
                : 'border-transparent text-slate-400 hover:text-slate-600 hover:border-slate-200'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>
    </div>
  );
}
