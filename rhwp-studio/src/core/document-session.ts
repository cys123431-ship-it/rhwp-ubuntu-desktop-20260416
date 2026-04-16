import type { DocumentCapabilities, DocumentFormat, DocumentInfo, DocumentSession, RecentDocument } from './types';

const EMPTY_SESSION: DocumentSession = {
  hasDocument: false,
  fileName: '',
  filePath: '',
  sourceFormat: 'unknown',
  saveFormat: 'hwp',
  editMode: 'editable-safe',
  isProtected: false,
  dirty: false,
  encrypted: false,
  distribution: false,
  blockers: [],
  warnings: [],
};

function inferFormatFromName(fileName: string): DocumentFormat {
  const lower = fileName.toLowerCase();
  if (lower.endsWith('.hwpx')) return 'hwpx';
  if (lower.endsWith('.hwp')) return 'hwp';
  return 'unknown';
}

export class DocumentSessionStore {
  private session: DocumentSession = { ...EMPTY_SESSION };

  get current(): Readonly<DocumentSession> {
    return this.session;
  }

  reset(): DocumentSession {
    this.session = { ...EMPTY_SESSION };
    return this.session;
  }

  load(
    fileName: string,
    filePath: string,
    docInfo: DocumentInfo,
    capabilities: DocumentCapabilities,
  ): DocumentSession {
    const sourceFormat = capabilities.sourceFormat ?? docInfo.sourceFormat ?? inferFormatFromName(fileName);
    const saveFormat = capabilities.preferredSaveFormat ?? (sourceFormat === 'unknown' ? 'hwp' : sourceFormat);

    this.session = {
      hasDocument: true,
      fileName,
      filePath: filePath || capabilities.filePath || '',
      sourceFormat,
      saveFormat,
      editMode: capabilities.editMode,
      isProtected: capabilities.isProtected,
      dirty: capabilities.dirty ?? docInfo.dirty,
      encrypted: capabilities.encrypted ?? docInfo.encrypted,
      distribution: capabilities.distribution ?? docInfo.distribution,
      blockers: [...capabilities.blockers],
      warnings: [...capabilities.warnings],
    };

    return this.session;
  }

  syncCapabilities(capabilities: DocumentCapabilities): DocumentSession {
    if (!this.session.hasDocument) return this.session;
    this.session = {
      ...this.session,
      filePath: capabilities.filePath || this.session.filePath,
      saveFormat: capabilities.preferredSaveFormat,
      editMode: capabilities.editMode,
      isProtected: capabilities.isProtected,
      dirty: capabilities.dirty,
      encrypted: capabilities.encrypted,
      distribution: capabilities.distribution,
      blockers: [...capabilities.blockers],
      warnings: [...capabilities.warnings],
    };
    return this.session;
  }

  updateFile(fileName: string, filePath = ''): DocumentSession {
    this.session = {
      ...this.session,
      fileName,
      filePath,
    };
    return this.session;
  }

  applySaveResult(result: { fileName: string; filePath?: string; format: DocumentFormat }): DocumentSession {
    this.session = {
      ...this.session,
      fileName: result.fileName,
      filePath: result.filePath ?? '',
      sourceFormat: result.format,
      saveFormat: result.format,
      dirty: false,
    };
    return this.session;
  }

  markDirty(): DocumentSession {
    if (!this.session.hasDocument || this.session.isProtected || this.session.dirty) {
      return this.session;
    }

    this.session = {
      ...this.session,
      dirty: true,
    };
    return this.session;
  }

  clearDirty(): DocumentSession {
    if (!this.session.hasDocument || !this.session.dirty) return this.session;
    this.session = {
      ...this.session,
      dirty: false,
    };
    return this.session;
  }
}

export function createRecentDocument(session: DocumentSession, source: 'web' | 'desktop'): RecentDocument {
  return {
    name: session.fileName,
    path: session.filePath || undefined,
    format: session.sourceFormat,
    source,
    lastOpenedAt: new Date().toISOString(),
  };
}

