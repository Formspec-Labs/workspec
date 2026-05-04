import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

export class DashboardPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Dashboard');
    // Wait for the header text to be visible
    await expect(this.page.getByRole('heading', { name: /Operations Dashboard/i })).toBeVisible();
  }

  async getKPICard(label: string) {
    // Look for a div that contains the label and has a value (number or percentage)
    return this.page.locator('div').filter({ hasText: new RegExp(`^${label}$`, 'i') }).locator('xpath=..');
  }

  async checkHeatmapVisible() {
    await expect(this.page.getByText(/Workflow Heatmap/i)).toBeVisible();
  }

  async checkPipelineVisible() {
    await expect(this.page.getByText(/Redetermination Pipeline/i)).toBeVisible();
  }
}
