# 단계별 완료 보고서 — Stage 2

**이슈**: #29  
**타이틀**: 웹 접근성(A11y) 개선 및 Tauri 데스크톱 보안 샌드박싱 추가 — 2단계 완료 보고서  
**마일스톤**: M100  
**작성일**: 2026-05-21  

---

## 2단계 작업 내용

### [toolbar.ts](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/src/ui/toolbar.ts) 동적 접근성 제어 로직 보강 완료

1. **동적 `aria-pressed` 상태 동기화**:
   - `Toolbar.setActive(btn, active)` 공통 비공개 메서드 내부에 `btn.setAttribute('aria-pressed', String(active))` 추가 적용.
   - 이를 통해 진하게, 기울임, 밑줄, 취소선, 글자 효과 등의 개별 속성 토글 시 스크린 리더 등 보조 기기가 상태 변화를 즉각 인지할 수 있도록 보장.
2. **드롭다운 동적 확장 상태 (`aria-expanded`) 처리**:
   - **글자 효과 드롭다운 (`charfxDropdown`)**: 드롭다운 버튼 클릭으로 열릴 때 `aria-expanded="true"`, 메뉴 항목 클릭이나 외부 마우스 클릭으로 닫힐 때 `aria-expanded="false"` 동적 토글 기능 적용.
   - **형광펜 드롭다운 (`highlightDropdown`)**: 팔레트 토글 버튼 클릭 시 `aria-expanded` 속성 동적 토글 및 "색 없음", "다른 색...", 각 색상 스워치 클릭 및 외부 클릭으로 닫힐 때 `false`로의 원격 갱신 보강.
3. **문단 정렬 상태 업데이트 보강**:
   - `updateParaState(props: ParaProperties)` 함수 내에서 `props.alignment` 상태를 받아와 왼쪽, 가운데, 오른쪽, 양쪽, 배분, 나눔 정렬 버튼들의 `active` 클래스와 `aria-pressed` 속성을 실시간 동화하도록 정렬 상태 동화 루프 구현 완료.

---

## 검증 결과

- 웹 브라우저 콘솔 및 DOM 검사를 통해 Bold/Italic/Underline 버튼 클릭 시 `aria-pressed="true"`와 `aria-pressed="false"`가 실시간으로 동적 교체됨을 확인하였습니다.
- 글자 효과 버튼 및 형광펜 버튼 클릭 시 `aria-expanded` 속성이 `"true"`와 `"false"`로 정상 제어됨을 확인하였습니다.
- 문단의 정렬 상태가 변경될 때 대응하는 정렬 단추의 `aria-pressed` 값이 정확하게 연동됨을 교차 검증하였습니다.
