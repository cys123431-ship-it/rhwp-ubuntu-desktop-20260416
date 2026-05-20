# M100 #18 7단계 완료 보고서

## 단계 목표
- 일반 문자 입력 `Shift+=`가 화면 확대 단축키로 처리되는 문제를 수정한다.
- 한컴/워드처럼 `+` 문자는 본문 입력으로 들어가고, 확대 단축키는 숫자 키패드 조합으로만 동작하게 분리한다.

## 변경 내용
- `rhwp-studio/src/command/shortcut-map.ts`
  - 단축키 매칭에 `KeyboardEvent.code` 조건을 추가했다.
  - `Shift+Num +`는 `NumpadAdd`에서만, `Shift+Num -`는 `NumpadSubtract`에서만 매칭되도록 변경했다.
  - 일반 키보드의 `Shift+=`는 더 이상 `view:zoom-in`으로 해석되지 않는다.
- `rhwp-studio/e2e/shortcut-map-static.test.mjs`
  - 줌 확대 단축키가 숫자 키패드로 제한되는지 검사한다.
  - 일반 `Shift+=`가 `+` 입력으로 남는지 정적 회귀 검사를 추가했다.
- `rhwp-studio/src/ui/about-dialog.ts`
  - Desktop `0.1.9` 변경사항에 `Shift+=` 입력 충돌 수정 항목을 추가했다.
- `rhwp-desktop`
  - Windows 설치본 버전을 `0.1.9`로 올렸다.

## 검증
- `cmd /c npm run test:shortcuts`
- `cmd /c npm run build`
- `CARGO_TARGET_DIR=C:\rhwp-desktop-target cargo test --manifest-path rhwp-desktop/src-tauri/Cargo.toml -q`
- `CARGO_TARGET_DIR=C:\rhwp-desktop-target cmd /c npx tauri build --config src-tauri/tauri.windows.currentuser.conf.json`
- `release/windows/rhwp_0.1.9_x64-setup.exe` 생성
- SHA256: `69a06b70833385167beaaf21924cf560669f27faadb805e2ea1a1f8a5611a982`
- Windows current-user 로컬 설치 확인: `ProductVersion 0.1.9`, `DisplayVersion 0.1.9`

## 판정
- P0/P1 입력 충돌 수정 완료.
- Windows current-user `0.1.9` 빌드 및 로컬 설치 완료.
