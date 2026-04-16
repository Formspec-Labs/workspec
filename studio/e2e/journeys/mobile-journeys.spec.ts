import { test, expect } from '@playwright/test';
import { AdminPage } from '../pages/AdminPage';
import { AuditPage } from '../pages/AuditPage';

test.describe('Mobile Specific Journeys', () => {
  test.use({ viewport: { width: 390, height: 844 } }); // iPhone 13 dimensions

  test('Admin: Card-based Agent Registry on Mobile', async ({ page }) => {
    const admin = new AdminPage(page);
    await admin.goto();
    
    // Switch to Agents tab
    await admin.switchTab('Agents');
    
    // Check for card layout (table should be hidden)
    const cardLayout = page.locator('.grid.grid-cols-1.gap-4.lg\\:hidden');
    await expect(cardLayout).toBeVisible();
    
    // Verify at least one agent card is visible
    await expect(page.locator('.lg\\:hidden').getByText('Intake Classifier').first()).toBeVisible();
    await expect(page.locator('.lg\\:hidden').getByText('96%').first()).toBeVisible(); // Accuracy
  });

  test('Audit: Vertical Authority Chain on Mobile', async ({ page }) => {
    const audit = new AuditPage(page);
    await audit.goto();
    
    // Select a record
    await audit.selectRecord('CASE-2026-89A2');
    
    // Check for vertical stacking in authority chain
    // Use a more specific selector for the authority chain container
    const authorityChain = page.locator('section').filter({ hasText: /Authority Chain/i }).locator('.flex-col');
    await expect(authorityChain.first()).toBeVisible();
    
    // Verify chain elements
    await expect(page.getByText('Sarah Jenkins').first()).toBeVisible();
    await expect(page.getByText(/Director Sarah Jenkins/i).first()).toBeVisible();
  });
});
