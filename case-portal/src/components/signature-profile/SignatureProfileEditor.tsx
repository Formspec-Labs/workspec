import React, { useState, useEffect, useCallback } from 'react';
import {
  FileSignature,
  Plus,
  Trash2,
  CheckCircle2,
  AlertTriangle,
  Activity,
  Shield,
  Clock,
  Ban,
  UserCog,
  Eye,
  ChevronRight,
  GripVertical,
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { useSignatureProfile } from '../../context/WosContext';
import type { SignatureProfileSummary } from '../../services/WosPorts';
import type {
  WOSSignatureProfileDocument,
  SignatureRole,
  SignatureDocument,
  SigningStep,
} from '../../types/wos/signature-profile';

type EditorTab = 'general' | 'roles' | 'documents' | 'flow' | 'evidence' | 'policies';
type SetProfile = React.Dispatch<React.SetStateAction<WOSSignatureProfileDocument>>;

export function SignatureProfileEditor() {
  const port = useSignatureProfile();
  const [summaries, setSummaries] = useState<SignatureProfileSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [profile, setProfile] = useState<WOSSignatureProfileDocument | null>(null);
  const [activeTab, setActiveTab] = useState<EditorTab>('general');
  const [isLoading, setIsLoading] = useState(true);
  const [validationIssues, setValidationIssues] = useState<{ isValid: boolean; issues: { severity: string; message: string }[] }>({ isValid: true, issues: [] });

  useEffect(() => {
    port.list().then(list => {
      setSummaries(list);
      if (list.length > 0 && !selectedId) {
        setSelectedId(list[0].id);
      }
    }).finally(() => setIsLoading(false));
  }, [port]);

  useEffect(() => {
    if (!selectedId) { setProfile(null); return; }
    port.load(selectedId).then(p => {
      if (p) setProfile(p);
      setActiveTab('general');
      setValidationIssues({ isValid: true, issues: [] });
    });
  }, [selectedId, port]);

  const handleValidate = async () => {
    if (!profile) return;
    const result = await port.validate(profile);
    setValidationIssues(result);
  };

  const handleSave = async () => {
    if (!profile) return;
    const result = await port.save(profile);
    setValidationIssues(result);
    if (result.isValid) {
      port.list().then(setSummaries);
    }
  };

  if (isLoading) return (
    <div className="flex-1 flex items-center justify-center bg-gray-50">
      <div className="flex flex-col items-center gap-4">
        <Activity className="w-8 h-8 text-blue-500 animate-spin" />
        <p className="text-sm font-medium text-gray-500">Loading signature profiles...</p>
      </div>
    </div>
  );

  return (
    <div className="flex-1 flex flex-col bg-gray-50 overflow-hidden">
      <div className="bg-white border-b border-gray-200 px-4 sm:px-8 py-6 shrink-0">
        <div className="max-w-7xl mx-auto">
          <h1 className="text-xl sm:text-2xl font-bold text-gray-900 tracking-tight">Signature Profiles</h1>
          <p className="text-xs sm:text-sm text-gray-500 mt-1">Author and validate signing ceremony configurations.</p>
        </div>

        <div className="max-w-7xl mx-auto mt-4 flex items-center gap-2 flex-wrap">
          {summaries.map(s => (
            <button
              key={s.id}
              onClick={() => setSelectedId(s.id)}
              className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all border ${
                selectedId === s.id
                  ? 'bg-[#141414] text-white border-[#141414] shadow-md'
                  : 'bg-white text-gray-500 border-gray-200 hover:bg-gray-100'
              }`}
            >
              <span className="flex items-center gap-1.5">
                <FileSignature className="w-3 h-3" />
                {s.id}
              </span>
            </button>
          ))}
        </div>
      </div>

      {profile && (
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="bg-white border-b border-gray-200 px-4 sm:px-8 shrink-0">
            <div className="max-w-7xl mx-auto flex items-center gap-1 overflow-x-auto no-scrollbar py-2">
              <TabBtn active={activeTab === 'general'} onClick={() => setActiveTab('general')} label="General" />
              <TabBtn active={activeTab === 'roles'} onClick={() => setActiveTab('roles')} label="Roles" />
              <TabBtn active={activeTab === 'documents'} onClick={() => setActiveTab('documents')} label="Documents" />
              <TabBtn active={activeTab === 'flow'} onClick={() => setActiveTab('flow')} label="Signing Flow" />
              <TabBtn active={activeTab === 'evidence'} onClick={() => setActiveTab('evidence')} label="Evidence" />
              <TabBtn active={activeTab === 'policies'} onClick={() => setActiveTab('policies')} label="Policies" />
            </div>
          </div>

          <main className="flex-1 overflow-y-auto p-8">
            <div className="max-w-4xl mx-auto space-y-6">
              <AnimatePresence mode="wait">
                {activeTab === 'general' && <GeneralPanel key="general" profile={profile} setProfile={setProfile} />}
                {activeTab === 'roles' && <RolesPanel key="roles" profile={profile} setProfile={setProfile} />}
                {activeTab === 'documents' && <DocumentsPanel key="documents" profile={profile} setProfile={setProfile} />}
                {activeTab === 'flow' && <FlowPanel key="flow" profile={profile} setProfile={setProfile} />}
                {activeTab === 'evidence' && <EvidencePanel key="evidence" profile={profile} setProfile={setProfile} />}
                {activeTab === 'policies' && <PoliciesPanel key="policies" profile={profile} setProfile={setProfile} />}
              </AnimatePresence>

              <div className="flex items-center justify-between pt-6 border-t border-gray-200">
                <div className="flex items-center gap-3">
                  {validationIssues.issues.length > 0 && (
                    <span className={`flex items-center gap-1.5 px-3 py-1 rounded-lg text-[10px] font-bold uppercase ${
                      validationIssues.isValid ? 'bg-emerald-50 text-emerald-700' : 'bg-rose-50 text-rose-700'
                    }`}>
                      {validationIssues.isValid ? <CheckCircle2 className="w-3.5 h-3.5" /> : <AlertTriangle className="w-3.5 h-3.5" />}
                      {validationIssues.isValid ? 'Valid' : `${validationIssues.issues.filter(i => i.severity === 'error').length} error(s)`}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={handleValidate}
                    className="px-4 py-2 text-sm font-bold text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-xl transition-colors"
                  >
                    Validate
                  </button>
                  <button
                    onClick={handleSave}
                    className="px-6 py-2 text-sm font-bold text-white bg-[#141414] hover:bg-black shadow-sm rounded-xl transition-colors"
                  >
                    Save Profile
                  </button>
                </div>
              </div>

              {validationIssues.issues.length > 0 && (
                <div className="space-y-2">
                  {validationIssues.issues.map((issue, idx) => (
                    <div key={idx} className={`flex items-start gap-2 p-3 rounded-lg text-sm ${
                      issue.severity === 'error' ? 'bg-rose-50 text-rose-700' : 'bg-amber-50 text-amber-700'
                    }`}>
                      {issue.severity === 'error' ? <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" /> : <Eye className="w-4 h-4 shrink-0 mt-0.5" />}
                      {issue.message}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </main>
        </div>
      )}

      {!profile && (
        <div className="flex-1 flex items-center justify-center">
          <p className="text-sm text-gray-400">Select a profile to edit.</p>
        </div>
      )}
    </div>
  );
}

function TabBtn({ active, onClick, label }: { active: boolean; onClick: () => void; label: string }) {
  return (
    <button
      onClick={onClick}
      className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all ${
        active ? 'bg-[#141414] text-white shadow-md' : 'text-gray-500 hover:bg-gray-100'
      }`}
    >
      {label}
    </button>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return <h2 className="text-lg font-bold text-gray-900">{children}</h2>;
}

function FieldLabel({ children }: { children: React.ReactNode }) {
  return <label className="block text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-1">{children}</label>;
}

function TextInput({ value, onChange, placeholder }: { value: string; onChange: (v: string) => void; placeholder?: string }) {
  return (
    <input
      type="text"
      value={value}
      onChange={e => onChange(e.target.value)}
      placeholder={placeholder}
      className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none"
    />
  );
}

function GeneralPanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const set = useCallback(
    (next: WOSSignatureProfileDocument) => setProfile(next),
    [setProfile],
  );

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-6">
      <SectionHeading>General</SectionHeading>
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 space-y-4">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <FieldLabel>Target Workflow URL</FieldLabel>
            <TextInput value={profile.targetWorkflow?.url ?? ''} onChange={v => set({ ...profile, targetWorkflow: { ...profile.targetWorkflow, url: v } })} placeholder="https://agency.gov/workflows/..." />
          </div>
          <div>
            <FieldLabel>Profile Version</FieldLabel>
            <TextInput value={profile.version ?? ''} onChange={v => set({ ...profile, version: v })} placeholder="1.0.0" />
          </div>
        </div>
        <div>
          <FieldLabel>Title</FieldLabel>
          <TextInput value={profile.title ?? ''} onChange={v => set({ ...profile, title: v })} placeholder="Benefits adjudication signature profile" />
        </div>
        <div>
          <FieldLabel>Description</FieldLabel>
          <textarea
            value={profile.description ?? ''}
            onChange={e => set({ ...profile, description: e.target.value })}
            placeholder="Describe the signing ceremony..."
            rows={3}
            className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none resize-none"
          />
        </div>
        <div className="flex items-center gap-2 px-3 py-2 bg-blue-50 rounded-lg text-xs text-blue-700">
          <Shield className="w-4 h-4" />
          <span className="font-bold">$wosSignatureProfile:</span>
          <span className="font-mono">{profile.$wosSignatureProfile}</span>
        </div>
      </div>
    </motion.div>
  );
}

function RolesPanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const roles = profile.roles;
  const roleOptions: SignatureRole['role'][] = ['signer', 'in-person-signer', 'witness', 'notary', 'approver', 'form-filler', 'viewer', 'certified-recipient'];

  const updateRoleField = (idx: number, field: string, value: string | boolean) => {
    const next = [...roles];
    (next[idx] as unknown as Record<string, typeof value>)[field] = value;
    setProfile({ ...profile, roles: next as typeof roles });
  };

  const addRole = () => {
    const entry: SignatureRole = { ...{ id: `role-${Date.now()}`, role: 'signer', actorId: '' } };
    setProfile({ ...profile, roles: [...roles, entry] as typeof roles });
  };

  const removeRole = (idx: number) => {
    if (roles.length <= 1) return;
    setProfile({ ...profile, roles: roles.filter((_, i) => i !== idx) as typeof roles });
  };

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-4">
      <div className="flex items-center justify-between">
        <SectionHeading>Signer Roles ({roles.length})</SectionHeading>
        <button onClick={addRole} className="flex items-center gap-1.5 px-3 py-1.5 bg-[#141414] text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:bg-black shadow-sm">
          <Plus className="w-3.5 h-3.5" /> Add Role
        </button>
      </div>
      {roles.map((role, idx) => (
        <div key={idx} className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <GripVertical className="w-4 h-4 text-gray-300" />
              <span className="text-sm font-bold text-gray-900">{role.id}</span>
            </div>
            <button onClick={() => removeRole(idx)} className="p-1 text-gray-400 hover:text-rose-500 transition-colors" disabled={roles.length <= 1}>
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div>
              <FieldLabel>Role ID</FieldLabel>
              <TextInput value={role.id} onChange={v => updateRoleField(idx, 'id', v)} />
            </div>
            <div>
              <FieldLabel>Type</FieldLabel>
              <select value={role.role} onChange={e => updateRoleField(idx, 'role', e.target.value)} className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                {roleOptions.map(r => <option key={r} value={r}>{r}</option>)}
              </select>
            </div>
            <div>
              <FieldLabel>Actor ID</FieldLabel>
              <TextInput value={role.actorId} onChange={v => updateRoleField(idx, 'actorId', v)} placeholder="kernel actor id" />
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <FieldLabel>Auth Policy Key</FieldLabel>
              <TextInput value={role.authenticationPolicyKey ?? ''} onChange={v => updateRoleField(idx, 'authenticationPolicyKey', v)} placeholder="references authenticationPolicies[].key" />
            </div>
            <div className="flex items-end pb-1">
              <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
                <input type="checkbox" checked={role.required ?? true} onChange={e => updateRoleField(idx, 'required', e.target.checked)} className="rounded border-gray-300" />
                Required
              </label>
            </div>
          </div>
        </div>
      ))}
    </motion.div>
  );
}

function DocumentsPanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const docs = profile.documents;

  const updateDocField = (idx: number, field: string, value: string) => {
    const next = [...docs];
    (next[idx] as unknown as Record<string, string>)[field] = value;
    setProfile({ ...profile, documents: next as typeof docs });
  };

  const addDoc = () => {
    const entry: SignatureDocument = { ...{ id: `doc-${Date.now()}`, documentRef: '', documentHash: '', documentHashAlgorithm: 'sha-256' } };
    setProfile({ ...profile, documents: [...docs, entry] as typeof docs });
  };

  const removeDoc = (idx: number) => {
    if (docs.length <= 1) return;
    setProfile({ ...profile, documents: docs.filter((_, i) => i !== idx) as typeof docs });
  };

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-4">
      <div className="flex items-center justify-between">
        <SectionHeading>Documents ({docs.length})</SectionHeading>
        <button onClick={addDoc} className="flex items-center gap-1.5 px-3 py-1.5 bg-[#141414] text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:bg-black shadow-sm">
          <Plus className="w-3.5 h-3.5" /> Add Document
        </button>
      </div>
      {docs.map((doc, idx) => (
        <div key={idx} className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm font-bold text-gray-900">{doc.id}</span>
            <button onClick={() => removeDoc(idx)} className="p-1 text-gray-400 hover:text-rose-500 transition-colors" disabled={docs.length <= 1}>
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <FieldLabel>Document ID</FieldLabel>
              <TextInput value={doc.id} onChange={v => updateDocField(idx, 'id', v)} />
            </div>
            <div>
              <FieldLabel>Document Ref (URI)</FieldLabel>
              <TextInput value={doc.documentRef} onChange={v => updateDocField(idx, 'documentRef', v)} placeholder="urn:doc:application" />
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="sm:col-span-2">
              <FieldLabel>Document Hash</FieldLabel>
              <TextInput value={doc.documentHash} onChange={v => updateDocField(idx, 'documentHash', v)} placeholder="hex digest" />
            </div>
            <div>
              <FieldLabel>Hash Algorithm</FieldLabel>
              <select value={doc.documentHashAlgorithm} onChange={e => updateDocField(idx, 'documentHashAlgorithm', e.target.value)} className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                <option value="sha-256">sha-256</option>
                <option value="sha-384">sha-384</option>
                <option value="sha-512">sha-512</option>
              </select>
            </div>
          </div>
          <div>
            <FieldLabel>Formspec Response Ref</FieldLabel>
            <TextInput value={doc.formspecResponseRef ?? ''} onChange={v => updateDocField(idx, 'formspecResponseRef', v)} placeholder="urn:formspec:response:..." />
          </div>
        </div>
      ))}
    </motion.div>
  );
}

function FlowPanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const flow = profile.signingFlow;
  const steps = flow.steps;
  const roles = profile.roles;
  const docs = profile.documents;

  const setFlow = (next: WOSSignatureProfileDocument['signingFlow']) => {
    setProfile({ ...profile, signingFlow: next });
  };

  const updateStepField = (idx: number, field: string, value: string) => {
    const next = [...steps];
    (next[idx] as unknown as Record<string, string>)[field] = value;
    setFlow({ ...flow, steps: next as typeof steps });
  };

  const addStep = () => {
    const entry: SigningStep = { ...{ id: `step-${Date.now()}`, roleId: roles[0]?.id ?? '', documentId: docs[0]?.id ?? '' } };
    setFlow({ ...flow, steps: [...steps, entry] as typeof steps });
  };

  const removeStep = (idx: number) => {
    if (steps.length <= 1) return;
    setFlow({ ...flow, steps: steps.filter((_, i) => i !== idx) as typeof steps });
  };

  const flowTypes: WOSSignatureProfileDocument['signingFlow']['type'][] = ['sequential', 'parallel', 'routed', 'free-for-all'];

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-4">
      <SectionHeading>Signing Flow</SectionHeading>
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
        <div>
          <FieldLabel>Flow Type</FieldLabel>
          <div className="flex gap-2">
            {flowTypes.map(t => (
              <button
                key={t}
                onClick={() => setFlow({ ...flow, type: t })}
                className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-widest rounded-lg transition-all border ${
                  flow.type === t ? 'bg-[#141414] text-white border-[#141414]' : 'bg-white text-gray-500 border-gray-200 hover:bg-gray-100'
                }`}
              >
                {t}
              </button>
            ))}
          </div>
        </div>
        <div>
          <FieldLabel>Completion</FieldLabel>
          <select
            value={flow.completion?.type ?? 'all-required'}
            onChange={e => setFlow({ ...flow, completion: { ...flow.completion, type: e.target.value as 'all-required' | 'any-required' | 'count' | 'role-set' } })}
            className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white"
          >
            <option value="all-required">All Required</option>
            <option value="any-required">Any Required</option>
            <option value="count">Count</option>
            <option value="role-set">Role Set</option>
          </select>
        </div>
      </div>

      <div className="flex items-center justify-between">
        <h3 className="text-sm font-bold text-gray-700">Steps ({steps.length})</h3>
        <button onClick={addStep} className="flex items-center gap-1.5 px-3 py-1.5 bg-[#141414] text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:bg-black shadow-sm">
          <Plus className="w-3.5 h-3.5" /> Add Step
        </button>
      </div>

      {steps.map((step, idx) => (
        <div key={idx} className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-bold text-gray-900">
              <ChevronRight className="w-4 h-4 text-gray-400" />
              {step.id}
            </div>
            <button onClick={() => removeStep(idx)} className="p-1 text-gray-400 hover:text-rose-500 transition-colors" disabled={steps.length <= 1}>
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div>
              <FieldLabel>Step ID</FieldLabel>
              <TextInput value={step.id} onChange={v => updateStepField(idx, 'id', v)} />
            </div>
            <div>
              <FieldLabel>Role</FieldLabel>
              <select value={step.roleId} onChange={e => updateStepField(idx, 'roleId', e.target.value)} className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                {roles.map(r => <option key={r.id} value={r.id}>{r.id} ({r.role})</option>)}
              </select>
            </div>
            <div>
              <FieldLabel>Document</FieldLabel>
              <select value={step.documentId} onChange={e => updateStepField(idx, 'documentId', e.target.value)} className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white">
                {docs.map(d => <option key={d.id} value={d.id}>{d.id}</option>)}
              </select>
            </div>
          </div>
          {flow.type === 'routed' && (
            <div>
              <FieldLabel>Guard (FEL expression)</FieldLabel>
              <TextInput value={step.guard ?? ''} onChange={v => updateStepField(idx, 'guard', v)} placeholder="caseFile.amount > 1000" />
            </div>
          )}
        </div>
      ))}
    </motion.div>
  );
}

function EvidencePanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const evidence = profile.evidence;
  const set = useCallback(
    (evidenceNext: typeof evidence) => setProfile({ ...profile, evidence: evidenceNext }),
    [profile, setProfile],
  );

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-4">
      <SectionHeading>Evidence</SectionHeading>
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 space-y-4">
        <div className="flex items-center gap-2 px-3 py-2 bg-emerald-50 rounded-lg text-xs text-emerald-700">
          <Shield className="w-4 h-4" />
          <span className="font-bold">Record Kind:</span> {evidence.recordKind}
          {evidence.custodyHookEligible && <span className="ml-2 px-2 py-0.5 bg-emerald-100 rounded text-[9px] font-bold uppercase">Custody Eligible</span>}
        </div>

        <div>
          <FieldLabel>Required Fields</FieldLabel>
          <div className="space-y-1">
            {evidence.requiredFields.map((f, idx) => (
              <div key={idx} className="flex items-center gap-2 px-3 py-1.5 bg-gray-50 rounded-lg text-sm font-mono text-gray-600">
                <ChevronRight className="w-3 h-3 text-gray-400" />{f}
              </div>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <FieldLabel>Consent Text Ref</FieldLabel>
            <TextInput value={evidence.consentReference?.consentTextRef ?? ''} onChange={v => set({ ...evidence, consentReference: { ...evidence.consentReference, consentTextRef: v } })} />
          </div>
          <div>
            <FieldLabel>Consent Version</FieldLabel>
            <TextInput value={evidence.consentReference?.consentVersion ?? ''} onChange={v => set({ ...evidence, consentReference: { ...evidence.consentReference, consentVersion: v } })} />
          </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <FieldLabel>Accepted-at Path</FieldLabel>
            <TextInput value={evidence.consentReference?.acceptedAtPath ?? ''} onChange={v => set({ ...evidence, consentReference: { ...evidence.consentReference, acceptedAtPath: v } })} />
          </div>
          <div>
            <FieldLabel>Affirmation Path</FieldLabel>
            <TextInput value={evidence.consentReference?.affirmationPath ?? ''} onChange={v => set({ ...evidence, consentReference: { ...evidence.consentReference, affirmationPath: v } })} />
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 pt-2 border-t border-gray-100">
          <div>
            <FieldLabel>Identity Binding Method</FieldLabel>
            <TextInput value={evidence.identityBinding?.method ?? ''} onChange={v => set({ ...evidence, identityBinding: { ...evidence.identityBinding, method: v } })} />
          </div>
          <div>
            <FieldLabel>Assurance Level</FieldLabel>
            <select
              value={evidence.identityBinding?.assuranceLevel ?? 'standard'}
              onChange={e => set({ ...evidence, identityBinding: { ...evidence.identityBinding, assuranceLevel: e.target.value as 'none' | 'low' | 'standard' | 'high' | 'very-high' } })}
              className="w-full p-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none appearance-none bg-white"
            >
              <option value="none">none</option>
              <option value="low">low</option>
              <option value="standard">standard</option>
              <option value="high">high</option>
              <option value="very-high">very-high</option>
            </select>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

function PoliciesPanel({ profile, setProfile }: { profile: WOSSignatureProfileDocument; setProfile: SetProfile }) {
  const set = useCallback(
    (patch: WOSSignatureProfileDocument) => setProfile(patch),
    [setProfile],
  );

  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="space-y-4">
      <SectionHeading>Policies</SectionHeading>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
        <div className="flex items-center gap-2 text-sm font-bold text-gray-900"><Clock className="w-4 h-4" /> Expiry Policy</div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <FieldLabel>Event Name</FieldLabel>
            <TextInput value={profile.expiryPolicy?.eventName ?? ''} onChange={v => set({ ...profile, expiryPolicy: { ...profile.expiryPolicy, eventName: v, after: profile.expiryPolicy?.after ?? 'P7D' } })} placeholder="signature.expired" />
          </div>
          <div>
            <FieldLabel>After (ISO 8601 duration)</FieldLabel>
            <TextInput value={profile.expiryPolicy?.after ?? ''} onChange={v => set({ ...profile, expiryPolicy: { ...profile.expiryPolicy, after: v, eventName: profile.expiryPolicy?.eventName ?? '' } })} placeholder="P7D" />
          </div>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
        <div className="flex items-center gap-2 text-sm font-bold text-gray-900"><Ban className="w-4 h-4" /> Decline Policy</div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <FieldLabel>Transition ID</FieldLabel>
            <TextInput value={profile.declinePolicy?.transitionId ?? ''} onChange={v => set({ ...profile, declinePolicy: { ...profile.declinePolicy, transitionId: v } })} placeholder="signature.declined" />
          </div>
          <div className="flex items-end pb-1">
            <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
              <input type="checkbox" checked={profile.declinePolicy?.reasonRequired ?? false} onChange={e => set({ ...profile, declinePolicy: { ...profile.declinePolicy, reasonRequired: e.target.checked } })} className="rounded border-gray-300" />
              Reason Required
            </label>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
        <div className="flex items-center gap-2 text-sm font-bold text-gray-900"><Ban className="w-4 h-4" /> Void Policy</div>
        <div className="space-y-3">
          <div>
            <FieldLabel>Authorized Actor IDs (comma-separated)</FieldLabel>
            <TextInput
              value={(profile.voidPolicy?.authorizedActorIds ?? []).join(', ')}
              onChange={v => set({ ...profile, voidPolicy: { ...profile.voidPolicy, authorizedActorIds: v.split(',').map(s => s.trim()).filter(Boolean) as [string, ...string[]] } })}
              placeholder="caseworker, supervisor"
            />
          </div>
          <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
            <input type="checkbox" checked={profile.voidPolicy?.reasonRequired ?? false} onChange={e => set({ ...profile, voidPolicy: { ...profile.voidPolicy, reasonRequired: e.target.checked } })} className="rounded border-gray-300" />
            Reason Required
          </label>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 space-y-3">
        <div className="flex items-center gap-2 text-sm font-bold text-gray-900"><UserCog className="w-4 h-4" /> Reassignment Policy</div>
        <div className="space-y-3">
          <div>
            <FieldLabel>Authorized Actor IDs (comma-separated)</FieldLabel>
            <TextInput
              value={(profile.reassignmentPolicy?.authorizedActorIds ?? []).join(', ')}
              onChange={v => set({ ...profile, reassignmentPolicy: { ...profile.reassignmentPolicy, authorizedActorIds: v.split(',').map(s => s.trim()).filter(Boolean) as [string, ...string[]] } })}
              placeholder="caseworker, supervisor"
            />
          </div>
          <label className="flex items-center gap-2 text-sm text-gray-600 cursor-pointer">
            <input type="checkbox" checked={profile.reassignmentPolicy?.reasonRequired ?? false} onChange={e => set({ ...profile, reassignmentPolicy: { ...profile.reassignmentPolicy, reasonRequired: e.target.checked } })} className="rounded border-gray-300" />
            Reason Required
          </label>
        </div>
      </div>
    </motion.div>
  );
}
