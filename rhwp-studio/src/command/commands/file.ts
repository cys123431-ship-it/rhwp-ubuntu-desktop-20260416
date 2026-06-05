import type { CommandDef, CommandServices } from '../types';
import type { DocumentFormat, ExportTargetFormat } from '@/core/types';
import { PageSetupDialog } from '@/ui/page-setup-dialog';
import { AboutDialog } from '@/ui/about-dialog';
import { showConfirm } from '@/ui/confirm-dialog';
import { showJpgExportDialog } from '@/ui/export-jpg-dialog';
import { createRecentDocument } from '@/core/document-session';

const PDF_MIME = 'application/pdf';
const DOCX_MIME =
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
const JPG_MIME = 'image/jpeg';

function getSuggestedDocumentName(fileName: string, format: DocumentFormat): string {
  const trimmed = fileName.trim();
  if (!trimmed) return `document.${format}`;
  const lower = trimmed.toLowerCase();
  if (lower.endsWith('.hwp') || lower.endsWith('.hwpx')) {
    return trimmed;
  }
  return `${trimmed}.${format}`;
}

function getBaseName(fileName: string, fallback = 'document'): string {
  const trimmed = fileName.trim();
  if (!trimmed) return fallback;
  const dotIndex = trimmed.lastIndexOf('.');
  if (dotIndex <= 0) return trimmed;
  return trimmed.slice(0, dotIndex);
}

function joinFilePath(directory: string, fileName: string): string {
  if (directory.endsWith('\\') || directory.endsWith('/')) {
    return `${directory}${fileName}`;
  }
  const separator = directory.includes('\\') && !directory.includes('/') ? '\\' : '/';
  return `${directory}${separator}${fileName}`;
}

function getBinaryMimeType(format: ExportTargetFormat): string {
  switch (format) {
    case 'pdf':
      return PDF_MIME;
    case 'docx':
      return DOCX_MIME;
    case 'jpg':
      return JPG_MIME;
    case 'hwpx':
      return 'application/vnd.hancom.hwpx';
    case 'hwp':
    default:
      return 'application/x-hwp';
  }
}

function setStatusMessage(message: string): void {
  const statusEl = document.getElementById('sb-message');
  if (statusEl) {
    statusEl.textContent = message;
  }
}

async function saveDocument(
  services: CommandServices,
  mode: 'save' | 'save-as',
  format: DocumentFormat,
): Promise<void> {
  const fileName = getSuggestedDocumentName(
    services.session.current.fileName,
    format,
  );
  const recoverySnapshotId = services.session.current.recoverySnapshotId;
  const bytes = services.wasm.save(format);
  const result = await services.documentIO.saveDocument({
    mode,
    fileName,
    filePath: services.session.current.filePath,
    format,
    bytes,
  });

  if (!result) return;

  services.wasm.fileName = result.fileName;
  services.wasm.filePath = result.filePath ?? '';
  services.wasm.clearDirty();
  services.session.applySaveResult(result);
  if (recoverySnapshotId) {
    await services.documentIO.deleteRecoverySnapshot(recoverySnapshotId);
  }
  await services.documentIO.rememberRecentDocument(
    createRecentDocument(services.session.current, services.documentIO.kind),
  );
  services.eventBus.emit('command-state-changed');
  setStatusMessage(`${result.fileName} 저장 완료`);
}

function getSaveMode(services: CommandServices): 'save' | 'save-as' {
  return services.wasm.isNewDocument ? 'save-as' : 'save';
}

async function saveDerivedFormat(
  services: CommandServices,
  format: 'hwp' | 'hwpx',
): Promise<void> {
  const currentName = services.session.current.fileName || `document.${format}`;
  const suggestedName = `${getBaseName(currentName)}.${format}`;
  const bytes = format === 'hwpx'
    ? services.wasm.exportHwpxLossy()
    : services.wasm.exportHwp();
  const result = await services.documentIO.saveBinaryFile({
    suggestedName,
    format,
    mimeType: getBinaryMimeType(format),
    bytes,
  });
  if (result) {
    setStatusMessage(`${result.fileName} 저장 완료`);
  }
}

