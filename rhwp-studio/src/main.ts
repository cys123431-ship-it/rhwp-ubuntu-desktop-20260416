import { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentInfo, DocumentSession, RecentDocument, RecoverySnapshotMeta } from '@/core/types';
import { EventBus } from '@/core/event-bus';
import { createDocumentIO, type OpenDocumentResult } from '@/core/document-io';
import { DocumentSessionStore, createRecentDocument } from '@/core/document-session';
import { CanvasView } from '@/view/canvas-view';
import { InputHandler } from '@/engine/input-handler';
import { Toolbar } from '@/ui/toolbar';
import { MenuBar } from '@/ui/menu-bar';
import { loadWebFonts } from '@/core/font-loader';
import { CommandRegistry } from '@/command/registry';
import { CommandDispatcher } from '@/command/dispatcher';
import type { EditorContext, CommandServices } from '@/command/types';
import { fileCommands } from '@/command/commands/file';
import { editCommands } from '@/command/commands/edit';
import { viewCommands } from '@/command/commands/view';
import { formatCommands } from '@/command/commands/format';
import { insertCommands } from '@/command/commands/insert';
import { tableCommands } from '@/command/commands/table';
import { pageCommands } from '@/command/commands/page';
import { toolCommands } from '@/command/commands/tool';
import { ContextMenu } from '@/ui/context-menu';
import { CommandPalette } from '@/ui/command-palette';
import { CellSelectionRenderer } from '@/engine/cell-selection-renderer';
import { TableObjectRenderer } from '@/engine/table-object-renderer';
import { TableResizeRenderer } from '@/engine/table-resize-renderer';
import { Ruler } from '@/view/ruler';
import { showConfirm } from '@/ui/confirm-dialog';

const wasm = new WasmBridge();
const eventBus = new EventBus();
const documentIO = createDocumentIO();
const documentSession = new DocumentSessionStore();
let e2eReady = false;

type RhwpE2EBannerState = {
  visible: boolean;
  text: string;
  className: string;
  actions: string[];
};

type RhwpE2EDialogState = {
  visible: boolean;
  title: string;
  message: string;
};

type RhwpE2ERuntimeState = {
  ready: boolean;
  pageCount: number;
  statusMessage: string;
  session: DocumentSession;
  banner: RhwpE2EBannerState;
  dialog: RhwpE2EDialogState;
};

type RhwpE2EBridge = {
  isReady: () => boolean;
  getRuntimeState: () => RhwpE2ERuntimeState;
  clickBannerAction: (label?: string) => Promise<boolean>;
  markDocumentDirty: () => Promise<boolean>;
  appendTextToParagraph: (text: string, sectionIndex?: number, paragraphIndex?: number) => Promise<boolean>;
  saveCurrentDocument: () => Promise<boolean>;
  flushRecoverySnapshot: () => Promise<string | null>;
  listRecoverySnapshots: () => Promise<RecoverySnapshotMeta[]>;
  getRecentDocuments: () => Promise<RecentDocument[]>;
  getOpenDialog: () => RhwpE2EDialogState;
  acceptActiveDialog: () => Promise<boolean>;
  dismissActiveDialog: () => Promise<boolean>;
};

declare global {
  interface Window {
    __RHWP_E2E__?: RhwpE2EBridge;
  }
}

// E2E 테스트용 전역 노출 (개발 모드 전용)
if (import.meta.env.DEV) {
  (window as any).__wasm = wasm;
  (window as any).__eventBus = eventBus;
}
let canvasView: CanvasView | null = null;
let inputHandler: InputHandler | null = null;
let toolbar: Toolbar | null = null;
let ruler: Ruler | null = null;
let startupFilesReceived = false;
let recoveryTimer: number | null = null;
let recoveryWriteInFlight = false;


// ─── 커맨드 시스템 ─────────────────────────────
const registry = new CommandRegistry();

function getContext(): EditorContext {
  const session = documentSession.current;
  return {
    hasDocument: wasm.pageCount > 0,
    hasSelection: inputHandler?.hasSelection() ?? false,
    inTable: inputHandler?.isInTable() ?? false,
    inCellSelectionMode: inputHandler?.isInCellSelectionMode() ?? false,
    inTableObjectSelection: inputHandler?.isInTableObjectSelection() ?? false,
    inPictureObjectSelection: inputHandler?.isInPictureObjectSelection() ?? false,
    inField: inputHandler?.isInField() ?? false,
    isEditable: session.hasDocument && !session.isProtected,
    isProtected: session.isProtected,
    canSave: session.hasDocument && !session.isProtected,
    saveFormat: session.saveFormat,
    canUndo: inputHandler?.canUndo() ?? false,
    canRedo: inputHandler?.canRedo() ?? false,
    zoom: canvasView?.getViewportManager().getZoom() ?? 1.0,
    showControlCodes: wasm.getShowControlCodes(),
  };
}

const commandServices: CommandServices = {
  eventBus,
  wasm,
  documentIO,
  session: documentSession,
  getContext,
  getInputHandler: () => inputHandler,
  getViewportManager: () => canvasView?.getViewportManager() ?? null,
};

const dispatcher = new CommandDispatcher(registry, commandServices, eventBus);

