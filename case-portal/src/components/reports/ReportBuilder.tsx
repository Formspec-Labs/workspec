import React, { useState, useEffect } from 'react';
import { 
  BarChart3, 
  LineChart, 
  PieChart, 
  Table as TableIcon, 
  Calendar, 
  Filter, 
  Download, 
  Share2, 
  Clock, 
  FileText, 
  ShieldAlert, 
  BrainCircuit, 
  Users, 
  Activity,
  Plus,
  Settings,
  ChevronDown,
  LayoutGrid,
  X,
  Pin,
  Check
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { 
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer,
  LineChart as RechartsLineChart, Line
} from 'recharts';
import { useBackend } from '../../context/WosContext';
import { ReportTemplate, ReportConfig } from '../../types';

const ICON_MAP: Record<string, any> = {
  'Activity': Activity,
  'FileText': FileText,
  'Clock': Clock,
  'BarChart3': BarChart3,
  'ShieldAlert': ShieldAlert,
  'Users': Users
};

const STUB_TEMPLATES: ReportTemplate[] = [
  { id: 'decision-drift', title: 'Decision Drift Analysis', description: 'Detect rubber-stamping and AI accuracy trends over time.', category: 'ai-performance', icon: 'Activity' },
  { id: 'regulatory-comparison', title: 'Regulatory Version Comparison', description: 'Compare case outcomes across regulatory version changes.', category: 'compliance', icon: 'FileText' },
  { id: 'team-performance', title: 'Team Performance Metrics', description: 'Track throughput, accuracy, and SLA compliance by team.', category: 'operational', icon: 'Users' },
  { id: 'caseload-overview', title: 'Caseload Overview', description: 'Current and historical caseload distribution.', category: 'caseload', icon: 'BarChart3' },
];

const STUB_DRIFT_DATA = [
  { month: 'Jan', overrideRate: 15, timeOnTask: 8.5 },
  { month: 'Feb', overrideRate: 12, timeOnTask: 6.2 },
  { month: 'Mar', overrideRate: 8, timeOnTask: 3.1 },
  { month: 'Apr', overrideRate: 5, timeOnTask: 1.2 },
  { month: 'May', overrideRate: 3, timeOnTask: 0.6 },
  { month: 'Jun', overrideRate: 2, timeOnTask: 0.5 },
];

const STUB_REG_DATA = [
  { ruleVersion: 'FY2024-Q3', approved: 120, denied: 30 },
  { ruleVersion: 'FY2025-Q1', approved: 95, denied: 55 },
  { ruleVersion: 'FY2025-Q3', approved: 110, denied: 40 },
  { ruleVersion: 'FY2026-Q1', approved: 105, denied: 45 },
];

function getStubReportData(templateId: string): any[] {
  if (templateId === 'decision-drift') return STUB_DRIFT_DATA;
  if (templateId === 'regulatory-comparison') return STUB_REG_DATA;
  return [];
}

export function ReportBuilder() {
  useBackend();
  const [templates] = useState<ReportTemplate[]>(STUB_TEMPLATES);
  const [mode, setMode] = useState<'templates' | 'custom'>('templates');
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [showScheduleModal, setShowScheduleModal] = useState(false);
  const [reportData, setReportData] = useState<any>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [isSharing, setIsSharing] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [livePreview, setLivePreview] = useState(true);

  const [selectedMetrics, setSelectedMetrics] = useState<string[]>([]);
  const [selectedDimensions, setSelectedDimensions] = useState<string[]>([]);
  const [selectedViz, setSelectedViz] = useState<'bar' | 'line' | 'table'>('bar');

  useEffect(() => {
  }, []);

  useEffect(() => {
    if (mode === 'custom' && livePreview && (selectedMetrics.length > 0 || selectedDimensions.length > 0)) {
      const timer = setTimeout(() => {
        handleGenerate();
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [selectedMetrics, selectedDimensions, selectedViz, livePreview, mode]);

  const handleGenerate = async () => {
    if (!selectedTemplateId && mode === 'templates') return;
    
    setIsGenerating(true);
    setShowPreview(false);
    
    const config: ReportConfig = {
      name: mode === 'templates' ? templates.find(t => t.id === selectedTemplateId)?.title || 'Report' : 'Custom Report',
      type: mode === 'templates' ? 'template' : 'custom',
      templateId: selectedTemplateId || undefined,
      metrics: selectedMetrics,
      dimensions: selectedDimensions,
      visualization: selectedViz,
      filters: {},
      dateRange: { start: '2026-01-01', end: '2026-06-30' }
    };

    const data = getStubReportData(selectedTemplateId || 'custom');
    setReportData(data);
    
    setIsGenerating(false);
    setShowPreview(true);
  };

  const handleExport = async () => {
    setIsExporting(true);
    await new Promise(resolve => setTimeout(resolve, 1000));
    setIsExporting(false);
  };

  const handleShare = async () => {
    setIsSharing(true);
    await new Promise(resolve => setTimeout(resolve, 1000));
    setIsSharing(false);
  };

  const handleSaveSchedule = async () => {
    await new Promise(resolve => setTimeout(resolve, 1000));
    setShowScheduleModal(false);
  };

  const handlePin = async () => {
    setIsPinned(true);
    await new Promise(resolve => setTimeout(resolve, 800));
    setTimeout(() => setIsPinned(false), 3000);
  };

  const renderPreviewContent = () => {
    if (!reportData) return null;

    if (mode === 'templates' && selectedTemplateId === 'decision-drift') {
      return (
        <div className="space-y-8">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-2xl font-black text-slate-900 tracking-tight">Decision Drift Analysis</h3>
              <p className="text-sm font-bold text-slate-400 uppercase tracking-widest mt-1">Jan 2026 - Jun 2026 • All Teams</p>
            </div>
            <div className="flex items-center gap-2">
              <span className="px-4 py-1.5 bg-rose-50 text-rose-700 text-[10px] font-black uppercase tracking-widest rounded-full border border-rose-100 shadow-sm">
                Warning: Rubber-stamping detected
              </span>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-6">
            <motion.div 
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm"
            >
              <div className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-2">Avg Time-on-Task (AI Cases)</div>
              <div className="text-3xl font-black text-slate-900">0.5 mins</div>
              <div className="text-[10px] font-black text-rose-600 mt-2 flex items-center gap-1 uppercase tracking-wider">
                <ArrowDownRight className="w-3.5 h-3.5" /> -88% from Jan
              </div>
            </motion.div>
            <motion.div 
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm"
            >
              <div className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-2">Override Rate</div>
              <div className="text-3xl font-black text-slate-900">2.0%</div>
              <div className="text-[10px] font-black text-rose-600 mt-2 flex items-center gap-1 uppercase tracking-wider">
                <ArrowDownRight className="w-3.5 h-3.5" /> -83% from Jan
              </div>
            </motion.div>
            <motion.div 
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
              className="bg-white p-6 rounded-2xl border border-slate-200 shadow-sm"
            >
              <div className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-2">AI Accuracy (Audit)</div>
              <div className="text-3xl font-black text-slate-900">96.0%</div>
              <div className="text-[10px] font-black text-emerald-600 mt-2 flex items-center gap-1 uppercase tracking-wider">
                <ArrowUpRight className="w-3.5 h-3.5" /> +2% from Jan
              </div>
            </motion.div>
          </div>

          <div className="bg-white p-8 rounded-2xl border border-slate-200 shadow-sm h-[450px]">
            <h4 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-8">Override Rate vs. Time-on-Task</h4>
            <ResponsiveContainer width="100%" height="100%">
              <RechartsLineChart data={reportData}>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
                <XAxis dataKey="month" axisLine={false} tickLine={false} tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 900 }} />
                <YAxis yAxisId="left" axisLine={false} tickLine={false} tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 900 }} />
                <YAxis yAxisId="right" orientation="right" axisLine={false} tickLine={false} tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 900 }} />
                <Tooltip 
                  contentStyle={{ borderRadius: '16px', border: 'none', boxShadow: '0 10px 15px -3px rgb(0 0 0 / 0.1)', padding: '12px' }}
                />
                <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px', fontSize: '10px', fontWeight: 900, textTransform: 'uppercase', letterSpacing: '0.1em' }} />
                <Line yAxisId="left" type="monotone" dataKey="overrideRate" name="Override Rate (%)" stroke="#3b82f6" strokeWidth={4} dot={{ r: 6, strokeWidth: 2, fill: '#fff' }} activeDot={{ r: 8 }} />
                <Line yAxisId="right" type="monotone" dataKey="timeOnTask" name="Time on Task (mins)" stroke="#f59e0b" strokeWidth={4} dot={{ r: 6, strokeWidth: 2, fill: '#fff' }} activeDot={{ r: 8 }} />
              </RechartsLineChart>
            </ResponsiveContainer>
          </div>

          <div className="bg-amber-50 border border-amber-200 rounded-2xl p-6 shadow-sm">
            <h4 className="text-[10px] font-black text-amber-900 uppercase tracking-[0.2em] mb-3 flex items-center gap-2">
              <BrainCircuit className="w-4 h-4" />
              AI Insights & Annotations
            </h4>
            <p className="text-sm text-amber-800 font-medium leading-relaxed">
              <strong className="font-black">Anomaly Detected:</strong> While AI accuracy has remained stable at ~96%, the human override rate has plummeted to 2%, and average time-on-task has dropped to 30 seconds. This strongly indicates that workers are no longer meaningfully reviewing AI suggestions and are instead "rubber-stamping" approvals.
            </p>
          </div>
        </div>
      );
    }

    if (mode === 'templates' && selectedTemplateId === 'regulatory-comparison') {
      return (
        <div className="space-y-8">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-2xl font-black text-slate-900 tracking-tight">Regulatory Version Comparison</h3>
              <p className="text-sm font-bold text-slate-400 uppercase tracking-widest mt-1">FY2024 vs FY2025 Rules</p>
            </div>
          </div>
          <div className="bg-white p-8 rounded-2xl border border-slate-200 shadow-sm h-[450px]">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={reportData}>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
                <XAxis dataKey="ruleVersion" axisLine={false} tickLine={false} tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 900 }} />
                <YAxis axisLine={false} tickLine={false} tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 900 }} />
                <Tooltip cursor={{ fill: '#f8fafc' }} contentStyle={{ borderRadius: '16px', border: 'none', boxShadow: '0 10px 15px -3px rgb(0 0 0 / 0.1)', padding: '12px' }} />
                <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px', fontSize: '10px', fontWeight: 900, textTransform: 'uppercase', letterSpacing: '0.1em' }} />
                <Bar dataKey="approved" name="Approved Cases" fill="#10b981" radius={[6, 6, 0, 0]} />
                <Bar dataKey="denied" name="Denied Cases" fill="#f43f5e" radius={[6, 6, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      );
    }

    return (
      <div className="bg-white p-10 rounded-3xl border border-slate-200 shadow-sm">
        <h3 className="text-xl font-black text-slate-900 mb-8 tracking-tight">Custom Report Results</h3>
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-slate-100">
                <th className="py-4 px-6 text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Dimension</th>
                {selectedMetrics.map(m => (
                  <th key={m} className="py-4 px-6 text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] text-right">{m}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50">
              <tr className="hover:bg-slate-50 transition-colors">
                <td className="py-5 px-6 text-sm font-bold text-slate-900">Sample Data Row</td>
                {selectedMetrics.map(m => (
                  <td key={m} className="py-5 px-6 text-sm font-black text-slate-600 text-right">1,240</td>
                ))}
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  return (
    <div className="flex-1 flex flex-col lg:flex-row bg-[#f8fafc] overflow-hidden font-sans">
      <div className="w-full lg:w-[400px] bg-white border-b lg:border-b-0 lg:border-r border-slate-200 flex flex-col shrink-0 z-10 max-h-[50vh] lg:max-h-none">
        <div className="p-4 sm:p-8 border-b border-slate-50">
          <div className="flex items-center gap-4 mb-4 sm:mb-8">
            <div className="w-10 h-10 sm:w-12 sm:h-12 bg-blue-600 rounded-xl sm:rounded-2xl flex items-center justify-center text-white shadow-lg shadow-blue-100">
              <BarChart3 className="w-5 h-5 sm:w-6 sm:h-6" />
            </div>
            <div>
              <h2 className="text-lg sm:text-xl font-black text-slate-900 tracking-tight">Report Builder</h2>
              <p className="text-[9px] sm:text-[10px] font-black text-slate-400 uppercase tracking-widest">Analytics Engine v2.0</p>
            </div>
          </div>
          
          <div className="flex bg-slate-100 p-1.5 rounded-2xl">
            <button 
              onClick={() => { setMode('templates'); setShowPreview(false); }}
              className={`flex-1 py-2 sm:py-2.5 text-[10px] sm:text-xs font-black uppercase tracking-wider rounded-xl transition-all ${mode === 'templates' ? 'bg-white text-slate-900 shadow-sm' : 'text-slate-500 hover:text-slate-700'}`}
            >
              Templates
            </button>
            <button 
              onClick={() => { setMode('custom'); setShowPreview(false); }}
              className={`flex-1 py-2 sm:py-2.5 text-[10px] sm:text-xs font-black uppercase tracking-wider rounded-xl transition-all ${mode === 'custom' ? 'bg-white text-slate-900 shadow-sm' : 'text-slate-500 hover:text-slate-700'}`}
            >
              Custom
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4 sm:p-8">
          {mode === 'templates' ? (
            <div className="space-y-4">
              <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4 sm:mb-6">Pre-built Reports</h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-1 gap-4">
                {templates.map(template => {
                  const Icon = ICON_MAP[template.icon] || LayoutGrid;
                  return (
                    <motion.div 
                      whileHover={{ y: -2 }}
                      whileTap={{ scale: 0.98 }}
                      key={template.id}
                      onClick={() => { setSelectedTemplateId(template.id); setShowPreview(false); }}
                      className={`p-4 sm:p-5 rounded-2xl border-2 cursor-pointer transition-all ${selectedTemplateId === template.id ? 'border-blue-600 bg-blue-50 shadow-lg shadow-blue-50' : 'border-slate-100 bg-white hover:border-slate-300 hover:shadow-md'}`}
                    >
                      <div className="flex items-center gap-4 mb-3">
                        <div className={`p-2 sm:p-2.5 rounded-xl ${selectedTemplateId === template.id ? 'bg-blue-600 text-white shadow-md' : 'bg-slate-50 text-slate-400'}`}>
                          <Icon className="w-4 h-4 sm:w-5 sm:h-5" />
                        </div>
                        <h4 className={`font-black text-xs sm:text-sm tracking-tight ${selectedTemplateId === template.id ? 'text-blue-900' : 'text-slate-900'}`}>{template.title}</h4>
                      </div>
                      <p className={`text-[10px] sm:text-xs font-medium leading-relaxed ${selectedTemplateId === template.id ? 'text-blue-800' : 'text-slate-500'}`}>
                        {template.description}
                      </p>
                    </motion.div>
                  );
                })}
              </div>
            </div>
          ) : (
            <div className="space-y-8 sm:space-y-10">
              <div>
                <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4 sm:mb-6">1. Select Metrics</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-1 gap-3">
                  {['Case Count', 'Approval Rate', 'Avg Processing Time', 'Override Rate'].map(metric => (
                    <label key={metric} className="flex items-center gap-4 p-3 sm:p-4 bg-white border border-slate-200 rounded-2xl hover:border-blue-400 hover:shadow-md cursor-pointer transition-all group">
                      <input 
                        type="checkbox" 
                        className="w-4 h-4 sm:w-5 sm:h-5 text-blue-600 rounded-lg border-slate-300 focus:ring-blue-500 transition-all"
                        checked={selectedMetrics.includes(metric)}
                        onChange={(e) => {
                          if (e.target.checked) setSelectedMetrics([...selectedMetrics, metric]);
                          else setSelectedMetrics(selectedMetrics.filter(m => m !== metric));
                        }}
                      />
                      <span className="text-xs sm:text-sm font-black text-slate-700 group-hover:text-slate-900">{metric}</span>
                    </label>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4 sm:mb-6">2. Select Dimensions</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-1 gap-3">
                  {['Time (Month)', 'Team', 'Regulatory Version', 'AI Agent'].map(dim => (
                    <label key={dim} className="flex items-center gap-4 p-3 sm:p-4 bg-white border border-slate-200 rounded-2xl hover:border-blue-400 hover:shadow-md cursor-pointer transition-all group">
                      <input 
                        type="checkbox" 
                        className="w-4 h-4 sm:w-5 sm:h-5 text-blue-600 rounded-lg border-slate-300 focus:ring-blue-500 transition-all"
                        checked={selectedDimensions.includes(dim)}
                        onChange={(e) => {
                          if (e.target.checked) setSelectedDimensions([...selectedDimensions, dim]);
                          else setSelectedDimensions(selectedDimensions.filter(d => d !== dim));
                        }}
                      />
                      <span className="text-xs sm:text-sm font-black text-slate-700 group-hover:text-slate-900">{dim}</span>
                    </label>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4 sm:mb-6">3. Visualization</h3>
                <div className="grid grid-cols-3 gap-3">
                  <button onClick={() => setSelectedViz('bar')} className={`p-3 sm:p-4 rounded-2xl border flex flex-col items-center gap-2 sm:gap-3 transition-all active:scale-95 ${selectedViz === 'bar' ? 'border-blue-600 bg-blue-50 text-blue-700 shadow-md' : 'border-slate-200 text-slate-400 hover:bg-slate-50'}`}>
                    <BarChart3 className="w-5 h-5 sm:w-6 sm:h-6" />
                    <span className="text-[8px] sm:text-[9px] font-black uppercase tracking-widest">Bar</span>
                  </button>
                  <button onClick={() => setSelectedViz('line')} className={`p-3 sm:p-4 rounded-2xl border flex flex-col items-center gap-2 sm:gap-3 transition-all active:scale-95 ${selectedViz === 'line' ? 'border-blue-600 bg-blue-50 text-blue-700 shadow-md' : 'border-slate-200 text-slate-400 hover:bg-slate-50'}`}>
                    <LineChart className="w-5 h-5 sm:w-6 sm:h-6" />
                    <span className="text-[8px] sm:text-[9px] font-black uppercase tracking-widest">Line</span>
                  </button>
                  <button onClick={() => setSelectedViz('table')} className={`p-3 sm:p-4 rounded-2xl border flex flex-col items-center gap-2 sm:gap-3 transition-all active:scale-95 ${selectedViz === 'table' ? 'border-blue-600 bg-blue-50 text-blue-700 shadow-md' : 'border-slate-200 text-slate-400 hover:bg-slate-50'}`}>
                    <TableIcon className="w-5 h-5 sm:w-6 sm:h-6" />
                    <span className="text-[8px] sm:text-[9px] font-black uppercase tracking-widest">Table</span>
                  </button>
                </div>
              </div>

              <div className="pt-6 border-t border-slate-100">
                <label className="flex items-center justify-between cursor-pointer group">
                  <div className="space-y-0.5">
                    <span className="text-xs font-black text-slate-900 uppercase tracking-wider">Live Preview</span>
                    <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">Update charts in real-time</p>
                  </div>
                  <div className="relative inline-flex items-center">
                    <input 
                      type="checkbox" 
                      className="sr-only peer"
                      checked={livePreview}
                      onChange={(e) => setLivePreview(e.target.checked)}
                    />
                    <div className="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
                  </div>
                </label>
              </div>
            </div>
          )}
        </div>

        <div className="p-4 sm:p-8 border-t border-slate-100 bg-slate-50/50">
          <button 
            onClick={handleGenerate}
            disabled={mode === 'templates' ? !selectedTemplateId : selectedMetrics.length === 0}
            className="w-full py-3 sm:py-4 bg-blue-600 text-white rounded-2xl font-black uppercase tracking-[0.2em] text-[10px] sm:text-xs hover:bg-blue-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-3 shadow-xl shadow-blue-100 active:scale-95"
          >
            {isGenerating ? <Activity className="w-4 h-4 sm:w-5 sm:h-5 animate-spin" /> : <LayoutGrid className="w-4 h-4 sm:w-5 sm:h-5" />}
            Generate Report
          </button>
        </div>
      </div>

      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="h-auto lg:h-20 bg-white border-b border-slate-200 px-4 sm:px-10 py-4 lg:py-0 flex flex-col lg:flex-row lg:items-center justify-between shrink-0 shadow-sm gap-4">
          <div className="flex flex-wrap items-center gap-3 sm:gap-4">
            <div className="flex items-center gap-2 sm:gap-3 px-3 sm:px-4 py-2 sm:py-2.5 bg-slate-50 rounded-xl border border-slate-200 cursor-pointer hover:bg-white transition-all">
              <Calendar className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-400" />
              <span className="text-[10px] sm:text-xs font-black text-slate-700 uppercase tracking-wider">Last 6 Months</span>
              <ChevronDown className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-400" />
            </div>
            <div className="flex items-center gap-2 sm:gap-3 px-3 sm:px-4 py-2 sm:py-2.5 bg-slate-50 rounded-xl border border-slate-200 cursor-pointer hover:bg-white transition-all">
              <Filter className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-400" />
              <span className="text-[10px] sm:text-xs font-black text-slate-700 uppercase tracking-wider">Add Filter</span>
              <Plus className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-slate-400" />
            </div>
          </div>
          
          <div className="flex items-center gap-2 sm:gap-3">
            <button 
              onClick={handlePin}
              disabled={!showPreview || isPinned}
              className={`flex-1 lg:flex-none flex items-center justify-center gap-2 px-3 sm:px-5 py-2 sm:py-2.5 border rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider transition-all disabled:opacity-50 active:scale-95 shadow-sm ${isPinned ? 'bg-emerald-50 border-emerald-200 text-emerald-600' : 'bg-white border-slate-200 text-slate-700 hover:bg-slate-50'}`}
            >
              {isPinned ? <Check className="w-3.5 h-3.5 sm:w-4 sm:h-4" /> : <Pin className="w-3.5 h-3.5 sm:w-4 sm:h-4" />}
              {isPinned ? 'Pinned' : 'Pin to Dashboard'}
            </button>
            <button 
              onClick={() => setShowScheduleModal(true)}
              disabled={!showPreview}
              className="flex-1 lg:flex-none flex items-center justify-center gap-2 px-3 sm:px-5 py-2 sm:py-2.5 border border-slate-200 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider text-slate-700 hover:bg-slate-50 transition-all disabled:opacity-50 active:scale-95 shadow-sm"
            >
              <Clock className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
              Schedule
            </button>
            <button 
              onClick={handleShare}
              disabled={!showPreview || isSharing}
              className="flex-1 lg:flex-none flex items-center justify-center gap-2 px-3 sm:px-5 py-2 sm:py-2.5 border border-slate-200 rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider text-slate-700 hover:bg-slate-50 transition-all disabled:opacity-50 active:scale-95 shadow-sm"
            >
              {isSharing ? <Activity className="w-3.5 h-3.5 sm:w-4 sm:h-4 animate-spin" /> : <Share2 className="w-3.5 h-3.5 sm:w-4 sm:h-4" />}
              Share
            </button>
            <button 
              onClick={handleExport}
              disabled={!showPreview || isExporting}
              className="flex-1 lg:flex-none flex items-center justify-center gap-2 px-4 sm:px-6 py-2 sm:py-2.5 bg-slate-900 text-white rounded-xl text-[10px] sm:text-xs font-black uppercase tracking-wider hover:bg-black transition-all disabled:opacity-50 active:scale-95 shadow-lg shadow-slate-200"
            >
              {isExporting ? <Activity className="w-3.5 h-3.5 sm:w-4 sm:h-4 animate-spin" /> : <Download className="w-3.5 h-3.5 sm:w-4 sm:h-4" />}
              Export
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4 sm:p-10 bg-[#f8fafc]">
          <AnimatePresence mode="wait">
            {isGenerating ? (
              <motion.div 
                key="loading"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="h-full flex flex-col items-center justify-center text-slate-400"
              >
                <div className="relative">
                  <div className="w-20 h-20 border-4 border-blue-100 rounded-full animate-pulse"></div>
                  <Activity className="w-10 h-10 animate-spin absolute inset-0 m-auto text-blue-600" />
                </div>
                <p className="font-black text-slate-900 uppercase tracking-[0.2em] text-xs mt-8">Querying Data Warehouse</p>
                <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest mt-2">Processing multi-dimensional aggregates...</p>
              </motion.div>
            ) : showPreview ? (
              <motion.div 
                key="preview"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="max-w-6xl mx-auto"
              >
                {renderPreviewContent()}
              </motion.div>
            ) : (
              <motion.div 
                key="empty"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="h-full flex flex-col items-center justify-center text-slate-400"
              >
                <div className="w-32 h-32 bg-white rounded-[40px] flex items-center justify-center mb-8 shadow-xl shadow-slate-200/50 border border-slate-100">
                  <BarChart3 className="w-12 h-12 text-slate-200" />
                </div>
                <h3 className="text-xl font-black text-slate-900 mb-3 tracking-tight">Report Preview</h3>
                <p className="max-w-md text-center text-sm font-medium text-slate-500 leading-relaxed">
                  Configure your report parameters in the left panel and click <span className="font-black text-slate-900">Generate</span> to visualize the results here.
                </p>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      <AnimatePresence>
        {showScheduleModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-slate-900/40 backdrop-blur-md"
              onClick={() => setShowScheduleModal(false)}
            />
            <motion.div 
              initial={{ opacity: 0, scale: 0.9, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.9, y: 20 }}
              className="relative bg-white rounded-[32px] shadow-2xl w-full max-w-lg overflow-hidden border border-slate-100"
            >
              <div className="px-10 py-8 border-b border-slate-50 bg-slate-50/50 flex items-center justify-between">
                <div>
                  <h3 className="text-xl font-black text-slate-900 tracking-tight flex items-center gap-3">
                    <Clock className="w-6 h-6 text-blue-600" />
                    Schedule Report
                  </h3>
                  <p className="text-[10px] font-black text-slate-400 uppercase tracking-widest mt-1">Automated Delivery System</p>
                </div>
                <button 
                  onClick={() => setShowScheduleModal(false)}
                  className="p-2 hover:bg-slate-200 rounded-xl text-slate-400 transition-all active:scale-90"
                >
                  <X className="w-6 h-6" />
                </button>
              </div>
              <div className="p-10 space-y-8">
                <div className="space-y-3">
                  <label className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Recurrence Pattern</label>
                  <select className="w-full p-4 bg-slate-50 border border-slate-200 rounded-2xl font-bold text-slate-700 focus:ring-2 focus:ring-blue-500 outline-none transition-all">
                    <option>Weekly (Monday 8:00 AM)</option>
                    <option>Monthly (1st of Month)</option>
                    <option>Quarterly</option>
                  </select>
                </div>
                <div className="space-y-3">
                  <label className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Recipients</label>
                  <input type="text" placeholder="Email addresses (comma separated)" className="w-full p-4 bg-slate-50 border border-slate-200 rounded-2xl font-bold text-slate-700 focus:ring-2 focus:ring-blue-500 outline-none transition-all" />
                </div>
                <div className="space-y-4">
                  <label className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">Export Formats</label>
                  <div className="flex gap-6">
                    <label className="flex items-center gap-3 cursor-pointer group">
                      <input type="checkbox" defaultChecked className="w-5 h-5 text-blue-600 rounded-lg border-slate-300 focus:ring-blue-500" /> 
                      <span className="text-xs font-black text-slate-700 uppercase tracking-widest group-hover:text-slate-900">PDF</span>
                    </label>
                    <label className="flex items-center gap-3 cursor-pointer group">
                      <input type="checkbox" className="w-5 h-5 text-blue-600 rounded-lg border-slate-300 focus:ring-blue-500" /> 
                      <span className="text-xs font-black text-slate-700 uppercase tracking-widest group-hover:text-slate-900">Excel</span>
                    </label>
                    <label className="flex items-center gap-3 cursor-pointer group">
                      <input type="checkbox" className="w-5 h-5 text-blue-600 rounded-lg border-slate-300 focus:ring-blue-500" /> 
                      <span className="text-xs font-black text-slate-700 uppercase tracking-widest group-hover:text-slate-900">CSV</span>
                    </label>
                  </div>
                </div>
              </div>
              <div className="px-10 py-8 bg-slate-50/50 border-t border-slate-50 flex justify-end gap-4">
                <button onClick={() => setShowScheduleModal(false)} className="px-6 py-3 text-xs font-black uppercase tracking-wider text-slate-500 hover:bg-slate-200 rounded-xl transition-all active:scale-95">Cancel</button>
                <button onClick={handleSaveSchedule} className="px-8 py-3 text-xs font-black uppercase tracking-wider bg-blue-600 text-white hover:bg-blue-700 rounded-xl transition-all shadow-xl shadow-blue-100 active:scale-95">Save Schedule</button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

function ArrowDownRight(props: any) {
  return <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}><path d="m7 7 10 10"/><path d="M17 7v10H7"/></svg>;
}
function ArrowUpRight(props: any) {
  return <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" {...props}><path d="M7 17 17 7"/><path d="M7 7h10v10"/></svg>;
}
