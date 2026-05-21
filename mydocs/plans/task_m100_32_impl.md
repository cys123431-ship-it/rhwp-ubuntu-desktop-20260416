# 구현 계획서 — Task #32

**이슈**: #32 (임시 번호)  
**타이틀**: 모달 다이얼로그 접근성 보강 및 버저닝·DOM 라이프사이클 리팩터링 — 구현 계획서  
**마일스톤**: M100  
**담당자**: Antigravity  
**작성일**: 2026-05-21  

---

## 1. 구현 단계별 세부 계획

### [1단계] `ModalDialog` 아키텍처 개편 및 훅 이식
- **대상 파일**: [dialog.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/dialog.ts)
- **작업 내용**:
  - `build()` 내에서 footer 버튼 생성 시 직접 로직을 처리하는 대신 `protected createFooterButtons(): HTMLElement[]` 메소드를 도입하여 서브클래스 오버라이드 유연성 확보.
  - 디폴트 버튼 구성(확인, 취소)을 `createFooterButtons()` 로 마이그레이션.
- **검증**: 기존 다이얼로그의 레이아웃이나 동작의 회기성(Regression)이 없음을 타입 검사로 검증.

### [2단계] `AboutDialog` DOM 리사이클 방지 및 포커스 안착
- **대상 파일**: [about-dialog.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/about-dialog.ts)
- **작업 내용**:
  - `createFooterButtons()` 를 오버라이드하여 `dialog-btn-primary` 스타일의 '닫기' 버튼 1개만 정의.
  - `show()` 함수에서 `super.show()` 만 깔끔하게 호출하고, 기존의 `footer.innerHTML = ''` 및 `closeBtn` 수동 생성 로직 완전 파괴.
- **검증**: 다이얼로그가 열릴 때 '닫기' 버튼에 포커스가 정확히 인가되는지 점검.

### [3단계] 데스크톱 패키지 버전 빌드 자동 싱크 탑재
- **대상 파일**: 
  - [vite.config.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/vite.config.ts)
  - [about-dialog.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/about-dialog.ts)
- **작업 내용**:
  - `vite.config.ts` 에서 `../rhwp-desktop/package.json` 을 비동기/동기 로드하여 `desktopVersion` 값 추출.
  - `define` 상수에 `__DESKTOP_VERSION__` 문자열 추가 정의.
  - `about-dialog.ts` 에서 `DESKTOP_RELEASE_VERSION` 상수를 `__DESKTOP_VERSION__` 과 매핑 연동.
- **검증**: `npm run build` 시 컴파일러 오류 없이 번들링이 통과하는지 확인.

### [4단계] 형상 정돈 및 최종 종합 테스팅
- **대상 파일**: 
  - `mydocs/plans/` (Untracked 기안서 정리)
  - `rhwp-studio/src/` (TypeScript 타입 안전성)
- **작업 내용**:
  - `git status` 에 잔존하던 `task_m100_28.md` 등의 파일들을 `mydocs/plans/archives/` 디렉터리로 안전하게 이동하여 형상 클린 상태 복원.
  - `npx tsc --noEmit` 구동을 통한 TS 무결 검사.
- **검증**: `git status`가 완벽하게 깔끔하고 정돈되었는지 검증.

---

## 2. 일정 및 마일스톤
- 오늘 중 1~4단계를 막힘없이 고속 추진하여, 사용자가 로컬 deb 패키지를 완벽한 형태로 기분 좋게 사용할 수 있게 완료하겠습니다.