// 모든 내장 커맨드 등록
registry.registerAll(fileCommands);
registry.registerAll(editCommands);
registry.registerAll(viewCommands);
registry.registerAll(formatCommands);
registry.registerAll(insertCommands);
registry.registerAll(tableCommands);
registry.registerAll(pageCommands);
registry.registerAll(toolCommands);

// 상태 바 요소
const sbMessage = () => document.getElementById('sb-message')!;
const sbPage = () => document.getElementById('sb-page')!;
const sbSection = () => document.getElementById('sb-section')!;
const sbZoomVal = () => document.getElementById('sb-zoom-val')!;
const sessionBanner = () => document.getElementById('session-banner')!;

function toUint8Array(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}

function cloneSession(session: Readonly<DocumentSession>): DocumentSession {
  return {
    ...session,
    blockers: [...session.blockers],
    warnings: [...session.warnings],
    compatibilityIssues: session.compatibilityIssues.map((item) => ({ ...item })),
    fontSubstitutions: session.fontSubstitutions.map((item) => ({ ...item })),
    associationStatus: session.associationStatus
      ? {
        ...session.associationStatus,
        pendingMimeTypes: [...session.associationStatus.pendingMimeTypes],
      }
      : null,
  };
}

function getSessionBannerState(): RhwpE2EBannerState {
  const banner = sessionBanner();
  const text = banner.querySelector('.session-banner__text')?.textContent?.trim() ?? '';
  const actions = Array.from(banner.querySelectorAll<HTMLButtonElement>('.session-banner__button'))
    .map((button) => button.textContent?.trim() ?? '')
    .filter((label) => label.length > 0);

  return {
    visible: !banner.hidden,
    text,
    className: banner.className,
    actions,
  };
}

function getDialogState(): RhwpE2EDialogState {
  const overlay = document.querySelector<HTMLElement>('.modal-overlay');
  if (!overlay) {
    return { visible: false, title: '', message: '' };
  }

  return {
    visible: true,
    title: overlay.querySelector('.dialog-title')?.textContent?.replace('×', '').trim() ?? '',
    message: overlay.querySelector('.dialog-body')?.textContent?.trim() ?? '',
  };
}

function getRuntimeState(): RhwpE2ERuntimeState {
  return {
    ready: e2eReady,
    pageCount: wasm.pageCount,
    statusMessage: sbMessage()?.textContent?.trim() ?? '',
    session: cloneSession(documentSession.current),
    banner: getSessionBannerState(),
    dialog: getDialogState(),
  };
}

async function waitForCondition(
  predicate: () => boolean,
  timeoutMs = 10000,
  intervalMs = 100,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return true;
    }
    await new Promise((resolve) => window.setTimeout(resolve, intervalMs));
  }
  return predicate();
}

async function clickBannerAction(label?: string): Promise<boolean> {
  const banner = sessionBanner();
  if (banner.hidden) return false;

  const buttons = Array.from(banner.querySelectorAll<HTMLButtonElement>('.session-banner__button'));
  const target = label
    ? buttons.find((button) => button.textContent?.trim() === label)
    : buttons[0];
  if (!target) return false;

  target.click();
  return true;
}

async function markDocumentDirty(): Promise<boolean> {
  if (!documentSession.current.hasDocument || documentSession.current.isProtected) {
    return false;
  }

  documentSession.markDirty();
  wasm.markDirty();
  syncSessionFromWasm();
  eventBus.emit('command-state-changed');
  return true;
}

async function appendTextToParagraph(
  text: string,
  sectionIndex = 0,
  paragraphIndex = 0,
): Promise<boolean> {
  if (!documentSession.current.hasDocument || documentSession.current.isProtected) {
    return false;
  }

  const charOffset = wasm.getParagraphLength(sectionIndex, paragraphIndex);
  wasm.insertText(sectionIndex, paragraphIndex, charOffset, text);
  eventBus.emit('document-changed');
  return true;
}

async function saveCurrentDocument(): Promise<boolean> {
  if (!documentSession.current.hasDocument || documentSession.current.isProtected) {
    return false;
  }

  const dispatched = dispatcher.dispatch('file:save');
  if (!dispatched) {
    return false;
  }

  return waitForCondition(() => !documentSession.current.dirty, 15000, 100);
}

async function flushRecoverySnapshot(): Promise<string | null> {
  await persistRecoverySnapshot();
  return documentSession.current.recoverySnapshotId;
}

async function acceptActiveDialog(): Promise<boolean> {
  const button = document.querySelector<HTMLButtonElement>('.modal-overlay .dialog-btn-primary');
  if (!button) return false;
  button.click();
  return true;
}

async function dismissActiveDialog(): Promise<boolean> {
  const button = Array.from(
    document.querySelectorAll<HTMLButtonElement>('.modal-overlay .dialog-btn'),
  ).find((candidate) => !candidate.classList.contains('dialog-btn-primary'));
  if (!button) return false;
  button.click();
  return true;
}

function installE2EBridge(): void {
  window.__RHWP_E2E__ = {
    isReady: () => e2eReady,
    getRuntimeState,
    clickBannerAction,
    markDocumentDirty,
    appendTextToParagraph,
    saveCurrentDocument,
    flushRecoverySnapshot,
    listRecoverySnapshots: () => documentIO.listRecoverySnapshots(),
    getRecentDocuments: () => documentIO.getRecentDocuments(),
    getOpenDialog: getDialogState,
    acceptActiveDialog,
    dismissActiveDialog,
  };
}

