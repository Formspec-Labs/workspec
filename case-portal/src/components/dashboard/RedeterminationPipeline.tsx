import React from 'react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { CalendarDays, Users } from 'lucide-react';
import type { PipelineDataPoint } from '../../services/WosPorts';

export function RedeterminationPipeline({ data }: { data: PipelineDataPoint[] }) {
  return (
    <div className="bg-white rounded-2xl border border-slate-200 shadow-sm overflow-hidden flex flex-col">
      <div className="px-8 py-6 border-b border-slate-50 flex items-center justify-between bg-slate-50/30">
        <div>
          <h2 className="text-sm font-black text-slate-900 uppercase tracking-widest flex items-center gap-3">
            <CalendarDays className="w-5 h-5 text-blue-600" />
            Redetermination Pipeline
          </h2>
          <p className="text-[10px] font-bold text-slate-400 uppercase tracking-widest mt-1">Upcoming volume vs. team capacity</p>
        </div>
      </div>
      
      <div className="p-8 flex-1 flex flex-col">
        <div className="grid grid-cols-2 gap-6 mb-8">
          <div className="bg-slate-50/50 rounded-2xl p-5 border border-slate-100 shadow-sm">
            <div className="text-[10px] font-black text-slate-400 uppercase tracking-widest mb-2">Upcoming (90d)</div>
            <div className="text-3xl font-black text-slate-900 tracking-tight">1,880</div>
          </div>
          <div className="bg-rose-50/50 rounded-2xl p-5 border border-rose-100 shadow-sm">
            <div className="text-[10px] font-black text-rose-600 uppercase tracking-widest mb-2">Currently Overdue</div>
            <div className="text-3xl font-black text-rose-700 tracking-tight">42</div>
          </div>
        </div>

        <div className="flex-1 min-h-[250px] w-full">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={data} margin={{ top: 20, right: 30, left: 0, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#f1f5f9" />
              <XAxis 
                dataKey="name" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 700 }} 
                dy={15} 
              />
              <YAxis 
                axisLine={false} 
                tickLine={false} 
                tick={{ fontSize: 10, fill: '#94a3b8', fontWeight: 700 }} 
                dx={-15}
              />
              <Tooltip 
                cursor={{ fill: '#f8fafc' }}
                contentStyle={{ 
                  borderRadius: '16px', 
                  border: '1px solid #f1f5f9', 
                  boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
                  padding: '12px'
                }}
              />
              <Legend 
                verticalAlign="top" 
                align="right"
                height={48} 
                iconType="circle" 
                wrapperStyle={{ fontSize: '10px', fontWeight: 900, textTransform: 'uppercase', letterSpacing: '0.1em', color: '#64748b' }} 
              />
              <Bar dataKey="volume" name="Expected Volume" fill="#3b82f6" radius={[6, 6, 0, 0]} barSize={48} />
              <Bar dataKey="capacity" name="Projected Capacity" fill="#e2e8f0" radius={[6, 6, 0, 0]} barSize={48} />
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="mt-8 flex items-start gap-4 p-6 bg-amber-50/50 border border-amber-100 rounded-2xl shadow-sm">
          <div className="p-2 bg-amber-100 rounded-xl text-amber-600 shrink-0">
            <Users className="w-5 h-5" />
          </div>
          <div className="text-sm text-amber-800 leading-relaxed">
            <div className="font-black uppercase tracking-widest text-[10px] mb-1">Staffing Alert</div>
            <p className="font-medium">
              Projected volume for the 31-60 day window (<span className="font-black">820 cases</span>) exceeds current team capacity (<span className="font-black">500 cases</span>). Recommend cross-training or temporary reassignment to handle the spike.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

