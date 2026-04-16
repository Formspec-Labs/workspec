import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

/**
 * Page Object for the Caseworker Inbox.
 * Encapsulates behaviors related to triaging and managing tasks.
 */
export class InboxPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Inbox');
  }

  async filterBy(viewName: string) {
    await this.page.getByRole('button', { name: viewName }).click();
  }

  async peekTask(caseId: string) {
    // Look for the "Eye" icon (Quick Peek) in the row containing the caseId
    const taskRow = this.page.locator('[data-testid="task-item"]').filter({ hasText: caseId }).first();
    await taskRow.locator('button[title="Quick Peek"]').click();
  }

  async expectPeekDrawerVisible() {
    await expect(this.page.getByText('Quick Peek')).toBeVisible();
    await expect(this.page.getByText('AI Insights')).toBeVisible();
  }

  async selectTask(caseId: string) {
    const taskRow = this.page.locator('[data-testid="task-item"]').filter({ hasText: caseId }).first();
    await taskRow.locator('input[type="checkbox"]').check();
  }

  async expectBulkActionBarVisible() {
    await expect(this.page.getByText('Selected Tasks')).toBeVisible();
  }
}
