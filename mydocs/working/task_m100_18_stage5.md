# M100 #18 단계 5 완료 보고서

## 단계 범위

- 사용자가 제공한 3개 이미지 중 3번 이미지를 새 RHWP 아이콘으로 선택했다.
- GitHub README에서 보이는 로고, 웹 파비콘, Windows/Ubuntu 데스크톱 패키지에서 쓰는 Tauri 아이콘 세트를 같은 이미지 계열로 교체했다.
- 새 아이콘 설치본 식별을 위해 데스크톱 버전을 `0.1.7`로 올렸다.

## 선택 기준

- 1번은 문서/디스켓 느낌이 강해 HWP 호환 편집기 정체성이 덜 직접적이었다.
- 2번은 작은 크기에서 단순하고 힘은 있지만, 앱 아이콘으로 봤을 때 정보량이 부족했다.
- 3번은 한글 심볼, 문서 앱 실루엣, 금속 질감이 함께 살아 있어 GitHub README와 Windows 아이콘 양쪽에 가장 적합했다.

## 변경 내용

- `assets/logo/*`
  - README 로고, 공용 PNG 로고, favicon을 3번 이미지 기반으로 교체했다.
  - `assets/logo/rhwp-icon-source.png`를 새 원본 아이콘 소스로 추가했다.
- `rhwp-studio/public/favicon.ico`
  - 웹/Studio 파비콘을 새 아이콘으로 교체했다.
- `rhwp-desktop/src-tauri/icons/*`
  - Tauri 아이콘 생성기를 사용해 Windows `.ico`, macOS `.icns`, Linux용 PNG, Windows 타일, 모바일 파생 아이콘을 모두 재생성했다.
- `rhwp-logo.png`
  - 저장소 루트 로고 PNG도 새 아이콘으로 교체했다.
- `rhwp-desktop`
  - 데스크톱 설치본 버전을 `0.1.7`로 갱신했다.
- `rhwp-studio/src/ui/about-dialog.ts`
  - 제품정보에 새 아이콘 변경사항을 표시했다.

## 검증

- 통과: `cmd /c npm run build`
- 통과: `npx tauri build --config src-tauri/tauri.windows.currentuser.conf.json`
- 생성 파일: `release/windows/rhwp_0.1.7_x64-setup.exe`
- SHA256: `69a25709538dd8fc81ca91fb55c40c82a57122925f0e5f3c5e69aa4602dcdfbd`
- 로컬 설치 경로: `%LOCALAPPDATA%\rhwp\rhwp.exe`
- 로컬 설치 버전: `0.1.7`
- `.hwp`, `.hwpx` 열기 명령: `%LOCALAPPDATA%\rhwp\rhwp.exe "%1"`
