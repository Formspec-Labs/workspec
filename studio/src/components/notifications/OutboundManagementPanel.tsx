import React, { useState, useEffect } from 'react';
import { Mail, Send, CheckCircle2, AlertCircle, Eye, RotateCcw, Search, Filter, Hash, Clock, User, X, RefreshCw, Download, ShieldAlert } from 'lucide-react';
import { useBackend } from '../../context/WosContext';
import { OutboundNotification } from '../../types';
import { motion, AnimatePresence } from 'motion/react';

const STUB_OUTBOUND: OutboundNotification[] = [
  { id: 'out1', recipient: 'John Doe', type: 'denial-letter', caseId: 'CASE-2026-0042', status: 'sent', timestamp: '2026-04-09T14:00:00Z', channel: 'email', contentHash: 'sha256:abc123', auditTrail: [{ event: 'Composed', timestamp: '2026-04-09T13:55:00Z', actor: 'System' }, { event: 'Sent', timestamp: '2026-04-09T14:00:00Z', actor: 'Notification Service' }] },
  { id: 'out2', recipient: 'Jane Smith', type: 'request-for-info', caseId: 'CASE-2026-0038', status: 'confirmed', timestamp: '2026-04-08T11:00:00Z', channel: 'mail', contentHash: 'sha256:def456', auditTrail: [{ event: 'Composed', timestamp: '2026-04-08T10:50:00Z', actor: 'System' }, { event: 'Sent', timestamp: '2026-04-08T11:00:00Z', actor: 'Notification Service' }, { event: 'Delivered', timestamp: '2026-04-08T11:30:00Z', actor: 'USPS Tracking' }] },
  { id: 'out3', recipient: 'Maria Garcia', type: 'approval-letter', caseId: 'CASE-2026-0020', status: 'failed', timestamp: '2026-04-07T16:00:00Z', channel: 'email', contentHash: 'sha256:ghi789', auditTrail: [{ event: 'Composed', timestamp: '2026-04-07T15:55:00Z', actor: 'System' }, { event: 'Send Failed', timestamp: '2026-04-07T16:00:00Z', actor: 'Notification Service' }] },
  { id: 'out4', recipient: 'Robert Chen', type: 'status-update', caseId: 'CASE-2026-0015', status: 'sent', timestamp: '2026-04-07T09:00:00Z', channel: 'sms', contentHash: 'sha256:jkl012', auditTrail: [{ event: 'Composed', timestamp: '2026-04-07T08:55:00Z', actor: 'System' }, { event: 'Sent', timestamp: '2026-04-07T09:00:00Z', actor: 'Notification Service' }] },
];

