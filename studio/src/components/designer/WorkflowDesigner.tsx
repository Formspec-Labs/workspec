import React, { useState, useEffect } from 'react';
import {
  Settings,
  Play,
  Save,
  History,
  Sparkles,
  Plus,
  RefreshCw,
  X,
  GitBranch,
  GitCommit,
  GitMerge,
  Cpu,
  ArrowRight,
  MousePointer2,
  Clock,
  Layers
} from 'lucide-react';
import { useWorkflowDesign, useRealtime } from '../../context/WosContext';
import { kernelToDesigner, designerToKernel, type WorkflowStage, type WorkflowConnection, type DesignerWorkflow } from '../../services/KernelToDesigner';
import type { WOSKernelDocument } from '../../types/wos/kernel';
import { motion, AnimatePresence } from 'motion/react';
import { DesignerCanvas } from './DesignerCanvas';
import { DesignerPropertiesPanel } from './DesignerPropertiesPanel';
import { DesignerToolbar } from './DesignerToolbar';
import { DesignerValidationDock } from './DesignerValidation';
import { mapWosValidation, type WorkflowValidation, type PaletteItemData, type PatternItemData, type ValidationIssue } from './designer-utils';

const MAX_HISTORY = 100;

const PALETTE_ITEMS: PaletteItemData[] = [
  { id: 'simple', label: 'Task', icon: 'Plus', color: 'bg-blue-50', description: 'A human review or action task' },
  { id: 'ai-pipeline', label: 'AI Pipeline', icon: 'Cpu', color: 'bg-purple-50', description: 'AI-powered document extraction or analysis' },
  { id: 'adaptive', label: 'Adaptive', icon: 'RefreshCw', color: 'bg-amber-50', description: 'Adaptive phase with dynamic activities' },
  { id: 'parallel', label: 'Parallel', icon: 'Layers', color: 'bg-emerald-50', description: 'Parallel execution section' },
  { id: 'decision', label: 'Decision', icon: 'GitMerge', color: 'bg-orange-50', description: 'Rules engine decision point' },
  { id: 'split', label: 'Split', icon: 'Split', color: 'bg-cyan-50', description: 'Parallel split into branches' },
  { id: 'join', label: 'Join', icon: 'Merge', color: 'bg-teal-50', description: 'Wait for all parallel branches' },
  { id: 'timer', label: 'Timer', icon: 'Timer', color: 'bg-rose-50', description: 'Timer-based escalation' },
  { id: 'api', label: 'API', icon: 'Webhook', color: 'bg-indigo-50', description: 'External API or webhook call' },
  { id: 'final', label: 'Final', icon: 'CheckCircle2', color: 'bg-gray-50', description: 'Terminal state' },
];

const PATTERNS: PatternItemData[] = [];

function VersionItem({ version, date, author, current }: { version: string; date: string; author: string; current?: boolean }) {
  return (
    <div className={`p-5 rounded-2xl border transition-all cursor-pointer relative group ${current ? 'bg-blue-50 border-blue-200 ring-1 ring-blue-100' : 'bg-white border-slate-200 hover:border-slate-400'}`}>
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-black text-slate-900 tracking-tight">Version {version}</span>
        {current && <span className="text-[9px] font-black text-blue-600 uppercase tracking-[0.2em] bg-white px-2 py-0.5 rounded-lg border border-blue-100 shadow-sm">Active</span>}
      </div>
      <div className="flex items-center gap-3 text-[10px] text-slate-500 font-bold uppercase tracking-wider">
        <div className="flex items-center gap-1.5"><Clock className="w-3.5 h-3.5 text-slate-300" />{date}</div>
        <span className="text-slate-200">•</span>
        <div className="flex items-center gap-1.5"><Settings className="w-3.5 h-3.5 text-slate-300" />{author}</div>
      </div>
    </div>
  );
}

