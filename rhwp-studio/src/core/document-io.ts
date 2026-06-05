import { showSaveAs } from '@/ui/save-as-dialog';
import type {
  DocumentFormat,
  ExportTargetFormat,
  FileAssociationStatus,
  JpgExportScope,
  RecentDocument,
  RecoverySnapshotMeta,
  RecoverySnapshotPayload,
} from './types';

const RECENT_DOCS_KEY = 'rhwp.recent-documents';
const MAX_RECENT_DOCS = 10;

declare global {
  interface FileSystemFileHandle {
    readonly name: string;
    createWritable(): Promise<{
      write(data: Blob | Uint8Array): Promise<void>;
      close(): Promise<void>;
    }>;
  }

  interface Window {
    showSaveFilePicker?: (options?: {
      suggestedName?: string;
      types?: { description: string; accept: Record<string, string[]> }[];
    }) => Promise<FileSystemFileHandle>;
    __RHWP_DESKTOP__?: RhwpDesktopBridge;
    __TAURI__?: {
      core?: {
        invoke: <T>(command: string, payload?: Record<string, unknown>) => Promise<T>;
      };
      event?: {
        listen: (
          event: string,
          handler: (payload: { payload: unknown }) => void,
        ) => Promise<() => void>;
      };
      window?: {
        getCurrentWindow?: () => {
          onCloseRequested: (
            handler: (event: { preventDefault: () => void }) => void | Promise<void>,
          ) => Promise<() => void>;
          destroy: () => Promise<void>;
        };
      };
    };
  }
}

export interface OpenDocumentResult {
  fileName: string;
  filePath?: string;
  data: Uint8Array;
}

export interface SaveDocumentRequest {
  mode: 'save' | 'save-as';
  fileName: string;
  filePath?: string;
  format: DocumentFormat;
  bytes: Uint8Array;
}

export interface SaveDocumentResult {
  fileName: string;
  filePath?: string;
  format: DocumentFormat;
}

export interface SaveBinaryRequest {
  suggestedName: string;
  filePath?: string;
  format: ExportTargetFormat;
  mimeType: string;
  bytes: Uint8Array;
}

export interface SaveBinaryResult {
  fileName: string;
  filePath?: string;
  format: ExportTargetFormat;
}

export interface ExportDocumentRequest {
  format: ExportTargetFormat;
  fileName: string;
  filePath?: string;
  jpgScope?: JpgExportScope;
}

export interface WriteRecoverySnapshotRequest {
  snapshotId?: string;
  fileName: string;
  filePath?: string;
  format: DocumentFormat;
  bytes: Uint8Array;
}

export interface RhwpDesktopBridge {
  openDocument?: () => Promise<OpenDocumentResult | null>;
  openDocumentAtPath?: (path: string) => Promise<OpenDocumentResult>;
  consumeStartupFiles?: () => Promise<OpenDocumentResult[]>;
  saveDocument: (request: SaveDocumentRequest) => Promise<SaveDocumentResult | null>;
  saveBinaryFile?: (request: SaveBinaryRequest) => Promise<SaveBinaryResult | null>;
  pickExportDirectory?: () => Promise<string | null>;
  exportPdfFromSvgs?: (svgs: string[]) => Promise<Uint8Array>;
  getRecentDocuments?: () => Promise<RecentDocument[]>;
  getFileAssociationStatus?: () => Promise<FileAssociationStatus>;
  setDefaultFileAssociation?: () => Promise<FileAssociationStatus>;
  listRecoverySnapshots?: () => Promise<RecoverySnapshotMeta[]>;
  readRecoverySnapshot?: (snapshotId: string) => Promise<RecoverySnapshotPayload>;
  writeRecoverySnapshot?: (request: WriteRecoverySnapshotRequest) => Promise<RecoverySnapshotMeta>;
  deleteRecoverySnapshot?: (snapshotId: string) => Promise<void>;
  revealInFolder?: (path: string) => Promise<void>;
  onOpenFiles?: (handler: (files: OpenDocumentResult[]) => void) => void;
}

