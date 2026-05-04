import React from 'react';
import { AlertTriangle, Clock, ShieldAlert } from 'lucide-react';
import type { AlertView } from '../../services/WosPorts';

export function AlertsPanel({ alerts }: { alerts: AlertView[] }) {
  const getIcon = (type: string) => {
    switch (type) {
      case 'drift': return <ShieldAlert className="w-4 h-4" />;
      case 'queue': return <Clock className="w-4 h-4" />;
      case 'sla': return <AlertTriangle className="w-4 h-4" />;
      default: return <AlertTriangle className="w-4 h-4" />;
    }
  };

  const getColors = (severity: string) => {
    switch (severity) {
      case 'critical': return 'border-l-rose-500 bg-rose-50/50 text-rose-600';
      case 'warning': return 'border-l-amber-500 bg-amber-50/50 text-amber-600';
      case 'info': return 'border-l-blue-500 bg-blue-50/50 text-blue-600';
      default: return 'border-l-slate-500 bg-slate-50/50 text-slate-600';
    }
  };

  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden h-full flex flex-col">
      <div className="px-6 py-5 border-b border-slate-50 flex items-center justify-between bg-slate-50/30">
        <h2 className="text-sm font-black text-slate-900 uppercase tracking-widest flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-amber-500" />
          Active Alerts
        </h2>
        <span className="bg-amber-100 text-amber-800 text-[10px] font-black px-2.5 py-1 rounded-lg uppercase tracking-wider border border-amber-200 shadow-sm">{alerts.length} New</span>
      </div>
      
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {alerts.map(alert => {
          const colors = getColors(alert.severity);
          const borderClass = colors.split(' ')[0];
          const bgClass = colors.split(' ')[1];
          const textClass = colors.split(' ')[2];

          return (
            <div key={alert.id} className={`p-5 rounded-xl border border-slate-100 hover:border-slate-200 hover:shadow-md transition-all cursor-pointer border-l-4 ${borderClass} bg-white group`}>
              <div className="flex items-start gap-4">
                <div className={`p-2.5 rounded-xl shrink-0 transition-all group-hover:scale-110 ${bgClass} ${textClass}`}>
                  {getIcon(alert.type)}
                </div>
                <div className="min-w-0 flex-1">
                  <h4 className="text-sm font-black text-slate-900 tracking-tight truncate">{alert.title}</h4>
                  <p className="text-xs text-slate-500 mt-1 leading-relaxed font-medium">{alert.description}</p>
                  <div className="flex items-center gap-2 mt-3">
                    <Clock className="w-3 h-3 text-slate-300" />
                    <span className="text-[10px] font-black text-slate-400 uppercase tracking-widest">{alert.timeAgo}</span>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

