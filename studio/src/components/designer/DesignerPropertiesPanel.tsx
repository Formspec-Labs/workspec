import React, { useState } from 'react';
import { User, Cpu, RefreshCw } from 'lucide-react';
import type { WorkflowStage, DesignerWorkflow } from '../../services/KernelToDesigner';

export interface DesignerPropertiesPanelProps {
  element: { type: 'stage' | 'connection'; id: string };
  workflow: DesignerWorkflow;
  onUpdate: (wf: DesignerWorkflow) => void;
  onDelete: (id: string, type: 'stage' | 'connection') => void;
}

export function DesignerPropertiesPanel({ element, workflow, onUpdate, onDelete }: DesignerPropertiesPanelProps) {
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
