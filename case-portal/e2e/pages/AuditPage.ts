import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

export class AuditPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Audit');
    await expect(this.page.getByText(/Provenance Explorer/i)).toBeVisible();
  }

  async searchRecord(query: string) {
    await this.page.getByPlaceholder(/Search actor, event, or ID/i).fill(query);
  }

  async selectRecord(id: string) {
    const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    await this.page.getByRole('button', { name: new RegExp(escaped) }).first().click();
  }

  async verifyIntegrity() {
    await this.page.getByRole('button', { name: /Verify/i }).click();
    await expect(this.page.getByText(/Cryptographic Integrity Verified/i)).toBeVisible();
  }

  async exportAudit() {
    await this.page.getByRole('button', { name: /Export/i }).click();
    // In a real test we might check for download, but here we just check button exists
  }
}
