import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

export class AdminPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Admin');
    await expect(this.page.getByText(/System Administration/i)).toBeVisible();
  }

  async switchTab(tabName: string) {
    await this.page.getByRole('button', { name: new RegExp(tabName, 'i') }).click();
  }

  async checkAIAgents() {
    await this.switchTab('Agents');
    await expect(this.page.getByText(/Registered AI Agents/i)).toBeVisible();
  }

  async checkDelegations() {
    await this.switchTab('Delegations');
    await expect(this.page.getByText(/Authority Delegations/i)).toBeVisible();
  }

  async checkHealth() {
    await this.switchTab('Health');
    await expect(this.page.getByText(/System Health/i)).toBeVisible();
  }
}
