import React, { useState, useEffect } from 'react';
import { Bell, Check, Clock, AlertTriangle, UserPlus, ShieldAlert, ExternalLink, X } from 'lucide-react';
import { useBackend } from '../../context/WosContext';
import { Notification } from '../../types';

const STUB_NOTIFICATIONS: Notification[] = [
  { id: 'n1', type: 'assignment', urgency: 'info', title: 'New case assigned', message: 'CASE-2026-0042 has been assigned to you.', timestamp: '2026-04-09T15:30:00Z', read: false, link: { type: 'task', id: 'task-1' } },
  { id: 'n2', type: 'sla-warning', urgency: 'warning', title: 'SLA approaching', message: 'CASE-2026-0038 deadline in 2 days.', timestamp: '2026-04-09T14:00:00Z', read: false, link: { type: 'case', id: 'urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8' } },
  { id: 'n3', type: 'sla-breach', urgency: 'critical', title: 'SLA breached', message: 'CASE-2026-0020 has exceeded the 30-day deadline.', timestamp: '2026-04-09T10:00:00Z', read: true },
  { id: 'n4', type: 'system', urgency: 'info', title: 'System update', message: 'Regulatory version FY2026-Q3 is now active.', timestamp: '2026-04-08T09:00:00Z', read: true },
];

interface NotificationTrayProps {
  onClose: () => void;
  onNavigate: (link: { type: string; id: string }) => void;
}

export function NotificationTray({ onClose, onNavigate }: NotificationTrayProps) {
  useBackend();
  const [notifications, setNotifications] = useState<Notification[]>(STUB_NOTIFICATIONS);
  const [filter, setFilter] = useState<'all' | 'unread'>('all');

  const unreadCount = notifications.filter(n => !n.read).length;

  const getIcon = (type: string, urgency: string) => {
    const colorClass = urgency === 'critical' ? 'text-red-500' : urgency === 'warning' ? 'text-amber-500' : 'text-blue-500';
    switch (type) {
      case 'assignment': return <UserPlus className={`w-4 h-4 ${colorClass}`} />;
      case 'sla-warning': return <Clock className={`w-4 h-4 ${colorClass}`} />;
      case 'sla-breach': return <ShieldAlert className={`w-4 h-4 ${colorClass}`} />;
      case 'hold-expired': return <AlertTriangle className={`w-4 h-4 ${colorClass}`} />;
      default: return <Bell className={`w-4 h-4 ${colorClass}`} />;
    }
  };

  const filteredNotifications = filter === 'unread' 
    ? notifications.filter(n => !n.read) 
    : notifications;

  const handleAcknowledgeAll = async () => {
    setNotifications(prev => prev.map(n => ({ ...n, read: true })));
  };

  const handleRead = async (id: string) => {
    setNotifications(prev => prev.map(n => n.id === id ? { ...n, read: true } : n));
  };

  return (
    <div className="absolute right-0 mt-2 w-96 bg-white rounded-xl shadow-2xl border border-gray-200 overflow-hidden z-50 animate-in fade-in zoom-in-95 duration-100 origin-top-right">
      <div className="px-4 py-3 border-b border-gray-200 bg-gray-50 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="font-bold text-gray-900">Notifications</h3>
          {unreadCount > 0 && (
            <span className="bg-blue-600 text-white text-[10px] font-bold px-1.5 py-0.5 rounded-full">
              {unreadCount}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button 
            onClick={handleAcknowledgeAll}
            className="text-xs text-blue-600 hover:underline font-medium"
          >
            Mark all read
          </button>
          <button onClick={onClose} className="p-1 hover:bg-gray-200 rounded text-gray-400">
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      <div className="flex border-b border-gray-100">
        <button 
          onClick={() => setFilter('all')}
          className={`flex-1 py-2 text-xs font-medium border-b-2 transition-colors ${filter === 'all' ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}`}
        >
          All
        </button>
        <button 
          onClick={() => setFilter('unread')}
          className={`flex-1 py-2 text-xs font-medium border-b-2 transition-colors ${filter === 'unread' ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}`}
        >
          Unread
        </button>
      </div>

      <div className="max-h-[400px] overflow-y-auto divide-y divide-gray-100">
        {filteredNotifications.length > 0 ? (
          filteredNotifications.map(notification => (
            <div 
              key={notification.id} 
              className={`p-4 hover:bg-gray-50 transition-colors cursor-pointer group relative ${!notification.read ? 'bg-blue-50/30' : ''}`}
              onClick={() => {
                handleRead(notification.id);
                if (notification.link) onNavigate(notification.link);
              }}
            >
              <div className="flex items-start gap-3">
                <div className={`mt-1 p-2 rounded-lg bg-white border border-gray-100 shadow-sm`}>
                  {getIcon(notification.type, notification.urgency)}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between gap-2">
                    <h4 className={`text-sm font-semibold truncate ${!notification.read ? 'text-gray-900' : 'text-gray-600'}`}>
                      {notification.title}
                    </h4>
                    <span className="text-[10px] text-gray-400 whitespace-nowrap">
                      {new Date(notification.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </span>
                  </div>
                  <p className="text-xs text-gray-500 mt-0.5 line-clamp-2">{notification.message}</p>
                  
                  {notification.link && (
                    <div className="mt-2 flex items-center gap-1 text-[10px] font-bold text-blue-600 uppercase tracking-wider group-hover:underline">
                      View {notification.link.type} <ExternalLink className="w-2.5 h-2.5" />
                    </div>
                  )}
                </div>
              </div>
              {!notification.read && (
                <div className="absolute left-0 top-0 bottom-0 w-1 bg-blue-600"></div>
              )}
            </div>
          ))
        ) : (
          <div className="p-8 text-center">
            <Bell className="w-8 h-8 text-gray-200 mx-auto mb-2" />
            <p className="text-sm text-gray-400">No notifications found</p>
          </div>
        )}
      </div>

      <div className="p-3 border-t border-gray-200 bg-gray-50 text-center">
        <button className="text-xs font-semibold text-gray-500 hover:text-gray-700 transition-colors">
          Notification Settings
        </button>
      </div>
    </div>
  );
}
