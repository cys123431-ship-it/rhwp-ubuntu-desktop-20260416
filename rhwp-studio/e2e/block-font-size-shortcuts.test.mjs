import {
  runTest,
  createNewDocument,
  clickEditArea,
  typeText,
  screenshot,
  assert,
} from './helpers.mjs';

const wait = (page, ms = 250) => page.evaluate((delay) => new Promise((resolve) => setTimeout(resolve, delay)), ms);

async function selectAll(page) {
  await page.keyboard.down('Control');
  await page.keyboard.press('a');
  await page.keyboard.up('Control');
  await wait(page);
}

async function pressCombo(page, modifiers, key) {
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  await page.keyboard.press(key);
  for (const modifier of [...modifiers].reverse()) {
    await page.keyboard.up(modifier);
  }
  await wait(page);
}

async function getFontState(page) {
  return await page.evaluate(() => {
    const selection = window.__inputHandler?.getSelection?.() ?? null;
    const props = window.__wasm?.getCharPropertiesAt?.(0, 0, 0);
    const toolbarInput = document.getElementById('font-size');
    const toolbarValue = toolbarInput && 'value' in toolbarInput ? toolbarInput.value : '';
    return {
      fontSize: props?.fontSize ?? null,
      toolbarValue,
      selection,
    };
  });
}

function assertSelectedText(state, label) {
  assert(
    state.selection?.start?.charOffset === 0 && state.selection?.end?.charOffset === 3,
    `${label}: selected block remains 0..3 (${JSON.stringify(state.selection)})`,
  );
}

function assertFontSize(state, expected, label) {
  assert(state.fontSize === expected, `${label}: CharShape fontSize ${expected} (${state.fontSize})`);
  assert(
    Math.abs(parseFloat(state.toolbarValue) - expected / 100) < 0.01,
    `${label}: toolbar font size synced (${state.toolbarValue})`,
  );
  assertSelectedText(state, label);
}

runTest('block font size shortcuts', async ({ page }) => {
  await createNewDocument(page);
  await clickEditArea(page);
  await typeText(page, 'abc');
  await selectAll(page);

  assertSelectedText(await getFontState(page), 'Ctrl+A');

  await pressCombo(page, ['Alt', 'Shift'], 'KeyE');
  assertFontSize(await getFontState(page), 1100, 'Alt+Shift+E');

  await pressCombo(page, ['Alt', 'Shift'], 'KeyR');
  assertFontSize(await getFontState(page), 1000, 'Alt+Shift+R');

  await pressCombo(page, ['Control'], ']');
  assertFontSize(await getFontState(page), 1100, 'Ctrl+]');

  await pressCombo(page, ['Control'], '[');
  assertFontSize(await getFontState(page), 1000, 'Ctrl+[');

  await pressCombo(page, ['Control'], 'z');
  assertFontSize(await getFontState(page), 1100, 'Undo after Ctrl+[');

  await pressCombo(page, ['Control'], 'y');
  assertFontSize(await getFontState(page), 1000, 'Redo after Ctrl+[');

  await screenshot(page, 'block-font-size-shortcuts');
});
