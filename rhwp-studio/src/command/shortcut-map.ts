import type { CommandRegistry } from './registry';

export interface ShortcutStroke {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export type ShortcutContext =
  | 'any'
  | 'body'
  | 'table'
  | 'table-selection'
  | 'table-object'
  | 'picture-object'
  | 'header-footer'
  | 'footnote';

export interface ShortcutBinding {
  commandId: string;
  label: string;
  sequence: ShortcutStroke[];
  contexts?: ShortcutContext[];
}

export interface PendingShortcutChord {
  prefix: ShortcutStroke;
  commandIds: string[];
}

type ShortcutHost = {
  cursor?: {
    isInCell?: () => boolean;
    isInTextBox?: () => boolean;
    isInHeaderFooter?: () => boolean;
    isInFootnote?: () => boolean;
    isInTableObjectSelection?: () => boolean;
    isInPictureObjectSelection?: () => boolean;
    getCellSelectionPhase?: () => number;
  };
};

const single = (
  commandId: string,
  label: string,
  stroke: ShortcutStroke,
  contexts?: ShortcutContext[],
): ShortcutBinding => ({
  commandId,
  label,
  sequence: [stroke],
  contexts,
});

const chord = (
  commandId: string,
  label: string,
  prefix: ShortcutStroke,
  second: ShortcutStroke,
  contexts?: ShortcutContext[],
): ShortcutBinding => ({
  commandId,
  label,
  sequence: [prefix, second],
  contexts,
});

const ctrl = (key: string, extra: Omit<ShortcutStroke, 'key' | 'ctrl'> = {}): ShortcutStroke => ({
  key,
  ctrl: true,
  ...extra,
});

const alt = (key: string, extra: Omit<ShortcutStroke, 'key' | 'alt'> = {}): ShortcutStroke => ({
  key,
  alt: true,
  ...extra,
});

const plain = (key: string, extra: Omit<ShortcutStroke, 'key'> = {}): ShortcutStroke => ({
  key,
  ...extra,
});

export const hancomShortcuts: ShortcutBinding[] = [
  single('file:new-doc', 'Alt+N', alt('n')),
  single('file:open', 'Ctrl+O', ctrl('o')),
  single('file:open', 'Alt+O', alt('o')),
  single('file:save', 'Ctrl+S', ctrl('s')),
  single('file:save', 'Alt+S', alt('s')),
  single('file:save-as', 'Ctrl+Shift+S', ctrl('s', { shift: true })),
  single('file:save-as', 'Alt+V', alt('v')),
  single('file:print', 'Ctrl+P', ctrl('p')),
  single('file:print', 'Alt+P', alt('p')),

  single('edit:undo', 'Ctrl+Z', ctrl('z')),
  single('edit:redo', 'Ctrl+Shift+Z', ctrl('z', { shift: true })),
  single('edit:redo', 'Ctrl+Y', ctrl('y')),
  single('edit:cut', 'Ctrl+X', ctrl('x')),
  single('edit:copy', 'Ctrl+C', ctrl('c')),
  single('edit:paste', 'Ctrl+V', ctrl('v')),
  single('edit:format-copy', 'Alt+C', alt('c')),
  single('edit:delete', 'Ctrl+E', ctrl('e')),
  single('edit:select-all', 'Ctrl+A', ctrl('a')),
  chord('edit:find', 'Ctrl+Q,F', ctrl('q'), plain('f')),
  single('edit:find-replace', 'Ctrl+F2', ctrl('f2')),
  single('edit:find-again', 'Ctrl+L', ctrl('l')),
  single('edit:goto', 'Alt+G', alt('g')),

  single('format:bold', 'Ctrl+B', ctrl('b')),
  single('format:italic', 'Ctrl+I', ctrl('i')),
  single('format:underline', 'Ctrl+U', ctrl('u')),
  single('format:char-shape', 'Alt+L', alt('l')),
  single('format:para-shape', 'Alt+T', alt('t')),
  chord('format:para-num-shape', 'Ctrl+K,N', ctrl('k'), plain('n')),
  single('format:font-size-increase', 'Ctrl+]', ctrl(']')),
  single('format:font-size-increase', 'Alt+Shift+E', alt('e', { shift: true })),
  single('format:font-size-decrease', 'Ctrl+[', ctrl('[')),
  single('format:font-size-decrease', 'Alt+Shift+R', alt('r', { shift: true })),
  single('format:line-spacing-decrease', 'Alt+Shift+A', alt('a', { shift: true })),
  single('format:line-spacing-increase', 'Alt+Shift+Z', alt('z', { shift: true })),
  single('format:align-left', 'Ctrl+Shift+L', ctrl('l', { shift: true })),
  single('format:align-justify', 'Ctrl+Shift+M', ctrl('m', { shift: true })),
  single('format:align-right', 'Alt+Shift+H', alt('h', { shift: true })),
  single('format:align-center', 'Alt+Shift+C', alt('c', { shift: true })),
  single('format:align-distribute', 'Alt+Shift+D', alt('d', { shift: true })),
  single('format:style-dialog', 'F6', plain('f6')),

  single('insert:symbols', 'Alt+F10', alt('f10')),
  chord('insert:bookmark', 'Ctrl+K,B', ctrl('k'), plain('b')),
  chord('table:create', 'Ctrl+N,T', ctrl('n'), plain('t'), ['body']),
  chord('table:formula', 'Ctrl+N,F', ctrl('n'), plain('f'), ['table']),

  single('page:break', 'Ctrl+Enter', ctrl('enter'), ['body']),
  single('page:break', 'Ctrl+J', ctrl('j'), ['body']),
  single('page:column-break', 'Ctrl+Shift+Enter', ctrl('enter', { shift: true }), ['body']),
  chord('page:hide', 'Ctrl+N,S', ctrl('n'), plain('s'), ['body']),

  single('table:insert-row-below', 'Ctrl+Enter', ctrl('enter'), ['table']),
  single('table:insert-col-left', 'Alt+Insert', alt('insert'), ['table']),
  single('table:delete-col', 'Alt+Delete', alt('delete'), ['table']),
  single('table:cell-split', 'S', plain('s'), ['table-selection']),
  single('table:cell-merge', 'M', plain('m'), ['table-selection']),

  chord('field:edit', 'Ctrl+N,K', ctrl('n'), plain('k')),

  chord('view:ctrl-mark', 'Ctrl+G,C', ctrl('g'), plain('c')),
  chord('view:para-mark', 'Ctrl+G,T', ctrl('g'), plain('t')),
  chord('view:zoom-fit-page', 'Ctrl+G,P', ctrl('g'), plain('p')),
  chord('view:zoom-fit-width', 'Ctrl+G,W', ctrl('g'), plain('w')),
  chord('view:zoom-100', 'Ctrl+G,Q', ctrl('g'), plain('q')),
  single('view:zoom-in', 'Shift+Num +', plain('+', { shift: true })),
  single('view:zoom-out', 'Shift+Num -', plain('-', { shift: true })),
  chord('view:border-transparent', 'Alt+V,T', alt('v'), plain('t')),
];

const shortcutLabels = new Map<string, string[]>();
for (const binding of hancomShortcuts) {
  const labels = shortcutLabels.get(binding.commandId) ?? [];
  if (!labels.includes(binding.label)) {
    labels.push(binding.label);
  }
  shortcutLabels.set(binding.commandId, labels);
}

function sameStroke(a: ShortcutStroke, b: ShortcutStroke): boolean {
  return (
    a.key === b.key
    && (a.ctrl ?? false) === (b.ctrl ?? false)
    && (a.shift ?? false) === (b.shift ?? false)
    && (a.alt ?? false) === (b.alt ?? false)
  );
}

function eventMatchesStroke(e: KeyboardEvent, stroke: ShortcutStroke): boolean {
  const ctrlOrMeta = e.ctrlKey || e.metaKey;
  return (
    e.key.toLowerCase() === stroke.key
    && ((stroke.ctrl ?? false) === ctrlOrMeta)
    && ((stroke.shift ?? false) === e.shiftKey)
    && ((stroke.alt ?? false) === e.altKey)
  );
}

function matchesContext(host: ShortcutHost, contexts?: ShortcutContext[]): boolean {
  if (!contexts || contexts.length === 0 || contexts.includes('any')) return true;

  const cursor = host.cursor;
  const inTable = Boolean(cursor?.isInCell?.() && !cursor?.isInTextBox?.());
  const inTableSelection = (cursor?.getCellSelectionPhase?.() ?? 0) > 0;
  const inHeaderFooter = Boolean(cursor?.isInHeaderFooter?.());
  const inFootnote = Boolean(cursor?.isInFootnote?.());
  const inTableObject = Boolean(cursor?.isInTableObjectSelection?.());
  const inPictureObject = Boolean(cursor?.isInPictureObjectSelection?.());
  const inBody =
    !inTable
    && !inHeaderFooter
    && !inFootnote
    && !inTableObject
    && !inPictureObject;

  return contexts.some((context) => {
    switch (context) {
      case 'body':
        return inBody;
      case 'table':
        return inTable;
      case 'table-selection':
        return inTableSelection;
      case 'table-object':
        return inTableObject;
      case 'picture-object':
        return inPictureObject;
      case 'header-footer':
        return inHeaderFooter;
      case 'footnote':
        return inFootnote;
      default:
        return true;
    }
  });
}

export function matchShortcut(e: KeyboardEvent, host: ShortcutHost): string | null {
  for (const binding of hancomShortcuts) {
    if (binding.sequence.length !== 1) continue;
    if (!matchesContext(host, binding.contexts)) continue;
    if (eventMatchesStroke(e, binding.sequence[0])) {
      return binding.commandId;
    }
  }

  return null;
}

export function beginShortcutChord(
  e: KeyboardEvent,
  host: ShortcutHost,
): PendingShortcutChord | null {
  const matches = hancomShortcuts.filter((binding) => (
    binding.sequence.length === 2
    && matchesContext(host, binding.contexts)
    && eventMatchesStroke(e, binding.sequence[0])
  ));

  if (matches.length === 0) return null;

  return {
    prefix: matches[0].sequence[0],
    commandIds: matches.map((binding) => binding.commandId),
  };
}

export function resolveShortcutChord(
  e: KeyboardEvent,
  pending: PendingShortcutChord,
  host: ShortcutHost,
): string | null {
  for (const binding of hancomShortcuts) {
    if (binding.sequence.length !== 2) continue;
    if (!pending.commandIds.includes(binding.commandId)) continue;
    if (!matchesContext(host, binding.contexts)) continue;
    if (sameStroke(binding.sequence[0], pending.prefix) && eventMatchesStroke(e, binding.sequence[1])) {
      return binding.commandId;
    }
  }

  return null;
}

export function getShortcutLabel(commandId: string): string | undefined {
  const labels = shortcutLabels.get(commandId);
  if (!labels || labels.length === 0) return undefined;
  return labels.join(' / ');
}

export function syncShortcutLabels(registry: CommandRegistry, root?: ParentNode | null): void {
  for (const commandId of registry.getAllIds()) {
    const def = registry.get(commandId);
    if (!def) continue;
    (def as { shortcutLabel?: string }).shortcutLabel = getShortcutLabel(commandId);
  }

  if (!root) return;

  const menuItems = root.querySelectorAll<HTMLElement>('.md-item[data-cmd]');
  for (const item of menuItems) {
    const commandId = item.dataset.cmd;
    if (!commandId) continue;

    const label = getShortcutLabel(commandId);
    let shortcutEl = item.querySelector<HTMLElement>('.md-shortcut');

    if (!label) {
      shortcutEl?.remove();
      continue;
    }

    if (!shortcutEl) {
      shortcutEl = document.createElement('span');
      shortcutEl.className = 'md-shortcut';
      item.appendChild(shortcutEl);
    }
    shortcutEl.textContent = label;
  }
}
