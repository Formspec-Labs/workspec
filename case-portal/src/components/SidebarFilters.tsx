import React from 'react';
import { Filter, ChevronDown } from 'lucide-react';

interface InboxFilters {
  status: string[];
  impactLevel: string[];
  configuration: string[];
}

interface SidebarFiltersProps {
  filters: InboxFilters;
  setFilters: React.Dispatch<React.SetStateAction<InboxFilters>>;
  className?: string;
}

const TASK_STATUSES: { value: string; label: string }[] = [
  { value: 'created', label: 'New' },
  { value: 'assigned', label: 'Assigned' },
  { value: 'claimed', label: 'Claimed' },
  { value: 'delegated', label: 'Delegated' },
  { value: 'escalated', label: 'Escalated' },
];

const IMPACT_LEVELS = [
  { value: 'rights-impacting', label: 'Rights-Impacting' },
  { value: 'safety-impacting', label: 'Safety-Impacting' },
  { value: 'operational', label: 'Operational' },
  { value: 'informational', label: 'Informational' },
];

export function SidebarFilters({ filters, setFilters, className = '' }: SidebarFiltersProps) {
  const [expandedSections, setExpandedSections] = React.useState<Set<string>>(new Set(['status', 'impact']));

  const toggleSection = (section: string) => {
    const newSet = new Set(expandedSections);
    if (newSet.has(section)) newSet.delete(section);
    else newSet.add(section);
    setExpandedSections(newSet);
  };

  const handleToggle = (field: keyof InboxFilters, value: string) => {
    setFilters(prev => ({
      ...prev,
      [field]: (prev[field] as string[]).includes(value)
        ? (prev[field] as string[]).filter(s => s !== value)
        : [...(prev[field] as string[]), value],
    }));
  };

  const clearAll = () => setFilters({ status: [], impactLevel: [], configuration: [] });

  const activeCount = filters.status.length + filters.impactLevel.length;

  return (
    <div className={`flex flex-col h-full bg-gray-50 ${className}`}>
      <div className="px-4 py-4 border-b border-gray-200 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-bold text-gray-700 uppercase tracking-wider">
          <Filter className="w-4 h-4" />
          Filters
          {activeCount > 0 && (
            <span className="bg-blue-600 text-white text-[10px] font-bold px-1.5 py-0.5 rounded-full">{activeCount}</span>
          )}
        </div>
        {activeCount > 0 && (
          <button onClick={clearAll} className="text-[10px] font-bold text-blue-600 hover:text-blue-800 uppercase tracking-wider">Clear</button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        <FilterSection title="Task Status" expanded={expandedSections.has('status')} onToggle={() => toggleSection('status')}>
          {TASK_STATUSES.map(s => (
            <FilterCheckbox key={s.value} label={s.label} checked={filters.status.includes(s.value)} onChange={() => handleToggle('status', s.value)} />
          ))}
        </FilterSection>

        <FilterSection title="Impact Level" expanded={expandedSections.has('impact')} onToggle={() => toggleSection('impact')}>
          {IMPACT_LEVELS.map(l => (
            <FilterCheckbox key={l.value} label={l.label} checked={filters.impactLevel.includes(l.value)} onChange={() => handleToggle('impactLevel', l.value)} />
          ))}
        </FilterSection>
      </div>
    </div>
  );
}

function FilterSection({ title, expanded, onToggle, children }: { title: string; expanded: boolean; onToggle: () => void; children: React.ReactNode }) {
  return (
    <div className="border-b border-gray-100">
      <button onClick={onToggle} className="w-full px-4 py-3 flex items-center justify-between text-xs font-bold text-gray-600 uppercase tracking-wider hover:bg-gray-100">
        {title}
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${expanded ? 'rotate-180' : ''}`} />
      </button>
      {expanded && <div className="px-4 pb-3 space-y-2">{children}</div>}
    </div>
  );
}

function FilterCheckbox({ label, checked, onChange }: { label: string; checked: boolean; onChange: () => void }) {
  return (
    <label className="flex items-center gap-2 text-sm text-gray-700 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={onChange} className="rounded border-gray-300 text-blue-600 focus:ring-blue-500" />
      {label}
    </label>
  );
}
