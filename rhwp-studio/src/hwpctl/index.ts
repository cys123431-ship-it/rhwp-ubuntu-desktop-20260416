/**
 * hwpctl 호환 HwpCtrl 클래스
 *
 * 한컴 웹기안기의 HwpCtrl ActiveX/JavaScript 객체와 동일한 인터페이스.
 * 내부적으로 rhwp WASM API를 호출한다.
 */
import { Action } from './action';
import { ParameterSet } from './parameter-set';
import {
  getActionDef,
  getRegisteredCount,
  getImplementedCount,
  getExecutableCount,
  getAllActions,
  getActionStatusCounts,
} from './action-registry';

// Wave 1~6: Action executor 등록 (import 시 자동 등록)
import './actions/table';
import './actions/text';
import './actions/format';
import './actions/table-edit';
import './actions/navigate';
import './actions/clipboard';
import './actions/page';

export { ParameterSet } from './parameter-set';
export { Action } from './action';

export class HwpCtrl {
  /** rhwp WASM 문서 객체 */
  private wasmDoc: any;
  /** 현재 커서 위치 */
  private cursorSection = 0;
  private cursorPara = 0;
  private cursorPos = 0;
  /** hwpctl 직접 실행용 문서 스냅샷 히스토리 */
  private undoStack: Uint8Array[] = [];
  private redoStack: Uint8Array[] = [];
  private readonly historyLimit = 50;
  /** 이벤트 리스너 */
  private listeners: Map<number, Function[]> = new Map();

  constructor(wasmDoc: any) {
    this.wasmDoc = wasmDoc;
  }

  /** 내부: WASM 문서 객체 접근 */
  getWasmDoc(): any {
    return this.wasmDoc;
  }

  /** 내부: Studio 입력 핸들러 접근 */
  getInputHandler(): any | null {
    return (globalThis as any).__inputHandler ?? null;
  }

  /** 내부: 현재 커서를 Studio 입력 핸들러 기준으로 동기화 */
  syncCursorFromInputHandler(): void {
    const ih = this.getInputHandler();
    const pos = ih?.getCursorPosition?.() ?? ih?.getPosition?.();
    if (!pos) return;
    this.cursorSection = pos.sectionIndex ?? this.cursorSection;
    this.cursorPara = pos.paragraphIndex ?? this.cursorPara;
    this.cursorPos = pos.charOffset ?? this.cursorPos;
  }

  /** 내부: 문서 변경 알림 */
  notifyDocumentChanged(): void {
    const ih = this.getInputHandler();
    if (ih?.triggerAfterEdit) {
      ih.triggerAfterEdit();
      this.syncCursorFromInputHandler();
      return;
    }
    (globalThis as any).__eventBus?.emit?.('document-changed');
  }

  private captureSnapshot(): Uint8Array | null {
    try {
      const bytes = this.wasmDoc.exportHwp?.() ?? this.wasmDoc.save?.('hwp');
      if (!bytes) return null;
      return new Uint8Array(bytes);
    } catch (e) {
      console.warn('[hwpctl] 문서 스냅샷 생성 실패:', e);
      return null;
    }
  }

  private restoreSnapshot(bytes: Uint8Array): boolean {
    try {
      this.wasmDoc = new (this.wasmDoc.constructor)(bytes);
      this.cursorSection = 0;
      this.cursorPara = 0;
      this.cursorPos = 0;
      this.notifyDocumentChanged();
      return true;
    } catch (e) {
      console.error('[hwpctl] 문서 스냅샷 복원 실패:', e);
      return false;
    }
  }

  /** 내부: hwpctl 직접 실행 히스토리에 현재 문서를 저장 */
  recordUndoSnapshot(): void {
    const snapshot = this.captureSnapshot();
    if (!snapshot) return;
    this.undoStack.push(snapshot);
    if (this.undoStack.length > this.historyLimit) {
      this.undoStack.shift();
    }
    this.redoStack = [];
  }

