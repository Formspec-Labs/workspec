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
import type { WosDocumentBundle } from '../services/WosBackend';

export type { WosDocumentBundle };

export function loadBenefitsAdjudicationBundle(): WosDocumentBundle {
  return {
    kernel: kernelFixture as unknown as WOSKernelDocument,
    governance: governanceFixture as unknown as WOSWorkflowGovernanceDocument,
    ai: aiFixture as unknown as WOSAIIntegrationDocument,
    policyParameters: policyParamsFixture as unknown as WOSPolicyParameterConfig,
    notificationTemplates: notificationTemplateFixture as unknown as WOSNotificationTemplateConfig,
    businessCalendar: businessCalendarFixture as unknown as WOSBusinessCalendarConfig,
    advanced: advancedFixture as unknown as WOSAdvancedGovernanceDocument,
    equity: equityFixture as unknown as WOSEquityConfig,
    driftMonitor: driftMonitorFixture as unknown as WOSDriftMonitorConfig,
    agentConfigs: [agentConfigFixture as unknown as WOSAgentConfig],
    verificationReport: verificationReportFixture as unknown as WOSVerificationReport,
    correspondenceMetadata: correspondenceMetadataFixture as unknown as WOSCorrespondenceMetadataConfig,
    semanticProfile: semanticFixture as unknown as WOSSemanticProfileDocument,
    integrationProfile: integrationFixture as unknown as WOSIntegrationProfileDocument,
    lifecycleDetail: lifecycleDetailFixture as unknown as WOSLifecycleDetailConfiguration,
  };
}

export function loadPurchaseOrderBundle(): WosDocumentBundle {
  return {
    kernel: purchaseOrderFixture as unknown as WOSKernelDocument,
  };
}
