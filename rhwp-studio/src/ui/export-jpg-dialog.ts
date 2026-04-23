import type { JpgExportScope } from '@/core/types';
import { ModalDialog } from './dialog';

interface ExportJpgDialogOptions {
  enableAllPages: boolean;
}

class ExportJpgDialog extends ModalDialog {
  private resolve!: (value: JpgExportScope | null) => void;
  private currentRadio!: HTMLInputElement;
  private allRadio!: HTMLInputElement;

  constructor(private options: ExportJpgDialogOptions) {
    super('JPG로 저장', 360);
  }

  protected createBody(): HTMLElement {
    const body = document.createElement('div');
    body.style.padding = '16px 20px';

    const description = document.createElement('p');
    description.textContent = '저장 범위를 선택하세요.';
    description.style.margin = '0 0 12px';
    description.style.fontSize = '13px';
    body.appendChild(description);

    const currentLabel = document.createElement('label');
    currentLabel.style.display = 'flex';
    currentLabel.style.alignItems = 'center';
    currentLabel.style.gap = '8px';
    currentLabel.style.marginBottom = '10px';
    this.currentRadio = document.createElement('input');
    this.currentRadio.type = 'radio';
    this.currentRadio.name = 'jpg-export-scope';
    this.currentRadio.value = 'current';
    this.currentRadio.checked = true;
    currentLabel.appendChild(this.currentRadio);
    currentLabel.appendChild(document.createTextNode('현재 쪽만 저장'));
    body.appendChild(currentLabel);

    const allLabel = document.createElement('label');
    allLabel.style.display = 'flex';
    allLabel.style.alignItems = 'center';
    allLabel.style.gap = '8px';
    this.allRadio = document.createElement('input');
    this.allRadio.type = 'radio';
    this.allRadio.name = 'jpg-export-scope';
    this.allRadio.value = 'all';
    this.allRadio.disabled = !this.options.enableAllPages;
    allLabel.appendChild(this.allRadio);
    allLabel.appendChild(document.createTextNode('전체 쪽을 개별 JPG로 저장'));
    body.appendChild(allLabel);

    const allHint = document.createElement('p');
    allHint.style.margin = '8px 0 0 24px';
    allHint.style.fontSize = '12px';
    allHint.style.color = '#6b7280';
    allHint.textContent = this.options.enableAllPages
      ? '전체 쪽 JPG 저장은 페이지별 파일로 저장됩니다.'
      : '전체 쪽 JPG 저장은 현재 데스크톱 앱에서만 지원합니다.';
    body.appendChild(allHint);

    return body;
  }

  protected onConfirm(): void {
    this.resolve(this.allRadio.checked ? 'all' : 'current');
  }

  override hide(): void {
    this.resolve(null);
    super.hide();
  }

  showAsync(): Promise<JpgExportScope | null> {
    return new Promise((resolve) => {
      let done = false;
      this.resolve = (value) => {
        if (done) return;
        done = true;
        resolve(value);
      };
      super.show();
      requestAnimationFrame(() => {
        this.currentRadio.focus();
      });
    });
  }
}

export function showJpgExportDialog(
  options: ExportJpgDialogOptions = { enableAllPages: true },
): Promise<JpgExportScope | null> {
  return new ExportJpgDialog(options).showAsync();
}
