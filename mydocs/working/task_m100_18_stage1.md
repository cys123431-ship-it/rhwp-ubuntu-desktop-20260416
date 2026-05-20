# M100 #18 단계 1 완료 보고서

## 단계 범위

- 한컴 기능/단축키 기준선 작성
- 블록 선택 후 글자 크기 단축키 보강
- 정적 단축키 검사와 E2E 회귀 테스트 추가
- Undo/Redo에서 글자 서식 복원 결함 수정

## 변경 내용

- `shortcut-map.ts`
  - `Alt+Shift+E`: 글자 크기 크게
  - `Alt+Shift+R`: 글자 크기 작게
  - 기존 `Ctrl+]`, `Ctrl+[` 유지
- `command.ts`, `history.ts`, `cursor.ts`, `input-handler.ts`
  - 글자 서식 Undo/Redo가 선택 범위를 유지하도록 보강
  - 이전 CharShape 속성을 함께 저장해 WASM 갱신 전후 모두 복원 가능하게 처리
- Rust core
  - `charShapeId`를 `CharShapeMods`에 추가하고 JSON 파서/적용 경로에서 직접 복원할 수 있게 처리
- E2E
  - `shortcut-map-static.test.mjs`
  - `block-font-size-shortcuts.test.mjs`
  - Windows headless Chrome 경로 기본값 지원
- 문서/코퍼스
  - `hancom-shortcut-audit.tsv`
  - `hancom-feature-parity-matrix.tsv`
  - `hancom_shortcut_full_audit.md`
  - `hancom_feature_parity_audit_20260428.md`

## 검증

- 통과: `cmd /c npm run test:shortcuts`
- 통과: `cmd /c npm run build`
- 통과: `cargo check --lib --no-default-features -q` (`CARGO_TARGET_DIR=C:\rhwp-target`)
- 통과: `cargo test parses_char_shape_id_for_undo_restore -q` (`CARGO_TARGET_DIR=C:\rhwp-target`)
- 통과: `node e2e/block-font-size-shortcuts.test.mjs --mode=headless`

## 남은 작업

- 블록 서식 저장 후 HWP/HWPX 재열기 검증
- 표 셀 블록 선택 후 글자/문단 서식 E2E
- Ubuntu `.deb` 설치 스모크와 Windows 관리자 설치본 공존 검증
