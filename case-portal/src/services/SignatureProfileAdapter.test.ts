import { describe, it, expect } from 'vitest';
import { FixtureSignatureProfilePort } from './FixtureAdapter';
import type { WOSSignatureProfileDocument } from '../types/wos/signature-profile';

describe('FixtureSignatureProfilePort', () => {
  const port = new FixtureSignatureProfilePort();

  it('lists all loaded fixture profiles', async () => {
    const summaries = await port.list();
    expect(summaries.length).toBeGreaterThanOrEqual(9);
  });

  it('includes a sequential profile in the listing', async () => {
    const summaries = await port.list();
    const seq = summaries.find(s => s.id === 'signature-runtime-sequential');
    expect(seq).toBeDefined();
    expect(seq!.flowType).toBe('sequential');
    expect(seq!.roleCount).toBeGreaterThanOrEqual(1);
    expect(seq!.documentCount).toBeGreaterThanOrEqual(1);
  });

  it('includes a parallel profile in the listing', async () => {
    const summaries = await port.list();
    const par = summaries.find(s => s.id === 'signature-runtime-parallel');
    expect(par).toBeDefined();
    expect(par!.flowType).toBe('parallel');
    expect(par!.roleCount).toBeGreaterThanOrEqual(2);
    expect(par!.documentCount).toBeGreaterThanOrEqual(1);
  });

  it('loads the sequential profile and validates it as structurally sound', async () => {
    const profile = await port.load('signature-runtime-sequential');
    expect(profile).not.toBeNull();
    expect(profile!.$wosSignatureProfile).toBe('1.0');
    expect(profile!.signingFlow.type).toBe('sequential');
    expect(profile!.signingFlow.steps.length).toBeGreaterThanOrEqual(1);

    const result = await port.validate(profile!);
    expect(result.isValid).toBe(true);
    expect(result.issues.filter(i => i.severity === 'error')).toEqual([]);
  });

  it('loads the parallel profile and validates it as structurally sound', async () => {
    const profile = await port.load('signature-runtime-parallel');
    expect(profile).not.toBeNull();
    expect(profile!.$wosSignatureProfile).toBe('1.0');
    expect(profile!.signingFlow.type).toBe('parallel');
    expect(profile!.signingFlow.steps.length).toBeGreaterThanOrEqual(2);

    const result = await port.validate(profile!);
    expect(result.isValid).toBe(true);
    expect(result.issues.filter(i => i.severity === 'error')).toEqual([]);
  });

  it('sequential profile has all required policy sections', async () => {
    const profile = await port.load('signature-runtime-sequential');
    expect(profile).not.toBeNull();

    expect(profile!.declinePolicy).toBeDefined();
    expect(profile!.declinePolicy!.reasonRequired).toBe(true);

    expect(profile!.voidPolicy).toBeDefined();
    expect(profile!.voidPolicy!.reasonRequired).toBe(true);

    expect(profile!.reassignmentPolicy).toBeDefined();
    expect(profile!.reassignmentPolicy!.reasonRequired).toBe(true);

    expect(profile!.expiryPolicy).toBeDefined();
    expect(profile!.expiryPolicy!.after).toBeDefined();
  });

  it('sequential profile step roleIds and documentIds resolve', async () => {
    const profile = await port.load('signature-runtime-sequential');
    expect(profile).not.toBeNull();

    const roleIds = new Set(profile!.roles.map(r => r.id));
    const docIds = new Set(profile!.documents.map(d => d.id));

    for (const step of profile!.signingFlow.steps) {
      expect(roleIds.has(step.roleId)).toBe(true);
      expect(docIds.has(step.documentId)).toBe(true);
    }
  });

  it('parallel profile steps reference distinct roles', async () => {
    const profile = await port.load('signature-runtime-parallel');
    expect(profile).not.toBeNull();

    const roleIds = profile!.signingFlow.steps.map(s => s.roleId);
    const uniqueRoleIds = new Set(roleIds);
    expect(uniqueRoleIds.size).toBe(roleIds.length);
  });

  it('returns null for a nonexistent profile', async () => {
    const profile = await port.load('nonexistent-profile');
    expect(profile).toBeNull();
  });

  it('rejects a profile with missing targetWorkflow.url', async () => {
    const bad: unknown = {
      $wosSignatureProfile: '1.0',
      roles: [{ id: 'signer', role: 'signer', actorId: 'applicant' }],
      documents: [{ id: 'doc1', documentRef: 'urn:test:doc' }],
      signingFlow: { type: 'sequential', steps: [{ id: 's1', roleId: 'signer', documentId: 'doc1' }], completion: { type: 'all-required' } },
      evidence: { recordKind: 'signatureAffirmation', requiredFields: [] },
    };
    const result = await port.validate(bad as WOSSignatureProfileDocument);
    expect(result.isValid).toBe(false);
    expect(result.issues.some(i => i.message.includes('targetWorkflow.url'))).toBe(true);
  });

  it('rejects a profile with orphaned step roleId', async () => {
    const bad: unknown = {
      $wosSignatureProfile: '1.0',
      targetWorkflow: { url: 'urn:test:orphan' },
      roles: [{ id: 'signer', role: 'signer', actorId: 'applicant' }],
      documents: [{ id: 'doc1', documentRef: 'urn:test:doc' }],
      signingFlow: {
        type: 'sequential',
        steps: [{ id: 's1', roleId: 'nonexistent-role', documentId: 'doc1' }],
        completion: { type: 'all-required' },
      },
      evidence: { recordKind: 'signatureAffirmation', requiredFields: [] },
    };
    const result = await port.validate(bad as WOSSignatureProfileDocument);
    expect(result.isValid).toBe(false);
    expect(result.issues.some(i => i.category === 'policy' && i.targetId === 's1')).toBe(true);
  });

  it('save persists a valid profile and load retrieves it', async () => {
    const fresh = new FixtureSignatureProfilePort();
    const original = await fresh.load('signature-runtime-sequential');
    expect(original).not.toBeNull();

    const clone = structuredClone(original!);
    clone.title = 'Modified for save test';

    const saveResult = await fresh.save(clone);
    expect(saveResult.isValid).toBe(true);

    const loaded = await fresh.load(original!.targetWorkflow.url);
    expect(loaded).not.toBeNull();
    expect(loaded!.title).toBe('Modified for save test');
  });
});
