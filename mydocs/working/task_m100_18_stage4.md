# M100 #18 단계 4 완료 보고서

## 단계 범위

- Ctrl+A 또는 드래그 블록 선택 시 논리 선택 범위와 화면 하이라이트 범위가 어긋나는 결함을 수정했다.
- 사용자가 제보한 현상은 “실제로는 전체가 선택되어 글자 크기 변경이 전체에 적용되지만, 화면에는 마지막 줄 일부가 선택되지 않은 것처럼 보이는 문제”로 분류했다.
- 이번 설치본에서 사용자가 제품정보로 변경 사항을 확인할 수 있도록 데스크톱 버전을 `0.1.6`으로 올렸다.

## 원인

- 기존 선택 표시 사각형은 문단의 `LINE_SEG` 기반 줄 범위를 먼저 신뢰했다.
- 새 문서를 편집하면서 브라우저 렌더 결과와 `LINE_SEG` 추정 줄 범위가 벌어지면, 커서/선택의 논리 범위는 맞아도 마지막 래핑 줄의 선택 사각형이 누락될 수 있었다.
- 결과적으로 서식 적용은 전체 선택으로 동작하지만, 사용자는 한컴 한글이나 MS Word처럼 “잡은 만큼 파랗게 보이는” 확신을 얻지 못했다.

## 변경 내용

- `src/document_core/queries/cursor_nav.rs`
  - 선택 사각형 계산을 실제 렌더 트리의 `TextRun` 기준으로 우선 수행하도록 보정했다.
  - 본문과 표 셀 문맥을 구분해 선택 범위와 `TextRun` 문자 범위가 겹치는 부분만 사각형으로 만든다.
  - 실제 렌더 기반 사각형을 만들 수 없는 경우 기존 `LINE_SEG` 기반 계산으로 되돌아가도록 했다.
- `rhwp-studio/e2e/selection-visual.test.mjs`
  - 긴 한글 텍스트 입력 후 Ctrl+A를 실행하고, 선택 사각형이 여러 줄과 문서 끝 줄까지 포함되는지 검증한다.
  - WASM 선택 사각형 개수와 DOM `.selection-layer` 표시 개수가 일치하는지 확인한다.
- `rhwp-studio/package.json`
  - `npm run e2e:selection-visual` 스크립트를 추가했다.
- `rhwp-studio/src/ui/about-dialog.ts`
  - 제품정보 한국어 문구를 정리하고 `Desktop 0.1.6` 및 이번 설치본 변경사항을 표시했다.
- `rhwp-desktop`
  - Windows 데스크톱 설치본 버전을 `0.1.6`으로 갱신했다.

## 검증

- 통과: `wasm-pack build --target web`
- 통과: `cmd /c npm run build`
- 통과: `npm run e2e:selection-visual -- --mode=headless`
  - 선택 사각형 41개 생성
  - 래핑된 문단 6개 시각 줄 선택 확인
  - 문서 끝 줄 선택 포함 확인
  - DOM 선택 레이어 41/41 반영 확인
- 통과: `cargo test --lib --no-default-features`
  - 837 passed, 1 ignored
- 통과: `cargo clippy --lib --no-default-features -- -D warnings`
- 통과: `npx tauri build --config src-tauri/tauri.windows.currentuser.conf.json`

## 로컬 설치 확인

- 설치 파일: `release/windows/rhwp_0.1.6_x64-setup.exe`
- SHA256: `c0e06cac50e6e4349372d0e314111fabd7b5c420f1f0ace8a3e70b6472f8afca`
- 설치 경로: `%LOCALAPPDATA%\rhwp\rhwp.exe`
- 설치된 실행 파일 버전: `0.1.6`
- Windows 설치 목록: `rhwp 0.1.6`
- `.hwp`, `.hwpx` 열기 명령: `%LOCALAPPDATA%\rhwp\rhwp.exe "%1"`
