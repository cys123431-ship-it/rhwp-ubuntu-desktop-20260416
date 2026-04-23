import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  screenshot,
  assert,
  getPageCount,
} from './helpers.mjs';

runTest('편집 핵심 회귀', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, 'abc');

  await page.keyboard.down('Control');
  await page.keyboard.press('a');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));

  const selectAllState = await page.evaluate(() => window.__inputHandler?.getSelection?.() ?? null);
  assert(
    selectAllState?.start?.charOffset === 0 && selectAllState?.end?.charOffset === 3,
    `Ctrl+A 전체 선택 범위 확인 (${JSON.stringify(selectAllState)})`,
  );

  await page.evaluate(() => {
    const btn = document.getElementById('btn-size-up');
    btn?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
  });
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));

  const fontState = await page.evaluate(() => {
    const toolbarInput = document.getElementById('font-size');
    const toolbarValue = toolbarInput && 'value' in toolbarInput ? toolbarInput.value : '';
    const props = window.__wasm?.getCharPropertiesAt?.(0, 0, 0);
    const selection = window.__inputHandler?.getSelection?.() ?? null;
    return {
      toolbarValue,
      fontSize: props?.fontSize ?? null,
      selection,
    };
  });

  assert(
    Math.abs(parseFloat(fontState.toolbarValue) - 11.0) < 0.01,
    `툴바 글자 크기 값 동기화 (${fontState.toolbarValue})`,
  );
  assert(fontState.fontSize === 1100, `실제 글자 크기 적용 (${fontState.fontSize})`);
  assert(Boolean(fontState.selection), '글자 크기 적용 뒤 선택 유지');
  await screenshot(page, 'editing-core-01-font-size');

  await page.keyboard.press('End');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 200)));

  await page.keyboard.down('Control');
  await page.keyboard.press('Enter');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 400)));

  const pageBreakState = await page.evaluate(() => {
    const pos = window.__inputHandler?.getCursorPosition?.() ?? null;
    const rect = window.__inputHandler?.cursor?.getRect?.() ?? null;
    return {
      pageCount: window.__wasm?.pageCount ?? 0,
      paragraphIndex: pos?.paragraphIndex ?? null,
      charOffset: pos?.charOffset ?? null,
      pageIndex: rect?.pageIndex ?? null,
    };
  });

  assert(pageBreakState.pageCount === 2, `강제 쪽 나누기 후 페이지 수 증가 (${pageBreakState.pageCount})`);
  assert(
    pageBreakState.paragraphIndex === 1 && pageBreakState.charOffset === 0,
    `쪽 나누기 뒤 커서가 새 문단 시작으로 이동 (${JSON.stringify(pageBreakState)})`,
  );
  assert(pageBreakState.pageIndex === 1, `쪽 나누기 뒤 커서가 새 페이지에 위치 (${pageBreakState.pageIndex})`);

  await page.keyboard.down('Control');
  await page.keyboard.press('z');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));
  assert(await getPageCount(page) === 1, '쪽 나누기 Undo 동작');

  await page.keyboard.down('Control');
  await page.keyboard.press('y');
  await page.keyboard.up('Control');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));
  assert(await getPageCount(page) === 2, '쪽 나누기 Redo 동작');
  await screenshot(page, 'editing-core-02-page-break');

  const fileMenuState = await page.evaluate(() => {
    const fileTitle = document.querySelector('.menu-item[data-menu="file"] .menu-title');
    fileTitle?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));

    const enabled = (cmd) => {
      const el = document.querySelector(`.menu-item[data-menu="file"] .md-item[data-cmd="${cmd}"]`);
      return Boolean(el) && !el.classList.contains('disabled');
    };

    const exportSub = document.querySelector('.menu-item[data-menu="file"] .md-sub');
    return {
      isDesktop: Boolean(window.__RHWP_DESKTOP__),
      saveAs: enabled('file:save-as'),
      exportPdf: enabled('file:export-pdf'),
      exportDocx: enabled('file:export-docx'),
      exportJpg: enabled('file:export-jpg'),
      exportSubEnabled: exportSub ? !exportSub.classList.contains('disabled') : false,
    };
  });

  assert(fileMenuState.saveAs, '파일 메뉴 다른 이름으로 저장 활성화');
  assert(fileMenuState.exportSubEnabled, '파일 메뉴 다른 확장자로 저장 활성화');
  assert(
    fileMenuState.exportPdf === fileMenuState.isDesktop,
    `파일 메뉴 PDF 저장 활성화 상태 확인 (${JSON.stringify(fileMenuState)})`,
  );
  assert(fileMenuState.exportDocx, '파일 메뉴 DOCX 저장 활성화');
  assert(fileMenuState.exportJpg, '파일 메뉴 JPG 저장 활성화');
});
