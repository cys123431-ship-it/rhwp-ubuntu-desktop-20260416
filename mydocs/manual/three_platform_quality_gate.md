# 3플랫폼 품질 게이트

**기준일**: 2026-04-24

이 문서는 RHWP의 웹, Ubuntu 데스크톱, Windows 데스크톱 검증 기준을 통일한다. 기능 완료는 한 플랫폼에서만 동작하는 상태가 아니라 저장 후 재열기까지 포함한 3플랫폼 통과를 뜻한다.

## 공통 완료 기준

- HWP/HWPX 원본을 연다.
- 페이지 수와 주요 레이아웃 좌표가 허용오차 안에 들어온다.
- 편집 기능은 저장 후 같은 파일을 다시 열어 손상 없이 확인한다.
- 보호 보기 문서는 저장으로 원본을 손상시키지 않는다.
- hwpctl Action은 `implemented`, `partial`, `stub`, `unsupported` 상태를 노출한다.
- 신규 스텁은 요구사항 추적표와 이슈에 연결한다.

## Windows 로컬 검증 기준

현재 Windows 로컬 환경에서는 다음 차단 요인이 확인됐다.

| 항목 | 현상 | 표준 대응 |
|---|---|---|
| PowerShell `npm.ps1` | 실행 정책으로 `npm run build`가 차단됨 | `cmd /c npm run build` 사용 |
| GNU Rust + 한글 경로 | `dlltool`이 한글 경로의 `.def` 파일을 열지 못함 | ASCII 경로에서 검증하거나 MSVC 툴체인 사용 |
| MSVC Rust | `link.exe`가 없어 링크 실패 | Visual Studio Build Tools의 C++ build tools 설치 |

권장 명령:

```powershell
scripts\windows-local-check.ps1
```

웹 빌드만 직접 확인할 때:

```powershell
cd rhwp-studio
cmd /c npm run build
```

MSVC 환경이 준비된 뒤 Rust 검증:

```powershell
cargo +stable-x86_64-pc-windows-msvc test
cargo +stable-x86_64-pc-windows-msvc clippy -- -D warnings
```

GNU 환경을 유지할 때는 저장소를 ASCII 경로로 복제한 뒤 검증한다.

## Web 게이트

```powershell
cd rhwp-studio
cmd /c npm run build
npx vite --host 0.0.0.0 --port 7700
node e2e/text-flow.test.mjs
node e2e/hwpctl-basic.test.mjs
```

통과 조건:

- TypeScript 빌드 통과
- Vite 번들 생성
- hwpctl 기본 E2E 통과
- 편집, 표, 폼 컨트롤, 반응형, 렌더링 시나리오 통과
- 브라우저 콘솔에 신규 오류 없음

## Ubuntu 데스크톱 게이트

```bash
cargo test
cargo clippy -- -D warnings
docker compose --env-file .env.docker run --rm wasm
```

추가 설치 스모크:

- Ubuntu 22.04 `.deb` 설치
- Ubuntu 24.04 `.deb` 설치
- HWP/HWPX 더블 클릭 열기
- 파일 연결, 기본 앱 흐름, 인쇄 또는 내보내기 확인
- 큰 문서 열기와 스크롤 성능 확인

## Windows 데스크톱 게이트

선행 조건:

- Visual Studio Build Tools C++ build tools 설치
- `link.exe`가 PATH에서 검색됨
- PowerShell 실행 정책이 빌드를 막으면 `cmd /c` 사용

검증 항목:

- Rust MSVC 테스트 통과
- Windows 설치 패키지(NSIS/MSI) 생성
- 설치 후 HWP/HWPX 파일 연결 확인
- 더블 클릭 열기
- 보호 보기 문서 저장 차단 확인
- 자동 복구, 폰트 경고, 인쇄/내보내기 확인

## 호환성 코퍼스 게이트

```powershell
cargo run --bin rhwp -- compat-corpus compatibility-corpus/phase1-supported.tsv
cargo run --bin rhwp -- compat-corpus compatibility-corpus/phase1-protected.tsv
cargo run --bin rhwp -- compat-corpus compatibility-corpus/phase2-extended.tsv
```

한컴 품질 승격 샘플은 `compatibility-corpus/hancom-quality-manifest.tsv`에서 별도 추적한다. `TBD`가 남아 있는 항목은 릴리즈 후보로 승격할 수 없다.

## 릴리즈 후보 기준

- 요구사항 추적률 100%
- 신규 스텁 0개 또는 승인된 이슈 연결
- 웹 빌드 통과
- Ubuntu 22.04/24.04 설치 스모크 통과
- Windows 설치 스모크 통과
- 주요 샘플 페이지 수 불일치 0건
- 보호 보기 문서 저장 손상 0건
- 필수 보안/폰트/브라우저 확장 문서 확인 완료
