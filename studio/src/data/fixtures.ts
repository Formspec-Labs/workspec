import kernelFixture from '../../../fixtures/kernel/benefits-adjudication.json';
import governanceFixture from '../../../fixtures/governance/benefits-adjudication-governance.json';
import aiFixture from '../../../fixtures/ai/benefits-adjudication-ai.json';
import policyParamsFixture from '../../../fixtures/governance/benefits-policy-parameters.json';
import notificationTemplateFixture from '../../../fixtures/sidecars/benefits-notification-templates.json';
import businessCalendarFixture from '../../../fixtures/sidecars/benefits-business-calendar.json';
import advancedFixture from '../../../fixtures/advanced/benefits-advanced-governance.json';
import equityFixture from '../../../fixtures/advanced/benefits-equity-config.json';
import semanticFixture from '../../../fixtures/profiles/semantic-benefits-adjudication.json';
import integrationFixture from '../../../fixtures/profiles/integration-benefits-adjudication.json';
import lifecycleDetailFixture from '../../../fixtures/companions/benefits-lifecycle-detail.json';
import driftMonitorFixture from '../../../fixtures/ai/benefits-drift-monitor.json';
import agentConfigFixture from '../../../fixtures/ai/document-extractor-config.json';
import purchaseOrderFixture from '../../../fixtures/kernel/purchase-order-approval.json';
import verificationReportFixture from '../../../fixtures/advanced/verification-report.json';
import correspondenceMetadataFixture from '../../../fixtures/kernel/benefits-correspondence-metadata.json';
import sigSequential from '../../../fixtures/profiles/signature-runtime-sequential.json';
import sigParallel from '../../../fixtures/profiles/signature-runtime-parallel.json';
import sigRouted from '../../../fixtures/profiles/signature-runtime-routed.json';
import sigFfa from '../../../fixtures/profiles/signature-runtime-free-for-all.json';
import sigNotary from '../../../fixtures/profiles/signature-runtime-notary.json';
import sigWitness from '../../../fixtures/profiles/signature-runtime-witness.json';
import sigBenefits from '../../../fixtures/profiles/signature-benefits-attestation.json';
import sigRoutedNotary from '../../../fixtures/profiles/signature-routed-notary.json';
import sigCounter from '../../../fixtures/profiles/signature-parallel-countersignature.json';

import type { WOSKernelDocument } from '../types/wos/kernel';
import type { WOSWorkflowGovernanceDocument } from '../types/wos/workflow-governance';
import type { WOSAIIntegrationDocument } from '../types/wos/ai-integration';
import type { WOSPolicyParameterConfig } from '../types/wos/policy-parameters';
import type { WOSNotificationTemplateConfig } from '../types/wos/notification-template';
import type { WOSBusinessCalendarConfig } from '../types/wos/business-calendar';
import type { WOSAdvancedGovernanceDocument } from '../types/wos/advanced';
import type { WOSEquityConfig } from '../types/wos/equity';
import type { WOSVerificationReport } from '../types/wos/verification-report';
import type { WOSCorrespondenceMetadataConfig } from '../types/wos/correspondence-metadata';
import type { WOSSemanticProfileDocument } from '../types/wos/semantic-profile';
import type { WOSIntegrationProfileDocument } from '../types/wos/integration-profile';
import type { WOSLifecycleDetailConfiguration } from '../types/wos/lifecycle-detail';
import type { WOSDriftMonitorConfig } from '../types/wos/drift-monitor';
import type { WOSAgentConfig } from '../types/wos/agent-config';
import type { WOSSignatureProfileDocument } from '../types/wos/signature-profile';
import type { WosDocumentBundle } from '../services/WosBackend';
import { validateAndCast } from '../services/schema-validator';

export type { WosDocumentBundle };

export function loadBenefitsAdjudicationBundle(): WosDocumentBundle {
  return {
    kernel: validateAndCast<WOSKernelDocument>(kernelFixture, 'WOSKernelDocument'),
    governance: validateAndCast<WOSWorkflowGovernanceDocument>(governanceFixture, 'WOSWorkflowGovernanceDocument'),
    ai: validateAndCast<WOSAIIntegrationDocument>(aiFixture, 'WOSAIIntegrationDocument'),
    policyParameters: validateAndCast<WOSPolicyParameterConfig>(policyParamsFixture, 'WOSPolicyParameterConfig'),
    notificationTemplates: validateAndCast<WOSNotificationTemplateConfig>(notificationTemplateFixture, 'WOSNotificationTemplateConfig'),
    businessCalendar: validateAndCast<WOSBusinessCalendarConfig>(businessCalendarFixture, 'WOSBusinessCalendarConfig'),
    advanced: validateAndCast<WOSAdvancedGovernanceDocument>(advancedFixture, 'WOSAdvancedGovernanceDocument'),
    equity: validateAndCast<WOSEquityConfig>(equityFixture, 'WOSEquityConfig'),
    driftMonitor: validateAndCast<WOSDriftMonitorConfig>(driftMonitorFixture, 'WOSDriftMonitorConfig'),
    agentConfigs: [validateAndCast<WOSAgentConfig>(agentConfigFixture, 'WOSAgentConfig')],
    verificationReport: validateAndCast<WOSVerificationReport>(verificationReportFixture, 'WOSVerificationReport'),
    correspondenceMetadata: validateAndCast<WOSCorrespondenceMetadataConfig>(correspondenceMetadataFixture, 'WOSCorrespondenceMetadataConfig'),
    semanticProfile: validateAndCast<WOSSemanticProfileDocument>(semanticFixture, 'WOSSemanticProfileDocument'),
    integrationProfile: validateAndCast<WOSIntegrationProfileDocument>(integrationFixture, 'WOSIntegrationProfileDocument'),
    lifecycleDetail: validateAndCast<WOSLifecycleDetailConfiguration>(lifecycleDetailFixture, 'WOSLifecycleDetailConfiguration'),
  };
}

export function loadPurchaseOrderBundle(): WosDocumentBundle {
  return {
    kernel: validateAndCast<WOSKernelDocument>(purchaseOrderFixture, 'WOSKernelDocument'),
  };
}

export function loadSignatureProfiles(): Map<string, WOSSignatureProfileDocument> {
  const entries: [string, unknown][] = [
    ['signature-runtime-sequential', sigSequential],
    ['signature-runtime-parallel', sigParallel],
    ['signature-runtime-routed', sigRouted],
    ['signature-runtime-free-for-all', sigFfa],
    ['signature-runtime-notary', sigNotary],
    ['signature-runtime-witness', sigWitness],
    ['signature-benefits-attestation', sigBenefits],
    ['signature-routed-notary', sigRoutedNotary],
    ['signature-parallel-countersignature', sigCounter],
  ];
  const map = new Map<string, WOSSignatureProfileDocument>();
  for (const [id, raw] of entries) {
    map.set(id, validateAndCast<WOSSignatureProfileDocument>(raw, 'WOSSignatureProfileDocument'));
  }
  return map;
}
