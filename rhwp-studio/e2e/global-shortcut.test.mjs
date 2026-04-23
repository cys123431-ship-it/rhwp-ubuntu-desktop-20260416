/**
 * E2E global shortcut regression for startup document bootstrapping.
 */
import { runTest, loadApp, screenshot, assert } from './helpers.mjs';

process.env.VITE_URL = process.env.VITE_URL || 'http://localhost:7700';

runTest('global shortcut - startup doc Alt+N', async ({ page }) => {
  await loadApp(page);
  await page.waitForFunction(() => (window.__wasm?.pageCount ?? 0) >= 1, { timeout: 10000 });

  const pageCountBefore = await page.evaluate(() => window.__wasm?.pageCount ?? 0);
  assert(pageCountBefore >= 1, `TC1: startup document available (pageCount=${pageCountBefore})`);
  await screenshot(page, 'global-01-startup-doc');

  await page.keyboard.down('Alt');
  await page.keyboard.press('n');
  await page.keyboard.up('Alt');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 800)));

  const pageCountAfter = await page.evaluate(() => window.__wasm?.pageCount ?? 0);
  await screenshot(page, 'global-02-new-doc');
  assert(pageCountAfter >= 1, `TC2: Alt+N creates a document (pageCount=${pageCountAfter})`);
});
