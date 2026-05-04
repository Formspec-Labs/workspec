import React from 'react';
import { motion } from 'motion/react';
import { AlertTriangle, Clock, Users, ArrowRight, CheckCircle2, ShieldAlert } from 'lucide-react';

export function InterventionWorkspace() {
  return (
    <motion.div 
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-7xl mx-auto space-y-8"
    >
      <div className="bg-white border border-rose-200 rounded-3xl p-8 shadow-sm relative overflow-hidden">
        <div className="absolute top-0 left-0 w-2 h-full bg-rose-500" />
        <div className="flex items-center gap-4 mb-6">
          <div className="p-3 bg-rose-100 text-rose-600 rounded-2xl">
            <ShieldAlert className="w-6 h-6" />
          </div>
          <div>
            <h2 className="text-xl font-black text-slate-900 tracking-tight">Active Intervention Required</h2>
            <p className="text-sm text-slate-500 font-medium">12 cases in 'Verification' are approaching SLA breach. Immediate routing action needed.</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-slate-100">
                <th className="py-4 px-4 text-[10px] font-black uppercase tracking-widest text-slate-400">Case ID</th>
                <th className="py-4 px-4 text-[10px] font-black uppercase tracking-widest text-slate-400">Time in Stage</th>
                <th className="py-4 px-4 text-[10px] font-black uppercase tracking-widest text-slate-400">Complexity</th>
                <th className="py-4 px-4 text-[10px] font-black uppercase tracking-widest text-slate-400 text-right">Action</th>
              </tr>
            </thead>
            <tbody>
              {[1, 2, 3, 4, 5].map((i) => (
                <tr key={i} className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors">
                  <td className="py-4 px-4">
                    <div className="font-bold text-slate-900 text-sm">CASE-2026-{8000 + i}</div>
                    <div className="text-[10px] text-slate-500 font-medium">Income Verification</div>
                  </td>
                  <td className="py-4 px-4">
                    <div className="flex items-center gap-2 text-rose-600 font-bold text-sm">
                      <Clock className="w-4 h-4" />
                      47h 12m
                    </div>
                    <div className="text-[10px] text-slate-400">SLA: 48h</div>
                  </td>
                  <td className="py-4 px-4">
                    <span className="px-2.5 py-1 bg-amber-100 text-amber-700 rounded-lg text-[10px] font-black uppercase tracking-wider">
                      High
                    </span>
                  </td>
                  <td className="py-4 px-4 text-right">
                    <div className="flex items-center justify-end gap-2">
                      <button className="p-2 text-slate-400 hover:text-blue-600 hover:bg-blue-50 rounded-xl transition-colors">
                        <Users className="w-4 h-4" />
                      </button>
                      <button className="px-4 py-2 bg-slate-900 text-white rounded-xl text-[10px] font-black uppercase tracking-wider hover:bg-black transition-colors shadow-md">
                        Reassign
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        
        <div className="mt-6 flex items-center justify-between pt-6 border-t border-slate-100">
          <div className="text-sm font-medium text-slate-500">Showing 5 of 12 critical cases</div>
          <button className="flex items-center gap-2 text-blue-600 font-bold text-sm hover:text-blue-700 transition-colors">
            View All Critical Cases <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        <div className="bg-white border border-slate-200 rounded-3xl p-8 shadow-sm">
          <h3 className="text-sm font-black text-slate-900 uppercase tracking-wider mb-6">Bulk Actions</h3>
          <div className="space-y-3">
            <button className="w-full flex items-center justify-between p-4 bg-slate-50 hover:bg-blue-50 border border-slate-100 hover:border-blue-200 rounded-2xl transition-all group">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-white rounded-xl shadow-sm group-hover:text-blue-600"><Users className="w-5 h-5" /></div>
                <div className="text-left">
                  <div className="text-sm font-bold text-slate-900 group-hover:text-blue-700">Reassign to Tiger Team</div>
                  <div className="text-[10px] text-slate-500 font-medium">Route all 12 cases to senior analysts</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-slate-300 group-hover:text-blue-500" />
            </button>
            <button className="w-full flex items-center justify-between p-4 bg-slate-50 hover:bg-amber-50 border border-slate-100 hover:border-amber-200 rounded-2xl transition-all group">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-white rounded-xl shadow-sm group-hover:text-amber-600"><Clock className="w-5 h-5" /></div>
                <div className="text-left">
                  <div className="text-sm font-bold text-slate-900 group-hover:text-amber-700">Extend SLA by 24h</div>
                  <div className="text-[10px] text-slate-500 font-medium">Requires Director approval</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-slate-300 group-hover:text-amber-500" />
            </button>
          </div>
        </div>

        <div className="bg-white border border-slate-200 rounded-3xl p-8 shadow-sm">
          <h3 className="text-sm font-black text-slate-900 uppercase tracking-wider mb-6">Root Cause Analysis</h3>
          <div className="p-5 bg-blue-50 border border-blue-100 rounded-2xl">
            <div className="flex items-start gap-3">
              <AlertTriangle className="w-5 h-5 text-blue-600 shrink-0 mt-0.5" />
              <div>
                <p className="text-sm text-slate-700 font-medium leading-relaxed">
                  The bottleneck in 'Verification' is correlated with a 40% increase in cases requiring manual income verification due to a recent policy change.
                </p>
                <button className="mt-3 text-[10px] font-black uppercase tracking-widest text-blue-600 hover:text-blue-700 transition-colors flex items-center gap-1">
                  View Policy Impact Report <ArrowRight className="w-3 h-3" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
