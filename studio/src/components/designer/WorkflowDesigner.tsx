import React, { useState, useEffect, useRef, memo, useCallback, useMemo } from 'react';
import { 
  Settings, 
  Play, 
  Save, 
  History, 
  Sparkles, 
  AlertCircle, 
  CheckCircle2, 
  Info, 
  ChevronRight, 
  Plus, 
  MousePointer2, 
  ArrowUpRight,
  Layers,
  Cpu,
  RefreshCw,
  GitBranch,
  Search,
  X,
  Clock,
  User,
  Split,
  Merge,
  GitMerge,
  GitCommit,
  Timer,
  Webhook,
  Network,
  Maximize2,
  Minimize2,
  Map as MapIcon,
  ArrowRight
} from 'lucide-react';
import { useWorkflowDesign, useRealtime } from '../../context/WosContext';
import { kernelToDesigner, designerToKernel, type KernelToDesignerResult, type WorkflowStage, type WorkflowConnection, type DesignerWorkflow } from '../../services/KernelToDesigner';
import type { WosValidationResult } from '../../services/WosPorts';
import type { WOSKernelDocument } from '../../types/wos/kernel';
import { motion, AnimatePresence } from 'motion/react';

interface PaletteItemData {
  id: string;
  label: string;
  icon: string;
  color: string;
  description?: string;
}

interface PatternItemData {
  id: string;
  label: string;
}

interface ValidationIssue {
  id: string;
  severity: 'error' | 'warning';
  category: 'structure' | 'policy' | 'soundness' | 'satisfiability';
  message: string;
  targetId?: string;
}

interface WorkflowValidation {
  isValid: boolean;
  issues: ValidationIssue[];
  status: {
    structure: boolean;
    policy: boolean;
    soundness: boolean;
    satisfiability: boolean;
  };
}

function mapWosValidation(result: WosValidationResult): WorkflowValidation {
  const issues: ValidationIssue[] = result.issues.map((issue, i) => ({
    id: `v-port-${i}`,
    ...issue,
  }));
  return {
    isValid: result.isValid,
    issues,
    status: {
      structure: issues.filter(i => i.category === 'structure').every(i => i.severity !== 'error'),
      policy: issues.filter(i => i.category === 'policy').every(i => i.severity !== 'error'),
      soundness: issues.filter(i => i.category === 'soundness').every(i => i.severity !== 'error'),
      satisfiability: issues.filter(i => i.category === 'satisfiability').every(i => i.severity !== 'error'),
    },
  };
}

const StageNode = memo(({ 
  stage, 
  isSelected, 
  activeTool, 
  validation, 
  onPointerDown, 
  onPointerUp,
  onStartConnection
}: { 
  stage: WorkflowStage; 
  isSelected: boolean; 
  activeTool: string; 
  validation: WorkflowValidation | null;
  onPointerDown: (e: React.PointerEvent, id: string) => void;
  onPointerUp: (e: React.PointerEvent, id: string) => void;
  onStartConnection: (id: string) => void;
}) => {
  return (
    <motion.div
      layoutId={stage.id}
      initial={false}
      tabIndex={0}
      role="button"
      aria-label={`Workflow stage: ${stage.name}`}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          onPointerDown(e as any, stage.id);
        }
      }}
      style={{ left: stage.position.x, top: stage.position.y }}
      className={`absolute w-[180px] min-h-[80px] bg-white border-2 rounded-xl shadow-sm group transition-all select-none ${activeTool === 'connect' ? 'cursor-crosshair' : 'cursor-move'} ${isSelected ? 'border-blue-600 ring-4 ring-blue-50 shadow-xl z-10' : 'border-[#141414] hover:shadow-md'}`}
      onPointerDown={(e) => onPointerDown(e, stage.id)}
      onPointerUp={(e) => onPointerUp(e, stage.id)}
    >
      <div className={`px-3 py-2 border-b-2 border-[#141414] flex items-center justify-between rounded-t-[10px] ${getStageColor(stage.type)}`}>
        <div className="flex items-center gap-2">
          {getStageIcon(stage.type)}
          <span className="text-[10px] font-bold uppercase tracking-wider truncate max-w-[100px]">{stage.name}</span>
        </div>
        {validation?.issues.some(i => i.targetId === stage.id) && (
          <AlertCircle className="w-3.5 h-3.5 text-amber-500" />
        )}
      </div>
      <div className="p-3">
        {stage.description && (
          <p className="text-[9px] text-slate-500 mb-2 line-clamp-2 leading-tight">{stage.description}</p>
        )}
        {stage.config.assignee && (
          <div className="mb-2 flex items-center gap-1.5 px-2 py-1 bg-slate-50 border border-slate-100 rounded-lg">
            {stage.config.assignee.type === 'agent' ? <Sparkles className="w-3 h-3 text-purple-500" /> : <User className="w-3 h-3 text-blue-500" />}
            <span className="text-[9px] font-bold text-slate-600 truncate">{stage.config.assignee.label}</span>
          </div>
        )}
        {stage.type === 'ai-pipeline' && (
          <div className="space-y-1.5">
            {stage.config.steps?.map((step: string, i: number) => (
              <div key={i} className="flex items-center gap-2">
                <div className="w-1.5 h-1.5 rounded-full bg-purple-400"></div>
                <span className="text-[9px] font-medium text-gray-600 truncate">{step}</span>
              </div>
            ))}
          </div>
        )}
        {stage.type === 'adaptive' && (
          <div className="flex flex-wrap gap-1">
            {stage.config.activities?.map((act: string, i: number) => (
              <span key={i} className="text-[8px] font-bold bg-amber-50 text-amber-700 border border-amber-100 px-1 rounded">
                {act}
              </span>
            ))}
          </div>
        )}
        {stage.type === 'simple' && (
          <p className="text-[9px] text-gray-400 italic">Human review task</p>
        )}
        {stage.type === 'decision' && (
          <div className="flex flex-col gap-1">
            <div className="text-[8px] font-bold text-orange-600 bg-orange-50 px-1.5 py-0.5 rounded border border-orange-100 w-fit">Rules Engine</div>
            <p className="text-[9px] text-gray-400 italic truncate">Evaluates case data</p>
          </div>
        )}
        {stage.type === 'timer' && (
          <div className="flex flex-col gap-1">
            <div className="text-[8px] font-bold text-rose-600 bg-rose-50 px-1.5 py-0.5 rounded border border-rose-100 w-fit">Escalation</div>
            <p className="text-[9px] text-gray-400 italic truncate">3 Days</p>
          </div>
        )}
        {stage.type === 'api' && (
          <div className="flex flex-col gap-1">
            <div className="text-[8px] font-bold text-indigo-600 bg-indigo-50 px-1.5 py-0.5 rounded border border-indigo-100 w-fit">Webhook</div>
            <p className="text-[9px] text-gray-400 italic truncate">POST /api/notify</p>
          </div>
        )}
        {stage.type === 'split' && (
          <p className="text-[9px] text-gray-400 italic">Parallel branches</p>
        )}
        {stage.type === 'join' && (
          <p className="text-[9px] text-gray-400 italic">Wait for all</p>
        )}
      </div>
      
      {/* Connection Points */}
      <div className="absolute -left-1.5 top-1/2 -translate-y-1/2 w-3 h-3 bg-white border-2 border-slate-300 rounded-full opacity-0 group-hover:opacity-100 transition-opacity z-10"></div>
      <div 
        className="absolute -right-1.5 top-1/2 -translate-y-1/2 w-4 h-4 bg-blue-600 border-2 border-white rounded-full opacity-0 group-hover:opacity-100 transition-all cursor-crosshair z-10 hover:scale-125 shadow-sm"
        onPointerDown={(e) => {
          e.stopPropagation();
          onStartConnection(stage.id);
        }}
        title="Drag to connect"
      ></div>
    </motion.div>
  );
});

