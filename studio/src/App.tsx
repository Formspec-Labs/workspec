/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { SidebarFilters } from './components/SidebarFilters';
import { TaskList } from './components/TaskList';
import { FormWorkspace, type WorkspaceTarget } from './components/workspace/FormWorkspace';
import { CaseViewer } from './components/viewer/CaseViewer';
import { ProcessDashboard } from './components/dashboard/ProcessDashboard';
import { OutboundManagementPanel } from './components/notifications/OutboundManagementPanel';
import { WorkflowDesigner } from './components/designer/WorkflowDesigner';
import { AdminConsole } from './components/admin/AdminConsole';
import { AuditViewer } from './components/audit/AuditViewer';
import { ApplicantPortal } from './components/portal/ApplicantPortal';
import { ReportBuilder } from './components/reports/ReportBuilder';
import { BackgroundJobTray } from './components/BackgroundJobTray';
import { ErrorBoundary } from './components/ui/ErrorBoundary';
import { Toaster } from 'sonner';
import { useInbox, useBackend } from './context/WosContext';
import type { TaskListItem } from './services/WosPorts';
import type { CaseInstanceView } from './services/WosBackend';

const ViewErrorBoundary: React.FC<{ name: string; children: React.ReactNode }> = ({ name, children }) => {
  return (
    <ErrorBoundary fallback={
      <div className="flex-1 flex items-center justify-center bg-gray-50">
        <div className="text-center space-y-4">
          <p className="text-lg font-semibold text-gray-700">{name} encountered an error</p>
          <button onClick={() => window.location.reload()} className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm font-medium hover:bg-blue-700 transition-colors">Reload</button>
        </div>
      </div>
    }>
      {children}
    </ErrorBoundary>
  );
};

type ViewState = 'inbox' | 'workspace' | 'viewer' | 'dashboard' | 'outbound' | 'designer' | 'admin' | 'audit' | 'portal' | 'reports';

