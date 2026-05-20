# M100 #18 UI/UX 및 기능 전수 검토 보고서 겸 통합 구현 계획서

## 1. UI/UX 및 기능 영역 전수 검토 보고

### 1.1 이전 지시사항 정밀 기술 검증
1. **`+` 입력 충돌 해제 검증 (이상 없음)**
   * **검사 결과**: 일반 텍스트 입력 중 `+` 키를 기입할 때 화면 줌 인(`Shift+Num +`) 단축키가 강제로 기동되어 글자가 기입되지 않던 충돌이 완전히 해결되었습니다.
   * **기술적 해결 방식**: `shortcut-map.ts` 에서 줌 단축키의 입력 시그널을 일반 키보드 최상단의 `+`/`=` 키(코드: `Equal`)와 분리하고, 키패드 영역의 `+` 키(코드: `NumpadAdd`)와 Shift 키가 결합(`Shift+Num +`)될 때만 작동하도록 키보드 매핑을 매칭하여 충돌을 원천 차단하였습니다.
2. **글자 크기 및 줄 간격 한컴 단축키 검증 (이상 없음)**
   * **검사 결과**: `shortcut-map.ts` 에 한컴오피스 표준인 글자 크기 증가(`Ctrl+]` / `Alt+Shift+E`), 글자 크기 감소(`Ctrl+[` / `Alt+Shift+R`), 줄 간격 조절(`Alt+Shift+A` / `Alt+Shift+Z`)이 충돌 없이 정확하게 매핑되어 있으며, `input-handler-keyboard.ts` 의 커맨드 버스를 거쳐 코어 엔진으로 정상 연동됩니다.
3. **데스크톱 앱 아이콘 교체 상태 검사 (이상 없음)**
   * **검사 결과**: `tauri.conf.json` 및 `tauri.linux.conf.json` 의 번들 구성(`bundle.icon`)에 새로운 고해상도 `../../assets/logo/logo-*.png` 리소스들이 단단히 연결되어 있습니다. 리눅스 패키지 빌드 시 브랜드 아이콘이 무결하게 이식됩니다.
4. **리눅스 우분투 시스템 의존성 패키징 (이상 없음)**
   * **검사 결과**: `tauri.linux.conf.json` 의 데비안 패키지 종속성에 `libgtk-3-0`, `libwebkit2gtk-4.1-0`, `xdg-utils` 가 규정되어 실행 라이브러리 부재로 인한 충돌 없이 매끄러운 런타임을 만족합니다.

---

## 2. 에디터 UI/UX 레이아웃 결함 개선 계획 (수정 승인 요청)

전수 조사 결과, 해상도가 낮거나 화면 크기 조절 시 시각적 짤림 현상이 유발되는 취약 영역 3곳을 도출하여 스타일 수정 계획을 수립하였습니다.

1. **서식 도구 모음 (`style-bar.css`) - 가로 너비 확장**:
   드롭다운 화살표 영역과 글자가 겹쳐 핵심 정보가 가려지는 것을 막기 위해 콤보박스 가로 너비를 안전폭으로 조절합니다.
   - 스타일 이름 셀렉터(`#style-name`): `60px` ➡️ **`78px`** ("바탕글" 글자 안착)
   - 글꼴 언어 셀렉터(`.sb-font-lang`): `44px` ➡️ **`56px`** ("대표" 글자 안착)
   - 글꼴 이름 셀렉터(`.sb-font`): `110px` ➡️ **`125px`** ("함초롬바탕" 및 긴 글꼴명 안착)
   - 줄 간격 셀렉터(`.sb-ls-select`): `56px` ➡️ **`68px`** ("160%" 끝 퍼센트 기호 안착)
2. **모든 다이얼로그 본문 (`dialogs.css`) - 세로 높이 제한 및 스크롤 활성화**:
   창 세로 크기가 작아지거나 제품 정보(`AboutDialog`)처럼 내용이 아주 길어질 때, 대화상자가 화면을 수직으로 뚫고 나가는 것을 방지합니다.
   - `.dialog-body`에 `max-height: 65vh;` 및 `overflow-y: auto;` 적용. 본문 내용만 콤팩트하게 스크롤되고 하단 확인/닫기 버튼이 늘 뷰포트 내에 고정 노출되도록 제어합니다.
