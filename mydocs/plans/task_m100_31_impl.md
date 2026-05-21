# 구현 계획서 — Task #31

**이슈**: #31 임시 번호 (GitHub CLI 미설치로 원격 이슈 생성 보류)  
**타이틀**: 에디터 캔버스 텍스트 가독성 및 소수점 픽셀 어긋남 개선 & 데스크톱 버전 표시 정합성 수정  
**마일스톤**: M100  
**브랜치**: `local/task31`  
**작성일**: 2026-05-21  

---

## 구현 단계 계획

### [1단계] `canvas-view.ts` 내 캔버스 배치 위치 정수 픽셀(Pixel Snap) 계산 및 적용
- **대상 파일**: [canvas-view.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/view/canvas-view.ts)
- **세부 작업**:
  - `renderPage` (L157-L164) 부분 수정:
    - `pageLeft`가 0보다 작은 경우(단일 열 중앙 정렬 상태), `left: '50%'` 대신 부모 컨테이너 너비와 캔버스의 CSS 너비를 가져와 정수 left 값 계산.
    - `const parentWidth = this.scrollContent.clientWidth;`
    - `const cssWidth = canvas.width / dpr;`
    - `const calculatedLeft = Math.max(0, Math.floor((parentWidth - cssWidth) / 2));`
    - `canvas.style.left = \`\${calculatedLeft}px\`;`
    - `canvas.style.transform = 'none';`
  - `refreshPages` (L285-L292) 부분 수정:
    - 동일하게 정수화된 left 배치 계산 로직을 동일 구조로 적용하여 편집 시 위치 갱신에 어긋남이 없도록 보장.
- **완료 기준**: 캔버스에 설정되는 left 스타일 속성이 소수점 없는 문자열(예: `234px`)로 강제 주입될 것.

### [2단계] `editor.css` 의 캔버스 스타일 재정의 및 폰트 렌더링 최적화
- **대상 파일**: [editor.css](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/styles/editor.css)
- **세부 작업**:
  - `#scroll-content canvas` 스타일 정의에서 불필요하고 충돌 여지가 있는 `left: 50%; transform: translateX(-50%);` 속성 완전 제거.
  - 가독성 향상과 번짐 제어를 위한 CSS 속성 주입:
    - `-webkit-font-smoothing: antialiased;`
    - `-moz-osx-font-smoothing: grayscale;`
    - `image-rendering: -webkit-optimize-contrast;`
    - `image-rendering: crisp-edges;`
- **완료 기준**: CSS 빌드가 깨지지 않으며 캔버스 속성이 스타일 우선순위를 획득할 것.

### [3단계] `about-dialog.ts` 하드코딩된 제품 정보 버전 갱신 적용
- **대상 파일**: [about-dialog.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/about-dialog.ts)
- **세부 작업**:
  - `const DESKTOP_RELEASE_VERSION = '0.1.11';` 라인을 `const DESKTOP_RELEASE_VERSION = '0.1.12';` 로 수정.
  - `RELEASE_HIGHLIGHTS` 내용 검토 및 버전 명명 정합성 확인.
- **완료 기준**: 제품 정보 다이얼로그 렌더링 시 데스크톱 릴리즈 버전 명기가 `0.1.12` 로 출력될 것.

### [4단계] TypeScript 빌드 및 타입 안정성 검증
- **대상 파일**: 프로젝트 전체
- **세부 작업**:
  - `rhwp-studio` 디렉토리 하위에서 `npx tsc --noEmit`을 실행하여 TS 타입 오류가 없는지 최종 확인.
  - 빌드 파이프라인 검증 (`npm run build` 또는 개발 서버 확인).
- **완료 기준**: 빌드 에러 및 경고 없이 완벽히 통과할 것.

### [5단계] Vite 로컬 서버 구동을 통한 가독성 및 버전 표시 최종 검증
- **대상 파일**: 프로젝트 전체
- **세부 작업**:
  - Vite 로컬 개발 서버를 실행하고 브라우저에서 가독성 정합성 수동 검증.
  - 캔버스의 `left` 값이 정수 그리드에 스냅되고 있는지 요소 검사로 최종 체킹.
  - 제품 정보 대화상자를 띄워 버전 정보가 `Desktop 0.1.12` 로 정상 표출되는지 검증.
- **완료 기준**: 글꼴 선명화 상태가 보장되고 픽셀 어긋남이 제거되며 최신 버전 정보가 노출될 것.

---

## 승인 요청

위 구현계획서를 승인해주시면 즉각 브랜치를 생성하고 본 개발에 착수하도록 하겠습니다.
