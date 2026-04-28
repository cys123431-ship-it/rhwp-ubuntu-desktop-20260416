/**
 * hwpctl 호환 레이어 E2E 테스트 - 기본 동작
 */
import { runTest, screenshot, assert } from './helpers.mjs';

const VITE_URL = process.env.VITE_URL || 'http://localhost:7700';

runTest('hwpctl 호환 레이어 기본 동작', async ({ page }) => {
  console.log('  [1] 테스트 페이지 로드...');
  await page.goto(`${VITE_URL}/hwpctl-test.html`, { waitUntil: 'networkidle2', timeout: 30000 });
  await new Promise(r => setTimeout(r, 3000));

  console.log('  [2] HwpCtrl 초기화 확인...');
  assert(await page.evaluate(() => !!window.HwpCtrl), 'HwpCtrl 객체가 전역에 존재해야 함');

  console.log('  [3] Action 등록 확인...');
  const actionCount = await page.evaluate(() => window.HwpCtrl.constructor.getRegisteredActionCount());
  assert(actionCount >= 30, `등록 Action 30개 이상 (실제: ${actionCount})`);
  console.log(`     등록 Action: ${actionCount}개`);

  console.log('  [4] CreateAction 동작 확인...');
  const actionInfo = await page.evaluate(() => {
    const action = window.HwpCtrl.CreateAction("TableCreate");
    return {
      actId: action.ActID,
      status: action.GetCompatibilityStatus(),
    };
  });
  const unknownActionStatus = await page.evaluate(() =>
    window.HwpCtrl.CreateAction("DefinitelyUnknownAction").GetCompatibilityStatus(),
  );
  assert(actionInfo.status === 'partial', `TableCreate status = "partial" (실제: "${actionInfo.status}")`);
  assert(unknownActionStatus === 'unsupported', `미등록 Action status = "unsupported" (실제: "${unknownActionStatus}")`);
  assert(actionInfo.actId === 'TableCreate', `ActID = "TableCreate" (실제: "${actionInfo.actId}")`);

  console.log('  [5] ParameterSet 동작 확인...');
  const setResult = await page.evaluate(() => {
    const set = window.HwpCtrl.CreateSet("TableCreation");
    set.SetItem("Rows", 10);
    set.SetItem("Cols", 6);
    return { rows: set.GetItem("Rows"), cols: set.GetItem("Cols"), name: set.name };
  });
  assert(setResult.rows === 10, `Rows = 10 (실제: ${setResult.rows})`);
  assert(setResult.cols === 6, `Cols = 6 (실제: ${setResult.cols})`);
  assert(setResult.name === 'TableCreation', `Set name = "TableCreation"`);

  console.log('  [6] InsertText 동작 확인...');
  const textResult = await page.evaluate(() => {
    window.HwpCtrl.Clear();
    const ok = window.HwpCtrl.InsertText("테스트 문장");
    const pos = window.HwpCtrl.GetPos();
    return { ok, pos };
  });
  assert(textResult.ok === true, 'InsertText 성공');
  assert(textResult.pos.pos > 0, `커서 이동 (pos=${textResult.pos.pos})`);

  console.log('  [6-1] Undo/Redo 스냅샷 동작 확인...');
  const undoRedoInfo = await page.evaluate(() => {
    window.HwpCtrl.Clear();
    window.HwpCtrl.InsertText("ABC");
    const lenAfterInsert = window.HwpCtrl.getWasmDoc().getParagraphLength(0, 0);
    const undoOk = window.HwpCtrl.Run("Undo");
    const lenAfterUndo = window.HwpCtrl.getWasmDoc().getParagraphLength(0, 0);
    const redoOk = window.HwpCtrl.Run("Redo");
    const lenAfterRedo = window.HwpCtrl.getWasmDoc().getParagraphLength(0, 0);
    return { lenAfterInsert, undoOk, lenAfterUndo, redoOk, lenAfterRedo };
  });
  assert(undoRedoInfo.undoOk, 'Undo Action이 성공해야 함');
  assert(undoRedoInfo.redoOk, 'Redo Action이 성공해야 함');
  assert(undoRedoInfo.lenAfterInsert >= 3, `삽입 후 길이 >= 3 (실제: ${undoRedoInfo.lenAfterInsert})`);
  assert(undoRedoInfo.lenAfterUndo === 0, `Undo 후 빈 문단이어야 함 (실제: ${undoRedoInfo.lenAfterUndo})`);
  assert(undoRedoInfo.lenAfterRedo >= 3, `Redo 후 길이 복원 (실제: ${undoRedoInfo.lenAfterRedo})`);

  console.log('  [6-2] 구역/다단/쪽 번호 Action 동작 확인...');
  const pageActionInfo = await page.evaluate(() => {
    window.HwpCtrl.Clear();
    window.HwpCtrl.InsertText("구역 테스트");
    const beforeSections = window.HwpCtrl.getWasmDoc().getSectionCount();
    const breakSectionOk = window.HwpCtrl.Run("BreakSection");
    const afterSections = window.HwpCtrl.getWasmDoc().getSectionCount();

    const colSet = window.HwpCtrl.CreateSet("ColDef");
    colSet.SetItem("ColCount", 2);
    const colOk = window.HwpCtrl.CreateAction("BreakColDef").Execute(colSet);

    const pnSet = window.HwpCtrl.CreateSet("PageNumPos");
    pnSet.SetItem("Position", 5);
    pnSet.SetItem("Format", 0);
    const pageNumOk = window.HwpCtrl.CreateAction("PageNumPos").Execute(pnSet);

    return {
      beforeSections,
      afterSections,
      breakSectionOk,
      colOk,
      pageNumOk,
      breakSectionStatus: window.HwpCtrl.CreateAction("BreakSection").GetCompatibilityStatus(),
      pageNumStatus: window.HwpCtrl.CreateAction("PageNumPos").GetCompatibilityStatus(),
    };
  });
  assert(pageActionInfo.breakSectionOk, 'BreakSection Action이 성공해야 함');
  assert(
    pageActionInfo.afterSections === pageActionInfo.beforeSections + 1,
    `구역 수가 1 증가해야 함 (${pageActionInfo.beforeSections} -> ${pageActionInfo.afterSections})`,
  );
  assert(pageActionInfo.colOk, 'BreakColDef Action이 성공해야 함');
  assert(pageActionInfo.pageNumOk, 'PageNumPos Action이 성공해야 함');
  assert(pageActionInfo.breakSectionStatus === 'implemented', `BreakSection status = implemented (실제: ${pageActionInfo.breakSectionStatus})`);
  assert(pageActionInfo.pageNumStatus === 'partial', `PageNumPos status = partial (실제: ${pageActionInfo.pageNumStatus})`);

  console.log('  [7] 구현률 확인...');
  const implRate = await page.evaluate(() => {
    const total = window.HwpCtrl.constructor.getRegisteredActionCount();
    const impl = window.HwpCtrl.constructor.getImplementedActionCount();
    const executable = window.HwpCtrl.constructor.getExecutableActionCount();
    const statusCounts = window.HwpCtrl.constructor.getActionStatusCounts();
    return { total, impl, executable, statusCounts, rate: Math.round(impl / total * 100) };
  });
  assert(implRate.statusCounts.partial > 0, 'partial 상태 Action이 노출되어야 함');
  assert(implRate.statusCounts.stub > 0, 'stub 상태 Action이 노출되어야 함');
  assert(implRate.executable >= implRate.impl, '실행 함수 연결 수는 implemented 수 이상이어야 함');
  console.log(`     한컴 호환 완료: ${implRate.impl}/${implRate.total} (${implRate.rate}%)`);
  console.log(`     상태: ${JSON.stringify(implRate.statusCounts)} / executable=${implRate.executable}`);
  await screenshot(page, 'hwpctl-basic');
  console.log('\n✅ 모든 테스트 통과!');
}, { skipLoadApp: true });
