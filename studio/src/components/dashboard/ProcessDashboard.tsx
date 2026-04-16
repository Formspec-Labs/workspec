import React, { useState, useEffect } from 'react';
import { KPISection } from './KPISection';
import { WorkflowHeatmap } from './WorkflowHeatmap';
import { AlertsPanel } from './AlertsPanel';
import { DecisionDriftChart } from './DecisionDriftChart';
import { RedeterminationPipeline } from './RedeterminationPipeline';
import { Filter, Calendar, RefreshCw, Download, ShieldAlert } from 'lucide-react';
import { useDashboard } from '../../context/WosContext';
import type { DashboardMetrics, StageMetricView, AlertView, DriftDataPoint, PipelineDataPoint } from '../../services/WosPorts';
import { motion, AnimatePresence } from 'motion/react';

import { InterventionWorkspace } from './InterventionWorkspace';

export function ProcessDashboard() {
  const dashboard = useDashboard();
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null);
  const [stages, setStages] = useState<StageMetricView[]>([]);
  const [alerts, setAlerts] = useState<AlertView[]>([]);
  const [driftData, setDriftData] = useState<DriftDataPoint[]>([]);
  const [pipelineData, setPipelineData] = useState<PipelineDataPoint[]>([]);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [activeTeam, setActiveTeam] = useState<'overview' | 'alpha' | 'beta'>('overview');
  const [isExporting, setIsExporting] = useState(false);
  const [isInterventionMode, setIsInterventionMode] = useState(false);

  const loadData = async () => {
    setIsRefreshing(true);
    await Promise.all([
      dashboard.getMetrics().then(setMetrics),
      dashboard.getStageMetrics().then(setStages),
      dashboard.getAlerts().then(setAlerts),
      dashboard.getDriftData().then(setDriftData),
      dashboard.getPipelineData().then(setPipelineData)
    ]);
    setIsRefreshing(false);
  };

  useEffect(() => {
    loadData();
  }, [dashboard, activeTeam]);

  const handleExport = () => {
    setIsExporting(true);
    setTimeout(() => {
      setIsExporting(false);
    }, 1500);
  };

  if (!metrics) return (
    <div className="flex-1 flex items-center justify-center bg-gray-50">
      <div className="flex flex-col items-center gap-4">
        <RefreshCw className="w-8 h-8 text-blue-500 animate-spin" />
        <p className="text-sm font-medium text-gray-500">Initializing operations data...</p>
      </div>
    </div>
  );

  return (
    <div className="flex-1 flex flex-col overflow-hidden bg-[#f8fafc]">
      <div className="bg-white border-b border-slate-200 px-4 sm:px-8 py-8 shrink-0 z-10 shadow-sm relative overflow-hidden">
        <div className="absolute top-0 right-0 w-64 h-64 bg-blue-50 rounded-full -mr-32 -mt-32 opacity-50 blur-3xl" />
        
        <AnimatePresence>
          {alerts.some(a => a.severity === 'critical') && (
            <motion.div 
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="bg-rose-600 text-white px-8 py-2 -mx-8 -mt-8 mb-6 flex items-center justify-between"
            >
              <div className="flex items-center gap-3">
                <ShieldAlert className="w-4 h-4 animate-pulse" />
                <span className="text-[10px] font-black uppercase tracking-widest">System Pulse: 12 cases approaching SLA breach in 'Verification' stage. Intervention recommended.</span>
              </div>
              <button 
                onClick={() => setIsInterventionMode(!isInterventionMode)}
                className="text-[10px] font-black uppercase tracking-widest bg-white/20 hover:bg-white/30 px-3 py-1 rounded-lg transition-colors"
              >
                {isInterventionMode ? 'Exit Intervention' : 'Enter Intervention Mode'}
              </button>
            </motion.div>
          )}
        </AnimatePresence>

        <div className="max-w-7xl mx-auto flex flex-col md:flex-row md:items-end justify-between gap-8 relative z-10">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-blue-600 font-black text-[10px] uppercase tracking-[0.3em]">
              <div className="w-2 h-2 rounded-full bg-blue-600 animate-pulse shadow-[0_0_8px_rgba(37,99,235,0.6)]" />
              Live System Status
            </div>
            <h1 className="text-3xl sm:text-4xl font-black text-slate-900 tracking-tight">Operations Dashboard</h1>
            <p className="text-sm text-slate-500 max-w-md font-medium leading-relaxed">Real-time oversight of case throughput, AI accuracy, and team performance metrics.</p>
          </div>
          
          <div className="flex flex-wrap items-center gap-4">
            <div className="flex items-center bg-slate-100/50 border border-slate-200 rounded-2xl p-1.5 shadow-inner">
              <button 
                onClick={() => setActiveTeam('overview')}
                className={`px-5 py-2 text-[10px] font-black uppercase tracking-widest rounded-xl transition-colors ${activeTeam === 'overview' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
              >
                Overview
              </button>
              <button 
                onClick={() => setActiveTeam('alpha')}
                className={`px-5 py-2 text-[10px] font-black uppercase tracking-widest rounded-xl transition-colors ${activeTeam === 'alpha' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
              >
                Team Alpha
              </button>
              <button 
                onClick={() => setActiveTeam('beta')}
                className={`px-5 py-2 text-[10px] font-black uppercase tracking-widest rounded-xl transition-colors ${activeTeam === 'beta' ? 'text-slate-900 bg-white shadow-sm border border-slate-200' : 'text-slate-400 hover:text-slate-600'}`}
              >
                Team Beta
              </button>
            </div>
            
            <div className="h-10 w-px bg-slate-200 mx-2 hidden sm:block" />
            
            <div className="flex items-center gap-3">
              <button 
                onClick={loadData}
                disabled={isRefreshing}
                className="p-3 text-slate-400 hover:text-blue-600 hover:bg-blue-50 rounded-2xl border border-slate-200 bg-white transition-all active:scale-95 shadow-sm"
              >
                <RefreshCw className={`w-5 h-5 ${isRefreshing ? 'animate-spin' : ''}`} />
              </button>
              
              <button 
                onClick={handleExport}
                disabled={isExporting}
                className="flex items-center gap-2.5 px-6 py-3 bg-slate-900 text-white rounded-2xl text-[10px] font-black uppercase tracking-[0.15em] shadow-xl shadow-slate-200 hover:bg-slate-800 transition-all active:scale-95 border border-slate-800 disabled:opacity-70"
              >
                {isExporting ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Download className="w-4 h-4" />}
                {isExporting ? 'Exporting...' : 'Export Report'}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 sm:p-10">
        {isInterventionMode ? (
          <InterventionWorkspace />
        ) : (
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="max-w-7xl mx-auto space-y-10"
          >
            <KPISection metrics={metrics} />

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-10">
              <div className="lg:col-span-2">
                <WorkflowHeatmap stages={stages} />
              </div>
              <div className="lg:col-span-1">
                <AlertsPanel alerts={alerts} />
              </div>
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-10 pb-16">
              <DecisionDriftChart data={driftData} />
              <RedeterminationPipeline data={pipelineData} />
            </div>
          </motion.div>
        )}
      </div>
    </div>
  );
}
