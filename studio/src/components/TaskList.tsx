import React, { useState, useMemo, useEffect } from 'react';
import { TaskItem } from './TaskItem';
import { BulkActionBar } from './BulkActionBar';
import { ArrowUpDown, Search, Filter as FilterIcon, X, AlertCircle, FileText } from 'lucide-react';
import { SidebarFilters } from './SidebarFilters';
import { motion, AnimatePresence } from 'motion/react';
import { ConfirmationModal } from './ui/ConfirmationModal';
import { List } from 'react-window';
import { useInbox } from '../context/WosContext';
import type { TaskListItem } from '../services/WosPorts';
import type { InstanceFilter } from '../services/WosBackend';
import type { SortConfig, BulkActionImpact } from '../types';

function mapWosStatus(status: string): string {
  switch (status) {
    case 'created':
    case 'assigned': return 'new';
    case 'claimed': return 'in-progress';
    case 'escalated': return 'escalated';
    case 'delegated': return 'on-hold';
    default: return status;
  }
}

function daysUntilDeadline(deadline?: string): number {
  if (!deadline) return Infinity;
  return Math.ceil((new Date(deadline).getTime() - Date.now()) / (1000 * 60 * 60 * 24));
}

interface TaskFilters {
  status: string[];
  impactLevel: string[];
  configuration: string[];
}

interface TaskListProps {
  tasks: TaskListItem[];
  filters: TaskFilters;
  setFilters: React.Dispatch<React.SetStateAction<TaskFilters>>;
  onTaskClick: (id: string) => void;
}

