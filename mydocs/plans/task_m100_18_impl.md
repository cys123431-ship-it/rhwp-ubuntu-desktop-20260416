# M100 #18 구현 계획서: 한컴 기능/단축키 전수 검토

## 1단계: 기준선과 추적표 작성

- 한컴 공식 단축키/기능 기준을 `hancom-shortcut-audit.tsv`, `hancom-feature-parity-matrix.tsv`로 정리한다.
- 각 행에 RHWP 커맨드, hwpctl Action, 테스트, 플랫폼 게이트, 상태값을 연결한다.
- 기존 `compatibility-corpus/README.md`에 새 추적표를 등록한다.

## 2단계: 글자 크기 단축키 호환 보강

- `shortcut-map.ts`에 한컴 기본 별칭 `Alt+Shift+E`, `Alt+Shift+R`을 추가한다.
- 기존 `Ctrl+]`, `Ctrl+[` 별칭은 유지한다.
- 단축키 라벨 동기화가 여러 별칭을 함께 표시하도록 기존 `getShortcutLabel` 흐름을 그대로 사용한다.

## 3단계: 자동 검증 추가

- `shortcut-map-static.test.mjs`로 단축키 맵과 커맨드 registry 연결을 검사한다.
- `block-font-size-shortcuts.test.mjs`로 블록 선택 후 글자 크기 증가/감소, 선택 유지, 툴바 동기화, Undo/Redo를 검증한다.
- `package.json`에 `npm run test:shortcuts`를 추가한다.

## 4단계: 보고서와 남은 결함 분류

- `mydocs/tech/hancom_shortcut_full_audit.md`에 단축키 정책과 현재 판정을 기록한다.
- `mydocs/report/hancom_feature_parity_audit_20260428.md`에 P0/P1/P2 잔여 리스크를 기록한다.
- Ubuntu/Windows 설치 스모크처럼 로컬에서 즉시 끝내기 어려운 항목은 `TBD` 게이트로 남기고 다음 작업 이슈로 승격한다.

## 5단계: 검증

- `cmd /c npm run test:shortcuts`
- `cmd /c npm run build`
- 가능하면 Vite 서버에서 `node e2e/block-font-size-shortcuts.test.mjs --mode=headless` 또는 host CDP 모드로 실행한다.
