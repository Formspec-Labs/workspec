import React, { useState, useEffect } from 'react';
import { FileText, ExternalLink, Mail, Activity } from 'lucide-react';
import { useBackend } from '../../../context/WosContext';
import type { WosDocumentBundle } from '../../../services/WosBackend';

interface DocumentsTabProps {
  caseId: string;
}

function BindingBadge({ binding }: { binding: string }) {
  const styles: Record<string, string> = {
    formspec: 'bg-blue-50 text-blue-700 border-blue-100',
    jsonSchema: 'bg-violet-50 text-violet-700 border-violet-100',
  };
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium border ${styles[binding] ?? 'bg-gray-50 text-gray-700 border-gray-200'}`}>
      {binding}
    </span>
  );
}

function CategoryBadge({ category }: { category: string }) {
  const styles: Record<string, string> = {
    'adverse-decision': 'bg-red-50 text-red-700 border-red-100',
    'hold-notification': 'bg-amber-50 text-amber-700 border-amber-100',
    'appeal-acknowledgment': 'bg-indigo-50 text-indigo-700 border-indigo-100',
    'sla-warning': 'bg-orange-50 text-orange-700 border-orange-100',
    'case-status-update': 'bg-emerald-50 text-emerald-700 border-emerald-100',
    'resume-notification': 'bg-teal-50 text-teal-700 border-teal-100',
  };
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium border ${styles[category] ?? 'bg-gray-50 text-gray-700 border-gray-200'}`}>
      {category}
    </span>
  );
}

export function DocumentsTab({ caseId }: DocumentsTabProps) {
  const backend = useBackend();
  const [bundle, setBundle] = useState<WosDocumentBundle | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    setIsLoading(true);
    backend.getInstance(caseId).then(instance => {
      if (!instance) {
        setIsLoading(false);
        return;
      }
      backend.loadBundle(instance.definitionUrl).then(b => {
        setBundle(b);
        setIsLoading(false);
      });
    });
  }, [backend, caseId]);

  if (isLoading) {
    return (
      <div className="p-12 flex items-center justify-center">
        <div className="w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  if (!bundle) {
    return (
      <div className="max-w-5xl mx-auto p-8">
        <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
          <div className="p-12 text-center">
            <div className="w-16 h-16 bg-slate-50 rounded-2xl flex items-center justify-center mx-auto mb-6 border border-slate-100">
              <FileText className="w-8 h-8 text-slate-300" />
            </div>
            <h4 className="text-base font-bold text-slate-700 mb-2">No workflow definition found</h4>
            <p className="text-sm text-slate-500 max-w-md mx-auto leading-relaxed">
              Unable to load the workflow definition for this case instance.
            </p>
          </div>
        </div>
      </div>
    );
  }

  const contracts = bundle.workflow.contracts ?? {};
  const contractEntries = Object.entries(contracts);
  const templates = bundle.delivery?.notificationTemplates?.templates ?? {};
  const templateEntries = Object.entries(templates);

  return (
    <div className="max-w-5xl mx-auto p-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-gray-900">Documents &amp; Correspondence</h2>
          <p className="text-sm text-gray-500 mt-1">{bundle.workflow.title}</p>
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
          <FileText className="w-4 h-4 text-gray-500" />
          <h3 className="text-sm font-semibold text-gray-900">Contracts ({contractEntries.length})</h3>
        </div>
        {contractEntries.length === 0 ? (
          <div className="p-8 text-center text-sm text-gray-500">
            No contracts defined in this workflow.
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {contractEntries.map(([name, contract]) => (
              <div key={name} className="p-5 hover:bg-gray-50 transition-colors">
                <div className="flex items-start gap-4">
                  <div className="w-10 h-10 bg-blue-50 rounded-lg flex items-center justify-center flex-shrink-0 border border-blue-100">
                    <FileText className="w-5 h-5 text-blue-600" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="text-sm font-semibold text-gray-900">{name}</h4>
                      <BindingBadge binding={contract.binding} />
                    </div>
                    <p className="text-xs font-mono text-gray-500 mb-1 break-all">{contract.ref}</p>
                    {contract.description && (
                      <p className="text-sm text-gray-600">{contract.description}</p>
                    )}
                  </div>
                  <a
                    href={contract.ref}
                    className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-600 text-white hover:bg-blue-700 transition-colors flex-shrink-0"
                  >
                    <ExternalLink className="w-3 h-3" />
                    View Definition
                  </a>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="bg-white border border-gray-200 rounded-xl shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center gap-2">
          <Mail className="w-4 h-4 text-gray-500" />
          <h3 className="text-sm font-semibold text-gray-900">Correspondence Templates ({templateEntries.length})</h3>
        </div>
        {templateEntries.length === 0 ? (
          <div className="p-8 text-center text-sm text-gray-500">
            No notification templates defined for this workflow.
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {templateEntries.map(([name, template]) => (
              <div key={name} className="p-5 hover:bg-gray-50 transition-colors">
                <div className="flex items-start gap-4">
                  <div className="w-10 h-10 bg-indigo-50 rounded-lg flex items-center justify-center flex-shrink-0 border border-indigo-100">
                    <Mail className="w-5 h-5 text-indigo-600" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="text-sm font-semibold text-gray-900">{name}</h4>
                      <CategoryBadge category={template.category} />
                    </div>
                    {template.subject && (
                      <p className="text-xs text-gray-500 mb-1 font-mono">{template.subject}</p>
                    )}
                    {template.description && (
                      <p className="text-sm text-gray-600 mb-2">{template.description}</p>
                    )}
                    <div className="flex items-center gap-3 flex-wrap">
                      {template.deliveryChannels && (
                        <div className="flex items-center gap-1">
                          {template.deliveryChannels.map(ch => (
                            <span key={ch} className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-gray-100 text-gray-600 border border-gray-200">
                              {ch}
                            </span>
                          ))}
                        </div>
                      )}
                      {template.authority && (
                        <span className="text-xs text-gray-400">{template.authority}</span>
                      )}
                      {template.requiredVariables && (
                        <span className="text-xs text-gray-400">
                          {template.requiredVariables.length} variable{template.requiredVariables.length !== 1 ? 's' : ''}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