  /** 내부: 문서 변경 작업을 undo 스냅샷과 함께 실행 */
  executeWithUndo(operation: () => boolean): boolean {
    this.recordUndoSnapshot();
    let ok = false;
    try {
      ok = operation();
    } catch (e) {
      console.error('[hwpctl] 문서 변경 작업 실패:', e);
      ok = false;
    }
    if (!ok) {
      const snapshot = this.undoStack.pop();
      if (snapshot) this.restoreSnapshot(snapshot);
      return false;
    }
    this.notifyDocumentChanged();
    return true;
  }

  /** 내부: 현재 커서 위치 */
  getCursor(): { section: number; para: number; pos: number } {
    this.syncCursorFromInputHandler();
    return { section: this.cursorSection, para: this.cursorPara, pos: this.cursorPos };
  }

  // ── HwpCtrl API ──

  /** 문서 열기 (Blob/ArrayBuffer) */
  Open(data: ArrayBuffer | Uint8Array, callback?: (success: boolean) => void): boolean {
    try {
      const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : data;
      this.wasmDoc = new (this.wasmDoc.constructor)(bytes);
      this.undoStack = [];
      this.redoStack = [];
      this.cursorSection = 0;
      this.cursorPara = 0;
      this.cursorPos = 0;
      callback?.(true);
      return true;
    } catch (e) {
      console.error('[hwpctl] Open 실패:', e);
      callback?.(false);
      return false;
    }
  }

  /** 빈 문서 생성 */
  Clear(): void {
    this.recordUndoSnapshot();
    this.wasmDoc.createBlankDocument();
    this.cursorSection = 0;
    this.cursorPara = 0;
    this.cursorPos = 0;
    this.notifyDocumentChanged();
  }

