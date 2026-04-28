# 한컴 호환 요구사항 추적표

**기준일**: 2026-04-24

이 문서는 RHWP의 기능을 한컴 한글 호환성 관점에서 추적하기 위한 기준표다. 구현 여부는 단순 실행 가능 여부가 아니라 파일 열기, 렌더링, 편집, 저장, 재열기, 웹/Ubuntu/Windows 검증을 통과했는지로 판단한다.

## 상태 정의

| 상태 | 의미 |
|---|---|
| 완료 | 한컴 기준 핵심 동작을 3플랫폼에서 검증 완료 |
| 부분 | 기본 동작은 있으나 한컴 세부 규칙, 저장 재열기, 플랫폼 검증이 남음 |
| 스텁 | 메뉴 또는 Action은 있으나 사용자 기대 동작을 수행하지 않음 |
| 미지원 | 현재 범위 밖이거나 RHWP가 의도적으로 지원하지 않음 |

## 요구사항 매트릭스

| ID | 축 | 한컴 기준 기능 | RHWP 현재 연결점 | hwpctl/명령 | 테스트/코퍼스 | 플랫폼 게이트 | 우선순위 | 상태 |
|---|---|---|---|---|---|---|---|---|
| FILE-001 | 파일 | 새 문서 | Studio 새 문서 흐름, Rust 문서 모델 | `New` 계열 Action 후보 | 신규 코퍼스 필요 | 웹/Ubuntu/Windows 새 문서 저장 | P0 | 부분 |
| FILE-002 | 파일 | HWP/HWPX 열기 | Rust parser, WASM loader, Studio file open | `parse`, `compat-report` | `phase1-supported.tsv`, `hancom-quality-manifest.tsv` | 3플랫폼 열기 | P0 | 부분 |
| FILE-003 | 파일 | 저장, 다른 이름 저장, 저장 후 재열기 | save/roundtrip 경로 | `compat-corpus`, `compat-report` | `roundtrip_mode=save-reparse` | 3플랫폼 저장 재열기 | P0 | 부분 |
| FILE-004 | 파일 | PDF/SVG/JPG 내보내기 | CLI export, Studio export 후보 | `export-svg` | SVG 골든 필요 | 웹/데스크톱 내보내기 | P1 | 부분 |
| FILE-005 | 파일 | 손상/보호 문서 보호 보기 | compatibility report, protected view | `compat-report` | `phase1-protected.tsv` | 저장 손상 방지 | P0 | 부분 |
| VIEW-001 | 보기 | 쪽 보기, 확대/축소 | Studio status/zoom UI | `render` | E2E 렌더링 시나리오 | 3플랫폼 확대/축소 | P1 | 부분 |
| VIEW-002 | 보기 | 인쇄 미리보기 | SVG/PDF 렌더링 기반 | `export-svg`, PDF 후보 | 골든 페이지 비교 | 데스크톱 인쇄 스모크 | P1 | 스텁 |
| INPUT-001 | 입력 | 본문 텍스트 입력 | Studio edit command, WASM edit | `InsertText` | text-flow E2E | 저장 재열기 | P0 | 부분 |
| INPUT-002 | 입력 | 문단/쪽/단 나누기 | edit command | `BreakPara`, `BreakPage`, `BreakColumn` | shift-return, break 샘플 | 3플랫폼 저장 재열기 | P0 | 부분 |
| INPUT-003 | 입력 | 필드 입력 | parser/field preservation | 필드 Action 후보 | `samples/field-01.hwp` | 저장 재열기 | P0 | 스텁 |
| INPUT-004 | 입력 | 하이퍼링크 | field/hyperlink preservation | Hyperlink Action 후보 | `fixtures/hyperlink-field.hwpx` | 클릭, 저장 재열기 | P0 | 부분 |
| INPUT-005 | 입력 | 그림/도형/수식 | parser/render preservation | `compat-report` | `eq-01.hwp`, shape fixtures | 좌표 허용오차 | P1 | 부분 |
| FORMAT-001 | 서식 | 글자 모양 | hwpctl format bridge | `CharShape`, `CharShapeBold` | 폰트/서식 샘플 | 저장 재열기 | P0 | 부분 |
| FORMAT-002 | 서식 | 문단 모양 | hwpctl format bridge | `ParagraphShape` | 줄간격/들여쓰기 샘플 | 저장 재열기 | P0 | 부분 |
| FORMAT-003 | 서식 | 스타일 적용 | Studio style bar 후보 | Style Action 후보 | 신규 스타일 코퍼스 | 3플랫폼 적용/저장 | P1 | 스텁 |
| PAGE-001 | 쪽 | 편집 용지 | page def bridge | `PageSetup` | 페이지 크기 골든 | 3플랫폼 렌더/저장 | P0 | 부분 |
| PAGE-002 | 쪽 | 머리말/꼬리말 | header/footer parser and command | `HeaderFooter` | `phase1-header-footer.hwpx` | 렌더/저장 재열기 | P0 | 부분 |
| PAGE-003 | 쪽 | 각주/미주 | parser/render | Note Action 후보 | `footnote-01.hwp`, `endnote-01.hwp` | 번호/위치 검증 | P0 | 부분 |
| PAGE-004 | 쪽 | 쪽 번호 위치 | page num model 후보 | `PageNumPos` | 신규 코퍼스 필요 | 저장 재열기 | P0 | 스텁 |
| PAGE-005 | 쪽 | 구역/다단 | section/col def 후보 | `BreakSection`, `BreakColDef` | 다단 샘플 필요 | 레이아웃 허용오차 | P0 | 스텁 |
| TABLE-001 | 표 | 표 만들기 | hwpctl table bridge | `TableCreate` | table E2E | 저장 재열기 | P0 | 부분 |
| TABLE-002 | 표 | 줄/칸 삽입 삭제 | table edit bridge | `TableInsertRowColumn`, `TableDeleteRowColumn` | table E2E | 저장 재열기 | P0 | 부분 |
| TABLE-003 | 표 | 셀 나누기/합치기 | table edit bridge | `TableSplitCell`, Merge 후보 | table fixture 필요 | 저장 재열기 | P0 | 부분 |
| TABLE-004 | 표 | 표 테두리/배경 | cell style bridge | `CellBorderFill` | table style fixture | 저장 재열기 | P1 | 부분 |
| TABLE-005 | 표 | 너비/높이 같게, 블록 계산식 | table layout/edit 후보 | Action 후보 | 신규 코퍼스 필요 | 저장 재열기 | P1 | 스텁 |
| REVIEW-001 | 검토 | 찾기/바꾸기 | Studio command 후보 | Find/Replace Action 후보 | E2E 필요 | 3플랫폼 UI | P1 | 스텁 |
| REVIEW-002 | 검토 | 맞춤법 | 외부 엔진 정책 필요 | Spell Action 후보 | 정책 문서 필요 | 3플랫폼 UI | P2 | 미지원 |
| REVIEW-003 | 검토 | 주석 | parser/render preservation | Comment Action 후보 | `hidden-comment.hwpx` | 렌더/저장 재열기 | P0 | 부분 |
| SECURITY-001 | 보안 | 문서 암호 | protected view/security model | 보안 명령 후보 | 보호 문서 코퍼스 필요 | 저장 손상 방지 | P0 | 스텁 |
| SECURITY-002 | 보안 | 폰트/라이선스 경고 | font fallback strategy | `--font-style`, `--embed-fonts` | font fallback 샘플 | 3플랫폼 폰트 경고 | P1 | 부분 |
| PLATFORM-001 | 플랫폼 | 웹 정적 배포 | `rhwp-studio` Vite build | `cmd /c npm run build` | web build | Windows/Web | P0 | 부분 |
| PLATFORM-002 | 플랫폼 | Ubuntu 데스크톱 | `.deb` 후보 | packaging command 후보 | 설치 스모크 필요 | Ubuntu 22.04/24.04 | P0 | 스텁 |
| PLATFORM-003 | 플랫폼 | Windows 데스크톱 | NSIS/MSI 후보 | installer command 후보 | 설치 스모크 필요 | Windows 파일 연결 | P0 | 스텁 |

## 신규 기능 등록 규칙

1. 기능 구현 전 이 표에 요구사항 ID를 추가한다.
2. hwpctl Action이 있으면 `compatibilityStatus`를 지정한다.
3. 코퍼스 샘플 또는 E2E 테스트를 연결한다.
4. 웹, Ubuntu, Windows 완료 조건을 분리해서 기록한다.
5. `partial` 또는 `stub` 상태를 숨기지 않는다.