export interface DocumentIO {
  readonly kind: 'web' | 'desktop';
  openWithPicker(): Promise<OpenDocumentResult | null>;
  openAtPath(path: string): Promise<OpenDocumentResult | null>;
  consumeStartupFiles(): Promise<OpenDocumentResult[]>;
  saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult | null>;
  saveBinaryFile(request: SaveBinaryRequest): Promise<SaveBinaryResult | null>;
  pickExportDirectory(): Promise<string | null>;
  exportPdfFromSvgs(svgs: string[]): Promise<Uint8Array | null>;
  getRecentDocuments(): Promise<RecentDocument[]>;
  rememberRecentDocument(doc: RecentDocument): Promise<void>;
  getFileAssociationStatus(): Promise<FileAssociationStatus | null>;
  setDefaultFileAssociation(): Promise<FileAssociationStatus | null>;
  listRecoverySnapshots(): Promise<RecoverySnapshotMeta[]>;
  readRecoverySnapshot(snapshotId: string): Promise<RecoverySnapshotPayload | null>;
  writeRecoverySnapshot(request: WriteRecoverySnapshotRequest): Promise<RecoverySnapshotMeta | null>;
  deleteRecoverySnapshot(snapshotId: string): Promise<void>;
  revealInFolder(path: string): Promise<void>;
  onOpenFiles(handler: (files: OpenDocumentResult[]) => void): void;
}

function normalizeOpenDocumentResult(result: OpenDocumentResult | null): OpenDocumentResult | null {
  if (!result) {
    return null;
  }

  return {
    ...result,
    data: result.data instanceof Uint8Array ? result.data : new Uint8Array(result.data),
  };
}

function normalizeOpenDocumentResults(results: OpenDocumentResult[] | null | undefined): OpenDocumentResult[] {
  if (!Array.isArray(results)) {
    return [];
  }

  return results
    .map((result) => normalizeOpenDocumentResult(result))
    .filter((result): result is OpenDocumentResult => result !== null);
}

function getMimeType(format: DocumentFormat): string {
  return format === 'hwpx' ? 'application/vnd.hancom.hwpx' : 'application/x-hwp';
}

function getBinaryExtension(format: ExportTargetFormat): string {
  switch (format) {
    case 'hwpx':
      return '.hwpx';
    case 'pdf':
      return '.pdf';
    case 'docx':
      return '.docx';
    case 'jpg':
      return '.jpg';
    case 'hwp':
    default:
      return '.hwp';
  }
}

function getBinaryMimeType(format: ExportTargetFormat): string {
  switch (format) {
    case 'hwpx':
      return 'application/vnd.hancom.hwpx';
    case 'pdf':
      return 'application/pdf';
    case 'docx':
      return 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
    case 'jpg':
      return 'image/jpeg';
    case 'hwp':
    default:
      return 'application/x-hwp';
  }
}

function normalizeBinaryName(fileName: string, format: ExportTargetFormat): string {
  const targetExt = getBinaryExtension(format);
  if (fileName.toLowerCase().endsWith(targetExt)) {
    return fileName;
  }
  return fileName.replace(/\.(hwp|hwpx|pdf|docx|jpg|jpeg)$/i, '') + targetExt;
}

function ensureExtension(fileName: string, format: DocumentFormat): string {
  const targetExt = format === 'hwpx' ? '.hwpx' : '.hwp';
  if (fileName.toLowerCase().endsWith(targetExt)) {
    return fileName;
  }
  return fileName.replace(/\.(hwp|hwpx)$/i, '') + targetExt;
}

function loadRecentDocuments(): RecentDocument[] {
  try {
    const raw = localStorage.getItem(RECENT_DOCS_KEY);
    if (!raw) return [];
    const docs = JSON.parse(raw) as RecentDocument[];
    return Array.isArray(docs) ? docs : [];
  } catch {
    return [];
  }
}

function saveRecentDocuments(docs: RecentDocument[]): void {
  localStorage.setItem(RECENT_DOCS_KEY, JSON.stringify(docs.slice(0, MAX_RECENT_DOCS)));
}

class WebDocumentIO implements DocumentIO {
  readonly kind = 'web' as const;
  private currentHandle: FileSystemFileHandle | null = null;

  async openWithPicker(): Promise<OpenDocumentResult | null> {
    return null;
  }