export function TaskList({ tasks, filters, setFilters, onTaskClick }: TaskListProps) {
  const inbox = useInbox();
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [sortField, setSortField] = useState<'deadline' | 'impactLevel'>('impactLevel');
  const [sortDesc, setSortDesc] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [isMobileFilterOpen, setIsMobileFilterOpen] = useState(false);
  const [isBatchConfirmOpen, setIsBatchConfirmOpen] = useState(false);
  const [isReassignOpen, setIsReassignOpen] = useState(false);
  const [isHoldOpen, setIsHoldOpen] = useState(false);
  const [peekId, setPeekId] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<'all' | 'priority' | 'appeals'>('all');
  const [page, setPage] = useState(1);
  const [totalTasks, setTotalTasks] = useState(0);
  const [batchImpact, setBatchImpact] = useState<BulkActionImpact | null>(null);
  const [isImpactLoading, setIsImpactLoading] = useState(false);

  const savedViews = [
    { id: 'all', label: 'All Tasks', icon: <Search className="w-3.5 h-3.5" /> },
    { id: 'priority', label: 'High Priority', icon: <AlertCircle className="w-3.5 h-3.5 text-rose-500" /> },
    { id: 'appeals', label: 'Pending Appeals', icon: <FileText className="w-3.5 h-3.5 text-blue-500" /> },
  ];

  const handleSort = (field: 'deadline' | 'impactLevel') => {
    if (sortField === field) {
      setSortDesc(!sortDesc);
    } else {
      setSortField(field);
      setSortDesc(field === 'deadline' ? false : true);
    }
  };

  const filteredAndSortedTasks = useMemo(() => {
    let result = tasks
      .filter(task => {
        if (activeView === 'priority' && task.impactLevel !== 'critical' && task.impactLevel !== 'high') return false;
        if (activeView === 'appeals' && !task.taskRef.toLowerCase().includes('appeal')) return false;
        if (filters.status.length > 0 && !filters.status.includes(task.status)) return false;
        if (filters.impactLevel.length > 0 && !filters.impactLevel.includes(task.impactLevel || '')) return false;
        if (searchQuery) {
          const query = searchQuery.toLowerCase();
          return task.taskRef.toLowerCase().includes(query) || task.instanceId.toLowerCase().includes(query);
        }
        return true;
      })
      .sort((a, b) => {
        let comparison = 0;
        if (sortField === 'deadline') {
          comparison = daysUntilDeadline(a.deadline) - daysUntilDeadline(b.deadline);
        } else if (sortField === 'impactLevel') {
          comparison = (a.impactLevel || '').localeCompare(b.impactLevel || '');
        }
        return sortDesc ? -comparison : comparison;
      });
    return result;
  }, [tasks, filters, sortField, sortDesc, searchQuery, activeView]);

  useEffect(() => {
    const filter: InstanceFilter | undefined = activeView === 'priority'
      ? { impactLevel: ['critical', 'high'] }
      : undefined;

    inbox.listTasks(filter).then(res => {
      setTotalTasks(res.total);
    });
  }, [page, sortField, sortDesc, activeView, inbox]);

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredAndSortedTasks.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredAndSortedTasks.map(t => t.taskId)));
    }
  };

  const toggleSelect = (id: string) => {
    const newSet = new Set(selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setSelectedIds(newSet);
  };

  const handleBatchApprove = async () => {
    setIsBatchConfirmOpen(true);
  };

  const confirmBatchApprove = async () => {
    setSelectedIds(new Set());
    setIsBatchConfirmOpen(false);
  };

  const handleReassign = () => {
    setIsReassignOpen(true);
  };

  const confirmReassign = async () => {
    setSelectedIds(new Set());
    setIsReassignOpen(false);
  };

  const handleHold = () => {
    setIsHoldOpen(true);
  };

  const confirmHold = async () => {
    setSelectedIds(new Set());
    setIsHoldOpen(false);
  };

  const TaskRow = ({ index, style }: { index: number; style: React.CSSProperties }) => {
    const task = filteredAndSortedTasks[index];
    if (!task) return null;
    return (
      <div style={style}>
        <TaskItem 
          task={task} 
          isSelected={selectedIds.has(task.taskId)}
          onSelect={toggleSelect}
          onClick={(id) => {
            onTaskClick(id);
            setPeekId(id);
          }}
          onPeek={() => setPeekId(task.taskId)}
        />
      </div>
    );
  };

  const peekedTask = peekId ? tasks.find(t => t.taskId === peekId) : null;

  return (
    <div className="flex-1 flex overflow-hidden bg-white relative">
      <div className={`flex-1 flex flex-col min-w-0 border-r border-slate-200 transition-all duration-300 ${peekId ? 'lg:flex-[0.4]' : 'flex-1'}`}>
        <div className="px-4 sm:px-8 py-6 border-b border-slate-200 flex flex-col gap-6 sticky top-0 bg-white/95 backdrop-blur-md z-10 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="space-y-1">
              <h2 className="text-2xl font-black text-slate-900 tracking-tight">Task Inbox</h2>
              <div className="flex items-center gap-2">
                <span className="flex h-2 w-2 rounded-full bg-emerald-500"></span>
                <span className="text-xs font-bold text-slate-500 uppercase tracking-widest">
                  {filteredAndSortedTasks.length} Active Tasks
                </span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <div className="hidden md:flex items-center gap-1 p-1 bg-slate-100 rounded-xl border border-slate-200 mr-2">
              {savedViews.map(view => (
                <button
                  key={view.id}
                  onClick={() => setActiveView(view.id as any)}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-black uppercase tracking-wider transition-all ${activeView === view.id ? 'bg-white text-slate-900 shadow-sm' : 'text-slate-400 hover:text-slate-600'}`}
                >
                  {view.icon}
                  {view.label}
                </button>
              ))}
            </div>
            <button 
              onClick={() => setIsMobileFilterOpen(true)}
              className="lg:hidden flex items-center gap-2 px-4 py-2 bg-slate-100 text-slate-700 rounded-xl text-sm font-bold hover:bg-slate-200 transition-all active:scale-95"
            >
              <FilterIcon className="w-4 h-4" />
              Filters
            </button>
          </div>
        </div>

        <div className="flex md:hidden items-center gap-1 p-1 bg-slate-100 rounded-xl border border-slate-200 overflow-x-auto no-scrollbar">
          {savedViews.map(view => (
            <button
              key={`mobile-${view.id}`}
              type="button"
              onClick={() => setActiveView(view.id as 'all' | 'priority' | 'appeals')}
              className={`flex shrink-0 items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-black uppercase tracking-wider transition-all ${activeView === view.id ? 'bg-white text-slate-900 shadow-sm' : 'text-slate-400 hover:text-slate-600'}`}
            >
              {view.icon}
              {view.label}
            </button>
          ))}
        </div>

        <div className="flex flex-col md:flex-row md:items-center gap-4">
          <div className="relative flex-1">
            <Search className="w-4 h-4 text-slate-400 absolute left-4 top-1/2 transform -translate-y-1/2" />
            <input 
              type="text" 
              placeholder="Search by case ID, task ref..."
              className="pl-11 pr-10 py-3 bg-slate-50 border border-slate-200 rounded-xl text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 w-full transition-all"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery && (
              <button 
                onClick={() => setSearchQuery('')}
                className="absolute right-3 top-1/2 -translate-y-1/2 p-1.5 text-slate-400 hover:text-slate-600 rounded-full hover:bg-slate-200 transition-colors"
                title="Clear search"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
          
          <div className="flex items-center gap-1.5 p-1.5 bg-slate-100 rounded-xl border border-slate-200 overflow-x-auto no-scrollbar">
            <button 
              onClick={() => handleSort('impactLevel')}
              className={`px-4 py-2 text-xs font-black uppercase tracking-wider rounded-lg flex items-center gap-2 whitespace-nowrap transition-all ${sortField === 'impactLevel' ? 'bg-white shadow-sm text-blue-600' : 'text-slate-500 hover:text-slate-700'}`}
            >
              Impact
              {sortField === 'impactLevel' && <ArrowUpDown className={`w-3.5 h-3.5 transition-transform ${sortDesc ? '' : 'rotate-180'}`} />}
            </button>
            <button 
              onClick={() => handleSort('deadline')}
              className={`px-4 py-2 text-xs font-black uppercase tracking-wider rounded-lg flex items-center gap-2 whitespace-nowrap transition-all ${sortField === 'deadline' ? 'bg-white shadow-sm text-blue-600' : 'text-slate-500 hover:text-slate-700'}`}
            >
              Deadline
              {sortField === 'deadline' && <ArrowUpDown className={`w-3.5 h-3.5 transition-transform ${sortDesc ? '' : 'rotate-180'}`} />}
            </button>
          </div>
        </div>
      </div>

      <div className="px-4 sm:px-8 py-3 border-b border-slate-200 bg-slate-50/50 flex items-center gap-4 text-[10px] font-black uppercase tracking-[0.15em] text-slate-400">
        <div className="flex-shrink-0">
          <input 
            type="checkbox" 
            className="rounded-md border-slate-300 text-blue-600 focus:ring-blue-500 w-4 h-4 cursor-pointer transition-all"
            checked={selectedIds.size > 0 && selectedIds.size === filteredAndSortedTasks.length}
            ref={input => {
              if (input) {
                input.indeterminate = selectedIds.size > 0 && selectedIds.size < filteredAndSortedTasks.length;
              }
            }}
            onChange={toggleSelectAll}
          />
        </div>
        <div className="flex-shrink-0 w-12 text-center hidden xs:block">Impact</div>
        <div className="flex-grow">Case Details</div>
        <div className="hidden lg:block flex-shrink-0 w-40 text-right">Status</div>
      </div>

      <AnimatePresence>
        {isMobileFilterOpen && (
          <>
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setIsMobileFilterOpen(false)}
              className="fixed inset-0 bg-black/40 backdrop-blur-sm z-[60] lg:hidden"
            />
            <motion.div 
              initial={{ y: '100%' }}
              animate={{ y: 0 }}
              exit={{ y: '100%' }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className="fixed inset-x-0 bottom-0 max-h-[80vh] bg-white rounded-t-3xl shadow-2xl z-[70] lg:hidden flex flex-col"
            >
              <div className="p-4 border-b border-gray-100 flex items-center justify-between sticky top-0 bg-white rounded-t-3xl">
                <h3 className="font-bold text-gray-900">Filter Tasks</h3>
                <button onClick={() => setIsMobileFilterOpen(false)} className="p-2 text-gray-400 hover:bg-gray-100 rounded-full">
                  <X className="w-5 h-5" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto">
                <SidebarFilters filters={filters} setFilters={setFilters} />
              </div>
              <div className="p-4 border-t border-gray-100 bg-gray-50">
                <button 
                  onClick={() => setIsMobileFilterOpen(false)}
                  className="w-full py-3 bg-blue-600 text-white font-bold rounded-xl shadow-lg shadow-blue-200"
                >
                  Apply Filters
                </button>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      <div className="flex-1 pb-24">
        {filteredAndSortedTasks.length > 0 ? (
          <List
            rowCount={filteredAndSortedTasks.length}
            rowHeight={100}
            className="no-scrollbar"
            rowComponent={TaskRow as any}
            rowProps={{}}
            style={{ height: 800, width: '100%' }}
          />
        ) : (
          <motion.div 
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="flex flex-col items-center justify-center h-96 text-slate-400 p-8 text-center"
          >
            <div className="w-20 h-20 bg-slate-50 rounded-3xl flex items-center justify-center mb-6 border border-slate-100 shadow-inner">
              <Search className="w-10 h-10 text-slate-200" />
            </div>
            <h3 className="text-xl font-black text-slate-900 tracking-tight mb-2">No tasks found</h3>
            <p className="text-sm font-medium text-slate-500 max-w-xs leading-relaxed">
              We couldn't find any tasks matching your current filters or search query.
            </p>
            <button 
              onClick={() => {
                setFilters({ status: [], impactLevel: [], configuration: [] });
                setSearchQuery('');
              }}
              className="mt-8 px-6 py-2.5 bg-slate-900 text-white rounded-xl text-[10px] font-black uppercase tracking-widest hover:bg-slate-800 transition-all active:scale-95"
            >
              Clear All Filters
            </button>
          </motion.div>
        )}
      </div>

      <BulkActionBar 
        selectedCount={selectedIds.size} 
        onClear={() => setSelectedIds(new Set())}
        onBatchApprove={handleBatchApprove}
        onReassign={handleReassign}
        onHold={handleHold}
      />

      <ConfirmationModal 
        isOpen={isBatchConfirmOpen}
        onClose={() => {
          setIsBatchConfirmOpen(false);
          setBatchImpact(null);
        }}
        onConfirm={confirmBatchApprove}
        title="Batch Approve Cases"
        message={
          <div className="space-y-4">
            <p>Are you sure you want to approve {selectedIds.size} selected cases? This action will be recorded in the audit trail and cannot be undone.</p>
            {batchImpact && (
              <div className={`p-4 rounded-xl border ${batchImpact.riskLevel === 'high' ? 'bg-rose-50 border-rose-100' : batchImpact.riskLevel === 'medium' ? 'bg-amber-50 border-amber-100' : 'bg-slate-50 border-slate-200'}`}>
                <div className="flex items-center gap-2 mb-2">
                  <AlertCircle className={`w-4 h-4 ${batchImpact.riskLevel === 'high' ? 'text-rose-600' : batchImpact.riskLevel === 'medium' ? 'text-amber-600' : 'text-slate-600'}`} />
                  <span className="text-xs font-black uppercase tracking-widest">Impact Summary</span>
                </div>
                <ul className="space-y-1">
                  {batchImpact.warnings.map((w, i) => (
                    <li key={i} className="text-xs font-bold text-slate-700 flex items-start gap-2">
                      <span className="mt-1 w-1 h-1 rounded-full bg-slate-400 flex-shrink-0" />
                      {w}
                    </li>
                  ))}
                  {batchImpact.warnings.length === 0 && (
                    <li className="text-xs font-bold text-slate-500 italic">No critical risks identified.</li>
                  )}
                </ul>
              </div>
            )}
          </div>
        }
        confirmLabel={isImpactLoading ? "Analyzing..." : "Approve All"}
        variant={batchImpact?.riskLevel === 'high' ? 'danger' : 'warning'}
      />

      <ConfirmationModal 
        isOpen={isReassignOpen}
        onClose={() => setIsReassignOpen(false)}
        onConfirm={confirmReassign}
        title="Reassign Cases"
        message={`Are you sure you want to reassign ${selectedIds.size} selected cases?`}
        confirmLabel="Reassign"
        variant="info"
      />

      <ConfirmationModal 
        isOpen={isHoldOpen}
        onClose={() => setIsHoldOpen(false)}
        onConfirm={confirmHold}
        title="Put Cases on Hold"
        message={`Are you sure you want to put ${selectedIds.size} selected cases on hold?`}
        confirmLabel="Put on Hold"
        variant="warning"
      />

      <AnimatePresence>
        {peekId && peekedTask && (
          <motion.div 
            initial={{ x: '100%' }}
            animate={{ x: 0 }}
            exit={{ x: '100%' }}
            className="hidden lg:flex flex-[0.6] flex-col border-l border-slate-200 bg-white z-20"
          >
            <div className="p-6 border-b border-slate-100 flex items-center justify-between bg-slate-50">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 bg-blue-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-blue-100">
                  <Search className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="text-lg font-black text-slate-900 tracking-tight">Case Detail</h3>
                  <p className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Case ID: {peekedTask.instanceId}</p>
                </div>
              </div>
              <button onClick={() => setPeekId(null)} className="p-2 hover:bg-slate-200 rounded-xl text-slate-400 transition-all">
                <X className="w-6 h-6" />
              </button>
            </div>
            <div className="flex-1 overflow-y-auto p-8 space-y-8">
              <div className="p-6 bg-slate-50 rounded-2xl border border-slate-100">
                <h4 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4">Case Summary</h4>
                <p className="text-sm font-bold text-slate-900 leading-relaxed">
                  {peekedTask.taskRef}
                </p>
                <div className="mt-4 flex items-center gap-4">
                  <div className="px-3 py-1 bg-white border border-slate-200 rounded-lg text-[10px] font-black uppercase tracking-wider text-slate-600">
                    Impact: {peekedTask.impactLevel || 'Normal'}
                  </div>
                  {peekedTask.deadline && (
                    <div className="px-3 py-1 bg-white border border-slate-200 rounded-lg text-[10px] font-black uppercase tracking-wider text-slate-600">
                      Deadline: {daysUntilDeadline(peekedTask.deadline)}d
                    </div>
                  )}
                </div>
              </div>
            </div>
            <div className="p-8 border-t border-slate-100 bg-slate-50/50">
              <button 
                onClick={() => {
                  onTaskClick(peekId!);
                }}
                className="w-full py-4 bg-slate-900 text-white rounded-2xl font-black text-xs uppercase tracking-[0.2em] hover:bg-black transition-all active:scale-95 shadow-xl shadow-slate-200"
              >
                Open Full Workspace
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {peekId && peekedTask && (
          <>
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setPeekId(null)}
              className="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-[60] lg:hidden"
            />
            <motion.div 
              initial={{ x: '100%' }}
              animate={{ x: 0 }}
              exit={{ x: '100%' }}
              className="fixed inset-y-0 right-0 w-full sm:w-[500px] bg-white shadow-2xl z-[70] flex flex-col lg:hidden"
            >
              <div className="p-6 border-b border-slate-100 flex items-center justify-between bg-slate-50">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 bg-blue-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-blue-100">
                    <Search className="w-5 h-5" />
                  </div>
                  <div>
                    <h3 className="text-lg font-black text-slate-900 tracking-tight">Quick Peek</h3>
                    <p className="text-[10px] font-black text-slate-400 uppercase tracking-widest">Case ID: {peekedTask.instanceId}</p>
                  </div>
                </div>
                <button onClick={() => setPeekId(null)} className="p-2 hover:bg-slate-200 rounded-xl text-slate-400 transition-all">
                  <X className="w-6 h-6" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto p-8 space-y-8">
                <div className="p-6 bg-slate-50 rounded-2xl border border-slate-100">
                  <h4 className="text-[10px] font-black text-slate-400 uppercase tracking-[0.2em] mb-4">Case Summary</h4>
                  <p className="text-sm font-bold text-slate-900 leading-relaxed">
                    {peekedTask.taskRef}
                  </p>
                  <div className="mt-4 flex items-center gap-4">
                    <div className="px-3 py-1 bg-white border border-slate-200 rounded-lg text-[10px] font-black uppercase tracking-wider text-slate-600">
                      Impact: {peekedTask.impactLevel || 'Normal'}
                    </div>
                    {peekedTask.deadline && (
                      <div className="px-3 py-1 bg-white border border-slate-200 rounded-lg text-[10px] font-black uppercase tracking-wider text-slate-600">
                        Deadline: {daysUntilDeadline(peekedTask.deadline)}d
                      </div>
                    )}
                  </div>
                </div>
              </div>
              <div className="p-8 border-t border-slate-100 bg-slate-50/50">
                <button 
                  onClick={() => {
                    onTaskClick(peekId!);
                    setPeekId(null);
                  }}
                  className="w-full py-4 bg-slate-900 text-white rounded-2xl font-black text-xs uppercase tracking-[0.2em] hover:bg-black transition-all active:scale-95 shadow-xl shadow-slate-200"
                >
                  Open Full Workspace
                </button>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
      </div>
    </div>
  );
}
