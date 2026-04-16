import React from 'react';
import { ArrowRight, AlertCircle } from 'lucide-react';
import { motion } from 'motion/react';
import type { StageMetricView } from '../../services/WosPorts';

export function WorkflowHeatmap({ stages }: { stages: StageMetricView[] }) {
  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden h-full flex flex-col">
      <div className="px-8 py-6 border-b border-slate-50 flex items-center justify-between bg-slate-50/30">
        <div>
          <h2 className="text-sm font-black text-slate-900 uppercase tracking-widest">Workflow Heatmap</h2>
          <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest mt-1">Queue depths and latency by stage</p>
        </div>
      </div>
      
      <div className="flex-1 overflow-x-auto flex flex-col justify-center">
        <div className="min-w-full w-max flex flex-col lg:flex-row items-center justify-between relative gap-10 lg:gap-6 p-10 mx-auto">
          {/* Connecting Line (Desktop only) */}
          <div className="hidden lg:block absolute top-1/2 left-10 right-10 h-0.5 bg-slate-100 -translate-y-1/2 z-0"></div>
          {/* Connecting Line (Mobile only) */}
          <div className="lg:hidden absolute left-1/2 top-10 bottom-10 w-0.5 bg-slate-100 -translate-x-1/2 z-0"></div>
          
          {stages.map((stage, index) => (
            <React.Fragment key={stage.name}>
              <div className="relative z-10 flex flex-col items-center group cursor-pointer w-full lg:w-auto">
                {/* Node */}
                <motion.div 
                  whileHover={{ y: -4, scale: 1.05 }}
                  className={`w-full max-w-[180px] lg:w-36 h-24 sm:h-28 rounded-2xl border-2 flex flex-col items-center justify-center bg-white transition-all shadow-sm
                  ${stage.status === 'bottleneck' ? 'border-rose-500 shadow-rose-100 bg-rose-50/10' : 
                    stage.status === 'warning' ? 'border-amber-400 shadow-amber-100 bg-amber-50/10' : 
                    'border-slate-200 hover:border-blue-400'}
                `}>
                  <span className={`text-2xl sm:text-3xl font-black tracking-tight ${
                    stage.status === 'bottleneck' ? 'text-rose-600' : 
                    stage.status === 'warning' ? 'text-amber-600' : 
                    'text-slate-900'
                  }`}>
                    {stage.count}
                  </span>
                  <span className="text-[9px] sm:text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mt-2">Active Cases</span>
                </motion.div>
                
                {/* Label */}
                <div className="mt-4 sm:mt-6 text-center">
                  <h3 className="text-xs sm:text-sm font-black text-slate-900 tracking-tight">{stage.name}</h3>
                  <div className="flex items-center justify-center gap-2 mt-2">
                    <span className="text-[9px] font-black text-slate-400 uppercase tracking-widest">Latency:</span>
                    <span className={`text-[10px] font-black uppercase tracking-wider px-2 py-0.5 rounded-lg border ${
                      stage.status === 'bottleneck' ? 'text-rose-600 bg-rose-50 border-rose-100' : 'text-slate-600 bg-slate-50 border-slate-100'
                    }`}>
                      {stage.avgWait}
                    </span>
                  </div>
                </div>

                {/* Alert Badge */}
                {stage.status === 'bottleneck' && (
                  <div className="absolute -top-3 -right-3 bg-rose-600 text-white p-2 rounded-xl border-4 border-white shadow-lg animate-bounce">
                    <AlertCircle className="w-4 h-4" />
                  </div>
                )}
              </div>
              
              {index < stages.length - 1 && (
                <div className="z-10 bg-white p-2 rounded-xl border border-slate-100 text-slate-300 rotate-90 lg:rotate-0 shadow-sm">
                  <ArrowRight className="w-4 h-4" />
                </div>
              )}
            </React.Fragment>
          ))}
        </div>
      </div>
    </div>
  );
}