async function exportPdf(services: CommandServices): Promise<void> {
  if (services.documentIO.kind !== 'desktop') {
    alert('PDF 저장은 현재 데스크톱 앱에서만 지원합니다.');
    return;
  }

  const statusEl = document.getElementById('sb-message');
  const originalStatus = statusEl?.textContent || '';

  try {
    const pageCount = services.wasm.pageCount;
    if (pageCount === 0) return;

    const svgPages: string[] = [];
    for (let index = 0; index < pageCount; index += 1) {
      if (statusEl) {
        statusEl.textContent = `PDF 생성 중... (${index + 1}/${pageCount})`;
      }
      svgPages.push(services.wasm.renderPageSvg(index));
      if (index % 4 === 0) {
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }

    const pdfBytes = await services.documentIO.exportPdfFromSvgs(svgPages);
    if (!pdfBytes) {
      throw new Error('PDF 변환 엔진을 사용할 수 없습니다.');
    }

    const result = await services.documentIO.saveBinaryFile({
      suggestedName: `${getBaseName(services.session.current.fileName)}.pdf`,
      format: 'pdf',
      mimeType: PDF_MIME,
      bytes: pdfBytes,
    });
    if (result) {
      setStatusMessage(`${result.fileName} 내보내기 완료`);
    }
  } finally {
    if (statusEl && statusEl.textContent === originalStatus) {
      statusEl.textContent = originalStatus;
    }
  }
}

async function exportDocx(services: CommandServices): Promise<void> {
  const bytes = services.wasm.exportDocx();
  const result = await services.documentIO.saveBinaryFile({
    suggestedName: `${getBaseName(services.session.current.fileName)}.docx`,
    format: 'docx',
    mimeType: DOCX_MIME,
    bytes,
  });
  if (result) {
    setStatusMessage(`${result.fileName} 내보내기 완료`);
  }
}

async function blobToBytes(blob: Blob): Promise<Uint8Array> {
  return new Uint8Array(await blob.arrayBuffer());
}

async function renderSvgToJpegBytes(
  svg: string,
  width: number,
  height: number,
): Promise<Uint8Array> {
  const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  try {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error('JPG 변환용 SVG 이미지를 불러오지 못했습니다.'));
      img.src = url;
    });

    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.ceil(width));
    canvas.height = Math.max(1, Math.ceil(height));
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('캔버스 컨텍스트를 초기화하지 못했습니다.');
    }

    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(image, 0, 0, canvas.width, canvas.height);

    const jpegBlob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((result) => {
        if (result) {
          resolve(result);
        } else {
          reject(new Error('JPG 블롭 생성에 실패했습니다.'));
        }
      }, JPG_MIME, 0.92);
    });

    return blobToBytes(jpegBlob);
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function exportCurrentPageJpg(services: CommandServices): Promise<void> {
  const pageIndex = Math.max(0, Math.min(
    services.getCurrentPageIndex(),
    Math.max(services.wasm.pageCount - 1, 0),
  ));
  const pageInfo = services.wasm.getPageInfo(pageIndex);
  const svg = services.wasm.renderPageSvg(pageIndex);
  const bytes = await renderSvgToJpegBytes(svg, pageInfo.width, pageInfo.height);

  const result = await services.documentIO.saveBinaryFile({
    suggestedName: `${getBaseName(services.session.current.fileName)}.jpg`,
    format: 'jpg',
    mimeType: JPG_MIME,
    bytes,
  });
  if (result) {
    setStatusMessage(`${result.fileName} 내보내기 완료`);
  }
}

