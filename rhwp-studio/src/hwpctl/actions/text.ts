/**
 * Wave 1: InsertText, BreakPara, BreakPage, BreakColumn Action executors
 */
import { registerAction } from '../action-registry';
import type { HwpCtrl } from '../index';
import type { ParameterSet } from '../parameter-set';

function executeInsertText(ctrl: HwpCtrl, set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  const text = set?.GetItem('Text') ?? '';

  if (!text) return false;

  return ctrl.executeWithUndo(() => {
    const result = doc.insertText(cursor.section, cursor.para, cursor.pos, text);
    const parsed = JSON.parse(result);
    if (parsed.ok) {
      ctrl.SetPos(cursor.section, cursor.para, parsed.charOffset ?? cursor.pos + text.length);
    }
    return parsed.ok === true;
  });
}

function executeBreakPara(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  return ctrl.executeWithUndo(() => {
    const result = doc.splitParagraph(cursor.section, cursor.para, cursor.pos);
    const parsed = JSON.parse(result);
    if (parsed.ok) {
      ctrl.SetPos(cursor.section, cursor.para + 1, 0);
    }
    return parsed.ok === true;
  });
}

function executeBreakPage(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const ih = ctrl.getInputHandler();
  if (ih?.performPageBreak) {
    const ok = ih.performPageBreak();
    ctrl.syncCursorFromInputHandler();
    return ok !== false;
  }

  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  return ctrl.executeWithUndo(() => {
    const result = doc.insertPageBreak(cursor.section, cursor.para, cursor.pos);
    const parsed = JSON.parse(result);
    if (parsed.ok) {
      ctrl.SetPos(cursor.section, parsed.paraIdx ?? cursor.para + 1, 0);
    }
    return parsed.ok === true;
  });
}

function executeBreakColumn(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const ih = ctrl.getInputHandler();
  if (ih?.performColumnBreak) {
    const ok = ih.performColumnBreak();
    ctrl.syncCursorFromInputHandler();
    return ok !== false;
  }

  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  return ctrl.executeWithUndo(() => {
    const result = doc.insertColumnBreak(cursor.section, cursor.para, cursor.pos);
    const parsed = JSON.parse(result);
    if (parsed.ok) {
      ctrl.SetPos(cursor.section, parsed.paraIdx ?? cursor.para + 1, 0);
    }
    return parsed.ok === true;
  });
}

// 기존 stub를 구현으로 교체
registerAction({ id: 'InsertText', parameterSetId: 'InsertText', description: '텍스트 삽입', executor: executeInsertText, compatibilityStatus: 'implemented', statusNote: '현재 커서 위치 텍스트 삽입 기본 동작을 지원합니다.' });
registerAction({ id: 'BreakPara', parameterSetId: null, description: '문단 나누기', executor: executeBreakPara, compatibilityStatus: 'implemented', statusNote: '현재 커서 위치 문단 나누기를 지원합니다.' });
registerAction({ id: 'BreakPage', parameterSetId: null, description: '쪽 나누기', executor: executeBreakPage, compatibilityStatus: 'implemented', statusNote: '현재 커서 위치 쪽 나누기를 지원합니다.' });
registerAction({ id: 'BreakColumn', parameterSetId: null, description: '단 나누기', executor: executeBreakColumn, compatibilityStatus: 'implemented', statusNote: '현재 커서 위치 단 나누기를 지원합니다.' });