export default function App() {
  const inbox = useInbox();
  const backend = useBackend();
  const [tasks, setTasks] = useState<TaskListItem[]>([]);
  const [wosInstances, setWosInstances] = useState<CaseInstanceView[]>([]);
  const [workspaceTarget, setWorkspaceTarget] = useState<WorkspaceTarget | null>(null);
  const [viewingCaseId, setViewingCaseId] = useState<string | null>(null);
  const [view, setView] = useState<ViewState>('inbox');
  const [filters, setFilters] = useState({
    status: [] as string[],
    impactLevel: [] as string[],
    configuration: [] as string[],
  });

  useEffect(() => {
    inbox.listTasks().then(res => setTasks(res.items));
    backend.listInstances().then(res => setWosInstances(res.items));

    const handleDesignerNav = (e: any) => {
      setView('designer');
    };

    window.addEventListener('navigate-to-designer', handleDesignerNav);
    return () => window.removeEventListener('navigate-to-designer', handleDesignerNav);
  }, [inbox, backend]);

  const handleNavigate = (link: { type: string; id: string }) => {
    if (link.type === 'task') {
      setWorkspaceTarget({ kind: 'task', id: link.id });
      setView('workspace');
    } else if (link.type === 'case') {
      setViewingCaseId(link.id);
      setView('viewer');
    } else if (link.type === 'dashboard') {
      setView('dashboard');
    }
  };

  const renderContent = () => {
    switch (view) {
      case 'dashboard':
        return <ViewErrorBoundary name="Dashboard"><ProcessDashboard /></ViewErrorBoundary>;
      case 'outbound':
        return <ViewErrorBoundary name="Outbound"><OutboundManagementPanel /></ViewErrorBoundary>;
      case 'designer':
        return <ViewErrorBoundary name="Designer"><WorkflowDesigner /></ViewErrorBoundary>;
      case 'admin':
        return <ViewErrorBoundary name="Admin"><AdminConsole /></ViewErrorBoundary>;
      case 'audit':
        return <ViewErrorBoundary name="Audit"><AuditViewer /></ViewErrorBoundary>;
      case 'portal':
        return <ViewErrorBoundary name="Portal"><ApplicantPortal /></ViewErrorBoundary>;
      case 'reports':
        return <ViewErrorBoundary name="Reports"><ReportBuilder /></ViewErrorBoundary>;
      case 'viewer':
        return <ViewErrorBoundary name="Case Viewer">{viewingCaseId ? <CaseViewer caseId={viewingCaseId} onBack={() => setView('inbox')} /> : null}</ViewErrorBoundary>;
      case 'workspace':
        return <ViewErrorBoundary name="Workspace">{workspaceTarget ? <FormWorkspace target={workspaceTarget} onBack={() => setView('inbox')} /> : null}</ViewErrorBoundary>;
      default:
        return (
          <ViewErrorBoundary name="Inbox">
          <div className="flex flex-1 overflow-hidden">
            <SidebarFilters
              filters={filters}
              setFilters={setFilters}
              className="w-64 bg-gray-50 border-r border-gray-200 h-full hidden lg:block"
            />
            <div className="flex flex-col flex-1 overflow-hidden">
              {wosInstances.length > 0 && (
                <div className="px-4 pt-3 pb-1 bg-slate-50 border-b border-slate-100">
                  <div className="text-[9px] font-black text-slate-400 uppercase tracking-[0.2em] mb-2">WOS Active Instances</div>
                  <div className="flex gap-2 overflow-x-auto no-scrollbar pb-1">
                    {wosInstances.map(inst => (
                      <button
                        key={inst.instanceId}
                        onClick={() => {
                          setWorkspaceTarget({ kind: 'instance', id: inst.instanceId });
                          setView('workspace');
                        }}
                        className="flex items-center gap-2 px-3 py-1.5 bg-white border border-slate-200 rounded-xl text-[10px] font-bold text-slate-700 hover:border-blue-300 hover:bg-blue-50 transition-all shrink-0 shadow-sm"
                      >
                        <span className={`w-2 h-2 rounded-full ${inst.status === 'active' ? 'bg-emerald-500' : inst.status === 'suspended' ? 'bg-amber-500' : 'bg-slate-300'}`} />
                        <span className="truncate max-w-[140px]">{inst.configuration.join(', ') || inst.instanceId.split(':').pop()}</span>
                        {inst.impactLevel === 'rights-impacting' && (
                          <span className="text-[8px] text-rose-600 font-black uppercase">R-I</span>
                        )}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              <TaskList tasks={tasks} filters={filters} setFilters={setFilters} onTaskClick={(id) => { setWorkspaceTarget({ kind: 'task', id }); setView('workspace'); }} />
            </div>
          </div>
          </ViewErrorBoundary>
        );
    }
  };

  return (
    <ErrorBoundary>
      <div className="min-h-screen bg-white flex flex-col font-sans text-gray-900 overflow-hidden">
        {view !== 'portal' && (
          <Header
            onViewInbox={() => setView('inbox')}
            onViewDashboard={() => setView('dashboard')}
            onViewOutbound={() => setView('outbound')}
            onViewDesigner={() => setView('designer')}
            onViewAdmin={() => setView('admin')}
            onViewAudit={() => setView('audit')}
            onViewPortal={() => setView('portal')}
            onViewReports={() => setView('reports')}
            onViewSampleCase={() => { setViewingCaseId('urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4'); setView('viewer'); }}
            onNavigate={handleNavigate}
            currentView={view}
          />
        )}
        {view === 'portal' && (
          <header className="bg-emerald-800 text-white px-6 py-4 flex items-center justify-between shrink-0 shadow-md">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-white text-emerald-800 rounded-full flex items-center justify-center font-bold">
                Gov
              </div>
              <h1 className="text-xl font-bold">State Benefits Portal</h1>
            </div>
            <button
              onClick={() => setView('inbox')}
              className="text-emerald-100 hover:text-white text-sm font-medium transition-colors"
            >
              Return to Agency View
            </button>
          </header>
        )}
        <main className="flex-1 overflow-hidden flex flex-col">
          {renderContent()}
        </main>
        <BackgroundJobTray />
        <Toaster position="bottom-right" richColors closeButton />
      </div>
    </ErrorBoundary>
  );
}
