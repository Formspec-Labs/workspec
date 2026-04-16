import React, { useState } from 'react';
import { AITouchedField } from './AITouchedField';
import { AlertTriangle } from 'lucide-react';
import type { CaseInstanceView } from '../../services/WosBackend';
import type { WOSKernelDocument, FieldDefinition } from '../../types/wos/kernel';

interface CaseFormProps {
  instance: CaseInstanceView;
  kernel: WOSKernelDocument | null;
}

function fieldLabel(key: string): string {
  return key
    .replace(/([A-Z])/g, ' $1')
    .replace(/[_-]/g, ' ')
    .replace(/^./, (s) => s.toUpperCase())
    .trim();
}

function formatFieldValue(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'object') return JSON.stringify(value, null, 2);
  return String(value);
}

function renderObjectFields(
  obj: Record<string, unknown>,
  parentKey: string,
  definedFieldKeys: Set<string>,
) {
  return Object.entries(obj).map(([key, value]) => {
    const fullKey = parentKey ? `${parentKey}.${key}` : key;
    const label = fieldLabel(key);
    const strVal = formatFieldValue(value);

    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      return (
        <div key={fullKey} className="col-span-2">
          <label className="block text-sm font-medium text-gray-700 mb-1">{label}</label>
          <div className="bg-gray-50 border border-gray-200 rounded-md p-3 text-sm font-mono text-gray-700 whitespace-pre-wrap">
            {strVal}
          </div>
        </div>
      );
    }

    return (
      <div key={fullKey}>
        <label className="block text-sm font-medium text-gray-700 mb-1">{label}</label>
        <input
          type="text"
          className="w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
          defaultValue={strVal}
        />
      </div>
    );
  });
}

