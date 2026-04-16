import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

export class ReportPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Reports');
    await expect(this.page.getByText(/Report Builder/i)).toBeVisible();
  }

  async selectTemplate(name: string) {
    await this.page.getByText(name).click();
  }

  async generateReport() {
    await this.page.getByRole('button', { name: /Generate Report/i }).click();
    // Wait for the report content to appear, which indicates loading is done
    // Use first() to avoid strict mode violation if the title appears in multiple places
    await expect(this.page.getByText(/Decision Drift Analysis/i).first().or(this.page.getByText(/Custom Report Results/i).first())).toBeVisible({ timeout: 10000 });
  }

  async pinToDashboard() {
    await this.page.getByRole('button', { name: /Pin to Dashboard/i }).click();
    await expect(this.page.getByText(/Pinned/i)).toBeVisible();
  }

  async openSchedule() {
    await this.page.getByRole('button', { name: /Schedule/i }).click();
    await expect(this.page.getByText(/Schedule Report/i)).toBeVisible();
  }
}
