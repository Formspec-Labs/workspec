import React, { useState, useEffect } from 'react';
import { CaseForm } from './CaseForm';
import { ReferenceWorkspace } from './ReferenceWorkspace';
import { ActionBar } from './ActionBar';
import { ArrowLeft, BookOpen, ShieldAlert, X } from 'lucide-react';
import { useBackend } from '../../context/WosContext';
import type { CaseInstanceView, ActiveTaskView } from '../../services/WosBackend';
import type { WOSKernelDocument } from '../../types/wos/kernel';

export type WorkspaceTarget = { kind: 'task'; id: string } | { kind: 'instance'; id: string };

interface FormWorkspaceProps {
  target: WorkspaceTarget;
  onBack: () => void;
}

function formatTaskTitle(taskRef: string): string {
  return taskRef
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (s) => s.toUpperCase())
    .trim();
}

function getImpactBadgeConfig(impactLevel: string) {
  switch (impactLevel) {
    case 'rights-impacting':
      return { bg: 'bg-red-100', text: 'text-red-800', border: 'border-red-200', label: 'Rights-Impacting' };
    case 'safety-impacting':
      return { bg: 'bg-orange-100', text: 'text-orange-800', border: 'border-orange-200', label: 'Safety-Impacting' };
    case 'operational':
      return { bg: 'bg-blue-100', text: 'text-blue-800', border: 'border-blue-200', label: 'Operational' };
    case 'informational':
      return { bg: 'bg-gray-100', text: 'text-gray-800', border: 'border-gray-200', label: 'Informational' };
    default:
      return { bg: 'bg-gray-100', text: 'text-gray-800', border: 'border-gray-200', label: impactLevel };
  }
}

