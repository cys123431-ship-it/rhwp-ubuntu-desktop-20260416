import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  screenshot,
  assert,
} from './helpers.mjs';

function uniqueRows(rects) {
  return [...new Set(rects.map((rect) => Math.round(rect.y)))].sort((a, b) => a - b);
}

runTest('Ctrl+A visual selection covers wrapped document end', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);

  const chunk = '\uAC00\uB098\uB2E4\uB77C\uB9D0\uB098\uC544\uC790\uB9AC\uB108\uC77C\uB108 or';
  await typeText(page, Array.from({ length: 18 }, () => chunk).join(' '));
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 800)));

  await page.keyboard.down('Control');
  await page.keyboard.press('a');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 500)));

  const state = await page.evaluate(() => {
    const selection = window.__inputHandler?.getSelection?.() ?? null;
    const cursorRect = window.__inputHandler?.cursor?.getRect?.() ?? null;
    const rects = selection
      ? window.__wasm?.getSelectionRects?.(
          selection.start.sectionIndex,
          selection.start.paragraphIndex,
          selection.start.charOffset,
          selection.end.paragraphIndex,
          selection.end.charOffset,
        ) ?? []
      : [];
    const domRects = Array.from(document.querySelectorAll('.selection-layer > div')).map((el) => {
      const style = window.getComputedStyle(el);
      return {
        left: parseFloat(style.left),
        top: parseFloat(style.top),
        width: parseFloat(style.width),
        height: parseFloat(style.height),
      };
    });
    return { selection, cursorRect, rects, domRects };
  });

  assert(Boolean(state.selection), 'Ctrl+A creates logical selection');
  assert(state.rects.length > 0, `selection rectangles created (${state.rects.length})`);

  const rows = uniqueRows(state.rects);
  assert(rows.length >= 6, `wrapped paragraph has multiple selected visual rows (${rows.length})`);

  const endLineSelected = state.rects.some((rect) => (
    rect.pageIndex === state.cursorRect?.pageIndex
    && Math.abs(rect.y - state.cursorRect.y) <= Math.max(3, rect.height)
  ));
  assert(
    endLineSelected,
    `visual selection includes document-end line (cursor=${JSON.stringify(state.cursorRect)}, rows=${rows.join(',')})`,
  );

  assert(
    state.domRects.length === state.rects.length,
    `DOM selection layer reflects all WASM rectangles (${state.domRects.length}/${state.rects.length})`,
  );

  await screenshot(page, 'selection-visual-ctrl-a');

  const canvas = await page.$('#scroll-container canvas');
  const box = await canvas?.boundingBox();
  if (!box) throw new Error('canvas bounding box is unavailable');

  await page.mouse.move(box.x + 115, box.y + 140);
  await page.mouse.down();
  await page.mouse.move(box.x + 400, box.y + 247, { steps: 1 });
  await page.mouse.up();
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 250)));

  const dragState = await page.evaluate(() => {
    const selection = window.__inputHandler?.getSelection?.() ?? null;
    const rects = selection
      ? window.__wasm?.getSelectionRects?.(
          selection.start.sectionIndex,
          selection.start.paragraphIndex,
          selection.start.charOffset,
          selection.end.paragraphIndex,
          selection.end.charOffset,
        ) ?? []
      : [];
    return { selection, rects };
  });
  const dragRows = uniqueRows(dragState.rects);
  assert(
    dragState.selection?.end?.charOffset - dragState.selection?.start?.charOffset > 100,
    `fast drag applies the mouseup endpoint (${JSON.stringify(dragState.selection)})`,
  );
  assert(dragRows.length >= 5, `fast drag reaches the final pointer row (${dragRows.length})`);
});
