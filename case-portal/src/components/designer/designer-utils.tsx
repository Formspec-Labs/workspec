import React from 'react';
import { Plus, Cpu, RefreshCw, Layers, CheckCircle2, Split, Merge, GitMerge, Timer, Webhook } from 'lucide-react';
import type { WosValidationResult } from '../../services/WosPorts';

export interface PaletteItemData {
  id: string;
  label: string;
  icon: string;
  color: string;
  description?: string;
}

export interface PatternItemData {
  id: string;
  label: string;
}

export interface ValidationIssue {
  id: string;
  severity: 'error' | 'warning';
  category: 'structure' | 'policy' | 'soundness' | 'satisfiability';
  message: string;
  targetId?: string;
}

export interface WorkflowValidation {
  isValid: boolean;
  issues: ValidationIssue[];
  status: {
    structure: boolean;
    policy: boolean;
    soundness: boolean;
    satisfiability: boolean;
  };
}

export function mapWosValidation(result: WosValidationResult): WorkflowValidation {
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

export function getStageColor(type: string) {
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

export function getStageIcon(type: string) {
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