export function FormWorkspace({ target, onBack }: FormWorkspaceProps) {
  const [showReference, setShowReference] = useState(window.innerWidth > 1024);
  const [referenceWidth, setReferenceWidth] = useState(55);
  const [isResizing, setIsResizing] = useState(false);
  const [mobileView, setMobileView] = useState<'form' | 'reference'>('form');
  const [isSaving, setIsSaving] = useState(false);
  const [instance, setInstance] = useState<CaseInstanceView | null>(null);
  const [kernel, setKernel] = useState<WOSKernelDocument | null>(null);

  const backend = useBackend();

  useEffect(() => {
    let cancelled = false;
    async function load() {
      let found: CaseInstanceView | null = null;
      if (target.kind === 'instance') {
        found = await backend.getInstance(target.id);
      } else {
        const tasks = await backend.listInstances();
        found = tasks.items.find(i => i.activeTasks.some(t => t.taskId === target.id)) ?? null;
      }
      if (cancelled || !found) return;
      setInstance(found);
      const bundle = await backend.loadBundle(found.definitionUrl);
      if (!cancelled) setKernel(bundle.kernel);
    }
    load();
    return () => { cancelled = true; };
  }, [backend, target]);

  const startResizing = React.useCallback((mouseDownEvent: React.MouseEvent) => {
    setIsResizing(true);
  }, []);

  const stopResizing = React.useCallback(() => {
    setIsResizing(false);
  }, []);

  const resize = React.useCallback((mouseMoveEvent: MouseEvent) => {
    if (isResizing) {
      const newWidth = 100 - (mouseMoveEvent.clientX / window.innerWidth) * 100;
      setReferenceWidth(Math.min(Math.max(newWidth, 20), 80));
    }
  }, [isResizing]);

  React.useEffect(() => {
    window.addEventListener('mousemove', resize);
    window.addEventListener('mouseup', stopResizing);
    return () => {
      window.removeEventListener('mousemove', resize);
      window.removeEventListener('mouseup', stopResizing);
    };
  }, [resize, stopResizing]);

  React.useEffect(() => {
    const handleUpdate = () => {
      setIsSaving(true);
      setTimeout(() => setIsSaving(false), 1000);
    };
    window.addEventListener('form-update', handleUpdate);
    return () => window.removeEventListener('form-update', handleUpdate);
  }, []);

  const activeTask: ActiveTaskView | undefined = instance?.activeTasks?.[0];
  const taskTitle = activeTask ? formatTaskTitle(activeTask.taskRef) : 'Loading...';
  const impactLevel = instance?.impactLevel ?? activeTask?.impactLevel ?? 'operational';
  const impactBadge = getImpactBadgeConfig(impactLevel);
  const activeDelegations = instance?.governanceState?.activeDelegations ?? [];

  return (
    <div className="flex flex-col flex-1 overflow-hidden bg-gray-50">
      <div className="bg-white border-b border-gray-200 px-4 py-2 flex items-center justify-between shrink-0 h-14 sm:h-16">
        <div className="flex items-center gap-3 sm:gap-4">
          <button onClick={onBack} className="p-1.5 hover:bg-gray-100 rounded-md text-gray-500 transition-colors">
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-sm sm:text-lg font-semibold text-gray-900 leading-tight truncate max-w-[120px] xs:max-w-[200px] sm:max-w-none">{taskTitle}</h2>
              <span className={`hidden sm:inline-flex items-center px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider ${impactBadge.bg} ${impactBadge.text} border ${impactBadge.border}`}>
                {impactBadge.label}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <p className="text-[10px] sm:text-sm text-gray-500 font-mono">{instance?.instanceId ?? '...'}</p>
              <span className="text-gray-300">&bull;</span>
              <div className="flex items-center gap-1.5">
                {isSaving ? (
                  <div className="flex items-center gap-1">
                    <div className="w-1 h-1 bg-blue-400 rounded-full animate-pulse" />
                    <span className="text-[10px] font-bold text-blue-500 uppercase tracking-widest">Saving...</span>
                  </div>
                ) : (
                  <div className="flex items-center gap-1">
                    <ShieldAlert className="w-3 h-3 text-emerald-500" />
                    <span className="text-[10px] font-bold text-emerald-600 uppercase tracking-widest">Saved</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="hidden md:flex items-center -space-x-2 mr-2">
            <div className="w-7 h-7 rounded-full border-2 border-white bg-indigo-100 flex items-center justify-center text-[10px] font-bold text-indigo-600 shadow-sm" title="Admin User (Viewing Now)">AU</div>
            <div className="w-7 h-7 rounded-full border-2 border-white bg-emerald-100 flex items-center justify-center text-[10px] font-bold text-emerald-600 shadow-sm" title="System Auditor (Viewing Now)">SA</div>
            <div className="w-7 h-7 rounded-full border-2 border-white bg-slate-100 flex items-center justify-center text-[10px] font-bold text-slate-400 shadow-sm">+1</div>
          </div>
          <div className="flex sm:hidden bg-gray-100 p-1 rounded-lg mr-2">
            <button 
              onClick={() => setMobileView('form')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-wider rounded-md transition-all ${
                mobileView === 'form' ? 'bg-white text-blue-600 shadow-sm' : 'text-gray-500'
              }`}
            >
              Form
            </button>
            <button 
              onClick={() => setMobileView('reference')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-wider rounded-md transition-all ${
                mobileView === 'reference' ? 'bg-white text-blue-600 shadow-sm' : 'text-gray-500'
              }`}
            >
              Evidence
            </button>
          </div>

          <button 
            onClick={() => setShowReference(!showReference)} 
            className={`hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-md transition-all ${
              showReference ? 'bg-blue-50 text-blue-600 ring-1 ring-blue-200' : 'bg-white text-gray-600 border border-gray-200 hover:bg-gray-50'
            }`}
            title="Toggle Reference Panel"
          >
            <BookOpen className="w-4 h-4" />
            <span className="text-sm font-medium">Reference</span>
          </button>
        </div>
      </div>

      {activeDelegations.length > 0 && activeDelegations.map((del, idx) => (
        <div key={idx} className="bg-blue-50 border-b border-blue-100 px-4 py-1.5 flex items-center gap-2 text-[10px] sm:text-xs text-blue-800 shrink-0">
          <ShieldAlert className="w-3.5 h-3.5 text-blue-600" />
          <span className="truncate">
            <strong>Delegated Authority:</strong> Acting for {del.delegatorId} (Scope: {del.scope}, Authority: {del.authority ?? 'general'}).
          </span>
        </div>
      ))}

      <div className={`flex flex-1 overflow-hidden relative ${isResizing ? 'cursor-col-resize select-none' : ''}`}>
        <div 
          className={`flex-1 overflow-y-auto relative bg-white transition-all duration-300 ${mobileView === 'reference' ? 'hidden sm:block' : 'block'}`}
          style={{ width: showReference ? `${100 - referenceWidth}%` : '100%' }}
        >
          {instance && <CaseForm instance={instance} kernel={kernel} />}
          {!instance && (
            <div className="flex items-center justify-center h-full text-gray-400 text-sm">Loading case...</div>
          )}
        </div>

        {showReference && (
          <div 
            onMouseDown={startResizing}
            className="hidden sm:block w-1 hover:w-1.5 bg-gray-200 hover:bg-blue-400 cursor-col-resize transition-all z-20"
          />
        )}

        <div 
          className={`transition-all duration-300 border-l border-gray-200 bg-white ${
            showReference ? 'translate-x-0' : 'w-0 translate-x-full sm:hidden'
          } ${mobileView === 'form' ? 'hidden sm:block' : 'block absolute inset-0 z-30 sm:relative sm:z-auto sm:block'}`}
          style={{ width: showReference ? `${referenceWidth}%` : '0' }}
        >
          <ReferenceWorkspace />
          
          <button 
            onClick={() => setMobileView('form')}
            className="sm:hidden absolute top-3 right-3 p-2 bg-white rounded-full shadow-xl border border-gray-200 z-40"
          >
            <X className="w-5 h-5 text-gray-500" />
          </button>
        </div>
      </div>

      <ActionBar instanceId={instance?.instanceId} />
    </div>
  );
}
