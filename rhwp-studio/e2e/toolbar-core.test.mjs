import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  screenshot,
  assert,
  getParaText,
} from './helpers.mjs';

async function clickToolbarCommand(page, commandId) {
  const clicked = await page.evaluate((cmd) => {
    const btn = document.querySelector(`#icon-toolbar [data-cmd="${cmd}"]`);
    if (!btn) return false;
    btn.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
    return true;
  }, commandId);
  assert(clicked, `toolbar command button exists: ${commandId}`);
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));
}

async function selectRange(page, startOffset, endOffset) {
  await page.evaluate(({ startOffset: start, endOffset: end }) => {
    const ih = window.__inputHandler;
    if (!ih?.cursor) return;
    const startPos = { sectionIndex: 0, paragraphIndex: 0, charOffset: start };
    const endPos = { sectionIndex: 0, paragraphIndex: 0, charOffset: end };
    ih.cursor.setSelection(startPos, endPos);
    ih.updateCaret?.();
  }, { startOffset, endOffset });
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 150)));
}

runTest('toolbar core commands', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, 'abcdefg');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 400)));

  await page.keyboard.down('Control');
  await page.keyboard.press('a');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 200)));

  await clickToolbarCommand(page, 'edit:copy');
  await page.keyboard.press('End');
  await clickToolbarCommand(page, 'edit:paste');

  const pasted = await getParaText(page, 0, 0, 64);
  assert(pasted.includes('abcdefgabcdefg'), `toolbar copy/paste duplicates text (${pasted})`);

  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, 'source target');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 400)));

  await page.evaluate(() => {
    const ih = window.__inputHandler;
    const start = { sectionIndex: 0, paragraphIndex: 0, charOffset: 0 };
    const end = { sectionIndex: 0, paragraphIndex: 0, charOffset: 6 };
    ih?.applyCharPropsToRange?.(start, end, { fontSize: 1400, bold: true });
  });
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));

  await page.evaluate(() => {
    window.__inputHandler?.moveCursorTo?.({ sectionIndex: 0, paragraphIndex: 0, charOffset: 1 });
  });
  await clickToolbarCommand(page, 'edit:format-copy');
  await selectRange(page, 7, 13);
  await clickToolbarCommand(page, 'edit:format-copy');

  const formatState = await page.evaluate(() => {
    const source = window.__wasm?.getCharPropertiesAt?.(0, 0, 0);
    const target = window.__wasm?.getCharPropertiesAt?.(0, 0, 7);
    return {
      sourceFontSize: source?.fontSize ?? null,
      targetFontSize: target?.fontSize ?? null,
      sourceBold: Boolean(source?.bold),
      targetBold: Boolean(target?.bold),
    };
  });

  assert(formatState.sourceFontSize === 1400, `source format prepared (${JSON.stringify(formatState)})`);
  assert(formatState.targetFontSize === 1400, `format copy applies font size (${JSON.stringify(formatState)})`);
  assert(formatState.targetBold, `format copy applies bold (${JSON.stringify(formatState)})`);

  await screenshot(page, 'toolbar-core-final');
});
