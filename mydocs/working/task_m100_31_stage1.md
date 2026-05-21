# 단계별 완료 보고서 — Stage 1

**이슈**: #31 (임시 번호)  
**타이틀**: 에디터 캔버스 텍스트 가독성 개선 및 데스크톱 버전 표시 정합성 수정 — 1단계 완료 보고서  
**마일스톤**: M100  
**작성일**: 2026-05-21  

---

## 1단계 작업 내용

### [canvas-view.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/view/canvas-view.ts) 내 캔버스 배치 위치 정수 픽셀(Pixel Snap) 계산 및 적용 완료

1. **`renderPage` 내 정수 좌표 배치 알고리즘 적용 (L157-L164)**:
   - 캔버스 중앙 정렬 시 사용되던 `left: '50%'` 및 `transform = 'translateX(-50%)'` 속성을 완전히 걷어냈습니다.
   - 부모 스크롤 콘텐츠 영역의 너비(`clientWidth`)와 캔버스의 논리적인 CSS 너비(`pageInfo.width * zoom`)를 기반으로 정수로 딱 떨어지는 left 좌표값을 동적으로 계산하였습니다.
   - `Math.max(0, Math.floor((parentWidth - cssWidth) / 2))`를 통해 음수가 되지 않는 정확한 정수 left 픽셀 좌표값을 추출하여 직접 스타일로 주입했습니다.
   - `transform = 'none'`을 명시하여 브라우저의 GPU 가속에 의한 미세 스케일링 번짐(Subpixel Interpolation)을 완벽히 차단했습니다.
2. **`refreshPages` 내 정수 좌표 배치 알고리즘 적용 (L285-L292)**:
   - 본문 문서 타이핑 및 편집 수정 후의 픽셀 갱신(Re-render) 지점인 `refreshPages`에도 동일하게 정수화된 left 좌표 계산식을 이식하였습니다.
   - 줌 변경이나 창 크기 조절 시에도 일시적인 픽셀 어긋남 없이 정교하게 픽셀 스냅 그리드가 보존되도록 안정성을 높였습니다.

---

## 검증 결과

- `rhwp-studio/src/view/canvas-view.ts` 가 TypeScript 문법 오류 없이 완벽히 수정된 것을 확인했습니다.
- 동적 계산 공식 `Math.floor((parentWidth - cssWidth) / 2)` 가 `parentWidth` 와 `cssWidth` 의 실제 소수점 단위 계측값들을 깔끔하게 정수 픽셀 단위로 스냅하여 브라우저 빌리니어 흐림 필터를 회피하기 위한 환경을 갖추었습니다.
