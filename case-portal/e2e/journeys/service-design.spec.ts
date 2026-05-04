import { test, expect } from '@playwright/test';
import { InboxPage } from '../pages/InboxPage';
import { DesignerPage } from '../pages/DesignerPage';

/**
 * Service Design Journey: Caseworker Efficiency
 * Scenario: A caseworker needs to quickly triage high-priority cases.
 */
test.describe('Caseworker Journey: Efficient Triage', () => {
  test('should allow caseworker to peek at high priority tasks without leaving the inbox', async ({ page }) => {
    const inbox = new InboxPage(page);
    
    await inbox.goto();
    
    // User Behavior: Filter to see only high priority items
    await inbox.filterBy('High Priority');
    
    // User Behavior: Peek at a specific case to see AI insights
    // Using a known case ID from mock data or first row
    await inbox.peekTask('urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4');
    
    // Outcome: Insights are visible instantly
    await inbox.expectPeekDrawerVisible();
  });

  test('should show bulk actions when multiple tasks are selected', async ({ page }) => {
    const inbox = new InboxPage(page);
    await inbox.goto();
    
    await inbox.selectTask('urn:wos:instance:benefits-adj:2026-04-09:a1b2c3d4');
    await inbox.selectTask('urn:wos:instance:benefits-adj:2026-04-07:e5f6g7h8');
    
    await inbox.expectBulkActionBarVisible();
  });
});

/**
 * Service Design Journey: Administrative Agility
 * Scenario: An admin needs to modify the workflow to handle new regulatory requirements.
 */
test.describe('Admin Journey: Workflow Evolution', () => {
  test('should allow admin to find and navigate to specific stages easily', async ({ page }) => {
    const designer = new DesignerPage(page);
    
    // Navigate to designer using POM
    await designer.goto(); 
    
    // User Behavior: Search for a specific stage
    await designer.searchStage('Income');
    
    // Outcome: Stage is found
    await designer.expectStageInSearchResults('Income Verification');
  });

  test('should allow admin to toggle the mini-map for better spatial awareness', async ({ page }) => {
    const designer = new DesignerPage(page);
    await designer.goto();
    
    // Initial state: Mini-map is visible
    await designer.expectMiniMapVisible(true);
    
    // User Behavior: Toggle mini-map off to clear space
    await designer.toggleMiniMap();
    
    // Outcome: Mini-map is hidden
    await designer.expectMiniMapVisible(false);
  });
});
