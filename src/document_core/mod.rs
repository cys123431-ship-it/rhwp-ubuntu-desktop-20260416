//! 문서 핵심 도메인 모델
//!
//! HWP 문서의 도메인 상태와 로직을 캡슐화한다.
//! WASM/PyO3/MCP 등 어떤 어댑터에서도 독립적으로 사용할 수 있다.

pub(crate) mod helpers;
pub(crate) use helpers::*;

mod commands;
pub(crate) mod html_table_import;
mod queries;
pub mod table_calc;

use crate::model::document::Document;
use crate::model::event::DocumentEvent;
use crate::model::paragraph::Paragraph;
use crate::parser::hwpx::reader::HwpxPackageSnapshot;
use crate::renderer::composer::ComposedParagraph;
use crate::renderer::height_measurer::{MeasuredSection, MeasuredTable};
use crate::renderer::layout::LayoutEngine;
use crate::renderer::pagination::PaginationResult;
use crate::renderer::render_tree::PageRenderTree;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::DEFAULT_DPI;
use std::cell::RefCell;
use std::collections::HashMap;

/// 기본 폰트 fallback 경로
pub const DEFAULT_FALLBACK_FONT: &str = "/usr/share/fonts/truetype/nanum/NanumGothic.ttf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentSourceFormat {
    Hwp,
    Hwpx,
    Unknown,
}

impl DocumentSourceFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            DocumentSourceFormat::Hwp => "hwp",
            DocumentSourceFormat::Hwpx => "hwpx",
            DocumentSourceFormat::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentEditMode {
    EditableSafe,
    ProtectedView,
}

