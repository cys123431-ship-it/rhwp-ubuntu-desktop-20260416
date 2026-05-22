import {
  runTest,
  loadApp,
  createNewDocument,
  clickEditArea,
  typeText,
  assert,
} from './helpers.mjs';

async function triggerClose(page) {
  await page.evaluate(async () => {
    await window.__closeHandler?.({
      preventDefault() {
        window.__closePrevented = true;
      },
    });
  });
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 300)));
}

async function clickDialogButton(page, label) {
  return await page.evaluate((text) => {
    const buttons = Array.from(document.querySelectorAll('.modal-overlay .dialog-btn'));
    const button = buttons.find((candidate) => candidate.textContent?.trim() === text);
    button?.click();
    return Boolean(button);
  }, label);
}

runTest('desktop close asks to save dirty document', async ({ page }) => {
  await loadApp(page);

  await page.evaluate(() => {
    window.__closePrevented = false;
    window.__destroyCalled = false;
    window.__closeHandler = null;
    window.__TAURI__ = {
      window: {
        getCurrentWindow: () => ({
          onCloseRequested: async (handler) => {
            window.__closeHandler = handler;
            return () => {};
          },
          destroy: async () => {
            window.__destroyCalled = true;
          },
        }),
      },
    };
    window.__installDesktopCloseGuard?.();
  });

  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, '닫기 확인 테스트');

  await triggerClose(page);
  const dialog = await page.evaluate(() => window.__RHWP_E2E__?.getOpenDialog?.());
  assert(await page.evaluate(() => document.readyState === 'complete'), 'test page is alive after intercepted close');
  assert(await page.evaluate(() => window.__closePrevented === true), 'desktop close event is prevented');
  assert(dialog.visible && dialog.title === '문서 저장', 'dirty close shows save confirmation dialog');
  assert(dialog.message.includes('변경 내용을 저장할까요'), 'save confirmation explains dirty document');

  assert(await clickDialogButton(page, '취소'), 'cancel button is available');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 200)));
  assert(await page.evaluate(() => window.__destroyCalled === false), 'cancel keeps the desktop window open');

  await triggerClose(page);
  assert(await clickDialogButton(page, '저장 안 함'), 'discard button is available');
  await page.evaluate(() => new Promise((resolve) => setTimeout(resolve, 200)));
  assert(await page.evaluate(() => window.__destroyCalled === true), 'discard closes the desktop window');
}, { skipLoadApp: true });
