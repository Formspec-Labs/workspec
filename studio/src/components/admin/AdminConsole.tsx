import React, { useState, useEffect } from 'react';
import { 
  Shield, 
  Cpu, 
  Users, 
  Calendar as CalendarIcon, 
  Activity, 
  Plus, 
  MoreVertical, 
  ExternalLink, 
  AlertTriangle, 
  CheckCircle2, 
  Clock, 
  FileText, 
  Trash2,
  ChevronRight,
  Search,
  Filter,
  ArrowUpRight,
  ArrowDownRight,
  Settings,
  Layers
} from 'lucide-react';
import { useGovernance } from '../../context/WosContext';
import type { AgentView, DelegationEntry, DeonticConstraintView, QualityControlsView, PipelineView, VerificationReportView, EquityConfigView, PolicyVersionView, CalendarEventView, ServiceHealthView } from '../../services/WosPorts';
import { motion, AnimatePresence } from 'motion/react';

import { ConfirmationModal } from '../ui/ConfirmationModal';

type AdminTab = 'agents' | 'deontic' | 'quality' | 'pipelines' | 'equity' | 'verification' | 'delegations' | 'regulatory' | 'calendar' | 'health';
type AdminPersona = 'it-admin' | 'policy-admin' | 'ops-admin';

const DEFAULT_WORKFLOW_URL = 'https://agency.gov/workflows/benefits-adjudication';