  async consumeStartupFiles(): Promise<OpenDocumentResult[]> {
    return [];
  }

  async openAtPath(_path: string): Promise<OpenDocumentResult | null> {
    return null;
  }

  async saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult | null> {
    const fileName = ensureExtension(request.fileName, request.format);
    const blob = new Blob([request.bytes as unknown as BlobPart], { type: getMimeType(request.format) });

    if (request.mode === 'save' && this.currentHandle) {
      const writable = await this.currentHandle.createWritable();
      await writable.write(blob);
      await writable.close();
      return {
        fileName: this.currentHandle.name,
        format: request.format,
      };
    }

    if (window.showSaveFilePicker) {
      try {
        const handle = await window.showSaveFilePicker({
          suggestedName: fileName,
          types: [{
            description: request.format === 'hwpx' ? 'HWPX document' : 'HWP document',
            accept: { [getMimeType(request.format)]: [request.format === 'hwpx' ? '.hwpx' : '.hwp'] },
          }],
        });
        const writable = await handle.createWritable();
        await writable.write(blob);
        await writable.close();
        this.currentHandle = handle;
        return {
          fileName: handle.name,
          format: request.format,
        };
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') {
          return null;
        }
        console.warn('[DocumentIO] save picker fallback:', error);
      }
    }

    const suggested = request.mode === 'save-as'
      ? await showSaveAs(fileName, request.format)
      : fileName;
    if (!suggested) return null;

    const downloadName = ensureExtension(suggested, request.format);
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = downloadName;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);

    return {
      fileName: downloadName,
      format: request.format,
    };
  }

  async saveBinaryFile(request: SaveBinaryRequest): Promise<SaveBinaryResult | null> {
    const fileName = normalizeBinaryName(request.suggestedName, request.format);
    const blob = new Blob([request.bytes as unknown as BlobPart], {
      type: request.mimeType || getBinaryMimeType(request.format),
    });

    if (window.showSaveFilePicker) {
      try {
        const handle = await window.showSaveFilePicker({
          suggestedName: fileName,
          types: [{
            description: `${request.format.toUpperCase()} file`,
            accept: {
              [request.mimeType || getBinaryMimeType(request.format)]: [getBinaryExtension(request.format)],
            },
          }],
        });
        const writable = await handle.createWritable();
        await writable.write(blob);
        await writable.close();
        return {
          fileName: handle.name,
          format: request.format,
        };
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') {
          return null;
        }
        console.warn('[DocumentIO] binary save picker fallback:', error);
      }
    }

    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = fileName;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);

    return {
      fileName,
      format: request.format,
    };
  }

  async pickExportDirectory(): Promise<string | null> {
    return null;
  }

  async exportPdfFromSvgs(_svgs: string[]): Promise<Uint8Array | null> {
    return null;
  }

  async getRecentDocuments(): Promise<RecentDocument[]> {
    return loadRecentDocuments();
  }

  async rememberRecentDocument(doc: RecentDocument): Promise<void> {
    const current = loadRecentDocuments();
    const deduped = current.filter((item) => {
      if (doc.path && item.path) return item.path !== doc.path;
      return item.name !== doc.name;
    });
    deduped.unshift(doc);
    saveRecentDocuments(deduped);
  }

  async getFileAssociationStatus(): Promise<FileAssociationStatus | null> {
    return null;
  }

  async setDefaultFileAssociation(): Promise<FileAssociationStatus | null> {
    return null;
  }

  async listRecoverySnapshots(): Promise<RecoverySnapshotMeta[]> {
    return [];
  }

  async readRecoverySnapshot(_snapshotId: string): Promise<RecoverySnapshotPayload | null> {
    return null;
  }

  async writeRecoverySnapshot(_request: WriteRecoverySnapshotRequest): Promise<RecoverySnapshotMeta | null> {
    return null;
  }

  async deleteRecoverySnapshot(_snapshotId: string): Promise<void> {
    return;
  }

  async revealInFolder(_path: string): Promise<void> {
    return;
  }

  onOpenFiles(_handler: (files: OpenDocumentResult[]) => void): void {
    return;
  }
}

class DesktopDocumentIO implements DocumentIO {
  readonly kind = 'desktop' as const;