async function exportAllPagesJpg(services: CommandServices): Promise<void> {
  if (services.documentIO.kind !== 'desktop') {
    alert('전체 쪽 JPG 저장은 현재 데스크톱 앱에서만 지원합니다.');
    return;
  }

  const directory = await services.documentIO.pickExportDirectory();
  if (!directory) return;

  const statusEl = document.getElementById('sb-message');
  const originalStatus = statusEl?.textContent || '';
  const baseName = getBaseName(services.session.current.fileName);

  try {
    for (let pageIndex = 0; pageIndex < services.wasm.pageCount; pageIndex += 1) {
      if (statusEl) {
        statusEl.textContent = `JPG 저장 중... (${pageIndex + 1}/${services.wasm.pageCount})`;
      }
      const pageInfo = services.wasm.getPageInfo(pageIndex);
      const svg = services.wasm.renderPageSvg(pageIndex);
      const bytes = await renderSvgToJpegBytes(svg, pageInfo.width, pageInfo.height);
      const fileName = `${baseName}_p${String(pageIndex + 1).padStart(3, '0')}.jpg`;
      await services.documentIO.saveBinaryFile({
        suggestedName: fileName,
        filePath: joinFilePath(directory, fileName),
        format: 'jpg',
        mimeType: JPG_MIME,
        bytes,
      });
      if (pageIndex % 3 === 0) {
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }
    setStatusMessage(`${services.wasm.pageCount}개 JPG 페이지 내보내기 완료`);
  } finally {
    if (statusEl && statusEl.textContent === originalStatus) {
      statusEl.textContent = originalStatus;
    }
  }
}

async function exportJpg(services: CommandServices): Promise<void> {
  const scope = await showJpgExportDialog({
    enableAllPages: services.documentIO.kind === 'desktop',
  });
  if (!scope) return;
  if (scope === 'current') {
    await exportCurrentPageJpg(services);
    return;
  }
  await exportAllPagesJpg(services);
}

async function runSaveCommand(
  services: CommandServices,
  mode: 'save' | 'save-as',
  format: DocumentFormat,
): Promise<void> {
  try {
    await saveDocument(services, mode, format);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`[file:${mode}] 저장 실패`, message);
    alert(`문서를 저장하지 못했습니다.\n${message}`);
  }
}

async function runExportCommand(
  label: string,
  callback: () => Promise<void>,
): Promise<void> {
  try {
    await callback();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`[${label}] 내보내기 실패`, message);
    alert(`${label} 작업을 완료하지 못했습니다.\n${message}`);
  }
}

