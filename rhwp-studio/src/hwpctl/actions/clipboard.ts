/**
 * Wave 5: 클립보드 + 실행취소 Action executors
 * Copy, Cut, Paste, Undo, Redo
 */
import { registerAction } from '../action-registry';
import type { HwpCtrl } from '../index';
import type { ParameterSet } from '../parameter-set';

function executeCopy(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const ih = ctrl.getInputHandler();
  if (ih?.performCopy) {
    ih.performCopy();
    return true;
  }

  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  try {
    doc.copySelection(cursor.section, cursor.para, 0, cursor.para, 65535);
    return true;
  } catch (e) {
    console.error('[hwpctl] Copy 실패:', e);
    return false;
  }
}

function executeCut(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const ih = ctrl.getInputHandler();
  if (ih?.performCut) {
    ih.performCut();
    ctrl.syncCursorFromInputHandler();
    return true;
  }

  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  return ctrl.executeWithUndo(() => {
    doc.copySelection(cursor.section, cursor.para, 0, cursor.para, 65535);
    doc.deleteText(cursor.section, cursor.para, 0, 65535);
    return true;
  });
}

function executePaste(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const ih = ctrl.getInputHandler();
  if (ih?.performPaste) {
    ih.performPaste();
    ctrl.syncCursorFromInputHandler();
    return true;
  }

  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  return ctrl.executeWithUndo(() => {
    const result = doc.pasteInternal(cursor.section, cursor.para, cursor.pos);
    const parsed = JSON.parse(result);
    if (parsed.ok && parsed.charOffset !== undefined) {
      ctrl.SetPos(cursor.section, cursor.para, parsed.charOffset);
    }
    return parsed.ok === true;
  });
}

function executeUndo(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  return ctrl.Undo();
}

function executeRedo(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  return ctrl.Redo();
}

// Action 등록
registerAction({ id: 'Copy', parameterSetId: null, description: '복사', executor: executeCopy, compatibilityStatus: 'partial', statusNote: '현재 커서 문단 기반의 내부 복사 흐름입니다.' });
registerAction({ id: 'Cut', parameterSetId: null, description: '잘라내기', executor: executeCut, compatibilityStatus: 'partial', statusNote: '현재 커서 문단 기반의 내부 잘라내기 흐름입니다.' });
registerAction({ id: 'Paste', parameterSetId: null, description: '붙여넣기', executor: executePaste, compatibilityStatus: 'partial', statusNote: '내부 클립보드 붙여넣기 흐름만 연결되어 있습니다.' });
registerAction({ id: 'Undo', parameterSetId: null, description: '실행취소', executor: executeUndo, compatibilityStatus: 'implemented', statusNote: 'Studio 입력 히스토리 또는 hwpctl 문서 스냅샷 히스토리로 되돌립니다.' });
registerAction({ id: 'Redo', parameterSetId: null, description: '다시실행', executor: executeRedo, compatibilityStatus: 'implemented', statusNote: 'Studio 입력 히스토리 또는 hwpctl 문서 스냅샷 히스토리로 다시 실행합니다.' });
