import React from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { AlertTriangle, X } from 'lucide-react';

interface ConfirmationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: React.ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'danger' | 'warning' | 'info';
  disabled?: boolean;
}

export function ConfirmationModal({ 
  isOpen, 
  onClose, 
  onConfirm, 
  title, 
  message, 
  confirmLabel = 'Confirm', 
  cancelLabel = 'Cancel',
  variant = 'info',
  disabled = false
}: ConfirmationModalProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <>
          <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="fixed inset-0 bg-black/40 backdrop-blur-sm z-[100]"
          />
          <div className="fixed inset-0 flex items-center justify-center z-[101] p-4 pointer-events-none">
            <motion.div 
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="bg-white rounded-2xl shadow-2xl w-full max-w-md overflow-hidden pointer-events-auto"
            >
              <div className="p-6">
                <div className="flex items-start justify-between mb-4">
                  <div className={`p-3 rounded-xl ${
                    variant === 'danger' ? 'bg-red-50 text-red-600' : 
                    variant === 'warning' ? 'bg-amber-50 text-amber-600' : 
                    'bg-blue-50 text-blue-600'
                  }`}>
                    <AlertTriangle className="w-6 h-6" />
                  </div>
                  <button onClick={onClose} className="p-2 text-gray-400 hover:bg-gray-100 rounded-full transition-colors">
                    <X className="w-5 h-5" />
                  </button>
                </div>
                
                <h3 className="text-xl font-bold text-gray-900 mb-2">{title}</h3>
                <div className="text-gray-600 leading-relaxed">{message}</div>
              </div>
              
              <div className="px-6 py-4 bg-gray-50 flex flex-col sm:flex-row gap-3 sm:justify-end">
                <button 
                  onClick={onClose}
                  className="px-4 py-2 text-sm font-bold text-gray-700 hover:bg-gray-200 rounded-xl transition-colors order-2 sm:order-1"
                >
                  {cancelLabel}
                </button>
                <button 
                  onClick={() => { onConfirm(); onClose(); }}
                  disabled={disabled}
                  className={`px-6 py-2 text-sm font-bold text-white rounded-xl shadow-lg transition-all order-1 sm:order-2 disabled:opacity-50 disabled:cursor-not-allowed ${
                    variant === 'danger' ? 'bg-red-600 shadow-red-200 hover:bg-red-700' : 
                    variant === 'warning' ? 'bg-amber-600 shadow-amber-200 hover:bg-amber-700' : 
                    'bg-blue-600 shadow-blue-200 hover:bg-blue-700'
                  }`}
                >
                  {confirmLabel}
                </button>
              </div>
            </motion.div>
          </div>
        </>
      )}
    </AnimatePresence>
  );
}
