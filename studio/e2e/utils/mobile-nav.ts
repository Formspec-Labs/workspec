import { Page } from '@playwright/test';

export async function navigateMobile(page: Page, label: string) {
  const menuButton = page.getByLabel('Toggle mobile menu');
  
  // Check if we are in mobile view by looking at the menu button visibility
  if (await menuButton.isVisible()) {
    await menuButton.click();
    // Wait for the mobile menu to be visible
    const navItem = page.locator('nav, div[role="dialog"], div.fixed').getByRole('button', { name: label, exact: true });
    await navItem.waitFor({ state: 'visible' });
    await navItem.click();
  } else {
    // Desktop navigation
    await page.getByRole('button', { name: label, exact: true }).click();
  }
}
