import React, { useState, useEffect } from 'react';
import { Bell, User, ChevronDown, CheckCircle2, Clock, AlertTriangle, Inbox, LayoutDashboard, Mail, Search, Settings, Shield, History, BarChart3, Menu, X } from 'lucide-react';
import { NotificationTray } from './notifications/NotificationTray';
import { useBackend } from '../context/WosContext';
import { Notification } from '../types';
import { motion, AnimatePresence } from 'motion/react';

const STUB_NOTIFICATIONS: Notification[] = [
  { id: 'n1', type: 'assignment', urgency: 'info', title: 'New case assigned', message: 'CASE-2026-0042 has been assigned to you.', timestamp: '2026-04-09T15:30:00Z', read: false, link: { type: 'task', id: 'task-1' } },
  { id: 'n2', type: 'sla-warning', urgency: 'warning', title: 'SLA approaching', message: 'CASE-2026-0038 deadline in 2 days.', timestamp: '2026-04-09T14:00:00Z', read: false, link: { type: 'case', id: 'urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8' } },
  { id: 'n3', type: 'system', urgency: 'info', title: 'System update', message: 'Regulatory version FY2026-Q3 is now active.', timestamp: '2026-04-08T09:00:00Z', read: true },
];

interface HeaderProps {
  onViewInbox: () => void;
  onViewDashboard: () => void;
  onViewOutbound: () => void;
  onViewDesigner: () => void;
  onViewAdmin: () => void;
  onViewAudit: () => void;
  onViewPortal: () => void;
  onViewReports: () => void;
  onViewSampleCase: () => void;
  onNavigate: (link: { type: string; id: string }) => void;
  currentView: 'inbox' | 'dashboard' | 'outbound' | 'workspace' | 'viewer' | 'designer' | 'admin' | 'audit' | 'portal' | 'reports';
}

