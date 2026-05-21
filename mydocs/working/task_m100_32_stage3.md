# 단계별 완료 보고서 — Task #32 [3단계]

**이슈**: #32 (임시 번호)  
**타이틀**: 데스크톱 버전 빌드 자동화 및 하드코딩 제거 완료 보고서  
**마일스톤**: M100  
**담당자**: Antigravity  
**작성일**: 2026-05-21  

---

## 1. 3단계 작업 개요

- **목표**: 릴리즈 명칭 및 데스크톱 버전이 소스코드 내부에 고정값으로 하드코딩되어 빌드본과 어긋나던 문제를 완치하기 위해, 빌드 도구 컴파일 타임에 `rhwp-desktop/package.json` 의 실시간 정보를 가져와 주입하도록 자동화 파이프라인을 구축합니다.
- **대상 파일**:
  - [vite.config.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/vite.config.ts)
  - [about-dialog.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/about-dialog.ts)

---

## 2. 세부 수정 내역

### vite.config.ts
- `../rhwp-desktop/package.json` 파일을 실시간 파싱하여 `desktopVersion` 값 추출 로직 구성 (실패 시 예외 처리 적용하여 기존 웹 뷰어 빌드와의 호환성 확보).
- `define` 상수에 `__DESKTOP_VERSION__` 을 추가 등록하여 컴파일 번들러가 동적으로 문자열 치환 주입하도록 가동.

### about-dialog.ts
- 기존 `const DESKTOP_RELEASE_VERSION = '0.1.12';` 하드코딩 덩어리를 삭제.
- `declare const __DESKTOP_VERSION__: string;` 정적 타입 정의와 함께 `const DESKTOP_RELEASE_VERSION = __DESKTOP_VERSION__;` 을 매핑하여 빌드 산출물과 정밀 동기화 완료.

---

## 3. 검증 결과

- **동적 치환 작동성**: `vite` 빌더 컴파일 시, package.json에 명시된 `0.1.12` 버전을 깔끔하게 번들 리소스 파일에 이식 성공.
- **버전 정합성**: 향후 데스크톱 패키지 버전 스펙이 바뀔 때 수동 소스 수정을 유발하지 않는 완벽한 릴리즈 파이프라인 가용 상태 확인.
