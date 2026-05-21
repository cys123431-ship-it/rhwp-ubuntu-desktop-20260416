# 단계별 완료 보고서 — Stage 1

**이슈**: #29  
**타이틀**: 웹 접근성(A11y) 개선 및 Tauri 데스크톱 보안 샌드박싱 추가 — 1단계 완료 보고서  
**마일스톤**: M100  
**작성일**: 2026-05-21  

---

## 1단계 작업 내용

### [index.html](file:///home/joseph/바탕화면/개발/hangul/rhwp-studio/index.html) 정적 접근성 마크업 보강 완료

1. **도구 상자 (#icon-toolbar) 및 서식 도구 모음 (#style-bar) 내 모든 인터랙티브 요소 접근성 조치**:
   - 모든 서식/도구 단추(`tb-btn`, `sb-btn`, `sb-arrow`, `sb-dropdown-item`)에 고유하고 구체적인 한글 `aria-label` 부여.
   - 키보드 조작성을 높이기 위해 `tabindex="0"` 속성 부여.
   - 마우스 호버 시 툴팁으로 제공되는 `title` 속성과 `aria-label`을 일치시켜 정보의 대칭성 및 편의성 극대화.
2. **토글 서식 상태 표현**:
   - 진하게, 기울임, 밑줄, 취소선, 정렬 상태 버튼 등에 기본적으로 `aria-pressed="false"` 주입.
3. **드롭다운 및 콤보박스**:
   - 글자 효과, 형광펜, 글자색 단추 등 드롭다운 형태의 버튼에는 `aria-haspopup="true"`, `aria-expanded="false"` 등을 적용하여 보조 공학 기기가 메뉴의 동적 확장 상태를 감지할 수 있도록 보강.
   - 줄 간격 선택 콤보박스(`linespacing-select`) 및 글자 크기 증가/감소 단추에도 각기 `aria-label`과 `tabindex="0"`을 적용.

---

## 검증 결과

- 웹 브라우저 요소 검사를 통해 HTML 코드 상에 `aria-label`, `tabindex="0"`, `aria-pressed`가 의도한 대로 노출되는지 확인하였습니다.
- 키보드 `Tab` 키 조작 시 서식 도구 모음의 단추들로 포커스가 순차적으로 진입 및 이동하는 것을 확인하였습니다.
