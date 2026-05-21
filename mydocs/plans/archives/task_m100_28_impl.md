# 구현 계획서 — Task #28

**이슈**: #28 임시 번호 (GitHub CLI 미설치로 원격 이슈 생성 보류)  
**타이틀**: 리눅스 드래그 블록 선택/최신 설치 안정화  
**마일스톤**: M100  
**브랜치**: `local/task28`  
**작성일**: 2026-05-21

---

## 코드 파악 결과

### 드래그 선택 흐름

```
InputHandler constructor
  └─ #scroll-container mousedown/mousemove 등록
      └─ onClick()
          ├─ hitTestFromEvent()
          ├─ cursor.moveTo()
          ├─ cursor.setAnchor()
          ├─ isDragging = true
          └─ document mouseup 1회 등록
      └─ onMouseMove()
          └─ isDragging이면 hitTestFromEvent() → cursor.moveTo() → updateCaret()
```

### 핵심 파일

| 파일 | 역할 |
|------|------|
| `rhwp-studio/src/engine/input-handler.ts` | 이벤트 등록, caret/selection 공통 처리, 스크롤 보정 |
| `rhwp-studio/src/engine/input-handler-mouse.ts` | 마우스 클릭/드래그/해제 처리 |
| `rhwp-studio/src/engine/selection-renderer.ts` | 선택 영역 DOM 렌더링 |
| `rhwp-studio/e2e/selection-visual.test.mjs` | 기존 선택 시각 테스트 |
| `src/renderer/composer/re_sample_gen.rs` | `samples/re-*.hwp` 생성 테스트 |
| `rhwp-desktop/src-tauri/tauri.conf.json` | Tauri 패키지 표시 버전 |
| `rhwp-desktop/src-tauri/Cargo.toml` | 데스크톱 Rust 패키지 버전 |

---

## 단계별 구현 계획

### 1단계: 드래그 선택 이벤트 수신 안정화

**목표**: 드래그 중 포인터가 페이지/컨테이너 경계를 벗어나도 선택 갱신이 계속되게 한다.

**변경 방안**:

- 텍스트 선택 드래그 시작 시 전용 드래그 리스너를 등록한다.
  - `document.addEventListener('mousemove', onMouseMoveBound)`
  - `document.addEventListener('mouseup', onMouseUpBound, { once: true })`
- 드래그 종료 시 전역 `mousemove`를 해제한다.
- 가능하면 `PointerEvent`/`setPointerCapture`를 검토하되, 기존 마우스 기반 코드와 충돌을 줄이기 위해 1차 구현은 전역 mousemove 보강을 우선한다.
- 기존 그림/표 이동·리사이즈 드래그와 충돌하지 않도록 텍스트 선택 드래그 상태에서만 적용한다.

**검증**:

- 위→아래 드래그 선택
- 아래→위 드래그 선택
- 편집 영역 상단/하단 밖으로 살짝 나간 상태에서 선택 유지

### 2단계: 드래그 선택 자동 스크롤

**목표**: 선택 드래그 중 편집 영역 상단/하단 가장자리에서 문서가 자연스럽게 스크롤되고 선택 끝점이 갱신되게 한다.

**변경 방안**:

- `isDragging` 상태에서 마우스 y좌표가 컨테이너 상/하단 임계값 안에 들어오면 requestAnimationFrame 루프를 시작한다.
- 스크롤 속도는 가장자리와의 거리 기반으로 제한한다.
- 스크롤이 진행될 때 마지막 마우스 좌표로 `hitTestFromEvent()`를 반복 호출해 선택 focus를 갱신한다.
- 드래그 종료 시 자동 스크롤 루프와 저장 좌표를 정리한다.

**검증**:

- 다중 줄 문단을 아래 방향으로 선택하며 하단 자동 스크롤
- 문서 아래쪽에서 위 방향으로 선택하며 상단 자동 스크롤
- 자동 스크롤 종료 후 단순 클릭 선택 해제 동작 유지

### 3단계: 마우스 드래그 선택 E2E 추가

**목표**: 사용자 제보 경로를 자동 테스트로 고정한다.

**변경 방안**:

- `rhwp-studio/e2e/selection-drag.test.mjs` 신규 작성
- 새 문서 생성 → 긴 한글 문단 입력 → 렌더 완료 대기
- DOM selection layer 또는 `window.__inputHandler.getSelection()`로 검증
- 케이스:
  - 위→아래 드래그 선택
  - 아래→위 드래그 선택
  - 컨테이너 경계 밖으로 살짝 끌어도 선택 유지

**검증**:

- headless Chrome에서 신규 E2E 통과
- 기존 `selection-visual.test.mjs` 회귀 없음

### 4단계: 버전/패키지 일관화 및 최신 로컬 설치

**목표**: `v0.1.12` 태그와 실제 설치본의 버전을 맞춘다.

**변경 방안**:

- `rhwp-desktop/package.json`
- `rhwp-desktop/package-lock.json`
- `rhwp-desktop/src-tauri/tauri.conf.json`
- `rhwp-desktop/src-tauri/Cargo.toml`

위 네 파일의 버전을 `0.1.12`로 일치시킨다.

**검증**:

- Tauri 빌드 산출물 이름이 `rhwp_0.1.12_amd64.deb`인지 확인
- 로컬 설치 후 `dpkg -s rhwp` 버전이 `0.1.12`인지 확인

### 5단계: 테스트 fixture 오염 및 포맷 수정

**목표**: 기본 검증 명령이 작업 트리를 오염시키지 않게 한다.

**변경 방안**:

- `src/renderer/composer/re_sample_gen.rs`의 샘플 생성 테스트를 기본 `cargo test`에서 제외한다.
- 수동 실행 목적이 분명하므로 `#[ignore]`를 붙이거나 출력 디렉터리를 `output/`/임시 디렉터리로 변경한다.
- `cargo fmt`가 요구하는 기존 포맷 차이를 반영한다.

**검증**:

- 깨끗한 상태에서 `cargo test` 실행 후 `git status --short`가 새 오염을 만들지 않는지 확인
- `cargo fmt --check` 통과

### 6단계: 통합 검증, 보고서, 설치

**목표**: 구현 산출물을 검증하고 로컬 최신 설치를 완료한다.

**검증 명령**:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cd rhwp-studio && npx tsc --noEmit
cd rhwp-studio && npx vite build
cd rhwp-studio && node e2e/selection-visual.test.mjs --mode=headless
cd rhwp-studio && node e2e/selection-drag.test.mjs --mode=headless
cd rhwp-desktop && npm run build
dpkg -s rhwp
```

**문서 산출물**:

- `mydocs/working/task_m100_28_stage1.md`
- `mydocs/working/task_m100_28_stage2.md`
- `mydocs/working/task_m100_28_stage3.md`
- `mydocs/working/task_m100_28_stage4.md`
- `mydocs/working/task_m100_28_stage5.md`
- `mydocs/working/task_m100_28_report.md`

---

## 예상 변경 파일

| 파일 | 변경 내용 |
|------|-----------|
| `rhwp-studio/src/engine/input-handler.ts` | 드래그 전역 리스너/자동 스크롤 상태와 정리 로직 |
| `rhwp-studio/src/engine/input-handler-mouse.ts` | 드래그 시작/이동/종료 처리 보강 |
| `rhwp-studio/e2e/selection-drag.test.mjs` | 사용자 제보 경로 회귀 테스트 |
| `src/renderer/composer/re_sample_gen.rs` | 샘플 생성 테스트 기본 실행 제외 또는 출력 격리 |
| `src/document_core/mod.rs` | rustfmt 반영 |
| `src/wasm_api/tests.rs` | rustfmt 반영 |
| `rhwp-desktop/package.json` | 버전 `0.1.12` |
| `rhwp-desktop/package-lock.json` | 버전 `0.1.12` |
| `rhwp-desktop/src-tauri/tauri.conf.json` | 버전 `0.1.12` |
| `rhwp-desktop/src-tauri/Cargo.toml` | 버전 `0.1.12` |
| `mydocs/orders/20260521.md` | 오늘할일 갱신 |
| `mydocs/plans/task_m100_28.md` | 수행 계획서 |
| `mydocs/plans/task_m100_28_impl.md` | 구현 계획서 |

---

## 승인 요청

위 구현 계획을 승인해주시면 1단계부터 구현을 시작하겠습니다.