impl DocumentEditMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            DocumentEditMode::EditableSafe => "editable-safe",
            DocumentEditMode::ProtectedView => "protected-view",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityIssueEntry {
    pub code: &'static str,
    pub severity: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReportData {
    pub source_format: DocumentSourceFormat,
    pub preferred_save_format: DocumentSourceFormat,
    pub edit_mode: DocumentEditMode,
    pub issues: Vec<CompatibilityIssueEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSubstitutionEntry {
    pub lang: String,
    pub original: String,
    pub resolved: String,
    pub substituted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSubstitutionReportData {
    pub fallback_font: String,
    pub items: Vec<FontSubstitutionEntry>,
}

/// 내부 클립보드 데이터
pub(crate) struct ClipboardData {
    /// 복사된 문단들 (서식 정보 포함)
    pub(crate) paragraphs: Vec<Paragraph>,
    /// 플레인 텍스트
    pub(crate) plain_text: String,
}

/// HWP 문서 핵심 도메인 모델
///
/// 문서 데이터, 레이아웃 상태, 설정, 캐시를 포함한다.
/// WASM 바인딩 없이 순수 Rust 타입만 사용한다.
pub struct DocumentCore {
    /// IR 문서
    pub(crate) document: Document,
    /// 페이지 분할 결과
    pub(crate) pagination: Vec<PaginationResult>,
    /// 해소된 스타일 세트
    pub(crate) styles: ResolvedStyleSet,
    /// 구역별 구성된 문단 목록
    pub(crate) composed: Vec<Vec<ComposedParagraph>>,
    /// DPI
    pub(crate) dpi: f64,
    /// 대체 폰트 경로
    pub(crate) fallback_font: String,
    /// 레이아웃 엔진 (자동 번호 카운터 포함)
    pub(crate) layout_engine: LayoutEngine,
    /// 내부 클립보드
    pub(crate) clipboard: Option<ClipboardData>,
    /// 문단부호(¶) 표시 여부
    pub(crate) show_paragraph_marks: bool,
    /// 조판부호 표시 여부 (개체 마커 [표]/[그림] 등, 문단부호 포함)
    pub(crate) show_control_codes: bool,
    /// 투명선 표시 여부
    pub(crate) show_transparent_borders: bool,
    /// 잘림 보기 (body/셀 클리핑 활성화 여부)
    pub(crate) clip_enabled: bool,
    /// 디버그 오버레이 표시 여부 (문단/표 경계 + pi/ci 라벨)
    pub(crate) debug_overlay: bool,
    /// 구역별 표 측정 데이터 (페이지네이션 결과 보존)
    pub(crate) measured_tables: Vec<Vec<MeasuredTable>>,
    /// 구역별 dirty 플래그 (true = 재페이지네이션 필요)
    pub(crate) dirty_sections: Vec<bool>,
    /// 구역별 측정 캐시 (증분 측정용)
    pub(crate) measured_sections: Vec<MeasuredSection>,
    /// 구역별 문단 dirty 비트맵.
    /// None = 전체 dirty (초기 로드 또는 전체 재구성 시).
    /// Some(vec) = vec[para_idx] = true이면 해당 문단만 재측정.
    pub(crate) dirty_paragraphs: Vec<Option<Vec<bool>>>,
    /// 구역별 문단→단 인덱스 매핑 (페이지네이션에서 결정)
    /// para_column_map[section_idx][para_idx] = column_index
    pub(crate) para_column_map: Vec<Vec<u16>>,
    /// 페이지별 렌더 트리 캐시 (지연 구축, 부분 무효화)
    pub(crate) page_tree_cache: RefCell<Vec<Option<PageRenderTree>>>,
    /// Batch 모드 플래그 — true이면 paginate() 스킵
    pub(crate) batch_mode: bool,
    /// 이벤트 로그 (Command 실행 시 누적)
    pub(crate) event_log: Vec<DocumentEvent>,
    /// 글상자 오버플로우 연결 캐시 (섹션별, 지연 계산)
    pub(crate) overflow_links_cache:
        RefCell<HashMap<usize, Vec<queries::doc_tree_nav::OverflowLink>>>,
    /// Undo/Redo용 Document 스냅샷 저장소 (ID → Document 클론)
    pub(crate) snapshot_store: Vec<(u32, Document)>,
    /// 다음 스냅샷 ID
    pub(crate) next_snapshot_id: u32,
    /// 머리말/꼬리말 감추기: (global_page_index, is_header) 조합
    pub(crate) hidden_header_footer: std::collections::HashSet<(u32, bool)>,
    /// 파일 이름 (머리말/꼬리말 필드 치환용)
    pub(crate) file_name: String,
    /// 파일 경로 (데스크톱 세션 메타데이터)
    pub(crate) file_path: String,
    /// 원본 문서 포맷
    pub(crate) source_format: DocumentSourceFormat,
    /// 저장되지 않은 변경 여부
    pub(crate) dirty: bool,
    /// 원본 HWPX 패키지 바이트 (보호 모드 round-trip 보존용)
    pub(crate) hwpx_package_snapshot: Option<HwpxPackageSnapshot>,
    /// 현재 활성 필드 위치 (커서가 진입한 누름틀 — 안내문 렌더링 스킵용)
    /// (section_idx, para_idx, field_control_idx)
    pub(crate) active_field: Option<ActiveFieldInfo>,
    /// 구역별 문단 인덱스 오프셋 (삽입=+N, 삭제=-N, 페이지네이션 수렴 감지용)
    /// paginate() 후 리셋.
    pub(crate) para_offset: Vec<i32>,
}

/// 활성 필드 위치 정보
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveFieldInfo {
    pub section_idx: usize,
    pub para_idx: usize,
    /// field_ranges의 control_idx (controls[] 내 Field 컨트롤 인덱스)
    pub control_idx: usize,
    /// 셀 내부 필드인 경우의 전체 경로
    /// 단일 표: vec![(parent_para_idx, ctrl, cell)]
    /// 중첩 표: vec![(outer_ctrl, outer_cell, ..), (inner_ctrl, inner_cell, ..)]
    /// parent_para_idx는 별도 필드에 포함하지 않고 첫 번째 요소의 context로 사용
    pub cell_path: Option<Vec<(usize, usize, usize)>>, // Vec<(parent_para_idx_or_ctrl, ctrl_or_cell, cell_or_para)>
}

impl DocumentCore {
    fn compatibility_issue_entries(&self) -> Vec<(&'static str, &'static str, String)> {
        let mut issues = Vec::new();

        if self.document.header.encrypted {
            issues.push((
                "encrypted-document",
                "blocker",
                "Encrypted documents open in protected view during phase 1.".to_string(),
            ));
        }
        if self.document.header.distribution {
            issues.push((
                "distribution-document",
                "blocker",
                "Distribution documents open in protected view to avoid save corruption."
                    .to_string(),
            ));
        }
        if !self.document.doc_info.font_faces.is_empty() {
            issues.push((
                "font-substitution",
                "warning",
                "Layout may differ if required Hancom fonts are missing on this system."
                    .to_string(),
            ));
        }
        if self.source_format == DocumentSourceFormat::Hwpx {
            let report = self.hwpx_support_report();
            issues.extend(
                report
                    .issues
                    .into_iter()
                    .filter_map(|issue| match issue.code {
                        "hwpx-encrypted-document" | "hwpx-distribution-document" => None,
                        _ => Some((issue.code, issue.severity.as_str(), issue.message)),
                    }),
            );
        }

        issues
    }

    fn font_substitution_entries(&self) -> Vec<(String, String, String, bool)> {
        use crate::renderer::style_resolver::resolve_font_substitution;

        let lang_names = [
            "hangul", "latin", "hanja", "japanese", "other", "symbol", "user",
        ];
        let mut entries = std::collections::BTreeSet::new();

        for (lang_idx, lang_fonts) in self.document.doc_info.font_faces.iter().enumerate() {
            for font in lang_fonts {
                let resolved = resolve_font_substitution(&font.name, font.alt_type, lang_idx)
                    .unwrap_or(&font.name)
                    .to_string();
                let substituted = resolved != font.name;
                entries.insert((
                    lang_names.get(lang_idx).unwrap_or(&"hangul").to_string(),
                    font.name.clone(),
                    resolved,
                    substituted,
                ));
            }
        }

        entries.into_iter().collect()
    }

    /// 총 페이지 수를 반환한다.
    pub fn page_count(&self) -> u32 {
        self.pagination
            .iter()
            .map(|pr| pr.pages.len() as u32)
            .sum::<u32>()
            .max(1)
    }

    pub fn source_format(&self) -> DocumentSourceFormat {
        self.source_format
    }

    pub fn preferred_save_format(&self) -> DocumentSourceFormat {
        match self.source_format {
            DocumentSourceFormat::Unknown => DocumentSourceFormat::Hwp,
            fmt => fmt,
        }
    }

    fn hwpx_support_report(&self) -> crate::serializer::HwpxSupportReport {
        crate::serializer::analyze_hwpx_support_with_context(
            &self.document,
            crate::serializer::HwpxPreservationContext {
                snapshot: self.hwpx_package_snapshot.as_ref(),
                dirty_sections: Some(&self.dirty_sections),
                doc_info_dirty: self.document.doc_info.raw_stream_dirty,
            },
        )
    }

    pub fn edit_mode(&self) -> DocumentEditMode {
        if self.document.header.encrypted || self.document.header.distribution {
            return DocumentEditMode::ProtectedView;
        }

        if self.source_format == DocumentSourceFormat::Hwpx
            && !self.hwpx_support_report().is_supported()
        {
            return DocumentEditMode::ProtectedView;
        }

        DocumentEditMode::EditableSafe
    }

    pub fn is_protected_view(&self) -> bool {
        self.edit_mode() == DocumentEditMode::ProtectedView
    }

    pub fn document_blockers(&self) -> Vec<String> {
        let mut blockers = Vec::new();

        if self.document.header.encrypted {
            blockers.push("Encrypted documents open in protected view during phase 1.".to_string());
        }
        if self.document.header.distribution {
            blockers.push(
                "Distribution documents open in protected view to avoid save corruption."
                    .to_string(),
            );
        }
        if self.source_format == DocumentSourceFormat::Hwpx {
            blockers.extend(self.hwpx_support_report().blockers);
        }

        blockers
    }

    pub fn document_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if !self.document.doc_info.font_faces.is_empty() {
            warnings.push(
                "Layout may differ if required Hancom fonts are missing on this system."
                    .to_string(),
            );
        }
        if self.source_format == DocumentSourceFormat::Hwpx {
            let report = self.hwpx_support_report();
            warnings.extend(report.warnings);
            warnings.extend(report.infos);
        }

        warnings
    }

    pub fn get_compatibility_report(&self) -> String {
        let report = self.compatibility_report_data();
        let issues = report
            .issues
            .into_iter()
            .map(|issue| {
                format!(
                    concat!(
                        "{{",
                        "\"code\":\"{}\",",
                        "\"severity\":\"{}\",",
                        "\"message\":\"{}\"",
                        "}}"
                    ),
                    issue.code,
                    issue.severity,
                    json_escape(&issue.message),
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        format!(
            concat!(
                "{{",
                "\"sourceFormat\":\"{}\",",
                "\"preferredSaveFormat\":\"{}\",",
                "\"editMode\":\"{}\",",
                "\"issues\":[{}]",
                "}}"
            ),
            report.source_format.as_str(),
            report.preferred_save_format.as_str(),
            report.edit_mode.as_str(),
            issues,
        )
    }

    pub fn get_font_substitution_report(&self) -> String {
        let report = self.font_substitution_report_data();
        let entries = report
            .items
            .into_iter()
            .map(|item| {
                format!(
                    concat!(
                        "{{",
                        "\"lang\":\"{}\",",
                        "\"original\":\"{}\",",
                        "\"resolved\":\"{}\",",
                        "\"substituted\":{}",
                        "}}"
                    ),
                    json_escape(&item.lang),
                    json_escape(&item.original),
                    json_escape(&item.resolved),
                    item.substituted,
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        format!(
            concat!("{{", "\"fallbackFont\":\"{}\",", "\"items\":[{}]", "}}"),
            json_escape(&report.fallback_font),
            entries,
        )
    }

    pub fn compatibility_report_data(&self) -> CompatibilityReportData {
        CompatibilityReportData {
            source_format: self.source_format(),
            preferred_save_format: self.preferred_save_format(),
            edit_mode: self.edit_mode(),
            issues: self
                .compatibility_issue_entries()
                .into_iter()
                .map(|(code, severity, message)| CompatibilityIssueEntry {
                    code,
                    severity,
                    message,
                })
                .collect(),
        }
    }

    pub fn font_substitution_report_data(&self) -> FontSubstitutionReportData {
        FontSubstitutionReportData {
            fallback_font: self.fallback_font.clone(),
            items: self
                .font_substitution_entries()
                .into_iter()
                .map(
                    |(lang, original, resolved, substituted)| FontSubstitutionEntry {
                        lang,
                        original,
                        resolved,
                        substituted,
                    },
                )
                .collect(),
        }
    }

    pub fn get_document_capabilities(&self) -> String {
        let blockers = self.document_blockers();
        let warnings = self.document_warnings();
        let blockers_json = blockers
            .iter()
            .map(|item| format!("\"{}\"", json_escape(item)))
            .collect::<Vec<_>>()
            .join(",");
        let warnings_json = warnings
            .iter()
            .map(|item| format!("\"{}\"", json_escape(item)))
            .collect::<Vec<_>>()
            .join(",");
        let hwpx_report = self.hwpx_support_report();
        let can_save_hwp = !self.is_protected_view();
        let can_save_hwpx = hwpx_report.is_supported();

        format!(
            concat!(
                "{{",
                "\"sourceFormat\":\"{}\",",
                "\"preferredSaveFormat\":\"{}\",",
                "\"editMode\":\"{}\",",
                "\"isProtected\":{},",
                "\"dirty\":{},",
                "\"encrypted\":{},",
                "\"distribution\":{},",
                "\"canSaveHwp\":{},",
                "\"canSaveHwpx\":{},",
                "\"filePath\":\"{}\",",
                "\"blockers\":[{}],",
                "\"warnings\":[{}]",
                "}}"
            ),
            self.source_format.as_str(),
            self.preferred_save_format().as_str(),
            self.edit_mode().as_str(),
            self.is_protected_view(),
            self.dirty,
            self.document.header.encrypted,
            self.document.header.distribution,
            can_save_hwp,
            can_save_hwpx,
            json_escape(&self.file_path),
            blockers_json,
            warnings_json,
        )
    }

    /// 문서 정보를 JSON 문자열로 반환한다.
    pub fn get_document_info(&self) -> String {
        use crate::renderer::style_resolver::resolve_font_substitution;

        let mut fonts = std::collections::BTreeSet::new();
        for (lang_idx, lang_fonts) in self.document.doc_info.font_faces.iter().enumerate() {
            for font in lang_fonts {
                let resolved = resolve_font_substitution(&font.name, font.alt_type, lang_idx)
                    .unwrap_or(&font.name);
                fonts.insert(resolved.to_string());
            }
        }
        let fonts_json: Vec<String> = fonts
            .iter()
            .map(|f| {
                // 폰트 이름의 특수문자를 JSON 이스케이프 처리
                let escaped: String = f
                    .chars()
                    .flat_map(|c| match c {
                        '"' => vec!['\\', '"'],
                        '\\' => vec!['\\', '\\'],
                        '\n' => vec!['\\', 'n'],
                        '\r' => vec!['\\', 'r'],
                        '\t' => vec!['\\', 't'],
                        c if c < '\x20' => vec![],
                        c => vec![c],
                    })
                    .collect();
                format!("\"{}\"", escaped)
            })
            .collect();

        let escaped_fallback: String = self
            .fallback_font
            .chars()
            .flat_map(|c| match c {
                '"' => vec!['\\', '"'],
                '\\' => vec!['\\', '\\'],
                c => vec![c],
            })
            .collect();
        format!(
            concat!(
                "{{",
                "\"version\":\"{}.{}.{}.{}\",",
                "\"sectionCount\":{},",
                "\"pageCount\":{},",
                "\"encrypted\":{},",
                "\"distribution\":{},",
                "\"sourceFormat\":\"{}\",",
                "\"dirty\":{},",
                "\"fallbackFont\":\"{}\",",
                "\"fontsUsed\":[{}]",
                "}}"
            ),
            self.document.header.version.major,
            self.document.header.version.minor,
            self.document.header.version.build,
            self.document.header.version.revision,
            self.document.sections.len(),
            self.page_count(),
            self.document.header.encrypted,
            self.document.header.distribution,
            self.source_format.as_str(),
            self.dirty,
            escaped_fallback,
            fonts_json.join(","),
        )
    }

    /// 이벤트 로그를 JSON 배열로 직렬화한다.
    pub fn serialize_event_log(&self) -> String {
        crate::model::event::serialize_event_log(&self.event_log)
    }

    /// DPI를 설정하고 스타일을 재해소한 후 재페이지네이션한다.
    pub fn set_dpi(&mut self, dpi: f64) {
        use crate::renderer::style_resolver::resolve_styles;
        self.dpi = dpi;
        self.styles = resolve_styles(&self.document.doc_info, dpi);
        self.paginate();
    }

    /// 빈 문서를 생성한다 (테스트/미리보기용).
    pub fn new_empty() -> Self {
        DocumentCore {
            document: Document::default(),
            pagination: Vec::new(),
            styles: ResolvedStyleSet::default(),
            composed: Vec::new(),
            dpi: DEFAULT_DPI,
            fallback_font: DEFAULT_FALLBACK_FONT.to_string(),
            layout_engine: LayoutEngine::new(DEFAULT_DPI),
            clipboard: None,
            show_paragraph_marks: false,
            show_control_codes: false,
            show_transparent_borders: false,
            clip_enabled: true,
            debug_overlay: false,
            measured_tables: Vec::new(),
            dirty_sections: Vec::new(),
            measured_sections: Vec::new(),
            dirty_paragraphs: Vec::new(),
            para_column_map: Vec::new(),
            page_tree_cache: RefCell::new(Vec::new()),
            batch_mode: false,
            event_log: Vec::new(),
            overflow_links_cache: RefCell::new(HashMap::new()),
            snapshot_store: Vec::new(),
            next_snapshot_id: 0,
            hidden_header_footer: std::collections::HashSet::new(),
            file_name: String::new(),
            file_path: String::new(),
            source_format: DocumentSourceFormat::Unknown,
            dirty: false,
            hwpx_package_snapshot: None,
            active_field: None,
            para_offset: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::paragraph::Paragraph;
    use crate::model::style::Font;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn compatibility_report_marks_encrypted_documents_as_blocked() {
        let mut core = DocumentCore::new_empty();
        core.source_format = DocumentSourceFormat::Hwpx;
        core.document.header.encrypted = true;
        core.file_path = "/tmp/report.hwpx".to_string();

        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();
        let issues = report["issues"].as_array().unwrap();

        assert_eq!(report["sourceFormat"], "hwpx");
        assert_eq!(report["preferredSaveFormat"], "hwpx");
        assert_eq!(report["editMode"], "protected-view");
        assert!(issues.iter().any(|issue| {
            issue["code"] == "encrypted-document"
                && issue["severity"] == "blocker"
                && issue["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("protected view")
        }));
    }

    #[test]
    fn font_substitution_report_lists_declared_fonts() {
        let mut core = DocumentCore::new_empty();
        core.document.doc_info.font_faces = vec![vec![Font {
            name: "CustomMissingFont".to_string(),
            ..Default::default()
        }]];

        let report: Value = serde_json::from_str(&core.get_font_substitution_report()).unwrap();
        let items = report["items"].as_array().unwrap();

        assert_eq!(report["fallbackFont"], DEFAULT_FALLBACK_FONT);
        assert!(items
            .iter()
            .any(|item| { item["lang"] == "hangul" && item["original"] == "CustomMissingFont" }));
    }

    #[test]
    fn compatibility_report_uses_feature_issue_codes_for_hwpx_blockers() {
        let mut core = DocumentCore::new_empty();
        core.source_format = DocumentSourceFormat::Hwpx;

        let mut paragraph = Paragraph::new_empty();
        paragraph
            .controls
            .push(crate::model::control::Control::Shape(Box::new(
                crate::model::shape::ShapeObject::Curve(Default::default()),
            )));
        core.document
            .sections
            .push(crate::model::document::Section {
                paragraphs: vec![paragraph],
                ..Default::default()
            });

        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();
        let issues = report["issues"].as_array().unwrap();
        assert!(issues
            .iter()
            .any(|issue| issue["code"] == "hwpx-shape-curve"));
    }

    #[test]
    fn compatibility_report_snapshot_for_tac_img_wave2_sample() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples/tac-img-02.hwpx");
        let bytes = std::fs::read(sample).expect("read tac-img-02.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse tac-img-02.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("tac_img_wave2_compatibility_report", report);
    }

    #[test]
    fn compatibility_report_snapshot_for_form_sample() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("rhwp-studio/public/samples/form-002.hwpx");
        let bytes = std::fs::read(sample).expect("read form-002.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse form-002.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("form_002_compatibility_report", report);
    }

    #[test]
    fn compatibility_report_snapshot_for_table_vpos_preservation_sample() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples/table-vpos-01.hwpx");
        let bytes = std::fs::read(sample).expect("read table-vpos-01.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse table-vpos-01.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("table_vpos_preservation_compatibility_report", report);
    }

    #[test]
    fn compatibility_report_snapshot_for_equation_fixture() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("compatibility-corpus/fixtures/equation-basic.hwpx");
        let bytes = std::fs::read(sample).expect("read equation-basic.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse equation-basic.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("equation_basic_compatibility_report", report);
    }

    #[test]
    fn compatibility_report_snapshot_for_ruby_fixture() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("compatibility-corpus/fixtures/ruby-basic.hwpx");
        let bytes = std::fs::read(sample).expect("read ruby-basic.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse ruby-basic.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("ruby_basic_compatibility_report", report);
    }

    #[test]
    fn compatibility_report_snapshot_for_hidden_comment_fixture() {
        let sample = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("compatibility-corpus/fixtures/hidden-comment.hwpx");
        let bytes = std::fs::read(sample).expect("read hidden-comment.hwpx");
        let core = DocumentCore::from_bytes(&bytes).expect("parse hidden-comment.hwpx");
        let report: Value = serde_json::from_str(&core.get_compatibility_report()).unwrap();

        insta::assert_json_snapshot!("hidden_comment_compatibility_report", report);
    }
}
