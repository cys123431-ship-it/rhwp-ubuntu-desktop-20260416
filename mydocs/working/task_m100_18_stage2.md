# M100 #18 단계 2 완료 보고서

## 단계 범위

- 이미지 기준 도구 상자 핵심 기능 연결 검증
- 기본 단축키와 블록 글자 크기 E2E 재검증
- Windows current-user 설치본 v0.1.4 빌드 및 로컬 설치

## 변경 내용

- `rhwp-studio/index.html`
  - 도구 상자 `오려두기`, `복사하기`, `붙이기`, `모양 복사`, `격자 보기`, `하이퍼링크`, `미주` 버튼에 `data-cmd` 연결을 추가했다.
- `rhwp-studio/src/command/commands/edit.ts`
  - `edit:format-copy`를 무조건 비활성 스텁에서 실제 형식 복사/적용 흐름으로 전환했다.
  - 첫 실행은 현재 커서의 글자/문단 형식을 복사하고, 다음 실행에서 선택 영역이 있으면 복사한 형식을 적용한다.
- `rhwp-studio/e2e/toolbar-command-static.test.mjs`
  - 이미지 도구 상자에 보이는 주요 버튼의 커맨드 연결과 스텁 여부를 정적으로 검사한다.
- `rhwp-studio/e2e/toolbar-core.test.mjs`
  - 도구 상자 복사/붙이기와 모양 복사 실제 동작을 Headless Chrome에서 검증한다.
- `rhwp-desktop`
  - Windows 데스크톱 버전을 `0.1.4`로 올렸다.

## 검증

- 통과: `cmd /c npm run test:shortcuts`
- 통과: `cmd /c npm run build`
- 통과: `node e2e/toolbar-core.test.mjs --mode=headless`
- 통과: `node e2e/block-font-size-shortcuts.test.mjs --mode=headless`
- 통과: `cargo check --lib --no-default-features -q` (`CARGO_TARGET_DIR=C:\rhwp-target`)
- 통과: `cargo test parses_char_shape_id_for_undo_restore -q` (`CARGO_TARGET_DIR=C:\rhwp-target`)

## 설치 결과

- 설치본: `release/windows/rhwp_0.1.4_x64-setup.exe`
- SHA256: `d43dbb710e763c0bd21a6fb74ea2cc79550c219fc5ccbc950ea98302318e554f`
- 로컬 설치 경로: `%LOCALAPPDATA%\rhwp\rhwp.exe`
- 설치된 파일 버전: `0.1.4`
- `.hwp`, `.hwpx` HKCU 열기 명령은 `%LOCALAPPDATA%\rhwp\rhwp.exe "%1"`로 연결됨

## 남은 작업

- `insert:hyperlink`, `insert:endnote`는 버튼 연결과 상태 추적은 되었지만 아직 명시적 스텁이다.
- Ubuntu `.deb` 신규 빌드/설치 스모크는 이번 Windows 로컬 설치 단계에 포함하지 않았다.
