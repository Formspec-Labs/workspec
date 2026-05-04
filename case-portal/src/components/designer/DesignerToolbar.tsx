import React, { useRef, useState } from 'react';
import { X, Plus, ChevronRight, Layers, RefreshCw, Cpu, CheckCircle2, Split, Merge, GitMerge, Timer, Webhook, Network } from 'lucide-react';
import { motion } from 'motion/react';
import { AnimatePresence } from 'motion/react';
import type { PaletteItemData, PatternItemData } from './designer-utils';

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

export interface DesignerToolbarProps {
  palette: PaletteItemData[];
  patterns: PatternItemData[];
  isOpen: boolean;
  onClose: () => void;
  onAddStage: (type: string, label: string) => void;
}

export function DesignerToolbar({ palette, patterns, isOpen, onClose, onAddStage }: DesignerToolbarProps) {
  const [hoveredItem, setHoveredItem] = useState<{ id: string; description: string; rect: DOMRect } | null>(null);

  return (
    <>
      <div className={`lg:w-72 bg-white border-r border-slate-200 flex flex-col shrink-0 z-[60] transition-all duration-300 ${isOpen ? 'fixed inset-y-0 left-0 w-full sm:w-72 shadow-2xl' : 'hidden lg:flex'}`}>
        <div className="p-6 border-b border-slate-50 flex items-center justify-between lg:block shrink-0">
          <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-0 lg:mb-6">Stage Components</h3>
          <button
            onClick={onClose}
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
                    onHover={(rect) => setHoveredItem({ id: item.id, description: item.description || '', rect })}
                    onLeave={() => setHoveredItem(null)}
                    onClick={() => onAddStage(item.id, item.label)}
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

      <AnimatePresence>
        {hoveredItem && (
          <motion.div
            initial={{ opacity: 0, x: -10, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: -10, scale: 0.95 }}
            transition={{ delay: 0.4, duration: 0.15 }}
            style={{
              position: 'fixed',
              top: hoveredItem.rect.top,
              left: hoveredItem.rect.right + 12,
            }}
            className="w-52 p-3 bg-slate-900 text-white text-[10px] rounded-xl shadow-2xl z-[9999] pointer-events-none hidden lg:block border border-slate-800"
          >
            <div className="absolute left-0 top-6 -ml-1 w-2 h-2 bg-slate-900 rotate-45 border-l border-b border-slate-800" />
            <p className="font-medium leading-relaxed text-slate-200">{hoveredItem.description}</p>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
