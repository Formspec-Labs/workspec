import React from 'react';
import { TrendingUp, TrendingDown, Clock, CheckCircle2, AlertTriangle, Sparkles } from 'lucide-react';
import type { DashboardMetrics } from '../../services/WosPorts';

import { motion } from 'motion/react';

interface KPICardProps {
  title: string;
  value: string | number;
  trend: number; // positive is good, negative is bad (usually)
  trendLabel: string;
  icon: React.ReactNode;
  inverseTrend?: boolean; // if true, negative trend is good (e.g., processing time)
  delay?: number;
}

function KPICard({ title, value, trend, trendLabel, icon, inverseTrend = false, delay = 0 }: KPICardProps) {
  const isPositive = trend > 0;
  const isGood = inverseTrend ? !isPositive : isPositive;
  
  return (
    <motion.div 
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay }}
      className="bg-white rounded-2xl border border-slate-200 p-6 shadow-sm hover:shadow-md transition-all group"
    >
      <div className="flex items-start justify-between mb-4">
        <h3 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em]">{title}</h3>
        <div className="p-2.5 bg-slate-50 rounded-xl text-slate-400 group-hover:text-blue-600 group-hover:bg-blue-50 transition-all">
          {icon}
        </div>
      </div>
      <div className="flex items-baseline gap-2">
        <span className="text-3xl font-black text-slate-900 tracking-tight">{value}</span>
      </div>
      <div className="mt-4 flex items-center gap-2 text-xs">
        <span className={`flex items-center gap-1 px-2 py-0.5 rounded-lg font-black uppercase tracking-wider text-[9px] ${isGood ? 'bg-emerald-50 text-emerald-600 border border-emerald-100' : 'bg-rose-50 text-rose-600 border border-rose-100'}`}>
          {isPositive ? <TrendingUp className="w-3 h-3" /> : <TrendingDown className="w-3 h-3" />}
          {Math.abs(trend)}%
        </span>
        <span className="text-slate-400 font-bold uppercase tracking-widest text-[9px]">{trendLabel}</span>
      </div>
    </motion.div>
  );
}

export function KPISection({ metrics }: { metrics: DashboardMetrics }) {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-6">
      <KPICard 
        title="Active Cases" 
        value={metrics.activeInstances} 
        trend={metrics.activeInstancesTrend}
        trendLabel="vs last week" 
        icon={<AlertTriangle className="w-5 h-5" />} 
        inverseTrend={true}
        delay={0.1}
      />
      <KPICard 
        title="Completed (7d)" 
        value={metrics.completed7d.toLocaleString()} 
        trend={metrics.completed7dTrend} 
        trendLabel="vs last week" 
        icon={<CheckCircle2 className="w-5 h-5" />} 
        delay={0.2}
      />
      <KPICard 
        title="SLA Compliance" 
        value={`${metrics.slaCompliance}%`} 
        trend={metrics.slaComplianceTrend} 
        trendLabel="vs last week" 
        icon={<Clock className="w-5 h-5" />} 
        delay={0.3}
      />
      <KPICard 
        title="Avg Processing Time" 
        value={`${metrics.avgProcessingTimeDays} days`} 
        trend={metrics.avgProcessingTimeTrend} 
        trendLabel="vs last week" 
        icon={<Clock className="w-5 h-5" />} 
        inverseTrend={true}
        delay={0.4}
      />
      <KPICard 
        title="AI Acceptance Rate" 
        value={`${metrics.aiAcceptanceRate}%`} 
        trend={metrics.aiAcceptanceRateTrend} 
        trendLabel="vs last week" 
        icon={<Sparkles className="w-5 h-5" />} 
        delay={0.5}
      />
    </div>
  );
}