const ConnectionLine = memo(({ 
  conn, 
  fromStage, 
  toStage, 
  isSelected, 
  onSelect 
}: { 
  conn: WorkflowConnection; 
  fromStage: WorkflowStage; 
  toStage: WorkflowStage; 
  isSelected: boolean;
  onSelect: (id: string) => void;
}) => {
  const startX = fromStage.position.x + 180;
  const startY = fromStage.position.y + 40;
  const endX = toStage.position.x;
  const endY = toStage.position.y + 40;
  
  return (
    <g className="cursor-pointer pointer-events-auto group" onClick={(e) => { e.stopPropagation(); onSelect(conn.id); }}>
      <path 
        d={`M ${startX} ${startY} C ${startX + 50} ${startY}, ${endX - 50} ${endY}, ${endX} ${endY}`} 
        fill="none" 
        stroke={isSelected ? '#2563eb' : '#141414'} 
        strokeWidth={isSelected ? 4 : 2}
        markerEnd="url(#arrowhead)"
        className="transition-all group-hover:stroke-blue-400"
      />
      {/* Invisible wider path for easier clicking */}
      <path 
        d={`M ${startX} ${startY} C ${startX + 50} ${startY}, ${endX - 50} ${endY}, ${endX} ${endY}`} 
        fill="none" 
        stroke="transparent" 
        strokeWidth={15}
      />
      {conn.condition && (
        <foreignObject x={(startX + endX) / 2 - 40} y={(startY + endY) / 2 - 10} width="80" height="20">
          <div className="bg-white border border-gray-200 rounded px-1.5 py-0.5 text-[9px] font-bold text-gray-500 truncate text-center shadow-sm">
            {conn.condition}
          </div>
        </foreignObject>
      )}
    </g>
  );
});

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
  const [palette] = useState<PaletteItemData[]>([
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
  ]);
  const [patterns] = useState<PatternItemData[]>([]);
  const [activeTool, setActiveTool] = useState<'select' | 'connect'>('select');
  const [isRunning, setIsRunning] = useState(false);
  const [isComparing, setIsComparing] = useState(false);
  const [showCompareModal, setShowCompareModal] = useState(false);
  const [isShadowMode, setIsShadowMode] = useState(false);
  const [shadowTraffic, setShadowTraffic] = useState(10);
  const [isMobilePaletteOpen, setIsMobilePaletteOpen] = useState(false);
  const [isMobilePropertiesOpen, setIsMobilePropertiesOpen] = useState(false);
  const [hoveredPaletteItem, setHoveredPaletteItem] = useState<{ id: string; description: string; rect: DOMRect } | null>(null);

  const [history, setHistory] = useState<DesignerWorkflow[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);

  useEffect(() => {
    realtime.connect();

    const toDesignerWorkflow = (kernel: WOSKernelDocument): DesignerWorkflow => {
      const { stages, connections } = kernelToDesigner(kernel);
      return {
        id: kernel.url ?? 'wf-1',
        name: kernel.title ?? 'Untitled',
        version: kernel.version ?? '0.0.0',
        status: kernel.status === 'active' ? 'published' : kernel.status === 'retired' ? 'archived' : 'draft',
        stages,
        connections,
        lastModified: new Date().toISOString(),
        author: 'WOS Kernel',
      };
    };

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

    realtime.onCollaboratorsUpdate((users) => {
      setCollaborators(users);
    });

    realtime.onCursorUpdate((cursor) => {
      setRemoteCursors(prev => ({ ...prev, [cursor.userId]: cursor.cursor }));
    });

    return () => {
      realtime.disconnect();
    };
  }, [realtime]);

  useEffect(() => {
    workflowDesign.loadKernel('https://agency.gov/workflows/benefits-adjudication').then(kernel => {
      if (kernel && !workflow) {
        setCurrentKernel(kernel);
        const { stages, connections } = kernelToDesigner(kernel);
        const wf: DesignerWorkflow = {
          id: kernel.url ?? 'wf-1',
          name: kernel.title ?? 'Untitled',
          version: kernel.version ?? '0.0.0',
          status: kernel.status === 'active' ? 'published' : kernel.status === 'retired' ? 'archived' : 'draft',
          stages,
          connections,
          lastModified: new Date().toISOString(),
          author: 'WOS Kernel',
        };
        setWorkflow(wf);
        setHistory([wf]);
        setHistoryIndex(0);
        validateWorkflow(wf);
      }
    });
  }, [workflowDesign]);

  const validateWorkflow = (wf: DesignerWorkflow) => {
    const issues: ValidationIssue[] = [];
    
    // Check for unreachable stages (except start)
    const startStage = wf.stages[0];
    if (startStage) {
      const reached = new Set([startStage.id]);
      let changed = true;
      while (changed) {
        changed = false;
        wf.connections.forEach(c => {
          if (reached.has(c.from) && !reached.has(c.to)) {
            reached.add(c.to);
            changed = true;
          }
        });
      }
      wf.stages.forEach(s => {
        if (!reached.has(s.id)) {
          issues.push({ id: `v-reach-${s.id}`, severity: 'warning', category: 'structure', message: `Stage "${s.name}" is unreachable.`, targetId: s.id });
        }
        
        // Check for dead ends (no outgoing connections for non-final stages)
        const outgoing = wf.connections.filter(c => c.from === s.id);
        if (outgoing.length === 0 && s.type !== 'final') {
          issues.push({ id: `v-dead-${s.id}`, severity: 'error', category: 'structure', message: `Stage "${s.name}" is a dead end. Add an exit path.`, targetId: s.id });
        }
      });
    }

    // Check for cycles
    const hasCycle = (current: string, visited: Set<string>, stack: Set<string>): boolean => {
      visited.add(current);
      stack.add(current);
      const neighbors = wf.connections.filter(c => c.from === current).map(c => c.to);
      for (const n of neighbors) {
        if (!visited.has(n)) {
          if (hasCycle(n, visited, stack)) return true;
        } else if (stack.has(n)) {
          return true;
        }
      }
      stack.delete(current);
      return false;
    };

    if (startStage) {
      if (hasCycle(startStage.id, new Set(), new Set())) {
        issues.push({ id: 'v-cycle', severity: 'error', category: 'structure', message: 'Workflow contains a circular dependency.' });
      }
    }

    setValidation({
      isValid: issues.filter(i => i.severity === 'error').length === 0,
      issues,
      status: {
        structure: issues.filter(i => i.category === 'structure').length === 0,
        policy: true,
        soundness: true,
        satisfiability: true
      }
    });
  };

  const updateWorkflow = (newWf: DesignerWorkflow, isRemote = false) => {
    const newHistory = history.slice(0, historyIndex + 1);
    newHistory.push(newWf);
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
      if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
        redo();
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedElement) {
          if (selectedElement.type === 'stage') {
            handleDeleteStage(selectedElement.id);
          } else {
            handleDeleteConnection(selectedElement.id);
          }
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [historyIndex, history, selectedElement]);

  const handleDeleteStage = (id: string) => {
    if (!workflow) return;
    const newStages = workflow.stages.filter(s => s.id !== id);
    const newConnections = workflow.connections.filter(c => c.from !== id && c.to !== id);
    updateWorkflow({ ...workflow, stages: newStages, connections: newConnections });
    setSelectedElement(null);
  };

  const handleDeleteConnection = (id: string) => {
    if (!workflow) return;
    const newConnections = workflow.connections.filter(c => c.id !== id);
    updateWorkflow({ ...workflow, connections: newConnections });
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
      setTimeout(() => setIsAiProcessing(false), 1000);
    }
  };

  const applyAiSuggestion = () => {
    if (aiSuggestion) {
      updateWorkflow(aiSuggestion.suggestion);
      setAiSuggestion(null);
      setAiPrompt('');
      setShowAiPanel(false);
    }
  };

  if (!workflow) return <div className="p-8 text-center text-slate-500">Loading designer...</div>;

  const ICON_MAP: Record<string, any> = {
    'Plus': Plus,
    'Layers': Layers,
    'RefreshCw': RefreshCw,
    'Cpu': Cpu,
    'CheckCircle2': CheckCircle2,
    'Split': Split,
    'Merge': Merge,
    'GitMerge': GitMerge,
    'Timer': Timer,
    'Webhook': Webhook,
    'Network': Network
  };

  return (
    <div className="flex flex-col h-full bg-[#f1f5f9] font-sans text-slate-900 overflow-hidden">
      {/* Workbench Header */}
      <div className="relative h-20 bg-white border-b border-slate-200 shrink-0 z-20 shadow-sm overflow-hidden">
        <div className="absolute inset-y-0 right-0 w-8 bg-gradient-to-l from-white to-transparent z-30 pointer-events-none sm:hidden" />
        <div className="h-full flex items-center justify-between px-4 sm:px-8 overflow-x-auto no-scrollbar">
          <div className="flex items-center gap-3 sm:gap-6 min-w-max">
            <div className="flex items-center gap-2 sm:gap-4">
              <button 
                onClick={() => setIsMobilePaletteOpen(!isMobilePaletteOpen)}
                className="lg:hidden p-2 bg-slate-100 rounded-xl text-slate-600 active:scale-95 transition-transform"
              >
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
            <button 
              onClick={() => {
                setIsRunning(true);
                setTimeout(() => setIsRunning(false), 2000);
              }}
              disabled={isRunning}
              className="flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 bg-white border border-slate-200 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider text-slate-700 hover:bg-slate-50 transition-all active:scale-95 shadow-sm disabled:opacity-50"
            >
              {isRunning ? <RefreshCw className="w-3.5 h-3.5 sm:w-4 h-4 animate-spin" /> : <Play className="w-3.5 h-3.5 sm:w-4 h-4" />}
              <span className="hidden xs:inline">{isRunning ? 'Running...' : 'Run Test'}</span>
            </button>
            <button 
              onClick={() => setShowAiPanel(!showAiPanel)}
              className={`flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider transition-all active:scale-95 ${showAiPanel ? 'bg-indigo-600 text-white shadow-lg shadow-indigo-100' : 'bg-white border border-indigo-200 text-indigo-600 hover:bg-indigo-50'}`}
            >
              <Sparkles className="w-3.5 h-3.5 sm:w-4 h-4" />
              <span className="hidden xs:inline">AI Architect</span>
            </button>
            <button 
              onClick={() => setIsShadowMode(!isShadowMode)}
              className={`flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider transition-all active:scale-95 ${isShadowMode ? 'bg-amber-100 text-amber-700 border-amber-200' : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50'}`}
            >
              <GitBranch className="w-3.5 h-3.5 sm:w-4 h-4" />
              <span className="hidden xs:inline">{isShadowMode ? 'Shadow Mode On' : 'Shadow Mode Off'}</span>
            </button>
            <button 
              onClick={handleSave}
              className={`flex items-center gap-2 px-3 sm:px-6 py-2 sm:py-2.5 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider shadow-lg transition-all active:scale-95 ${isShadowMode ? 'bg-amber-600 text-white shadow-amber-100 hover:bg-amber-700' : 'bg-slate-900 text-white shadow-slate-200 hover:bg-black'}`}
            >
              <Save className="w-3.5 h-3.5 sm:w-4 h-4" />
              <span className="hidden xs:inline">{isShadowMode ? 'Deploy Shadow' : 'Deploy Changes'}</span>
              <span className="xs:hidden">Deploy</span>
            </button>
            <button 
              onClick={() => setIsMobilePropertiesOpen(!isMobilePropertiesOpen)}
              className="lg:hidden p-2 bg-slate-100 rounded-xl text-slate-600 active:scale-95 transition-transform"
            >
              <Settings className="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>

      {/* Mobile Backdrops (Moved to top level for full coverage) */}
      <AnimatePresence>
        {(isMobilePaletteOpen || isMobilePropertiesOpen || showAiPanel || showVersionPanel) && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => {
              setIsMobilePaletteOpen(false);
              setIsMobilePropertiesOpen(false);
              setShowAiPanel(false);
              setShowVersionPanel(false);
            }}
            className="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-[40] lg:hidden"
          />
        )}
      </AnimatePresence>

      <div className="flex-1 flex overflow-hidden relative">

      {/* Left Palette */}
      <div className={`lg:w-72 bg-white border-r border-slate-200 flex flex-col shrink-0 z-[60] transition-all duration-300 ${isMobilePaletteOpen ? 'fixed inset-y-0 left-0 w-full sm:w-72 shadow-2xl' : 'hidden lg:flex'}`}>
        <div className="p-6 border-b border-slate-50 flex items-center justify-between lg:block shrink-0">
          <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-0 lg:mb-6">Stage Components</h3>
          <button 
            onClick={() => setIsMobilePaletteOpen(false)} 
            className="lg:hidden p-2 bg-slate-100 text-slate-600 rounded-xl active:scale-90 transition-all"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto no-scrollbar">
          <div className="p-6 border-b border-slate-50">
            <div className="grid grid-cols-2 gap-3">
              {palette.map(item => {
                const Icon = ICON_MAP[item.icon] || Plus;
                return (
                  <PaletteItem 
                    key={item.id}
                    icon={<Icon className="w-5 h-5" />} 
                    label={item.label} 
                    color={item.color} 
                    onHover={(rect) => setHoveredPaletteItem({ id: item.id, description: item.description || '', rect })}
                    onLeave={() => setHoveredPaletteItem(null)}
                    onClick={() => {
                      if (!workflow) return;
                      const newStage: WorkflowStage = {
                        id: `stage-${Date.now()}`,
                        name: `New ${item.label}`,
                        type: item.id as any,
                        position: { x: 100, y: 100 },
                        config: {}
                      };
                      updateWorkflow({ ...workflow, stages: [...workflow.stages, newStage] });
                      if (window.innerWidth < 1024) {
                        setIsMobilePaletteOpen(false);
                      }
                    }}
                  />
                );
              })}
            </div>
          </div>
          <div className="p-6">
            <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-6">Workflow Patterns</h3>
            <div className="space-y-3">
              {patterns.map(pattern => (
                <PatternItem key={pattern.id} label={pattern.label} />
              ))}
            </div>
          </div>
        </div>
      </div>

        {/* Canvas Area */}
        <div className={`flex-1 relative overflow-hidden bg-[radial-gradient(#cbd5e1_1px,transparent_1px)] [background-size:32px_32px] ${activeTool === 'connect' ? 'cursor-crosshair' : 'cursor-default'}`}>
          {isShadowMode && (
            <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 bg-amber-50 border border-amber-200 rounded-2xl shadow-xl p-4 flex flex-col items-center gap-3 w-[400px] max-w-[90vw]">
              <div className="flex items-center gap-2 text-amber-700 font-black text-[10px] uppercase tracking-widest">
                <div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
                Shadow Mode Active
              </div>
              <p className="text-xs text-amber-600 text-center font-medium">
                Routing <span className="font-bold">{shadowTraffic}%</span> of live traffic to this version. Shadow outcomes will not affect real cases.
              </p>
              <input 
                type="range" 
                min="1" 
                max="100" 
                value={shadowTraffic}
                onChange={(e) => setShadowTraffic(parseInt(e.target.value))}
                className="w-full accent-amber-600"
              />
            </div>
          )}

          <DesignerCanvas 
            workflow={workflow} 
            selectedId={selectedElement?.id} 
            onSelect={(type, id) => setSelectedElement({ type, id })}
            validation={validation}
            collaborators={collaborators}
            remoteCursors={remoteCursors}
            onUpdateStagePosition={(id, x, y) => {
              if (!workflow) return;
              const newStages = workflow.stages.map(s => s.id === id ? { ...s, position: { x, y } } : s);
              updateWorkflow({ ...workflow, stages: newStages });
            }}
            onAddConnection={(from, to) => {
              if (!workflow) return;
              const newConn: WorkflowConnection = {
                id: `conn-${Date.now()}`,
                from,
                to
              };
              updateWorkflow({ ...workflow, connections: [...workflow.connections, newConn] });
            }}
            activeTool={activeTool}
          />
          
          {/* Validation Panel (Docked Bottom) */}
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
                    onClick={() => issue.targetId && setSelectedElement({ type: 'stage', id: issue.targetId })}
                  >
                    {issue.severity === 'error' ? <AlertCircle className="w-4 h-4 text-rose-500" /> : <Info className="w-4 h-4 text-amber-500" />}
                    <span className="text-xs font-bold text-slate-700">{issue.message}</span>
                    <span className="text-[10px] font-black text-slate-400 ml-auto opacity-0 group-hover:opacity-100 uppercase tracking-widest">Focus Element</span>
                  </motion.div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right Sidebar (Contextual) */}
        <div className={`lg:w-80 bg-white border-l border-slate-200 flex flex-col shrink-0 z-[50] transition-all duration-300 ${isMobilePropertiesOpen ? 'fixed inset-y-0 right-0 w-full sm:w-80 shadow-2xl' : 'hidden lg:flex'}`}>
          <div className="p-6 border-b border-slate-50 flex items-center justify-between lg:hidden shrink-0">
            <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Properties</h3>
            <button 
              onClick={() => setIsMobilePropertiesOpen(false)} 
              className="p-2 bg-slate-100 text-slate-600 rounded-xl active:scale-90 transition-all"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto no-scrollbar">
            <AnimatePresence mode="wait">
              {selectedElement ? (
                <motion.div
                  key={selectedElement.id}
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 20 }}
                  className="flex-1 flex flex-col"
                >
                  <PropertiesPanel 
                    element={selectedElement} 
                    workflow={workflow} 
                    onUpdate={(updatedWf) => updateWorkflow(updatedWf)} 
                    onDelete={(id, type) => {
                      if (type === 'stage') {
                        handleDeleteStage(id);
                      } else {
                        handleDeleteConnection(id);
                      }
                      if (window.innerWidth < 1024) {
                        setIsMobilePropertiesOpen(false);
                      }
                    }}
                  />
                </motion.div>
              ) : (
                <div className="flex-1 flex flex-col items-center justify-center p-12 text-center">
                  <div className="w-16 h-16 bg-slate-50 rounded-3xl flex items-center justify-center mb-6">
                    <MousePointer2 className="w-8 h-8 text-slate-200" />
                  </div>
                  <h4 className="text-sm font-black text-slate-900 uppercase tracking-widest mb-2">No Selection</h4>
                  <p className="text-xs text-slate-400 leading-relaxed">Select a stage or connection to configure its properties</p>
                </div>
              )}
            </AnimatePresence>
          </div>
        </div>

        <AnimatePresence>
          {showAiPanel && (
            <motion.div 
              initial={{ x: '100%' }} animate={{ x: 0 }} exit={{ x: '100%' }}
              className="fixed lg:absolute right-0 top-0 bottom-0 w-full sm:w-[420px] bg-white border-l border-slate-200 shadow-2xl z-[60] flex flex-col"
            >
              <div className="p-6 sm:p-8 border-b border-slate-50 flex items-center justify-between bg-indigo-600 shrink-0">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-indigo-100">
                    <Sparkles className="w-5 h-5" />
                    <h3 className="font-black uppercase tracking-wider text-xs">AI Workflow Architect</h3>
                  </div>
                  <p className="text-[10px] text-indigo-200 font-medium">Describe changes in natural language</p>
                </div>
                <button 
                  onClick={() => setShowAiPanel(false)} 
                  className="p-2 bg-indigo-500/50 hover:bg-indigo-500 rounded-xl text-indigo-100 transition-all active:scale-90"
                >
                  <X className="w-6 h-6" />
                </button>
              </div>
              <div className="flex-1 p-8 overflow-y-auto space-y-8">
                <div className="space-y-4">
                  <label className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Natural Language Prompt</label>
                  <textarea 
                    value={aiPrompt}
                    onChange={(e) => setAiPrompt(e.target.value)}
                    placeholder="e.g., 'Add an appeal subprocess after the determination stage if the decision is denied'"
                    className="w-full h-40 p-5 bg-slate-50 border border-slate-200 rounded-2xl text-sm font-medium focus:ring-2 focus:ring-indigo-500 outline-none resize-none transition-all"
                  />
                  <button 
                    onClick={handleAiSubmit}
                    disabled={isAiProcessing || !aiPrompt}
                    className="w-full py-4 bg-indigo-600 text-white rounded-2xl font-black text-sm uppercase tracking-wider hover:bg-indigo-700 disabled:opacity-50 flex items-center justify-center gap-3 shadow-xl shadow-indigo-100 transition-all active:scale-95"
                  >
                    {isAiProcessing ? <RefreshCw className="w-5 h-5 animate-spin" /> : <Sparkles className="w-5 h-5" />}
                    Generate Architecture
                  </button>
                </div>

                {aiSuggestion && (
                  <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
                    <div className="p-6 bg-slate-50 border border-slate-200 rounded-2xl">
                      <h4 className="text-[10px] font-black text-slate-900 uppercase tracking-widest mb-4">Proposed Logic</h4>
                      <p className="text-xs text-slate-600 leading-relaxed font-medium">{aiSuggestion.explanation}</p>
                    </div>
                    <div className="flex gap-3">
                      <button 
                        onClick={applyAiSuggestion}
                        className="flex-1 py-3 bg-emerald-600 text-white rounded-xl text-xs font-black uppercase tracking-wider hover:bg-emerald-700 shadow-lg shadow-emerald-100 transition-all active:scale-95"
                      >
                        Apply Suggestion
                      </button>
                      <button 
                        onClick={() => setAiSuggestion(null)}
                        className="flex-1 py-3 bg-white border border-slate-200 text-slate-600 rounded-xl text-xs font-black uppercase tracking-wider hover:bg-slate-50 transition-all active:scale-95"
                      >
                        Discard
                      </button>
                    </div>
                  </motion.div>
                )}
              </div>
            </motion.div>
          )}

          {showVersionPanel && (
            <motion.div 
              initial={{ x: '100%' }} animate={{ x: 0 }} exit={{ x: '100%' }}
              className="fixed lg:absolute right-0 top-0 bottom-0 w-full sm:w-[420px] bg-white border-l border-slate-200 shadow-2xl z-[60] flex flex-col"
            >
              <div className="p-6 sm:p-8 border-b border-slate-50 flex items-center justify-between shrink-0">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 text-slate-900">
                    <History className="w-5 h-5" />
                    <h3 className="font-black uppercase tracking-wider text-xs">Version History</h3>
                  </div>
                  <p className="text-[10px] text-slate-400 font-medium">Audit trail of all workflow changes</p>
                </div>
                <button 
                  onClick={() => setShowVersionPanel(false)} 
                  className="p-2 bg-slate-100 hover:bg-slate-200 rounded-xl text-slate-400 transition-all active:scale-90"
                >
                  <X className="w-6 h-6" />
                </button>
              </div>
              <div className="flex-1 p-6 overflow-y-auto space-y-4">
                <VersionItem version="1.2.0" date="Apr 1, 2026" author="Admin User" current />
                <VersionItem version="1.1.0" date="Mar 15, 2026" author="Admin User" />
                <VersionItem version="1.0.0" date="Feb 28, 2026" author="System" />
              </div>
              <div className="p-8 border-t border-slate-50 bg-slate-50/50">
                <button 
                  onClick={() => {
                    setIsComparing(true);
                    setTimeout(() => {
                      setIsComparing(false);
                      setShowCompareModal(true);
                    }, 800);
                  }}
                  disabled={isComparing}
                  className="w-full py-4 bg-slate-900 text-white rounded-2xl font-black text-sm uppercase tracking-wider hover:bg-black shadow-xl shadow-slate-200 transition-all active:scale-95 disabled:opacity-50 flex items-center justify-center gap-2"
                >
                  {isComparing ? <RefreshCw className="w-4 h-4 animate-spin" /> : null}
                  {isComparing ? 'Comparing...' : 'Compare Selected Versions'}
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Shared Palette Tooltip */}
      <AnimatePresence>
        {hoveredPaletteItem && (
          <motion.div
            initial={{ opacity: 0, x: -10, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: -10, scale: 0.95 }}
            transition={{ delay: 0.4, duration: 0.15 }}
            style={{ 
              position: 'fixed',
              top: hoveredPaletteItem.rect.top,
              left: hoveredPaletteItem.rect.right + 12,
            }}
            className="w-52 p-3 bg-slate-900 text-white text-[10px] rounded-xl shadow-2xl z-[9999] pointer-events-none hidden lg:block border border-slate-800"
          >
            <div className="absolute left-0 top-6 -ml-1 w-2 h-2 bg-slate-900 rotate-45 border-l border-b border-slate-800" />
            <p className="font-medium leading-relaxed text-slate-200">{hoveredPaletteItem.description}</p>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Compare Versions Modal */}
      <AnimatePresence>
        {showCompareModal && (
          <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
            <motion.div 
              initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
              className="absolute inset-0 bg-slate-900/40 backdrop-blur-sm"
              onClick={() => setShowCompareModal(false)}
            />
            <motion.div 
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="relative bg-white rounded-3xl shadow-2xl w-full max-w-3xl overflow-hidden flex flex-col max-h-[85vh]"
            >
              <div className="p-6 border-b border-slate-100 flex items-center justify-between bg-slate-50/50">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-blue-100 text-blue-600 rounded-xl">
                    <GitMerge className="w-5 h-5" />
                  </div>
                  <div>
                    <h3 className="font-black text-slate-900 uppercase tracking-wider text-sm">Version Comparison</h3>
                    <p className="text-[10px] text-slate-500 font-medium uppercase tracking-widest mt-0.5">v1.2.0 (Current) vs v1.1.0</p>
                  </div>
                </div>
                <button onClick={() => setShowCompareModal(false)} className="p-2 text-slate-400 hover:bg-slate-100 rounded-xl transition-colors">
                  <X className="w-5 h-5" />
                </button>
              </div>
              
              <div className="flex-1 overflow-y-auto p-6 bg-slate-50/30">
                <div className="space-y-4">
                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-emerald-600 font-bold text-xs uppercase tracking-wider mb-3">
                      <Plus className="w-4 h-4" /> Added Stages
                    </div>
                    <div className="flex items-center gap-3 p-3 bg-emerald-50 border border-emerald-100 rounded-xl">
                      <div className="p-2 bg-white rounded-lg shadow-sm"><Cpu className="w-4 h-4 text-emerald-600" /></div>
                      <div>
                        <div className="text-xs font-bold text-slate-900">AI Document Extraction</div>
                        <div className="text-[10px] text-slate-500">Stage ID: stage-ai-extract</div>
                      </div>
                    </div>
                  </div>

                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-amber-600 font-bold text-xs uppercase tracking-wider mb-3">
                      <RefreshCw className="w-4 h-4" /> Modified Connections
                    </div>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between p-3 bg-amber-50 border border-amber-100 rounded-xl">
                        <div className="text-xs font-medium text-slate-700">Initial Review &rarr; Approval</div>
                        <ArrowRight className="w-4 h-4 text-amber-400 mx-2" />
                        <div className="text-xs font-bold text-slate-900">Initial Review &rarr; AI Document Extraction</div>
                      </div>
                    </div>
                  </div>

                  <div className="bg-white border border-slate-200 rounded-2xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 text-rose-600 font-bold text-xs uppercase tracking-wider mb-3">
                      <X className="w-4 h-4" /> Removed Stages
                    </div>
                    <div className="p-3 bg-rose-50 border border-rose-100 rounded-xl text-xs text-rose-700 font-medium italic">
                      No stages were removed in this version.
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="p-6 border-t border-slate-100 bg-white flex justify-end gap-3">
                <button 
                  onClick={() => setShowCompareModal(false)}
                  className="px-6 py-3 bg-white border border-slate-200 text-slate-600 rounded-xl text-xs font-black uppercase tracking-wider hover:bg-slate-50 transition-all active:scale-95"
                >
                  Close
                </button>
                <button 
                  onClick={() => setShowCompareModal(false)}
                  className="px-6 py-3 bg-blue-600 text-white rounded-xl text-xs font-black uppercase tracking-wider shadow-lg shadow-blue-200 hover:bg-blue-700 transition-all active:scale-95"
                >
                  Restore v1.1.0
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

function PaletteItem({ icon, label, color, onHover, onLeave, onClick }: { icon: React.ReactNode; label: string; color: string; onHover: (rect: DOMRect) => void; onLeave: () => void; onClick?: () => void }) {
  const buttonRef = useRef<HTMLButtonElement>(null);

  return (
    <motion.button 
      ref={buttonRef}
      whileHover={{ y: -2, scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      onHoverStart={() => {
        if (buttonRef.current) onHover(buttonRef.current.getBoundingClientRect());
      }}
      onHoverEnd={onLeave}
      onClick={onClick}
      className={`p-3 rounded-xl border flex flex-col items-center justify-center gap-2 cursor-pointer shadow-sm transition-all ${color} border-slate-200/50 w-full outline-none focus:ring-2 focus:ring-blue-500`}
    >
      <div className="p-1.5 bg-white/50 rounded-lg shadow-inner">
        {icon}
      </div>
      <span className="text-[8px] font-black uppercase tracking-[0.1em] text-center">{label}</span>
    </motion.button>
  );
}

function PatternItem({ label }: { label: string; key?: React.Key }) {
  return (
    <motion.div 
      whileHover={{ x: 4 }}
      className="p-4 bg-white border border-slate-200 rounded-2xl text-xs font-bold text-slate-700 hover:border-blue-400 hover:shadow-md cursor-pointer flex items-center justify-between group transition-all"
    >
      {label}
      <ChevronRight className="w-4 h-4 text-slate-300 group-hover:text-blue-500 transition-colors" />
    </motion.div>
  );
}

function ValidationStatus({ label, valid }: { label: string; valid: boolean }) {
  return (
    <div className="flex items-center gap-2.5">
      {valid ? <CheckCircle2 className="w-4 h-4 text-emerald-500" /> : <AlertCircle className="w-4 h-4 text-rose-500" />}
      <span className={`text-[10px] font-black uppercase tracking-widest ${valid ? 'text-slate-900' : 'text-rose-500'}`}>{label}</span>
    </div>
  );
}

function VersionItem({ version, date, author, current }: { version: string; date: string; author: string; current?: boolean }) {
  return (
    <div className={`p-5 rounded-2xl border transition-all cursor-pointer relative group ${current ? 'bg-blue-50 border-blue-200 ring-1 ring-blue-100' : 'bg-white border-slate-200 hover:border-slate-400'}`}>
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-black text-slate-900 tracking-tight">Version {version}</span>
        {current && <span className="text-[9px] font-black text-blue-600 uppercase tracking-[0.2em] bg-white px-2 py-0.5 rounded-lg border border-blue-100 shadow-sm">Active</span>}
      </div>
      <div className="flex items-center gap-3 text-[10px] text-slate-500 font-bold uppercase tracking-wider">
        <div className="flex items-center gap-1.5">
          <Clock className="w-3.5 h-3.5 text-slate-300" />
          {date}
        </div>
        <span className="text-slate-200">•</span>
        <div className="flex items-center gap-1.5">
          <User className="w-3.5 h-3.5 text-slate-300" />
          {author}
        </div>
      </div>
    </div>
  );
}

// --- Sub-components ---

function DesignerCanvas({ workflow, selectedId, onSelect, validation, onUpdateStagePosition, onAddConnection, activeTool, collaborators, remoteCursors }: { 
  workflow: DesignerWorkflow; 
  selectedId?: string; 
  onSelect: (type: 'stage' | 'connection', id: string) => void;
  validation: WorkflowValidation | null;
  onUpdateStagePosition: (id: string, x: number, y: number) => void;
  onAddConnection: (from: string, to: string) => void;
  activeTool: 'select' | 'connect';
  collaborators: any[];
  remoteCursors: Record<string, { x: number; y: number }>;
}) {
  const realtime = useRealtime();
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [connectingFrom, setConnectingFrom] = useState<string | null>(null);
  const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
  const [searchQuery, setSearchQuery] = useState('');
  const [showMiniMap, setShowMiniMap] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  const filteredStages = useMemo(() => {
    if (!searchQuery) return [];
    const query = searchQuery.toLowerCase();
    return workflow.stages.filter(s => 
      s.name.toLowerCase().includes(query) ||
      s.type.toLowerCase().includes(query) ||
      (s.type === 'adaptive' && s.config.activities?.some((a: string) => a.toLowerCase().includes(query))) ||
      (s.type === 'ai-pipeline' && s.config.steps?.some((step: string) => step.toLowerCase().includes(query)))
    );
  }, [workflow.stages, searchQuery]);

  const jumpToStage = (stageId: string) => {
    const stage = workflow.stages.find(s => s.id === stageId);
    if (stage && containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      setPan({
        x: rect.width / 2 - (stage.position.x * zoom + 90 * zoom),
        y: rect.height / 2 - (stage.position.y * zoom + 40 * zoom)
      });
      onSelect('stage', stageId);
      setSearchQuery('');
    }
  };

  const handlePointerDown = useCallback((e: React.PointerEvent, stageId: string) => {
    e.stopPropagation();
    onSelect('stage', stageId);
    
    const startX = e.clientX;
    const startY = e.clientY;
    const stage = workflow.stages.find(s => s.id === stageId);
    if (!stage) return;
    const initialX = stage.position.x;
    const initialY = stage.position.y;

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const dx = (moveEvent.clientX - startX) / zoom;
      const dy = (moveEvent.clientY - startY) / zoom;
      onUpdateStagePosition(stageId, initialX + dx, initialY + dy);
    };

    const handlePointerUp = () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
  }, [onSelect, workflow.stages, zoom, onUpdateStagePosition]);

  const handleStartConnection = (stageId: string) => {
    setConnectingFrom(stageId);
    const stage = workflow.stages.find(s => s.id === stageId);
    if (stage) {
      setMousePos({ x: stage.position.x + 180, y: stage.position.y + 40 });
    }
  };

  const handleCanvasPointerDown = (e: React.PointerEvent) => {
    if (e.button === 1 || e.button === 0) { // Middle click or left click on canvas
      const startX = e.clientX;
      const startY = e.clientY;
      const initialPanX = pan.x;
      const initialPanY = pan.y;

      const handlePointerMove = (moveEvent: PointerEvent) => {
        const dx = moveEvent.clientX - startX;
        const dy = moveEvent.clientY - startY;
        setPan({ x: initialPanX + dx, y: initialPanY + dy });
      };

      const handlePointerUp = () => {
        window.removeEventListener('pointermove', handlePointerMove);
        window.removeEventListener('pointerup', handlePointerUp);
      };

      window.addEventListener('pointermove', handlePointerMove);
      window.addEventListener('pointerup', handlePointerUp);
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      setZoom(prev => Math.min(Math.max(0.1, prev * zoomFactor), 3));
    }
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      const x = (e.clientX - rect.left - pan.x) / zoom;
      const y = (e.clientY - rect.top - pan.y) / zoom;
      
      realtime.sendCursorMove({ x, y });

      if (connectingFrom) {
        setMousePos({ x, y });
      }
    }
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (connectingFrom) {
      setConnectingFrom(null);
    }
  };

  const handleStagePointerUp = useCallback((e: React.PointerEvent, stageId: string) => {
    if (connectingFrom && connectingFrom !== stageId) {
      e.stopPropagation();
      onAddConnection(connectingFrom, stageId);
      setConnectingFrom(null);
    }
  }, [connectingFrom, onAddConnection]);

  return (
    <div 
      className="absolute inset-0 overflow-hidden"
      onPointerDown={handleCanvasPointerDown}
      onWheel={handleWheel}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      ref={containerRef}
    >
      <div 
        className="relative w-full h-full origin-top-left"
        style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
      >
        {/* Remote Cursors */}
        {Object.entries(remoteCursors).map(([userId, pos]) => (
          <motion.div
            key={userId}
            initial={false}
            animate={{ x: pos.x, y: pos.y }}
            className="absolute z-50 pointer-events-none"
          >
            <MousePointer2 className="w-4 h-4 text-pink-500 fill-pink-500" />
            <div className="ml-3 px-1.5 py-0.5 bg-pink-500 text-white text-[8px] font-bold rounded shadow-sm whitespace-nowrap">
              {collaborators.find(c => c.id === userId)?.name || 'Collaborator'}
            </div>
          </motion.div>
        ))}

        {/* Connections Layer */}
        <svg className="absolute inset-0 w-full h-full pointer-events-none overflow-visible">
          <defs>
            <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill="#141414" />
            </marker>
          </defs>
          {workflow.connections.map(conn => {
            const fromStage = workflow.stages.find(s => s.id === conn.from);
            const toStage = workflow.stages.find(s => s.id === conn.to);
            if (!fromStage || !toStage) return null;
            
            return (
              <ConnectionLine 
                key={conn.id}
                conn={conn}
                fromStage={fromStage}
                toStage={toStage}
                isSelected={selectedId === conn.id}
                onSelect={(id) => onSelect('connection', id)}
              />
            );
          })}
          {connectingFrom && (() => {
            const fromStage = workflow.stages.find(s => s.id === connectingFrom);
            if (!fromStage) return null;
            const startX = fromStage.position.x + 180;
            const startY = fromStage.position.y + 40;
            return (
              <path 
                d={`M ${startX} ${startY} C ${startX + 50} ${startY}, ${mousePos.x - 50} ${mousePos.y}, ${mousePos.x} ${mousePos.y}`} 
                fill="none" 
                stroke="#2563eb" 
                strokeWidth={2}
                strokeDasharray="5,5"
                markerEnd="url(#arrowhead)"
              />
            );
          })()}
        </svg>

        {/* Stages Layer */}
        {workflow.stages.map(stage => (
          <StageNode 
            key={stage.id}
            stage={stage}
            isSelected={selectedId === stage.id}
            activeTool={activeTool}
            validation={validation}
            onPointerDown={handlePointerDown}
            onPointerUp={handleStagePointerUp}
            onStartConnection={handleStartConnection}
          />
        ))}
      </div>

      {/* Collaborators List */}
      <div className="absolute top-24 left-8 flex items-center gap-2 z-30">
        <div className="flex -space-x-2">
          <div className="w-8 h-8 rounded-full bg-blue-600 border-2 border-white flex items-center justify-center text-white text-[10px] font-bold shadow-sm" title="You">Y</div>
          {collaborators.map(c => (
            <div key={c.id} className="w-8 h-8 rounded-full bg-pink-500 border-2 border-white flex items-center justify-center text-white text-[10px] font-bold shadow-sm" title={c.name}>
              {c.name[0]}
            </div>
          ))}
        </div>
        {collaborators.length > 0 && (
          <div className="px-2 py-1 bg-white/80 backdrop-blur-sm border border-slate-200 rounded-lg text-[10px] font-bold text-slate-600 shadow-sm">
            {collaborators.length} other{collaborators.length > 1 ? 's' : ''} editing
          </div>
        )}
      </div>

      {/* Floating Controls Layer */}
      <div className="absolute bottom-52 right-8 flex flex-col-reverse gap-4 z-30">
        {/* Mini-map */}
        <AnimatePresence>
          {showMiniMap && (
            <motion.div 
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              className="w-48 h-32 bg-white/80 backdrop-blur-md border border-slate-200 rounded-2xl shadow-2xl overflow-hidden relative group"
            >
              <div className="absolute inset-0 p-2 opacity-40 pointer-events-none">
                {workflow.stages.map(s => (
                  <div 
                    key={s.id}
                    className={`absolute rounded-sm ${getStageColor(s.type)} border border-slate-300`}
                    style={{
                      left: `${s.position.x / 15}%`,
                      top: `${s.position.y / 15}%`,
                      width: '12px',
                      height: '8px'
                    }}
                  />
                ))}
              </div>
              {/* Viewport Indicator */}
              <div 
                className="absolute border-2 border-blue-500 bg-blue-500/10 rounded-lg pointer-events-none transition-all"
                style={{
                  left: `${-pan.x / (15 * zoom)}%`,
                  top: `${-pan.y / (15 * zoom)}%`,
                  width: `${100 / zoom}%`,
                  height: `${100 / zoom}%`,
                  maxWidth: '100%',
                  maxHeight: '100%'
                }}
              />
              <div className="absolute bottom-2 right-2 flex gap-1">
                <button 
                  onClick={() => setZoom(prev => Math.min(prev + 0.1, 3))}
                  className="p-1 bg-white border border-slate-200 rounded-md shadow-sm hover:bg-slate-50"
                >
                  <Maximize2 className="w-3 h-3 text-slate-600" />
                </button>
                <button 
                  onClick={() => setZoom(prev => Math.max(prev - 0.1, 0.1))}
                  className="p-1 bg-white border border-slate-200 rounded-md shadow-sm hover:bg-slate-50"
                >
                  <Minimize2 className="w-3 h-3 text-slate-600" />
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
        
        <button 
          onClick={() => setShowMiniMap(!showMiniMap)}
          title="Toggle Mini-map"
          className={`w-12 h-12 rounded-2xl flex items-center justify-center shadow-lg transition-all active:scale-90 ${showMiniMap ? 'bg-slate-900 text-white' : 'bg-white text-slate-600 border border-slate-200'}`}
        >
          <MapIcon className="w-5 h-5" />
        </button>
      </div>

      {/* Stage Search */}
      <div className="absolute top-8 left-8 z-20 w-72">
        <div className="relative group">
          <Search className="w-4 h-4 text-slate-400 absolute left-4 top-1/2 -translate-y-1/2 group-focus-within:text-blue-500 transition-colors" />
          <input 
            type="text" 
            placeholder="Find stage..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-11 pr-4 py-3 bg-white/90 backdrop-blur-md border border-slate-200 rounded-2xl shadow-xl focus:ring-2 focus:ring-blue-500 outline-none text-sm transition-all"
          />
          <AnimatePresence>
            {searchQuery && (
              <motion.div 
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 10 }}
                className="absolute top-full left-0 right-0 mt-2 bg-white rounded-2xl shadow-2xl border border-slate-100 overflow-hidden max-h-64 overflow-y-auto no-scrollbar"
              >
                {filteredStages.length > 0 ? (
                  filteredStages.map(s => (
                    <button 
                      key={s.id}
                      onClick={() => jumpToStage(s.id)}
                      className="w-full px-4 py-3 flex items-center gap-3 hover:bg-slate-50 transition-colors border-b border-slate-50 last:border-0"
                    >
                      <div className={`p-1.5 rounded-lg ${getStageColor(s.type)}`}>
                        {getStageIcon(s.type)}
                      </div>
                      <div className="text-left">
                        <p className="text-xs font-black text-slate-900 leading-none mb-1">{s.name}</p>
                        <div className="flex flex-wrap gap-1">
                          <p className="text-[9px] font-bold text-slate-400 uppercase tracking-widest">{s.type}</p>
                          {s.type === 'adaptive' && s.config.activities?.map((a: string) => (
                            <span key={a} className="text-[8px] font-bold text-blue-500 uppercase tracking-tighter">/ {a}</span>
                          ))}
                          {s.type === 'ai-pipeline' && s.config.steps?.map((step: string) => (
                            <span key={step} className="text-[8px] font-bold text-purple-500 uppercase tracking-tighter">/ {step}</span>
                          ))}
                        </div>
                      </div>
                    </button>
                  ))
                ) : (
                  <div className="p-4 text-center text-slate-400 text-xs font-medium">No stages found</div>
                )}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}

function getStageColor(type: string) {
  switch (type) {
    case 'ai-pipeline': return 'bg-purple-50';
    case 'adaptive': return 'bg-amber-50';
    case 'parallel': return 'bg-emerald-50';
    case 'final': return 'bg-gray-100';
    case 'split': return 'bg-cyan-50';
    case 'join': return 'bg-teal-50';
    case 'decision': return 'bg-orange-50';
    case 'timer': return 'bg-rose-50';
    case 'api': return 'bg-indigo-50';
    default: return 'bg-blue-50';
  }
}

function getStageIcon(type: string) {
  switch (type) {
    case 'ai-pipeline': return <Cpu className="w-3 h-3 text-purple-600" />;
    case 'adaptive': return <RefreshCw className="w-3 h-3 text-amber-600" />;
    case 'parallel': return <Layers className="w-3 h-3 text-emerald-600" />;
    case 'final': return <CheckCircle2 className="w-3 h-3 text-gray-600" />;
    case 'split': return <Split className="w-3 h-3 text-cyan-600" />;
    case 'join': return <Merge className="w-3 h-3 text-teal-600" />;
    case 'decision': return <GitMerge className="w-3 h-3 text-orange-600" />;
    case 'timer': return <Timer className="w-3 h-3 text-rose-600" />;
    case 'api': return <Webhook className="w-3 h-3 text-indigo-600" />;
    default: return <Plus className="w-3 h-3 text-blue-600" />;
  }
}

function PropertiesPanel({ element, workflow, onUpdate, onDelete }: { 
  element: { type: 'stage' | 'connection'; id: string }; 
  workflow: DesignerWorkflow;
  onUpdate: (wf: DesignerWorkflow) => void;
  onDelete: (id: string, type: 'stage' | 'connection') => void;
}) {
  const stage = element.type === 'stage' ? workflow.stages.find(s => s.id === element.id) : null;
  const connection = element.type === 'connection' ? workflow.connections.find(c => c.id === element.id) : null;

  const [nameError, setNameError] = useState('');

  const updateStage = (updates: Partial<WorkflowStage>) => {
    if (!stage) return;
    if (updates.name !== undefined) {
      if (!updates.name.trim()) {
        setNameError('Stage name cannot be empty');
      } else {
        setNameError('');
      }
    }
    const updatedStages = workflow.stages.map(s => s.id === stage.id ? { ...s, ...updates } : s);
    onUpdate({ ...workflow, stages: updatedStages });
  };

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      <div className="p-6 border-b border-gray-100 bg-gray-50 flex items-center justify-between">
        <h3 className="font-bold text-gray-900 uppercase tracking-widest text-xs">Properties</h3>
        <span className="text-[10px] font-mono text-gray-400">{element.id}</span>
      </div>
      
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {stage && (
          <>
            <div className="space-y-2">
              <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Stage Name</label>
              <input 
                type="text" 
                value={stage.name} 
                onChange={(e) => updateStage({ name: e.target.value })}
                className={`w-full px-3 py-3 sm:py-2 border ${nameError ? 'border-rose-500 ring-1 ring-rose-500' : 'border-gray-200'} rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none`}
              />
              {nameError && <p className="text-xs text-rose-500 mt-1">{nameError}</p>}
            </div>

            <div className="space-y-2">
              <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Description</label>
              <textarea 
                value={stage.description || ''} 
                onChange={(e) => updateStage({ description: e.target.value })}
                placeholder="Describe this stage's purpose..."
                className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none min-h-[80px] resize-none"
              />
            </div>

            <div className="space-y-2">
              <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Type</label>
              <select 
                value={stage.type} 
                onChange={(e) => updateStage({ type: e.target.value as any })}
                className="w-full px-3 py-3 sm:py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white"
              >
                <option value="simple">Simple Stage</option>
                <option value="ai-pipeline">AI Pipeline</option>
                <option value="adaptive">Adaptive Phase</option>
                <option value="parallel">Parallel Section</option>
                <option value="split">Parallel Split</option>
                <option value="join">Parallel Join</option>
                <option value="decision">Decision Rule</option>
                <option value="timer">Timer / Escalation</option>
                <option value="api">API / Webhook</option>
                <option value="final">Final Stage</option>
              </select>
            </div>

            <div className="p-4 bg-slate-50 rounded-xl border border-slate-100 space-y-4">
              <div className="flex items-center gap-2 mb-2">
                <User className="w-4 h-4 text-slate-400" />
                <h4 className="text-[10px] font-black text-slate-900 uppercase tracking-widest">Assignment & Ownership</h4>
              </div>
              
              <div className="space-y-2">
                <label className="text-[9px] font-bold text-slate-500 uppercase tracking-wider">Assignee Type</label>
                <div className="grid grid-cols-2 gap-2">
                  {[
                    { id: 'none', label: 'Unassigned' },
                    { id: 'individual', label: 'Individual' },
                    { id: 'team', label: 'Team/Group' },
                    { id: 'agent', label: 'AI Agent' }
                  ].map(t => (
                    <button
                      key={t.id}
                      onClick={() => {
                        if (t.id === 'none') {
                          const { assignee, ...rest } = stage.config;
                          updateStage({ config: rest });
                        } else {
                          updateStage({ config: { ...stage.config, assignee: { type: t.id as any, id: '', label: '' } } });
                        }
                      }}
                      className={`px-2 py-2 rounded-lg text-[10px] font-bold uppercase tracking-wider border transition-all ${(!stage.config.assignee && t.id === 'none') || (stage.config.assignee?.type === t.id) ? 'bg-blue-600 border-blue-600 text-white shadow-md' : 'bg-white border-slate-200 text-slate-600 hover:border-slate-300'}`}
                    >
                      {t.label}
                    </button>
                  ))}
                </div>
              </div>

              {stage.config.assignee && (
                <div className="space-y-2 animate-in fade-in slide-in-from-top-2 duration-200">
                  <label className="text-[9px] font-bold text-slate-500 uppercase tracking-wider">
                    Select {stage.config.assignee.type === 'individual' ? 'Person' : stage.config.assignee.type === 'team' ? 'Team' : 'Agent'}
                  </label>
                  <select
                    value={stage.config.assignee.id}
                    onChange={(e) => {
                      const id = e.target.value;
                      updateStage({ config: { ...stage.config, assignee: { ...stage.config.assignee, id, label: id } } });
                    }}
                    className="w-full px-3 py-2 border border-slate-200 rounded-lg text-xs focus:ring-2 focus:ring-blue-500 outline-none bg-white"
                  >
                    <option value="">-- Select --</option>
                  </select>
                </div>
              )}
            </div>

            {stage.type === 'ai-pipeline' && (
              <div className="space-y-4">
                <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Pipeline Steps</label>
                <div className="space-y-2">
                  {stage.config.steps?.map((step: string, i: number) => (
                    <div key={i} className="flex items-center gap-2 p-2 bg-gray-50 border border-gray-200 rounded-lg text-xs">
                      <Cpu className="w-3 h-3 text-purple-500" />
                      {step}
                    </div>
                  ))}
                  <button 
                    onClick={() => {
                      const steps = stage.config.steps || [];
                      updateStage({ config: { ...stage.config, steps: [...steps, 'New AI Step'] } });
                    }}
                    className="w-full py-2 border-2 border-dashed border-gray-200 rounded-lg text-[10px] font-bold text-gray-400 hover:border-gray-400 hover:text-gray-600 transition-all"
                  >
                    + Add AI Step
                  </button>
                </div>
              </div>
            )}

            {stage.type === 'adaptive' && (
              <div className="space-y-4">
                <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Activities & Rules</label>
                <div className="space-y-2">
                  {stage.config.activities?.map((act: string, i: number) => (
                    <div key={i} className="flex items-center gap-2 p-2 bg-gray-50 border border-gray-200 rounded-lg text-xs">
                      <RefreshCw className="w-3 h-3 text-amber-500" />
                      {act}
                    </div>
                  ))}
                </div>
                <div className="p-3 bg-amber-50 border border-amber-100 rounded-lg">
                  <h4 className="text-[10px] font-bold text-amber-800 uppercase tracking-widest mb-2">Constraints</h4>
                  <div className="text-[10px] text-amber-700 italic">
                    "Identity Check" must be completed before "Income Verification" becomes available.
                  </div>
                </div>
              </div>
            )}
            {stage.type === 'decision' && (
              <div className="space-y-4">
                <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Routing Rules</label>
                <div className="space-y-2">
                  <div className="p-3 bg-gray-50 border border-gray-200 rounded-lg text-xs space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="font-bold text-gray-700">Rule 1</span>
                      <span className="text-[10px] text-gray-400">If</span>
                    </div>
                    <input type="text" placeholder="e.g., case.value > 50000" className="w-full p-2.5 sm:p-1.5 border border-gray-200 rounded text-xs" />
                    <div className="flex items-center justify-between mt-2">
                      <span className="text-[10px] text-gray-400">Then route to</span>
                    </div>
                    <select className="w-full p-2.5 sm:p-1.5 border border-gray-200 rounded text-xs appearance-none bg-white">
                      {workflow.stages.filter(s => s.id !== stage.id).map(s => (
                        <option key={s.id} value={s.id}>{s.name}</option>
                      ))}
                    </select>
                  </div>
                  <button className="w-full py-2 border-2 border-dashed border-gray-200 rounded-lg text-[10px] font-bold text-gray-400 hover:border-gray-400 hover:text-gray-600 transition-all">
                    + Add Rule
                  </button>
                </div>
              </div>
            )}

            {stage.type === 'timer' && (
              <div className="space-y-4">
                <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Escalation Timer</label>
                <div className="space-y-3">
                  <div>
                    <label className="text-[10px] text-gray-500 mb-1 block">Duration (Days)</label>
                    <input type="number" defaultValue="3" className="w-full px-3 py-3 sm:py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none" />
                  </div>
                  <div>
                    <label className="text-[10px] text-gray-500 mb-1 block">Action on Expiry</label>
                    <select className="w-full px-3 py-3 sm:py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                      <option>Reassign to Manager</option>
                      <option>Send Reminder Email</option>
                      <option>Route to Escalation Queue</option>
                    </select>
                  </div>
                </div>
              </div>
            )}

            {stage.type === 'api' && (
              <div className="space-y-4">
                <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Webhook Configuration</label>
                <div className="space-y-3">
                  <div>
                    <label className="text-[10px] text-gray-500 mb-1 block">Endpoint URL</label>
                    <input type="url" placeholder="https://api.example.com/webhook" className="w-full px-3 py-3 sm:py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none" />
                  </div>
                  <div>
                    <label className="text-[10px] text-gray-500 mb-1 block">Method</label>
                    <select className="w-full px-3 py-3 sm:py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                      <option>POST</option>
                      <option>PUT</option>
                      <option>GET</option>
                    </select>
                  </div>
                  <div className="flex items-center gap-3 mt-2 min-h-[44px]">
                    <input type="checkbox" id="wait-response" className="w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500" />
                    <label htmlFor="wait-response" className="text-xs text-gray-600 font-medium">Wait for synchronous response</label>
                  </div>
                </div>
              </div>
            )}

            {stage.type === 'split' && (
              <div className="p-3 bg-cyan-50 border border-cyan-100 rounded-lg">
                <h4 className="text-[10px] font-bold text-cyan-800 uppercase tracking-widest mb-2">Parallel Split</h4>
                <div className="text-[10px] text-cyan-700 leading-relaxed">
                  This node will duplicate the workflow token and send it down all connected outgoing paths simultaneously.
                </div>
              </div>
            )}

            {stage.type === 'join' && (
              <div className="p-3 bg-teal-50 border border-teal-100 rounded-lg">
                <h4 className="text-[10px] font-bold text-teal-800 uppercase tracking-widest mb-2">Parallel Join</h4>
                <div className="text-[10px] text-teal-700 leading-relaxed">
                  This node will wait until all incoming parallel paths have reached it before continuing to the next stage.
                </div>
              </div>
            )}
          </>
        )}

        {connection && (
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">Trigger Condition</label>
              <input 
                type="text" 
                value={connection.condition || ''} 
                onChange={(e) => {
                  const updatedConnections = workflow.connections.map(c => c.id === connection.id ? { ...c, condition: e.target.value } : c);
                  onUpdate({ ...workflow, connections: updatedConnections });
                }}
                placeholder="e.g., Status == Approved"
                className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none"
              />
            </div>
            <div className="p-4 bg-blue-50 border border-blue-100 rounded-xl">
              <p className="text-[10px] text-blue-700 leading-relaxed">
                This connection defines the transition between stages. You can specify complex logic using the expression builder.
              </p>
            </div>
          </div>
        )}
      </div>
      <div className="p-6 border-t border-gray-100 bg-gray-50">
        <button 
          onClick={() => onDelete(element.id, element.type)}
          className="w-full py-2.5 bg-white border border-rose-200 text-rose-600 rounded-lg text-xs font-bold uppercase tracking-wider hover:bg-rose-50 transition-all"
        >
          Delete {element.type === 'stage' ? 'Stage' : 'Connection'}
        </button>
      </div>
    </div>
  );
}
