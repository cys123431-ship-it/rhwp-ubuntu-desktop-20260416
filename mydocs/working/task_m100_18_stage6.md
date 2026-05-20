# M100 #18 단계 6 완료 보고서

## 단계 범위

- `golbin/hop` 저장소를 검토하고 RHWP에 즉시 적용할 가치가 있는 데스크톱 기능을 선별했다.
- HOP의 구조 중 네이티브 파일 드래그 앤 드롭 열기 흐름을 우리 앱 이벤트 구조에 맞게 독립 구현했다.
- HOP 코드를 그대로 복사하지 않고, MIT 라이선스 저장소의 공개 구현을 참고해 RHWP의 기존 `rhwp://open-files` 이벤트와 `DocumentIO.onOpenFiles()` 흐름에 연결했다.

## 검토 결과

- HOP에는 로컬 폰트 카탈로그, 업데이트 알림, 네이티브 메뉴, PDF/인쇄, 파일 드롭 열기, 다중 창 열기 구조가 있었다.
- RHWP에는 이미 파일 연결, 다중 창 시작 파일 큐, 최근 문서, 복구, PDF 저장 흐름이 구현되어 있었다.
- 빠르게 사용자 체감이 큰 누락점은 Tauri 네이티브 파일 드롭 이벤트 처리였다.

## 변경 내용

- `rhwp-desktop/src-tauri/src/main.rs`
  - `WindowEvent::DragDrop` / `DragDropEvent::Drop`를 감지한다.
  - 드롭된 경로 중 `.hwp`, `.hwpx`만 필터링한다.
  - 지원 파일을 읽고 최근 문서에 등록한 뒤 `rhwp://open-files` 이벤트로 현재 창에 전달한다.
  - 새로 생성되는 문서 창에도 같은 드롭 핸들러를 붙인다.
  - 지원 확장자 필터 테스트를 추가했다.
- `rhwp-studio/src/ui/about-dialog.ts`
  - 제품정보에 HOP 검토 기반 드래그 앤 드롭 열기 변경사항을 표시했다.
- `rhwp-desktop`
  - 데스크톱 설치본 버전을 `0.1.8`로 갱신했다.

## 검증

- 통과: `cargo test --manifest-path rhwp-desktop/src-tauri/Cargo.toml`
  - 5 passed
- 통과: `cmd /c npm run build`
- 통과: `npx tauri build --config src-tauri/tauri.windows.currentuser.conf.json`
- 생성 파일: `release/windows/rhwp_0.1.8_x64-setup.exe`
- SHA256: `e4caef8a7e0ecf30e8d6a53021868e3648a184518f5bd86923bbad788e77931f`
- 로컬 설치 경로: `%LOCALAPPDATA%\rhwp\rhwp.exe`
- 로컬 설치 버전: `0.1.8`
- `.hwp`, `.hwpx` 열기 명령: `%LOCALAPPDATA%\rhwp\rhwp.exe "%1"`
