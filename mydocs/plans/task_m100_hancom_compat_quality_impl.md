# 구현 계획서: M100 한컴 호환형 3플랫폼 품질 로드맵

**이슈**: 미지정(사용자 직접 지시)
**마일스톤**: M100
**기준일**: 2026-04-24

---

## 단계 1. hwpctl 호환 상태 모델 고정

**목표**: Action이 실행 함수 보유 여부만으로 구현 완료처럼 보이지 않게 한다.

**작업**
- `ActionCompatibilityStatus = implemented | partial | stub | unsupported` 타입 추가
- `ActionDef`, `Action`, `ActionRegistry`, `HwpCtrl` 공개 API에 상태 노출
- 대표 Action의 상태를 실제 호환 수준에 맞게 재분류
- E2E에서 상태 카운트와 미등록 Action의 `unsupported` 상태 검증

**완료 기준**
- `HwpCtrl.getActionStatusCounts()`가 상태별 개수를 반환한다.
- `HwpCtrl.getImplementedActionCount()`는 `implemented` 상태만 집계한다.
- 미등록 Action은 `unsupported`로 반환된다.

## 단계 2. 한컴 호환 요구사항 추적표 작성

**목표**: 한컴 공식 기능 축을 RHWP의 코드, 명령, 테스트, 코퍼스와 연결한다.

**작업**
- 기능 축을 `파일/보기/입력/서식/쪽/표/검토/보안/플랫폼`으로 분류
- 각 항목에 RHWP 산출물, hwpctl Action 또는 CLI 명령, 테스트, 3플랫폼 게이트를 연결
- 우선순위를 `P0/P1/P2`로 고정

**완료 기준**
- 신규 기능은 추적표 항목 없이 구현하지 않는다.
- 신규 스텁은 추적표와 이슈에 연결된다.

## 단계 3. 한컴 품질 코퍼스 매니페스트 확장

**목표**: 샘플 문서 단위로 한컴 호환 기대값을 기록한다.

**작업**
- `compatibility-corpus/hancom-quality-manifest.tsv` 추가
- 각 샘플에 원본 경로, 기능 영역, 예상 editMode, 저장 형식, 페이지 수, 레이아웃 허용오차, 저장 후 재파싱, 3플랫폼 게이트를 기록
- 기존 `phase1/phase2` 매니페스트와의 관계를 README에 명시

**완료 기준**
- 주요 샘플이 웹, Ubuntu, Windows 통과 여부를 같은 표에서 추적한다.
- 페이지 수와 좌표 허용오차가 비어 있으면 승격 전 `TBD`로 남아 있음을 명시한다.

## 단계 4. 3플랫폼 품질 게이트 표준화

**목표**: 로컬 검증 환경 차이를 문서와 스크립트로 고정한다.

**작업**
- Windows PowerShell `npm.ps1` 차단 회피 명령 표준화
- Windows GNU Rust의 한글 경로 문제와 MSVC `link.exe` 선행 조건 명시
- 웹, Ubuntu, Windows 데스크톱, 코퍼스, 릴리즈 후보 체크리스트 작성
- `scripts/windows-local-check.ps1`로 환경 진단과 웹 빌드 실행을 자동화

**완료 기준**
- Windows에서 왜 Rust 검증이 막히는지 결과가 명확히 나온다.
- 웹 빌드는 `cmd /c npm run build`로 재현된다.

## 단계 5. 기능 보강 타스크로 분해

**목표**: 3개월 로드맵의 실제 기능 구현을 독립 타스크로 분해한다.

**우선 타스크**
- 필드 입력
- 하이퍼링크 편집
- 주석/미주
- 캡션 방향별 삽입
- 쪽 번호 위치
- 구역/다단 세부 설정
- 블록 계산식
- 표 너비/높이 같게

**완료 기준**
- 각 기능은 Rust `DocumentCore`에서 시작해 WASM API, Studio Command, 플랫폼 IO, 저장 후 재열기까지 연결된다.
- 웹만 통과한 기능은 완료로 보지 않는다.
