import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  assert,
} from './helpers.mjs';

async function getClientPointForOffset(page, offset) {
  return await page.evaluate((charOffset) => {
    const wasm = window.__wasm;
    const canvasView = window.__canvasView;
    const container = document.querySelector('#scroll-container');
    if (!wasm || !canvasView || !container) return null;

    const cursorRect = wasm.getCursorRect(0, 0, charOffset);
    const virtualScroll = canvasView.getVirtualScroll();
    const viewport = canvasView.getViewportManager();
    const zoom = viewport.getZoom();
    const containerRect = container.getBoundingClientRect();
    const contentX = virtualScroll.getPageLeft(cursorRect.pageIndex) + cursorRect.x * zoom;
    const contentY = virtualScroll.getPageOffset(cursorRect.pageIndex)
      + (cursorRect.y + cursorRect.height / 2) * zoom;

    return {
      x: containerRect.left + contentX - container.scrollLeft,
      y: containerRect.top + contentY - container.scrollTop,
    };
  }, offset);
}

async function getSelection(page) {
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));
  return await page.evaluate(() => window.__inputHandler?.getSelection?.() ?? null);
}

async function dragBetweenOffsets(page, fromOffset, toOffset) {
  const from = await getClientPointForOffset(page, fromOffset);
  const to = await getClientPointForOffset(page, toOffset);
  assert(Boolean(from && to), `offset ${fromOffset} -> ${toOffset} client points are available`);

  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(to.x, to.y, { steps: 32 });
  await page.mouse.up();
  return await getSelection(page);
}

runTest('mouse drag selection works in both vertical directions', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);

  const chunk = '가나다라마바사아자차카타파하 한글 블록 선택 검증 ';
  const text = Array.from({ length: 16 }, () => chunk).join('');
  await typeText(page, text);

  const length = await page.evaluate(() => window.__wasm?.getParagraphLength?.(0, 0) ?? 0);
  const startOffset = 0;
  const endOffset = Math.min(length - 1, 210);
  assert(endOffset > 120, `test paragraph is long enough (${length} chars)`);

  const downSelection = await dragBetweenOffsets(page, startOffset, endOffset);
  assert(Boolean(downSelection), 'top-to-bottom drag creates a logical selection');
  assert(
    downSelection.start.charOffset <= startOffset + 2
      && downSelection.end.charOffset >= endOffset - 2,
    `top-to-bottom selection covers expected offsets (${downSelection.start.charOffset}-${downSelection.end.charOffset})`,
  );

  await page.mouse.click((await getClientPointForOffset(page, startOffset)).x, (await getClientPointForOffset(page, startOffset)).y);
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 200)));

  const upSelection = await dragBetweenOffsets(page, endOffset, startOffset);
  assert(Boolean(upSelection), 'bottom-to-top drag creates a logical selection');
  assert(
    upSelection.start.charOffset <= startOffset + 2
      && upSelection.end.charOffset >= endOffset - 2,
    `bottom-to-top selection covers expected offsets (${upSelection.start.charOffset}-${upSelection.end.charOffset})`,
  );
});
