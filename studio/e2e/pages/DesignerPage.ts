import { Page, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

/**
 * Page Object for the Workflow Designer.
 * Encapsulates behaviors for administrative service design.
 */
export class DesignerPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/');
    await navigateMobile(this.page, 'Designer');
  }

  async searchStage(name: string) {
    const searchInput = this.page.getByPlaceholder('Find stage...');
    await searchInput.fill(name);
  }

  async expectStageInSearchResults(name: string) {
    // The search results are in a dropdown, we look for a button with the text
    await expect(this.page.locator('button').filter({ hasText: name }).first()).toBeVisible();
  }

  async toggleMiniMap() {
    await this.page.locator('button[title="Toggle Mini-map"]').click();
  }

  async expectMiniMapVisible(visible: boolean) {
    const miniMap = this.page.locator('[data-testid="mini-map"]');
    if (visible) {
      await expect(miniMap).toBeVisible();
    } else {
      await expect(miniMap).not.toBeVisible();
    }
  }

  /**
   * Behavior: Drag from one node's port to another node.
   * This tests the "Intuitive Connection" logic.
   */
  async connectStages(fromStageId: string, toStageId: string) {
    const fromNode = this.page.locator(`[layoutid="${fromStageId}"]`);
    const toNode = this.page.locator(`[layoutid="${toStageId}"]`);
    
    const port = fromNode.locator('[title="Drag to connect"]');
    
    await port.hover();
    await this.page.mouse.down();
    await toNode.hover();
    await this.page.mouse.up();
  }
}
