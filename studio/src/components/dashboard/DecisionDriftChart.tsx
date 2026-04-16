import React from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, ReferenceLine } from 'recharts';
import { ShieldAlert } from 'lucide-react';
import type { DriftDataPoint } from '../../services/WosPorts';

export function DecisionDriftChart({ data }: { data: DriftDataPoint[] }) {
  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden">
      <div className="px-8 py-6 border-b border-slate-50 flex items-center justify-between bg-slate-50/30">
        <div>
          <h2 className="text-sm font-black text-slate-900 uppercase tracking-widest flex items-center gap-3">
            Decision Drift Monitor
            <span className="bg-rose-100 text-rose-700 text-[9px] uppercase tracking-[0.2em] font-black px-2.5 py-1 rounded-lg flex items-center gap-1.5 border border-rose-200 shadow-sm">
              <ShieldAlert className="w-3.5 h-3.5" /> Alert Active
            </span>
          </h2>
          <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest mt-1">AI override rate vs. review latency</p>
        </div>
      </div>
      
      <div className="p-8">
        <div className="h-80 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 20, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
              <XAxis 
                dataKey="week" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 700 }} 
                dy={15} 
              />
              <YAxis 
                yAxisId="left" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 700 }} 
                dx={-15} 
              />
              <YAxis 
                yAxisId="right" 
                orientation="right" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 700 }} 
                dx={15} 
              />
              <Tooltip 
                contentStyle={{ 
                  borderRadius: '16px', 
                  border: '1px solid #f1f5f9', 
                  boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
                  padding: '12px'
                }}
                labelStyle={{ fontWeight: 900, color: '#0f172a', marginBottom: '8px', fontSize: '12px', textTransform: 'uppercase', letterSpacing: '0.1em' }}
              />
              <Legend 
                verticalAlign="top" 
                align="right"
                height={48} 
                iconType="circle" 
                wrapperStyle={{ fontSize: '10px', fontWeight: 900, textTransform: 'uppercase', letterSpacing: '0.1em', color: '#64748b' }} 
              />
              <ReferenceLine 
                x="W5" 
                stroke="#f43f5e" 
                strokeDasharray="4 4" 
                label={{ position: 'top', value: 'DRIFT DETECTED', fill: '#f43f5e', fontSize: 9, fontWeight: 900, letterSpacing: '0.1em' }} 
                yAxisId="left" 
              />
              <Line 
                yAxisId="left" 
                type="monotone" 
                dataKey="overrideRate" 
                name="Override Rate (%)" 
                stroke="#3b82f6" 
                strokeWidth={4} 
                dot={{ r: 5, strokeWidth: 3, fill: '#fff' }} 
                activeDot={{ r: 8, strokeWidth: 0 }} 
              />
              <Line 
                yAxisId="right" 
                type="monotone" 
                dataKey="timeOnTask" 
                name="Avg Time on Task (mins)" 
                stroke="#8b5cf6" 
                strokeWidth={4} 
                dot={{ r: 5, strokeWidth: 3, fill: '#fff' }} 
                activeDot={{ r: 8, strokeWidth: 0 }} 
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
        <div className="mt-8 bg-rose-50/50 border border-rose-100 rounded-2xl p-6 text-sm text-rose-800 shadow-sm">
          <div className="flex items-center gap-2 mb-2">
            <ShieldAlert className="w-4 h-4 text-rose-600" />
            <span className="font-black uppercase tracking-widest text-[10px]">System Analysis</span>
          </div>
          <p className="leading-relaxed font-medium text-rose-700">
            Both override rates and time-on-task have dropped significantly since Week 5. This pattern strongly suggests workers are <span className="font-black underline decoration-rose-300">rubber-stamping AI suggestions</span> without performing genuine review. Intervention recommended.
          </p>
        </div>
      </div>
    </div>
  );
}

