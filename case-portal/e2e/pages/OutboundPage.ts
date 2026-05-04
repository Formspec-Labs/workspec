import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

export class OutboundPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Outbound');
    await expect(this.page.getByText(/Outbound Management/i)).toBeVisible();
  }

  async selectNotification(recipient: string) {
    await this.page.getByText(recipient).first().click();
  }

  async previewNotification() {
    await this.page.getByRole('button', { name: /Preview Content/i }).first().click();
    await expect(this.page.getByText(/Notification Preview/i)).toBeVisible();
  }

  async resendNotification() {
    await this.page.getByRole('button', { name: /Resend/i }).click();
    await expect(this.page.getByText(/Notification Resent/i)).toBeVisible();
  }

  async checkAuditTrail() {
    await expect(this.page.getByText(/Notification Audit Trail/i)).toBeVisible();
  }
}