export function OutboundManagementPanel() {
  useBackend();
  const [notifications] = useState<OutboundNotification[]>(STUB_OUTBOUND);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [previewId, setPreviewId] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [isExporting, setIsExporting] = useState(false);

  const loadData = async () => {
    setIsRefreshing(true);
    await new Promise(resolve => setTimeout(resolve, 500));
    setIsRefreshing(false);
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleExport = () => {
    setIsExporting(true);
    setTimeout(() => {
      setIsExporting(false);
    }, 1500);
  };

  const filteredNotifications = notifications.filter(n => 
    n.recipient.toLowerCase().includes(searchQuery.toLowerCase()) ||
    n.caseId.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const selectedNotification = notifications.find(n => n.id === selectedId);
  const previewNotification = notifications.find(n => n.id === previewId);

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'confirmed': return <span className="flex items-center gap-1.5 text-emerald-700 bg-emerald-50 px-2.5 py-1 rounded-lg text-[10px] font-black uppercase tracking-wider border border-emerald-100"><CheckCircle2 className="w-3.5 h-3.5" /> Confirmed</span>;
      case 'sent': return <span className="flex items-center gap-1.5 text-blue-700 bg-blue-50 px-2.5 py-1 rounded-lg text-[10px] font-black uppercase tracking-wider border border-blue-100"><Send className="w-3.5 h-3.5" /> Sent</span>;
      case 'failed': return <span className="flex items-center gap-1.5 text-rose-700 bg-rose-50 px-2.5 py-1 rounded-lg text-[10px] font-black uppercase tracking-wider border border-rose-100"><AlertCircle className="w-3.5 h-3.5" /> Failed</span>;
      default: return <span className="flex items-center gap-1.5 text-slate-600 bg-slate-50 px-2.5 py-1 rounded-lg text-[10px] font-black uppercase tracking-wider border border-slate-100"><Clock className="w-3.5 h-3.5" /> Pending</span>;
    }
  };

  const handleRetry = async (id: string) => {
    loadData();
  };

  return (
    <div className="flex flex-col h-full bg-[#f8fafc] overflow-hidden">
      <div className="bg-white border-b border-slate-200 px-4 sm:px-8 py-6 shrink-0 z-10 shadow-sm">
        <div className="max-w-7xl mx-auto flex flex-col md:flex-row md:items-end justify-between gap-6">
          <div className="space-y-1">
            <div className="flex items-center gap-2 text-blue-600 font-bold text-[10px] uppercase tracking-[0.2em]">
              <Mail className="w-3.5 h-3.5" />
              Correspondence Audit
            </div>
            <h1 className="text-2xl sm:text-3xl font-black text-slate-900 tracking-tight">Outbound Management</h1>
            <p className="text-sm text-slate-500 max-w-md">Monitor, audit, and resend applicant correspondence with full legal traceability.</p>
          </div>
          
          <div className="flex flex-wrap items-center gap-3">
            <div className="relative">
              <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
              <input 
                type="text" 
                placeholder="Search recipient or case..." 
                className="pl-10 pr-4 py-2.5 bg-slate-50 border border-slate-200 rounded-xl text-sm focus:ring-2 focus:ring-blue-500 outline-none w-64 transition-all"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            
            <button 
              onClick={loadData}
              disabled={isRefreshing}
              className="p-2.5 text-slate-500 hover:text-blue-600 hover:bg-blue-50 rounded-xl border border-slate-200 transition-all active:scale-95 disabled:opacity-70"
            >
              <RefreshCw className={`w-5 h-5 ${isRefreshing ? 'animate-spin' : ''}`} />
            </button>
            
            <button 
              onClick={handleExport}
              disabled={isExporting}
              className="flex items-center gap-2 px-4 py-2.5 bg-slate-900 text-white rounded-xl text-sm font-bold shadow-lg shadow-slate-200 hover:bg-slate-800 transition-all active:scale-95 disabled:opacity-70"
            >
              {isExporting ? <RefreshCw className="w-4 h-4 animate-spin" /> : <Download className="w-4 h-4" />}
              {isExporting ? 'Exporting...' : 'Export Logs'}
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 flex flex-col lg:flex-row overflow-hidden">
        <div className="flex-1 overflow-y-auto border-b lg:border-b-0 lg:border-r border-slate-200 bg-white">
          <div className="min-w-full inline-block align-middle">
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                <thead className="sticky top-0 bg-white z-10 shadow-sm">
                  <tr className="border-b border-slate-200">
                    <th className="px-4 sm:px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em]">Recipient</th>
                    <th className="hidden md:table-cell px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em]">Type</th>
                    <th className="hidden sm:table-cell px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em]">Case ID</th>
                    <th className="px-4 sm:px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em]">Status</th>
                    <th className="hidden lg:table-cell px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em]">Sent At</th>
                    <th className="px-4 sm:px-8 py-4 text-[10px] font-black text-slate-400 uppercase tracking-[0.15em] text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-50">
                  {filteredNotifications.length > 0 ? filteredNotifications.map(notif => (
                    <motion.tr 
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      key={notif.id} 
                      className={`hover:bg-slate-50 transition-all cursor-pointer relative group ${selectedId === notif.id ? 'bg-blue-50/50' : ''}`}
                      onClick={() => setSelectedId(notif.id)}
                    >
                      {selectedId === notif.id && (
                        <div className="absolute left-0 top-0 bottom-0 w-1 bg-blue-600" />
                      )}
                      <td className="px-4 sm:px-8 py-5">
                        <div className="text-sm font-bold text-slate-900 truncate max-w-[120px] sm:max-w-none">{notif.recipient}</div>
                        <div className="text-[10px] font-black text-slate-400 uppercase tracking-widest mt-0.5">{notif.channel}</div>
                        <div className="sm:hidden mt-1">
                          <span className="font-mono text-[10px] text-blue-600 font-bold">{notif.caseId}</span>
                        </div>
                      </td>
                      <td className="hidden md:table-cell px-8 py-5">
                        <span className="text-xs font-medium text-slate-700 capitalize bg-slate-100 px-2 py-1 rounded-md">{notif.type.replace('-', ' ')}</span>
                      </td>
                      <td className="hidden sm:table-cell px-8 py-5">
                        <span className="font-mono text-xs text-blue-600 font-bold bg-blue-50 px-2 py-1 rounded-md border border-blue-100">
                          {notif.caseId}
                        </span>
                      </td>
                      <td className="px-4 sm:px-8 py-5">
                        <div className="scale-90 sm:scale-100 origin-left">
                          {getStatusBadge(notif.status)}
                        </div>
                      </td>
                      <td className="hidden lg:table-cell px-8 py-5 text-xs text-slate-500 tabular-nums">
                        {new Date(notif.timestamp).toLocaleString()}
                      </td>
                      <td className="px-4 sm:px-8 py-5 text-right">
                        <div className="flex items-center justify-end gap-1 sm:gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                          <button 
                            onClick={(e) => { e.stopPropagation(); setPreviewId(notif.id); }}
                            className="p-1.5 sm:p-2 hover:bg-white rounded-xl border border-transparent hover:border-slate-200 text-slate-600 shadow-sm transition-all active:scale-90"
                            title="Preview Content"
                          >
                            <Eye className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                          </button>
                          {notif.status === 'failed' && (
                            <button 
                              onClick={(e) => { e.stopPropagation(); handleRetry(notif.id); }}
                              className="p-1.5 sm:p-2 hover:bg-white rounded-xl border border-transparent hover:border-slate-200 text-rose-600 shadow-sm transition-all active:scale-90"
                              title="Retry Delivery"
                            >
                              <RotateCcw className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                            </button>
                          )}
                        </div>
                      </td>
                    </motion.tr>
                  )) : (
                    <tr>
                      <td colSpan={6} className="px-8 py-12 text-center text-slate-400 text-sm font-bold uppercase tracking-widest">
                        No notifications found
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div className="w-full lg:w-96 bg-slate-50 overflow-y-auto p-4 sm:p-8 flex flex-col gap-8 border-t lg:border-t-0 lg:border-l border-slate-200 max-h-[50vh] lg:max-h-none">
          <AnimatePresence mode="wait">
            {selectedNotification ? (
              <motion.div 
                key={selectedNotification.id}
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 20 }}
                className="space-y-8"
              >
                <div>
                  <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-6">Notification Audit Trail</h3>
                  <div className="bg-white rounded-2xl border border-slate-200 p-6 shadow-sm">
                    <div className="flex items-center gap-2 text-[10px] font-mono text-slate-400 mb-6 bg-slate-50 p-2 rounded-lg border border-slate-100">
                      <Hash className="w-3 h-3" />
                      <span className="truncate">{selectedNotification.contentHash}</span>
                    </div>
                    
                    <div className="space-y-8 relative before:absolute before:left-[7px] before:top-2 before:bottom-2 before:w-0.5 before:bg-slate-100">
                      {selectedNotification.auditTrail.map((event, i) => (
                        <div key={i} className="relative pl-28">
                          <div className="absolute left-0 top-1.5 w-4 h-4 rounded-full bg-white border-2 border-blue-600 z-10 shadow-sm"></div>
                          <div className="text-sm font-bold text-slate-900">{event.event}</div>
                          <div className="flex items-center gap-2 text-[10px] text-slate-500 mt-2 font-medium">
                            <Clock className="w-3 h-3" /> {new Date(event.timestamp).toLocaleString()}
                            <span className="mx-1 text-slate-300">•</span>
                            <User className="w-3 h-3" /> {event.actor}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="bg-blue-600 rounded-2xl p-6 text-white shadow-xl shadow-blue-100">
                  <div className="flex items-center gap-2 mb-4">
                    <ShieldAlert className="w-5 h-5 text-blue-200" />
                    <h3 className="text-xs font-black uppercase tracking-wider">Legal Significance</h3>
                  </div>
                  <p className="text-xs text-blue-50 leading-relaxed font-medium">
                    This audit record proves exactly what was sent to <strong>{selectedNotification.recipient}</strong>. The content hash matches the version stored in the immutable case record. This trail is admissible for regulatory compliance and legal appeals.
                  </p>
                </div>
              </motion.div>
            ) : (
              <div className="h-full flex flex-col items-center justify-center text-center px-6">
                <div className="w-20 h-20 bg-slate-100 rounded-3xl flex items-center justify-center mb-6">
                  <Mail className="w-10 h-10 text-slate-300" />
                </div>
                <h3 className="text-sm font-bold text-slate-900 mb-2">No Selection</h3>
                <p className="text-xs text-slate-500 leading-relaxed">Select a notification from the list to view its complete audit trail and legal record.</p>
              </div>
            )}
          </AnimatePresence>
        </div>
      </div>

      <AnimatePresence>
        {previewNotification && (
          <div className="fixed inset-0 bg-slate-900/60 backdrop-blur-sm flex items-center justify-center z-[60] p-4 sm:p-8">
            <motion.div 
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="bg-white rounded-3xl shadow-2xl w-full max-w-5xl max-h-full flex flex-col overflow-hidden"
            >
              <div className="px-8 py-6 border-b border-slate-200 flex items-center justify-between bg-white">
                <div className="space-y-1">
                  <h3 className="font-black text-slate-900 flex items-center gap-2 text-lg tracking-tight">
                    <Eye className="w-5 h-5 text-blue-600" />
                    Notification Preview
                  </h3>
                  <p className="text-xs text-slate-500 font-medium">{previewNotification.type.replace('-', ' ')} • {previewNotification.caseId}</p>
                </div>
                <button 
                  onClick={() => setPreviewId(null)} 
                  className="p-2 hover:bg-slate-100 rounded-xl text-slate-500 transition-all active:scale-90"
                >
                  <X className="w-6 h-6" />
                </button>
              </div>
              
              <div className="flex-1 overflow-y-auto p-6 sm:p-12 bg-slate-100/50">
                <div className="bg-white shadow-2xl mx-auto max-w-[210mm] min-h-[297mm] p-10 sm:p-20 border border-slate-200">
                  <div className="flex justify-between items-start mb-16">
                    <div className="w-32 h-12 bg-slate-900 text-white rounded-lg flex items-center justify-center text-[10px] font-black tracking-widest uppercase">State Gov</div>
                    <div className="text-right text-[10px] text-slate-500 font-bold uppercase tracking-wider leading-relaxed">
                      <p className="text-slate-900">Department of Case Management</p>
                      <p>123 Government Plaza</p>
                      <p>Washington, D.C. 20001</p>
                    </div>
                  </div>
                  
                  <div className="mb-12">
                    <p className="text-sm font-black text-slate-900 uppercase tracking-tight">{previewNotification.recipient}</p>
                    <p className="text-sm text-slate-600">456 Applicant Lane</p>
                    <p className="text-sm text-slate-600">Cityville, ST 12345</p>
                  </div>

                  <div className="mb-12 flex justify-between items-end border-b border-slate-100 pb-6">
                    <div className="space-y-1">
                      <p className="text-[9px] font-black text-slate-400 uppercase tracking-widest">Date Issued</p>
                      <p className="text-sm font-bold text-slate-900">{new Date(previewNotification.timestamp).toLocaleDateString()}</p>
                    </div>
                    <div className="text-right space-y-1">
                      <p className="text-[9px] font-black text-slate-400 uppercase tracking-widest">Case Reference</p>
                      <p className="text-sm font-bold text-slate-900 font-mono">{previewNotification.caseId}</p>
                    </div>
                  </div>

                  <h2 className="text-2xl font-black mb-8 text-slate-900 uppercase tracking-tight">
                    Notice of {previewNotification.type.split('-')[0]}
                  </h2>

                  <div className="space-y-6 text-slate-700 leading-relaxed text-sm">
                    <p>Dear {previewNotification.recipient},</p>
                    <p>
                      This letter serves as official notification regarding your application for benefits under the 
                      Case Management Program. After a thorough review of your submitted documentation and 
                      eligibility criteria, we have reached a determination.
                    </p>
                    <div className="p-6 bg-slate-50 border-l-4 border-slate-900 rounded-r-xl">
                      <p className="font-bold text-slate-900 italic">
                        {previewNotification.type === 'approval-letter' 
                          ? "Your application has been APPROVED. You will receive further instructions regarding benefit disbursement within 10 business days."
                          : "Your application has been DENIED. The specific reasons for this determination are outlined in the attached detailed findings report."}
                      </p>
                    </div>
                    <p>
                      If you disagree with this decision, you have the right to file an appeal within 30 days of 
                      the date of this letter. Instructions for the appeal process can be found on our website.
                    </p>
                  </div>

                  <div className="mt-20">
                    <p className="text-sm font-medium text-slate-500">Sincerely,</p>
                    <div className="mt-8 w-64 h-16 border-b-2 border-slate-900 italic font-serif text-3xl text-slate-900 flex items-end pb-2">Jane Doe</div>
                    <p className="text-sm font-black text-slate-900 mt-4 uppercase tracking-wider">Jane Doe</p>
                    <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest">Case Manager, Region 4</p>
                  </div>
                </div>
              </div>
              
              <div className="px-8 py-6 border-t border-slate-200 bg-white flex justify-end gap-4">
                <button 
                  onClick={() => setPreviewId(null)} 
                  className="px-6 py-2.5 text-sm font-bold text-slate-600 hover:bg-slate-50 rounded-xl transition-all"
                >
                  Close Preview
                </button>
                <button className="px-8 py-2.5 text-sm font-black uppercase tracking-wider bg-blue-600 text-white hover:bg-blue-700 rounded-xl flex items-center gap-3 shadow-lg shadow-blue-100 transition-all active:scale-95">
                  <Send className="w-4 h-4" /> Resend Notification
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}
