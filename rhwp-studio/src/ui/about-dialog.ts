/**
 * 제품 정보 / 라이선스 대화상자.
 *
 * 공개 HWP 문서 형식을 참고한 호환 구현임을 고지하고,
 * 데스크톱 설치본에서 사용자가 현재 수정 사항을 바로 확인할 수 있게 표시한다.
 */
import { ModalDialog } from './dialog';

/** 주요 오픈소스 의존성 라이선스 정보 */
const THIRD_PARTY_LICENSES = [
  { name: 'wasm-bindgen', license: 'MIT / Apache-2.0' },
  { name: 'web-sys', license: 'MIT / Apache-2.0' },
  { name: 'js-sys', license: 'MIT / Apache-2.0' },
  { name: 'cfb', license: 'MIT' },
  { name: 'flate2', license: 'MIT / Apache-2.0' },
  { name: 'byteorder', license: 'MIT / Unlicense' },
  { name: 'base64', license: 'MIT / Apache-2.0' },
  { name: 'console_error_panic_hook', license: 'MIT / Apache-2.0' },
];

const DESKTOP_RELEASE_VERSION = '0.1.7';

const RELEASE_HIGHLIGHTS = [
  '앱 아이콘과 GitHub README 로고를 새 금속 한글 심볼 아이콘으로 교체',
  'Ctrl+A/드래그 블록 선택 시 화면 하이라이트가 실제 선택 범위 끝 줄까지 표시되도록 수정',
  '선택 표시를 줄 정보 추정값이 아니라 실제 렌더된 글자 조각(TextRun) 기준으로 계산',
  '블록 선택 후 글자 크기/서식 변경 시 논리 선택과 시각 표시가 일치하는지 E2E 검증 추가',
  '한글 기본 단축키와 도구 상자 개선 사항은 v0.1.5 기준 변경을 포함',
  'Windows current-user 설치본 및 .hwp/.hwpx 파일 연결 확인 대상',
];

const TRACKED_LIMITATIONS = [
  '남은 한컴 호환 기능은 기능/단축키 추적표 기준으로 계속 검증합니다. 한컴 자산이나 전용 폰트 재배포는 포함하지 않습니다.',
];

export class AboutDialog extends ModalDialog {
  constructor() {
    super('제품 정보', 540);
  }

  protected createBody(): HTMLElement {
    const body = document.createElement('div');
    body.className = 'about-body';

    // 제품 영문명
    const titleEn = document.createElement('div');
    titleEn.className = 'about-product-name';
    titleEn.textContent = 'HWP 5.0 Compatible Module for Rust';
    body.appendChild(titleEn);

    // 제품 한국어명
    const titleKo = document.createElement('div');
    titleKo.className = 'about-product-name-ko';
    titleKo.textContent = '한글 문서 호환 저장 도구';
    body.appendChild(titleKo);

    // 버전
    const version = document.createElement('div');
    version.className = 'about-version';
    version.textContent = `Core ${__APP_VERSION__} / Desktop ${DESKTOP_RELEASE_VERSION}`;
    body.appendChild(version);

    // 기술 스택
    const tech = document.createElement('div');
    tech.className = 'about-tech';
    tech.textContent = 'Rust + WebAssembly + TypeScript';
    body.appendChild(tech);

    // HWP 공개 문서 참고 고지
    const notice = document.createElement('div');
    notice.className = 'about-notice';
    notice.textContent =
      '본 제품은 한글과컴퓨터의 한글 문서 파일(.hwp/.hwpx) 공개 문서를 참고하여 개발한 호환 구현입니다.';
    body.appendChild(notice);

    const releaseTitle = document.createElement('div');
    releaseTitle.className = 'about-release-title';
    releaseTitle.textContent = `이번 설치본 변경사항 v${DESKTOP_RELEASE_VERSION}`;
    body.appendChild(releaseTitle);

    const releaseList = document.createElement('ul');
    releaseList.className = 'about-release-list';
    for (const item of RELEASE_HIGHLIGHTS) {
      const li = document.createElement('li');
      li.textContent = item;
      releaseList.appendChild(li);
    }
    body.appendChild(releaseList);

    const limitations = document.createElement('div');
    limitations.className = 'about-release-note';
    limitations.textContent = TRACKED_LIMITATIONS.join(' ');
    body.appendChild(limitations);

    // 오픈소스 라이선스
    const licenseTitle = document.createElement('div');
    licenseTitle.className = 'about-license-title';
    licenseTitle.textContent = '오픈소스 라이선스';
    body.appendChild(licenseTitle);

    const licenseTable = document.createElement('table');
    licenseTable.className = 'about-license-table';
    for (const lib of THIRD_PARTY_LICENSES) {
      const tr = document.createElement('tr');
      const tdName = document.createElement('td');
      tdName.textContent = lib.name;
      const tdLicense = document.createElement('td');
      tdLicense.textContent = lib.license;
      tr.appendChild(tdName);
      tr.appendChild(tdLicense);
      licenseTable.appendChild(tr);
    }
    body.appendChild(licenseTable);

    // 저작권
    const copyright = document.createElement('div');
    copyright.className = 'about-copyright';
    copyright.textContent = '\u00A9 2026';
    body.appendChild(copyright);

    return body;
  }

  protected onConfirm(): void {
    // 정보 표시용 대화상자이므로 확인 동작 없음
  }

  override show(): void {
    super.show();
    // footer를 닫기 버튼 하나로 교체
    const footer = this.dialog.querySelector('.dialog-footer');
    if (footer) {
      footer.innerHTML = '';
      const closeBtn = document.createElement('button');
      closeBtn.className = 'dialog-btn dialog-btn-primary';
      closeBtn.textContent = '닫기';
      closeBtn.addEventListener('click', () => this.hide());
      footer.appendChild(closeBtn);
    }
  }
}
