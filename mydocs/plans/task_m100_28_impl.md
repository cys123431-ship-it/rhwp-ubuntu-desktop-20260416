# Task M100 #28 구현 계획서: 리눅스 드래그 블록 선택/자동 스크롤 수정

## 1. 개요
리눅스 WebKitGTK/Tauri 환경에서 드래그 중 브라우저 경계를 벗어나면 선택이 멈추는 문제를 해결하고, 뷰포트 상단 및 하단 경계선에서 마우스를 유지할 때 자동 스크롤(Edge Auto-scroll) 기능을 추가합니다.
(작업지시자의 지시에 따라, 승인 과정을 생략하고 즉시 코드를 수정 후 빌드/테스트를 완수하였습니다.)

## 2. 변경 사항 정리
### 2.1. 글로벌 `mousemove` 이벤트 바인딩 추가
- **파일**: `rhwp-studio/src/engine/input-handler-mouse.ts` ( `onClick`, `onMouseUp` 등)
- **변경 내용**:
  - `mousedown` 시점에 기존에 있던 `document.addEventListener('mouseup', ...)`에 더하여 `document.addEventListener('mousemove', this.onMouseMoveBound)`를 추가하여 브라우저 창 바깥으로 포인터가 나가더라도 드래그 이벤트를 수신하도록 개선.
  - `onMouseUp` 시점에 등록한 글로벌 `mousemove` 이벤트를 제거 (`removeEventListener`).

### 2.2. 가장자리 자동 스크롤(Edge Auto-scroll) 구현
- **파일**: `rhwp-studio/src/engine/input-handler.ts`, `input-handler-mouse.ts`
- **변경 내용**:
  - `InputHandler` 클래스 필드에 `autoScrollRafId`(자동 스크롤 루프용), `lastDragEvent`(마지막 마우스 위치 저장용) 추가.
  - `onClick` 내부에서 드래그를 시작할 때 `requestAnimationFrame`을 이용한 `autoScrollLoop`를 가동.
  - 마우스 `clientY` 위치가 컨테이너 경계 기준 상하 40px(Threshold) 이내일 때, 지속적으로 `scrollTop`을 조정(-20px / +20px).
  - 스크롤이 변경되면 `hitTestFromEvent`를 통해 커서를 업데이트하고 캐럿 렌더링 갱신.
  - `onMouseUp` 및 드래그 해제 시(`isDragging = false`) `cancelAnimationFrame`을 통해 루프 해제.

## 3. 검증 결과
- `npm run build`를 통한 TypeScript 컴파일 확인 완료.
- E2E 테스트(`selection-visual.test.mjs`) 실행하여 블록 선택 기본 기능 회귀 없음 확인 완료.
- 향후, 마우스 드래그를 통한 글로벌 자동 스크롤 동작을 위해 Puppeteer에서 `mouse.move()`를 세밀하게 제어하는 추가 E2E 스크립트(`selection-drag.test.mjs`)를 고려할 수 있음.

(위 구현 내역이 코드에 전부 반영되었으며, 빌드 또한 정상 완료되었습니다.)
