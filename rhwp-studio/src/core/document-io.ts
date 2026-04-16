import { showSaveAs } from '@/ui/save-as-dialog';
import type { DocumentFormat, RecentDocument } from './types';

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

export interface RhwpDesktopBridge {
  openDocument?: () => Promise<OpenDocumentResult | null>;
  saveDocument: (request: SaveDocumentRequest) => Promise<SaveDocumentResult | null>;
  getRecentDocuments?: () => Promise<RecentDocument[]>;
  revealInFolder?: (path: string) => Promise<void>;
  onOpenFiles?: (handler: (files: OpenDocumentResult[]) => void) => void;
}

export interface DocumentIO {
  readonly kind: 'web' | 'desktop';
  openWithPicker(): Promise<OpenDocumentResult | null>;
  saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult | null>;
  getRecentDocuments(): Promise<RecentDocument[]>;
  rememberRecentDocument(doc: RecentDocument): Promise<void>;
  revealInFolder(path: string): Promise<void>;
  onOpenFiles(handler: (files: OpenDocumentResult[]) => void): void;
}

function getMimeType(format: DocumentFormat): string {
  return format === 'hwpx' ? 'application/x-hwpx' : 'application/x-hwp';
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
    return this.bridge.openDocument ? this.bridge.openDocument() : null;
  }

  async saveDocument(request: SaveDocumentRequest): Promise<SaveDocumentResult | null> {
    return this.bridge.saveDocument(request);
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

  async revealInFolder(path: string): Promise<void> {
    if (this.bridge.revealInFolder) {
      await this.bridge.revealInFolder(path);
    }
  }

  onOpenFiles(handler: (files: OpenDocumentResult[]) => void): void {
    this.bridge.onOpenFiles?.(handler);
  }
}

function createTauriBridge(): RhwpDesktopBridge | null {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) return null;

  return {
    openDocument: () => invoke<OpenDocumentResult | null>('open_document'),
    saveDocument: (request) => invoke<SaveDocumentResult | null>('save_document', { request }),
    getRecentDocuments: () => invoke<RecentDocument[]>('get_recent_documents'),
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
  if (window.__RHWP_DESKTOP__) {
    return new DesktopDocumentIO(window.__RHWP_DESKTOP__);
  }

  const tauriBridge = createTauriBridge();
  if (tauriBridge) {
    return new DesktopDocumentIO(tauriBridge);
  }
  return new WebDocumentIO();
}
