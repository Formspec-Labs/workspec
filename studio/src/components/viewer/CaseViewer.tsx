import React, { useState } from 'react';
import { CaseHeader } from './CaseHeader';
import { TimelineTab } from './tabs/TimelineTab';
import { CaseFileTab } from './tabs/CaseFileTab';
import { RelatedCasesTab } from './tabs/RelatedCasesTab';
import { ReviewHistoryTab } from './tabs/ReviewHistoryTab';
import { DocumentsTab } from './tabs/DocumentsTab';
import { motion, AnimatePresence } from 'motion/react';

interface CaseViewerProps {
  caseId: string;
  onBack: () => void;
}

export type TabType = 'timeline' | 'case-file' | 'related' | 'review-history' | 'documents';

export function CaseViewer({ caseId, onBack }: CaseViewerProps) {
  const [activeTab, setActiveTab] = useState<TabType>('timeline');

  return (
    <div className="flex flex-col flex-1 overflow-hidden bg-[#f8fafc]">
      <CaseHeader caseId={caseId} onBack={onBack} activeTab={activeTab} onTabChange={setActiveTab} />
      
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-7xl mx-auto p-4 sm:p-8">
          <AnimatePresence mode="wait">
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              className="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden"
            >
              {activeTab === 'timeline' && <TimelineTab caseId={caseId} />}
              {activeTab === 'case-file' && <CaseFileTab caseId={caseId} />}
              {activeTab === 'related' && <RelatedCasesTab caseId={caseId} />}
              {activeTab === 'review-history' && <ReviewHistoryTab caseId={caseId} />}
              {activeTab === 'documents' && <DocumentsTab caseId={caseId} />}
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}