function findMatchingRecoverySnapshot(
  snapshots: RecoverySnapshotMeta[],
  fileName: string,
  filePath: string,
  format: 'hwp' | 'hwpx',
): RecoverySnapshotMeta | null {
  if (filePath) {
    return snapshots.find((snapshot) => snapshot.filePath === filePath) ?? null;
  }

  return snapshots.find((snapshot) =>
    !snapshot.filePath
    && snapshot.fileName === fileName
    && snapshot.format === format,
  ) ?? null;
}

async function refreshDesktopAssociationStatus(): Promise<void> {
  if (documentIO.kind !== 'desktop') {
    documentSession.setAssociationStatus(null);
    renderSessionBanner();
    return;
  }

  try {
    documentSession.setAssociationStatus(await documentIO.getFileAssociationStatus());
  } catch (error) {
    console.warn('[desktop] file association status:', error);
    documentSession.setAssociationStatus(null);
  }

  renderSessionBanner();
}

function syncSessionFromWasm(): void {
  if (!documentSession.current.hasDocument) return;

  documentSession.syncCapabilities(wasm.getDocumentCapabilities());
  documentSession.setReports(
    wasm.getCompatibilityReport().issues,
    wasm.getFontSubstitutionReport().items,
  );

  toolbar?.setEnabled(!documentSession.current.isProtected);
  if (documentSession.current.isProtected) {
    inputHandler?.deactivate();
  }
  renderSessionBanner();
}

function renderSessionBanner(): void {
  const banner = sessionBanner();
  const session = documentSession.current;
  const parts: string[] = [];
  const actions: Array<{ label: string; handler: () => Promise<void> }> = [];

  if (
    session.associationStatus?.supported
    && !session.associationStatus.isDefault
    && session.associationStatus.actionMode !== 'none'
  ) {
    const associationActionLabel = session.associationStatus.actionMode === 'open-settings'
      ? 'Open Default Apps Settings'
      : 'Set as default app';

    parts.push(session.associationStatus.message);
    actions.push({
      label: associationActionLabel,
      handler: async () => {
        documentSession.setAssociationStatus(await documentIO.setDefaultFileAssociation());
        renderSessionBanner();
      },
    });
  }

  if (session.hasDocument && session.isProtected) {
    parts.push(`Protected view: ${session.blockers.join(' ')}`);
  }
  if (session.hasDocument && session.warnings.length > 0) {
    parts.push(session.warnings.join(' '));
  }
  if (session.hasDocument) {
    const substitutedFonts = session.fontSubstitutions.filter((item) => item.substituted);
    if (substitutedFonts.length > 0) {
      const preview = substitutedFonts
        .slice(0, 3)
        .map((item) => `${item.original}->${item.resolved}`)
        .join(', ');
      const suffix = substitutedFonts.length > 3 ? ` and ${substitutedFonts.length - 3} more` : '';
      parts.push(`Font substitutions active: ${preview}${suffix}`);
    }
  }

  if (parts.length === 0) {
    banner.hidden = true;
    banner.innerHTML = '';
    banner.className = 'session-banner';
    return;
  }

  banner.hidden = false;
  banner.className = `session-banner ${session.isProtected ? 'session-banner--protected' : 'session-banner--warning'}`;
  banner.innerHTML = '';

  const content = document.createElement('div');
  content.className = 'session-banner__content';

  const text = document.createElement('div');
  text.className = 'session-banner__text';
  text.textContent = parts.join(' ');
  content.appendChild(text);

  if (actions.length > 0) {
    const actionsWrap = document.createElement('div');
    actionsWrap.className = 'session-banner__actions';

    for (const action of actions) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'session-banner__button';
      button.textContent = action.label;
      button.addEventListener('click', () => {
        void action.handler();
      });
      actionsWrap.appendChild(button);
    }

    content.appendChild(actionsWrap);
  }

  banner.appendChild(content);
}

async function confirmDiscardIfDirty(title: string, message: string): Promise<boolean> {
  const session = documentSession.current;
  if (!session.hasDocument || !session.dirty || session.isProtected) {
    return true;
  }

  return showConfirm(title, message);
}

async function maybeRecoverOpenResult(
  result: OpenDocumentResult,
): Promise<{ result: OpenDocumentResult; snapshotId: string | null }> {
  if (documentIO.kind !== 'desktop') {
    return { result, snapshotId: null };
  }

  const format = result.fileName.toLowerCase().endsWith('.hwpx') ? 'hwpx' : 'hwp';

  try {
    const snapshots = await documentIO.listRecoverySnapshots();
    const matching = findMatchingRecoverySnapshot(
      snapshots,
      result.fileName,
      result.filePath ?? '',
      format,
    );
    if (!matching) {
      return { result, snapshotId: null };
    }

    const shouldRecover = await showConfirm(
      '자동 복구 문서 열기',
      `자동 복구본이 있습니다.\n\n문서: ${matching.fileName}\n시각: ${matching.updatedAt}\n\n자동 복구본으로 열까요?`,
    );
    if (!shouldRecover) {
      return { result, snapshotId: null };
    }

    const snapshot = await documentIO.readRecoverySnapshot(matching.id);
    if (!snapshot) {
      return { result, snapshotId: null };
    }

    return {
      result: {
        fileName: snapshot.fileName,
        filePath: snapshot.filePath,
        data: toUint8Array(snapshot.bytes),
      },
      snapshotId: snapshot.id,
    };
  } catch (error) {
    console.warn('[desktop] recovery lookup:', error);
    return { result, snapshotId: null };
  }
}

