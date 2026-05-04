import { Page, expect } from '@playwright/test';

/**
 * Utility for Service Design assertions.
 * Focuses on human-centric metrics like visibility, accessibility, and feedback.
 */
export const ServiceAssert = {
  /**
   * Ensures that a critical "Human Feedback" element is visible.
   * e.g., "Saved" indicators, "Live" status, etc.
   */
  async expectFeedbackVisible(page: Page, text: string) {
    await expect(page.getByText(text)).toBeVisible();
  },

  /**
   * Ensures that a persona-specific action is intuitive (visible and labeled).
   */
  async expectIntuitiveAction(page: Page, label: string) {
    const action = page.getByRole('button', { name: label });
    await expect(action).toBeVisible();
    await expect(action).toBeEnabled();
  },

  /**
   * Checks for "Service Transparency" elements.
   */
  async expectTransparency(page: Page) {
    // Check for provenance IDs or verified source labels
    await expect(page.getByText(/Provenance ID|Verified Source/i)).toBeVisible();
  }
};
