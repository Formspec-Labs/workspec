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
    
    const cardLayout = page.getByTestId('admin-agent-registry-mobile');
    await expect(cardLayout).toBeVisible();

    await expect(cardLayout.getByText('documentExtractor').first()).toBeVisible();
    await expect(cardLayout.getByText('eligibilityScreener').first()).toBeVisible();
  });

  test('Audit: Vertical Authority Chain on Mobile', async ({ page }) => {
    const audit = new AuditPage(page);
    await audit.goto();
    
    // Select a record
    await audit.selectRecord('urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4');
    
    // Check for vertical stacking in authority chain
    // Use a more specific selector for the authority chain container
    const authorityChain = page.locator('section').filter({ hasText: /Authority Chain/i }).locator('.flex-col');
    await expect(authorityChain.first()).toBeVisible();
    
    await expect(page.getByText('Sarah Jenkins').first()).toBeVisible();
    await expect(page.getByText(/Delegated by Director M\. Smith/i).first()).toBeVisible();
  });
});
