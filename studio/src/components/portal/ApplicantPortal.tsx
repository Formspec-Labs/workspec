import React, { useState, useEffect } from 'react';
import { 
  AlertCircle, 
  Clock, 
  FileText, 
  ShieldAlert, 
  CheckCircle2, 
  ChevronRight, 
  Upload, 
  HelpCircle,
  ArrowRight,
  Info,
  Eye,
  X
} from 'lucide-react';
import { useApplicant } from '../../context/WosContext';
import type { ApplicantDeterminationView } from '../../services/WosPorts';
import { motion, AnimatePresence } from 'motion/react';

export function ApplicantPortal({ caseId: propCaseId }: { caseId?: string }) {
  const applicant = useApplicant();
  const [determination, setDetermination] = useState<ApplicantDeterminationView | null>(null);
  const [isFilingAppeal, setIsFilingAppeal] = useState(false);
  const [selectedEvidence, setSelectedEvidence] = useState<string | null>(null);
  const [appealStep, setAppealStep] = useState(1);
  const [appealData, setAppealData] = useState({
    grounds: [] as string[],
    statement: '',
    hasRepresentative: false,
    repName: '',
    repPhone: '',
    accommodations: [] as string[]
  });

  const caseId = propCaseId || 'CASE-2026-12C5';

  useEffect(() => {
    applicant.getDetermination(caseId).then(setDetermination);
  }, [applicant, caseId]);

  if (!determination) {
    return (
      <div className="flex-1 flex items-center justify-center bg-gray-50">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  const deadline = new Date(determination.deadlineDate);
  const today = new Date('2026-04-09T00:00:00Z');
  const daysRemaining = Math.max(0, Math.ceil((deadline.getTime() - today.getTime()) / (1000 * 60 * 60 * 24)));

  const handleAppealSubmit = async () => {
    const reason = appealData.grounds.length > 0 ? appealData.grounds.join('; ') : appealData.statement || 'No reason provided';
    await applicant.submitAppeal(caseId, reason);
    setDetermination({ ...determination, appealStatus: 'filed' });
    setIsFilingAppeal(false);
  };

  return (
    <div className="flex-1 overflow-y-auto bg-gray-50 font-sans">
      {determination.appealStatus === 'not-filed' && (
        <div className="sticky top-0 z-50 bg-red-600 text-white px-6 py-4 shadow-md flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-6 h-6 shrink-0" />
            <div>
              <h2 className="font-bold text-lg">Action Required: Appeal Deadline Approaching</h2>
              <p className="text-red-100 text-sm">You must file your appeal by {deadline.toLocaleDateString()}</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="bg-white/20 px-4 py-2 rounded-lg text-center">
              <div className="text-2xl font-black">{daysRemaining}</div>
              <div className="text-[10px] uppercase tracking-wider font-bold">Days Left</div>
            </div>
            <button 
              onClick={() => setIsFilingAppeal(true)}
              className="bg-white text-red-600 px-6 py-3 rounded-lg font-bold hover:bg-red-50 transition-colors whitespace-nowrap shadow-sm"
            >
              Start Appeal Now
            </button>
          </div>
        </div>
      )}

      <div className="max-w-3xl mx-auto px-4 py-8 space-y-8">
        
        <div>
          <h1 className="text-3xl font-bold text-gray-900 mb-2">Notice of Determination</h1>
          <p className="text-gray-600">Case ID: {determination.instanceId} • Issued: {new Date(determination.dateIssued).toLocaleDateString()}</p>
        </div>

        {determination.benefitsContinue && (
          <div className="bg-blue-50 border border-blue-200 rounded-xl p-6 flex items-start gap-4">
            <div className="p-2 bg-blue-100 text-blue-700 rounded-full shrink-0">
              <CheckCircle2 className="w-6 h-6" />
            </div>
            <div>
              <h3 className="text-lg font-bold text-blue-900 mb-1">Your benefits will continue</h3>
              <p className="text-blue-800">
                Because you are within the appeal window, your current {determination.programName} benefits will continue without interruption while you decide whether to appeal.
              </p>
            </div>
          </div>
        )}

        <section className="bg-white rounded-2xl border border-gray-200 shadow-sm overflow-hidden">
          <div className="p-8 border-b border-gray-100">
            <h2 className="text-xl font-bold text-gray-900 mb-4">What was decided?</h2>
            <div className="flex items-center gap-3 mb-6">
              <span className={`px-3 py-1 font-bold rounded-full text-sm uppercase tracking-wider ${
                determination.decision === 'denied' ? 'bg-red-100 text-red-800' :
                determination.decision === 'approved' ? 'bg-emerald-100 text-emerald-800' :
                'bg-amber-100 text-amber-800'
              }`}>
                Application {determination.decision === 'denied' ? 'Denied' : determination.decision === 'approved' ? 'Approved' : 'Pending'}
              </span>
              <span className="text-gray-600 font-medium">{determination.programName}</span>
            </div>
            <p className="text-lg text-gray-800 leading-relaxed mb-6">
              {determination.summary}
            </p>

            <div className="bg-gray-50 rounded-xl p-6 border border-gray-100">
              <h3 className="font-bold text-gray-900 mb-3 flex items-center gap-2">
                <FileText className="w-5 h-5 text-gray-500" />
                Evidence we looked at:
              </h3>
              <div className="space-y-2">
                {determination.evidenceConsidered.map((evidence, i) => (
                  <button 
                    key={i} 
                    onClick={() => setSelectedEvidence(evidence)}
                    className="w-full flex items-center justify-between p-3 bg-white border border-gray-200 rounded-lg hover:border-blue-400 hover:shadow-sm transition-all group"
                  >
                    <span className="text-sm text-gray-700 font-medium">{evidence}</span>
                    <div className="flex items-center gap-2 text-[10px] font-black text-blue-600 uppercase tracking-widest opacity-0 group-hover:opacity-100 transition-opacity">
                      <Eye className="w-3.5 h-3.5" />
                      View Source
                    </div>
                  </button>
                ))}
              </div>
            </div>
          </div>

          {determination.aiDisclosure.wasUsed && (
            <div className="bg-slate-50 p-8 border-b border-gray-100">
              <h3 className="font-bold text-slate-900 mb-3 flex items-center gap-2">
                <ShieldAlert className="w-5 h-5 text-slate-500" />
                How was this decision made?
              </h3>
              <div className="space-y-4 text-slate-700">
                <p>{determination.aiDisclosure.description}</p>
                {determination.aiDisclosure.humanReviewer && (
                  <p className="font-medium text-slate-900">Human reviewer: {determination.aiDisclosure.humanReviewer}</p>
                )}
              </div>
            </div>
          )}

          <div className="p-8">
            <h3 className="font-bold text-gray-900 mb-4">What would change this decision?</h3>
            <div className="space-y-4">
              {determination.counterfactuals.positive.map((cf, i) => (
                <div key={i} className="flex items-start gap-3 p-4 bg-emerald-50 text-emerald-900 rounded-xl border border-emerald-100">
                  <CheckCircle2 className="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
                  <p>{cf}</p>
                </div>
              ))}
              {determination.counterfactuals.negative.map((cf, i) => (
                <div key={i} className="flex items-start gap-3 p-4 bg-gray-50 text-gray-700 rounded-xl border border-gray-200">
                  <Info className="w-5 h-5 text-gray-400 shrink-0 mt-0.5" />
                  <p>{cf}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {determination.appealStatus === 'not-filed' && !isFilingAppeal && (
          <section className="bg-white rounded-2xl border border-gray-200 shadow-sm p-8 text-center">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">Do you disagree with this decision?</h2>
            <p className="text-gray-600 mb-8 max-w-lg mx-auto">
              You have the right to appeal this decision and request a hearing before an independent administrative law judge.
            </p>
            <button 
              onClick={() => setIsFilingAppeal(true)}
              className="bg-blue-600 text-white px-8 py-4 rounded-xl font-bold text-lg hover:bg-blue-700 transition-colors shadow-sm"
            >
              Start My Appeal
            </button>
          </section>
        )}

        {determination.appealStatus !== 'not-filed' && (
          <section className="bg-white rounded-2xl border border-gray-200 shadow-sm p-8">
            <h2 className="text-xl font-bold text-gray-900 mb-2">Appeal Progress</h2>
            <p className="text-sm text-gray-600 mb-8 leading-relaxed">
              {determination.summary}
            </p>
            
            <div className="relative">
              <div className="absolute left-4 top-0 bottom-0 w-0.5 bg-gray-100"></div>
              <div className="space-y-8 relative">
                {determination.milestones?.map((milestone, i) => (
                  <StatusStep 
                    key={milestone.id}
                    title={milestone.label} 
                    date={milestone.date || (milestone.status === 'pending' ? 'Upcoming' : 'Pending')} 
                    completed={milestone.status === 'completed'} 
                    active={milestone.status === 'current'}
                    description={milestone.description}
                  />
                ))}
              </div>
            </div>
          </section>
        )}
      </div>

      <AnimatePresence>
        {isFilingAppeal && (
          <div className="fixed inset-0 z-[60] flex items-center justify-center p-4 sm:p-6">
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-gray-900/60 backdrop-blur-sm"
              onClick={() => setIsFilingAppeal(false)}
            />
            <motion.div 
              initial={{ opacity: 0, y: 20, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 20, scale: 0.95 }}
              className="relative bg-white rounded-2xl shadow-xl w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden"
            >
              <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between bg-gray-50 shrink-0">
                <h2 className="text-xl font-bold text-gray-900">File an Appeal</h2>
                <button onClick={() => setIsFilingAppeal(false)} className="text-gray-500 hover:text-gray-700 font-medium">Cancel</button>
              </div>
              
              <div className="flex-1 overflow-y-auto p-6 sm:p-8">
                {appealStep === 1 && (
                  <div className="space-y-6">
                    <div>
                      <h3 className="text-lg font-bold text-gray-900 mb-2">Why are you appealing?</h3>
                      <p className="text-gray-600 mb-4">Select all that apply.</p>
                      <div className="space-y-3">
                        {['The income calculation is wrong', 'My household size is wrong', 'I have new evidence to submit', 'I do not understand the decision', 'Other'].map(ground => (
                          <label key={ground} className="flex items-start gap-3 p-4 border border-gray-200 rounded-xl hover:bg-gray-50 cursor-pointer transition-colors">
                            <input 
                              type="checkbox" 
                              className="mt-1 w-5 h-5 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                              checked={appealData.grounds.includes(ground)}
                              onChange={(e) => {
                                if (e.target.checked) {
                                  setAppealData({ ...appealData, grounds: [...appealData.grounds, ground] });
                                } else {
                                  setAppealData({ ...appealData, grounds: appealData.grounds.filter(g => g !== ground) });
                                }
                              }}
                            />
                            <span className="font-medium text-gray-900">{ground}</span>
                          </label>
                        ))}
                      </div>
                    </div>
                    <div>
                      <h3 className="text-lg font-bold text-gray-900 mb-2">Explain why you disagree (Optional)</h3>
                      <textarea 
                        className="w-full p-4 border border-gray-300 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none resize-none h-32"
                        placeholder="Tell us what you think we got wrong..."
                        value={appealData.statement}
                        onChange={(e) => setAppealData({ ...appealData, statement: e.target.value })}
                      ></textarea>
                    </div>
                  </div>
                )}

                {appealStep === 2 && (
                  <div className="space-y-8">
                    <div>
                      <h3 className="text-lg font-bold text-gray-900 mb-2">Upload Documents (Optional)</h3>
                      <p className="text-gray-600 mb-4">Upload any new evidence, like recent pay stubs or tax returns.</p>
                      <div className="border-2 border-dashed border-gray-300 rounded-xl p-8 text-center hover:bg-gray-50 transition-colors cursor-pointer">
                        <Upload className="w-8 h-8 text-gray-400 mx-auto mb-3" />
                        <p className="font-medium text-gray-900">Click to upload or drag and drop</p>
                        <p className="text-sm text-gray-500 mt-1">PDF, JPG, or PNG (max 10MB)</p>
                      </div>
                    </div>
                    <div>
                      <h3 className="text-lg font-bold text-gray-900 mb-2">Do you need any accommodations?</h3>
                      <div className="space-y-3">
                        {['Interpreter needed', 'Sign language interpreter', 'Large print documents', 'Wheelchair access for in-person hearing'].map(acc => (
                          <label key={acc} className="flex items-center gap-3">
                            <input 
                              type="checkbox" 
                              className="w-5 h-5 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                              checked={appealData.accommodations.includes(acc)}
                              onChange={(e) => {
                                if (e.target.checked) {
                                  setAppealData({ ...appealData, accommodations: [...appealData.accommodations, acc] });
                                } else {
                                  setAppealData({ ...appealData, accommodations: appealData.accommodations.filter(a => a !== acc) });
                                }
                              }}
                            />
                            <span className="text-gray-700">{acc}</span>
                          </label>
                        ))}
                      </div>
                    </div>
                  </div>
                )}

                {appealStep === 3 && (
                  <div className="space-y-6">
                    <div className="bg-blue-50 p-6 rounded-xl border border-blue-100">
                      <h3 className="text-lg font-bold text-blue-900 mb-2">Review and Submit</h3>
                      <p className="text-blue-800 mb-4">Please review your appeal details before submitting.</p>
                      <div className="space-y-4 text-sm">
                        <div>
                          <span className="font-bold text-blue-900 block">Grounds for appeal:</span>
                          <span className="text-blue-800">{appealData.grounds.length > 0 ? appealData.grounds.join(', ') : 'None selected'}</span>
                        </div>
                        {appealData.statement && (
                          <div>
                            <span className="font-bold text-blue-900 block">Statement:</span>
                            <span className="text-blue-800">{appealData.statement}</span>
                          </div>
                        )}
                        <div>
                          <span className="font-bold text-blue-900 block">Accommodations:</span>
                          <span className="text-blue-800">{appealData.accommodations.length > 0 ? appealData.accommodations.join(', ') : 'None requested'}</span>
                        </div>
                      </div>
                    </div>
                    <p className="text-sm text-gray-600">
                      By submitting this form, you are officially requesting an appeal of the determination made on case {caseId}.
                    </p>
                  </div>
                )}
              </div>

              <div className="px-6 py-4 border-t border-gray-200 bg-gray-50 flex items-center justify-between shrink-0">
                {appealStep > 1 ? (
                  <button 
                    onClick={() => setAppealStep(appealStep - 1)}
                    className="px-6 py-2.5 rounded-lg font-bold text-gray-700 hover:bg-gray-200 transition-colors"
                  >
                    Back
                  </button>
                ) : <div></div>}
                
                {appealStep < 3 ? (
                  <button 
                    onClick={() => setAppealStep(appealStep + 1)}
                    className="px-6 py-2.5 bg-blue-600 text-white rounded-lg font-bold hover:bg-blue-700 transition-colors flex items-center gap-2"
                  >
                    Next <ArrowRight className="w-4 h-4" />
                  </button>
                ) : (
                  <button 
                    onClick={handleAppealSubmit}
                    className="px-8 py-2.5 bg-emerald-600 text-white rounded-lg font-bold hover:bg-emerald-700 transition-colors shadow-sm"
                  >
                    Submit Appeal
                  </button>
                )}
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {selectedEvidence && (
          <div className="fixed inset-0 z-[70] flex items-center justify-center p-4 sm:p-6">
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-slate-900/80 backdrop-blur-md"
              onClick={() => setSelectedEvidence(null)}
            />
            <motion.div 
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="relative bg-white rounded-2xl shadow-2xl w-full max-w-4xl h-[80vh] flex flex-col overflow-hidden border border-slate-200"
            >
              <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50 shrink-0">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-blue-600 text-white rounded-lg">
                    <FileText className="w-5 h-5" />
                  </div>
                  <div>
                    <h3 className="text-lg font-bold text-slate-900">{selectedEvidence}</h3>
                    <p className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Official Record • Verified Source</p>
                  </div>
                </div>
                <button 
                  onClick={() => setSelectedEvidence(null)}
                  aria-label="Close"
                  className="p-2 hover:bg-slate-200 rounded-xl text-slate-400 transition-all active:scale-90"
                >
                  <X className="w-6 h-6" />
                </button>
              </div>
              <div className="flex-1 bg-slate-100 p-8 overflow-y-auto">
                <div className="max-w-2xl mx-auto bg-white shadow-lg rounded-lg p-10 min-h-full">
                  <div className="border-b-2 border-slate-900 pb-6 mb-8 flex justify-between items-start">
                    <div>
                      <h4 className="text-xl font-black uppercase tracking-tighter">Official Document</h4>
                      <p className="text-xs font-bold text-slate-400 uppercase tracking-widest mt-1">Provenance ID: {Math.random().toString(36).substring(7).toUpperCase()}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs font-black uppercase tracking-widest">Date Received</p>
                      <p className="text-sm font-bold">Jan 12, 2026</p>
                    </div>
                  </div>
                  <div className="space-y-6">
                    <div className="p-4 bg-blue-50 border-l-4 border-blue-600">
                      <p className="text-xs font-black text-blue-900 uppercase tracking-widest mb-2">System Highlight</p>
                      <p className="text-sm text-blue-800 leading-relaxed font-medium italic">
                        "The reported monthly income of $4,250.00 exceeds the maximum threshold of $3,800.00 for a household size of 2 under the current regulatory version FY2026-Q3."
                      </p>
                    </div>
                    <div className="space-y-4">
                      <div className="h-4 bg-slate-100 rounded w-3/4"></div>
                      <div className="h-4 bg-slate-100 rounded w-full"></div>
                      <div className="h-4 bg-slate-100 rounded w-5/6"></div>
                      <div className="h-4 bg-slate-100 rounded w-4/6"></div>
                    </div>
                    <div className="pt-8 border-t border-slate-100 flex justify-center">
                      <div className="px-4 py-2 bg-slate-50 border border-slate-200 rounded-lg flex items-center gap-2">
                        <ShieldAlert className="w-4 h-4 text-slate-400" />
                        <span className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">End of Verified Content</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

function StatusStep({ title, date, completed, active, description }: { title: string; date: string; completed: boolean; active: boolean; description?: string }) {
  return (
    <div className="flex items-start gap-4 sm:gap-6">
      <div className={`relative z-10 w-8 h-8 sm:w-10 sm:h-10 rounded-full flex items-center justify-center shrink-0 mt-0.5 sm:mt-0 ${
        completed ? 'bg-emerald-500 text-white' : 
        active ? 'bg-blue-600 text-white ring-4 ring-blue-100' : 
        'bg-gray-200 text-gray-400'
      }`}>
        {completed ? <CheckCircle2 className="w-5 h-5 sm:w-6 sm:h-6" /> : <div className="w-2.5 h-2.5 sm:w-3 sm:h-3 rounded-full bg-current"></div>}
      </div>
      <div className="min-w-0 flex-1">
        <h4 className={`font-bold text-base sm:text-lg truncate ${active ? 'text-blue-900' : completed ? 'text-gray-900' : 'text-gray-500'}`}>{title}</h4>
        <div className="flex items-center justify-between gap-4">
          <p className={`text-xs sm:text-sm ${active ? 'text-blue-700 font-medium' : 'text-gray-500'}`}>{date}</p>
          {active && <span className="text-[10px] font-black text-blue-600 uppercase tracking-widest animate-pulse">Current Stage</span>}
        </div>
        {description && (
          <p className="mt-2 text-xs text-gray-500 leading-relaxed italic">{description}</p>
        )}
      </div>
    </div>
  );
}
