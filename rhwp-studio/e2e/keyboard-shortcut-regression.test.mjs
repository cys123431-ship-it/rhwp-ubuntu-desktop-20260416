import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  getParaText,
  assert,
} from './helpers.mjs';

const wait = (page, ms = 250) => page.evaluate(
  (delay) => new Promise((resolve) => setTimeout(resolve, delay)),
  ms,
);

runTest('keyboard input and global shortcuts', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, 'pass S class');

  const text = await getParaText(page, 0, 0);
  assert(text === 'pass S class', `plain s/S input is preserved (${JSON.stringify(text)})`);

  await page.evaluate(() => {
    const target = document.querySelector('[data-cmd="format:bold"]');
    if (target instanceof HTMLElement) {
      target.tabIndex = 0;
      target.focus();
    }
  });
  await page.keyboard.down('Control');
  await page.keyboard.press('a');
  await page.keyboard.up('Control');
  await wait(page);

  const selection = await page.evaluate(() => window.__inputHandler?.getSelection?.() ?? null);
  assert(
    selection?.start?.charOffset === 0 && selection?.end?.charOffset === 12,
    `Ctrl+A works after toolbar focus (${JSON.stringify(selection)})`,
  );
});
