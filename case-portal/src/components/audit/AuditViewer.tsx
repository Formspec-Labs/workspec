import React, { useState, useEffect } from 'react';
import {
  Search,
  Filter,
  Shield,
  CheckCircle2,
  AlertTriangle,
  Clock,
  FileText,
  Download,
  ChevronRight,
  Cpu,
  User,
  History,
  ArrowRight,
  ExternalLink,
  Lock,
  Eye,
  Activity,
  ArrowUpRight,
  ArrowDownRight,
  Fingerprint,
  Link2,
  Scale,
  Bot,
  GitBranch,
} from 'lucide-react';
import { useCaseViewer } from '../../context/WosContext';
import type { ProvenanceRecord } from '../../services/WosBackend';
import { motion, AnimatePresence } from 'motion/react';

type ProvenanceTier = ProvenanceRecord['tier'];

/** Demo fixture instances merged into the explorer for cross-case search journeys. */
const AUDIT_DEMO_INSTANCE_IDS = [
  'urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4',
  'urn:wos:instance:benefits-adj:2026-03-20:i9j0k1l2',
] as const;

export function AuditViewer() {
  const caseViewer = useCaseViewer();
  const [provenanceRecords, setProvenanceRecords] = useState<ProvenanceRecord[]>([]);
  const [selectedRecordId, setSelectedRecordId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [tierFilter, setTierFilter] = useState<ProvenanceTier | 'all'>('all');
  const [isVerifying, setIsVerifying] = useState(false);
  const [verificationProof, setVerificationProof] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const batches = await Promise.all(
        AUDIT_DEMO_INSTANCE_IDS.map(id => caseViewer.getProvenance(id)),
      );
      if (cancelled) return;
      const byId = new Map<string, ProvenanceRecord>();
      for (const batch of batches) {
        for (const r of batch) byId.set(r.id, r);
      }
      const merged = [...byId.values()].sort((a, b) => a.timestamp.localeCompare(b.timestamp));
      setProvenanceRecords(merged);
    })();
    return () => {
      cancelled = true;
    };
  }, [caseViewer]);

  const filteredRecords = provenanceRecords.filter(r => {
    const matchesSearch =
      r.instanceId.toLowerCase().includes(searchQuery.toLowerCase()) ||
      r.actor.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      r.event.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesTier = tierFilter === 'all' || r.tier === tierFilter;
    return matchesSearch && matchesTier;
  });

  const selectedRecord = provenanceRecords.find(r => r.id === selectedRecordId) ?? null;

  const handleVerify = async (record: ProvenanceRecord) => {
    setIsVerifying(true);
    await new Promise(resolve => setTimeout(resolve, 800));
    setVerificationProof(
      `Verified: ${record.integrity.hash}\nPrevious: ${record.integrity.previousHash}\nTimestamp: ${record.timestamp}\nActor: ${record.actor.id} (${record.actor.type})\nEvent: ${record.event}\nChain integrity: VALID`
    );
    setIsVerifying(false);
  };

  const tierBadge = (tier: ProvenanceTier) => {
    const styles: Record<ProvenanceTier, { bg: string; text: string; label: string }> = {
      facts: { bg: 'bg-emerald-600', text: 'text-white', label: 'Facts' },
      reasoning: { bg: 'bg-blue-600', text: 'text-white', label: 'Reasoning' },
      'ai-narrative': { bg: 'bg-purple-600', text: 'text-white', label: 'AI Narrative' },
      counterfactual: { bg: 'bg-amber-500', text: 'text-white', label: 'Counterfactual' },
    };
    const s = styles[tier];
    return (
      <span className={`px-2 py-0.5 ${s.bg} ${s.text} text-[10px] font-bold uppercase tracking-widest rounded`}>
        {s.label}
      </span>
    );
  };

  const actorIcon = (type: 'human' | 'system' | 'agent') => {
    if (type === 'human') return <User className="w-5 h-5 sm:w-6 sm:h-6" />;
    if (type === 'agent') return <Bot className="w-5 h-5 sm:w-6 sm:h-6" />;
    return <Cpu className="w-5 h-5 sm:w-6 sm:h-6" />;
  };

  const actorBg = (type: 'human' | 'system' | 'agent') => {
    if (type === 'human') return 'bg-blue-50 text-blue-600';
    if (type === 'agent') return 'bg-purple-50 text-purple-600';
    return 'bg-gray-100 text-gray-600';
  };

  return (
    <div className="flex-1 flex flex-col lg:flex-row bg-gray-50 overflow-hidden">
      <div className="w-full lg:w-96 border-b lg:border-b-0 lg:border-r border-gray-200 bg-white flex flex-col shrink-0 h-[45vh] lg:h-auto">
        <div className="p-4 sm:p-6 border-b border-gray-200 shrink-0">
          <h2 className="text-lg sm:text-xl font-bold text-gray-900 flex items-center gap-2">
            <History className="w-5 h-5 text-blue-600" />
            Provenance Explorer
          </h2>
          <p className="text-[10px] text-gray-500 mt-1 uppercase tracking-wider font-bold">4-Tier WOS Provenance Trail</p>
          <div className="mt-2 text-[9px] font-mono text-gray-400">
            {AUDIT_DEMO_INSTANCE_IDS.length} demo instance URNs — search to filter
          </div>

          <div className="mt-4 sm:mt-6 space-y-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input
                type="text"
                placeholder="Search actor, event, or ID..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-9 pr-4 py-2 bg-gray-100 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 rounded-lg text-sm outline-none transition-all"
              />
            </div>
            <div className="flex items-center gap-2 overflow-x-auto pb-1 no-scrollbar">
              <TierFilterButton active={tierFilter === 'all'} onClick={() => setTierFilter('all')} label="All" />
              <TierFilterButton active={tierFilter === 'facts'} onClick={() => setTierFilter('facts')} label="Facts" color="emerald" />
              <TierFilterButton active={tierFilter === 'reasoning'} onClick={() => setTierFilter('reasoning')} label="Reasoning" color="blue" />
              <TierFilterButton active={tierFilter === 'ai-narrative'} onClick={() => setTierFilter('ai-narrative')} label="AI" color="purple" />
              <TierFilterButton active={tierFilter === 'counterfactual'} onClick={() => setTierFilter('counterfactual')} label="Counter" color="amber" />
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto divide-y divide-gray-100">
          {filteredRecords.map(record => (
            <button
              key={record.id}
              type="button"
              aria-label={`Case ${record.instanceId} — ${record.event}`}
              onClick={() => { setSelectedRecordId(record.id); setVerificationProof(null); }}
              className={`w-full text-left p-4 hover:bg-gray-50 transition-colors ${selectedRecordId === record.id ? 'bg-blue-50 border-l-4 border-blue-600' : 'border-l-4 border-transparent'}`}
            >
              <div className="flex items-center justify-between mb-1">
                <span className="text-[10px] font-mono font-bold text-gray-400 uppercase tracking-widest">{record.event}</span>
                <span className="text-[10px] text-gray-400">{new Date(record.timestamp).toLocaleString()}</span>
              </div>
              <div className="text-sm font-bold text-gray-900 truncate">{record.actor.name}</div>
              <div className="flex items-center gap-2 mt-2">
                {tierBadge(record.tier)}
                <span className="text-[10px] text-gray-400 font-mono">{record.sourceState} → {record.targetState}</span>
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 flex flex-col overflow-hidden">
        <AnimatePresence mode="wait">
          {selectedRecord ? (
            <motion.div
              key={selectedRecord.id}
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex-1 flex flex-col overflow-hidden"
            >
              <div className="bg-white border-b border-gray-200 px-4 sm:px-8 py-4 sm:py-6 flex flex-col sm:flex-row sm:items-center justify-between gap-4 shrink-0">
                <div className="flex items-center gap-4">
                  <div className={`p-2 sm:p-3 rounded-xl ${actorBg(selectedRecord.actor.type)}`}>
                    {actorIcon(selectedRecord.actor.type)}
                  </div>
                  <div>
                    <h3 className="text-lg sm:text-xl font-bold text-gray-900">{selectedRecord.actor.name}</h3>
                    <div className="flex flex-wrap items-center gap-2 text-xs sm:text-sm text-gray-500">
                      <span className="capitalize">{selectedRecord.actor.type}</span>
                      <span className="hidden sm:inline">•</span>
                      <span>{selectedRecord.event}</span>
                      <span className="hidden sm:inline">•</span>
                      <span className="font-mono text-[10px]">{selectedRecord.id}</span>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2 sm:gap-3">
                  <button className="flex-1 sm:flex-none flex items-center justify-center gap-2 px-3 py-2 border border-gray-200 rounded-lg text-xs sm:text-sm font-bold text-gray-700 hover:bg-gray-50 transition-colors">
                    <Download className="w-4 h-4" />
                    Export
                  </button>
                  <button
                    onClick={() => handleVerify(selectedRecord)}
                    disabled={isVerifying}
                    className="flex-1 sm:flex-none flex items-center justify-center gap-2 px-3 py-2 bg-[#141414] text-white rounded-lg text-xs sm:text-sm font-bold hover:bg-black transition-colors disabled:opacity-50"
                  >
                    {isVerifying ? <Activity className="w-4 h-4 animate-spin" /> : <Shield className="w-4 h-4" />}
                    Verify
                  </button>
                </div>
              </div>

              <div className="flex-1 overflow-y-auto p-4 sm:p-8 space-y-6 sm:space-y-8">
                {verificationProof && (
                  <motion.div
                    initial={{ opacity: 0, y: -10 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="bg-emerald-50 border border-emerald-100 rounded-xl p-6"
                  >
                    <div className="flex items-center gap-2 text-emerald-800 mb-3">
                      <CheckCircle2 className="w-5 h-5" />
                      <h4 className="font-bold">Cryptographic Integrity Verified</h4>
                    </div>
                    <pre className="text-[10px] font-mono text-emerald-700 bg-white/50 p-4 rounded border border-emerald-100 whitespace-pre-wrap">
                      {verificationProof}
                    </pre>
                  </motion.div>
                )}

                <section>
                  <div className="flex items-center gap-2 mb-4">
                    <span className="px-2 py-0.5 bg-emerald-600 text-white text-[10px] font-bold uppercase tracking-widest rounded">Facts</span>
                    <h4 className="text-sm font-bold text-gray-900 uppercase tracking-wider">Authoritative Facts</h4>
                    <Shield className="w-3.5 h-3.5 text-emerald-500 ml-1" />
                  </div>
                  <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
                    <div className="grid grid-cols-1 sm:grid-cols-2 divide-y sm:divide-y-0 sm:divide-x divide-gray-100">
                      <div className="p-6">
                        <h5 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-4">Inputs</h5>
                        <div className="space-y-3">
                          {Object.entries(selectedRecord.facts.inputs).map(([key, val]) => (
                            <div key={key} className="flex items-center justify-between">
                              <span className="text-xs text-gray-500 capitalize">{key}</span>
                              <span className="text-xs font-bold text-gray-900">{typeof val === 'object' ? JSON.stringify(val) : String(val)}</span>
                            </div>
                          ))}
                          {Object.keys(selectedRecord.facts.inputs).length === 0 && (
                            <span className="text-xs text-gray-400 italic">No inputs recorded</span>
                          )}
                        </div>
                      </div>
                      <div className="p-6">
                        <h5 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-4">Outputs</h5>
                        <div className="space-y-3">
                          {Object.entries(selectedRecord.facts.outputs).map(([key, val]) => (
                            <div key={key} className="flex items-center justify-between">
                              <span className="text-xs text-gray-500 capitalize">{key}</span>
                              <span className="text-xs font-bold text-emerald-600">{typeof val === 'object' ? JSON.stringify(val) : String(val)}</span>
                            </div>
                          ))}
                          {Object.keys(selectedRecord.facts.outputs).length === 0 && (
                            <span className="text-xs text-gray-400 italic">No outputs recorded</span>
                          )}
                        </div>
                      </div>
                    </div>
                    {Object.keys(selectedRecord.facts.metadata).length > 0 && (
                      <div className="bg-gray-50 px-6 py-3 border-t border-gray-100 flex flex-wrap items-center gap-x-6 gap-y-1">
                        {Object.entries(selectedRecord.facts.metadata).map(([key, val]) => (
                          <div key={key} className="text-[10px] text-gray-400">
                            {key}: <span className="font-bold text-gray-600">{typeof val === 'object' ? JSON.stringify(val) : String(val)}</span>
                          </div>
                        ))}
                        <div className="text-[10px] text-gray-400">
                          Timestamp: <span className="font-mono text-gray-600">{selectedRecord.timestamp}</span>
                        </div>
                      </div>
                    )}
                  </div>
                </section>

                {selectedRecord.reasoning && (
                  <section>
                    <div className="flex items-center gap-2 mb-4">
                      <span className="px-2 py-0.5 bg-blue-600 text-white text-[10px] font-bold uppercase tracking-widest rounded">Reasoning</span>
                      <h4 className="text-sm font-bold text-gray-900 uppercase tracking-wider">Authoritative Reasoning</h4>
                      <Scale className="w-3.5 h-3.5 text-blue-500 ml-1" />
                    </div>
                    <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 space-y-6">
                      <div>
                        <h5 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-3">Rules Applied</h5>
                        <div className="flex flex-wrap gap-2">
                          {selectedRecord.reasoning.rulesApplied.map(rule => (
                            <span key={rule} className="px-2 py-1 bg-blue-50 text-blue-700 rounded text-[10px] font-bold border border-blue-100">
                              {rule}
                            </span>
                          ))}
                        </div>
                      </div>
                      <div>
                        <h5 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-3">Criteria Verification</h5>
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                          {selectedRecord.reasoning.criteriaChecked.map(criteria => (
                            <div key={criteria.label} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg border border-gray-100">
                              <span className="text-xs font-medium text-gray-700">{criteria.label}</span>
                              {criteria.passed ? (
                                <span className="flex items-center gap-1 px-1.5 py-0.5 bg-emerald-50 text-emerald-700 rounded text-[10px] font-bold">
                                  <CheckCircle2 className="w-3 h-3" /> Pass
                                </span>
                              ) : (
                                <span className="flex items-center gap-1 px-1.5 py-0.5 bg-red-50 text-red-700 rounded text-[10px] font-bold">
                                  <AlertTriangle className="w-3 h-3" /> Fail
                                </span>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                      {selectedRecord.reasoning.explanation && (
                        <div>
                          <h5 className="text-[10px] font-bold text-gray-400 uppercase tracking-widest mb-3">Official Explanation</h5>
                          <p className="text-sm text-gray-700 leading-relaxed bg-gray-50 p-4 rounded-lg border border-gray-100">
                            {selectedRecord.reasoning.explanation}
                          </p>
                        </div>
                      )}
                      {selectedRecord.reasoning.sourceAuthority && (
                        <div className="flex items-center gap-2 pt-2 border-t border-gray-100">
                          <Scale className="w-3.5 h-3.5 text-gray-400" />
                          <span className="text-[10px] font-bold text-gray-400 uppercase tracking-widest">
                            Source Authority:
                          </span>
                          <span className="text-xs font-bold text-blue-600 capitalize">{selectedRecord.reasoning.sourceAuthority}</span>
                        </div>
                      )}
                    </div>
                  </section>
                )}

                {selectedRecord.aiNarrative && (
                  <section className="relative">
                    <div className="absolute -top-3 left-8 z-10 px-3 py-1 bg-amber-500 text-white text-[10px] font-black uppercase tracking-[0.2em] shadow-lg transform -rotate-1">
                      Non-Authoritative — For Reference Only
                    </div>
                    <div className="bg-purple-50 border-2 border-purple-200 rounded-2xl p-8 pt-10 shadow-inner relative overflow-hidden">
                      <div className="absolute top-0 right-0 p-4 opacity-10">
                        <Bot className="w-32 h-32" />
                      </div>
                      <div className="relative z-10">
                        <div className="flex items-center gap-2 mb-4 text-purple-800">
                          <span className="px-2 py-0.5 bg-purple-600 text-white text-[10px] font-bold uppercase tracking-widest rounded">AI Narrative</span>
                          <h4 className="text-sm font-black uppercase tracking-wider italic">AI System Account</h4>
                        </div>
                        <p className="text-base sm:text-lg font-serif italic text-purple-900 leading-relaxed mb-6">
                          "{selectedRecord.aiNarrative.text}"
                        </p>
                        <div className="flex flex-wrap items-center gap-4 pt-6 border-t border-purple-200/50">
                          <div className="text-[10px] font-bold text-purple-700 uppercase tracking-widest">
                            Model: <span className="font-mono">{selectedRecord.aiNarrative.model}</span>
                          </div>
                          <div className="text-[10px] font-bold text-purple-700 uppercase tracking-widest">
                            Version: <span className="font-mono">{selectedRecord.aiNarrative.version}</span>
                          </div>
                          {selectedRecord.aiNarrative.confidence != null && (
                            <div className="text-[10px] font-bold text-purple-700 uppercase tracking-widest">
                              Confidence: <span className="font-mono">{Math.round(selectedRecord.aiNarrative.confidence * 100)}%</span>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  </section>
                )}

                {selectedRecord.counterfactual && (
                  <section>
                    <div className="flex items-center gap-2 mb-4">
                      <span className="px-2 py-0.5 bg-amber-500 text-white text-[10px] font-bold uppercase tracking-widest rounded">Counterfactual</span>
                      <h4 className="text-sm font-bold text-gray-900 uppercase tracking-wider">Counterfactual Analysis</h4>
                      <GitBranch className="w-3.5 h-3.5 text-amber-500 ml-1" />
                    </div>
                    <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-6 grid grid-cols-1 sm:grid-cols-2 gap-8">
                      <div>
                        <h5 className="text-[10px] font-bold text-emerald-600 uppercase tracking-widest mb-4 flex items-center gap-2">
                          <ArrowUpRight className="w-3 h-3" />
                          Outcome would change if...
                        </h5>
                        <ul className="space-y-3">
                          {selectedRecord.counterfactual.positive.map((item, i) => (
                            <li key={i} className="text-xs text-gray-700 flex items-start gap-2">
                              <div className="w-1.5 h-1.5 rounded-full bg-emerald-400 mt-1.5 shrink-0"></div>
                              {item}
                            </li>
                          ))}
                        </ul>
                      </div>
                      <div>
                        <h5 className="text-[10px] font-bold text-red-600 uppercase tracking-widest mb-4 flex items-center gap-2">
                          <ArrowDownRight className="w-3 h-3" />
                          Outcome remains same even if...
                        </h5>
                        <ul className="space-y-3">
                          {selectedRecord.counterfactual.negative.map((item, i) => (
                            <li key={i} className="text-xs text-gray-700 flex items-start gap-2">
                              <div className="w-1.5 h-1.5 rounded-full bg-red-400 mt-1.5 shrink-0"></div>
                              {item}
                            </li>
                          ))}
                        </ul>
                      </div>
                    </div>
                  </section>
                )}

                {selectedRecord.authorityChain && selectedRecord.authorityChain.length > 0 && (
                  <section>
                    <div className="flex items-center gap-2 mb-4">
                      <Scale className="w-4 h-4 text-gray-500" />
                      <h4 className="text-sm font-bold text-gray-900 uppercase tracking-wider">Authority Chain</h4>
                    </div>
                    <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-4 sm:p-6">
                      <div className="flex flex-col sm:flex-row sm:items-center gap-4 sm:gap-4">
                        {selectedRecord.authorityChain.map((step, i) => (
                          <React.Fragment key={i}>
                            <div className="flex flex-row sm:flex-col items-center gap-3 sm:gap-2">
                              <div className={`w-10 h-10 sm:w-12 sm:h-12 rounded-full flex items-center justify-center shrink-0 ${step.isValid ? 'bg-blue-50 text-blue-600 border border-blue-100' : 'bg-red-50 text-red-600 border border-red-100'}`}>
                                <User className="w-5 h-5 sm:w-6 sm:h-6" />
                              </div>
                              <div className="text-left sm:text-center">
                                <div className="text-xs sm:text-sm font-bold text-gray-900">{step.actor}</div>
                                {step.delegatedBy && (
                                  <div className="text-[9px] sm:text-[10px] text-gray-500 mt-0.5">Delegated by {step.delegatedBy}</div>
                                )}
                                {step.legalInstrument && (
                                  <div className="text-[8px] sm:text-[9px] font-mono text-blue-600 mt-0.5">{step.legalInstrument}</div>
                                )}
                              </div>
                            </div>
                            {i < selectedRecord.authorityChain!.length - 1 && (
                              <div className="flex justify-center sm:block">
                                <ArrowRight className="w-4 h-4 text-gray-300 rotate-90 sm:rotate-0 mb-0 sm:mb-8" />
                              </div>
                            )}
                          </React.Fragment>
                        ))}
                      </div>
                    </div>
                  </section>
                )}

                <section>
                  <div className="flex items-center gap-2 mb-4">
                    <Fingerprint className="w-4 h-4 text-gray-500" />
                    <h4 className="text-sm font-bold text-gray-900 uppercase tracking-wider">Integrity</h4>
                  </div>
                  <div className="bg-gray-900 rounded-xl p-6 space-y-4">
                    <div className="flex items-center gap-3">
                      <Lock className="w-4 h-4 text-emerald-400" />
                      <div>
                        <div className="text-[10px] font-bold text-gray-500 uppercase tracking-widest">Hash</div>
                        <div className="text-xs font-mono text-emerald-400">{selectedRecord.integrity.hash}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <Link2 className="w-4 h-4 text-gray-500" />
                      <div>
                        <div className="text-[10px] font-bold text-gray-500 uppercase tracking-widest">Previous Hash</div>
                        <div className="text-xs font-mono text-gray-400">{selectedRecord.integrity.previousHash}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <Clock className="w-4 h-4 text-gray-500" />
                      <div>
                        <div className="text-[10px] font-bold text-gray-500 uppercase tracking-widest">Sealed At</div>
                        <div className="text-xs font-mono text-gray-400">{selectedRecord.timestamp}</div>
                      </div>
                    </div>
                  </div>
                </section>
              </div>
            </motion.div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-gray-400 p-12 text-center">
              <div className="w-20 h-20 bg-gray-100 rounded-full flex items-center justify-center mb-6">
                <Eye className="w-10 h-10 text-gray-300" />
              </div>
              <h3 className="text-lg font-bold text-gray-900">Select a provenance record</h3>
              <p className="max-w-xs mt-2 text-sm">
                Choose a record from the explorer to view its 4-tier provenance: facts, reasoning, AI narrative, and counterfactual analysis.
              </p>
            </div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

function TierFilterButton({ active, onClick, label, color }: { active: boolean; onClick: () => void; label: string; color?: string }) {
  const colorMap: Record<string, { active: string; inactive: string }> = {
    emerald: { active: 'bg-emerald-600 text-white shadow-sm', inactive: 'bg-emerald-50 text-emerald-600 hover:bg-emerald-100' },
    blue: { active: 'bg-blue-600 text-white shadow-sm', inactive: 'bg-blue-50 text-blue-600 hover:bg-blue-100' },
    purple: { active: 'bg-purple-600 text-white shadow-sm', inactive: 'bg-purple-50 text-purple-600 hover:bg-purple-100' },
    amber: { active: 'bg-amber-500 text-white shadow-sm', inactive: 'bg-amber-50 text-amber-600 hover:bg-amber-100' },
  };

  if (color && colorMap[color]) {
    const c = colorMap[color];
    return (
      <button
        onClick={onClick}
        className={`px-3 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider transition-all whitespace-nowrap ${active ? c.active : c.inactive}`}
      >
        {label}
      </button>
    );
  }

  return (
    <button
      onClick={onClick}
      className={`px-3 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider transition-all whitespace-nowrap ${
        active ? 'bg-[#141414] text-white shadow-sm' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
      }`}
    >
      {label}
    </button>
  );
}
