# 수행 계획서: M100 한컴 호환형 3플랫폼 품질 로드맵

**이슈**: 미지정(사용자 직접 지시)
**마일스톤**: M100
**브랜치**: 현재 작업 브랜치
**기준일**: 2026-04-24

---

## 1. 목표

RHWP를 한컴 한글과 비슷한 오픈소스 대체 구현으로 만들기 위해 UI 모양보다 문서 호환성을 우선한다. 완료 기준은 웹, Ubuntu 데스크톱, Windows 데스크톱에서 동일 문서를 열고, 렌더링하고, 편집하고, 저장한 뒤 다시 열었을 때 손상 없이 동작하는 것이다.

## 2. 범위

- 한컴 공식 기능 축을 `파일/보기/입력/서식/쪽/표/검토/보안/플랫폼` 요구사항으로 나눈다.
- 요구사항을 이슈, 테스트, 명령, hwpctl Action, 코퍼스 샘플에 연결한다.
- hwpctl Action은 `implemented`, `partial`, `stub`, `unsupported` 상태를 노출한다.
- 호환성 코퍼스는 예상 editMode, 저장 형식, 페이지 수, 레이아웃 허용오차, 저장 후 재파싱, 3플랫폼 통과 여부를 기록한다.
- Windows 로컬 검증 환경의 현재 차이를 표준화한다.

## 3. 제외 범위

- 한컴 자산, 전용 폰트, 비공개 명세, 상표 UI의 복제는 포함하지 않는다.
- 한 번의 작업으로 모든 편집 기능을 완성하지 않는다. 이번 작업은 3개월 로드맵을 실제 추적 가능한 품질 체계로 고정하는 기반 작업이다.

## 4. 필수 참조 문서

- `mydocs/manual/browser_extension_dev_guide.md`
- `mydocs/tech/font_fallback_strategy.md`
- `mydocs/report/browser_extension_security_audit.md`
- `compatibility-corpus/README.md`
- `mydocs/manual/ir_diff_command.md`

## 5. 산출물

| 산출물 | 경로 | 목적 |
|---|---|---|
| 구현 계획서 | `mydocs/plans/task_m100_hancom_compat_quality_impl.md` | 3개월 로드맵을 5단계 실행 단위로 고정 |
| 요구사항 추적표 | `mydocs/tech/hancom_compatibility_traceability.md` | 한컴 기능 축과 RHWP 구현, 테스트, 플랫폼 게이트 연결 |
| 3플랫폼 품질 게이트 | `mydocs/manual/three_platform_quality_gate.md` | 웹, Ubuntu, Windows 검증 절차 표준화 |
| 호환성 매니페스트 | `compatibility-corpus/hancom-quality-manifest.tsv` | 한컴 품질 기준 샘플별 기대값과 통과 상태 추적 |
| Windows 로컬 점검 스크립트 | `scripts/windows-local-check.ps1` | PowerShell/npm/Rust/MSVC 경로 문제를 재현 가능한 점검으로 고정 |

## 6. 성공 기준

- hwpctl Action 목록에서 실행 연결 여부와 한컴 호환 완료 여부가 분리되어 보인다.
- 미구현 Action이 성공처럼 보이지 않는다.
- 한컴 호환 요구사항이 테스트와 코퍼스 항목에 연결된다.
- Windows 환경의 알려진 차단 요인이 검증 절차에 명시된다.
- 웹 빌드가 통과한다.

## 7. 위험과 대응

| 위험 | 영향 | 대응 |
|---|---|---|
| Windows GNU Rust가 한글 경로에서 실패 | 로컬 Rust 테스트 차단 | ASCII 경로 또는 MSVC Build Tools를 표준 검증 조건으로 문서화 |
| MSVC `link.exe` 미설치 | MSVC 테스트 차단 | Visual Studio Build Tools 설치를 Windows 게이트 선행 조건으로 고정 |
| PowerShell `npm.ps1` 실행 정책 차단 | 웹 빌드 오탐 실패 | Windows 검증 명령은 `cmd /c npm run build`로 표준화 |
| hwpctl executor만 보고 구현률 산정 | 실제 호환률 과대 표시 | `compatibilityStatus` 기준으로 구현률 계산 |
