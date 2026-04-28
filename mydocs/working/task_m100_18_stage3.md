# M100 #18 단계 3 완료 보고서

## 단계 범위

- 제품 정보 다이얼로그에 이번 설치본 변경사항 표시
- Windows 데스크톱 설치본을 `0.1.5`로 재빌드
- 로컬 current-user 설치 및 설치 목록 확인

## 변경 내용

- `rhwp-studio/src/ui/about-dialog.ts`
  - 제품 정보 버전 표시를 `Core 0.7.2 / Desktop 0.1.5` 형식으로 변경했다.
  - `이번 설치본 변경사항 v0.1.5` 섹션을 추가했다.
  - 단축키, 도구 상자, 모양 복사, Windows 설치 연결 변경사항을 사용자 확인 가능하게 표시했다.
- `rhwp-studio/src/styles/dialogs.css`
  - 제품 정보 변경사항 목록과 추적 항목 안내 스타일을 추가했다.
- `rhwp-desktop`
  - 데스크톱 패키지 버전을 `0.1.5`로 올렸다.

## 검증

- 통과: `cmd /c npm run build`
- 통과: `npx tauri build --config src-tauri/tauri.windows.currentuser.conf.json`
- 설치본: `release/windows/rhwp_0.1.5_x64-setup.exe`
- SHA256: `eda8bcfae65932a0635d0ddcc2999f9a609a7efe977cffde0c72cddd8a3d4f74`

## 로컬 설치 확인

- 설치 경로: `%LOCALAPPDATA%\rhwp\rhwp.exe`
- 설치된 파일 버전: `0.1.5`
- Windows 설치 목록: `rhwp 0.1.5`
- `.hwp`, `.hwpx` 열기 명령: `%LOCALAPPDATA%\rhwp\rhwp.exe "%1"`