export function CaseForm({ instance, kernel }: CaseFormProps) {
  const [independentAssessmentDone, setIndependentAssessmentDone] = useState(false);
  const [determination, setDetermination] = useState('');

  const caseFileFields = kernel?.caseFile?.fields ?? {};
  const caseState = instance.caseState ?? {};
  const definedFieldKeys = new Set(Object.keys(caseFileFields));

  if (!independentAssessmentDone) {
    return (
      <div className="max-w-3xl mx-auto p-4 sm:p-8">
        <div className="bg-white border border-blue-200 rounded-xl shadow-sm overflow-hidden">
          <div className="bg-blue-50 border-b border-blue-100 px-4 sm:px-6 py-4">
            <h2 className="text-base sm:text-lg font-semibold text-blue-900">Independent Assessment Required</h2>
            <p className="text-xs sm:text-sm text-blue-700 mt-1">This task requires structured oversight. Please review the source documents and record your initial assessment before viewing AI suggestions.</p>
          </div>
          <div className="p-4 sm:p-6 space-y-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Estimated Income Level</label>
              <select className="w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm">
                <option value="">Select...</option>
                <option value="below">Below Threshold</option>
                <option value="near">Near Threshold (Requires Review)</option>
                <option value="above">Above Threshold</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Initial Impression</label>
              <textarea rows={3} className="w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm" placeholder="Briefly note any immediate concerns or observations..."></textarea>
            </div>
            <button 
              onClick={() => setIndependentAssessmentDone(true)}
              className="w-full sm:w-auto bg-blue-600 text-white px-6 py-2.5 rounded-lg sm:rounded-md font-bold sm:font-medium hover:bg-blue-700 transition-colors shadow-lg shadow-blue-100 sm:shadow-none"
            >
              Submit Assessment & View Form
            </button>
          </div>
        </div>
      </div>
    );
  }

  const definedSections = Object.entries(caseFileFields).map(([key, fieldDef]) => {
    const stateValue = caseState[key];
    const isObject = fieldDef.type === 'object' && stateValue && typeof stateValue === 'object';

    return { key, fieldDef, stateValue, isObject };
  });

  const undefinedKeys = Object.keys(caseState).filter((k) => !definedFieldKeys.has(k));

  return (
    <div className="max-w-4xl mx-auto p-4 sm:p-8 pb-32">
      <div className="bg-white border border-gray-200 rounded-xl shadow-sm">
        <div className="px-4 sm:px-8 py-4 sm:py-6 border-b border-gray-200">
          <h1 className="text-xl sm:text-2xl font-semibold text-gray-900">{kernel?.title ?? 'Case Review'}</h1>
          <p className="text-xs sm:text-sm text-gray-500 mt-1">{kernel?.description ?? 'Review case details and record determination.'}</p>
        </div>

        <div className="p-4 sm:p-8 space-y-8 sm:space-y-10">
          {definedSections.length > 0 && (
            <section>
              <h3 className="text-base sm:text-lg font-medium text-gray-900 border-b border-gray-200 pb-2 mb-4 sm:mb-6">Case Fields</h3>
              <div className="space-y-6">
                {definedSections.map(({ key, fieldDef, stateValue, isObject }) => {
                  if (isObject) {
                    const obj = stateValue as Record<string, unknown>;
                    return (
                      <div key={key} className="space-y-4">
                        <h4 className="text-sm font-semibold text-gray-800">{fieldLabel(key)}</h4>
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4 sm:gap-y-6 pl-2 border-l-2 border-gray-100">
                          {Object.entries(obj).map(([subKey, subVal]) => {
                            const subStr = formatFieldValue(subVal);
                            if (typeof subVal === 'object' && subVal !== null) {
                              return (
                                <div key={subKey} className="col-span-2">
                                  <label className="block text-sm font-medium text-gray-700 mb-1">{fieldLabel(subKey)}</label>
                                  <div className="bg-gray-50 border border-gray-200 rounded-md p-3 text-sm font-mono text-gray-700 whitespace-pre-wrap">
                                    {subStr}
                                  </div>
                                </div>
                              );
                            }
                            return (
                              <AITouchedField
                                key={subKey}
                                id={`${key}.${subKey}`}
                                label={fieldLabel(subKey)}
                                value={subStr}
                                confidence="high"
                                verification="pass"
                                source="Case State"
                              />
                            );
                          })}
                        </div>
                      </div>
                    );
                  }

                  const strVal = formatFieldValue(stateValue);
                  return (
                    <AITouchedField
                      key={key}
                      id={key}
                      label={fieldLabel(key)}
                      value={strVal}
                      confidence="medium"
                      verification="pass"
                      source="Case State"
                    />
                  );
                })}
              </div>
            </section>
          )}

          {undefinedKeys.length > 0 && (
            <section>
              <h3 className="text-base sm:text-lg font-medium text-gray-900 border-b border-gray-200 pb-2 mb-4 sm:mb-6">Additional State</h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4 sm:gap-y-6">
                {undefinedKeys.map((key) => {
                  const value = caseState[key];
                  const strVal = formatFieldValue(value);
                  return (
                    <div key={key} className={typeof value === 'object' && value !== null ? 'col-span-2' : ''}>
                      <label className="block text-sm font-medium text-gray-700 mb-1">{fieldLabel(key)}</label>
                      <div className="bg-gray-50 border border-gray-200 rounded-md px-3 py-2 text-sm text-gray-700 read-only">
                        {strVal || <span className="text-gray-400 italic">Empty</span>}
                      </div>
                    </div>
                  );
                })}
              </div>
            </section>
          )}

          <section className="bg-amber-50 border border-amber-200 rounded-lg p-6">
            <h3 className="text-sm font-semibold text-amber-900 flex items-center gap-2 mb-2">
              <AlertTriangle className="w-4 h-4" />
              Consider the Opposite
            </h3>
            <p className="text-sm text-amber-800 mb-4">Before finalizing, consider: what evidence would support denying this application?</p>
            <textarea rows={2} className="w-full border-amber-300 rounded-md shadow-sm focus:ring-amber-500 focus:border-amber-500 bg-white" placeholder="Record your thoughts..."></textarea>
          </section>

          <section className="border-t border-gray-200 pt-6 sm:pt-8">
            <h3 className="text-base sm:text-lg font-medium text-gray-900 mb-4 sm:mb-6">Determination</h3>
            <div className="space-y-6">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">Decision</label>
                <div className="flex flex-col sm:flex-row gap-2 sm:gap-4">
                  <label className="flex items-center gap-2 p-3 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50 flex-1">
                    <input type="radio" name="decision" value="approve" onChange={(e) => setDetermination(e.target.value)} className="text-blue-600 focus:ring-blue-500" />
                    <span className="font-medium text-gray-900 text-sm sm:text-base">Approve</span>
                  </label>
                  <label className="flex items-center gap-2 p-3 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50 flex-1">
                    <input type="radio" name="decision" value="deny" onChange={(e) => setDetermination(e.target.value)} className="text-blue-600 focus:ring-blue-500" />
                    <span className="font-medium text-gray-900 text-sm sm:text-base">Deny</span>
                  </label>
                  <label className="flex items-center gap-2 p-3 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50 flex-1">
                    <input type="radio" name="decision" value="refer" onChange={(e) => setDetermination(e.target.value)} className="text-blue-600 focus:ring-blue-500" />
                    <span className="font-medium text-gray-900 text-sm sm:text-base">Refer</span>
                  </label>
                </div>
              </div>

              {determination === 'deny' && (
                <div className="bg-gray-50 border border-gray-200 rounded-lg p-4 sm:p-6 space-y-4 animate-in fade-in slide-in-from-top-2">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">Rationale for Denial</label>
                    <textarea rows={3} className="w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm" placeholder="Explain the specific criteria not met..."></textarea>
                  </div>
                  <div className="bg-white border border-gray-200 rounded p-3 sm:p-4">
                    <h4 className="text-[10px] sm:text-xs font-bold text-gray-500 uppercase tracking-wider mb-2">Applicant Letter Preview</h4>
                    <div className="text-xs sm:text-sm text-gray-800 font-serif leading-relaxed">
                      Dear {(() => { const app = caseState.application; if (app && typeof app === 'object') return String((app as Record<string, unknown>).applicantName ?? 'Applicant'); return 'Applicant'; })()},<br/><br/>
                      We have reviewed your application for {kernel?.title ?? 'benefits'}. Based on the information provided, a determination has been made regarding your eligibility.<br/><br/>
                      [Rationale will be inserted here]<br/><br/>
                      You have the right to appeal this decision within 30 days...
                    </div>
                  </div>
                </div>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
