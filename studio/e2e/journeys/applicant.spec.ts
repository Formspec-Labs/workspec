import { test, expect } from '@playwright/test';
import { navigateMobile } from '../utils/mobile-nav';

/**
 * Service Design Journey: Applicant Transparency
 * Scenario: An applicant wants to understand the evidence used for their determination.
 */
test.describe('Applicant Journey: Transparency & Trust', () => {
  test('should allow applicant to view the source document for a piece of evidence', async ({ page }) => {
    // Navigate to applicant portal
    await page.goto('/');
    await navigateMobile(page, 'Applicant Portal');
    
    // User Behavior: Click on a piece of evidence
    const evidenceItem = page.getByText('Tax Return 2025').first();
    await evidenceItem.click();
    
    // Outcome: Evidence viewer modal opens with official record details
    await expect(page.getByText('Official Record • Verified Source')).toBeVisible();
    await expect(page.getByText('System Highlight')).toBeVisible();
    
    // User Behavior: Close the viewer
    await page.getByLabel('Close').click();
    
    // Outcome: Modal is closed
    await expect(page.getByText('Official Record • Verified Source')).not.toBeVisible();
  });
});