  constructor(private bridge: RhwpDesktopBridge) {}

  async openWithPicker(): Promise<OpenDocumentResult | null> {
    if (!this.bridge.openDocument) {
      return null;
    }

    return normalizeOpenDocumentResult(await this.bridge.openDocument());
  }

  async consumeStartupFiles(): Promise<OpenDocumentResult[]> {
    if (this.bridge.consumeStartupFiles) {
      return normalizeOpenDocumentResults(await this.bridge.consumeStartupFiles());
    }
    return [];
  }

  async openAtPath(path: string): Promise<OpenDocumentResult | null> {
    if (!this.bridge.openDocumentAtPath) {
      return null;
    }

    return normalizeOpenDocumentResult(await this.bridge.openDocumentAtPath(path));
  }

  async saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult | null> {
    return this.bridge.saveDocument(request);
  }

  async saveBinaryFile(request: SaveBinaryRequest): Promise<SaveBinaryResult | null> {
    if (!this.bridge.saveBinaryFile) {
      return null;
    }
    return this.bridge.saveBinaryFile(request);
  }

  async pickExportDirectory(): Promise<string | null> {
    if (!this.bridge.pickExportDirectory) {
      return null;
    }
    return this.bridge.pickExportDirectory();
  }

  async exportPdfFromSvgs(svgs: string[]): Promise<Uint8Array | null> {
    if (!this.bridge.exportPdfFromSvgs) {
      return null;
    }
    const bytes = await this.bridge.exportPdfFromSvgs(svgs);
    return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  }

  async getRecentDocuments(): Promise<RecentDocument[]> {
    if (this.bridge.getRecentDocuments) {
      return this.bridge.getRecentDocuments();
    }
    return loadRecentDocuments();
  }

  async rememberRecentDocument(doc: RecentDocument): Promise<void> {
    const current = loadRecentDocuments();
    const deduped = current.filter((item) => item.path !== doc.path && item.name !== doc.name);
    deduped.unshift(doc);
    saveRecentDocuments(deduped);
  }

  async getFileAssociationStatus(): Promise<FileAssociationStatus | null> {
    if (this.bridge.getFileAssociationStatus) {
      return this.bridge.getFileAssociationStatus();
    }
    return null;
  }

  async setDefaultFileAssociation(): Promise<FileAssociationStatus | null> {
    if (this.bridge.setDefaultFileAssociation) {
      return this.bridge.setDefaultFileAssociation();
    }
    return null;
  }

  async listRecoverySnapshots(): Promise<RecoverySnapshotMeta[]> {
    if (this.bridge.listRecoverySnapshots) {
      return this.bridge.listRecoverySnapshots();
    }
    return [];
  }

  async readRecoverySnapshot(snapshotId: string): Promise<RecoverySnapshotPayload | null> {
    if (this.bridge.readRecoverySnapshot) {
      return this.bridge.readRecoverySnapshot(snapshotId);
    }
    return null;
  }

  async writeRecoverySnapshot(request: WriteRecoverySnapshotRequest): Promise<RecoverySnapshotMeta | null> {
    if (this.bridge.writeRecoverySnapshot) {
      return this.bridge.writeRecoverySnapshot(request);
    }
    return null;
  }

  async deleteRecoverySnapshot(snapshotId: string): Promise<void> {
    if (this.bridge.deleteRecoverySnapshot) {
      await this.bridge.deleteRecoverySnapshot(snapshotId);
    }
  }

  async revealInFolder(path: string): Promise<void> {
    if (this.bridge.revealInFolder) {
      await this.bridge.revealInFolder(path);
    }
  }

  onOpenFiles(handler: (files: OpenDocumentResult[]) => void): void {
    this.bridge.onOpenFiles?.((files) => {
      handler(normalizeOpenDocumentResults(files));
    });
  }
}

function createActiveDesktopDocumentIO(): DesktopDocumentIO | null {
  if (window.__RHWP_DESKTOP__) {
    return new DesktopDocumentIO(window.__RHWP_DESKTOP__);
  }

  const tauriBridge = createTauriBridge();
  if (tauriBridge) {
    return new DesktopDocumentIO(tauriBridge);
  }

  return null;
}