3. **도구 상자 (`toolbar.css`) - 가로 오버플로우 스크롤 안전장치 이식**:
   화면 가로 폭을 줄였을 때 툴바 내 버튼들이 짤려서 소실되는 것을 방지합니다.
   - `#icon-toolbar`에 `overflow-x: auto; scrollbar-width: none;` 및 `-webkit-scrollbar { display: none; }` 적용. 폭이 좁아져도 가로 방향 휠/터치 스크롤로 모든 기능 버튼에 100% 접근 가능하게 UX를 보완합니다.

---

## 3. 제안된 스타일시트(CSS) 변경 세부 사항 (Proposed Changes)

### [Component: rhwp-studio/src/styles]

#### [MODIFY] [style-bar.css](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/styles/style-bar.css)
- **수정 사양**:
  - `#style-name`: `width: 60px;` ➡️ **`width: 78px;`**
  - `.sb-font-lang`: `width: 44px;` ➡️ **`width: 56px;`**
  - `.sb-font`: `width: 110px;` ➡️ **`width: 125px;`**
  - `.sb-ls-select`: `width: 56px;` ➡️ **`width: 68px;`**

#### [MODIFY] [dialogs.css](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/styles/dialogs.css)
- **수정 사양**:
  - `.dialog-body` 공통 스타일시트에 세로 비율 제약과 스크롤 제어를 부착합니다.
  ```css
  .dialog-body {
    padding: 12px;
    max-height: 65vh;
    overflow-y: auto;
  }
  ```

#### [MODIFY] [toolbar.css](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/styles/toolbar.css)
- **수정 사양**:
  - `#icon-toolbar` 스타일에 오버플로우 발생 시 가로 스크롤링이 가능하도록 제어하는 코드를 주입하고, 모던 CSS의 `:has()` 가상 클래스를 활용해 드롭다운 메뉴 활성화 시 외부 잘림 현상을 철저히 예방합니다.
  ```css
  #icon-toolbar {
    display: flex;
    align-items: stretch;
    padding: 2px 8px;
    background: linear-gradient(to bottom, #fafafa, #f0f0f0);
    border-bottom: 1px solid #c8c8c8;
    flex-shrink: 0;
    height: 56px;
    overflow-x: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  #icon-toolbar::-webkit-scrollbar {
    display: none;
  }

  /* 툴바 내에 스플릿 메뉴나 스타일바 드롭다운, 색상 선택창이 열렸을 때는 overflow를 visible로 복구해 잘림 현상 원천 차단 */
  #icon-toolbar:has(.tb-split.open),
  #icon-toolbar:has(.sb-dropdown.open),
  #icon-toolbar:has(.sb-color-wrap.open) {
    overflow: visible;
  }
  ```


---

## 4. 검증 및 빌드 절차 (Verification Plan)

### A. 빌드 검증
1. **rhwp-studio 프론트엔드 모듈 정적 컴파일**: `npm run build`
2. **rhwp-desktop Tauri 기반 리눅스 배포 패키지 컴파일**: `npm run build:linux`

### B. 수동 시각 무결성 검증
1. **업그레이드 설치**:
   ```bash
   sudo apt install ./rhwp-desktop/src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/rhwp_*.deb
   ```
2. **기능 검사**:
   - 서식 도구 모음에서 "바탕글", "대표", "함초롬바탕", "160%" 가 잘리지 않고 출력되는지 확인.
   - 제품 정보 다이얼로그에서 세로 스크롤바 작동 하에 닫기 버튼이 영역에 안착하는지 확인.
   - 창 가로 크기를 최소 규격 이하로 조절할 때, 툴바 버튼들이 짤려 유失되지 않고 가로 스크롤로 매끄럽게 탐색 가능한지 확인.
