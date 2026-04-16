import type { CommandDef, CommandServices } from '../types';
import type { DocumentFormat } from '@/core/types';
import { PageSetupDialog } from '@/ui/page-setup-dialog';
import { AboutDialog } from '@/ui/about-dialog';
import { showConfirm } from '@/ui/confirm-dialog';
import { createRecentDocument } from '@/core/document-session';

async function saveDocument(
  services: CommandServices,
  mode: 'save' | 'save-as',
  format: DocumentFormat,
): Promise<void> {
  const fileName = services.session.current.fileName || `document.${format}`;
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
  await services.documentIO.rememberRecentDocument(
    createRecentDocument(services.session.current, services.documentIO.kind),
  );
  services.eventBus.emit('command-state-changed');
}

function getSaveMode(services: CommandServices): 'save' | 'save-as' {
  return services.wasm.isNewDocument ? 'save-as' : 'save';
}

export const fileCommands: CommandDef[] = [
  {
    id: 'file:new-doc',
    label: '?덈줈 留뚮뱾湲?',
    icon: 'icon-new-doc',
    shortcutLabel: 'Alt+N',
    canExecute: () => true,
    async execute(services) {
      const ctx = services.getContext();
      if (ctx.hasDocument) {
        const ok = await showConfirm(
          '?덈줈 留뚮뱾湲?',
          '?꾩옱 臾몄꽌瑜??リ퀬 ??臾몄꽌瑜?留뚮뱶?쒓쿋?듬땲源?\n??ν븯吏 ?딆? ?댁슜? ?щ씪吏묐땲??',
        );
        if (!ok) return;
      }
      services.eventBus.emit('create-new-document');
    },
  },
  {
    id: 'file:open',
    label: '?닿린',
    execute(services) {
      services.eventBus.emit('request-open-document');
    },
  },
  {
    id: 'file:save',
    label: '???',
    icon: 'icon-save',
    shortcutLabel: 'Ctrl+S',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    async execute(services) {
      try {
        await saveDocument(services, getSaveMode(services), services.getContext().saveFormat);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:save] ????ㅽ뙣:', msg);
        alert(`?뚯씪 ??μ뿉 ?ㅽ뙣?덉뒿?덈떎:\n${msg}`);
      }
    },
  },
  {
    id: 'file:save-as',
    label: '?ㅻⅨ ?대쫫?쇰줈 ???',
    canExecute: (ctx) => ctx.hasDocument && ctx.canSave,
    async execute(services, params) {
      try {
        const requestedFormat = params?.format as DocumentFormat | undefined;
        await saveDocument(
          services,
          'save-as',
          requestedFormat && requestedFormat !== 'unknown'
            ? requestedFormat
            : services.getContext().saveFormat,
        );
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:save-as] ????ㅽ뙣:', msg);
        alert(`?뚯씪 ??μ뿉 ?ㅽ뙣?덉뒿?덈떎:\n${msg}`);
      }
    },
  },
  {
    id: 'file:page-setup',
    label: '?몄쭛 ?⑹?',
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
    label: '?몄뇙',
    icon: 'icon-print',
    shortcutLabel: 'Ctrl+P',
    canExecute: (ctx) => ctx.hasDocument,
    async execute(services) {
      const wasm = services.wasm;
      const pageCount = wasm.pageCount;
      if (pageCount === 0) return;

      const statusEl = document.getElementById('sb-message');
      const origStatus = statusEl?.textContent || '';

      try {
        const svgPages: string[] = [];
        for (let i = 0; i < pageCount; i++) {
          if (statusEl) statusEl.textContent = `?몄뇙 以鍮?以?.. (${i + 1}/${pageCount})`;
          svgPages.push(wasm.renderPageSvg(i));
          if (i % 5 === 0) await new Promise((resolve) => setTimeout(resolve, 0));
        }

        const pageInfo = wasm.getPageInfo(0);
        const widthMm = Math.round(pageInfo.width * 25.4 / 96);
        const heightMm = Math.round(pageInfo.height * 25.4 / 96);

        const printWin = window.open('', '_blank');
        if (!printWin) {
          alert('?앹뾽??李⑤떒?섏뿀?듬땲?? ?앹뾽 ?덉슜 ???ㅼ떆 ?쒕룄?댁＜?몄슂.');
          return;
        }

        printWin.document.write(`<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>${wasm.fileName} ???몄뇙</title>
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
  <button id="print-btn">?몄뇙</button>
  <button id="close-btn" style="background:#475569">?リ린</button>
  <span>${wasm.fileName} ??${pageCount}?섏씠吏</span>
</div>
${svgPages.map((svg) => `<div class="page">${svg}</div>`).join('\n')}

</body>
</html>`);
        printWin.document.close();

        printWin.document.getElementById('print-btn')?.addEventListener('click', () => {
          printWin.print();
        });
        printWin.document.getElementById('close-btn')?.addEventListener('click', () => {
          printWin.close();
        });

        if (statusEl) statusEl.textContent = origStatus;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[file:print]', msg);
        if (statusEl) statusEl.textContent = `?몄뇙 ?ㅽ뙣: ${msg}`;
      }
    },
  },
  {
    id: 'file:about',
    label: '?쒗뭹 ?뺣낫',
    icon: 'icon-help',
    execute() {
      new AboutDialog().show();
    },
  },
];
