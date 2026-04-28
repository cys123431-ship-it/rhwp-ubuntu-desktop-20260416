# 한컴 기능 호환성 1차 감사 보고서

## 요약

- 감사일: 2026-04-28
- 이슈: #18
- 범위: 기능, 단축키, 블록 편집, 저장/재열기, Web/Ubuntu/Windows 게이트
- 결론: 블록 선택 후 글자 크기 조절의 한컴 기본 단축키 별칭은 보강했고, 기능/단축키 전수 추적표를 추가했다. 저장/재열기와 3플랫폼 스모크가 남은 항목은 `partial`로 유지한다.

## 완료한 항목

- `Alt+Shift+E`, `Alt+Shift+R`을 글자 크기 증가/감소 단축키로 추가했다.
- 기존 `Ctrl+]`, `Ctrl+[` 단축키는 유지했다.
- `shortcut-map-static.test.mjs`로 단축키 맵과 커맨드 연결을 검사한다.
- `block-font-size-shortcuts.test.mjs`로 블록 선택, 글자 크기 변경, 선택 유지, 툴바 동기화, Undo/Redo를 검증한다.
- 글자 서식 Undo가 이전 `charShapeId`를 복원하지 못하던 문제를 고쳤다. 현재 WASM이 갱신되기 전에도 전체 이전 속성을 함께 전달해 글자 크기 Undo가 동작한다.
- `hancom-shortcut-audit.tsv`와 `hancom-feature-parity-matrix.tsv`를 추가해 작은 기능 누락을 추적할 수 있게 했다.

## 검증 결과

| 명령 | 결과 | 비고 |
|---|---|---|
| `cmd /c npm run test:shortcuts` | 통과 | 단축키 별칭, 커맨드 연결, 충돌 검사 |
| `cmd /c npm run build` | 통과 | Vite chunk 크기 경고만 있음 |
| `cargo check --lib --no-default-features -q` with `CARGO_TARGET_DIR=C:\rhwp-target` | 통과 | GNU Rust 한글 경로 문제 회피 |
| `cargo test parses_char_shape_id_for_undo_restore -q` with `CARGO_TARGET_DIR=C:\rhwp-target` | 통과 | `charShapeId` 파싱 회귀 테스트 |
| `node e2e/block-font-size-shortcuts.test.mjs --mode=headless` | 통과 | Windows Chrome headless, `CHROME_PATH` 지정 |

## P0 리스크

| ID | 내용 | 현재 상태 | 다음 조치 |
|---|---|---|---|
| P0-01 | 저장 후 재열기 기반 서식 검증 부족 | `partial` | 블록 서식 변경 후 HWP/HWPX 저장, 재파싱, 한컴 열기 샘플 추가 |
| P0-02 | Ubuntu `.deb` 설치 스모크 미확인 | `partial` | Ubuntu 22.04/24.04 설치 후 파일 연결과 더블 클릭 열기 검증 |
| P0-03 | Windows 관리자 설치본과 사용자 설치본 공존 검증 필요 | `partial` | 기존 구버전 설치 상태에서 파일 연결, 기본 앱, 실행 경로 확인 |
| P0-04 | 표 셀 블록 선택 후 서식 적용 검증 부족 | `partial` | F5 셀 선택 모드에서 글자/문단 서식 E2E 추가 |

## P1/P2 리스크

| ID | 내용 | 현재 상태 |
|---|---|---|
| P1-01 | 스타일 적용/편집/저장 기능은 stub 또는 부분 구현으로 분리 필요 | `stub` |
| P1-02 | 블록 계산식, 블록 합계, 표 너비/높이 같게는 메뉴 표면과 실제 동작 분리 필요 | `stub` |
| P1-03 | 찾기/바꾸기는 UI 동작과 치환 Undo 검증이 부족함 | `partial` |
| P2-01 | 레거시 단축키 문서 일부가 터미널에서 깨져 보이므로 새 UTF-8 기준 문서로 대체 필요 | `partial` |

## 완료 기준 갱신

- `implemented` 승격 조건:
  - 공식 기준 또는 실기 확인 기준이 문서화됨
  - RHWP 커맨드/hwpctl/WASM 연결이 명시됨
  - 웹 자동 테스트 또는 명시적 수동 검증이 있음
  - 저장/재열기와 플랫폼 게이트가 필요한 기능은 Web/Ubuntu/Windows 결과가 모두 기록됨
- `stub` 또는 `partial` 항목은 메뉴에 보여도 완료로 계산하지 않는다.
