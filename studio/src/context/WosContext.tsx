import React, { createContext, useContext, useMemo, useRef } from 'react';
import { FixtureBackend, FixtureInboxPort, FixtureCaseViewerPort, FixtureWorkflowDesignPort, FixtureGovernancePort, FixtureDashboardPort, FixtureApplicantPort, FixtureAuthPort } from '../services/FixtureAdapter';
import { HttpWosBackend, HttpInboxPort, HttpCaseViewerPort, HttpWorkflowDesignPort, HttpGovernancePort, HttpDashboardPort, HttpApplicantPort, HttpAuthPort } from '../services/HttpWosBackend';
import { SocketIORealtimePort } from '../services/SocketIORealtimePort';
import type { IWosBackend } from '../services/WosBackend';
import type { IInboxPort, ICaseViewerPort, IWorkflowDesignPort, IGovernancePort, IGovernanceWriter, IDashboardPort, IApplicantPort, IRealtimePort, IAuthPort } from '../services/WosPorts';

export interface WosPorts {
  backend: IWosBackend;
  inbox: IInboxPort;
  caseViewer: ICaseViewerPort;
  workflowDesign: IWorkflowDesignPort;
  governance: IGovernancePort;
  governanceWriter?: IGovernanceWriter;
  dashboard: IDashboardPort;
  applicant: IApplicantPort;
  realtime: IRealtimePort;
  auth: IAuthPort;
}

export type WosBackendKind = 'fixture' | 'http';

const WosContext = createContext<WosPorts | null>(null);

function createFixturePorts(): WosPorts {
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
    auth: new FixtureAuthPort(),
  };
}

function createHttpPorts(): WosPorts {
  return {
    backend: new HttpWosBackend(),
    inbox: new HttpInboxPort(),
    caseViewer: new HttpCaseViewerPort(),
    workflowDesign: new HttpWorkflowDesignPort(),
    governance: new HttpGovernancePort(),
    dashboard: new HttpDashboardPort(),
    applicant: new HttpApplicantPort(),
    realtime: new SocketIORealtimePort(),
    auth: new HttpAuthPort(),
  };
}

function resolveBackendKind(explicit?: WosBackendKind): WosBackendKind {
  if (explicit) return explicit;
  const env = (import.meta.env.VITE_WOS_BACKEND ?? '').toString().toLowerCase();
  if (env === 'http' || env === 'fixture') return env;
  if (typeof window !== 'undefined' && '__WOS_USE_HTTP__' in window) return 'http';
  return 'fixture';
}

function createPorts(kind: WosBackendKind): WosPorts {
  return kind === 'http' ? createHttpPorts() : createFixturePorts();
}

export interface WosProviderProps {
  children: React.ReactNode;
  ports?: Partial<WosPorts>;
  /**
   * Which adapter set to instantiate. **Read once at first render** and then
   * latched for the provider's lifetime — changing this prop after mount has
   * no effect. To switch backends at runtime, unmount and remount the
   * provider (for example by giving it a `key` tied to the desired kind).
   */
  backendKind?: WosBackendKind;
}

export const WosProvider: React.FC<WosProviderProps> = ({ children, ports, backendKind }) => {
  const kind = resolveBackendKind(backendKind);
  // Create the default ports exactly once per provider lifetime so every
  // consumer sees a stable reference. Overrides via `ports` are merged on
  // top without losing referential stability for the untouched ports.
  const defaultsRef = useRef<WosPorts | null>(null);
  if (defaultsRef.current === null) {
    defaultsRef.current = createPorts(kind);
  }
  const value = useMemo<WosPorts>(() => ({ ...defaultsRef.current!, ...ports }), [ports]);
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
export const useGovernanceWriter = () => useWosContext().governanceWriter ?? useWosContext().governance;
export const useDashboard = () => useWosContext().dashboard;
export const useApplicant = () => useWosContext().applicant;
export const useRealtime = () => useWosContext().realtime;
export const useAuth = () => useWosContext().auth;
