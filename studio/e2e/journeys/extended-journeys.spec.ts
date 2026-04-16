import { test, expect } from '@playwright/test';
import { DashboardPage } from '../pages/DashboardPage';
import { OutboundPage } from '../pages/OutboundPage';
import { AdminPage } from '../pages/AdminPage';
import { AuditPage } from '../pages/AuditPage';
import { ReportPage } from '../pages/ReportPage';

test.describe('Extended Service Journeys', () => {
  
  test('Dashboard: The Morning Briefing & Bottleneck Hunt', async ({ page }) => {
    const dashboard = new DashboardPage(page);
    await dashboard.goto();

    // Morning Briefing: Check KPIs
    const throughput = await dashboard.getKPICard('SLA Compliance');
    await expect(throughput).toBeVisible();
    await dashboard.checkHeatmapVisible();

    // Bottleneck Hunt: Check Pipeline
    await dashboard.checkPipelineVisible();
  });

  test('Outbound: Letter Preview & Delivery Receipt', async ({ page }) => {
    const outbound = new OutboundPage(page);
    await outbound.goto();

    // Select a notification
    await outbound.selectNotification('John Doe');

    // Letter Preview
    await outbound.previewNotification();
    await page.keyboard.press('Escape'); // Close preview if it's a modal

    // Delivery Receipt / Audit Trail
    await outbound.checkAuditTrail();
  });

  test('Admin: Robot Training & System Health', async ({ page }) => {
    const admin = new AdminPage(page);
    await admin.goto();

    // Robot Training (AI Agents)
    await admin.checkAIAgents();

    // System Health
    await admin.checkHealth();
  });

  test('Audit: Forensic Search & Integrity Verification', async ({ page }) => {
    const audit = new AuditPage(page);
    await audit.goto();

    // Forensic Search
    await audit.searchRecord('CASE-2026-12C5');
    await audit.selectRecord('CASE-2026-12C5');

    // Integrity Verification
    await audit.verifyIntegrity();
  });

  test('Reports: Live Sketch & Dashboard Pinning', async ({ page }) => {
    const reports = new ReportPage(page);
    await reports.goto();

    // Select Template
    await reports.selectTemplate('Decision Drift Analysis');

    // Generate Report
    await reports.generateReport();

    // Dashboard Pinning
    await reports.pinToDashboard();
  });

});