async function maybeRestoreUntitledRecovery(): Promise<void> {
  if (documentIO.kind !== 'desktop' || documentSession.current.hasDocument || startupFilesReceived) {
    return;
  }

  try {
    const snapshots = await documentIO.listRecoverySnapshots();
    const snapshotMeta = snapshots.find((snapshot) => !snapshot.filePath);
    if (!snapshotMeta) {
      return;
    }

    const shouldRecover = await showConfirm(
      '자동 복구 문서 열기',
      `저장되지 않은 자동 복구 문서가 있습니다.\n\n문서: ${snapshotMeta.fileName}\n시각: ${snapshotMeta.updatedAt}\n\n복구할까요?`,
    );
    if (!shouldRecover) {
      return;
    }

    const snapshot = await documentIO.readRecoverySnapshot(snapshotMeta.id);
    if (!snapshot) {
      return;
    }

    const docInfo = wasm.loadDocument(
      toUint8Array(snapshot.bytes),
      snapshot.fileName,
      snapshot.filePath ?? '',
    );
    /*
    await initializeDocument(
      docInfo,
      `${snapshot.fileName} 자동 복구본`,
      snapshot.fileName,
      snapshot.filePath ?? '',
      snapshot.id,
    );
    */
    await initializeDocument(
      docInfo,
      `${snapshot.fileName} recovered draft`,
      snapshot.fileName,
      snapshot.filePath ?? '',
      snapshot.id,
    );
    await rememberCurrentDocument();
  } catch (error) {
    console.warn('[desktop] untitled recovery:', error);
  }
}

async function persistRecoverySnapshot(): Promise<void> {
  const session = documentSession.current;
  if (documentIO.kind !== 'desktop' || recoveryWriteInFlight) return;
  if (!session.hasDocument || session.isProtected || !session.dirty) return;

  recoveryWriteInFlight = true;
  try {
    const meta = await documentIO.writeRecoverySnapshot({
      snapshotId: session.recoverySnapshotId ?? undefined,
      fileName: session.fileName || `document.${session.saveFormat}`,
      filePath: session.filePath || undefined,
      format: session.saveFormat,
      bytes: wasm.save(session.saveFormat),
    });
    if (meta) {
      documentSession.setRecoverySnapshotId(meta.id);
    }
  } catch (error) {
    console.warn('[desktop] autosave recovery snapshot:', error);
  } finally {
    recoveryWriteInFlight = false;
  }
}

async function rememberCurrentDocument(): Promise<void> {
  if (!documentSession.current.hasDocument) return;
  await documentIO.rememberRecentDocument(
    createRecentDocument(documentSession.current, documentIO.kind),
  );
}

async function loadOpenResult(result: OpenDocumentResult): Promise<void> {
  /*
  const docInfo = wasm.loadDocument(result.data, result.fileName, result.filePath ?? '');
  await initializeDocument(docInfo, `${result.fileName} · ${docInfo.pageCount}페이지`, result.fileName, result.filePath ?? '');
  await rememberCurrentDocument();
  */
  const recovered = await maybeRecoverOpenResult(result);
  const docInfo = wasm.loadDocument(
    recovered.result.data,
    recovered.result.fileName,
    recovered.result.filePath ?? '',
  );
  await initializeDocument(
    docInfo,
    `${recovered.result.fileName} (${docInfo.pageCount} pages)`,
    recovered.result.fileName,
    recovered.result.filePath ?? '',
    recovered.snapshotId,
  );
  await rememberCurrentDocument();
}

