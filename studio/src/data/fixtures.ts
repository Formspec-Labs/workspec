import kernelFixture from '../../../fixtures/kernel/benefits-adjudication.json';
import notificationTemplateFixture from '../../../fixtures/sidecars/benefits-notification-templates.json';
import businessCalendarFixture from '../../../fixtures/sidecars/benefits-business-calendar.json';
import correspondenceMetadataFixture from '../../../fixtures/kernel/benefits-correspondence-metadata.json';
import semanticFixture from '../../../fixtures/profiles/semantic-benefits-adjudication.json';
import integrationFixture from '../../../fixtures/profiles/integration-benefits-adjudication.json';
import purchaseOrderFixture from '../../../fixtures/kernel/purchase-order-approval.json';
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
import type { WOSNotificationTemplateConfig } from '../types/wos/notification-template';
import type { WOSBusinessCalendarConfig } from '../types/wos/business-calendar';
import type { WOSCorrespondenceMetadataConfig } from '../types/wos/correspondence-metadata';
import type { WOSSemanticProfileDocument } from '../types/wos/semantic-profile';
import type { WOSIntegrationProfileDocument } from '../types/wos/integration-profile';
import type { WOSSignatureProfileDocument } from '../types/wos/signature-profile';
import type { WosDocumentBundle } from '../services/WosBackend';
import { validateAndCast } from '../services/schema-validator';

export type { WosDocumentBundle };

export function loadBenefitsAdjudicationBundle(): WosDocumentBundle {
  return {
    workflow: validateAndCast<WOSKernelDocument>(kernelFixture, 'WOSKernelDocument'),
    delivery: {
      notificationTemplates: validateAndCast<WOSNotificationTemplateConfig>(notificationTemplateFixture, 'WOSNotificationTemplateConfig'),
      businessCalendar: validateAndCast<WOSBusinessCalendarConfig>(businessCalendarFixture, 'WOSBusinessCalendarConfig'),
      correspondenceMetadata: validateAndCast<WOSCorrespondenceMetadataConfig>(correspondenceMetadataFixture, 'WOSCorrespondenceMetadataConfig'),
    },
    ontologyAlignment: {
      semanticProfile: validateAndCast<WOSSemanticProfileDocument>(semanticFixture, 'WOSSemanticProfileDocument'),
      integrationProfile: validateAndCast<WOSIntegrationProfileDocument>(integrationFixture, 'WOSIntegrationProfileDocument'),
    },
  };
}

export function loadPurchaseOrderBundle(): WosDocumentBundle {
  return {
    workflow: validateAndCast<WOSKernelDocument>(purchaseOrderFixture, 'WOSKernelDocument'),
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
    const normalized =
      raw && typeof raw === 'object' && !Array.isArray(raw) && 'signature' in raw
        ? (raw as Record<string, unknown>).signature
        : raw;
    map.set(id, validateAndCast<WOSSignatureProfileDocument>(normalized, 'WOSSignatureProfileDocument'));
  }
  return map;
}
