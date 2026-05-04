import React, { useState } from 'react';
import { 
  FileText, 
  History, 
  ShieldCheck, 
  ZoomIn, 
  ZoomOut, 
  Download, 
  Maximize2, 
  Columns, 
  ChevronRight,
  ChevronLeft,
  Link as LinkIcon,
  AlertCircle
} from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';

type Tab = 'documents' | 'context' | 'policy';

export function ReferenceWorkspace() {
  const [activeTab, setActiveTab] = useState<Tab>('documents');
  const [selectedDoc, setSelectedDoc] = useState('Tax_Return_2025.pdf');
  const [isCompareMode, setIsCompareMode] = useState(false);

  const documents = [
    { id: 'Tax_Return_2025.pdf', type: 'PDF', label: 'Tax Return 2025' },
    { id: 'ID_Scan.jpg', type: 'IMG', label: 'ID Scan' },
    { id: 'Bank_Statement_Mar.pdf', type: 'PDF', label: 'Bank Statement (Mar)' },
  ];

  return (
    <div className="flex flex-col h-full bg-gray-50 border-l border-gray-200 w-full">
      {/* Tab Navigation */}
      <div className="flex items-center bg-white border-b border-gray-200 px-2 shrink-0">
        <button 
          onClick={() => setActiveTab('documents')}
          className={`px-4 py-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2 ${
            activeTab === 'documents' ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <FileText className="w-4 h-4" />
          Documents
          <span className="bg-gray-100 text-gray-600 text-[10px] px-1.5 py-0.5 rounded-full">{documents.length}</span>
        </button>
        <button 
          onClick={() => setActiveTab('context')}
          className={`px-4 py-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2 ${
            activeTab === 'context' ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <History className="w-4 h-4" />
          Context
        </button>
        <button 
          onClick={() => setActiveTab('policy')}
          className={`px-4 py-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2 ${
            activeTab === 'policy' ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <ShieldCheck className="w-4 h-4" />
          Policy
        </button>
      </div>

      <div className="flex-1 overflow-hidden flex flex-col">
        <AnimatePresence mode="wait">
          {activeTab === 'documents' && (
            <motion.div 
              key="docs"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex-1 flex overflow-hidden"
            >
              {/* Document Filmstrip */}
              <div className="w-16 sm:w-20 border-r border-gray-200 bg-white flex flex-col items-center py-4 gap-4 shrink-0 overflow-y-auto">
                {documents.map((doc) => (
                  <button
                    key={doc.id}
                    onClick={() => setSelectedDoc(doc.id)}
                    className={`group relative flex flex-col items-center gap-1 p-2 rounded-lg transition-all ${
                      selectedDoc === doc.id ? 'bg-blue-50 ring-1 ring-blue-200' : 'hover:bg-gray-50'
                    }`}
                  >
                    <div className={`w-10 h-12 sm:w-12 sm:h-16 rounded border shadow-sm flex items-center justify-center ${
                      selectedDoc === doc.id ? 'bg-white border-blue-300' : 'bg-gray-50 border-gray-200'
                    }`}>
                      <FileText className={`w-6 h-6 ${selectedDoc === doc.id ? 'text-blue-600' : 'text-gray-400'}`} />
                    </div>
                    <span className={`text-[8px] sm:text-[10px] text-center leading-tight truncate w-12 sm:w-16 ${
                      selectedDoc === doc.id ? 'text-blue-700 font-medium' : 'text-gray-500'
                    }`}>
                      {doc.label}
                    </span>
                    {selectedDoc === doc.id && (
                      <motion.div layoutId="activeDoc" className="absolute -left-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-blue-600 rounded-r-full" />
                    )}
                  </button>
                ))}
              </div>

              {/* Main Document Viewer Area */}
              <div className="flex-1 flex flex-col bg-gray-100 overflow-hidden">
                {/* Viewer Toolbar */}
                <div className="h-10 bg-white border-b border-gray-200 px-4 flex items-center justify-between shrink-0">
                  <div className="flex items-center gap-4">
                    <span className="text-xs font-medium text-gray-700 truncate max-w-[150px]">{selectedDoc}</span>
                    <div className="h-4 w-px bg-gray-200 hidden sm:block" />
                    <div className="hidden sm:flex items-center gap-1">
                      <button className="p-1 hover:bg-gray-100 rounded text-gray-500"><ZoomOut className="w-4 h-4" /></button>
                      <span className="text-xs text-gray-500 w-10 text-center">100%</span>
                      <button className="p-1 hover:bg-gray-100 rounded text-gray-500"><ZoomIn className="w-4 h-4" /></button>
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <button 
                      onClick={() => setIsCompareMode(!isCompareMode)}
                      className={`p-1.5 rounded flex items-center gap-1.5 transition-colors ${
                        isCompareMode ? 'bg-blue-50 text-blue-600 ring-1 ring-blue-200' : 'text-gray-500 hover:bg-gray-100'
                      }`}
                      title="Compare Documents"
                    >
                      <Columns className="w-4 h-4" />
                      <span className="text-xs font-medium hidden sm:inline">Compare</span>
                    </button>
                    <div className="h-4 w-px bg-gray-200 mx-1" />
                    <button className="p-1.5 hover:bg-gray-100 rounded text-gray-500"><Download className="w-4 h-4" /></button>
                    <button className="p-1.5 hover:bg-gray-100 rounded text-gray-500"><Maximize2 className="w-4 h-4" /></button>
                  </div>
                </div>

                {/* Document Canvas */}
                <div className={`flex-1 overflow-auto p-4 sm:p-8 flex gap-4 transition-all duration-300 ${isCompareMode ? 'justify-start' : 'justify-center'}`}>
                  <div className={`bg-white shadow-xl w-full max-w-[800px] aspect-[8.5/11] relative border border-gray-200 shrink-0 transition-all duration-300 ${isCompareMode ? 'w-1/2' : 'w-full'}`}>
                    <div className="p-8">
                      <h1 className="text-xl sm:text-2xl font-serif font-bold text-center mb-8">Form 1040 (2025)</h1>
                      <div className="space-y-4">
                        <div className="flex border-b border-gray-300 pb-1">
                          <span className="w-1/3 text-xs sm:text-sm font-semibold">Name:</span>
                          <span className="w-2/3 text-xs sm:text-sm font-mono">JOHN DOE</span>
                        </div>
                        <div className="flex border-b border-gray-300 pb-1 relative">
                          <span className="w-1/3 text-xs sm:text-sm font-semibold">Adjusted Gross Income:</span>
                          <span className="w-2/3 text-xs sm:text-sm font-mono">$34,200</span>
                          <div className="absolute inset-0 bg-yellow-200/40 border-2 border-yellow-400 rounded pointer-events-none"></div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {isCompareMode && (
                    <motion.div 
                      initial={{ opacity: 0, scale: 0.95 }}
                      animate={{ opacity: 1, scale: 1 }}
                      className="bg-white shadow-xl w-1/2 max-w-[800px] aspect-[8.5/11] relative border border-gray-200 shrink-0"
                    >
                      <div className="p-8">
                        <h1 className="text-xl sm:text-2xl font-serif font-bold text-center mb-8">Form 1040 (2024)</h1>
                        <div className="space-y-4">
                          <div className="flex border-b border-gray-300 pb-1">
                            <span className="w-1/3 text-xs sm:text-sm font-semibold text-gray-400">Name:</span>
                            <span className="w-2/3 text-xs sm:text-sm font-mono text-gray-400">JOHN DOE</span>
                          </div>
                          <div className="flex border-b border-gray-300 pb-1">
                            <span className="w-1/3 text-xs sm:text-sm font-semibold text-gray-400">Adjusted Gross Income:</span>
                            <span className="w-2/3 text-xs sm:text-sm font-mono text-gray-400">$31,500</span>
                          </div>
                        </div>
                        <div className="mt-12 p-4 bg-blue-50 border border-blue-100 rounded text-xs text-blue-800">
                          Comparing with previous year for income trend analysis.
                        </div>
                      </div>
                    </motion.div>
                  )}
                </div>
              </div>
            </motion.div>
          )}

          {activeTab === 'context' && (
            <motion.div 
              key="context"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex-1 overflow-y-auto p-6 space-y-8 bg-white"
            >
              <section>
                <h3 className="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-4">
                  <FileText className="w-4 h-4" />
                  Case Summary
                </h3>
                <div className="bg-purple-50 border border-purple-100 rounded-xl p-4">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="bg-purple-600 text-white text-[10px] px-1.5 py-0.5 rounded font-bold uppercase tracking-wider">AI Insight</span>
                  </div>
                  <p className="text-sm text-purple-900 leading-relaxed">
                    Applicant is seeking housing benefit. Income appears to be slightly above the standard threshold, but medical expenses of $4,500 may qualify for a deduction under Section 8.2.
                  </p>
                </div>
              </section>

              <section>
                <h3 className="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-4">
                  <LinkIcon className="w-4 h-4" />
                  Related Cases
                </h3>
                <div className="space-y-3">
                  <div className="group p-3 border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 transition-all cursor-pointer">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm font-mono font-semibold text-blue-600">CASE-2024-11A9</span>
                      <span className="text-[10px] font-bold text-emerald-600 bg-emerald-50 px-1.5 py-0.5 rounded uppercase">Approved</span>
                    </div>
                    <p className="text-xs text-gray-600">Prior Annual Review - No discrepancies found.</p>
                  </div>
                </div>
              </section>

              <section>
                <h3 className="text-xs font-bold text-gray-400 uppercase tracking-widest flex items-center gap-2 mb-4">
                  <History className="w-4 h-4" />
                  Case Timeline
                </h3>
                <div className="space-y-6 relative before:absolute before:inset-0 before:ml-[7px] before:-translate-x-px before:h-full before:w-0.5 before:bg-gray-200">
                  <div className="relative flex items-start gap-4">
                    <div className="mt-1.5 flex items-center justify-center w-4 h-4 rounded-full border-2 border-white bg-blue-600 shrink-0 z-10 shadow-sm"></div>
                    <div>
                      <div className="text-sm font-semibold text-gray-900">Application Received</div>
                      <div className="text-xs text-gray-500">Apr 01, 2026 • 09:42 AM</div>
                    </div>
                  </div>
                  <div className="relative flex items-start gap-4">
                    <div className="mt-1.5 flex items-center justify-center w-4 h-4 rounded-full border-2 border-white bg-purple-500 shrink-0 z-10 shadow-sm"></div>
                    <div>
                      <div className="text-sm font-semibold text-gray-900">AI Extraction Completed</div>
                      <div className="text-xs text-gray-500">Apr 01, 2026 • 09:43 AM</div>
                      <div className="mt-1 text-[10px] text-purple-600 font-medium">98% confidence across 12 fields</div>
                    </div>
                  </div>
                </div>
              </section>
            </motion.div>
          )}

          {activeTab === 'policy' && (
            <motion.div 
              key="policy"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex-1 overflow-y-auto p-6 space-y-6 bg-white"
            >
              <div className="bg-amber-50 border border-amber-200 rounded-xl p-4 flex gap-3">
                <AlertCircle className="w-5 h-5 text-amber-600 shrink-0" />
                <div>
                  <h4 className="text-sm font-bold text-amber-900">Active Regulation: v2025.2</h4>
                  <p className="text-xs text-amber-800 mt-1">This case is subject to the 2025 Revised Housing Act. Key changes include adjusted income thresholds for single-parent households.</p>
                </div>
              </div>

              <div className="space-y-4">
                <h3 className="text-xs font-bold text-gray-400 uppercase tracking-widest">Applicable Rules</h3>
                {[
                  { id: 'H-101', title: 'Income Eligibility', status: 'Review Required' },
                  { id: 'H-204', title: 'Medical Deductions', status: 'Met' },
                  { id: 'H-301', title: 'Residency Requirements', status: 'Met' },
                ].map(rule => (
                  <div key={rule.id} className="p-3 border border-gray-100 rounded-lg flex items-center justify-between">
                    <div>
                      <div className="text-xs font-mono text-gray-400">{rule.id}</div>
                      <div className="text-sm font-medium text-gray-900">{rule.title}</div>
                    </div>
                    <span className={`text-[10px] font-bold px-2 py-1 rounded uppercase ${
                      rule.status === 'Met' ? 'bg-emerald-50 text-emerald-700' : 'bg-amber-50 text-amber-700'
                    }`}>
                      {rule.status}
                    </span>
                  </div>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