  /** HWP 파일로 내보내기 */
  SaveAs(filename: string, format?: string, arg?: string): boolean {
    try {
      const bytes = this.wasmDoc.exportHwp();
      const blob = new Blob([bytes as BlobPart], { type: 'application/x-hwp' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
      return true;
    } catch (e) {
      console.error('[hwpctl] SaveAs 실패:', e);
      return false;
    }
  }

  /** Action 생성 */
  CreateAction(actionId: string): Action {
    const def = getActionDef(actionId);
    if (!def) {
      console.warn(`[hwpctl] Action "${actionId}" 미등록`);
      return new Action(this, {
        id: actionId, parameterSetId: null,
        description: '미등록', executor: null,
        compatibilityStatus: 'unsupported',
        statusNote: 'rhwp hwpctl 레지스트리에 없는 Action입니다.',
      });
    }
    return new Action(this, def);
  }

  /** ParameterSet 생성 */
  CreateSet(setName: string): ParameterSet {
    return new ParameterSet(setName);
  }

  /** 컨트롤 삽입 (InsertCtrl) */
  InsertCtrl(ctrlName: string, set?: ParameterSet): boolean {
    // ctrlCode → Action 매핑
    const actionMap: Record<string, string> = {
      'tbl': 'TableCreate',
      'secd': 'PageSetup',
      'cold': 'BreakColDef',
    };
    const actionId = actionMap[ctrlName] || ctrlName;
    const action = this.CreateAction(actionId);
    return action.Execute(set || new ParameterSet(actionId));
  }

  /** 텍스트 삽입 */
  InsertText(text: string): boolean {
    return this.executeWithUndo(() => {
      this.wasmDoc.insertText(
        this.cursorSection, this.cursorPara, this.cursorPos, text,
      );
      this.cursorPos += text.length;
      return true;
    });
  }

  /** Action 단순 실행 */
  Run(actionId: string): boolean {
    const action = this.CreateAction(actionId);
    return action.Run();
  }

  /** 커서 위치 설정 */
  SetPos(list: number, para: number, pos: number): boolean {
    this.cursorSection = list;
    this.cursorPara = para;
    this.cursorPos = pos;
    const ih = this.getInputHandler();
    if (ih?.moveCursorTo) {
      ih.moveCursorTo({
        sectionIndex: list,
        paragraphIndex: para,
        charOffset: pos,
      });
      this.syncCursorFromInputHandler();
    }
    return true;
  }

  /** 커서 위치 반환 */
  GetPos(): { list: number; para: number; pos: number } {
    return { list: this.cursorSection, para: this.cursorPara, pos: this.cursorPos };
  }

  /** 페이지 수 */
  PageCount(): number {
    return this.wasmDoc.pageCount();
  }

  // ── 표 셀 텍스트 API ──

  /** 표 셀에 텍스트 설정 (행렬 좌표 기반)
   * @param tableParaIdx 표가 포함된 문단 인덱스
   * @param row 행 (0부터)
   * @param col 열 (0부터)
   * @param text 삽입할 텍스트
   * @param colCount 열 수 (생략 시 cellIdx = row * colCount + col 계산 불가 → cellIdx 직접 사용)
   * @param controlIdx 표 컨트롤 인덱스 (기본 0)
   */
  SetCellText(tableParaIdx: number, row: number, col: number, text: string, colCount: number, controlIdx = 0): boolean {
    return this.executeWithUndo(() => {
      const cellIdx = row * colCount + col;
      const result = this.wasmDoc.insertTextInCell(
        this.cursorSection, tableParaIdx, controlIdx, cellIdx, 0, 0, text,
      );
      const parsed = JSON.parse(result);
      return parsed.ok === true;
    });
  }

  /** 표 셀 텍스트 조회 (행렬 좌표 기반) */
  GetCellText(tableParaIdx: number, row: number, col: number, colCount: number, controlIdx = 0): string {
    try {
      const cellIdx = row * colCount + col;
      const path = `s${this.cursorSection}:p${tableParaIdx}:c${controlIdx}:cell${cellIdx}:p0`;
      const result = this.wasmDoc.getTextInCellByPath(path);
      return result || '';
    } catch (e) {
      console.error(`[hwpctl] GetCellText(pi=${tableParaIdx}, r=${row}, c=${col}) 실패:`, e);
      return '';
    }
  }

  /** 표 셀에서 계산식 실행 */
  EvaluateFormula(tableParaIdx: number, row: number, col: number, formula: string, writeResult = true, controlIdx = 0): any {
    if (writeResult) this.recordUndoSnapshot();
    try {
      const result = this.wasmDoc.evaluateTableFormula(
        this.cursorSection, tableParaIdx, controlIdx, row, col, formula, writeResult,
      );
      const parsed = JSON.parse(result);
      if (writeResult && parsed.ok) this.notifyDocumentChanged();
      return parsed;
    } catch (e) {
      console.error(`[hwpctl] EvaluateFormula 실패:`, e);
      return { ok: false, error: String(e) };
    }
  }

  /** 이벤트 리스너 등록 */
  addEventListener(eventType: number, callback: Function): void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, []);
    }
    this.listeners.get(eventType)!.push(callback);
  }

  // ── Field API (누름틀) ──

  /** 필드 목록 조회 */
  GetFieldList(): any[] {
    try {
      const json = this.wasmDoc.getFieldList();
      return JSON.parse(json);
    } catch (e) {
      console.error('[hwpctl] GetFieldList 실패:', e);
      return [];
    }
  }

  /** 필드로 커서 이동 */
  MoveToField(field: string, getText?: boolean, moveStart?: boolean, select?: boolean): boolean {
    try {
      const fields = this.GetFieldList();
      const found = fields.find((f: any) => f.name === field);
      if (!found) {
        console.warn(`[hwpctl] 필드 "${field}" 없음`);
        return false;
      }
      const loc = found.location;
      this.cursorSection = loc.sectionIndex ?? 0;
      this.cursorPara = loc.paraIndex ?? 0;
      this.cursorPos = 0;
      return true;
    } catch (e) {
      console.error('[hwpctl] MoveToField 실패:', e);
      return false;
    }
  }

  /** 필드 텍스트 설정 (한컴 호환: PutFieldText) */
  PutFieldText(field: string, text: string): boolean {
    return this.executeWithUndo(() => {
      const result = this.wasmDoc.setFieldValueByName(field, text);
      const parsed = JSON.parse(result);
      return parsed.ok === true;
    });
  }

  /** 필드 텍스트 조회 (한컴 호환: GetFieldText) */
  GetFieldText(field: string): string {
    try {
      const result = this.wasmDoc.getFieldValueByName(field);
      const parsed = JSON.parse(result);
      return parsed.ok ? parsed.value : '';
    } catch (e) {
      console.error(`[hwpctl] GetFieldText("${field}") 실패:`, e);
      return '';
    }
  }

  /** 커서 위치 이동 (한컴 호환: MovePos) */
  MovePos(pos: number): boolean {
    try {
      switch (pos) {
        case 2: // 문서 끝
          const pageCount = this.wasmDoc.pageCount();
          // 마지막 구역, 마지막 문단으로 이동
          this.cursorSection = 0;
          this.cursorPara = 0;
          this.cursorPos = 0;
          break;
        case 3: // 문서 시작
          this.cursorSection = 0;
          this.cursorPara = 0;
          this.cursorPos = 0;
          break;
        default:
          console.warn(`[hwpctl] MovePos(${pos}) 미지원`);
      }
      return true;
    } catch (e) {
      console.error('[hwpctl] MovePos 실패:', e);
      return false;
    }
  }

  /** 현재 필드 이름 설정 */
  SetCurFieldName(name: string): boolean {
    console.info(`[hwpctl] SetCurFieldName("${name}") — stub`);
    return true;
  }

  /** 필드 이름 변경 */
  RenameField(oldName: string, newName: string): boolean {
    console.info(`[hwpctl] RenameField("${oldName}" → "${newName}") — stub`);
    return true;
  }

  /** 실행취소 */
  Undo(): boolean {
    const ih = this.getInputHandler();
    if (ih?.canUndo?.() && ih?.performUndo) {
      ih.performUndo();
      this.syncCursorFromInputHandler();
      return true;
    }

    const snapshot = this.undoStack.pop();
    if (!snapshot) return false;
    const current = this.captureSnapshot();
    if (current) this.redoStack.push(current);
    return this.restoreSnapshot(snapshot);
  }

  /** 다시실행 */
  Redo(): boolean {
    const ih = this.getInputHandler();
    if (ih?.canRedo?.() && ih?.performRedo) {
      ih.performRedo();
      this.syncCursorFromInputHandler();
      return true;
    }

    const snapshot = this.redoStack.pop();
    if (!snapshot) return false;
    const current = this.captureSnapshot();
    if (current) this.undoStack.push(current);
    return this.restoreSnapshot(snapshot);
  }

  // ── 진행률 추적 ──

  /** 등록된 Action 수 */
  static getRegisteredActionCount(): number {
    return getRegisteredCount();
  }

  /** 구현된 Action 수 */
  static getImplementedActionCount(): number {
    return getImplementedCount();
  }

  /** 실행 함수가 연결된 Action 수 */
  static getExecutableActionCount(): number {
    return getExecutableCount();
  }

  /** 호환 상태별 Action 수 */
  static getActionStatusCounts() {
    return getActionStatusCounts();
  }

  /** 전체 Action 목록 (디버깅/테스트용) */
  static getAllActions() {
    return getAllActions();
  }
}

/**
 * hwpctl 호환 HwpCtrl 생성 (비동기 초기화)
 *
 * 사용:
 * ```javascript
 * const HwpCtrl = await createHwpCtrl({ wasmUrl: '/pkg/rhwp_bg.wasm' });
 * HwpCtrl.Open(fileBlob);
 * ```
 */
export async function createHwpCtrl(options: {
  wasmUrl?: string;
  wasmModule?: any;
}): Promise<HwpCtrl> {
  let wasmDoc: any;

  if (options.wasmModule) {
    // 이미 로딩된 WASM 모듈 사용
    wasmDoc = options.wasmModule;
  } else {
    // 동적 로딩
    const { default: init, HwpDocument } = await import('@wasm/rhwp');
    await init(options.wasmUrl);
    wasmDoc = HwpDocument.createEmpty();
  }

  return new HwpCtrl(wasmDoc);
}
