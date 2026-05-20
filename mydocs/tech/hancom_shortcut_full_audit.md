# 한컴 단축키 전수 검토 기준선

## 기준

- 확인일: 2026-04-28
- 공식 기준:
  - Hancom Hwp 단축키 도움말: <https://help.hancom.com/hoffice100/ko-KR/Hwp/view/toolbar/shortcut%28table%29.htm?rhtocid=_13_0>
  - Hancom Word 제품 설명: <https://www.hancom.com/en/product/office/forWindows/hanword>
  - Hancom Office Viewer 지원 형식: <https://www.hancom.com/en/product/office/officeViewer>
- RHWP 기준 파일:
  - `rhwp-studio/src/command/shortcut-map.ts`
  - `rhwp-studio/src/engine/input-handler-keyboard.ts`
  - `rhwp-studio/src/command/commands/*.ts`

## 상태값

| 상태 | 의미 |
|---|---|
| `implemented` | 한컴 기준 동작과 RHWP 구현, 자동/수동 검증이 연결됨 |
| `partial` | 기본 동작은 있으나 저장/재열기, 3플랫폼, 세부 한컴 규칙 중 일부가 남음 |
| `stub` | 메뉴/커맨드 표면만 있고 사용자 기대 동작이 없음 |
| `unsupported` | RHWP 범위 밖으로 명시한 기능 |
| `missing` | 한컴 기준에는 있으나 RHWP 표면이 없음 |
| `bug` | 구현은 있으나 한컴 기준 또는 기존 기대 동작과 충돌 |

## 이번 기준선에서 확정한 단축키

| 기능 | 한컴/RHWP 단축키 | RHWP 커맨드 | 상태 | 검증 |
|---|---|---|---|---|
| 글자 크기 크게 | `Ctrl+]`, `Alt+Shift+E` | `format:font-size-increase` | `implemented` | `block-font-size-shortcuts.test.mjs` |
| 글자 크기 작게 | `Ctrl+[`, `Alt+Shift+R` | `format:font-size-decrease` | `implemented` | `block-font-size-shortcuts.test.mjs` |
| 전체 선택 | `Ctrl+A` | `edit:select-all` | `implemented` | `editing-core-regression.test.mjs` |
| 쪽 나누기 | `Ctrl+Enter`, `Ctrl+J` | `page:break` | `implemented` | `editing-core-regression.test.mjs` |
| 표 안 줄 삽입 | `Ctrl+Enter` | `table:insert-row-below` | `partial` | 수동 검증 필요 |
| 100% 보기 | `Ctrl+G,Q` | `view:zoom-100` | `implemented` | `shortcut-map-static.test.mjs` |

## 블록 글자 크기 판정

- 블록 선택 후 글자 크기 조절은 P0 회귀 항목이다.
- `Ctrl+A`로 본문 `abc`를 선택한 뒤 다음 단축키를 순서대로 검증한다.
  - `Alt+Shift+E`: 10pt에서 11pt로 증가
  - `Alt+Shift+R`: 11pt에서 10pt로 감소
  - `Ctrl+]`: 10pt에서 11pt로 증가
  - `Ctrl+[`: 11pt에서 10pt로 감소
  - `Ctrl+Z`, `Ctrl+Y`: 마지막 글자 크기 변경을 Undo/Redo
- 성공 조건:
  - 선택 범위가 `0..3`으로 유지된다.
  - `getCharPropertiesAt(0, 0, 0).fontSize`가 100 HWPUNIT 단위로 변경된다.
  - 서식 도구 모음 글자 크기 값이 실제 CharShape와 동기화된다.

## 충돌 정책

- 동일 단축키가 컨텍스트별로 다른 기능을 수행하는 경우 `shortcut-map.ts`의 `contexts`로 분리한다.
- 현재 의도된 충돌:
  - `Ctrl+Enter`: 본문은 쪽 나누기, 표 안은 줄 삽입
- 브라우저나 OS가 선점하는 단축키는 한컴 별칭을 보존하되 RHWP 대체 별칭도 함께 기록한다.
- 한글 IME 상태에서 영문 키가 한글 자모로 들어오는 chord 단축키는 `input-handler-keyboard.ts`에서 직접 보강한다.

## 남은 전수 조사 작업

- 공식 도움말의 모든 표 단축키를 `hancom-shortcut-audit.tsv`에 계속 추가한다.
- `stub` 커맨드는 메뉴에 보이더라도 완료로 판정하지 않는다.
- `partial` 항목은 저장/재열기와 Web/Ubuntu/Windows 게이트가 모두 통과해야 `implemented`로 올린다.
