/**
 * Wave 6: 용지/머리말 Action executors
 * PageSetup, HeaderFooter, BreakSection, BreakColDef, PageNumPos
 */
import { registerAction } from '../action-registry';
import type { HwpCtrl } from '../index';
import type { ParameterSet } from '../parameter-set';

/**
 * PageSetup (SecDef ParameterSet)
 * hwpctl Item → rhwp JSON:
 *   PaperWidth → width, PaperHeight → height
 *   TopMargin → marginTop, BottomMargin → marginBottom
 *   LeftMargin → marginLeft, RightMargin → marginRight
 *   HeaderMargin → headerMargin, FooterMargin → footerMargin
 *   Landscape → landscape (0=세로, 1=가로)
 */
function executePageSetup(ctrl: HwpCtrl, set: ParameterSet | null): boolean {
  if (!set) return false;
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  const obj: Record<string, any> = {};
  const mapping: [string, string][] = [
    ['PaperWidth', 'width'], ['PaperHeight', 'height'],
    ['TopMargin', 'marginTop'], ['BottomMargin', 'marginBottom'],
    ['LeftMargin', 'marginLeft'], ['RightMargin', 'marginRight'],
    ['HeaderMargin', 'headerMargin'], ['FooterMargin', 'footerMargin'],
  ];

  for (const [hwpKey, rhwpKey] of mapping) {
    const v = set.GetItem(hwpKey);
    if (v !== undefined) obj[rhwpKey] = v;
  }

  const landscape = set.GetItem('Landscape');
  if (landscape !== undefined) obj.landscape = !!landscape;

  return ctrl.executeWithUndo(() => {
    const result = doc.setPageDef(cursor.section, JSON.stringify(obj));
    return JSON.parse(result).ok === true;
  });
}

/**
 * HeaderFooter ParameterSet:
 *   Type: 0=머리말, 1=꼬리말
 *   Apply: 0=양쪽, 1=짝수, 2=홀수
 */
function executeHeaderFooter(ctrl: HwpCtrl, set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  const type = set?.GetItem('Type') ?? 0; // 0=머리말, 1=꼬리말
  const apply = set?.GetItem('Apply') ?? 0; // 0=양쪽, 1=짝수, 2=홀수
  const isHeader = type === 0;

  const applyMap: Record<number, string> = { 0: 'Both', 1: 'Even', 2: 'Odd' };
  const applyTo = applyMap[apply] ?? 'Both';

  return ctrl.executeWithUndo(() => {
    const result = doc.createHeaderFooter(
      cursor.section, cursor.para,
      isHeader, applyTo,
    );
    return JSON.parse(result).ok === true;
  });
}

function executeBreakSection(ctrl: HwpCtrl, _set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();

  return ctrl.executeWithUndo(() => {
    const result = doc.insertSectionBreak(cursor.section, cursor.para, cursor.pos);
    const parsed = JSON.parse(result);
    if (parsed.ok) {
      ctrl.SetPos(parsed.sectionIdx ?? cursor.section + 1, parsed.paraIdx ?? 0, parsed.charOffset ?? 0);
    }
    return parsed.ok === true;
  });
}

function executeBreakColDef(ctrl: HwpCtrl, set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  const columnCount = set?.GetItem('ColCount')
    ?? set?.GetItem('ColumnCount')
    ?? set?.GetItem('Count')
    ?? 2;
  const columnType = set?.GetItem('ColumnType') ?? set?.GetItem('Type') ?? 0;
  const sameWidth = set?.GetItem('SameWidth') ?? 1;
  const spacing = set?.GetItem('Spacing') ?? set?.GetItem('Gap') ?? 2268;

  return ctrl.executeWithUndo(() => {
    const result = doc.setColumnDef(cursor.section, columnCount, columnType, sameWidth, spacing);
    return JSON.parse(result).ok === true;
  });
}

function executePageNumPos(ctrl: HwpCtrl, set: ParameterSet | null): boolean {
  const doc = ctrl.getWasmDoc();
  const cursor = ctrl.getCursor();
  const position = set?.GetItem('Pos')
    ?? set?.GetItem('Position')
    ?? set?.GetItem('PageNumPos')
    ?? 5;
  const format = set?.GetItem('Format')
    ?? set?.GetItem('NumberFormat')
    ?? 0;
  const dashChar = set?.GetItem('SideChar')
    ?? set?.GetItem('DashChar')
    ?? '-';

  return ctrl.executeWithUndo(() => {
    const result = doc.setPageNumberPos(cursor.section, cursor.para, format, position, String(dashChar));
    return JSON.parse(result).ok === true;
  });
}

// Action 등록
registerAction({ id: 'PageSetup', parameterSetId: 'SecDef', description: '편집 용지', executor: executePageSetup, compatibilityStatus: 'partial', statusNote: '기본 용지/여백 항목만 매핑합니다.' });
registerAction({ id: 'HeaderFooter', parameterSetId: 'HeaderFooter', description: '머리말/꼬리말', executor: executeHeaderFooter, compatibilityStatus: 'partial', statusNote: '생성 흐름만 연결되어 있고 편집 UI/필드 규칙은 Studio 커맨드가 담당합니다.' });
registerAction({ id: 'BreakSection', parameterSetId: null, description: '구역 나누기', executor: executeBreakSection, compatibilityStatus: 'implemented', statusNote: '현재 커서 위치에서 문단을 분할하고 뒤쪽 내용을 새 구역으로 이동합니다.' });
registerAction({ id: 'BreakColDef', parameterSetId: null, description: '단 정의', executor: executeBreakColDef, compatibilityStatus: 'partial', statusNote: '기본 단 개수, 단 종류, 같은 너비, 간격을 ColumnDef로 반영합니다.' });
registerAction({ id: 'PageNumPos', parameterSetId: 'PageNumPos', description: '쪽 번호', executor: executePageNumPos, compatibilityStatus: 'partial', statusNote: '쪽 번호 위치/형식/대시 문자를 PageNumberPos 컨트롤로 반영합니다.' });