export function Header({ onViewInbox, onViewDashboard, onViewOutbound, onViewDesigner, onViewAdmin, onViewAudit, onViewPortal, onViewReports, onViewSampleCase, onNavigate, currentView }: HeaderProps) {
  useBackend();
  const [showNotifications, setShowNotifications] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [showProfileMenu, setShowProfileMenu] = useState(false);
  const [showSettingsMenu, setShowSettingsMenu] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    const count = STUB_NOTIFICATIONS.filter(n => !n.read).length;
    setUnreadCount(count);
    const interval = setInterval(() => {
      setUnreadCount(count);
    }, 30000);
    return () => clearInterval(interval);
  }, []);

  const handleSearch = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && searchQuery.trim()) {
      onViewSampleCase();
      setSearchQuery('');
    }
  };

  const navItems = [
    { label: 'Inbox', icon: Inbox, action: onViewInbox, view: 'inbox' },
    { label: 'Dashboard', icon: LayoutDashboard, action: onViewDashboard, view: 'dashboard' },
    { label: 'Outbound', icon: Mail, action: onViewOutbound, view: 'outbound' },
    { label: 'Designer', icon: Settings, action: onViewDesigner, view: 'designer' },
    { label: 'Admin', icon: Shield, action: onViewAdmin, view: 'admin' },
    { label: 'Audit', icon: History, action: onViewAudit, view: 'audit' },
    { label: 'Reports', icon: BarChart3, action: onViewReports, view: 'reports' },
  ];

  return (
    <header className="bg-white border-b border-gray-200 px-4 sm:px-6 py-3 flex items-center justify-between sticky top-0 z-40 shrink-0">
      <div className="flex items-center gap-4 lg:gap-8">
        <button 
          onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          className="lg:hidden p-2 -ml-2 text-gray-500 hover:bg-gray-100 rounded-md"
          aria-label="Toggle mobile menu"
        >
          {isMobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
        </button>

        <div className="flex items-center gap-2 cursor-pointer" onClick={onViewInbox}>
          <div className="w-8 h-8 bg-blue-700 rounded flex items-center justify-center text-white font-bold text-sm shrink-0">
            WOS
          </div>
          <h1 className="text-base sm:text-lg font-semibold text-gray-900 tracking-tight truncate max-w-[120px] sm:max-w-none">Case Management</h1>
        </div>

        <nav className="hidden lg:flex items-center gap-1">
          {navItems.map((item) => (
            <button 
              key={item.label}
              onClick={item.action}
              className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${currentView === item.view ? 'text-blue-600 bg-blue-50' : 'text-gray-600 hover:text-blue-600 hover:bg-blue-50'}`}
            >
              <item.icon className="w-4 h-4" />
              {item.label}
            </button>
          ))}
          <div className="w-px h-6 bg-gray-300 mx-2"></div>
          <button 
            onClick={onViewPortal}
            className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${currentView === 'portal' ? 'text-emerald-700 bg-emerald-50' : 'text-emerald-600 hover:bg-emerald-50'}`}
          >
            <User className="w-4 h-4" />
            Applicant Portal
          </button>
        </nav>
      </div>

      <AnimatePresence>
        {isMobileMenuOpen && (
          <>
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setIsMobileMenuOpen(false)}
              className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40 lg:hidden"
            />
            <motion.div 
              initial={{ x: '-100%' }}
              animate={{ x: 0 }}
              exit={{ x: '-100%' }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className="fixed inset-y-0 left-0 w-72 bg-white shadow-xl z-[100] lg:hidden flex flex-col"
            >
              <div className="p-4 border-b border-gray-200 flex items-center justify-between bg-gray-50">
                <div className="flex items-center gap-2">
                  <div className="w-8 h-8 bg-blue-700 rounded flex items-center justify-center text-white font-bold text-sm">
                    WOS
                  </div>
                  <span className="font-bold text-gray-900">Navigation</span>
                </div>
                <button onClick={() => setIsMobileMenuOpen(false)} className="p-2 text-gray-400 hover:bg-gray-200 rounded-md">
                  <X className="w-5 h-5" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto p-4 space-y-1">
                {navItems.map((item) => (
                  <button 
                    key={item.label}
                    onClick={() => { item.action(); setIsMobileMenuOpen(false); }}
                    className={`flex items-center gap-3 w-full px-4 py-3 text-sm font-bold rounded-xl transition-all ${currentView === item.view ? 'bg-blue-600 text-white shadow-lg shadow-blue-200' : 'text-gray-600 hover:bg-gray-100'}`}
                  >
                    <item.icon className="w-5 h-5" />
                    {item.label}
                  </button>
                ))}
                <div className="h-px bg-gray-100 my-4"></div>
                <button 
                  onClick={() => { onViewPortal(); setIsMobileMenuOpen(false); }}
                  className={`flex items-center gap-3 w-full px-4 py-3 text-sm font-bold rounded-xl transition-all ${currentView === 'portal' ? 'bg-emerald-600 text-white shadow-lg shadow-emerald-200' : 'text-emerald-600 hover:bg-emerald-50'}`}
                >
                  <User className="w-5 h-5" />
                  Applicant Portal
                </button>
              </div>
              <div className="p-4 border-t border-gray-100 bg-gray-50">
                <div className="flex items-center gap-3 p-2">
                  <div className="w-10 h-10 bg-blue-600 rounded-full flex items-center justify-center text-white font-bold">JD</div>
                  <div>
                    <p className="text-sm font-bold text-gray-900">Jane Doe</p>
                    <p className="text-[10px] font-medium text-gray-500 uppercase tracking-wider">Supervisor</p>
                  </div>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>




      <div className="flex items-center gap-2 sm:gap-4">
        <div className="relative hidden xl:block">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input 
            type="text" 
            placeholder="Search cases... (Press Enter)" 
            className="pl-9 pr-4 py-1.5 bg-gray-100 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 rounded-lg text-sm w-64 transition-all outline-none"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={handleSearch}
          />
        </div>

        <button onClick={onViewSampleCase} className="hidden md:block text-xs font-medium text-gray-500 hover:text-blue-600 border border-gray-200 hover:border-blue-200 px-2 py-1.5 rounded transition-colors">
          Sample Case
        </button>

        <div className="hidden sm:block h-6 w-px bg-gray-200 mx-1"></div>

        <div className="relative">
          <button 
            onClick={() => {
              setShowNotifications(!showNotifications);
              setShowProfileMenu(false);
              setShowSettingsMenu(false);
            }}
            className={`p-2 rounded-full relative transition-colors ${showNotifications ? 'bg-blue-50 text-blue-600' : 'text-gray-500 hover:bg-gray-100'}`}
          >
            <Bell className="w-5 h-5" />
            {unreadCount > 0 && (
              <span className="absolute top-1.5 right-1.5 w-4 h-4 bg-red-500 border-2 border-white rounded-full text-[10px] font-bold text-white flex items-center justify-center">
                {unreadCount}
              </span>
            )}
          </button>

          {showNotifications && (
            <NotificationTray 
              onClose={() => setShowNotifications(false)} 
              onNavigate={(link) => {
                setShowNotifications(false);
                onNavigate(link);
              }}
            />
          )}
        </div>

        <div className="relative hidden sm:block">
          <button 
            onClick={() => {
              setShowSettingsMenu(!showSettingsMenu);
              setShowNotifications(false);
              setShowProfileMenu(false);
            }}
            className={`p-2 rounded-full transition-colors ${showSettingsMenu ? 'bg-blue-50 text-blue-600' : 'text-gray-500 hover:bg-gray-100'}`}
          >
            <Settings className="w-5 h-5" />
          </button>
          
          <AnimatePresence>
            {showSettingsMenu && (
              <motion.div 
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 10 }}
                className="absolute right-0 mt-2 w-48 bg-white rounded-xl shadow-xl border border-gray-100 py-2 z-50"
              >
                <div className="px-4 py-2 border-b border-gray-100">
                  <p className="text-xs font-bold text-gray-500 uppercase tracking-wider">Preferences</p>
                </div>
                <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50">Theme Settings</button>
                <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50">Notification Rules</button>
                <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50">Keyboard Shortcuts</button>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        <div className="relative">
          <button 
            onClick={() => {
              setShowProfileMenu(!showProfileMenu);
              setShowNotifications(false);
              setShowSettingsMenu(false);
            }}
            className="flex items-center gap-2 hover:bg-gray-50 py-1 px-1 sm:px-2 rounded-md transition-colors border border-transparent hover:border-gray-200"
          >
            <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-full flex items-center justify-center text-white font-bold text-xs shadow-sm border-2 border-white ring-1 ring-gray-200 shrink-0">
              JD
            </div>
            <div className="text-left hidden sm:block">
              <p className="text-sm font-bold text-gray-900 leading-none">Jane Doe</p>
              <p className="text-[10px] font-medium text-gray-500 mt-1 uppercase tracking-wider">Supervisor</p>
            </div>
            <ChevronDown className={`w-4 h-4 text-gray-400 hidden sm:block transition-transform ${showProfileMenu ? 'rotate-180' : ''}`} />
          </button>
          
          <AnimatePresence>
            {showProfileMenu && (
              <motion.div 
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 10 }}
                className="absolute right-0 mt-2 w-56 bg-white rounded-xl shadow-xl border border-gray-100 py-2 z-50"
              >
                <div className="px-4 py-3 border-b border-gray-100">
                  <p className="text-sm font-bold text-gray-900">Jane Doe</p>
                  <p className="text-xs text-gray-500">jane.doe@example.com</p>
                </div>
                <div className="py-1">
                  <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 flex items-center gap-2">
                    <User className="w-4 h-4" /> My Profile
                  </button>
                  <button className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 flex items-center gap-2">
                    <Shield className="w-4 h-4" /> Security
                  </button>
                </div>
                <div className="border-t border-gray-100 py-1">
                  <button 
                    onClick={() => {
                      setShowProfileMenu(false);
                    }}
                    className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 font-medium"
                  >
                    Sign out
                  </button>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </header>
  );
}