export function AdminConsole() {
  const governance = useGovernance();
  const [activeTab, setActiveTab] = useState<AdminTab>('agents');
  const [activePersona, setActivePersona] = useState<AdminPersona>('it-admin');
  const [agents, setAgents] = useState<AgentView[]>([]);
  const [deonticConstraints, setDeonticConstraints] = useState<DeonticConstraintView[]>([]);
  const [qualityControls, setQualityControls] = useState<QualityControlsView | null>(null);
  const [pipelines, setPipelines] = useState<PipelineView[]>([]);
  const [verificationReport, setVerificationReport] = useState<VerificationReportView | null>(null);
  const [equityConfig, setEquityConfig] = useState<EquityConfigView | null>(null);
  const [delegations, setDelegations] = useState<DelegationEntry[]>([]);
  const [versions, setVersions] = useState<PolicyVersionView[]>([]);
  const [calendarEvents, setCalendarEvents] = useState<CalendarEventView[]>([]);
  const [healthStatus, setHealthStatus] = useState<ServiceHealthView[]>([]);

  const [isRegisterAgentOpen, setIsRegisterAgentOpen] = useState(false);
  const [isCreateDelegationOpen, setIsCreateDelegationOpen] = useState(false);
  const [isDefineVersionOpen, setIsDefineVersionOpen] = useState(false);
  const [isAddHolidayOpen, setIsAddHolidayOpen] = useState(false);
  const [isMigrationPolicyOpen, setIsMigrationPolicyOpen] = useState(false);
  const [isViewAffectedCasesOpen, setIsViewAffectedCasesOpen] = useState(false);

  const [agentForm, setAgentForm] = useState({ name: '', type: 'llm', version: '1.0.0' });
  const [delegationForm, setDelegationForm] = useState({ delegator: '', delegate: '', authorityType: 'Administrative', legalInstrument: '' });
  const [versionForm, setVersionForm] = useState({ version: '', effectiveDate: '' });
  const [holidayForm, setHolidayForm] = useState({ title: '', date: '', type: 'federal' });

  useEffect(() => {
    governance.listAgents(DEFAULT_WORKFLOW_URL).then(setAgents);
    governance.listDeonticConstraints(DEFAULT_WORKFLOW_URL).then(setDeonticConstraints);
    governance.getQualityControls(DEFAULT_WORKFLOW_URL).then(setQualityControls);
    governance.listPipelines(DEFAULT_WORKFLOW_URL).then(setPipelines);
    governance.getVerificationReport(DEFAULT_WORKFLOW_URL).then(setVerificationReport);
    governance.getEquityConfig(DEFAULT_WORKFLOW_URL).then(setEquityConfig);
    governance.listDelegations(DEFAULT_WORKFLOW_URL).then(setDelegations);
    governance.listPolicyVersions(DEFAULT_WORKFLOW_URL).then(setVersions);
    governance.listCalendarEvents(DEFAULT_WORKFLOW_URL).then(setCalendarEvents);
    governance.getHealthStatus().then(setHealthStatus);
  }, [governance]);

  const handleRegisterAgent = async () => {
    governance.listAgents(DEFAULT_WORKFLOW_URL).then(setAgents);
    setIsRegisterAgentOpen(false);
    setAgentForm({ name: '', type: 'llm', version: '1.0.0' });
  };

  const handleCreateDelegation = async () => {
    governance.listDelegations(DEFAULT_WORKFLOW_URL).then(setDelegations);
    setIsCreateDelegationOpen(false);
    setDelegationForm({ delegator: '', delegate: '', authorityType: 'Administrative', legalInstrument: '' });
  };

  const handleDefineVersion = async () => {
    governance.listPolicyVersions(DEFAULT_WORKFLOW_URL).then(setVersions);
    setIsDefineVersionOpen(false);
    setVersionForm({ version: '', effectiveDate: '' });
  };

  const [migrationPolicy, setMigrationPolicy] = useState({ type: 'grandfather', effectiveDate: '' });
  const [selectedVersionId, setSelectedVersionId] = useState<string | null>(null);

  const handleConfigureMigration = async () => {
    setIsMigrationPolicyOpen(false);
  };

  const handleGenerateImpactReport = async () => {
    setIsViewAffectedCasesOpen(false);
  };

  const handleAddHoliday = async () => {
    governance.listCalendarEvents(DEFAULT_WORKFLOW_URL).then(setCalendarEvents);
    setIsAddHolidayOpen(false);
    setHolidayForm({ title: '', date: '', type: 'federal' });
  };

  return (
    <div className="flex-1 flex flex-col bg-gray-50 overflow-hidden">
      <div className="bg-white border-b border-gray-200 px-4 sm:px-8 py-6 shrink-0">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div>
            <h1 className="text-xl sm:text-2xl font-bold text-gray-900 tracking-tight">System Administration</h1>
            <p className="text-xs sm:text-sm text-gray-500 mt-1">Configure AI agents, delegations, regulatory rules, and system health.</p>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center bg-gray-100 p-1 rounded-xl border border-gray-200">
              <button 
                onClick={() => { setActivePersona('it-admin'); setActiveTab('agents'); }}
                className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all ${activePersona === 'it-admin' ? 'bg-white text-slate-900 shadow-sm' : 'text-gray-400 hover:text-gray-600'}`}
              >
                IT Admin
              </button>
              <button 
                onClick={() => { setActivePersona('policy-admin'); setActiveTab('regulatory'); }}
                className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all ${activePersona === 'policy-admin' ? 'bg-white text-slate-900 shadow-sm' : 'text-gray-400 hover:text-gray-600'}`}
              >
                Policy
              </button>
              <button 
                onClick={() => { setActivePersona('ops-admin'); setActiveTab('calendar'); }}
                className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all ${activePersona === 'ops-admin' ? 'bg-white text-slate-900 shadow-sm' : 'text-gray-400 hover:text-gray-600'}`}
              >
                Ops
              </button>
            </div>
            <div className="h-8 w-px bg-gray-200 mx-1" />
            <div className="flex items-center gap-2 px-3 py-1.5 bg-emerald-50 text-emerald-700 rounded-lg border border-emerald-100 text-[10px] sm:text-xs font-bold">
              <Shield className="w-3.5 h-3.5" />
              Verified
            </div>
          </div>
        </div>

        <div className="max-w-7xl mx-auto mt-8 flex items-center gap-1 overflow-x-auto no-scrollbar pb-2 sm:pb-0">
          {activePersona === 'it-admin' && (
            <>
              <TabButton active={activeTab === 'agents'} onClick={() => setActiveTab('agents')} icon={<Cpu className="w-4 h-4" />} label="Agents" />
              <TabButton active={activeTab === 'verification'} onClick={() => setActiveTab('verification')} icon={<CheckCircle2 className="w-4 h-4" />} label="Verification" />
              <TabButton active={activeTab === 'health'} onClick={() => setActiveTab('health')} icon={<Activity className="w-4 h-4" />} label="Health" />
            </>
          )}
          {activePersona === 'policy-admin' && (
            <>
              <TabButton active={activeTab === 'deontic'} onClick={() => setActiveTab('deontic')} icon={<Shield className="w-4 h-4" />} label="Constraints" />
              <TabButton active={activeTab === 'quality'} onClick={() => setActiveTab('quality')} icon={<CheckCircle2 className="w-4 h-4" />} label="Quality" />
              <TabButton active={activeTab === 'pipelines'} onClick={() => setActiveTab('pipelines')} icon={<Layers className="w-4 h-4" />} label="Pipelines" />
              <TabButton active={activeTab === 'equity'} onClick={() => setActiveTab('equity')} icon={<Shield className="w-4 h-4" />} label="Equity" />
              <TabButton active={activeTab === 'regulatory'} onClick={() => setActiveTab('regulatory')} icon={<FileText className="w-4 h-4" />} label="Regulatory" />
              <TabButton active={activeTab === 'delegations'} onClick={() => setActiveTab('delegations')} icon={<Users className="w-4 h-4" />} label="Delegations" />
            </>
          )}
          {activePersona === 'ops-admin' && (
            <>
              <TabButton active={activeTab === 'calendar'} onClick={() => setActiveTab('calendar')} icon={<CalendarIcon className="w-4 h-4" />} label="Calendar" />
            </>
          )}
        </div>
      </div>

      <main className="flex-1 overflow-y-auto p-8">
        <div className="max-w-7xl mx-auto">
          <AnimatePresence mode="wait">
            {activeTab === 'agents' && <div key="agents"><AgentRegistry agents={agents} onRegister={() => setIsRegisterAgentOpen(true)} /></div>}
            {activeTab === 'deontic' && <div key="deontic"><DeonticConstraintsPanel constraints={deonticConstraints} /></div>}
            {activeTab === 'quality' && <div key="quality"><QualityControlsPanel controls={qualityControls} /></div>}
            {activeTab === 'pipelines' && <div key="pipelines"><PipelineViewerPanel pipelines={pipelines} /></div>}
            {activeTab === 'equity' && <div key="equity"><EquityGuardrailsPanel config={equityConfig} /></div>}
            {activeTab === 'verification' && <div key="verification"><VerificationReportPanel report={verificationReport} /></div>}
            {activeTab === 'delegations' && <div key="delegations"><DelegationPanel delegations={delegations} onRevoke={async (id) => { await governance.revokeDelegation(DEFAULT_WORKFLOW_URL, id); governance.listDelegations(DEFAULT_WORKFLOW_URL).then(setDelegations); }} onCreate={() => setIsCreateDelegationOpen(true)} /></div>}
            {activeTab === 'regulatory' && <div key="regulatory"><RegulatoryPanel versions={versions} onDefine={() => setIsDefineVersionOpen(true)} onConfigureMigration={(id) => { setSelectedVersionId(id); setIsMigrationPolicyOpen(true); }} /></div>}
            {activeTab === 'calendar' && <div key="calendar"><CalendarPanel events={calendarEvents} onAdd={() => setIsAddHolidayOpen(true)} onViewAffected={() => setIsViewAffectedCasesOpen(true)} /></div>}
            {activeTab === 'health' && <div key="health"><HealthPanel status={healthStatus} /></div>}
          </AnimatePresence>
        </div>
      </main>

      <ConfirmationModal 
        isOpen={isRegisterAgentOpen}
        onClose={() => setIsRegisterAgentOpen(false)}
        onConfirm={handleRegisterAgent}
        title="Register New AI Agent"
        message={
          <div className="space-y-4 mt-4 text-left">
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Agent Name</label>
              <input 
                type="text" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={agentForm.name}
                onChange={(e) => setAgentForm({ ...agentForm, name: e.target.value })}
                placeholder="e.g. DocumentExtractor"
              />
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Type</label>
              <select 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={agentForm.type}
                onChange={(e) => setAgentForm({ ...agentForm, type: e.target.value })}
              >
                <option value="llm">LLM</option>
                <option value="ml-model">ML Model</option>
                <option value="rules-engine">Rules Engine</option>
              </select>
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Version</label>
              <input 
                type="text" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={agentForm.version}
                onChange={(e) => setAgentForm({ ...agentForm, version: e.target.value })}
              />
            </div>
          </div>
        }
        confirmLabel="Register Agent"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isCreateDelegationOpen}
        onClose={() => setIsCreateDelegationOpen(false)}
        onConfirm={handleCreateDelegation}
        title="Create New Delegation"
        message={
          <div className="space-y-4 mt-4 text-left">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Delegator</label>
                <input 
                  type="text" 
                  className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                  value={delegationForm.delegator}
                  onChange={(e) => setDelegationForm({ ...delegationForm, delegator: e.target.value })}
                />
              </div>
              <div>
                <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Delegate</label>
                <input 
                  type="text" 
                  className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                  value={delegationForm.delegate}
                  onChange={(e) => setDelegationForm({ ...delegationForm, delegate: e.target.value })}
                />
              </div>
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Authority Type</label>
              <select 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={delegationForm.authorityType}
                onChange={(e) => setDelegationForm({ ...delegationForm, authorityType: e.target.value })}
              >
                <option value="Statutory">Statutory</option>
                <option value="Administrative">Administrative</option>
                <option value="Emergency">Emergency</option>
              </select>
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Legal Instrument ID</label>
              <input 
                type="text" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={delegationForm.legalInstrument}
                onChange={(e) => setDelegationForm({ ...delegationForm, legalInstrument: e.target.value })}
              />
            </div>
          </div>
        }
        confirmLabel="Create Delegation"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isDefineVersionOpen}
        onClose={() => setIsDefineVersionOpen(false)}
        onConfirm={handleDefineVersion}
        title="Define New Regulatory Version"
        message={
          <div className="space-y-4 mt-4 text-left">
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Version Name</label>
              <input 
                type="text" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={versionForm.version}
                onChange={(e) => setVersionForm({ ...versionForm, version: e.target.value })}
                placeholder="FY2026-Q3"
              />
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Effective Date</label>
              <input 
                type="date" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={versionForm.effectiveDate}
                onChange={(e) => setVersionForm({ ...versionForm, effectiveDate: e.target.value })}
              />
            </div>
          </div>
        }
        confirmLabel="Create Draft"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isAddHolidayOpen}
        onClose={() => setIsAddHolidayOpen(false)}
        onConfirm={handleAddHoliday}
        title="Add System Holiday"
        message={
          <div className="space-y-4 mt-4 text-left">
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Holiday Title</label>
              <input 
                type="text" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={holidayForm.title}
                onChange={(e) => setHolidayForm({ ...holidayForm, title: e.target.value })}
              />
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Date</label>
              <input 
                type="date" 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={holidayForm.date}
                onChange={(e) => setHolidayForm({ ...holidayForm, date: e.target.value })}
              />
            </div>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Type</label>
              <select 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={holidayForm.type}
                onChange={(e) => setHolidayForm({ ...holidayForm, type: e.target.value as any })}
              >
                <option value="federal">Federal</option>
                <option value="agency">Agency</option>
                <option value="other">Other</option>
              </select>
            </div>
          </div>
        }
        confirmLabel="Add Holiday"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isMigrationPolicyOpen}
        onClose={() => setIsMigrationPolicyOpen(false)}
        onConfirm={handleConfigureMigration}
        title="Configure Migration Policy"
        message={
          <div className="space-y-4 mt-4 text-left">
            <p className="text-xs text-gray-500">Define how existing active cases should be migrated to this regulatory version.</p>
            <div>
              <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Migration Strategy</label>
              <select 
                className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                value={migrationPolicy.type}
                onChange={(e) => setMigrationPolicy({ ...migrationPolicy, type: e.target.value })}
              >
                <option value="grandfather">Grandfathering (Keep on old rules)</option>
                <option value="immediate">Immediate (Move all active cases)</option>
                <option value="phased">Phased (By case priority/type)</option>
              </select>
            </div>
            {migrationPolicy.type === 'phased' && (
              <div>
                <label className="block text-xs font-bold text-gray-400 uppercase tracking-widest mb-1">Target Effective Date</label>
                <input 
                  type="date" 
                  className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                  value={migrationPolicy.effectiveDate}
                  onChange={(e) => setMigrationPolicy({ ...migrationPolicy, effectiveDate: e.target.value })}
                />
              </div>
            )}
          </div>
        }
        confirmLabel="Apply Policy"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isViewAffectedCasesOpen}
        onClose={() => setIsViewAffectedCasesOpen(false)}
        onConfirm={handleGenerateImpactReport}
        title="View Affected Cases"
        message={
          <div className="space-y-4 mt-4 text-left">
            <p className="text-xs text-gray-500">This will generate a detailed impact report for the selected calendar event.</p>
            <div className="p-4 bg-gray-50 rounded-lg border border-gray-100">
              <div className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-2">Estimated Impact</div>
              <div className="flex items-center justify-between">
                <span className="text-sm font-bold text-gray-900">124 Cases</span>
                <span className="text-xs text-amber-600 font-bold">+1 Day Delay</span>
              </div>
            </div>
          </div>
        }
        confirmLabel="Generate Report"
        variant="info"
      />
    </div>
  );
}

function TabButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return (
    <button 
      onClick={onClick}
      className={`flex items-center gap-2 px-4 py-2 text-sm font-bold rounded-lg transition-all ${active ? 'bg-[#141414] text-white shadow-md' : 'text-gray-500 hover:bg-gray-100'}`}
    >
      {icon}
      {label}
    </button>
  );
}

function AgentRegistry({ agents, onRegister }: { agents: AgentView[]; onRegister: () => void }) {
  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <h2 className="text-lg font-bold text-gray-900">Registered AI Agents</h2>
        <button onClick={onRegister} className="flex items-center justify-center gap-2 px-4 py-2 bg-[#141414] text-white rounded-lg text-sm font-bold hover:bg-black shadow-sm transition-all active:scale-95">
          <Plus className="w-4 h-4" />
          Register New Agent
        </button>
      </div>

      <div className="hidden lg:block bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-gray-50 border-b border-gray-200">
                <th className="px-6 py-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">Agent Name & Type</th>
                <th className="px-6 py-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">Version</th>
                <th className="px-6 py-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">Status</th>
                <th className="px-6 py-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">Confidence Floor</th>
                <th className="px-6 py-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">Capabilities</th>
                <th className="px-6 py-4"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {agents.map(agent => (
                <tr key={agent.id} className="hover:bg-gray-50 transition-colors group">
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-3">
                      <div className={`p-2 rounded-lg ${agent.type === 'llm' ? 'bg-purple-50 text-purple-600' : agent.type === 'ml-model' ? 'bg-blue-50 text-blue-600' : 'bg-amber-50 text-amber-600'}`}>
                        <Cpu className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-sm font-bold text-gray-900">{agent.name}</div>
                        <div className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">{agent.type}</div>
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4">
                    <span className="text-xs font-mono bg-gray-100 px-1.5 py-0.5 rounded border border-gray-200">{agent.version}</span>
                  </td>
                  <td className="px-6 py-4">
                    <span className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider ${
                      agent.status === 'active' ? 'bg-emerald-50 text-emerald-700' : 
                      agent.status === 'deprecated' ? 'bg-amber-50 text-amber-700' : 
                      'bg-gray-100 text-gray-500'
                    }`}>
                      {agent.status}
                    </span>
                  </td>
                  <td className="px-6 py-4">
                    <span className="text-sm font-bold text-gray-900">
                      {agent.confidenceFloor != null ? `${Math.round(agent.confidenceFloor * 100)}%` : '—'}
                    </span>
                  </td>
                  <td className="px-6 py-4">
                    <div className="flex flex-wrap gap-1">
                      {agent.capabilities.slice(0, 3).map(c => (
                        <span key={c.name} className="px-1.5 py-0.5 bg-blue-50 text-blue-700 rounded text-[9px] font-bold border border-blue-100">
                          {c.name}
                        </span>
                      ))}
                      {agent.capabilities.length > 3 && (
                        <span className="px-1.5 py-0.5 bg-gray-50 text-gray-500 rounded text-[9px] font-bold">+{agent.capabilities.length - 3}</span>
                      )}
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right">
                    <button className="p-2 hover:bg-gray-200 rounded-lg text-gray-400 transition-colors">
                      <MoreVertical className="w-4 h-4" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <div className="lg:hidden grid grid-cols-1 md:grid-cols-2 gap-4">
        {agents.map(agent => (
          <div key={agent.id} className="bg-white rounded-xl border border-gray-200 shadow-sm p-5 space-y-4">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className={`p-2.5 rounded-xl ${agent.type === 'llm' ? 'bg-purple-50 text-purple-600' : agent.type === 'ml-model' ? 'bg-blue-50 text-blue-600' : 'bg-amber-50 text-amber-600'}`}>
                  <Cpu className="w-5 h-5" />
                </div>
                <div>
                  <div className="text-sm font-bold text-gray-900">{agent.name}</div>
                  <div className="text-[10px] font-mono text-gray-400 uppercase tracking-wider">{agent.type} • v{agent.version}</div>
                </div>
              </div>
              <button className="p-2 hover:bg-gray-100 rounded-lg text-gray-400 transition-colors">
                <MoreVertical className="w-4 h-4" />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-4 py-4 border-y border-gray-50">
              <div>
                <div className="text-[9px] font-bold text-gray-400 uppercase tracking-widest mb-1">Status</div>
                <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[9px] font-bold uppercase tracking-wider ${
                  agent.status === 'active' ? 'bg-emerald-50 text-emerald-700' : 
                  agent.status === 'deprecated' ? 'bg-amber-50 text-amber-700' : 
                  'bg-gray-100 text-gray-500'
                }`}>
                  {agent.status}
                </span>
              </div>
              <div>
                <div className="text-[9px] font-bold text-gray-400 uppercase tracking-widest mb-1">Confidence</div>
                <span className="text-xs font-bold text-gray-900">
                  {agent.confidenceFloor != null ? `${Math.round(agent.confidenceFloor * 100)}%` : '—'}
                </span>
              </div>
              <div>
                <div className="text-[9px] font-bold text-gray-400 uppercase tracking-widest mb-1">Capabilities</div>
                <span className="text-xs font-bold text-gray-900">{agent.capabilities.length} registered</span>
              </div>
              <div>
                <div className="text-[9px] font-bold text-gray-400 uppercase tracking-widest mb-1">Autonomy</div>
                <span className="text-xs font-bold text-gray-900">{agent.capabilities[0]?.autonomy ?? '—'}</span>
              </div>
            </div>

            <button className="w-full py-2 bg-gray-50 hover:bg-gray-100 text-gray-600 rounded-lg text-xs font-bold transition-colors border border-gray-100">
              View Performance Logs
            </button>
          </div>
        ))}
      </div>
    </motion.div>
  );
}

function DelegationPanel({ delegations, onRevoke, onCreate }: { delegations: DelegationEntry[]; onRevoke: (id: string) => void; onCreate: () => void }) {
  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold text-gray-900">Active Delegations of Authority</h2>
        <button onClick={onCreate} className="flex items-center gap-2 px-4 py-2 bg-[#141414] text-white rounded-lg text-sm font-bold hover:bg-black shadow-sm">
          <Plus className="w-4 h-4" />
          Create New Delegation
        </button>
      </div>

      <div className="grid grid-cols-1 gap-4">
        {delegations.map(del => (
          <div key={del.id} className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 hover:shadow-md transition-shadow">
            <div className="flex items-start justify-between mb-6">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 bg-blue-50 rounded-full flex items-center justify-center text-blue-600">
                  <Users className="w-6 h-6" />
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-bold text-gray-900">{del.delegator}</span>
                    <ChevronRight className="w-4 h-4 text-gray-400" />
                    <span className="font-bold text-blue-600">{del.delegate}</span>
                  </div>
                  <div className="text-xs text-gray-500 mt-1">
                    Authority: <span className="font-medium text-gray-700">{del.authority ?? 'General'}</span>
                    {del.legalInstrument && <> • Instrument: <span className="font-mono text-gray-700">{del.legalInstrument}</span></>}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <span className={`px-2 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider ${del.status === 'active' ? 'bg-emerald-50 text-emerald-700' : 'bg-red-50 text-red-700'}`}>
                  {del.status}
                </span>
                <button 
                  onClick={() => onRevoke(del.id)}
                  className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors" 
                  title="Revoke Delegation"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-6 border-t border-gray-100">
              <div>
                <h4 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-2">Scope</h4>
                <div className="text-sm font-medium text-gray-700">{del.scope}</div>
              </div>
              <div>
                <h4 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-2">Effective Period</h4>
                <div className="flex items-center gap-3 text-xs font-medium text-gray-700">
                  <div>
                    <div className="text-[9px] text-gray-400 uppercase">Start</div>
                    {new Date(del.startDate).toLocaleDateString()}
                  </div>
                  <ChevronRight className="w-4 h-4 text-gray-300 mt-3" />
                  <div>
                    <div className="text-[9px] text-gray-400 uppercase">End</div>
                    {del.endDate ? new Date(del.endDate).toLocaleDateString() : 'Indefinite'}
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </motion.div>
  );
}

function RegulatoryPanel({ versions, onDefine, onConfigureMigration }: { versions: PolicyVersionView[]; onDefine: () => void; onConfigureMigration: (id: string) => void }) {
  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold text-gray-900">Regulatory Versions & Rules</h2>
        <button onClick={onDefine} className="flex items-center gap-2 px-4 py-2 bg-[#141414] text-white rounded-lg text-sm font-bold hover:bg-black shadow-sm">
          <Plus className="w-4 h-4" />
          Define New Version
        </button>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-8">
        <div className="relative h-24 flex items-center">
          <div className="absolute top-1/2 left-0 right-0 h-1 bg-gray-100 -translate-y-1/2"></div>
          <div className="flex-1 flex justify-around relative">
            {versions.map((v) => (
              <div key={v.id} className="relative flex flex-col items-center">
                <div className={`w-4 h-4 rounded-full border-4 border-white shadow-sm z-10 ${v.status === 'active' ? 'bg-blue-600 scale-125' : 'bg-gray-300'}`}></div>
                <div className="absolute top-8 text-center whitespace-nowrap">
                  <div className="text-sm font-bold text-gray-900">{v.label}</div>
                  <div className="text-[10px] text-gray-500">{new Date(v.effectiveDate).toLocaleDateString()}</div>
                </div>
                {v.status === 'active' && (
                  <div className="absolute -top-12 bg-blue-50 text-blue-600 px-3 py-1 rounded-full text-[10px] font-bold uppercase tracking-widest border border-blue-100">
                    Currently Active
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {versions.map(v => (
          <div key={v.id} className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between bg-gray-50">
              <div>
                <h3 className="text-sm font-bold text-gray-900">{v.label}</h3>
                <p className="text-[10px] text-gray-500">Effective from {new Date(v.effectiveDate).toLocaleDateString()}</p>
              </div>
              <span className={`px-2 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider ${v.status === 'active' ? 'bg-blue-50 text-blue-700' : 'bg-gray-100 text-gray-500'}`}>
                {v.status}
              </span>
            </div>
            <div className="p-6 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500">Parameters</span>
                <span className="text-sm font-bold text-gray-900">{v.parameterCount}</span>
              </div>
              <div className="flex flex-col gap-2">
                <button 
                  onClick={() => {
                    window.dispatchEvent(new CustomEvent('navigate-to-designer', { detail: { workflowId: v.id } }));
                  }}
                  className="w-full py-2.5 bg-slate-900 text-white text-xs font-bold rounded-lg transition-all active:scale-95 shadow-sm flex items-center justify-center gap-2"
                >
                  <Settings className="w-3.5 h-3.5" />
                  Edit Workflow Logic
                </button>
                <button 
                  onClick={() => onConfigureMigration(v.id)}
                  className="w-full py-2 text-xs font-bold text-blue-600 hover:bg-blue-50 rounded-lg transition-colors border border-slate-100 hover:border-blue-100"
                >
                  Configure Migration Policy
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </motion.div>
  );
}

function CalendarPanel({ events, onAdd, onViewAffected }: { events: CalendarEventView[]; onAdd: () => void; onViewAffected: () => void }) {
  const [impact] = useState<{ affectedCases: number; description: string }>({ affectedCases: 124, description: 'Holiday closure will delay processing deadlines for active cases in the Verification and Review stages.' });

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold text-gray-900">System Calendar &amp; Holidays</h2>
        <button onClick={onAdd} className="flex items-center gap-2 px-4 py-2 bg-[#141414] text-white rounded-lg text-sm font-bold hover:bg-black shadow-sm">
          <Plus className="w-4 h-4" />
          Add Holiday
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2 bg-white rounded-xl border border-gray-200 shadow-sm p-6">
          <div className="grid grid-cols-7 gap-px bg-gray-200 border border-gray-200 rounded-lg overflow-hidden">
            {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map(day => (
              <div key={day} className="bg-gray-50 p-2 text-center text-[10px] font-bold text-gray-400 uppercase tracking-widest">{day}</div>
            ))}
            {Array.from({ length: 35 }).map((_, i) => {
              const day = i - 2;
              const isToday = day === 10;
              const hasEvent = events.find(e => new Date(e.date).getDate() === day && new Date(e.date).getMonth() === 3);
              
              return (
                <div key={i} className={`bg-white min-h-[100px] p-2 relative ${day < 1 || day > 30 ? 'bg-gray-50 opacity-30' : ''}`}>
                  {day > 0 && day <= 30 && (
                    <>
                      <span className={`text-xs font-bold ${isToday ? 'bg-blue-600 text-white w-6 h-6 flex items-center justify-center rounded-full' : 'text-gray-400'}`}>{day}</span>
                      {hasEvent && (
                        <div className={`mt-2 p-1.5 rounded text-[9px] font-bold leading-tight ${
                          hasEvent.type === 'federal' ? 'bg-blue-50 text-blue-700 border border-blue-100' : 
                          hasEvent.type === 'agency' ? 'bg-purple-50 text-purple-700 border border-purple-100' : 
                          'bg-amber-50 text-amber-700 border border-amber-100'
                        }`}>
                          {hasEvent.name}
                        </div>
                      )}
                    </>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        <div className="space-y-6">
          <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6">
            <h3 className="text-sm font-bold text-gray-900 mb-4">Upcoming Holidays</h3>
            <div className="space-y-4">
              {events.map(event => (
                <div key={event.id} className="flex items-start gap-3">
                  <div className={`mt-1 w-2 h-2 rounded-full ${event.type === 'federal' ? 'bg-blue-500' : 'bg-purple-500'}`}></div>
                  <div>
                    <div className="text-sm font-bold text-gray-900">{event.name}</div>
                    <div className="text-xs text-gray-500">{new Date(event.date).toLocaleDateString()}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {impact && (
            <div className="bg-amber-50 border border-amber-100 rounded-xl p-6">
              <div className="flex items-center gap-2 text-amber-800 mb-2">
                <AlertTriangle className="w-4 h-4" />
                <h3 className="text-sm font-bold">Impact Preview</h3>
              </div>
              <p className="text-xs text-amber-700 leading-relaxed">
                {impact.description}
              </p>
              <button 
                onClick={onViewAffected}
                className="mt-4 w-full py-2 bg-amber-100 text-amber-800 rounded-lg text-xs font-bold hover:bg-amber-200 transition-colors"
              >
                View Affected Cases ({impact.affectedCases})
              </button>
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
}

function HealthPanel({ status }: { status: ServiceHealthView[] }) {
  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold text-gray-900">System Health &amp; Service Status</h2>
        <div className="text-xs text-gray-500">Last integrity check: <span className="font-mono font-bold text-gray-700">2026-04-09 17:00:00 UTC</span></div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {status.map(svc => (
          <div key={svc.id} className="bg-white rounded-xl border border-gray-200 shadow-sm p-6">
            <div className="flex items-center justify-between mb-4">
              <div className={`p-2 rounded-lg ${svc.status === 'healthy' ? 'bg-emerald-50 text-emerald-600' : 'bg-amber-50 text-amber-600'}`}>
                <Activity className="w-5 h-5" />
              </div>
              <span className={`text-[10px] font-bold uppercase tracking-widest ${svc.status === 'healthy' ? 'text-emerald-600' : 'text-amber-600'}`}>
                {svc.status}
              </span>
            </div>
            <h3 className="text-sm font-bold text-gray-900 mb-4">{svc.name}</h3>
            <div className="space-y-3 pt-4 border-t border-gray-50">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-gray-400 uppercase">Latency</span>
                <span className="text-xs font-bold text-gray-700">{svc.latency}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-gray-400 uppercase">Error Rate</span>
                <span className="text-xs font-bold text-gray-700">{svc.errorRate}</span>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-8">
        <h3 className="text-sm font-bold text-gray-900 mb-6">AI Endpoint Performance</h3>
        <div className="h-48 flex items-end gap-2">
          {Array.from({ length: 40 }).map((_, i) => (
            <div 
              key={i} 
              className="flex-1 bg-blue-500 rounded-t-sm hover:bg-blue-600 transition-colors cursor-pointer group relative"
              style={{ height: `${20 + Math.random() * 60}%` }}
            >
              <div className="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 bg-[#141414] text-white text-[10px] px-2 py-1 rounded opacity-0 group-hover:opacity-100 whitespace-nowrap z-10">
                {Math.floor(Math.random() * 100)}ms
              </div>
            </div>
          ))}
        </div>
        <div className="flex items-center justify-between mt-4 text-[10px] font-bold text-gray-400 uppercase tracking-widest">
          <span>17:00</span>
          <span>17:15</span>
          <span>17:30</span>
          <span>17:45</span>
          <span>18:00</span>
        </div>
      </div>
    </motion.div>
  );
}

function DeonticConstraintsPanel({ constraints }: { constraints: DeonticConstraintView[] }) {
  const KIND_STYLES: Record<string, { bg: string; text: string; border: string; icon: string }> = {
    permission: { bg: 'bg-emerald-50', text: 'text-emerald-700', border: 'border-emerald-100', icon: 'P' },
    prohibition: { bg: 'bg-red-50', text: 'text-red-700', border: 'border-red-100', icon: 'X' },
    obligation: { bg: 'bg-blue-50', text: 'text-blue-700', border: 'border-blue-100', icon: 'O' },
    right: { bg: 'bg-violet-50', text: 'text-violet-700', border: 'border-violet-100', icon: 'R' },
  };

  const byKind = (kind: string) => constraints.filter(c => c.kind === kind);
  const counts = {
    permissions: byKind('permission').length,
    prohibitions: byKind('prohibition').length,
    obligations: byKind('obligation').length,
    rights: byKind('right').length,
  };

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold text-gray-900">Deontic Constraints</h2>
          <p className="text-xs text-gray-500 mt-1">Permissions, prohibitions, obligations, and rights governing agent behavior</p>
        </div>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {([
          { label: 'Permissions', count: counts.permissions, style: KIND_STYLES.permission },
          { label: 'Prohibitions', count: counts.prohibitions, style: KIND_STYLES.prohibition },
          { label: 'Obligations', count: counts.obligations, style: KIND_STYLES.obligation },
          { label: 'Rights', count: counts.rights, style: KIND_STYLES.right },
        ]).map(({ label, count, style }) => (
          <div key={label} className={`${style.bg} border ${style.border} rounded-xl p-4`}>
            <div className="text-2xl font-black text-gray-900">{count}</div>
            <div className={`text-[10px] font-bold uppercase tracking-widest mt-1 ${style.text}`}>{label}</div>
          </div>
        ))}
      </div>

      {constraints.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-12 text-center">
          <Shield className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-bold text-gray-900 mb-1">No Deontic Constraints</h3>
          <p className="text-xs text-gray-500">This workflow has no AI deontic constraints defined.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {constraints.map(c => {
            const s = KIND_STYLES[c.kind];
            return (
              <div key={c.id} className="bg-white rounded-xl border border-gray-200 shadow-sm p-5 hover:shadow-md transition-shadow">
                <div className="flex items-start gap-4">
                  <div className={`w-10 h-10 ${s.bg} ${s.text} rounded-lg flex items-center justify-center flex-shrink-0 border ${s.border} text-sm font-black`}>
                    {s.icon}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="text-sm font-bold text-gray-900">{c.id}</h4>
                      <span className={`inline-flex items-center px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider ${s.bg} ${s.text} border ${s.border}`}>
                        {c.kind}
                      </span>
                      {c.bypassable && (
                        <span className="inline-flex items-center px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider bg-amber-50 text-amber-700 border border-amber-100">
                          bypassable
                        </span>
                      )}
                    </div>
                    <p className="text-xs font-mono text-gray-600 break-all leading-relaxed">{c.summary}</p>
                    {c.detail && (
                      <p className="text-xs text-gray-500 mt-2">{c.detail}</p>
                    )}
                    {c.onViolation && (
                      <div className="mt-3 flex items-center gap-2">
                        <span className="text-[9px] font-bold text-gray-400 uppercase tracking-widest">On violation:</span>
                        <span className={`px-2 py-0.5 rounded text-[9px] font-bold ${
                          c.onViolation === 'reject' ? 'bg-red-50 text-red-700' :
                          c.onViolation === 'escalateToHuman' ? 'bg-amber-50 text-amber-700' :
                          c.onViolation === 'switchToAssistive' ? 'bg-blue-50 text-blue-700' :
                          'bg-gray-50 text-gray-700'
                        }`}>
                          {c.onViolation}
                        </span>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </motion.div>
  );
}

function QualityControlsPanel({ controls }: { controls: QualityControlsView | null }) {
  if (!controls) {
    return (
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-gray-900">Quality Controls</h2>
            <p className="text-xs text-gray-500 mt-1">Review sampling, separation of duties, and override authority</p>
          </div>
        </div>
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-12 text-center">
          <CheckCircle2 className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-bold text-gray-900 mb-1">No Quality Controls Defined</h3>
          <p className="text-xs text-gray-500">This workflow has no quality control configuration.</p>
        </div>
      </motion.div>
    );
  }

  const rs = controls.reviewSampling;
  const sod = controls.separationOfDuties;
  const oa = controls.overrideAuthority;

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold text-gray-900">Quality Controls</h2>
          <p className="text-xs text-gray-500 mt-1">Review sampling, separation of duties, and override authority</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {rs && (
          <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
              <div className="w-8 h-8 bg-blue-50 text-blue-600 rounded-lg flex items-center justify-center border border-blue-100">
                <Activity className="w-4 h-4" />
              </div>
              <h3 className="text-sm font-bold text-gray-900">Review Sampling</h3>
            </div>
            <div className="p-6 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500">Sampling Rate</span>
                <span className="text-sm font-bold text-gray-900">{Math.round(rs.rate * 100)}%</span>
              </div>
              <div className="w-full bg-gray-100 rounded-full h-2">
                <div className="bg-blue-600 h-2 rounded-full" style={{ width: `${rs.rate * 100}%` }} />
              </div>
              {rs.method && (
                <div className="flex items-center justify-between">
                  <span className="text-xs text-gray-500">Method</span>
                  <span className="text-xs font-medium text-gray-700 capitalize">{rs.method}</span>
                </div>
              )}
              {rs.scope && (
                <div className="flex items-center justify-between">
                  <span className="text-xs text-gray-500">Scope</span>
                  <span className="text-xs font-medium text-gray-700 capitalize">{rs.scope}</span>
                </div>
              )}
            </div>
          </div>
        )}

        {sod && (
          <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
              <div className="w-8 h-8 bg-amber-50 text-amber-600 rounded-lg flex items-center justify-center border border-amber-100">
                <Users className="w-4 h-4" />
              </div>
              <h3 className="text-sm font-bold text-gray-900">Separation of Duties</h3>
            </div>
            <div className="p-6 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs text-gray-500">Scope</span>
                <span className="text-xs font-medium text-gray-700">{sod.scope === 'sameInstance' ? 'Same Instance' : 'Global'}</span>
              </div>
              {sod.excludeRoles && sod.excludeRoles.length > 0 && (
                <div>
                  <span className="text-xs text-gray-500 block mb-2">Excluded Roles</span>
                  <div className="flex flex-wrap gap-1">
                    {sod.excludeRoles.map(role => (
                      <span key={role} className="px-2 py-0.5 bg-amber-50 text-amber-700 rounded text-[10px] font-bold border border-amber-100">{role}</span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {oa && (
          <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
              <div className="w-8 h-8 bg-violet-50 text-violet-600 rounded-lg flex items-center justify-center border border-violet-100">
                <Shield className="w-4 h-4" />
              </div>
              <h3 className="text-sm font-bold text-gray-900">Override Authority</h3>
            </div>
            <div className="p-6 space-y-3">
              {[
                { label: 'Structured Rationale', value: oa.requireStructuredRationale },
                { label: 'Authority Verification', value: oa.requireAuthorityVerification },
                { label: 'Supporting Evidence', value: oa.requireSupportingEvidence },
              ].map(item => (
                <div key={item.label} className="flex items-center justify-between">
                  <span className="text-xs text-gray-500">{item.label}</span>
                  <span className={`w-5 h-5 rounded-full flex items-center justify-center ${item.value ? 'bg-emerald-50 text-emerald-600' : 'bg-gray-100 text-gray-400'}`}>
                    <CheckCircle2 className="w-3.5 h-3.5" />
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </motion.div>
  );
}

function PipelineViewerPanel({ pipelines }: { pipelines: PipelineView[] }) {
  const STAGE_TYPE_STYLES: Record<string, { bg: string; text: string; border: string }> = {
    'contract-validation': { bg: 'bg-blue-50', text: 'text-blue-700', border: 'border-blue-100' },
    'assertion-gate': { bg: 'bg-emerald-50', text: 'text-emerald-700', border: 'border-emerald-100' },
    'transform': { bg: 'bg-violet-50', text: 'text-violet-700', border: 'border-violet-100' },
    'human-review': { bg: 'bg-amber-50', text: 'text-amber-700', border: 'border-amber-100' },
  };

  const REJECTION_STYLES: Record<string, string> = {
    retryWithCorrections: 'bg-yellow-50 text-yellow-700 border-yellow-100',
    escalateToSupervisor: 'bg-red-50 text-red-700 border-red-100',
    holdPendingData: 'bg-orange-50 text-orange-700 border-orange-100',
    failWithExplanation: 'bg-red-50 text-red-700 border-red-100',
  };

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold text-gray-900">Validation Pipelines</h2>
          <p className="text-xs text-gray-500 mt-1">Data validation pipelines with assertion gates</p>
        </div>
      </div>

      {pipelines.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-12 text-center">
          <Layers className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-bold text-gray-900 mb-1">No Pipelines Defined</h3>
          <p className="text-xs text-gray-500">This workflow has no validation pipelines configured.</p>
        </div>
      ) : (
        pipelines.map(pipeline => (
          <div key={pipeline.id} className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-3">
              <div className="w-8 h-8 bg-indigo-50 text-indigo-600 rounded-lg flex items-center justify-center border border-indigo-100">
                <Layers className="w-4 h-4" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-gray-900">{pipeline.id}</h3>
                {pipeline.description && <p className="text-xs text-gray-500">{pipeline.description}</p>}
              </div>
              <span className="ml-auto text-[10px] font-bold text-gray-400 uppercase tracking-widest">{pipeline.stages.length} stages</span>
            </div>

            <div className="divide-y divide-gray-100">
              {pipeline.stages.map((stage, idx) => {
                const st = STAGE_TYPE_STYLES[stage.type] ?? STAGE_TYPE_STYLES['transform'];
                return (
                  <div key={stage.id} className="px-6 py-4">
                    <div className="flex items-start gap-3">
                      <div className="flex flex-col items-center gap-1 mt-1">
                        <div className={`w-7 h-7 ${st.bg} ${st.text} rounded-full flex items-center justify-center text-xs font-bold border ${st.border}`}>
                          {idx + 1}
                        </div>
                        {idx < pipeline.stages.length - 1 && (
                          <div className="w-px h-4 bg-gray-200" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <h4 className="text-sm font-bold text-gray-900">{stage.id}</h4>
                          <span className={`px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider ${st.bg} ${st.text} border ${st.border}`}>
                            {stage.type}
                          </span>
                          {stage.rejectionPolicy && (
                            <span className={`px-2 py-0.5 rounded text-[9px] font-bold ${REJECTION_STYLES[stage.rejectionPolicy] ?? 'bg-gray-50 text-gray-700'}`}>
                              {stage.rejectionPolicy}
                            </span>
                          )}
                        </div>
                        {stage.description && <p className="text-xs text-gray-500 mb-2">{stage.description}</p>}
                        {stage.contractRef && (
                          <p className="text-xs font-mono text-gray-400">{stage.contractRef}</p>
                        )}
                        {stage.assertions && stage.assertions.length > 0 && (
                          <div className="mt-2 flex flex-wrap gap-1">
                            {stage.assertions.map((a, ai) => (
                              <span key={ai} className="inline-flex items-center gap-1 px-2 py-1 bg-gray-50 rounded text-[10px] font-mono text-gray-600 border border-gray-200">
                                <span className={`w-1.5 h-1.5 rounded-full ${st.bg.replace('50', '500')}`} />
                                {a.type}: {a.expression ?? a.fields?.join(', ') ?? a.description ?? 'assertion'}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))
      )}
    </motion.div>
  );
}

function VerificationReportPanel({ report }: { report: VerificationReportView | null }) {
  if (!report) {
    return (
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
        <h2 className="text-lg font-bold text-gray-900">SMT Verification Report</h2>
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-12 text-center">
          <CheckCircle2 className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-bold text-gray-900 mb-1">No Verification Report</h3>
          <p className="text-xs text-gray-500">No SMT verification has been run for this workflow.</p>
        </div>
      </motion.div>
    );
  }

  const RESULT_STYLES: Record<string, { bg: string; text: string }> = {
    'proven-safe': { bg: 'bg-emerald-50', text: 'text-emerald-700' },
    'proven-unsafe': { bg: 'bg-red-50', text: 'text-red-700' },
    'inconclusive': { bg: 'bg-amber-50', text: 'text-amber-700' },
  };

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold text-gray-900">SMT Verification Report</h2>
          <p className="text-xs text-gray-500 mt-1">Formal verification of deontic constraints via {report.solver.name} v{report.solver.version}</p>
        </div>
      </div>

      {report.summary && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-emerald-50 border border-emerald-100 rounded-xl p-4">
            <div className="text-2xl font-black text-gray-900">{report.summary.provenSafe ?? 0}</div>
            <div className="text-[10px] font-bold uppercase tracking-widest mt-1 text-emerald-700">Proven Safe</div>
          </div>
          <div className="bg-red-50 border border-red-100 rounded-xl p-4">
            <div className="text-2xl font-black text-gray-900">{report.summary.provenUnsafe ?? 0}</div>
            <div className="text-[10px] font-bold uppercase tracking-widest mt-1 text-red-700">Proven Unsafe</div>
          </div>
          <div className="bg-amber-50 border border-amber-100 rounded-xl p-4">
            <div className="text-2xl font-black text-gray-900">{report.summary.inconclusive ?? 0}</div>
            <div className="text-[10px] font-bold uppercase tracking-widest mt-1 text-amber-700">Inconclusive</div>
          </div>
          <div className="bg-gray-50 border border-gray-200 rounded-xl p-4">
            <div className="text-2xl font-black text-gray-900">{report.summary.totalSolverTimeMs != null ? `${(report.summary.totalSolverTimeMs / 1000).toFixed(1)}s` : '—'}</div>
            <div className="text-[10px] font-bold uppercase tracking-widest mt-1 text-gray-500">Total Time</div>
          </div>
        </div>
      )}

      <div className="space-y-3">
        {report.results.map((r, i) => {
          const rs = RESULT_STYLES[r.result] ?? RESULT_STYLES.inconclusive;
          return (
            <div key={i} className="bg-white rounded-xl border border-gray-200 shadow-sm p-5">
              <div className="flex items-center gap-3 mb-2">
                <h4 className="text-sm font-bold text-gray-900">{r.constraintRef}</h4>
                <span className={`px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider ${rs.bg} ${rs.text}`}>
                  {r.result}
                </span>
                {r.solverTimeMs != null && (
                  <span className="text-[10px] text-gray-400">{r.solverTimeMs >= 1000 ? `${(r.solverTimeMs / 1000).toFixed(1)}s` : `${r.solverTimeMs}ms`}</span>
                )}
              </div>
              {r.notes && <p className="text-xs text-gray-600 leading-relaxed">{r.notes}</p>}
              {r.counterexample?.explanation && (
                <div className="mt-2 p-3 bg-red-50 border border-red-100 rounded-lg">
                  <span className="text-[9px] font-bold text-red-700 uppercase tracking-widest">Counterexample</span>
                  <p className="text-xs text-red-600 mt-1">{r.counterexample.explanation}</p>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </motion.div>
  );
}

function EquityGuardrailsPanel({ config }: { config: EquityConfigView | null }) {
  if (!config) {
    return (
      <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
        <h2 className="text-lg font-bold text-gray-900">Equity Guardrails</h2>
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-12 text-center">
          <Shield className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <h3 className="text-sm font-bold text-gray-900 mb-1">No Equity Configuration</h3>
          <p className="text-xs text-gray-500">This workflow has no equity monitoring configured.</p>
        </div>
      </motion.div>
    );
  }

  const ACTION_STYLES: Record<string, string> = {
    review: 'bg-amber-50 text-amber-700 border-amber-100',
    audit: 'bg-orange-50 text-orange-700 border-orange-100',
    suspend: 'bg-red-50 text-red-700 border-red-100',
  };

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-bold text-gray-900">Equity Guardrails</h2>
          <p className="text-xs text-gray-500 mt-1">Protected categories, disparity monitoring, and remediation triggers</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 bg-gray-50">
            <h3 className="text-sm font-bold text-gray-900">Protected Categories ({config.protectedCategories.length})</h3>
          </div>
          <div className="divide-y divide-gray-100">
            {config.protectedCategories.map(cat => (
              <div key={cat.id} className="p-5">
                <div className="flex items-center gap-2 mb-1">
                  <h4 className="text-sm font-bold text-gray-900">{cat.id}</h4>
                  <span className="text-[10px] font-mono text-gray-400">{cat.groupByPath}</span>
                </div>
                {cat.description && <p className="text-xs text-gray-500 mb-2">{cat.description}</p>}
                <div className="flex flex-wrap gap-1">
                  {cat.groups.map(g => (
                    <span key={g} className="px-2 py-0.5 bg-indigo-50 text-indigo-700 rounded text-[10px] font-bold border border-indigo-100">{g}</span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="space-y-6">
          <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-gray-200 bg-gray-50">
              <h3 className="text-sm font-bold text-gray-900">Disparity Methods ({config.disparityMethods.length})</h3>
            </div>
            <div className="p-5 space-y-2">
              {config.disparityMethods.map(m => (
                <div key={m.id} className="flex items-center justify-between">
                  <span className="text-xs font-medium text-gray-700">{m.id}</span>
                  <span className="text-[10px] font-mono text-gray-400">{m.method}</span>
                </div>
              ))}
            </div>
          </div>

          {config.remediationTriggers && config.remediationTriggers.length > 0 && (
            <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
              <div className="px-6 py-4 border-b border-gray-200 bg-gray-50">
                <h3 className="text-sm font-bold text-gray-900">Remediation Triggers</h3>
              </div>
              <div className="divide-y divide-gray-100">
                {config.remediationTriggers.map((t, i) => (
                  <div key={i} className="p-5">
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`px-2 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider border ${ACTION_STYLES[t.action] ?? 'bg-gray-50 text-gray-700'}`}>
                        {t.action}
                      </span>
                      <span className="text-xs font-mono text-gray-600">{t.condition}</span>
                    </div>
                    {t.description && <p className="text-xs text-gray-500 mt-1">{t.description}</p>}
                    <div className="flex flex-wrap gap-1 mt-2">
                      {t.notifyRoles.map(role => (
                        <span key={role} className="px-1.5 py-0.5 bg-gray-50 text-gray-600 rounded text-[9px] font-bold border border-gray-200">{role}</span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
}

