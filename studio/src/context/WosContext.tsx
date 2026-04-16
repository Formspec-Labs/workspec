import React, { createContext, useContext } from 'react';
import { FixtureBackend, FixtureInboxPort, FixtureCaseViewerPort, FixtureWorkflowDesignPort, FixtureGovernancePort, FixtureDashboardPort, FixtureApplicantPort } from '../services/FixtureAdapter';
import { SocketIORealtimePort } from '../services/SocketIORealtimePort';
import type { IWosBackend } from '../services/WosBackend';
import type { IInboxPort, ICaseViewerPort, IWorkflowDesignPort, IGovernancePort, IDashboardPort, IApplicantPort, IRealtimePort } from '../services/WosPorts';

export interface WosPorts {
  backend: IWosBackend;
  inbox: IInboxPort;
  caseViewer: ICaseViewerPort;
  workflowDesign: IWorkflowDesignPort;
  governance: IGovernancePort;
  dashboard: IDashboardPort;
  applicant: IApplicantPort;
  realtime: IRealtimePort;
}

const WosContext = createContext<WosPorts | null>(null);

function createDefaultPorts(): WosPorts {
  const backend = new FixtureBackend();
  return {
    backend,
    inbox: new FixtureInboxPort(backend),
    caseViewer: new FixtureCaseViewerPort(backend),
    workflowDesign: new FixtureWorkflowDesignPort(backend),
    governance: new FixtureGovernancePort(backend),
    dashboard: new FixtureDashboardPort(backend),
    applicant: new FixtureApplicantPort(backend),
    realtime: new SocketIORealtimePort(),
  };
}

export const WosProvider: React.FC<{ children: React.ReactNode; ports?: Partial<WosPorts> }> = ({ children, ports }) => {
  const defaults = createDefaultPorts();
  const value = { ...defaults, ...ports };
  return <WosContext.Provider value={value}>{children}</WosContext.Provider>;
};

function useWosContext(): WosPorts {
  const ctx = useContext(WosContext);
  if (!ctx) throw new Error('Must be used within a WosProvider');
  return ctx;
}

export const useBackend = () => useWosContext().backend;
export const useInbox = () => useWosContext().inbox;
export const useCaseViewer = () => useWosContext().caseViewer;
export const useWorkflowDesign = () => useWosContext().workflowDesign;
export const useGovernance = () => useWosContext().governance;
export const useDashboard = () => useWosContext().dashboard;
export const useApplicant = () => useWosContext().applicant;
export const useRealtime = () => useWosContext().realtime;