export function WorkflowDesigner() {
  const workflowDesign = useWorkflowDesign();
  const realtime = useRealtime();
  const [workflow, setWorkflow] = useState<DesignerWorkflow | null>(null);
  const [currentKernel, setCurrentKernel] = useState<WOSKernelDocument | null>(null);
  const [validation, setValidation] = useState<WorkflowValidation | null>(null);
  const [selectedElement, setSelectedElement] = useState<{ type: 'stage' | 'connection'; id: string } | null>(null);
  const [collaborators, setCollaborators] = useState<any[]>([]);
  const [remoteCursors, setRemoteCursors] = useState<Record<string, { x: number; y: number }>>({});
  const [showAiPanel, setShowAiPanel] = useState(false);
  const [showVersionPanel, setShowVersionPanel] = useState(false);
  const [aiPrompt, setAiPrompt] = useState('');
  const [isAiProcessing, setIsAiProcessing] = useState(false);
  const [aiSuggestion, setAiSuggestion] = useState<{ suggestion: DesignerWorkflow; explanation: string } | null>(null);
  const [activeTool, setActiveTool] = useState<'select' | 'connect'>('select');
  const [isRunning, setIsRunning] = useState(false);
  const [isComparing, setIsComparing] = useState(false);
  const [showCompareModal, setShowCompareModal] = useState(false);
  const [isShadowMode, setIsShadowMode] = useState(false);
  const [shadowTraffic, setShadowTraffic] = useState(10);
  const [isMobilePaletteOpen, setIsMobilePaletteOpen] = useState(false);
  const [isMobilePropertiesOpen, setIsMobilePropertiesOpen] = useState(false);
  const [history, setHistory] = useState<DesignerWorkflow[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);

  const toDesignerWorkflow = (kernel: WOSKernelDocument): DesignerWorkflow => {
    const { stages, connections } = kernelToDesigner(kernel);
    return {
      id: kernel.url ?? 'wf-1',
      name: kernel.title ?? 'Untitled',
      version: kernel.version ?? '0.0.0',
      status: kernel.status === 'active' ? 'published' : kernel.status === 'retired' ? 'archived' : 'draft',
      stages, connections,
      lastModified: new Date().toISOString(),
      author: 'WOS Kernel',
    };
  };

  useEffect(() => {
    realtime.connect();
    realtime.onKernelInit((kernel) => {
      const wf = toDesignerWorkflow(kernel);
      setCurrentKernel(kernel);
      setWorkflow(wf);
      setHistory([wf]);
      setHistoryIndex(0);
      validateWorkflow(wf);
    });
    realtime.onKernelChanged((kernel) => {
      const wf = toDesignerWorkflow(kernel);
      setCurrentKernel(kernel);
      setWorkflow(wf);
      validateWorkflow(wf);
    });
    realtime.onCollaboratorsUpdate((users) => setCollaborators(users));
    realtime.onCursorUpdate((cursor) => setRemoteCursors(prev => ({ ...prev, [cursor.userId]: cursor.cursor })));
    return () => { realtime.disconnect(); };
  }, [realtime]);

  useEffect(() => {
    workflowDesign.loadKernel('https://agency.gov/workflows/benefits-adjudication').then(kernel => {
      if (kernel && !workflow) {
        setCurrentKernel(kernel);
        const wf = toDesignerWorkflow(kernel);
        setWorkflow(wf);
        setHistory([wf]);
        setHistoryIndex(0);
        validateWorkflow(wf);
      }
    });
  }, [workflowDesign]);

  const validateWorkflow = (wf: DesignerWorkflow) => {
    const issues: ValidationIssue[] = [];
    const startStage = wf.stages[0];
    if (startStage) {
      const reached = new Set([startStage.id]);
      let changed = true;
      while (changed) {
        changed = false;
        wf.connections.forEach(c => {
          if (reached.has(c.from) && !reached.has(c.to)) { reached.add(c.to); changed = true; }
        });
      }
      wf.stages.forEach(s => {
        if (!reached.has(s.id)) {
          issues.push({ id: `v-reach-${s.id}`, severity: 'warning', category: 'structure', message: `Stage "${s.name}" is unreachable.`, targetId: s.id });
        }
        const outgoing = wf.connections.filter(c => c.from === s.id);
        if (outgoing.length === 0 && s.type !== 'final') {
          issues.push({ id: `v-dead-${s.id}`, severity: 'error', category: 'structure', message: `Stage "${s.name}" is a dead end. Add an exit path.`, targetId: s.id });
        }
      });
    }
    const hasCycle = (current: string, visited: Set<string>, stack: Set<string>): boolean => {
      visited.add(current); stack.add(current);
      const neighbors = wf.connections.filter(c => c.from === current).map(c => c.to);
      for (const n of neighbors) {
        if (!visited.has(n)) { if (hasCycle(n, visited, stack)) return true; }
        else if (stack.has(n)) return true;
      }
      stack.delete(current); return false;
    };
    if (startStage && hasCycle(startStage.id, new Set(), new Set())) {
      issues.push({ id: 'v-cycle', severity: 'error', category: 'structure', message: 'Workflow contains a circular dependency.' });
    }
    setValidation({
      isValid: issues.filter(i => i.severity === 'error').length === 0,
      issues,
      status: { structure: issues.filter(i => i.category === 'structure').length === 0, policy: true, soundness: true, satisfiability: true }
    });
  };

  const updateWorkflow = (newWf: DesignerWorkflow, isRemote = false) => {
    const newHistory = history.slice(0, historyIndex + 1);
    newHistory.push(newWf);
    if (newHistory.length > MAX_HISTORY) newHistory.shift();
    setHistory(newHistory);
    setHistoryIndex(newHistory.length - 1);
    setWorkflow(newWf);
    validateWorkflow(newWf);
    if (!isRemote) {
      realtime.sendKernelUpdate(designerToKernel(newWf, currentKernel ?? undefined));
    }
  };

  const undo = () => {
    if (historyIndex > 0) {
      const newIndex = historyIndex - 1;
      setHistoryIndex(newIndex);
      const wf = history[newIndex];
      setWorkflow(wf);
      workflowDesign.validateKernel(designerToKernel(wf, currentKernel ?? undefined)).then(result => {
        setValidation(mapWosValidation(result));
      });
    }
  };

  const redo = () => {
    if (historyIndex < history.length - 1) {
      const newIndex = historyIndex + 1;
      setHistoryIndex(newIndex);
      const wf = history[newIndex];
      setWorkflow(wf);
      workflowDesign.validateKernel(designerToKernel(wf, currentKernel ?? undefined)).then(result => {
        setValidation(mapWosValidation(result));
      });
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') { e.shiftKey ? redo() : undo(); }
      else if ((e.ctrlKey || e.metaKey) && e.key === 'y') { redo(); }
      else if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedElement) {
          selectedElement.type === 'stage' ? handleDeleteStage(selectedElement.id) : handleDeleteConnection(selectedElement.id);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [historyIndex, history, selectedElement]);

  const handleDeleteStage = (id: string) => {
    if (!workflow) return;
    updateWorkflow({ ...workflow, stages: workflow.stages.filter(s => s.id !== id), connections: workflow.connections.filter(c => c.from !== id && c.to !== id) });
    setSelectedElement(null);
  };

  const handleDeleteConnection = (id: string) => {
    if (!workflow) return;
    updateWorkflow({ ...workflow, connections: workflow.connections.filter(c => c.id !== id) });
    setSelectedElement(null);
  };

  const handleSave = async () => {
    if (workflow) {
      const kernel = designerToKernel(workflow, currentKernel ?? undefined);
      await workflowDesign.saveKernel(kernel);
      const result = await workflowDesign.validateKernel(kernel);
      setValidation(mapWosValidation(result));
    }
  };

  const handleAiSubmit = async () => {
    if (workflow && aiPrompt) {
      setIsAiProcessing(true);
      try {
        const res = await fetch('/api/ai/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ contents: [{ parts: [{ text: aiPrompt }] }] }),
        });
        const data = await res.json();
        const text = data?.candidates?.[0]?.content?.parts?.[0]?.text;
        if (text) {
          try { setAiSuggestion(JSON.parse(text)); } catch { setAiSuggestion(null); }
        }
      } catch { setAiSuggestion(null); }
      finally { setIsAiProcessing(false); }
    }
  };

  const applyAiSuggestion = () => {
    if (aiSuggestion) { updateWorkflow(aiSuggestion.suggestion); setAiSuggestion(null); setAiPrompt(''); setShowAiPanel(false); }
  };

  if (!workflow) return <div className="p-8 text-center text-slate-500">Loading designer...</div>;

  return (
    <div className="flex flex-col h-full bg-[#f1f5f9] font-sans text-slate-900 overflow-hidden">
      <div className="relative h-20 bg-white border-b border-slate-200 shrink-0 z-20 shadow-sm overflow-hidden">
        <div className="absolute inset-y-0 right-0 w-8 bg-gradient-to-l from-white to-transparent z-30 pointer-events-none sm:hidden" />
        <div className="h-full flex items-center justify-between px-4 sm:px-8 overflow-x-auto no-scrollbar">
          <div className="flex items-center gap-3 sm:gap-6 min-w-max">
            <div className="flex items-center gap-2 sm:gap-4">
              <button onClick={() => setIsMobilePaletteOpen(!isMobilePaletteOpen)} className="lg:hidden p-2 bg-slate-100 rounded-xl text-slate-600 active:scale-95 transition-transform">
                <Layers className="w-5 h-5" />
              </button>
              <div className="w-10 h-10 bg-slate-900 rounded-2xl flex items-center justify-center text-white font-black text-xl shadow-lg shadow-slate-200 hidden sm:flex">W</div>
              <div className="min-w-0">
                <div className="flex items-center gap-2 sm:gap-3">
                  <h2 className="font-black text-sm sm:text-lg tracking-tight truncate max-w-[120px] sm:max-w-none">{workflow.name}</h2>
                  <span className="text-[8px] sm:text-[10px] font-black font-mono bg-slate-100 px-1.5 py-0.5 border border-slate-200 rounded-lg uppercase tracking-wider">v{workflow.version}</span>
                </div>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <span className="flex h-1 w-1 sm:h-1.5 sm:w-1.5 rounded-full bg-emerald-500"></span>
                  <span className="text-[8px] sm:text-[10px] font-black text-slate-400 uppercase tracking-[0.15em] hidden xs:inline">Live Production</span>
                </div>
              </div>
            </div>
            <div className="h-8 w-px bg-slate-100 mx-1 sm:mx-2 hidden sm:block"></div>
            <div className="flex items-center gap-2">
              <div className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-50 border border-blue-100 rounded-lg hidden sm:flex">
                <GitCommit className="w-3.5 h-3.5 text-blue-600" />
                <span className="text-[10px] font-black text-blue-700 uppercase tracking-widest">Auto-Connect Active</span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 sm:gap-3 ml-4 min-w-max">
            <button onClick={() => { setIsRunning(true); setTimeout(() => setIsRunning(false), 2000); }} disabled={isRunning} className="flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 bg-white border border-slate-200 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider text-slate-700 hover:bg-slate-50 transition-all active:scale-95 shadow-sm disabled:opacity-50">
              {isRunning ? <RefreshCw className="w-3.5 h-3.5 sm:w-4 h-4 animate-spin" /> : <Play className="w-3.5 h-3.5 sm:w-4 h-4" />}
              <span className="hidden xs:inline">{isRunning ? 'Running...' : 'Run Test'}</span>
            </button>
            <button onClick={() => setShowAiPanel(!showAiPanel)} className={`flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider transition-all active:scale-95 ${showAiPanel ? 'bg-indigo-600 text-white shadow-lg shadow-indigo-100' : 'bg-white border border-indigo-200 text-indigo-600 hover:bg-indigo-50'}`}>
              <Sparkles className="w-3.5 h-3.5 sm:w-4 h-4" /><span className="hidden xs:inline">AI Architect</span>
            </button>
            <button onClick={() => setIsShadowMode(!isShadowMode)} className={`flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider transition-all active:scale-95 ${isShadowMode ? 'bg-amber-100 text-amber-700 border-amber-200' : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50'}`}>
              <GitBranch className="w-3.5 h-3.5 sm:w-4 h-4" /><span className="hidden xs:inline">{isShadowMode ? 'Shadow Mode On' : 'Shadow Mode Off'}</span>
            </button>
            <button onClick={handleSave} className={`flex items-center gap-2 px-3 sm:px-6 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider shadow-lg transition-all active:scale-95 ${isShadowMode ? 'bg-amber-600 text-white shadow-amber-100 hover:bg-amber-700' : 'bg-slate-900 text-white shadow-slate-200 hover:bg-black'}`}>
              <Save className="w-3.5 h-3.5 sm:w-4 h-4" />
              <span className="hidden xs:inline">{isShadowMode ? 'Deploy Shadow' : 'Deploy Changes'}</span>
              <span className="xs:hidden">Deploy</span>
            </button>
            <button onClick={() => setIsMobilePropertiesOpen(!isMobilePropertiesOpen)} className="lg:hidden p-2 bg-slate-100 rounded-xl text-slate-600 active:scale-95 transition-transform"><Settings className="w-5 h-5" /></button>
          </div>
        </div>
      </div>

      <AnimatePresence>
        {(isMobilePaletteOpen || isMobilePropertiesOpen || showAiPanel || showVersionPanel) && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={() => { setIsMobilePaletteOpen(false); setIsMobilePropertiesOpen(false); setShowAiPanel(false); setShowVersionPanel(false); }} className="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-[40] lg:hidden" />
        )}
      </AnimatePresence>

      <div className="flex-1 flex overflow-hidden relative">
        <DesignerToolbar
          palette={PALETTE_ITEMS}
          patterns={PATTERNS}
          isOpen={isMobilePaletteOpen}
          onClose={() => setIsMobilePaletteOpen(false)}
          onAddStage={(type, label) => {
            if (!workflow) return;
            const newStage: WorkflowStage = { id: `stage-${Date.now()}`, name: `New ${label}`, type: type as any, position: { x: 100, y: 100 }, config: {} };
            updateWorkflow({ ...workflow, stages: [...workflow.stages, newStage] });
            if (window.innerWidth < 1024) setIsMobilePaletteOpen(false);
          }}
        />

        <div className={`flex-1 relative overflow-hidden bg-[radial-gradient(#cbd5e1_1px,transparent_1px)] [background-size:32px_32px] ${activeTool === 'connect' ? 'cursor-crosshair' : 'cursor-default'}`}>
          {isShadowMode && (
            <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 bg-amber-50 border border-amber-200 rounded-2xl shadow-xl p-4 flex flex-col items-center gap-3 w-[400px] max-w-[90vw]">
              <div className="flex items-center gap-2 text-amber-700 font-black text-[10px] uppercase tracking-widest"><div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />Shadow Mode Active</div>
              <p className="text-xs text-amber-600 text-center font-medium">Routing <span className="font-bold">{shadowTraffic}%</span> of live traffic to this version. Shadow outcomes will not affect real cases.</p>
              <input type="range" min="1" max="100" value={shadowTraffic} onChange={(e) => setShadowTraffic(parseInt(e.target.value))} className="w-full accent-amber-600" />
            </div>
          )}
          <DesignerCanvas
            workflow={workflow} selectedId={selectedElement?.id} onSelect={(type, id) => setSelectedElement({ type, id })} validation={validation} collaborators={collaborators} remoteCursors={remoteCursors}
            onUpdateStagePosition={(id, x, y) => { if (!workflow) return; updateWorkflow({ ...workflow, stages: workflow.stages.map(s => s.id === id ? { ...s, position: { x, y } } : s) }); }}
            onAddConnection={(from, to) => { if (!workflow) return; updateWorkflow({ ...workflow, connections: [...workflow.connections, { id: `conn-${Date.now()}`, from, to }] }); }}
            activeTool={activeTool}
          />
          <DesignerValidationDock validation={validation} onFocusElement={(id) => setSelectedElement({ type: 'stage', id })} />
        </div>

        <div className={`lg:w-80 bg-white border-l border-slate-200 flex flex-col shrink-0 z-[50] transition-all duration-300 ${isMobilePropertiesOpen ? 'fixed inset-y-0 right-0 w-full sm:w-80 shadow-2xl' : 'hidden lg:flex'}`}>
          <div className="p-6 border-b border-slate-50 flex items-center justify-between lg:hidden shrink-0">
            <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Properties</h3>
            <button onClick={() => setIsMobilePropertiesOpen(false)} className="p-2 bg-slate-100 text-slate-600 rounded-xl active:scale-90 transition-all"><X className="w-5 h-5" /></button>
          </div>
          <div className="flex-1 overflow-y-auto no-scrollbar">
            <AnimatePresence mode="wait">
              {selectedElement ? (
                <motion.div key={selectedElement.id} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 20 }} className="flex-1 flex flex-col">
                  <DesignerPropertiesPanel element={selectedElement} workflow={workflow} onUpdate={(updatedWf) => updateWorkflow(updatedWf)} onDelete={(id, type) => { type === 'stage' ? handleDeleteStage(id) : handleDeleteConnection(id); if (window.innerWidth < 1024) setIsMobilePropertiesOpen(false); }} />
                </motion.div>
              ) : (
                <div className="flex-1 flex flex-col items-center justify-center p-12 text-center">
                  <div className="w-16 h-16 bg-slate-50 rounded-3xl flex items-center justify-center mb-6"><MousePointer2 className="w-8 h-8 text-slate-200" /></div>
                  <h4 className="text-sm font-black text-slate-900 uppercase tracking-widest mb-2">No Selection</h4>
                  <p className="text-xs text-slate-400 leading-relaxed">Select a stage or connection to configure its properties</p>
                </div>
              )}
            </AnimatePresence>
          </div>
        </div>

        <AnimatePresence>
          {showAiPanel && (
            <motion.div initial={{ x: '100%' }} animate={{ x: 0 }} exit={{ x: '100%' }} className="fixed lg:absolute right-0 top-0 bottom-0 w-full sm:w-[420px] bg-white border-l border-slate-200 shadow-2xl z-[60] flex flex-col">
              <div className="p-6 sm:p-8 border-b border-slate-50 flex items-center justify-between bg-indigo-600 shrink-0">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-indigo-100"><Sparkles className="w-5 h-5" /><h3 className="font-black uppercase tracking-wider text-xs">AI Workflow Architect</h3></div>
                  <p className="text-[10px] text-indigo-200 font-medium">Describe changes in natural language</p>
                </div>
                <button onClick={() => setShowAiPanel(false)} className="p-2 bg-indigo-500/50 hover:bg-indigo-500 rounded-xl text-indigo-100 transition-all active:scale-90"><X className="w-6 h-6" /></button>
              </div>
              <div className="flex-1 p-8 overflow-y-auto space-y-8">
                <div className="space-y-4">
                  <label className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Natural Language Prompt</label>
                  <textarea value={aiPrompt} onChange={(e) => setAiPrompt(e.target.value)} placeholder="e.g., 'Add an appeal subprocess after the determination stage if the decision is denied'" className="w-full h-40 p-5 bg-slate-50 border border-slate-200 rounded-2xl text-sm font-medium focus:ring-2 focus:ring-indigo-500 outline-none resize-none transition-all" />
                  <button onClick={handleAiSubmit} disabled={isAiProcessing || !aiPrompt} className="w-full py-4 bg-indigo-600 text-white rounded-2xl font-black text-sm uppercase tracking-wider hover:bg-indigo-700 disabled:opacity-50 flex items-center justify-center gap-3 shadow-xl shadow-indigo-100 transition-all active:scale-95">
                    {isAiProcessing ? <RefreshCw className="w-5 h-5 animate-spin" /> : <Sparkles className="w-5 h-5" />}Generate Architecture
                  </button>
                </div>
                {aiSuggestion && (
                  <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
                    <div className="p-6 bg-slate-50 border border-slate-200 rounded-2xl">
                      <h4 className="text-[10px] font-black text-slate-900 uppercase tracking-widest mb-4">Proposed Logic</h4>
                      <p className="text-xs text-slate-600 leading-relaxed font-medium">{aiSuggestion.explanation}</p>
                    </div>
                    <div className="flex gap-3">
                      <button onClick={applyAiSuggestion} className="flex-1 py-3 bg-emerald-600 text-white rounded-xl text-xs font-black uppercase tracking-wider hover:bg-emerald-700 shadow-lg shadow-emerald-100 transition-all active:scale-95">Apply Suggestion</button>
                      <button onClick={() => setAiSuggestion(null)} className="flex-1 py-3 bg-white border border-slate-200 text-slate-600 rounded-xl text-xs font-black uppercase tracking-wider hover:bg-slate-50 transition-all active:scale-95">Discard</button>
                    </div>
                  </motion.div>
                )}
              </div>
            </motion.div>
          )}
          {showVersionPanel && (
            <motion.div initial={{ x: '100%' }} animate={{ x: 0 }} exit={{ x: '100%' }} className="fixed lg:absolute right-0 top-0 bottom-0 w-full sm:w-[420px] bg-white border-l border-slate-200 shadow-2xl z-[60] flex flex-col">
              <div className="p-6 sm:p-8 border-b border-slate-50 flex items-center justify-between shrink-0">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-slate-900"><History className="w-5 h-5" /><h3 className="font-black uppercase tracking-wider text-xs">Version History</h3></div>
                  <p className="text-[10px] text-slate-400 font-medium">Audit trail of all workflow changes</p>
                </div>
                <button onClick={() => setShowVersionPanel(false)} className="p-2 bg-slate-100 hover:bg-slate-200 rounded-xl text-slate-400 transition-all active:scale-90"><X className="w-6 h-6" /></button>
              </div>
              <div className="flex-1 p-6 overflow-y-auto space-y-4">
                <VersionItem version="1.2.0" date="Apr 1, 2026" author="Admin User" current />
                <VersionItem version="1.1.0" date="Mar 15, 2026" author="Admin User" />
                <VersionItem version="1.0.0" date="Feb 28, 2026" author="System" />
              </div>
              <div className="p-8 border-t border-slate-50 bg-slate-50/50">
                <button onClick={() => { setIsComparing(true); setTimeout(() => { setIsComparing(false); setShowCompareModal(true); }, 800); }} disabled={isComparing} className="w-full py-4 bg-slate-900 text-white rounded-2xl font-black text-sm uppercase tracking-wider hover:bg-black shadow-xl shadow-slate-200 transition-all active:scale-95 disabled:opacity-50 flex items-center justify-center gap-2">
                  {isComparing ? <RefreshCw className="w-4 h-4 animate-spin" /> : null}{isComparing ? 'Comparing...' : 'Compare Selected Versions'}
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      <AnimatePresence>
        {showCompareModal && (
          <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="absolute inset-0 bg-slate-900/40 backdrop-blur-sm" onClick={() => setShowCompareModal(false)} />
            <motion.div initial={{ opacity: 0, scale: 0.95, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }} exit={{ opacity: 0, scale: 0.95, y: 20 }} className="relative bg-white rounded-3xl shadow-2xl w-full max-w-3xl overflow-hidden flex flex-col max-h-[85vh]">
              <div className="p-6 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-blue-100 text-blue-600 rounded-xl"><GitMerge className="w-5 h-5" /></div>
                  <div><h3 className="font-black text-slate-900 uppercase tracking-wider text-sm">Version Comparison</h3><p className="text-[10px] text-slate-500 font-medium uppercase tracking-widest mt-0.5">v1.2.0 (Current) vs v1.1.0</p></div>
                </div>
                <button onClick={() => setShowCompareModal(false)} className="p-2 text-slate-400 hover:bg-slate-100 rounded-xl transition-colors"><X className="w-5 h-5" /></button>
              </div>
              <div className="flex-1 overflow-y-auto p-6 bg-slate-50/30">
                <div className="space-y-4">
                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-emerald-600 font-bold text-xs uppercase tracking-wider mb-3"><Plus className="w-4 h-4" /> Added Stages</div>
                    <div className="flex items-center gap-3 p-3 bg-emerald-50 border border-emerald-100 rounded-xl">
                      <div className="p-2 bg-white rounded-lg shadow-sm"><Cpu className="w-4 h-4 text-emerald-600" /></div>
                      <div><div className="text-xs font-bold text-slate-900">AI Document Extraction</div><div className="text-[10px] text-slate-500">Stage ID: stage-ai-extract</div></div>
                    </div>
                  </div>
                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-amber-600 font-bold text-xs uppercase tracking-wider mb-3"><RefreshCw className="w-4 h-4" /> Modified Connections</div>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between p-3 bg-amber-50 border border-amber-100 rounded-xl">
                        <div className="text-xs font-medium text-slate-700">Initial Review &rarr; Approval</div>
                        <ArrowRight className="w-4 h-4 text-amber-400 mx-2" />
                        <div className="text-xs font-bold text-slate-900">Initial Review &rarr; AI Document Extraction</div>
                      </div>
                    </div>
                  </div>
                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-rose-600 font-bold text-xs uppercase tracking-wider mb-3"><X className="w-4 h-4" /> Removed Stages</div>
                    <div className="p-3 bg-rose-50 border border-rose-100 rounded-xl text-xs text-rose-700 font-medium italic">No stages were removed in this version.</div>
                  </div>
                </div>
              </div>
              <div className="p-6 border-t border-slate-100 bg-white flex justify-end gap-3">
                <button onClick={() => setShowCompareModal(false)} className="px-6 py-3 bg-white border border-slate-200 text-slate-600 rounded-xl text-xs font-black uppercase tracking-wider hover:bg-slate-50 transition-all active:scale-95">Close</button>
                <button onClick={() => setShowCompareModal(false)} className="px-6 py-3 bg-blue-600 text-white rounded-xl text-xs font-black uppercase tracking-wider shadow-lg shadow-blue-200 hover:bg-blue-700 transition-all active:scale-95">Restore v1.1.0</button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}
