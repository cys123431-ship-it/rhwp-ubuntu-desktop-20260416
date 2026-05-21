# 구현 계획서 — Task #29

**이슈**: #29 임시 번호 (GitHub CLI 미설치로 원격 이슈 생성 보류)  
**타이틀**: 웹 접근성(A11y) 개선 및 Tauri 데스크톱 보안 샌드박싱 추가  
**마일스톤**: M100  
**브랜치**: `local/task29`  
**작성일**: 2026-05-21  

---

## 구현 단계 계획

### [1단계] 툴바 버튼 정적 접근성 마크업 보강
- **대상 파일**: [index.html](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/index.html)
- **세부 작업**:
  - `#icon-toolbar` 내 모든 버튼(`class="tb-btn"`)에 `aria-label` 부여. (예: `aria-label="오려두기"`, `aria-label="복사하기"`, `aria-label="붙이기"` 등)
  - `#style-bar` 내 모든 버튼(`class="sb-btn"`, `class="sb-combo"`)에 `aria-label` 및 `tabindex="0"` 속성 부여.
  - 마우스 호버 안내용 `title`과 `aria-label` 값 동화.
- **완료 기준**: 요소 검사 시 모든 툴바 버튼들이 `aria-label`을 가지고 있으며 탭 포커스가 잡힐 것.

### [2단계] Toolbar 컴포넌트 내 동적 `aria-pressed` 상태 갱신 로직 구현
- **대상 파일**: [toolbar.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/toolbar.ts)
- **세부 작업**:
  - `Toolbar.setActive(btn, active)` 메서드 내에 `btn.setAttribute('aria-pressed', String(active))` 추가 적용.
  - 글자 효과 드롭다운 버튼(`charfxBtn`) 및 정렬 버튼들의 갱신 시 `aria-pressed` 속성 반영.
  - 드롭다운이 열릴 때 드롭다운 트리거 버튼에 `aria-expanded="true"`, 닫힐 때 `false` 설정 보강.
- **완료 기준**: Bold, Italic, Underline 등의 서식을 토글할 때 해당 버튼의 `aria-pressed` 값이 브라우저 상에서 동적으로 변경될 것.

### [3단계] 데스크톱 Rust 백엔드 파일 경로 탈출(Path Traversal) 검증 가드 구현
- **대상 파일**: [main.rs](file:///home/joseph/바탕화면/개발/hangul/rhwp-desktop/src-tauri/src/main.rs)
- **세부 작업**:
  - 경로 유효성 및 탈출 제어용 보안 가드 헬퍼 함수인 `validate_safe_path(path: &Path) -> Result<PathBuf, String>` 구현.
  - 상위 디렉토리 참조(`..`), 널 바이트(`\0`), 부적절한 심볼릭 링크 포함 여부를 `canonicalize`를 통해 검증.
  - `open_document_at_path` 및 `save_document` 커맨드의 입력 경로 인수 수신부에 경로 유효성 및 홈 디렉토리 또는 안전 영역 여부 가드 추가.
- **완료 기준**: 경로 탈출 패턴 입력 시 예외를 발생시키며 안전하게 로드 거부할 것.

### [4단계] 정적 검사 및 빌드 검증
- **대상 파일**: 프로젝트 전체
- **세부 작업**:
  - TypeScript 타입 체킹: `npm run tsc` (또는 `npx tsc --noEmit`) 실행 및 오류 여부 점검.
  - 전체 빌드 검증: `npm run build`를 통해 WASM 연동 및 데스크톱 패키지 컴파일 무오류 확인.
- **완료 기준**: 모든 컴파일, 린트 및 빌드 파이프라인 무사 통과.

---

## 승인 요청

위 구현계획서를 승인해주시면 각 단계별 작업을 신속하게 착수하겠습니다.