function toIpcBytes(bytes: Uint8Array): number[] {
  return Array.from(bytes);
}

function createTauriBridge(): RhwpDesktopBridge | null {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) return null;

  return {
    openDocument: () => invoke<OpenDocumentResult | null>('open_document'),
    openDocumentAtPath: (path) => invoke<OpenDocumentResult>('open_document_at_path', { path }),
    consumeStartupFiles: () => invoke<OpenDocumentResult[]>('consume_startup_files'),
    saveDocument: (request) => invoke<SaveDocumentResult | null>('save_document', {
      request: {
        ...request,
        bytes: toIpcBytes(request.bytes),
      },
    }),
    saveBinaryFile: (request) => invoke<SaveBinaryResult | null>('save_binary_file', {
      request: {
        ...request,
        bytes: toIpcBytes(request.bytes),
      },
    }),
    pickExportDirectory: () => invoke<string | null>('pick_export_directory'),
    exportPdfFromSvgs: (svgs) => invoke<Uint8Array>('export_pdf_from_svgs', { svgs }),
    getRecentDocuments: () => invoke<RecentDocument[]>('get_recent_documents'),
    getFileAssociationStatus: () => invoke<FileAssociationStatus>('get_file_association_status'),
    setDefaultFileAssociation: () => invoke<FileAssociationStatus>('set_default_file_association'),
    listRecoverySnapshots: () => invoke<RecoverySnapshotMeta[]>('list_recovery_snapshots'),
    readRecoverySnapshot: (snapshotId) => invoke<RecoverySnapshotPayload>('read_recovery_snapshot', { snapshotId }),
    writeRecoverySnapshot: (request) => invoke<RecoverySnapshotMeta>('write_recovery_snapshot', {
      request: {
        ...request,
        bytes: toIpcBytes(request.bytes),
      },
    }),
    deleteRecoverySnapshot: (snapshotId) => invoke<void>('delete_recovery_snapshot', { snapshotId }),
    revealInFolder: (path) => invoke<void>('reveal_in_folder', { path }),
    onOpenFiles: (handler) => {
      void window.__TAURI__?.event?.listen?.('rhwp://open-files', (event) => {
        const files = Array.isArray(event.payload) ? event.payload as OpenDocumentResult[] : [];
        handler(files);
      });
    },
  };
}

export function createDocumentIO(): DocumentIO {
  const webDocumentIO = new WebDocumentIO();

  return {
    get kind() {
      return createActiveDesktopDocumentIO() ? 'desktop' : 'web';
    },
    async openWithPicker() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).openWithPicker();
    },
    async consumeStartupFiles() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).consumeStartupFiles();
    },
    async openAtPath(path) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).openAtPath(path);
    },
    async saveDocument(request) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).saveDocument(request);
    },
    async saveBinaryFile(request) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).saveBinaryFile(request);
    },
    async pickExportDirectory() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).pickExportDirectory();
    },
    async exportPdfFromSvgs(svgs) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).exportPdfFromSvgs(svgs);
    },
    async getRecentDocuments() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).getRecentDocuments();
    },
    async rememberRecentDocument(doc) {
      await (createActiveDesktopDocumentIO() ?? webDocumentIO).rememberRecentDocument(doc);
    },
    async getFileAssociationStatus() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).getFileAssociationStatus();
    },
    async setDefaultFileAssociation() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).setDefaultFileAssociation();
    },
    async listRecoverySnapshots() {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).listRecoverySnapshots();
    },
    async readRecoverySnapshot(snapshotId) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).readRecoverySnapshot(snapshotId);
    },
    async writeRecoverySnapshot(request) {
      return (createActiveDesktopDocumentIO() ?? webDocumentIO).writeRecoverySnapshot(request);
    },
    async deleteRecoverySnapshot(snapshotId) {
      await (createActiveDesktopDocumentIO() ?? webDocumentIO).deleteRecoverySnapshot(snapshotId);
    },
    async revealInFolder(path) {
      await (createActiveDesktopDocumentIO() ?? webDocumentIO).revealInFolder(path);
    },
    onOpenFiles(handler) {
      createActiveDesktopDocumentIO()?.onOpenFiles(handler);
    },
  };
}