export const fileCommands: CommandDef[] = [
  {
    id: 'file:new-doc',
    label: '새 문서',
    icon: 'icon-new-doc',
    shortcutLabel: 'Alt+N',
    canExecute: () => true,
    async execute(services) {
      const ctx = services.getContext();
      if (ctx.hasDocument) {
        const ok = await showConfirm(
          '새 문서',
          '현재 문서를 닫고 새 문서를 만듭니다.\n저장하지 않은 변경 사항은 사라질 수 있습니다.',
        );
        if (!ok) return;
      }
      services.eventBus.emit('create-new-document');
    },
  },
  {
    id: 'file:open',
    label: '열기',
    execute(services) {
      services.eventBus.emit('request-open-document');
    },
  },
  {
    id: 'file:save',
    label: '저장',
    icon: 'icon-save',
    shortcutLabel: 'Ctrl+S',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    execute(services) {
      void runSaveCommand(services, getSaveMode(services), services.getContext().saveFormat);
    },
  },
  {
    id: 'file:save-as',
    label: '다른 이름으로 저장',
    shortcutLabel: 'Ctrl+Shift+S',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    execute(services, params) {
      const requestedFormat = params?.format as DocumentFormat | undefined;
      const format =
        requestedFormat && requestedFormat !== 'unknown'
          ? requestedFormat
          : services.getContext().saveFormat;
      void runSaveCommand(services, 'save-as', format);
    },
  },
  {
    id: 'file:export-hwp',
    label: 'HWP로 저장',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    execute(services) {
      void runExportCommand('HWP 저장', () => saveDerivedFormat(services, 'hwp'));
    },
  },
  {
    id: 'file:export-hwpx',
    label: 'HWPX로 저장',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    execute(services) {
      void runExportCommand('HWPX 저장', () => saveDerivedFormat(services, 'hwpx'));
    },
  },
  {
    id: 'file:export-pdf',
    label: 'PDF로 저장',
    canExecute: (ctx) => ctx.hasDocument && ctx.canExportPdf,
    execute(services) {
      void runExportCommand('PDF 저장', () => exportPdf(services));
    },
  },
  {
    id: 'file:export-docx',
    label: 'Word로 저장',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      void runExportCommand('Word 저장', () => exportDocx(services));
    },
  },
  {
    id: 'file:export-jpg',
    label: 'JPG로 저장',
    canExecute: (ctx) => ctx.hasDocument,
    execute(services) {
      void runExportCommand('JPG 저장', () => exportJpg(services));
    },
  },
  {
    id: 'file:page-setup',
    label: '편집 용지',
    icon: 'icon-page-setup',
    shortcutLabel: 'F7',
    canExecute: (ctx) => ctx.hasDocument && ctx.isEditable,
    execute(services) {
      const dialog = new PageSetupDialog(services.wasm, services.eventBus, 0);
      dialog.show();
    },
  },
  {
    id: 'file:print',
    label: '인쇄',
    icon: 'icon-print',
    shortcutLabel: 'Ctrl+P',
    canExecute: (ctx) => ctx.hasDocument,
    async execute(services) {
      const wasm = services.wasm;
      const pageCount = wasm.pageCount;
      if (pageCount === 0) return;

      const statusEl = document.getElementById('sb-message');
      const originalStatus = statusEl?.textContent || '';

      try {
        const svgPages: string[] = [];
        for (let index = 0; index < pageCount; index += 1) {
          if (statusEl) {
            statusEl.textContent = `인쇄 화면 준비 중... (${index + 1}/${pageCount})`;
          }
          svgPages.push(wasm.renderPageSvg(index));
          if (index % 5 === 0) {
            await new Promise((resolve) => setTimeout(resolve, 0));
          }
        }

        const pageInfo = wasm.getPageInfo(0);
        const widthMm = Math.round((pageInfo.width * 25.4) / 96);
        const heightMm = Math.round((pageInfo.height * 25.4) / 96);

        const printWindow = window.open('', '_blank');
        if (!printWindow) {
          alert('팝업 창을 열 수 없습니다. 브라우저 또는 앱의 팝업 차단 설정을 확인해 주세요.');
          return;
        }

        printWindow.document.write(`<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>${wasm.fileName} 인쇄</title>
<style>
  @page { size: ${widthMm}mm ${heightMm}mm; margin: 0; }
  * { margin: 0; padding: 0; }
  body { background: #fff; }
  .page { page-break-after: always; width: ${widthMm}mm; height: ${heightMm}mm; overflow: hidden; }
  .page:last-child { page-break-after: auto; }
  .page svg { width: 100%; height: 100%; }
  @media screen {
    body { background: #e5e7eb; display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 16px; }
    .page { background: #fff; box-shadow: 0 2px 8px rgba(0,0,0,0.15); }
    .print-bar { position: fixed; top: 0; left: 0; right: 0; background: #1e293b; color: #fff; padding: 8px 16px; display: flex; align-items: center; gap: 12px; font: 14px sans-serif; z-index: 100; }
    .print-bar button { padding: 6px 16px; background: #2563eb; color: #fff; border: none; border-radius: 4px; cursor: pointer; font-size: 14px; }
    .print-bar button:hover { background: #1d4ed8; }
    body { padding-top: 56px; }
  }
  @media print { .print-bar { display: none; } }
</style>
</head>
<body>
<div class="print-bar">
  <button id="print-btn">인쇄</button>
  <button id="close-btn" style="background:#475569">닫기</button>
  <span>${wasm.fileName} · ${pageCount}쪽</span>
</div>
${svgPages.map((svg) => `<div class="page">${svg}</div>`).join('\n')}
</body>
</html>`);
        printWindow.document.close();

        printWindow.document.getElementById('print-btn')?.addEventListener('click', () => {
          printWindow.print();
        });
        printWindow.document.getElementById('close-btn')?.addEventListener('click', () => {
          printWindow.close();
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error('[file:print]', message);
        if (statusEl) {
          statusEl.textContent = `인쇄 준비 실패: ${message}`;
        }
      } finally {
        if (statusEl) statusEl.textContent = originalStatus;
      }
    },
  },
  {
    id: 'file:about',
    label: '제품 정보',
    icon: 'icon-help',
    execute() {
      new AboutDialog().show();
    },
  },
];