async function initialize(): Promise<void> {
  const msg = sbMessage();
  e2eReady = false;
  try {
    msg.textContent = '웹폰트 로딩 중...';
    await loadWebFonts([]);  // CSS @font-face 등록 + CRITICAL 폰트만 로드
    msg.textContent = 'WASM 로딩 중...';
    await wasm.initialize();
    msg.textContent = 'HWP 파일을 선택해주세요.';

    const container = document.getElementById('scroll-container')!;
    canvasView = new CanvasView(container, wasm, eventBus);

    // 눈금자 초기화
    ruler = new Ruler(
      document.getElementById('h-ruler') as HTMLCanvasElement,
      document.getElementById('v-ruler') as HTMLCanvasElement,
      container,
      eventBus,
      wasm,
      canvasView.getVirtualScroll(),
      canvasView.getViewportManager(),
    );

    inputHandler = new InputHandler(
      container, wasm, eventBus,
      canvasView.getVirtualScroll(),
      canvasView.getViewportManager(),
    );

    toolbar = new Toolbar(document.getElementById('style-bar')!, wasm, eventBus, dispatcher);
    toolbar.setEnabled(false);

    // InputHandler에 커맨드 디스패처 및 컨텍스트 메뉴 주입
    inputHandler.setDispatcher(dispatcher);
    inputHandler.setContextMenu(new ContextMenu(dispatcher, registry));
    inputHandler.setCommandPalette(new CommandPalette(registry, dispatcher));
    inputHandler.setCellSelectionRenderer(
      new CellSelectionRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setTableObjectRenderer(
      new TableObjectRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setTableResizeRenderer(
      new TableResizeRenderer(container, canvasView.getVirtualScroll()),
    );
    inputHandler.setPictureObjectRenderer(
      new TableObjectRenderer(container, canvasView.getVirtualScroll(), true),
    );

    new MenuBar(document.getElementById('menu-bar')!, eventBus, dispatcher);

    // 툴바 내 data-cmd 버튼 클릭 → 커맨드 디스패치
    document.querySelectorAll('.tb-btn[data-cmd]').forEach(btn => {
      btn.addEventListener('mousedown', (e) => {
        e.preventDefault();
        const cmd = (btn as HTMLElement).dataset.cmd;
        if (cmd) dispatcher.dispatch(cmd, { anchorEl: btn as HTMLElement });
      });
    });

    // 스플릿 버튼 드롭다운 메뉴
    document.querySelectorAll('.tb-split').forEach(split => {
      const arrow = split.querySelector('.tb-split-arrow');
      if (arrow) {
        arrow.addEventListener('mousedown', (e) => {
          e.preventDefault();
          e.stopPropagation();
          // 다른 열린 메뉴 닫기
          document.querySelectorAll('.tb-split.open').forEach(s => {
            if (s !== split) s.classList.remove('open');
          });
          split.classList.toggle('open');
        });
      }
      split.querySelectorAll('.tb-split-item[data-cmd]').forEach(item => {
        item.addEventListener('mousedown', (e) => {
          e.preventDefault();
          split.classList.remove('open');
          const cmd = (item as HTMLElement).dataset.cmd;
          if (cmd) dispatcher.dispatch(cmd, { anchorEl: item as HTMLElement });
        });
      });
    });
    // 외부 클릭 시 스플릿 메뉴 닫기
    document.addEventListener('mousedown', () => {
      document.querySelectorAll('.tb-split.open').forEach(s => s.classList.remove('open'));
    });

    setupFileInput();
    setupZoomControls();
    setupEventListeners();
    setupDocumentIOListeners();
    setupGlobalShortcuts();
    void refreshDesktopAssociationStatus();
    if (recoveryTimer === null) {
      recoveryTimer = window.setInterval(() => {
        void persistRecoverySnapshot();
      }, 30000);
    }
    if (!new URLSearchParams(window.location.search).has('url')) {
      window.setTimeout(() => {
        void maybeRestoreUntitledRecovery();
      }, 600);
    }
    loadFromUrlParam();

    // E2E 테스트용 전역 노출 (개발 모드 전용)
    if (import.meta.env.DEV) {
      (window as any).__inputHandler = inputHandler;
      (window as any).__canvasView = canvasView;
    }
    e2eReady = true;
  } catch (error) {
    msg.textContent = `WASM 초기화 실패: ${error}`;
    console.error('[main] WASM 초기화 실패:', error);
  }
}

/**
 * 전역 단축키 핸들러 — InputHandler.active 여부와 무관하게 동작해야 하는 단축키.
 * 예: 문서 미로드 상태에서도 Alt+N(새 문서), Ctrl+O(열기) 등.
 */
function setupGlobalShortcuts(): void {
  document.addEventListener('keydown', (e) => {
    // input/textarea 등 편집 가능 요소 내부에서는 무시
    const target = e.target as HTMLElement;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return;
    // InputHandler가 활성 상태이면 자체 처리에 맡김
    if (inputHandler?.isActive()) return;

    const ctrlOrMeta = e.ctrlKey || e.metaKey;

    // Alt+N / Alt+ㅜ → 새 문서 (문서 미로드 상태에서도 동작)
    if (e.altKey && !ctrlOrMeta && !e.shiftKey) {
      if (e.key === 'n' || e.key === 'N' || e.key === 'ㅜ') {
        e.preventDefault();
        dispatcher.dispatch('file:new-doc');
        return;
      }
    }
  }, false);
}

function setupFileInput(): void {
  const fileInput = document.getElementById('file-input') as HTMLInputElement;

  fileInput.addEventListener('change', async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const name = file.name.toLowerCase();
    if (!name.endsWith('.hwp') && !name.endsWith('.hwpx')) {
      alert('HWP/HWPX 파일만 지원합니다.');
      return;
    }
    const okayToOpen = await confirmDiscardIfDirty(
      '문서 열기',
      '저장하지 않은 변경 사항이 있습니다.\n현재 문서를 닫고 다른 문서를 열까요?',
    );
    if (!okayToOpen) return;
    await loadFile(file);
  });

  // 문서 전체에서 브라우저 기본 드롭 동작 방지 (파일 열기/다운로드 방지)
  document.addEventListener('dragover', (e) => e.preventDefault());
  document.addEventListener('drop', (e) => e.preventDefault());

  // 드래그 앤 드롭 지원 (scroll-container 영역)
  const container = document.getElementById('scroll-container')!;
  container.addEventListener('dragover', (e) => {
    e.preventDefault();
    container.classList.add('drag-over');
  });
  container.addEventListener('dragleave', () => {
    container.classList.remove('drag-over');
  });
  container.addEventListener('drop', async (e) => {
    e.preventDefault();
    container.classList.remove('drag-over');
    const file = e.dataTransfer?.files[0];
    if (!file) return;
    const dropName = file.name.toLowerCase();
    if (!dropName.endsWith('.hwp') && !dropName.endsWith('.hwpx')) {
      alert('HWP/HWPX 파일만 지원합니다.');
      return;
    }
    const okayToDropOpen = await confirmDiscardIfDirty(
      '문서 열기',
      '저장하지 않은 변경 사항이 있습니다.\n현재 문서를 닫고 다른 문서를 열까요?',
    );
    if (!okayToDropOpen) return;
    await loadFile(file);
  });
}

function setupDocumentIOListeners(): void {
  eventBus.on('request-open-document', async () => {
    const okayToOpen = await confirmDiscardIfDirty(
      '문서 열기',
      '저장하지 않은 변경 사항이 있습니다.\n현재 문서를 닫고 다른 문서를 열까요?',
    );
    if (!okayToOpen) return;

    if (documentIO.kind === 'desktop') {
      const result = await documentIO.openWithPicker();
      if (result) {
        await loadOpenResult(result);
      }
      return;
    }

    document.getElementById('file-input')?.click();
  });

  documentIO.onOpenFiles(async (files) => {
    startupFilesReceived = files.length > 0;
    const first = files[0];
    if (!first) return;
    const okayToOpen = await confirmDiscardIfDirty(
      '문서 열기',
      '저장하지 않은 변경 사항이 있습니다.\n현재 문서를 닫고 다른 문서를 열까요?',
    );
    if (!okayToOpen) return;
    await loadOpenResult(first);
  });
}

function setupZoomControls(): void {
  if (!canvasView) return;
  const vm = canvasView.getViewportManager();

  document.getElementById('sb-zoom-in')!.addEventListener('click', () => {
    vm.setZoom(vm.getZoom() + 0.1);
  });
  document.getElementById('sb-zoom-out')!.addEventListener('click', () => {
    vm.setZoom(vm.getZoom() - 0.1);
  });

  // 폭 맞춤: 용지 폭에 맞게 줌 조절
  document.getElementById('sb-zoom-fit-width')!.addEventListener('click', () => {
    if (wasm.pageCount === 0) return;
    const container = document.getElementById('scroll-container')!;
    const containerWidth = container.clientWidth - 40; // 좌우 여백 제외
    const pageInfo = wasm.getPageInfo(0);
    // pageInfo.width는 이미 px 단위 (96dpi 기준)
    const zoom = containerWidth / pageInfo.width;
    console.log(`[zoom-fit-width] container=${containerWidth} page=${pageInfo.width} zoom=${zoom.toFixed(3)}`);
    vm.setZoom(Math.max(0.1, Math.min(zoom, 4.0)));
  });

  // 쪽 맞춤: 한 페이지 전체가 보이도록 줌 조절
  document.getElementById('sb-zoom-fit')!.addEventListener('click', () => {
    if (wasm.pageCount === 0) return;
    const container = document.getElementById('scroll-container')!;
    const containerWidth = container.clientWidth - 40;
    const containerHeight = container.clientHeight - 40;
    const pageInfo = wasm.getPageInfo(0);
    // pageInfo.width/height는 이미 px 단위 (96dpi 기준)
    const zoomW = containerWidth / pageInfo.width;
    const zoomH = containerHeight / pageInfo.height;
    console.log(`[zoom-fit-page] containerW=${containerWidth} containerH=${containerHeight} pageW=${pageInfo.width} pageH=${pageInfo.height} zoomW=${zoomW.toFixed(3)} zoomH=${zoomH.toFixed(3)}`);
    vm.setZoom(Math.max(0.1, Math.min(zoomW, zoomH, 4.0)));
  });

  // 모바일: 줌 값 클릭 → 100% 토글
  document.getElementById('sb-zoom-val')!.addEventListener('click', () => {
    const currentZoom = vm.getZoom();
    if (Math.abs(currentZoom - 1.0) < 0.05) {
      // 현재 100% → 쪽 맞춤으로 전환
      document.getElementById('sb-zoom-fit')!.click();
    } else {
      // 현재 쪽 맞춤/기타 → 100%로 전환
      vm.setZoom(1.0);
    }
  });

  document.addEventListener('keydown', (e) => {
    if (!e.ctrlKey && !e.metaKey) return;
    if (e.key === '=' || e.key === '+') {
      e.preventDefault();
      vm.setZoom(vm.getZoom() + 0.1);
    } else if (e.key === '-') {
      e.preventDefault();
      vm.setZoom(vm.getZoom() - 0.1);
    } else if (e.key === '0') {
      e.preventDefault();
      vm.setZoom(1.0);
    }
  });
}

let totalSections = 1;

function setupEventListeners(): void {
  eventBus.on('document-changed', () => {
    if (!documentSession.current.hasDocument || documentSession.current.isProtected) return;
    documentSession.markDirty();
    wasm.markDirty();
    syncSessionFromWasm();
    eventBus.emit('command-state-changed');
  });

  eventBus.on('current-page-changed', (page, _total) => {
    const pageIdx = page as number;
    sbPage().textContent = `${pageIdx + 1} / ${_total} 쪽`;

    // 구역 정보: 현재 페이지의 sectionIndex로 갱신
    if (wasm.pageCount > 0) {
      try {
        const pageInfo = wasm.getPageInfo(pageIdx);
        sbSection().textContent = `구역: ${pageInfo.sectionIndex + 1} / ${totalSections}`;
      } catch { /* 무시 */ }
    }
  });

  eventBus.on('zoom-level-display', (zoom) => {
    sbZoomVal().textContent = `${Math.round((zoom as number) * 100)}%`;
  });

  // 삽입/수정 모드 토글
  eventBus.on('insert-mode-changed', (insertMode) => {
    document.getElementById('sb-mode')!.textContent = (insertMode as boolean) ? '삽입' : '수정';
  });

  // 필드 정보 표시
  const sbField = document.getElementById('sb-field');
  eventBus.on('field-info-changed', (info) => {
    if (!sbField) return;
    const fi = info as { fieldId: number; fieldType: string; guideName?: string } | null;
    if (fi) {
      const label = fi.guideName || `#${fi.fieldId}`;
      sbField.textContent = `[누름틀] ${label}`;
      sbField.style.display = '';
    } else {
      sbField.textContent = '';
      sbField.style.display = 'none';
    }
  });

  // 개체 선택 시 회전/대칭 버튼 그룹 표시/숨김
  const rotateGroup = document.querySelector('.tb-rotate-group') as HTMLElement | null;
  if (rotateGroup) {
    eventBus.on('picture-object-selection-changed', (selected) => {
      rotateGroup.style.display = (selected as boolean) ? '' : 'none';
    });
  }

  // 머리말/꼬리말 편집 모드 시 도구상자 전환 + 본문 dimming
  const hfGroup = document.querySelector('.tb-headerfooter-group') as HTMLElement | null;
  const hfLabel = hfGroup?.querySelector('.tb-hf-label') as HTMLElement | null;
  const defaultTbGroups = document.querySelectorAll('#icon-toolbar > .tb-group:not(.tb-headerfooter-group):not(.tb-rotate-group), #icon-toolbar > .tb-sep');
  const scrollContainer = document.getElementById('scroll-container');
  const styleBar = document.getElementById('style-bar');

  eventBus.on('headerFooterModeChanged', (mode) => {
    const isActive = (mode as string) !== 'none';
    // 도구상자 전환
    if (hfGroup) {
      hfGroup.style.display = isActive ? '' : 'none';
    }
    if (hfLabel) {
      hfLabel.textContent = (mode as string) === 'header' ? '머리말' : (mode as string) === 'footer' ? '꼬리말' : '';
    }
    defaultTbGroups.forEach((el) => {
      (el as HTMLElement).style.display = isActive ? 'none' : '';
    });
    // 서식 도구 모음은 머리말/꼬리말 편집 시에도 유지 (문단/글자 모양 설정 필요)
    // 본문 dimming
    if (scrollContainer) {
      if (isActive) {
        scrollContainer.classList.add('hf-editing');
      } else {
        scrollContainer.classList.remove('hf-editing');
      }
    }
  });
}

/** 문서 초기화 공통 시퀀스 (loadFile, createNewDocument 양쪽에서 사용) */
async function initializeDocument(
  docInfo: DocumentInfo,
  displayName: string,
  fileName = wasm.fileName,
  filePath = wasm.filePath,
  recoverySnapshotId: string | null = null,
): Promise<void> {
  const msg = sbMessage();
  try {
    console.log('[initDoc] 1. 폰트 로딩 시작');
    if (docInfo.fontsUsed?.length) {
      await loadWebFonts(docInfo.fontsUsed, (loaded, total) => {
        msg.textContent = `폰트 로딩 중... (${loaded}/${total})`;
      });
    }
    console.log('[initDoc] 2. 폰트 로딩 완료');
    documentSession.load(fileName, filePath, docInfo, wasm.getDocumentCapabilities());
    documentSession.setRecoverySnapshotId(recoverySnapshotId);
    documentSession.setReports(
      wasm.getCompatibilityReport().issues,
      wasm.getFontSubstitutionReport().items,
    );
    renderSessionBanner();
    msg.textContent = displayName;
    totalSections = docInfo.sectionCount ?? 1;
    sbSection().textContent = `구역: 1 / ${totalSections}`;
    console.log('[initDoc] 3. inputHandler deactivate');
    inputHandler?.deactivate();
    console.log('[initDoc] 4. canvasView loadDocument');
    canvasView?.loadDocument();
    console.log('[initDoc] 5. toolbar setEnabled');
    toolbar?.setEnabled(!documentSession.current.isProtected);
    console.log('[initDoc] 6. toolbar initStyleDropdown');
    toolbar?.initStyleDropdown();
    console.log('[initDoc] 7. inputHandler activateWithCaretPosition');
    if (documentSession.current.isProtected) {
      inputHandler?.deactivate();
    } else {
      inputHandler?.activateWithCaretPosition();
    }
    eventBus.emit('command-state-changed');
    console.log('[initDoc] 8. 완료');
  } catch (error) {
    console.error('[initDoc] 오류:', error);
    if (window.innerWidth < 768) alert(`초기화 오류: ${error}`);
  }
}

async function loadFile(file: File): Promise<void> {
  const msg = sbMessage();
  try {
    msg.textContent = '파일 로딩 중...';
    const startTime = performance.now();
    const data = new Uint8Array(await file.arrayBuffer());
    const docInfo = wasm.loadDocument(data, file.name, '');
    const elapsed = performance.now() - startTime;
    await initializeDocument(docInfo, `${file.name} — ${docInfo.pageCount}페이지 (${elapsed.toFixed(1)}ms)`);
  } catch (error) {
    const errMsg = `파일 로드 실패: ${error}`;
    msg.textContent = errMsg;
    console.error('[main] 파일 로드 실패:', error);
    // 모바일에서 상태 메시지가 숨겨질 수 있으므로 alert으로도 표시
    if (window.innerWidth < 768) alert(errMsg);
  }
}

async function createNewDocument(): Promise<void> {
  const msg = sbMessage();
  try {
    msg.textContent = '새 문서 생성 중...';
    const docInfo = wasm.createNewDocument();
    await initializeDocument(docInfo, `새 문서.hwp — ${docInfo.pageCount}페이지`);
  } catch (error) {
    msg.textContent = `새 문서 생성 실패: ${error}`;
    console.error('[main] 새 문서 생성 실패:', error);
  }
}

// 커맨드에서 새 문서 생성 호출
eventBus.on('create-new-document', () => { createNewDocument(); });

// 수식 더블클릭 → 수식 편집 대화상자
eventBus.on('equation-edit-request', () => {
  dispatcher.dispatch('insert:equation-edit');
});

/**
 * URL 파라미터(?url=)로 전달된 HWP 파일을 자동 로드한다.
 * Chrome 확장 프로그램에서 뷰어 탭을 열 때 사용.
 */
async function loadFromUrlParam(): Promise<void> {
  const params = new URLSearchParams(window.location.search);
  const fileUrl = params.get('url');
  if (!fileUrl) return;

  const fileName = params.get('filename') || fileUrl.split('/').pop()?.split('?')[0] || 'document.hwp';
  const msg = sbMessage();

  try {
    msg.textContent = '파일 로딩 중...';
    console.log(`[loadFromUrlParam] ${fileUrl}`);

    let response: Response;

    // Chrome 확장 환경: Service Worker를 통한 CORS 우회 fetch
    if (typeof chrome !== 'undefined' && chrome.runtime?.sendMessage) {
      try {
        response = await fetch(fileUrl);
      } catch {
        // 직접 fetch 실패 시 Service Worker 프록시
        const result = await chrome.runtime.sendMessage({ type: 'fetch-file', url: fileUrl });
        if (result.error) throw new Error(result.error);
        const data = new Uint8Array(result.data);
        const docInfo = wasm.loadDocument(data, fileName);
        await initializeDocument(docInfo, `${fileName} — ${docInfo.pageCount}페이지`);
        return;
      }
    } else {
      response = await fetch(fileUrl);
    }

    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const buffer = await response.arrayBuffer();
    const data = new Uint8Array(buffer);
    const docInfo = wasm.loadDocument(data, fileName);
    await initializeDocument(docInfo, `${fileName} — ${docInfo.pageCount}페이지`);
  } catch (error) {
    const errMsg = `파일 로드 실패: ${error}`;
    msg.textContent = errMsg;
    console.error('[loadFromUrlParam]', error);
  }
}

installE2EBridge();
initialize();

window.addEventListener('beforeunload', (event) => {
  const session = documentSession.current;
  if (!session.hasDocument || !session.dirty || session.isProtected) {
    return;
  }

  event.preventDefault();
  event.returnValue = '';
});

// ── iframe 연동 API (postMessage) ──
// 부모 페이지에서 postMessage로 에디터를 제어할 수 있다.
// 요청: { type: 'rhwp-request', id, method, params }
// 응답: { type: 'rhwp-response', id, result?, error? }
window.addEventListener('message', async (e) => {
  const msg = e.data;
  if (!msg || typeof msg !== 'object') return;

  // 기존 hwpctl-load 호환
  if (msg.type === 'hwpctl-load' && msg.data) {
    try {
      const bytes = new Uint8Array(msg.data);
      const docInfo = wasm.loadDocument(bytes, msg.fileName || 'document.hwp');
      await initializeDocument(docInfo, `${msg.fileName || 'document'} — ${docInfo.pageCount}페이지`);
      e.source?.postMessage({ type: 'rhwp-response', id: msg.id, result: { pageCount: docInfo.pageCount } }, { targetOrigin: '*' });
    } catch (err: any) {
      e.source?.postMessage({ type: 'rhwp-response', id: msg.id, error: err.message || String(err) }, { targetOrigin: '*' });
    }
    return;
  }

  // rhwp-request: 범용 API
  if (msg.type !== 'rhwp-request' || !msg.method) return;
  const { id, method, params } = msg;
  const reply = (result?: any, error?: string) => {
    e.source?.postMessage({ type: 'rhwp-response', id, result, error }, { targetOrigin: '*' });
  };

  try {
    switch (method) {
      case 'loadFile': {
        const bytes = new Uint8Array(params.data);
        const docInfo = wasm.loadDocument(bytes, params.fileName || 'document.hwp');
        await initializeDocument(docInfo, `${params.fileName || 'document'} — ${docInfo.pageCount}페이지`);
        reply({ pageCount: docInfo.pageCount });
        break;
      }
      case 'pageCount':
        reply(wasm.pageCount);
        break;
      case 'getPageSvg':
        reply(wasm.renderPageSvg(params.page ?? 0));
        break;
      case 'ready':
        reply(true);
        break;
      default:
        reply(undefined, `Unknown method: ${method}`);
    }
  } catch (err: any) {
    reply(undefined, err.message || String(err));
  }
});
