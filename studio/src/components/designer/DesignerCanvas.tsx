import React, { useState, useRef, memo, useCallback, useMemo } from 'react';
import { AlertCircle, Sparkles, User, MousePointer2, Search, Maximize2, Minimize2, Map as MapIcon } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { useRealtime } from '../../context/WosContext';
import type { WorkflowStage, WorkflowConnection, DesignerWorkflow } from '../../services/KernelToDesigner';
import { getStageColor, getStageIcon, type WorkflowValidation } from './designer-utils';

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

export interface DesignerCanvasProps {
  workflow: DesignerWorkflow;
  selectedId?: string;
  onSelect: (type: 'stage' | 'connection', id: string) => void;
  validation: WorkflowValidation | null;
  onUpdateStagePosition: (id: string, x: number, y: number) => void;
  onAddConnection: (from: string, to: string) => void;
  activeTool: 'select' | 'connect';
  collaborators: any[];
  remoteCursors: Record<string, { x: number; y: number }>;
}

export function DesignerCanvas({
  workflow,
  selectedId,
  onSelect,
  validation,
  onUpdateStagePosition,
  onAddConnection,
  activeTool,
  collaborators,
  remoteCursors
}: DesignerCanvasProps) {
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
    if (e.button === 1 || e.button === 0) {
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

      <div className="absolute bottom-52 right-8 flex flex-col-reverse gap-4 z-30">
        <AnimatePresence>
          {showMiniMap && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              data-testid="mini-map"
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
