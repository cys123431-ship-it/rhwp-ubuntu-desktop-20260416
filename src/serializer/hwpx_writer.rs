use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::model::control::{
    AutoNumber, AutoNumberType, Bookmark, Control, Field, FieldType, NewNumber, PageHide,
    PageNumberPos,
};
use crate::model::document::{Document, Section, SectionDef};
use crate::model::header_footer::HeaderFooterApply;
use crate::model::image::{ImageEffect, Picture};
use crate::model::page::{ColumnDef, ColumnDirection, ColumnType, PageDef};
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::model::shape::{
    Caption, CaptionDirection, CaptionVertAlign, CommonObjAttr, HorzAlign, HorzRelTo,
    ShapeObject, TextWrap, VertAlign, VertRelTo,
};
use crate::model::style::{
    Alignment, BorderFill, BorderLine, BorderLineType, Bullet, CharShape, FillType, Font,
    HeadType, ImageFillMode, LineSpacingType, Numbering, ParaShape, TabDef, UnderlineType,
};
use crate::model::table::{Cell, Table, TablePageBreak, VerticalAlign};
use crate::parser::hwpx::reader::{HwpxPackageEntryRole, HwpxPackageSnapshot};
use crate::document_core::helpers::find_control_text_positions;

use super::cfb_writer::SerializeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HwpxIssueSeverity {
    Blocker,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HwpxIssueScope {
    Document,
    DocInfo,
    Section(usize),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HwpxPreservationContext<'a> {
    pub snapshot: Option<&'a HwpxPackageSnapshot>,
    pub dirty_sections: Option<&'a [bool]>,
    pub doc_info_dirty: bool,
}

#[cfg(test)]
mod tests_clean {
    use super::*;
    use crate::model::document::Document;
    use crate::model::paragraph::{CharShapeRef, FieldRange, Paragraph};
    use proptest::prelude::*;

    #[test]
    fn test_serialize_hwpx_roundtrip_text() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());

        let paragraph = Paragraph {
            text: "Hello HWPX".to_string(),
            char_offsets: (0..10).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            para_shape_id: 0,
            style_id: 0,
            char_count: 11,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].paragraphs[0].text, "Hello HWPX");
    }

    #[test]
    fn test_analyze_hwpx_support_allows_supported_line_shapes() {
        let mut document = Document::default();
        let mut paragraph = Paragraph::new_empty();
        paragraph.controls.push(Control::Shape(Box::new(
            crate::model::shape::ShapeObject::Line(Default::default()),
        )));
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(report.is_supported(), "{:?}", report.blockers);
        assert!(!report
            .issues
            .iter()
            .any(|issue| issue.code.starts_with("hwpx-shape")));
    }

    #[test]
    fn test_analyze_hwpx_support_rejects_curve_shapes() {
        let mut document = Document::default();
        let mut paragraph = Paragraph::new_empty();
        paragraph.controls.push(Control::Shape(Box::new(
            crate::model::shape::ShapeObject::Curve(Default::default()),
        )));
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(!report.is_supported());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "hwpx-shape-curve"));
    }

    #[test]
    fn test_serialize_hwpx_roundtrip_bullet_document() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape {
            head_type: HeadType::Bullet,
            numbering_id: 1,
            ..Default::default()
        });
        document.doc_info.bullets.push(Bullet {
            bullet_char: '*',
            width_adjust: 12,
            text_distance: 50,
            ..Default::default()
        });

        let paragraph = Paragraph {
            text: "Bullet item".to_string(),
            char_offsets: (0..11).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            para_shape_id: 0,
            style_id: 0,
            char_count: 12,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        assert_eq!(parsed.doc_info.bullets.len(), 1);
        assert_eq!(parsed.doc_info.bullets[0].bullet_char, '*');
        assert_eq!(parsed.doc_info.para_shapes[0].head_type, HeadType::Bullet);
        assert_eq!(parsed.doc_info.para_shapes[0].numbering_id, 1);
    }

    #[test]
    fn test_serialize_hwpx_roundtrip_field_ranges() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());

        let paragraph = Paragraph {
            text: "inside".to_string(),
            char_offsets: (0..6).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            field_ranges: vec![FieldRange {
                start_char_idx: 0,
                end_char_idx: 6,
                control_idx: 0,
            }],
            controls: vec![Control::Field(Field {
                field_type: FieldType::ClickHere,
                command: Field::build_clickhere_command("inside", "", "Sample"),
                properties: 1,
                field_id: 77,
                ctrl_id: 7,
                ctrl_data_name: Some("Sample".to_string()),
                ..Default::default()
            })],
            para_shape_id: 0,
            style_id: 0,
            char_count: 7,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(report.is_supported(), "{:?}", report.blockers);

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        let para = &parsed.sections[0].paragraphs[0];
        assert_eq!(para.text, "inside");
        assert_eq!(para.field_ranges.len(), 1);
        assert_eq!(para.field_ranges[0].start_char_idx, 0);
        assert_eq!(para.field_ranges[0].end_char_idx, 6);
        assert!(matches!(para.controls[0], Control::Field(_)));
    }

    #[test]
    fn test_analyze_hwpx_support_uses_stable_issue_codes() {
        let mut document = Document::default();
        document.doc_info.extra_records.push(Default::default());

        let report = analyze_hwpx_support(&document);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "hwpx-docinfo-extra-records"));
    }

    #[test]
    fn test_preservation_context_downgrades_clean_section_metadata_blockers() {
        let mut document = Document::default();
        document.sections.push(Section {
            section_def: SectionDef {
                extra_child_records: vec![Default::default()],
                ..Default::default()
            },
            ..Default::default()
        });

        let snapshot = HwpxPackageSnapshot {
            entries: vec![crate::parser::hwpx::reader::HwpxPackageEntrySnapshot {
                path: "Contents/section0.xml".to_string(),
                bytes: b"<hp:section/>".to_vec(),
                role: HwpxPackageEntryRole::Section(0),
                replaceable: true,
            }],
        };

        let clean_report = analyze_hwpx_support_with_context(
            &document,
            HwpxPreservationContext {
                snapshot: Some(&snapshot),
                dirty_sections: Some(&[false]),
                doc_info_dirty: false,
            },
        );
        assert!(clean_report.is_supported());
        assert!(clean_report
            .issues
            .iter()
            .any(|issue| issue.code == "hwpx-section-extra-records" && issue.severity == HwpxIssueSeverity::Warning));

        let dirty_report = analyze_hwpx_support_with_context(
            &document,
            HwpxPreservationContext {
                snapshot: Some(&snapshot),
                dirty_sections: Some(&[true]),
                doc_info_dirty: false,
            },
        );
        assert!(!dirty_report.is_supported());
        assert!(dirty_report
            .issues
            .iter()
            .any(|issue| issue.code == "hwpx-section-extra-records" && issue.severity == HwpxIssueSeverity::Blocker));
    }

    #[test]
    fn test_supported_shape_roundtrip_textbox_and_caption() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());

        let text_box = crate::model::shape::TextBox {
            list_attr: 0x20,
            vertical_align: crate::model::table::VerticalAlign::Top,
            margin_left: 200,
            margin_right: 200,
            margin_top: 200,
            margin_bottom: 200,
            max_width: 7200,
            raw_list_header_extra: vec![0; 13],
            paragraphs: vec![Paragraph {
                text: "textbox".to_string(),
                char_offsets: (0..7).collect(),
                char_shapes: vec![CharShapeRef {
                    start_pos: 0,
                    char_shape_id: 0,
                }],
                para_shape_id: 0,
                style_id: 0,
                char_count: 8,
                has_para_text: true,
                ..Default::default()
            }],
        };
        let caption = crate::model::shape::Caption {
            direction: crate::model::shape::CaptionDirection::Bottom,
            vert_align: crate::model::shape::CaptionVertAlign::Top,
            width: 7200,
            spacing: 200,
            max_width: 7200,
            include_margin: false,
            paragraphs: vec![Paragraph {
                text: "caption".to_string(),
                char_offsets: (0..7).collect(),
                char_shapes: vec![CharShapeRef {
                    start_pos: 0,
                    char_shape_id: 0,
                }],
                para_shape_id: 0,
                style_id: 0,
                char_count: 8,
                has_para_text: true,
                ..Default::default()
            }],
        };
        let rect = crate::model::shape::RectangleShape {
            common: crate::model::shape::CommonObjAttr {
                ctrl_id: 0x2472_6563,
                width: 7200,
                height: 4800,
                horizontal_offset: 1200,
                vertical_offset: 900,
                z_order: 1,
                instance_id: 0x4100_0101,
                ..Default::default()
            },
            drawing: crate::model::shape::DrawingObjAttr {
                shape_attr: crate::model::shape::ShapeComponentAttr {
                    ctrl_id: 0x2472_6563,
                    original_width: 7200,
                    original_height: 4800,
                    current_width: 7200,
                    current_height: 4800,
                    local_file_version: 1,
                    ..Default::default()
                },
                border_line: crate::model::style::ShapeBorderLine {
                    color: 0,
                    width: 33,
                    attr: 0xD1000041,
                    outline_style: 0,
                },
                fill: crate::model::style::Fill {
                    fill_type: FillType::Solid,
                    solid: Some(crate::model::style::SolidFill {
                        background_color: 0x00FF_FFFF,
                        pattern_color: 0,
                        pattern_type: -1,
                    }),
                    gradient: None,
                    image: None,
                    alpha: 0,
                },
                inst_id: 1,
                text_box: Some(text_box),
                caption: Some(caption),
                ..Default::default()
            },
            round_rate: 15,
            x_coords: [0, 7200, 7200, 0],
            y_coords: [0, 0, 4800, 4800],
        };

        let mut paragraph = Paragraph::new_empty();
        paragraph.controls.push(Control::Shape(Box::new(
            crate::model::shape::ShapeObject::Rectangle(rect),
        )));
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(report.is_supported(), "{:?}", report.blockers);

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        let parsed_para = &parsed.sections[0].paragraphs[0];
        let Control::Shape(shape) = &parsed_para.controls[0] else {
            panic!("expected shape control");
        };
        let crate::model::shape::ShapeObject::Rectangle(rect) = shape.as_ref() else {
            panic!("expected rectangle shape");
        };
        assert_eq!(
            rect.drawing
                .text_box
                .as_ref()
                .expect("textbox")
                .paragraphs[0]
                .text,
            "textbox"
        );
        assert_eq!(
            rect.drawing
                .caption
                .as_ref()
                .expect("caption")
                .paragraphs[0]
                .text,
            "caption"
        );
    }

    #[test]
    fn test_clean_snapshot_only_unsupported_shape_is_warning_until_dirty() {
        let document = Document::default();
        let snapshot = HwpxPackageSnapshot {
            entries: vec![crate::parser::hwpx::reader::HwpxPackageEntrySnapshot {
                path: "Contents/section0.xml".to_string(),
                bytes: b"<hp:section><hp:connectLine /></hp:section>".to_vec(),
                role: HwpxPackageEntryRole::Section(0),
                replaceable: true,
            }],
        };

        let clean_report = analyze_hwpx_support_with_context(
            &document,
            HwpxPreservationContext {
                snapshot: Some(&snapshot),
                dirty_sections: Some(&[false]),
                doc_info_dirty: false,
            },
        );
        assert!(clean_report.is_supported());
        assert!(clean_report.issues.iter().any(|issue| {
            issue.code == "hwpx-shape-unsupported" && issue.severity == HwpxIssueSeverity::Warning
        }));

        let dirty_report = analyze_hwpx_support_with_context(
            &document,
            HwpxPreservationContext {
                snapshot: Some(&snapshot),
                dirty_sections: Some(&[true]),
                doc_info_dirty: false,
            },
        );
        assert!(!dirty_report.is_supported());
        assert!(dirty_report.issues.iter().any(|issue| {
            issue.code == "hwpx-shape-unsupported" && issue.severity == HwpxIssueSeverity::Blocker
        }));
    }

    #[test]
    fn test_serialize_hwpx_with_context_preserves_untouched_entries() {
        use std::io::Write;
        use zip::ZipWriter;

        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());
        document.sections.push(Section {
            paragraphs: vec![Paragraph {
                text: "Snapshot".to_string(),
                char_offsets: (0..8).collect(),
                char_shapes: vec![CharShapeRef {
                    start_pos: 0,
                    char_shape_id: 0,
                }],
                para_shape_id: 0,
                style_id: 0,
                char_count: 8,
                has_para_text: true,
                ..Default::default()
            }],
            ..Default::default()
        });

        let base = serialize_hwpx(&document).expect("serialize hwpx");
        let base_snapshot = HwpxPackageSnapshot::from_bytes(&base).expect("snapshot");
        let mut out = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = ZipWriter::new(&mut out);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for entry in &base_snapshot.entries {
                writer.start_file(&entry.path, options).unwrap();
                writer.write_all(&entry.bytes).unwrap();
            }
            writer.start_file("META-INF/custom.xml", options).unwrap();
            writer.write_all(b"<custom preserved=\"yes\"/>").unwrap();
            writer.finish().unwrap();
        }

        let snapshot = HwpxPackageSnapshot::from_bytes(&out.into_inner()).expect("snapshot");
        let saved = serialize_hwpx_with_context(
            &document,
            HwpxPreservationContext {
                snapshot: Some(&snapshot),
                dirty_sections: Some(&[false]),
                doc_info_dirty: false,
            },
        )
        .expect("save with snapshot");
        let saved_snapshot = HwpxPackageSnapshot::from_bytes(&saved).expect("saved snapshot");
        let extra = saved_snapshot
            .entry("META-INF/custom.xml")
            .expect("preserved extra entry");
        assert_eq!(extra.bytes, b"<custom preserved=\"yes\"/>");
    }

    proptest! {
        #[test]
        fn prop_supported_rectangle_textbox_roundtrip_stays_editable_safe(text in "[A-Za-z0-9 ]{1,16}") {
            let mut document = Document::default();
            document.doc_info.font_faces = vec![Vec::new(); 7];
            document.doc_info.char_shapes.push(CharShape::default());
            document.doc_info.para_shapes.push(ParaShape::default());

            let text_len = text.chars().count() as u32;
            let text_box = crate::model::shape::TextBox {
                list_attr: 0x20,
                vertical_align: crate::model::table::VerticalAlign::Top,
                margin_left: 180,
                margin_right: 180,
                margin_top: 180,
                margin_bottom: 180,
                max_width: 6400,
                raw_list_header_extra: vec![0; 13],
                paragraphs: vec![Paragraph {
                    text: text.clone(),
                    char_offsets: (0..text_len).collect(),
                    char_shapes: vec![CharShapeRef {
                        start_pos: 0,
                        char_shape_id: 0,
                    }],
                    para_shape_id: 0,
                    style_id: 0,
                    char_count: text_len + 1,
                    has_para_text: true,
                    ..Default::default()
                }],
            };
            let rect = crate::model::shape::RectangleShape {
                common: crate::model::shape::CommonObjAttr {
                    ctrl_id: 0x2472_6563,
                    width: 6400,
                    height: 4200,
                    instance_id: 0x4100_0201,
                    ..Default::default()
                },
                drawing: crate::model::shape::DrawingObjAttr {
                    shape_attr: crate::model::shape::ShapeComponentAttr {
                        ctrl_id: 0x2472_6563,
                        original_width: 6400,
                        original_height: 4200,
                        current_width: 6400,
                        current_height: 4200,
                        ..Default::default()
                    },
                    text_box: Some(text_box),
                    ..Default::default()
                },
                x_coords: [0, 6400, 6400, 0],
                y_coords: [0, 0, 4200, 4200],
                ..Default::default()
            };

            let mut paragraph = Paragraph::new_empty();
            paragraph.controls.push(Control::Shape(Box::new(
                crate::model::shape::ShapeObject::Rectangle(rect),
            )));
            document.sections.push(Section {
                paragraphs: vec![paragraph],
                ..Default::default()
            });

            let report = analyze_hwpx_support(&document);
            prop_assert!(report.is_supported(), "{:?}", report.blockers);

            let bytes = serialize_hwpx(&document).expect("serialize hwpx");
            let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
            let reparsed_report = analyze_hwpx_support(&parsed);
            prop_assert!(reparsed_report.is_supported(), "{:?}", reparsed_report.blockers);

            let Control::Shape(shape) = &parsed.sections[0].paragraphs[0].controls[0] else {
                panic!("expected shape control");
            };
            let crate::model::shape::ShapeObject::Rectangle(rect) = shape.as_ref() else {
                panic!("expected rectangle shape");
            };
            prop_assert_eq!(
                rect.drawing
                    .text_box
                    .as_ref()
                    .expect("textbox")
                    .paragraphs[0]
                    .text
                    .as_str(),
                text
            );
            prop_assert_ne!(rect.common.instance_id, 0);
        }
    }
}

impl HwpxIssueSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            HwpxIssueSeverity::Blocker => "blocker",
            HwpxIssueSeverity::Warning => "warning",
            HwpxIssueSeverity::Info => "info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HwpxSupportIssue {
    pub code: &'static str,
    pub severity: HwpxIssueSeverity,
    pub scope: HwpxIssueScope,
    pub message: String,
}

#[derive(Debug, Default, Clone)]
pub struct HwpxSupportReport {
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub infos: Vec<String>,
    pub issues: Vec<HwpxSupportIssue>,
}

impl HwpxSupportReport {
    fn push(
        &mut self,
        scope: HwpxIssueScope,
        severity: HwpxIssueSeverity,
        code: &'static str,
        msg: impl Into<String>,
    ) {
        let message = msg.into();
        self.issues.push(HwpxSupportIssue {
            code,
            severity,
            scope,
            message,
        });
    }

    fn block(&mut self, scope: HwpxIssueScope, code: &'static str, msg: impl Into<String>) {
        self.push(scope, HwpxIssueSeverity::Blocker, code, msg);
    }

    fn warn(&mut self, scope: HwpxIssueScope, code: &'static str, msg: impl Into<String>) {
        self.push(scope, HwpxIssueSeverity::Warning, code, msg);
    }

    fn info(&mut self, scope: HwpxIssueScope, code: &'static str, msg: impl Into<String>) {
        self.push(scope, HwpxIssueSeverity::Info, code, msg);
    }

    fn dedupe(&mut self) {
        let mut seen = BTreeSet::new();
        let mut deduped = Vec::new();
        for issue in &self.issues {
            if seen.insert((issue.severity, issue.scope, issue.code, issue.message.clone())) {
                deduped.push(issue.clone());
            }
        }
        self.issues = deduped;
        self.blockers = self
            .issues
            .iter()
            .filter(|issue| issue.severity == HwpxIssueSeverity::Blocker)
            .map(|issue| issue.message.clone())
            .collect();
        self.warnings = self
            .issues
            .iter()
            .filter(|issue| issue.severity == HwpxIssueSeverity::Warning)
            .map(|issue| issue.message.clone())
            .collect();
        self.infos = self
            .issues
            .iter()
            .filter(|issue| issue.severity == HwpxIssueSeverity::Info)
            .map(|issue| issue.message.clone())
            .collect();
    }

    pub fn is_supported(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == HwpxIssueSeverity::Blocker)
    }
}

pub fn analyze_hwpx_support(doc: &Document) -> HwpxSupportReport {
    let mut report = HwpxSupportReport::default();

    if doc.header.encrypted {
        report.block(
            HwpxIssueScope::Document,
            "hwpx-encrypted-document",
            "Encrypted documents cannot be saved as HWPX yet.",
        );
    }
    if doc.header.distribution {
        report.block(
            HwpxIssueScope::Document,
            "hwpx-distribution-document",
            "Distribution documents stay protected to avoid save corruption.",
        );
    }
    if !doc.doc_info.extra_records.is_empty() {
        report.block(
            HwpxIssueScope::DocInfo,
            "hwpx-docinfo-extra-records",
            "Extra DocInfo records are not preserved in HWPX saves yet.",
        );
    }
    if doc
        .doc_info
        .bullets
        .iter()
        .any(|bullet| bullet.image_bullet != 0 || bullet.bullet_char == '\u{FFFF}')
    {
        report.block(
            HwpxIssueScope::DocInfo,
            "hwpx-image-bullet",
            "Image bullets are not written by the HWPX serializer yet.",
        );
    }
    if !doc.extra_streams.is_empty() {
        report.block(
            HwpxIssueScope::Document,
            "hwpx-extra-binary-streams",
            "Extra binary streams are not preserved in HWPX packages yet.",
        );
    }
    if doc.preview.is_some() {
        report.warn(
            HwpxIssueScope::Document,
            "hwpx-preview-regenerated",
            "Preview streams are regenerated lazily and are not preserved in HWPX yet.",
        );
    }

    for (section_idx, section) in doc.sections.iter().enumerate() {
        analyze_section(section, section_idx, &mut report);
    }

    report.dedupe();
    report
}

pub fn analyze_hwpx_support_with_context(
    doc: &Document,
    context: HwpxPreservationContext<'_>,
) -> HwpxSupportReport {
    let mut report = analyze_hwpx_support(doc);
    if context.snapshot.is_none() {
        return report;
    }

    append_snapshot_shape_issues(&mut report, context);

    for issue in &mut report.issues {
        issue.severity = classify_issue_severity(issue, context);
    }
    report.dedupe();
    report
}

fn classify_issue_severity(
    issue: &HwpxSupportIssue,
    context: HwpxPreservationContext<'_>,
) -> HwpxIssueSeverity {
    match issue.code {
        "hwpx-encrypted-document"
        | "hwpx-distribution-document"
        | "hwpx-extra-binary-streams" => HwpxIssueSeverity::Blocker,
        "hwpx-preview-regenerated" => HwpxIssueSeverity::Info,
        "hwpx-docinfo-extra-records" | "hwpx-image-bullet" => {
            if !context.doc_info_dirty {
                HwpxIssueSeverity::Warning
            } else {
                HwpxIssueSeverity::Blocker
            }
        }
        "hwpx-section-page-border-fill"
        | "hwpx-section-master-pages"
        | "hwpx-section-extra-records"
        | "hwpx-shape-curve"
        | "hwpx-shape-group-unsupported-child"
        | "hwpx-shape-unsupported" => {
            if issue_scope_is_clean(issue.scope, context.dirty_sections) {
                HwpxIssueSeverity::Warning
            } else {
                HwpxIssueSeverity::Blocker
            }
        }
        _ => issue.severity,
    }
}

fn issue_scope_is_clean(scope: HwpxIssueScope, dirty_sections: Option<&[bool]>) -> bool {
    match scope {
        HwpxIssueScope::Document => false,
        HwpxIssueScope::DocInfo => false,
        HwpxIssueScope::Section(section_idx) => dirty_sections
            .and_then(|sections| sections.get(section_idx))
            .copied()
            .map(|dirty| !dirty)
            .unwrap_or(false),
    }
}

fn append_snapshot_shape_issues(
    report: &mut HwpxSupportReport,
    context: HwpxPreservationContext<'_>,
) {
    let Some(snapshot) = context.snapshot else {
        return;
    };

    for entry in &snapshot.entries {
        let section_idx = match entry.role {
            HwpxPackageEntryRole::Section(index) => index,
            _ => continue,
        };
        let xml = String::from_utf8_lossy(&entry.bytes);
        if xml.contains("<hp:connectLine")
            || xml.contains("<connectLine")
            || xml.contains("<hp:ole")
            || xml.contains("<ole")
        {
            report.block(
                HwpxIssueScope::Section(section_idx),
                "hwpx-shape-unsupported",
                format!(
                    "Section {} contains preserved shape payloads that are not editable in HWPX yet.",
                    section_idx
                ),
            );
        }
    }
}

fn analyze_section(section: &Section, section_idx: usize, report: &mut HwpxSupportReport) {
    if section.section_def.page_border_fill.border_fill_id != 0 {
        report.block(HwpxIssueScope::Section(section_idx), "hwpx-section-page-border-fill", format!(
            "Section {} uses page border/fill settings that are not written to HWPX yet.",
            section_idx
        ));
    }
    if !section.section_def.master_pages.is_empty() {
        report.block(HwpxIssueScope::Section(section_idx), "hwpx-section-master-pages", format!(
            "Section {} uses master pages that are not written to HWPX yet.",
            section_idx
        ));
    }
    if !section.section_def.extra_page_border_fills.is_empty()
        || !section.section_def.extra_child_records.is_empty()
    {
        report.block(HwpxIssueScope::Section(section_idx), "hwpx-section-extra-records", format!(
            "Section {} carries extra section records that are not preserved in HWPX yet.",
            section_idx
        ));
    }

    for (para_idx, para) in section.paragraphs.iter().enumerate() {
        analyze_paragraph(para, section_idx, para_idx, report);
    }
}

fn analyze_paragraph(
    para: &Paragraph,
    section_idx: usize,
    para_idx: usize,
    report: &mut HwpxSupportReport,
) {
    if para.numbering_restart.is_some() {
        report.block(HwpxIssueScope::Section(section_idx), "hwpx-paragraph-numbering-restart", format!(
            "Paragraph {} in section {} uses numbering restarts that are not written to HWPX yet.",
            para_idx, section_idx
        ));
    }
    if para.text.chars().any(|ch| matches!(ch, '\u{0003}' | '\u{0004}')) {
        report.block(
            HwpxIssueScope::Section(section_idx),
            "hwpx-paragraph-stray-field-markers",
            format!(
                "Paragraph {} in section {} still contains raw field markers after parsing.",
                para_idx, section_idx
            ),
        );
    }

    for (control_idx, control) in para.controls.iter().enumerate() {
        analyze_control(control, para, control_idx, section_idx, para_idx, report);
    }
}

fn analyze_caption_paragraphs(
    caption: Option<&Caption>,
    section_idx: usize,
    para_idx: usize,
    report: &mut HwpxSupportReport,
) {
    if let Some(caption) = caption {
        for para in &caption.paragraphs {
            analyze_paragraph(para, section_idx, para_idx, report);
        }
    }
}

fn shape_issue_code(shape: &ShapeObject) -> Option<&'static str> {
    match shape {
        ShapeObject::Line(_)
        | ShapeObject::Rectangle(_)
        | ShapeObject::Ellipse(_)
        | ShapeObject::Arc(_)
        | ShapeObject::Polygon(_)
        | ShapeObject::Picture(_) => None,
        ShapeObject::Curve(_) => Some("hwpx-shape-curve"),
        ShapeObject::Group(group) => {
            if group.children.iter().all(|child| shape_issue_code(child).is_none()) {
                None
            } else {
                Some("hwpx-shape-group-unsupported-child")
            }
        }
    }
}

fn analyze_shape_object(
    shape: &ShapeObject,
    section_idx: usize,
    para_idx: usize,
    report: &mut HwpxSupportReport,
) {
    if let Some(code) = shape_issue_code(shape) {
        let message = match code {
            "hwpx-shape-curve" => format!(
                "Paragraph {} in section {} uses curve shapes that are not written to HWPX yet.",
                para_idx, section_idx
            ),
            _ => format!(
                "Paragraph {} in section {} contains grouped shapes with unsupported child payloads.",
                para_idx, section_idx
            ),
        };
        report.block(HwpxIssueScope::Section(section_idx), code, message);
        return;
    }

    match shape {
        ShapeObject::Line(line) => {
            analyze_caption_paragraphs(line.drawing.caption.as_ref(), section_idx, para_idx, report);
            if let Some(text_box) = line.drawing.text_box.as_ref() {
                for para in &text_box.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        ShapeObject::Rectangle(rect) => {
            analyze_caption_paragraphs(rect.drawing.caption.as_ref(), section_idx, para_idx, report);
            if let Some(text_box) = rect.drawing.text_box.as_ref() {
                for para in &text_box.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        ShapeObject::Ellipse(ellipse) => {
            analyze_caption_paragraphs(ellipse.drawing.caption.as_ref(), section_idx, para_idx, report);
            if let Some(text_box) = ellipse.drawing.text_box.as_ref() {
                for para in &text_box.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        ShapeObject::Arc(arc) => {
            analyze_caption_paragraphs(arc.drawing.caption.as_ref(), section_idx, para_idx, report);
            if let Some(text_box) = arc.drawing.text_box.as_ref() {
                for para in &text_box.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        ShapeObject::Polygon(polygon) => {
            analyze_caption_paragraphs(polygon.drawing.caption.as_ref(), section_idx, para_idx, report);
            if let Some(text_box) = polygon.drawing.text_box.as_ref() {
                for para in &text_box.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        ShapeObject::Curve(_) => {}
        ShapeObject::Group(group) => {
            analyze_caption_paragraphs(group.caption.as_ref(), section_idx, para_idx, report);
            for child in &group.children {
                analyze_shape_object(child, section_idx, para_idx, report);
            }
        }
        ShapeObject::Picture(picture) => {
            analyze_caption_paragraphs(picture.caption.as_ref(), section_idx, para_idx, report);
        }
    }
}

fn analyze_control(
    control: &Control,
    para: &Paragraph,
    control_idx: usize,
    section_idx: usize,
    para_idx: usize,
    report: &mut HwpxSupportReport,
) {
    match control {
        Control::SectionDef(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-inline-section-def", format!(
            "Paragraph {} in section {} has inline section-definition controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::ColumnDef(_) => {}
        Control::Table(table) => {
            for cell in &table.cells {
                for para in &cell.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        Control::Picture(pic) => {
            if let Some(caption) = pic.caption.as_ref() {
                for para in &caption.paragraphs {
                    analyze_paragraph(para, section_idx, para_idx, report);
                }
            }
        }
        Control::Header(header) => {
            for para in &header.paragraphs {
                analyze_paragraph(para, section_idx, para_idx, report);
            }
        }
        Control::Footer(footer) => {
            for para in &footer.paragraphs {
                analyze_paragraph(para, section_idx, para_idx, report);
            }
        }
        Control::Footnote(note) => {
            for para in &note.paragraphs {
                analyze_paragraph(para, section_idx, para_idx, report);
            }
        }
        Control::Endnote(note) => {
            for para in &note.paragraphs {
                analyze_paragraph(para, section_idx, para_idx, report);
            }
        }
        Control::AutoNumber(_)
        | Control::NewNumber(_)
        | Control::PageNumberPos(_)
        | Control::Bookmark(_)
        | Control::PageHide(_) => {}
        Control::Field(_) => {
            if !para
                .field_ranges
                .iter()
                .any(|range| range.control_idx == control_idx)
            {
                report.block(
                    HwpxIssueScope::Section(section_idx),
                    "hwpx-field-missing-range",
                    format!(
                        "Paragraph {} in section {} has a field control without a tracked text range.",
                        para_idx, section_idx
                    ),
                );
            }
        }
        Control::HiddenComment(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-hidden-comment", format!(
            "Paragraph {} in section {} uses hidden comments that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Shape(shape) => analyze_shape_object(shape, section_idx, para_idx, report),
        Control::Equation(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-equation", format!(
            "Paragraph {} in section {} uses equations that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Form(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-form", format!(
            "Paragraph {} in section {} uses form controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Hyperlink(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-hyperlink", format!(
            "Paragraph {} in section {} uses hyperlink controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Ruby(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-ruby", format!(
            "Paragraph {} in section {} uses ruby annotations that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::CharOverlap(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-char-overlap", format!(
            "Paragraph {} in section {} uses character-overlap controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Unknown(_) => report.block(HwpxIssueScope::Section(section_idx), "hwpx-unknown-control", format!(
            "Paragraph {} in section {} contains unknown controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
    }
}

pub fn serialize_hwpx(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    serialize_hwpx_with_context(doc, HwpxPreservationContext::default())
}

pub fn serialize_hwpx_with_context(
    doc: &Document,
    context: HwpxPreservationContext<'_>,
) -> Result<Vec<u8>, SerializeError> {
    let report = analyze_hwpx_support_with_context(doc, context);
    if !report.is_supported() {
        return Err(SerializeError::CfbError(report.blockers.join(" ")));
    }

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let section_paths = planned_section_paths(doc, context.snapshot);
    let bindata_paths = planned_bindata_paths(doc, context.snapshot);
    let content_xml = serialize_content_hpf(doc, &section_paths, &bindata_paths);
    let header_xml = serialize_header_xml(doc);

    let mut replacements = BTreeMap::new();
    let header_needs_rewrite = context.snapshot.is_none()
        || context.doc_info_dirty
        || context
            .snapshot
            .and_then(|snapshot| snapshot.entry("Contents/header.xml"))
            .is_none();
    if header_needs_rewrite {
        replacements.insert("Contents/header.xml".to_string(), header_xml.into_bytes());
    }

    let content_needs_rewrite = context.snapshot.is_none()
        || package_paths_changed(context.snapshot, &section_paths, &bindata_paths)
        || context
            .snapshot
            .and_then(|snapshot| snapshot.entry("Contents/content.hpf"))
            .is_none();
    if content_needs_rewrite {
        replacements.insert("Contents/content.hpf".to_string(), content_xml.into_bytes());
    }

    for (index, path) in section_paths.iter().enumerate() {
        if section_needs_rewrite(index, path, context) {
            let section = doc.sections.get(index).cloned().unwrap_or_default();
            let xml = serialize_section_xml(&section)?;
            replacements.insert(path.clone(), xml.into_bytes());
        }
    }

    for (index, path) in bindata_paths.iter().enumerate() {
        if let Some(content) = doc.bin_data_content.get(index) {
            if bindata_needs_rewrite(index, path, content.data.as_slice(), context) {
                replacements.insert(path.clone(), content.data.clone());
            }
        } else if context.snapshot.is_none()
            || context
                .snapshot
                .and_then(|snapshot| snapshot.entry(path))
                .is_none()
        {
            replacements.insert(path.clone(), Vec::new());
        }
    }

    let mut written_paths = BTreeSet::new();
    if let Some(snapshot) = context.snapshot {
        for entry in &snapshot.entries {
            if should_drop_snapshot_entry(entry, &section_paths, &bindata_paths) {
                continue;
            }
            if let Some(bytes) = replacements.remove(&entry.path) {
                write_zip_entry_bytes(&mut writer, &entry.path, &bytes, options)?;
            } else {
                write_zip_entry_bytes(&mut writer, &entry.path, &entry.bytes, options)?;
            }
            written_paths.insert(entry.path.clone());
        }
    }

    for (path, bytes) in replacements {
        if written_paths.insert(path.clone()) {
            write_zip_entry_bytes(&mut writer, &path, &bytes, options)?;
        }
    }

    writer
        .finish()
        .map_err(|e| SerializeError::CfbError(e.to_string()))
        .map(|cursor| cursor.into_inner())
}

fn planned_section_paths(
    doc: &Document,
    snapshot: Option<&HwpxPackageSnapshot>,
) -> Vec<String> {
    let existing_paths = snapshot
        .map(HwpxPackageSnapshot::section_paths)
        .unwrap_or_default();
    let section_count = doc.sections.len().max(1);

    (0..section_count)
        .map(|index| {
            existing_paths
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("Contents/section{}.xml", index))
        })
        .collect()
}

fn planned_bindata_paths(
    doc: &Document,
    snapshot: Option<&HwpxPackageSnapshot>,
) -> Vec<String> {
    let existing_paths = snapshot
        .map(HwpxPackageSnapshot::bindata_paths)
        .unwrap_or_default();
    let count = doc
        .doc_info
        .bin_data_list
        .len()
        .max(doc.bin_data_content.len());

    (0..count)
        .map(|index| {
            existing_paths
                .get(index)
                .cloned()
                .unwrap_or_else(|| default_bindata_path(doc, index))
        })
        .collect()
}

fn default_bindata_path(doc: &Document, index: usize) -> String {
    let (storage_id, extension) = if let Some(info) = doc.doc_info.bin_data_list.get(index) {
        let ext = info
            .extension
            .clone()
            .or_else(|| {
                doc.bin_data_content
                    .iter()
                    .find(|content| content.id == info.storage_id)
                    .map(|content| content.extension.clone())
            })
            .unwrap_or_else(|| "dat".to_string());
        (info.storage_id, ext)
    } else if let Some(content) = doc.bin_data_content.get(index) {
        (content.id, content.extension.clone())
    } else {
        ((index + 1) as u16, "dat".to_string())
    };

    format!("BinData/image{}.{}", storage_id, extension)
}

fn package_paths_changed(
    snapshot: Option<&HwpxPackageSnapshot>,
    section_paths: &[String],
    bindata_paths: &[String],
) -> bool {
    let Some(snapshot) = snapshot else {
        return true;
    };
    snapshot.section_paths() != section_paths || snapshot.bindata_paths() != bindata_paths
}

fn section_needs_rewrite(
    section_idx: usize,
    path: &str,
    context: HwpxPreservationContext<'_>,
) -> bool {
    if context.snapshot.is_none() {
        return true;
    }

    let is_dirty = context
        .dirty_sections
        .and_then(|sections| sections.get(section_idx))
        .copied()
        .unwrap_or(true);
    if is_dirty {
        return true;
    }

    context
        .snapshot
        .and_then(|snapshot| snapshot.entry(path))
        .is_none()
}

fn bindata_needs_rewrite(
    bindata_idx: usize,
    path: &str,
    bytes: &[u8],
    context: HwpxPreservationContext<'_>,
) -> bool {
    let Some(snapshot) = context.snapshot else {
        return true;
    };
    let Some(entry) = snapshot.entry(path) else {
        return true;
    };
    match entry.role {
        HwpxPackageEntryRole::BinData(index) if index == bindata_idx => entry.bytes != bytes,
        _ => true,
    }
}

fn should_drop_snapshot_entry(
    entry: &crate::parser::hwpx::reader::HwpxPackageEntrySnapshot,
    section_paths: &[String],
    bindata_paths: &[String],
) -> bool {
    match entry.role {
        HwpxPackageEntryRole::Section(index) => {
            section_paths.get(index).map(|path| path != &entry.path).unwrap_or(true)
        }
        HwpxPackageEntryRole::BinData(index) => {
            bindata_paths.get(index).map(|path| path != &entry.path).unwrap_or(true)
        }
        _ => false,
    }
}

fn write_zip_entry_bytes(
    writer: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    path: &str,
    content: &[u8],
    options: SimpleFileOptions,
) -> Result<(), SerializeError> {
    writer
        .start_file(path, options)
        .map_err(|e| SerializeError::CfbError(e.to_string()))?;
    writer
        .write_all(content)
        .map_err(|e| SerializeError::CfbError(e.to_string()))
}

fn serialize_content_hpf(doc: &Document, section_paths: &[String], bindata_paths: &[String]) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(
        r#"<opf:package xmlns:opf="http://www.idpf.org/2007/opf/" version="3.0" unique-identifier="rhwp-doc" id="rhwp-doc">"#,
    );
    xml.push_str(r#"<opf:manifest>"#);
    xml.push_str(r#"<opf:item id="header" href="Contents/header.xml" media-type="application/xml"/>"#);

    for (index, path) in bindata_paths.iter().enumerate() {
        let extension = path
            .rsplit('.')
            .next()
            .filter(|ext| !ext.contains('/'))
            .map(ToOwned::to_owned)
            .or_else(|| {
                doc.doc_info
                    .bin_data_list
                    .get(index)
                    .and_then(|info| info.extension.clone())
            })
            .or_else(|| doc.bin_data_content.get(index).map(|content| content.extension.clone()))
            .unwrap_or_else(|| "dat".to_string());
        let media_type = media_type_for_extension(&extension);
        xml.push_str(&format!(
            r#"<opf:item id="image{}" href="{}" media-type="{}" isEmbeded="1"/>"#,
            index + 1,
            xml_escape_attr(path),
            media_type,
        ));
    }

    for (index, path) in section_paths.iter().enumerate() {
        xml.push_str(&format!(
            r#"<opf:item id="section{}" href="{}" media-type="application/xml"/>"#,
            index,
            xml_escape_attr(path),
        ));
    }

    xml.push_str(r#"</opf:manifest><opf:spine>"#);
    xml.push_str(r#"<opf:itemref idref="header" linear="yes"/>"#);
    for index in 0..section_paths.len() {
        xml.push_str(&format!(
            r#"<opf:itemref idref="section{}" linear="yes"/>"#,
            index
        ));
    }
    xml.push_str(r#"</opf:spine></opf:package>"#);
    xml
}

fn serialize_header_xml(doc: &Document) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">"#);
    xml.push_str(&format!(
        r#"<hh:beginNum page="{}" footnote="{}" endnote="{}" pic="{}" tbl="{}" equation="{}"/>"#,
        doc.doc_properties.page_start_num,
        doc.doc_properties.footnote_start_num,
        doc.doc_properties.endnote_start_num,
        doc.doc_properties.picture_start_num,
        doc.doc_properties.table_start_num,
        doc.doc_properties.equation_start_num,
    ));

    let lang_names = [
        "HANGUL",
        "LATIN",
        "HANJA",
        "JAPANESE",
        "OTHER",
        "SYMBOL",
        "USER",
    ];
    for (lang_idx, lang_name) in lang_names.iter().enumerate() {
        let fonts = doc.doc_info.font_faces.get(lang_idx).cloned().unwrap_or_default();
        if fonts.is_empty() {
            continue;
        }
        xml.push_str(&format!(r#"<hh:fontface lang="{}">"#, lang_name));
        for font in fonts {
            xml.push_str(&serialize_font(&font));
        }
        xml.push_str(r#"</hh:fontface>"#);
    }

    for char_shape in &doc.doc_info.char_shapes {
        xml.push_str(&serialize_char_shape(char_shape));
    }
    for tab_def in &doc.doc_info.tab_defs {
        xml.push_str(&serialize_tab_def(tab_def));
    }
    for numbering in &doc.doc_info.numberings {
        xml.push_str(&serialize_numbering(numbering));
    }
    if !doc.doc_info.bullets.is_empty() {
        xml.push_str(&format!(
            r#"<hh:bullets itemCnt="{}">"#,
            doc.doc_info.bullets.len()
        ));
        for (index, bullet) in doc.doc_info.bullets.iter().enumerate() {
            xml.push_str(&serialize_bullet(bullet, index + 1));
        }
        xml.push_str(r#"</hh:bullets>"#);
    }
    for border_fill in &doc.doc_info.border_fills {
        xml.push_str(&serialize_border_fill(border_fill));
    }
    for para_shape in &doc.doc_info.para_shapes {
        xml.push_str(&serialize_para_shape(para_shape));
    }
    for style in &doc.doc_info.styles {
        xml.push_str(&format!(
            r#"<hh:style name="{}" engName="{}" type="{}" paraPrIDRef="{}" charPrIDRef="{}" nextStyleIDRef="{}"/>"#,
            xml_escape_attr(&style.local_name),
            xml_escape_attr(&style.english_name),
            if style.style_type == 1 { "CHAR" } else { "PARA" },
            style.para_shape_id,
            style.char_shape_id,
            style.next_style_id,
        ));
    }

    xml.push_str(r#"</hh:head>"#);
    xml
}

fn serialize_font(font: &Font) -> String {
    format!(r#"<hh:font face="{}"/>"#, xml_escape_attr(&font.name))
}

fn serialize_char_shape(char_shape: &CharShape) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hh:charPr height="{}" textColor="{}" shadeColor="{}" borderFillIDRef="{}">"#,
        char_shape.base_size,
        color_to_hex(char_shape.text_color),
        color_to_hex(char_shape.shade_color),
        char_shape.border_fill_id,
    ));
    xml.push_str(&format!(
        r#"<hh:fontRef hangul="{}" latin="{}" hanja="{}" japanese="{}" other="{}" symbol="{}" user="{}"/>"#,
        char_shape.font_ids[0],
        char_shape.font_ids[1],
        char_shape.font_ids[2],
        char_shape.font_ids[3],
        char_shape.font_ids[4],
        char_shape.font_ids[5],
        char_shape.font_ids[6],
    ));
    xml.push_str(&format!(
        r#"<hh:ratio hangul="{}" latin="{}" hanja="{}" japanese="{}" other="{}" symbol="{}" user="{}"/>"#,
        char_shape.ratios[0],
        char_shape.ratios[1],
        char_shape.ratios[2],
        char_shape.ratios[3],
        char_shape.ratios[4],
        char_shape.ratios[5],
        char_shape.ratios[6],
    ));
    xml.push_str(&format!(
        r#"<hh:spacing hangul="{}" latin="{}" hanja="{}" japanese="{}" other="{}" symbol="{}" user="{}"/>"#,
        char_shape.spacings[0],
        char_shape.spacings[1],
        char_shape.spacings[2],
        char_shape.spacings[3],
        char_shape.spacings[4],
        char_shape.spacings[5],
        char_shape.spacings[6],
    ));
    xml.push_str(&format!(
        r#"<hh:relSz hangul="{}" latin="{}" hanja="{}" japanese="{}" other="{}" symbol="{}" user="{}"/>"#,
        char_shape.relative_sizes[0],
        char_shape.relative_sizes[1],
        char_shape.relative_sizes[2],
        char_shape.relative_sizes[3],
        char_shape.relative_sizes[4],
        char_shape.relative_sizes[5],
        char_shape.relative_sizes[6],
    ));
    xml.push_str(&format!(
        r#"<hh:offset hangul="{}" latin="{}" hanja="{}" japanese="{}" other="{}" symbol="{}" user="{}"/>"#,
        char_shape.char_offsets[0],
        char_shape.char_offsets[1],
        char_shape.char_offsets[2],
        char_shape.char_offsets[3],
        char_shape.char_offsets[4],
        char_shape.char_offsets[5],
        char_shape.char_offsets[6],
    ));

    if char_shape.bold {
        xml.push_str(r#"<hh:bold/>"#);
    }
    if char_shape.italic {
        xml.push_str(r#"<hh:italic/>"#);
    }
    if char_shape.underline_type != UnderlineType::None {
        xml.push_str(&format!(
            r#"<hh:underline type="{}" color="{}" shape="{}"/>"#,
            underline_type_to_xml(char_shape.underline_type),
            color_to_hex(char_shape.underline_color),
            line_shape_to_xml(char_shape.underline_shape),
        ));
    }
    if char_shape.strikethrough {
        xml.push_str(&format!(
            r#"<hh:strikeout shape="{}" color="{}"/>"#,
            line_shape_to_xml(char_shape.strike_shape),
            color_to_hex(char_shape.strike_color),
        ));
    }
    if char_shape.outline_type != 0 {
        xml.push_str(&format!(
            r#"<hh:outline type="{}"/>"#,
            outline_type_to_xml(char_shape.outline_type),
        ));
    }
    if char_shape.shadow_type != 0 {
        xml.push_str(&format!(
            r#"<hh:shadow type="{}" color="{}"/>"#,
            shadow_type_to_xml(char_shape.shadow_type),
            color_to_hex(char_shape.shadow_color),
        ));
    }
    if char_shape.emboss {
        xml.push_str(r#"<hh:emboss/>"#);
    }
    if char_shape.engrave {
        xml.push_str(r#"<hh:engrave/>"#);
    }
    if char_shape.superscript {
        xml.push_str(r#"<hh:supscript/>"#);
    }
    if char_shape.subscript {
        xml.push_str(r#"<hh:subscript/>"#);
    }
    xml.push_str(r#"</hh:charPr>"#);
    xml
}

fn serialize_tab_def(tab_def: &TabDef) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hh:tabPr autoTabLeft="{}" autoTabRight="{}">"#,
        bool_to_attr(tab_def.auto_tab_left),
        bool_to_attr(tab_def.auto_tab_right),
    ));
    for tab in &tab_def.tabs {
        xml.push_str(&format!(
            r#"<hh:tabItem pos="{}" type="{}" leader="{}"/>"#,
            tab.position,
            tab_type_to_xml(tab.tab_type),
            tab_leader_to_xml(tab.fill_type),
        ));
    }
    xml.push_str(r#"</hh:tabPr>"#);
    xml
}

fn serialize_numbering(numbering: &Numbering) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(r#"<hh:numbering start="{}">"#, numbering.start_number));
    for level in 0..7 {
        xml.push_str(&format!(
            r#"<hh:paraHead level="{}" start="{}" text="{}" numFormat="{}" charPrIDRef="{}"/>"#,
            level + 1,
            numbering.level_start_numbers[level],
            xml_escape_attr(&numbering.level_formats[level]),
            numbering.heads[level].number_format,
            numbering.heads[level].char_shape_id,
        ));
    }
    xml.push_str(r#"</hh:numbering>"#);
    xml
}

fn serialize_bullet(bullet: &Bullet, id: usize) -> String {
    let bullet_char = if bullet.bullet_char == '\0' { '•' } else { bullet.bullet_char };
    let check_bullet_char = if bullet.check_bullet_char == '\0' {
        bullet_char
    } else {
        bullet.check_bullet_char
    };

    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hh:bullet id="{}" char="{}" useImage="{}" checkedChar="{}">"#,
        id,
        xml_escape_attr(&bullet_char.to_string()),
        bool_to_attr(bullet.image_bullet != 0 || bullet.bullet_char == '\u{FFFF}'),
        xml_escape_attr(&check_bullet_char.to_string()),
    ));
    xml.push_str(&format!(
        r#"<hh:paraHead level="0" align="LEFT" useInstWidth="0" autoIndent="1" widthAdjust="{}" textOffsetType="PERCENT" textOffset="{}" numFormat="DIGIT" charPrIDRef="4294967295" checkable="{}"/>"#,
        bullet.width_adjust,
        bullet.text_distance,
        bool_to_attr(check_bullet_char != bullet_char),
    ));
    xml.push_str(r#"</hh:bullet>"#);
    xml
}

fn serialize_border_fill(border_fill: &BorderFill) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<hh:borderFill>"#);
    xml.push_str(&serialize_border_line("leftBorder", &border_fill.borders[0]));
    xml.push_str(&serialize_border_line("rightBorder", &border_fill.borders[1]));
    xml.push_str(&serialize_border_line("topBorder", &border_fill.borders[2]));
    xml.push_str(&serialize_border_line("bottomBorder", &border_fill.borders[3]));

    if border_fill.diagonal.width > 0 || border_fill.diagonal.color != 0 {
        xml.push_str(&format!(
            r#"<hh:slash type="{}" width="{}" color="{}"/>"#,
            border_fill.diagonal.diagonal_type,
            border_width_to_xml(border_fill.diagonal.width),
            color_to_hex(border_fill.diagonal.color),
        ));
    }

    match border_fill.fill.fill_type {
        FillType::Solid => {
            if let Some(solid) = border_fill.fill.solid {
                xml.push_str(r#"<hh:fillBrush>"#);
                xml.push_str(&format!(
                    r#"<hh:winBrush faceColor="{}" hatchColor="{}" alpha="{}"/>"#,
                    color_to_hex(solid.background_color),
                    color_to_hex(solid.pattern_color),
                    alpha_to_decimal(border_fill.fill.alpha),
                ));
                xml.push_str(r#"</hh:fillBrush>"#);
            }
        }
        FillType::Gradient => {
            if let Some(gradient) = &border_fill.fill.gradient {
                xml.push_str(r#"<hh:fillBrush>"#);
                xml.push_str(&format!(
                    r#"<hh:gradation type="{}" angle="{}" centerX="{}" centerY="{}" blur="{}">"#,
                    gradient.gradient_type,
                    gradient.angle,
                    gradient.center_x,
                    gradient.center_y,
                    gradient.blur,
                ));
                for color in &gradient.colors {
                    xml.push_str(&format!(r#"<hh:color value="{}"/>"#, color_to_hex(*color)));
                }
                xml.push_str(r#"</hh:gradation>"#);
                xml.push_str(r#"</hh:fillBrush>"#);
            }
        }
        FillType::Image => {
            if let Some(image) = &border_fill.fill.image {
                xml.push_str(r#"<hh:fillBrush>"#);
                xml.push_str(&format!(
                    r#"<hh:imgBrush mode="{}" bright="{}" contrast="{}"/>"#,
                    image_fill_mode_to_xml(image.fill_mode),
                    image.brightness,
                    image.contrast,
                ));
                xml.push_str(&format!(
                    r#"<hh:img binaryItemIDRef="image{}"/>"#,
                    image.bin_data_id
                ));
                xml.push_str(r#"</hh:fillBrush>"#);
            }
        }
        FillType::None => {}
    }

    xml.push_str(r#"</hh:borderFill>"#);
    xml
}

fn serialize_border_line(tag_name: &str, border_line: &BorderLine) -> String {
    format!(
        r#"<hh:{} type="{}" width="{}" color="{}"/>"#,
        tag_name,
        border_line_type_to_xml(border_line.line_type),
        border_width_to_xml(border_line.width),
        color_to_hex(border_line.color),
    )
}

fn serialize_para_shape(para_shape: &ParaShape) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hh:paraPr tabPrIDRef="{}">"#,
        para_shape.tab_def_id
    ));
    xml.push_str(&format!(
        r#"<hh:align horizontal="{}"/>"#,
        alignment_to_xml(para_shape.alignment),
    ));
    if para_shape.head_type != HeadType::None || para_shape.numbering_id != 0 || para_shape.para_level != 0 {
        xml.push_str(&format!(
            r#"<hh:heading type="{}" idRef="{}" level="{}"/>"#,
            head_type_to_xml(para_shape.head_type),
            para_shape.numbering_id,
            para_shape.para_level,
        ));
    }
    xml.push_str(&format!(
        r#"<hh:margin left="{}" right="{}" indent="{}" prev="{}" next="{}"/>"#,
        para_shape.margin_left,
        para_shape.margin_right,
        para_shape.indent,
        para_shape.spacing_before,
        para_shape.spacing_after,
    ));
    xml.push_str(&format!(
        r#"<hh:lineSpacing type="{}" value="{}"/>"#,
        line_spacing_type_to_xml(para_shape.line_spacing_type),
        para_shape.line_spacing,
    ));
    if para_shape.border_fill_id != 0 || para_shape.border_spacing != [0, 0, 0, 0] {
        xml.push_str(&format!(
            r#"<hh:border borderFillIDRef="{}" offsetLeft="{}" offsetRight="{}" offsetTop="{}" offsetBottom="{}"/>"#,
            para_shape.border_fill_id,
            para_shape.border_spacing[0],
            para_shape.border_spacing[1],
            para_shape.border_spacing[2],
            para_shape.border_spacing[3],
        ));
    }

    let widow_orphan = para_shape.attr2 & (1 << 5) != 0;
    let keep_with_next = para_shape.attr2 & (1 << 6) != 0;
    let keep_lines = para_shape.attr2 & (1 << 7) != 0;
    let page_break_before = para_shape.attr2 & (1 << 8) != 0;
    if widow_orphan || keep_with_next || keep_lines || page_break_before {
        xml.push_str(&format!(
            r#"<hh:breakSetting widowOrphan="{}" keepWithNext="{}" keepLines="{}" pageBreakBefore="{}"/>"#,
            bool_to_attr(widow_orphan),
            bool_to_attr(keep_with_next),
            bool_to_attr(keep_lines),
            bool_to_attr(page_break_before),
        ));
    }

    let auto_space_kr_en = para_shape.attr1 & (1 << 20) != 0;
    let auto_space_kr_num = para_shape.attr1 & (1 << 21) != 0;
    if auto_space_kr_en || auto_space_kr_num {
        xml.push_str(&format!(
            r#"<hh:autoSpacing eAsianEng="{}" eAsianNum="{}"/>"#,
            bool_to_attr(auto_space_kr_en),
            bool_to_attr(auto_space_kr_num),
        ));
    }

    xml.push_str(r#"</hh:paraPr>"#);
    xml
}

fn serialize_section_xml(section: &Section) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(
        r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core">"#,
    );

    if section.paragraphs.is_empty() {
        let blank = Paragraph::new_empty();
        xml.push_str(&serialize_paragraph_xml(
            &blank,
            Some(&section.section_def),
            None,
        )?);
    } else {
        let embedded_col_def = section.paragraphs[0]
            .controls
            .iter()
            .position(|control| matches!(control, Control::ColumnDef(_)));
        for (index, para) in section.paragraphs.iter().enumerate() {
            xml.push_str(&serialize_paragraph_xml(
                para,
                (index == 0).then_some(&section.section_def),
                if index == 0 { embedded_col_def } else { None },
            )?);
        }
    }

    xml.push_str(r#"</hs:sec>"#);
    Ok(xml)
}

fn serialize_paragraph_xml(
    para: &Paragraph,
    section_def: Option<&SectionDef>,
    embedded_column_def_idx: Option<usize>,
) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:p paraPrIDRef="{}" styleIDRef="{}""#,
        para.para_shape_id, para.style_id
    ));
    match para.column_type {
        ColumnBreakType::Page => xml.push_str(r#" pageBreak="1""#),
        ColumnBreakType::Column => xml.push_str(r#" columnBreak="1""#),
        _ => {}
    }
    xml.push('>');

    if let Some(section_def) = section_def {
        let column_def = embedded_column_def_idx
            .and_then(|idx| para.controls.get(idx))
            .and_then(|control| match control {
                Control::ColumnDef(column_def) => Some(column_def),
                _ => None,
            });
        xml.push_str(&serialize_section_def_xml(section_def, column_def));
    }

    let control_positions = compute_control_insertions(para);
    let text_chars: Vec<char> = para.text.chars().collect();
    let skip_text_indices = compute_skipped_text_indices(para, &control_positions, &text_chars);
    let mut controls_by_pos: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (ctrl_idx, pos) in control_positions.iter().copied().enumerate() {
        if matches!(para.controls.get(ctrl_idx), Some(Control::Field(_))) {
            continue;
        }
        controls_by_pos.entry(pos).or_default().push(ctrl_idx);
    }

    let mut field_starts: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut field_ends: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut empty_field_ends: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut field_boundaries = BTreeSet::new();
    for (range_idx, range) in para.field_ranges.iter().enumerate() {
        field_starts.entry(range.start_char_idx).or_default().push(range_idx);
        if range.start_char_idx == range.end_char_idx {
            empty_field_ends.entry(range.end_char_idx).or_default().push(range_idx);
        } else {
            field_ends.entry(range.end_char_idx).or_default().push(range_idx);
        }
        field_boundaries.insert(range.start_char_idx);
        field_boundaries.insert(range.end_char_idx);
    }
    for ranges in field_starts.values_mut() {
        ranges.sort_by_key(|range_idx| para.field_ranges[*range_idx].control_idx);
    }
    for ranges in field_ends.values_mut() {
        ranges.sort_by_key(|range_idx| std::cmp::Reverse(para.field_ranges[*range_idx].control_idx));
    }
    for ranges in empty_field_ends.values_mut() {
        ranges.sort_by_key(|range_idx| para.field_ranges[*range_idx].control_idx);
    }

    let mut cursor = 0usize;
    while cursor <= text_chars.len() {
        if let Some(range_indices) = field_ends.get(&cursor) {
            for range_idx in range_indices {
                xml.push_str(&serialize_field_end_for_range(para, *range_idx)?);
            }
        }
        if let Some(range_indices) = field_starts.get(&cursor) {
            for range_idx in range_indices {
                xml.push_str(&serialize_field_begin_for_range(para, *range_idx)?);
            }
        }
        if let Some(range_indices) = empty_field_ends.get(&cursor) {
            for range_idx in range_indices {
                xml.push_str(&serialize_field_end_for_range(para, *range_idx)?);
            }
        }

        if let Some(ctrl_indices) = controls_by_pos.get(&cursor) {
            for ctrl_idx in ctrl_indices {
                if Some(*ctrl_idx) == embedded_column_def_idx {
                    continue;
                }
                xml.push_str(&serialize_control_xml(&para.controls[*ctrl_idx])?);
            }
        }

        while cursor < text_chars.len() && skip_text_indices.contains(&cursor) {
            cursor += 1;
        }
        if cursor >= text_chars.len() {
            break;
        }

        let current_shape = char_shape_id_at(para, cursor);
        let mut end = cursor + 1;
        while end < text_chars.len() {
            if skip_text_indices.contains(&end)
                || controls_by_pos.contains_key(&end)
                || field_boundaries.contains(&end)
            {
                break;
            }
            if char_shape_id_at(para, end) != current_shape {
                break;
            }
            end += 1;
        }

        xml.push_str(&serialize_run_xml(&text_chars[cursor..end], current_shape));
        cursor = end;
    }

    xml.push_str(&serialize_linesegs(para));
    xml.push_str(r#"</hp:p>"#);
    Ok(xml)
}

fn serialize_section_def_xml(
    section_def: &SectionDef,
    column_def: Option<&ColumnDef>,
) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:secPr textDirection="{}" tabStop="{}">"#,
        if section_def.text_direction == 1 {
            "VERTICAL"
        } else {
            "HORIZONTAL"
        },
        section_def.default_tab_spacing,
    ));
    xml.push_str(&serialize_page_def_xml(&section_def.page_def));
    if let Some(column_def) = column_def {
        xml.push_str(&format!(
            r#"<hp:colPr type="{}" layout="{}" colCount="{}" sameSz="{}" sameGap="{}"/>"#,
            column_type_to_xml(column_def.column_type),
            column_direction_to_xml(column_def.direction),
            column_def.column_count.max(1),
            bool_to_attr(column_def.same_width),
            column_def.spacing,
        ));
    }

    if section_def.page_num != 0
        || section_def.picture_num != 0
        || section_def.table_num != 0
        || section_def.equation_num != 0
    {
        xml.push_str(&format!(
            r#"<hp:startNum page="{}" pic="{}" tbl="{}" equation="{}"/>"#,
            section_def.page_num,
            section_def.picture_num,
            section_def.table_num,
            section_def.equation_num,
        ));
    }

    xml.push_str(&format!(
        r#"<hp:visibility hideFirstHeader="{}" hideFirstFooter="{}" hideFirstMasterPage="{}" border="{}" fill="{}" hideFirstEmptyLine="{}"/>"#,
        bool_to_attr(section_def.hide_header),
        bool_to_attr(section_def.hide_footer),
        bool_to_attr(section_def.hide_master_page),
        if section_def.hide_border { "HIDE_ALL" } else { "SHOW_ALL" },
        if section_def.hide_fill { "HIDE_ALL" } else { "SHOW_ALL" },
        bool_to_attr(section_def.hide_empty_line),
    ));
    xml.push_str(r#"</hp:secPr>"#);
    xml
}

fn serialize_page_def_xml(page_def: &PageDef) -> String {
    format!(
        concat!(
            r#"<hp:pagePr width="{}" height="{}" landscape="{}"/>"#,
            r#"<hp:margin left="{}" right="{}" top="{}" bottom="{}" header="{}" footer="{}" gutter="{}"/>"#
        ),
        page_def.width,
        page_def.height,
        bool_to_attr(page_def.landscape),
        page_def.margin_left,
        page_def.margin_right,
        page_def.margin_top,
        page_def.margin_bottom,
        page_def.margin_header,
        page_def.margin_footer,
        page_def.margin_gutter,
    )
}

fn compute_control_insertions(para: &Paragraph) -> Vec<usize> {
    find_control_text_positions(para)
}

fn compute_skipped_text_indices(
    para: &Paragraph,
    control_positions: &[usize],
    text_chars: &[char],
) -> BTreeSet<usize> {
    let mut skipped = BTreeSet::new();
    let mut controls_by_pos: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (ctrl_idx, pos) in control_positions.iter().copied().enumerate() {
        controls_by_pos.entry(pos).or_default().push(ctrl_idx);
    }

    for (pos, ctrl_indices) in controls_by_pos {
        let mut scan = pos;
        for ctrl_idx in ctrl_indices {
            if matches!(para.controls[ctrl_idx], Control::AutoNumber(_))
                && text_chars.get(scan) == Some(&' ')
            {
                skipped.insert(scan);
                scan += 1;
            }
        }
    }

    skipped
}

fn serialize_run_xml(text: &[char], char_shape_id: u32) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(r#"<hp:run charPrIDRef="{}"><hp:t>"#, char_shape_id));
    let mut plain = String::new();
    for ch in text {
        match ch {
            '\n' => {
                if !plain.is_empty() {
                    xml.push_str(&xml_escape_text(&plain));
                    plain.clear();
                }
                xml.push_str(r#"<hp:lineBreak/>"#);
            }
            '\t' => {
                if !plain.is_empty() {
                    xml.push_str(&xml_escape_text(&plain));
                    plain.clear();
                }
                xml.push_str(r#"<hp:tab/>"#);
            }
            '\u{00A0}' => {
                if !plain.is_empty() {
                    xml.push_str(&xml_escape_text(&plain));
                    plain.clear();
                }
                xml.push_str(r#"<hp:nbSpace/>"#);
            }
            '\u{2007}' => {
                if !plain.is_empty() {
                    xml.push_str(&xml_escape_text(&plain));
                    plain.clear();
                }
                xml.push_str(r#"<hp:fwSpace/>"#);
            }
            other => plain.push(*other),
        }
    }
    if !plain.is_empty() {
        xml.push_str(&xml_escape_text(&plain));
    }
    xml.push_str(r#"</hp:t></hp:run>"#);
    xml
}

fn serialize_linesegs(para: &Paragraph) -> String {
    if para.line_segs.is_empty() {
        return String::new();
    }

    let mut xml = String::new();
    xml.push_str(r#"<hp:linesegarray>"#);
    for seg in &para.line_segs {
        xml.push_str(&format!(
            r#"<hp:lineseg textpos="{}" vertpos="{}" vertsize="{}" textheight="{}" baseline="{}" spacing="{}" horzpos="{}" horzsize="{}" flags="{}"/>"#,
            seg.text_start,
            seg.vertical_pos,
            seg.line_height,
            seg.text_height,
            seg.baseline_distance,
            seg.line_spacing,
            seg.column_start,
            seg.segment_width,
            seg.tag,
        ));
    }
    xml.push_str(r#"</hp:linesegarray>"#);
    xml
}

fn serialize_control_xml(control: &Control) -> Result<String, SerializeError> {
    match control {
        Control::ColumnDef(column_def) => Ok(format!(
            r#"<hp:ctrl><hp:colPr type="{}" layout="{}" colCount="{}" sameSz="{}" sameGap="{}"/></hp:ctrl>"#,
            column_type_to_xml(column_def.column_type),
            column_direction_to_xml(column_def.direction),
            column_def.column_count.max(1),
            bool_to_attr(column_def.same_width),
            column_def.spacing,
        )),
        Control::Table(table) => serialize_table_xml(table),
        Control::Picture(picture) => serialize_picture_xml(picture),
        Control::Shape(shape) => serialize_shape_xml(shape, false),
        Control::Header(header) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_header_footer_control(
                "header",
                header.apply_to,
                &header.paragraphs,
            )?
        )),
        Control::Footer(footer) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_header_footer_control(
                "footer",
                footer.apply_to,
                &footer.paragraphs,
            )?
        )),
        Control::Footnote(note) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_note_control("footNote", note.number, &note.paragraphs)?
        )),
        Control::Endnote(note) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_note_control("endNote", note.number, &note.paragraphs)?
        )),
        Control::AutoNumber(auto_number) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_auto_number_control(auto_number)
        )),
        Control::NewNumber(new_number) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_new_number_control(new_number)
        )),
        Control::PageNumberPos(page_number_pos) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_page_number_control(page_number_pos)
        )),
        Control::Bookmark(bookmark) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_bookmark_control(bookmark)
        )),
        Control::PageHide(page_hide) => Ok(format!(
            r#"<hp:ctrl>{}</hp:ctrl>"#,
            serialize_page_hide_control(page_hide)
        )),
        other => Err(SerializeError::CfbError(format!(
            "Unsupported control reached HWPX serializer: {:?}",
            other
        ))),
    }
}

fn serialize_field_begin_for_range(
    para: &Paragraph,
    range_idx: usize,
) -> Result<String, SerializeError> {
    let range = para
        .field_ranges
        .get(range_idx)
        .ok_or_else(|| SerializeError::CfbError("Missing field range during HWPX save".to_string()))?;
    let field = match para.controls.get(range.control_idx) {
        Some(Control::Field(field)) => field,
        _ => {
            return Err(SerializeError::CfbError(format!(
                "Field range {} points to a non-field control",
                range_idx
            )))
        }
    };

    Ok(format!(
        r#"<hp:ctrl>{}</hp:ctrl>"#,
        serialize_field_begin_control(field, range.control_idx)
    ))
}

fn serialize_field_end_for_range(
    para: &Paragraph,
    range_idx: usize,
) -> Result<String, SerializeError> {
    let range = para
        .field_ranges
        .get(range_idx)
        .ok_or_else(|| SerializeError::CfbError("Missing field range during HWPX save".to_string()))?;
    let field = match para.controls.get(range.control_idx) {
        Some(Control::Field(field)) => field,
        _ => {
            return Err(SerializeError::CfbError(format!(
                "Field range {} points to a non-field control",
                range_idx
            )))
        }
    };

    Ok(format!(
        r#"<hp:ctrl>{}</hp:ctrl>"#,
        serialize_field_end_control(field, range.control_idx)
    ))
}

fn serialize_field_begin_control(field: &Field, control_idx: usize) -> String {
    let (ctrl_id, field_id) = resolved_field_ids(field, control_idx);
    let name = field
        .ctrl_data_name
        .as_deref()
        .or_else(|| {
            (field.field_type == FieldType::ClickHere)
                .then(|| field.field_name())
                .flatten()
        })
        .unwrap_or_default();

    let command = if field.command.is_empty()
        && field.field_type == FieldType::ClickHere
        && !name.is_empty()
    {
        Field::build_clickhere_command("", "", name)
    } else {
        field.command.clone()
    };

    let mut parameters = Vec::new();
    parameters.push(format!(
        r#"<hp:integerParam name="Prop">{}</hp:integerParam>"#,
        field.properties
    ));
    if !command.is_empty() {
        parameters.push(format!(
            r#"<hp:stringParam name="Command">{}</hp:stringParam>"#,
            xml_escape_text(&command)
        ));
    }
    if field.field_type == FieldType::ClickHere {
        if let Some(guide) = field.guide_text() {
            parameters.push(format!(
                r#"<hp:stringParam name="Direction">{}</hp:stringParam>"#,
                xml_escape_text(guide)
            ));
        }
        if let Some(memo) = field.memo_text() {
            parameters.push(format!(
                r#"<hp:stringParam name="HelpState">{}</hp:stringParam>"#,
                xml_escape_text(memo)
            ));
        }
        if !name.is_empty() {
            parameters.push(format!(
                r#"<hp:stringParam name="Name">{}</hp:stringParam>"#,
                xml_escape_text(name)
            ));
        }
    }

    let parameters_xml = if parameters.is_empty() {
        String::new()
    } else {
        format!(
            r#"<hp:parameters cnt="{}" name="">{}</hp:parameters>"#,
            parameters.len(),
            parameters.join("")
        )
    };

    format!(
        r#"<hp:fieldBegin id="{}" type="{}" name="{}" editable="{}" dirty="0" zorder="-1" fieldid="{}" metaTag="">{}</hp:fieldBegin>"#,
        ctrl_id,
        field_type_to_xml(field.field_type),
        xml_escape_attr(name),
        bool_to_attr(field.is_editable_in_form()),
        field_id,
        parameters_xml,
    )
}

fn serialize_field_end_control(field: &Field, control_idx: usize) -> String {
    let (ctrl_id, field_id) = resolved_field_ids(field, control_idx);
    format!(
        r#"<hp:fieldEnd beginIDRef="{}" fieldid="{}"/>"#,
        ctrl_id,
        field_id,
    )
}

fn resolved_field_ids(field: &Field, control_idx: usize) -> (u32, u32) {
    let fallback = (control_idx as u32).saturating_add(1);
    let ctrl_id = if field.ctrl_id != 0 {
        field.ctrl_id
    } else if field.field_id != 0 {
        field.field_id
    } else {
        fallback
    };
    let field_id = if field.field_id != 0 {
        field.field_id
    } else {
        ctrl_id
    };
    (ctrl_id, field_id)
}

fn serialize_table_xml(table: &Table) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:tbl rowCnt="{}" colCnt="{}" cellSpacing="{}" borderFillIDRef="{}" pageBreak="{}" repeatHeader="{}" textWrap="{}">"#,
        table.row_count,
        table.col_count,
        table.cell_spacing,
        table.border_fill_id,
        table_page_break_to_xml(table.page_break),
        bool_to_attr(table.repeat_header),
        text_wrap_to_xml(table.common.text_wrap),
    ));
    xml.push_str(&format!(
        r#"<hp:sz width="{}" height="{}"/>"#,
        table.common.width,
        table.common.height,
    ));
    xml.push_str(&serialize_pos_xml(&table.common));
    xml.push_str(&format!(
        r#"<hp:outMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        table.outer_margin_left,
        table.outer_margin_right,
        table.outer_margin_top,
        table.outer_margin_bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:inMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        table.padding.left,
        table.padding.right,
        table.padding.top,
        table.padding.bottom,
    ));
    for zone in &table.zones {
        xml.push_str(&format!(
            r#"<hp:cellzone startColAddr="{}" startRowAddr="{}" endColAddr="{}" endRowAddr="{}" borderFillIDRef="{}"/>"#,
            zone.start_col,
            zone.start_row,
            zone.end_col,
            zone.end_row,
            zone.border_fill_id,
        ));
    }
    if let Some(caption) = &table.caption {
        xml.push_str(&serialize_caption_xml(caption)?);
    }

    for row in 0..table.row_count {
        xml.push_str(r#"<hp:tr>"#);
        let mut row_cells: Vec<&Cell> = table.cells.iter().filter(|cell| cell.row == row).collect();
        row_cells.sort_by_key(|cell| cell.col);
        for cell in row_cells {
            xml.push_str(&serialize_table_cell_xml(cell)?);
        }
        xml.push_str(r#"</hp:tr>"#);
    }
    xml.push_str(r#"</hp:tbl>"#);
    Ok(xml)
}

fn serialize_table_cell_xml(cell: &Cell) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:tc borderFillIDRef="{}" header="{}">"#,
        cell.border_fill_id,
        bool_to_attr(cell.is_header),
    ));
    xml.push_str(&format!(
        r#"<hp:cellAddr colAddr="{}" rowAddr="{}"/>"#,
        cell.col, cell.row
    ));
    xml.push_str(&format!(
        r#"<hp:cellSpan colSpan="{}" rowSpan="{}"/>"#,
        cell.col_span.max(1),
        cell.row_span.max(1),
    ));
    xml.push_str(&format!(
        r#"<hp:cellSz width="{}" height="{}"/>"#,
        cell.width, cell.height
    ));
    xml.push_str(&format!(
        r#"<hp:cellMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        cell.padding.left,
        cell.padding.right,
        cell.padding.top,
        cell.padding.bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:subList vertAlign="{}">"#,
        vertical_align_to_xml(cell.vertical_align),
    ));
    if cell.paragraphs.is_empty() {
        xml.push_str(&serialize_paragraph_xml(&Paragraph::new_empty(), None, None)?);
    } else {
        for para in &cell.paragraphs {
            xml.push_str(&serialize_paragraph_xml(para, None, None)?);
        }
    }
    xml.push_str(r#"</hp:subList></hp:tc>"#);
    Ok(xml)
}

fn serialize_picture_xml(picture: &Picture) -> Result<String, SerializeError> {
    serialize_picture_xml_with_context(picture, false)
}

fn serialize_picture_xml_with_context(
    picture: &Picture,
    nested_in_group: bool,
) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:pic zOrder="{}" textWrap="{}" instid="{}" groupLevel="{}">"#,
        picture.common.z_order,
        text_wrap_to_xml(picture.common.text_wrap),
        resolved_instance_id(&picture.common),
        picture.shape_attr.group_level,
    ));
    xml.push_str(&serialize_size_xml(&picture.common));
    if !nested_in_group {
        xml.push_str(&serialize_pos_xml(&picture.common));
    }
    xml.push_str(&serialize_out_margin_xml(&picture.common));
    if let Some(caption) = &picture.caption {
        xml.push_str(&serialize_caption_xml(caption)?);
    }
    xml.push_str(&serialize_shape_component_xml(&picture.shape_attr, true));
    xml.push_str(&serialize_line_shape_xml(
        picture.border_attr.color,
        picture.border_attr.width,
        picture.border_attr.attr,
        picture.border_attr.outline_style,
    ));
    xml.push_str(&format!(
        r#"<hp:imgClip left="{}" right="{}" top="{}" bottom="{}"/>"#,
        picture.crop.left,
        picture.crop.right,
        picture.crop.top,
        picture.crop.bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:inMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        picture.padding.left,
        picture.padding.right,
        picture.padding.top,
        picture.padding.bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:img binaryItemIDRef="image{}" bright="{}" contrast="{}" effect="{}"/>"#,
        picture.image_attr.bin_data_id,
        picture.image_attr.brightness,
        picture.image_attr.contrast,
        image_effect_to_xml(picture.image_attr.effect),
    ));
    xml.push_str(r#"</hp:pic>"#);
    Ok(xml)
}

fn serialize_shape_xml(shape: &ShapeObject, nested_in_group: bool) -> Result<String, SerializeError> {
    match shape {
        ShapeObject::Picture(picture) => serialize_picture_xml_with_context(picture, nested_in_group),
        ShapeObject::Group(group) => {
            let mut xml = String::new();
            xml.push_str(&format!(
                r#"<hp:container zOrder="{}" textWrap="{}" instid="{}" groupLevel="{}">"#,
                group.common.z_order,
                text_wrap_to_xml(group.common.text_wrap),
                resolved_instance_id(&group.common),
                group.shape_attr.group_level,
            ));
            xml.push_str(&serialize_size_xml(&group.common));
            if !nested_in_group {
                xml.push_str(&serialize_pos_xml(&group.common));
            }
            xml.push_str(&serialize_out_margin_xml(&group.common));
            if let Some(caption) = &group.caption {
                xml.push_str(&serialize_caption_xml(caption)?);
            }
            xml.push_str(&serialize_shape_component_xml(&group.shape_attr, false));
            for child in &group.children {
                xml.push_str(&serialize_shape_xml(child, true)?);
            }
            xml.push_str(r#"</hp:container>"#);
            Ok(xml)
        }
        ShapeObject::Curve(_) => Err(SerializeError::CfbError(
            "Curve shapes are not written to HWPX yet.".to_string(),
        )),
        ShapeObject::Line(line) => {
            let mut xml = start_basic_shape_xml(
                "line",
                &line.common,
                line.drawing.caption.as_ref(),
                &line.drawing.shape_attr,
                nested_in_group,
            )?;
            xml.push_str(&serialize_line_shape_xml(
                line.drawing.border_line.color,
                line.drawing.border_line.width,
                line.drawing.border_line.attr,
                line.drawing.border_line.outline_style,
            ));
            if let Some(fill_xml) = serialize_fill_brush_xml(&line.drawing.fill) {
                xml.push_str(&fill_xml);
            }
            if let Some(text_box) = &line.drawing.text_box {
                xml.push_str(&serialize_draw_text_xml(text_box)?);
            }
            xml.push_str(&serialize_shadow_xml(
                line.drawing.shadow_type as u8,
                line.drawing.shadow_color,
                line.drawing.shadow_offset_x,
                line.drawing.shadow_offset_y,
                line.drawing.shadow_alpha,
            ));
            xml.push_str(&serialize_point_xml("startPt", line.start.x, line.start.y));
            xml.push_str(&serialize_point_xml("endPt", line.end.x, line.end.y));
            xml.push_str(r#"</hp:line>"#);
            Ok(xml)
        }
        ShapeObject::Rectangle(rect) => {
            let mut xml = start_basic_shape_xml_with_extra_attrs(
                "rect",
                &rect.common,
                rect.drawing.caption.as_ref(),
                &rect.drawing.shape_attr,
                nested_in_group,
                &format!(r#" ratio="{}""#, rect.round_rate),
            )?;
            xml.push_str(&serialize_line_shape_xml(
                rect.drawing.border_line.color,
                rect.drawing.border_line.width,
                rect.drawing.border_line.attr,
                rect.drawing.border_line.outline_style,
            ));
            if let Some(fill_xml) = serialize_fill_brush_xml(&rect.drawing.fill) {
                xml.push_str(&fill_xml);
            }
            if let Some(text_box) = &rect.drawing.text_box {
                xml.push_str(&serialize_draw_text_xml(text_box)?);
            }
            xml.push_str(&serialize_shadow_xml(
                rect.drawing.shadow_type as u8,
                rect.drawing.shadow_color,
                rect.drawing.shadow_offset_x,
                rect.drawing.shadow_offset_y,
                rect.drawing.shadow_alpha,
            ));
            for index in 0..4 {
                xml.push_str(&serialize_point_xml(
                    &format!("pt{}", index),
                    rect.x_coords[index],
                    rect.y_coords[index],
                ));
            }
            xml.push_str(r#"</hp:rect>"#);
            Ok(xml)
        }
        ShapeObject::Ellipse(ellipse) => {
            let mut extra_attrs = String::new();
            if ellipse.attr & 0x1 != 0 {
                extra_attrs.push_str(r#" intervalDirty="1""#);
            }
            if ellipse.attr & 0x2 != 0 {
                extra_attrs.push_str(r#" hasArcPr="1""#);
            }
            let mut xml = start_basic_shape_xml_with_extra_attrs(
                "ellipse",
                &ellipse.common,
                ellipse.drawing.caption.as_ref(),
                &ellipse.drawing.shape_attr,
                nested_in_group,
                &extra_attrs,
            )?;
            xml.push_str(&serialize_line_shape_xml(
                ellipse.drawing.border_line.color,
                ellipse.drawing.border_line.width,
                ellipse.drawing.border_line.attr,
                ellipse.drawing.border_line.outline_style,
            ));
            if let Some(fill_xml) = serialize_fill_brush_xml(&ellipse.drawing.fill) {
                xml.push_str(&fill_xml);
            }
            if let Some(text_box) = &ellipse.drawing.text_box {
                xml.push_str(&serialize_draw_text_xml(text_box)?);
            }
            xml.push_str(&serialize_shadow_xml(
                ellipse.drawing.shadow_type as u8,
                ellipse.drawing.shadow_color,
                ellipse.drawing.shadow_offset_x,
                ellipse.drawing.shadow_offset_y,
                ellipse.drawing.shadow_alpha,
            ));
            xml.push_str(&serialize_point_xml("center", ellipse.center.x, ellipse.center.y));
            xml.push_str(&serialize_point_xml("ax1", ellipse.axis1.x, ellipse.axis1.y));
            xml.push_str(&serialize_point_xml("ax2", ellipse.axis2.x, ellipse.axis2.y));
            xml.push_str(&serialize_point_xml("start1", ellipse.start1.x, ellipse.start1.y));
            xml.push_str(&serialize_point_xml("end1", ellipse.end1.x, ellipse.end1.y));
            xml.push_str(&serialize_point_xml("start2", ellipse.start2.x, ellipse.start2.y));
            xml.push_str(&serialize_point_xml("end2", ellipse.end2.x, ellipse.end2.y));
            xml.push_str(r#"</hp:ellipse>"#);
            Ok(xml)
        }
        ShapeObject::Arc(arc) => {
            let mut xml = start_basic_shape_xml_with_extra_attrs(
                "arc",
                &arc.common,
                arc.drawing.caption.as_ref(),
                &arc.drawing.shape_attr,
                nested_in_group,
                &format!(r#" type="{}""#, arc_type_to_xml(arc.arc_type)),
            )?;
            xml.push_str(&serialize_line_shape_xml(
                arc.drawing.border_line.color,
                arc.drawing.border_line.width,
                arc.drawing.border_line.attr,
                arc.drawing.border_line.outline_style,
            ));
            if let Some(fill_xml) = serialize_fill_brush_xml(&arc.drawing.fill) {
                xml.push_str(&fill_xml);
            }
            if let Some(text_box) = &arc.drawing.text_box {
                xml.push_str(&serialize_draw_text_xml(text_box)?);
            }
            xml.push_str(&serialize_shadow_xml(
                arc.drawing.shadow_type as u8,
                arc.drawing.shadow_color,
                arc.drawing.shadow_offset_x,
                arc.drawing.shadow_offset_y,
                arc.drawing.shadow_alpha,
            ));
            xml.push_str(&serialize_point_xml("center", arc.center.x, arc.center.y));
            xml.push_str(&serialize_point_xml("ax1", arc.axis1.x, arc.axis1.y));
            xml.push_str(&serialize_point_xml("ax2", arc.axis2.x, arc.axis2.y));
            xml.push_str(r#"</hp:arc>"#);
            Ok(xml)
        }
        ShapeObject::Polygon(polygon) => {
            let mut xml = start_basic_shape_xml(
                "polygon",
                &polygon.common,
                polygon.drawing.caption.as_ref(),
                &polygon.drawing.shape_attr,
                nested_in_group,
            )?;
            xml.push_str(&serialize_line_shape_xml(
                polygon.drawing.border_line.color,
                polygon.drawing.border_line.width,
                polygon.drawing.border_line.attr,
                polygon.drawing.border_line.outline_style,
            ));
            if let Some(fill_xml) = serialize_fill_brush_xml(&polygon.drawing.fill) {
                xml.push_str(&fill_xml);
            }
            if let Some(text_box) = &polygon.drawing.text_box {
                xml.push_str(&serialize_draw_text_xml(text_box)?);
            }
            xml.push_str(&serialize_shadow_xml(
                polygon.drawing.shadow_type as u8,
                polygon.drawing.shadow_color,
                polygon.drawing.shadow_offset_x,
                polygon.drawing.shadow_offset_y,
                polygon.drawing.shadow_alpha,
            ));
            for point in &polygon.points {
                xml.push_str(&serialize_point_xml("pt", point.x, point.y));
            }
            xml.push_str(r#"</hp:polygon>"#);
            Ok(xml)
        }
    }
}

fn start_basic_shape_xml(
    tag_name: &str,
    common: &CommonObjAttr,
    caption: Option<&Caption>,
    shape_attr: &crate::model::shape::ShapeComponentAttr,
    nested_in_group: bool,
) -> Result<String, SerializeError> {
    start_basic_shape_xml_with_extra_attrs(tag_name, common, caption, shape_attr, nested_in_group, "")
}

fn start_basic_shape_xml_with_extra_attrs(
    tag_name: &str,
    common: &CommonObjAttr,
    caption: Option<&Caption>,
    shape_attr: &crate::model::shape::ShapeComponentAttr,
    nested_in_group: bool,
    extra_attrs: &str,
) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:{} zOrder="{}" textWrap="{}" instid="{}" groupLevel="{}"{}>"#,
        tag_name,
        common.z_order,
        text_wrap_to_xml(common.text_wrap),
        resolved_instance_id(common),
        shape_attr.group_level,
        extra_attrs,
    ));
    xml.push_str(&serialize_size_xml(common));
    if !nested_in_group {
        xml.push_str(&serialize_pos_xml(common));
    }
    xml.push_str(&serialize_out_margin_xml(common));
    if let Some(caption) = caption {
        xml.push_str(&serialize_caption_xml(caption)?);
    }
    xml.push_str(&serialize_shape_component_xml(shape_attr, false));
    Ok(xml)
}

fn resolved_instance_id(common: &CommonObjAttr) -> u32 {
    if common.instance_id != 0 {
        return common.instance_id;
    }

    let seed = common.ctrl_id
        ^ common.width
        ^ common.height
        ^ (common.z_order as u32)
        ^ common.vertical_offset
        ^ common.horizontal_offset;
    let id = (seed | 0x4000_0000).max(1);
    if id == 0 { 0x4000_0001 } else { id }
}

fn serialize_size_xml(common: &CommonObjAttr) -> String {
    format!(
        r#"<hp:sz width="{}" widthRelTo="{}" height="{}" heightRelTo="{}" protect="0"/>"#,
        common.width,
        size_criterion_to_xml(common.width_criterion),
        common.height,
        size_criterion_to_xml(common.height_criterion),
    )
}

fn serialize_pos_xml(common: &CommonObjAttr) -> String {
    format!(
        r#"<hp:pos treatAsChar="{}" affectLSpacing="0" flowWithText="0" allowOverlap="1" holdAnchorAndSO="0" vertRelTo="{}" horzRelTo="{}" vertAlign="{}" horzAlign="{}" vertOffset="{}" horzOffset="{}"/>"#,
        bool_to_attr(common.treat_as_char),
        vert_rel_to_xml(common.vert_rel_to),
        horz_rel_to_xml(common.horz_rel_to),
        vert_align_to_xml(common.vert_align),
        horz_align_to_xml(common.horz_align),
        common.vertical_offset,
        common.horizontal_offset,
    )
}

fn serialize_out_margin_xml(common: &CommonObjAttr) -> String {
    format!(
        r#"<hp:outMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        common.margin.left,
        common.margin.right,
        common.margin.top,
        common.margin.bottom,
    )
}

fn serialize_shape_component_xml(
    shape_attr: &crate::model::shape::ShapeComponentAttr,
    rotate_image: bool,
) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:offset x="{}" y="{}"/>"#,
        shape_attr.offset_x.max(0),
        shape_attr.offset_y.max(0),
    ));
    xml.push_str(&format!(
        r#"<hp:orgSz width="{}" height="{}"/>"#,
        shape_attr.original_width,
        shape_attr.original_height,
    ));
    xml.push_str(&format!(
        r#"<hp:curSz width="{}" height="{}"/>"#,
        shape_attr.current_width.max(shape_attr.original_width),
        shape_attr.current_height.max(shape_attr.original_height),
    ));
    xml.push_str(&format!(
        r#"<hp:flip horizontal="{}" vertical="{}"/>"#,
        bool_to_attr(shape_attr.horz_flip),
        bool_to_attr(shape_attr.vert_flip),
    ));
    xml.push_str(&format!(
        r#"<hp:rotationInfo angle="{}" centerX="{}" centerY="{}" rotateimage="{}"/>"#,
        shape_attr.rotation_angle,
        shape_attr.rotation_center.x.max(0),
        shape_attr.rotation_center.y.max(0),
        bool_to_attr(rotate_image),
    ));
    xml.push_str(&serialize_rendering_info_xml(shape_attr));
    xml
}

fn serialize_rendering_info_xml(shape_attr: &crate::model::shape::ShapeComponentAttr) -> String {
    let a = if shape_attr.render_sx == 0.0 { 1.0 } else { shape_attr.render_sx };
    let d = if shape_attr.render_sy == 0.0 { 1.0 } else { shape_attr.render_sy };
    format!(
        r#"<hp:renderingInfo><hc:transMatrix e1="{}" e2="{}" e3="{}" e4="{}" e5="{}" e6="{}"/></hp:renderingInfo>"#,
        format_matrix_value(a),
        format_matrix_value(shape_attr.render_b),
        format_matrix_value(shape_attr.render_tx),
        format_matrix_value(shape_attr.render_c),
        format_matrix_value(d),
        format_matrix_value(shape_attr.render_ty),
    )
}

fn format_matrix_value(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        let mut text = format!("{value:.6}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

fn serialize_point_xml(tag_name: &str, x: i32, y: i32) -> String {
    format!(r#"<hc:{} x="{}" y="{}"/>"#, tag_name, x, y)
}

fn serialize_line_shape_xml(color: u32, width: i32, attr: u32, outline_style: u8) -> String {
    format!(
        r#"<hp:lineShape color="{}" width="{}" style="{}" endCap="FLAT" headStyle="NORMAL" tailStyle="NORMAL" headfill="1" tailfill="1" headSz="MEDIUM_MEDIUM" tailSz="MEDIUM_MEDIUM" outlineStyle="{}" alpha="0"/>"#,
        color_to_hex(color),
        width.max(0),
        line_shape_to_xml((attr & 0xFF) as u8),
        outline_style_to_xml(outline_style),
    )
}

fn serialize_fill_brush_xml(fill: &crate::model::style::Fill) -> Option<String> {
    match fill.fill_type {
        FillType::None => None,
        FillType::Solid => {
            let solid = fill.solid.as_ref()?;
            Some(format!(
                r#"<hc:fillBrush><hc:winBrush faceColor="{}" hatchColor="{}" alpha="{}"/></hc:fillBrush>"#,
                color_to_hex(solid.background_color),
                color_to_hex(solid.pattern_color),
                format_matrix_value(f64::from(fill.alpha) / 255.0),
            ))
        }
        FillType::Gradient => {
            let gradient = fill.gradient.as_ref()?;
            Some(format!(
                r#"<hc:fillBrush><hc:gradation type="{}" angle="{}" centerX="{}" centerY="{}"/></hc:fillBrush>"#,
                gradient.gradient_type,
                gradient.angle,
                gradient.center_x,
                gradient.center_y,
            ))
        }
        FillType::Image => {
            let image = fill.image.as_ref()?;
            Some(format!(
                r#"<hc:fillBrush><hc:imgBrush mode="{}"/></hc:fillBrush>"#,
                image_fill_mode_to_xml(image.fill_mode),
            ))
        }
    }
}

fn serialize_draw_text_xml(text_box: &crate::model::shape::TextBox) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:drawText lastWidth="{}" name="" editable="0">"#,
        text_box.max_width
    ));
    xml.push_str(&format!(
        r#"<hp:subList vertAlign="{}">"#,
        vertical_align_to_xml(text_box.vertical_align),
    ));
    if text_box.paragraphs.is_empty() {
        xml.push_str(&serialize_paragraph_xml(&Paragraph::new_empty(), None, None)?);
    } else {
        for para in &text_box.paragraphs {
            xml.push_str(&serialize_paragraph_xml(para, None, None)?);
        }
    }
    xml.push_str(r#"</hp:subList>"#);
    xml.push_str(&format!(
        r#"<hp:textMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        text_box.margin_left,
        text_box.margin_right,
        text_box.margin_top,
        text_box.margin_bottom,
    ));
    xml.push_str(r#"</hp:drawText>"#);
    Ok(xml)
}

fn serialize_shadow_xml(
    shadow_type: u8,
    shadow_color: u32,
    shadow_offset_x: i32,
    shadow_offset_y: i32,
    shadow_alpha: u8,
) -> String {
    format!(
        r#"<hp:shadow type="{}" color="{}" offsetX="{}" offsetY="{}" alpha="{}"/>"#,
        shadow_type_to_xml(shadow_type),
        color_to_hex(shadow_color),
        shadow_offset_x,
        shadow_offset_y,
        shadow_alpha,
    )
}

fn serialize_caption_xml(caption: &Caption) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:caption side="{}" gap="{}" width="{}" lastWidth="{}" fullSz="{}">"#,
        caption_direction_to_xml(caption.direction),
        caption.spacing,
        caption.width,
        caption.max_width,
        bool_to_attr(caption.include_margin),
    ));
    xml.push_str(&format!(
        r#"<hp:subList vertAlign="{}">"#,
        caption_vert_align_to_xml(caption.vert_align),
    ));
    if caption.paragraphs.is_empty() {
        xml.push_str(&serialize_paragraph_xml(&Paragraph::new_empty(), None, None)?);
    } else {
        for para in &caption.paragraphs {
            xml.push_str(&serialize_paragraph_xml(para, None, None)?);
        }
    }
    xml.push_str(r#"</hp:subList>"#);
    xml.push_str(r#"</hp:caption>"#);
    Ok(xml)
}

fn serialize_header_footer_control(
    tag_name: &str,
    apply_to: HeaderFooterApply,
    paragraphs: &[Paragraph],
) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:{} applyPageType="{}">"#,
        tag_name,
        apply_to_xml(apply_to),
    ));
    xml.push_str(r#"<hp:subList>"#);
    for para in paragraphs {
        xml.push_str(&serialize_paragraph_xml(para, None, None)?);
    }
    xml.push_str(r#"</hp:subList>"#);
    xml.push_str(&format!(r#"</hp:{}>"#, tag_name));
    Ok(xml)
}

fn serialize_note_control(
    tag_name: &str,
    number: u16,
    paragraphs: &[Paragraph],
) -> Result<String, SerializeError> {
    let mut xml = String::new();
    xml.push_str(&format!(r#"<hp:{} number="{}">"#, tag_name, number));
    xml.push_str(r#"<hp:subList>"#);
    for para in paragraphs {
        xml.push_str(&serialize_paragraph_xml(para, None, None)?);
    }
    xml.push_str(r#"</hp:subList>"#);
    xml.push_str(&format!(r#"</hp:{}>"#, tag_name));
    Ok(xml)
}

fn serialize_auto_number_control(auto_number: &AutoNumber) -> String {
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:autoNum num="{}" numType="{}">"#,
        auto_number.number,
        auto_number_type_to_xml(auto_number.number_type),
    ));
    xml.push_str(&format!(
        r#"<hp:autoNumFormat type="{}" userChar="{}" prefixChar="{}" suffixChar="{}" supscript="{}"/>"#,
        auto_number.format,
        xml_escape_attr(&auto_number.user_symbol.to_string()),
        xml_escape_attr(&auto_number.prefix_char.to_string()),
        xml_escape_attr(&auto_number.suffix_char.to_string()),
        bool_to_attr(auto_number.superscript),
    ));
    xml.push_str(r#"</hp:autoNum>"#);
    xml
}

fn serialize_new_number_control(new_number: &NewNumber) -> String {
    format!(
        r#"<hp:newNum num="{}" numType="{}"/>"#,
        new_number.number,
        auto_number_type_to_xml(new_number.number_type),
    )
}

fn serialize_page_hide_control(page_hide: &PageHide) -> String {
    format!(
        concat!(
            r#"<hp:pageHiding hideHeader="{}" hideFooter="{}" hideMasterPage="{}" "#,
            r#"hideBorder="{}" hideFill="{}" hidePageNum="{}"/>"#
        ),
        bool_to_attr(page_hide.hide_header),
        bool_to_attr(page_hide.hide_footer),
        bool_to_attr(page_hide.hide_master_page),
        bool_to_attr(page_hide.hide_border),
        bool_to_attr(page_hide.hide_fill),
        bool_to_attr(page_hide.hide_page_num),
    )
}

fn serialize_page_number_control(page_number_pos: &PageNumberPos) -> String {
    format!(
        r#"<hp:pageNum pos="{}" formatType="{}" sideChar="{}"/>"#,
        page_number_position_to_xml(page_number_pos.position),
        page_number_format_to_xml(page_number_pos.format),
        xml_escape_attr(&page_number_pos.dash_char.to_string()),
    )
}

fn serialize_bookmark_control(bookmark: &Bookmark) -> String {
    format!(
        r#"<hp:bookmark name="{}"/>"#,
        xml_escape_attr(&bookmark.name)
    )
}

fn char_shape_id_at(para: &Paragraph, char_idx: usize) -> u32 {
    if para.char_shapes.is_empty() {
        return 0;
    }
    let utf16_pos = para
        .char_offsets
        .get(char_idx)
        .copied()
        .unwrap_or(char_idx as u32);
    let mut current = para.char_shapes[0].char_shape_id;
    for char_shape_ref in &para.char_shapes {
        if char_shape_ref.start_pos <= utf16_pos {
            current = char_shape_ref.char_shape_id;
        } else {
            break;
        }
    }
    current
}

fn media_type_for_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "wmf" => "image/wmf",
        "emf" => "image/emf",
        _ => "application/octet-stream",
    }
}

fn xml_escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_escape_attr(text: &str) -> String {
    xml_escape_text(text)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn color_to_hex(color: u32) -> String {
    let r = color & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = (color >> 16) & 0xFF;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn bool_to_attr(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn alpha_to_decimal(alpha: u8) -> String {
    format!("{:.3}", f64::from(alpha) / 255.0)
}

fn underline_type_to_xml(underline_type: UnderlineType) -> &'static str {
    match underline_type {
        UnderlineType::None => "NONE",
        UnderlineType::Bottom => "BOTTOM",
        UnderlineType::Top => "TOP",
    }
}

fn line_shape_to_xml(shape: u8) -> &'static str {
    match shape {
        1 => "DASH",
        2 => "DOT",
        3 => "DASH_DOT",
        4 => "DASH_DOT_DOT",
        5 => "LONG_DASH",
        6 => "CIRCLE",
        7 => "DOUBLE_SLIM",
        8 => "SLIM_THICK",
        9 => "THICK_SLIM",
        10 => "SLIM_THICK_SLIM",
        11 => "WAVE",
        12 => "DOUBLE_WAVE",
        _ => "SOLID",
    }
}

fn outline_type_to_xml(outline_type: u8) -> &'static str {
    match outline_type {
        1 => "SOLID",
        2 => "DASH",
        3 => "DOT",
        _ => "NONE",
    }
}

fn shadow_type_to_xml(shadow_type: u8) -> &'static str {
    match shadow_type {
        1 => "DROP",
        _ => "NONE",
    }
}

fn tab_type_to_xml(tab_type: u8) -> &'static str {
    match tab_type {
        1 => "RIGHT",
        2 => "CENTER",
        3 => "DECIMAL",
        _ => "LEFT",
    }
}

fn tab_leader_to_xml(fill_type: u8) -> &'static str {
    match fill_type {
        1 => "SOLID",
        2 => "DOT",
        3 => "DASH",
        4 => "DASH_DOT",
        5 => "DASH_DOT_DOT",
        6 => "LONG_DASH",
        7 => "CIRCLE",
        8 => "DOUBLE_LINE",
        9 => "THIN_THICK",
        10 => "THICK_THIN",
        11 => "TRIM",
        _ => "NONE",
    }
}

fn border_line_type_to_xml(line_type: BorderLineType) -> &'static str {
    match line_type {
        BorderLineType::None => "NONE",
        BorderLineType::Solid => "SOLID",
        BorderLineType::Dash => "DASH",
        BorderLineType::Dot => "DOT",
        BorderLineType::DashDot => "DASH_DOT",
        BorderLineType::DashDotDot => "DASH_DOT_DOT",
        BorderLineType::LongDash => "LONG_DASH",
        BorderLineType::Circle => "CIRCLE",
        BorderLineType::Double => "DOUBLE_SLIM",
        BorderLineType::ThinThickDouble => "SLIM_THICK",
        BorderLineType::ThickThinDouble => "THICK_SLIM",
        BorderLineType::ThinThickThinTriple => "SLIM_THICK_SLIM",
        BorderLineType::Wave => "WAVE",
        BorderLineType::DoubleWave => "DOUBLE_WAVE",
        BorderLineType::Thick3D | BorderLineType::Thin3D => "SOLID",
        BorderLineType::Thick3DReverse | BorderLineType::Thin3DReverse => "SOLID",
    }
}

fn border_width_to_xml(width: u8) -> &'static str {
    match width {
        0 => "0.12 mm",
        1 => "0.25 mm",
        2 => "0.5 mm",
        3 => "1.0 mm",
        4 => "1.5 mm",
        _ => "2.0 mm",
    }
}

fn alignment_to_xml(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Justify => "JUSTIFY",
        Alignment::Left => "LEFT",
        Alignment::Right => "RIGHT",
        Alignment::Center => "CENTER",
        Alignment::Distribute => "DISTRIBUTE",
        Alignment::Split => "JUSTIFY",
    }
}

fn head_type_to_xml(head_type: HeadType) -> &'static str {
    match head_type {
        HeadType::None => "NONE",
        HeadType::Outline => "OUTLINE",
        HeadType::Number => "NUMBER",
        HeadType::Bullet => "BULLET",
    }
}

fn line_spacing_type_to_xml(line_spacing_type: LineSpacingType) -> &'static str {
    match line_spacing_type {
        LineSpacingType::Percent => "PERCENT",
        LineSpacingType::Fixed => "FIXED",
        LineSpacingType::SpaceOnly => "SPACE_ONLY",
        LineSpacingType::Minimum => "MINIMUM",
    }
}

fn column_type_to_xml(column_type: ColumnType) -> &'static str {
    match column_type {
        ColumnType::Normal => "NEWSPAPER",
        ColumnType::Distribute => "BalancedNewspaper",
        ColumnType::Parallel => "Parallel",
    }
}

fn column_direction_to_xml(direction: ColumnDirection) -> &'static str {
    match direction {
        ColumnDirection::LeftToRight => "LEFT",
        ColumnDirection::RightToLeft => "RIGHT",
    }
}

fn text_wrap_to_xml(text_wrap: TextWrap) -> &'static str {
    match text_wrap {
        TextWrap::Square => "SQUARE",
        TextWrap::Tight => "TIGHT",
        TextWrap::Through => "THROUGH",
        TextWrap::TopAndBottom => "TOP_AND_BOTTOM",
        TextWrap::BehindText => "BEHIND_TEXT",
        TextWrap::InFrontOfText => "IN_FRONT_OF_TEXT",
    }
}

fn size_criterion_to_xml(size_criterion: crate::model::shape::SizeCriterion) -> &'static str {
    match size_criterion {
        crate::model::shape::SizeCriterion::Paper => "PAPER",
        crate::model::shape::SizeCriterion::Page => "PAGE",
        crate::model::shape::SizeCriterion::Column => "COLUMN",
        crate::model::shape::SizeCriterion::Para => "PARA",
        crate::model::shape::SizeCriterion::Absolute => "ABSOLUTE",
    }
}

fn vert_rel_to_xml(vert_rel_to: VertRelTo) -> &'static str {
    match vert_rel_to {
        VertRelTo::Paper => "PAPER",
        VertRelTo::Page => "PAGE",
        VertRelTo::Para => "PARA",
    }
}

fn horz_rel_to_xml(horz_rel_to: HorzRelTo) -> &'static str {
    match horz_rel_to {
        HorzRelTo::Paper => "PAPER",
        HorzRelTo::Page => "PAGE",
        HorzRelTo::Column => "COLUMN",
        HorzRelTo::Para => "PARA",
    }
}

fn vert_align_to_xml(vert_align: VertAlign) -> &'static str {
    match vert_align {
        VertAlign::Top => "TOP",
        VertAlign::Center => "CENTER",
        VertAlign::Bottom => "BOTTOM",
        VertAlign::Inside => "INSIDE",
        VertAlign::Outside => "OUTSIDE",
    }
}

fn horz_align_to_xml(horz_align: HorzAlign) -> &'static str {
    match horz_align {
        HorzAlign::Left => "LEFT",
        HorzAlign::Center => "CENTER",
        HorzAlign::Right => "RIGHT",
        HorzAlign::Inside => "INSIDE",
        HorzAlign::Outside => "OUTSIDE",
    }
}

fn vertical_align_to_xml(vertical_align: VerticalAlign) -> &'static str {
    match vertical_align {
        VerticalAlign::Top => "TOP",
        VerticalAlign::Center => "CENTER",
        VerticalAlign::Bottom => "BOTTOM",
    }
}

fn table_page_break_to_xml(page_break: TablePageBreak) -> &'static str {
    match page_break {
        TablePageBreak::None => "NONE",
        TablePageBreak::CellBreak => "CELL_BREAK",
        TablePageBreak::RowBreak => "ROW_BREAK",
    }
}

fn image_effect_to_xml(effect: ImageEffect) -> &'static str {
    match effect {
        ImageEffect::RealPic => "REAL_PIC",
        ImageEffect::GrayScale => "GRAY_SCALE",
        ImageEffect::BlackWhite => "BLACK_WHITE",
        ImageEffect::Pattern8x8 => "REAL_PIC",
    }
}

fn image_fill_mode_to_xml(mode: ImageFillMode) -> &'static str {
    match mode {
        ImageFillMode::TileAll => "TILE_ALL",
        ImageFillMode::TileHorzTop => "TILE_HORZ_TOP",
        ImageFillMode::TileHorzBottom => "TILE_HORZ_BOTTOM",
        ImageFillMode::TileVertLeft => "TILE_VERT_LEFT",
        ImageFillMode::TileVertRight => "TILE_VERT_RIGHT",
        ImageFillMode::FitToSize => "FIT_TO_SIZE",
        ImageFillMode::Center => "CENTER",
        ImageFillMode::CenterTop => "CENTER_TOP",
        ImageFillMode::CenterBottom => "CENTER_BOTTOM",
        ImageFillMode::LeftCenter => "CENTER",
        ImageFillMode::LeftTop => "TOP_LEFT_ALIGN",
        ImageFillMode::LeftBottom => "CENTER_BOTTOM",
        ImageFillMode::RightCenter => "CENTER",
        ImageFillMode::RightTop => "TOP_LEFT_ALIGN",
        ImageFillMode::RightBottom => "CENTER_BOTTOM",
        ImageFillMode::None => "TILE_ALL",
    }
}

fn outline_style_to_xml(outline_style: u8) -> &'static str {
    match outline_style {
        1 => "OUTER",
        2 => "INNER",
        _ => "NORMAL",
    }
}

fn caption_direction_to_xml(direction: CaptionDirection) -> &'static str {
    match direction {
        CaptionDirection::Left => "LEFT",
        CaptionDirection::Right => "RIGHT",
        CaptionDirection::Top => "TOP",
        CaptionDirection::Bottom => "BOTTOM",
    }
}

fn caption_vert_align_to_xml(align: CaptionVertAlign) -> &'static str {
    match align {
        CaptionVertAlign::Top => "TOP",
        CaptionVertAlign::Center => "CENTER",
        CaptionVertAlign::Bottom => "BOTTOM",
    }
}

fn arc_type_to_xml(arc_type: u8) -> &'static str {
    match arc_type {
        1 => "PIE",
        2 => "CHORD",
        _ => "NORMAL",
    }
}

fn apply_to_xml(apply_to: HeaderFooterApply) -> &'static str {
    match apply_to {
        HeaderFooterApply::Both => "BOTH",
        HeaderFooterApply::Even => "EVEN",
        HeaderFooterApply::Odd => "ODD",
    }
}

fn auto_number_type_to_xml(number_type: AutoNumberType) -> &'static str {
    match number_type {
        AutoNumberType::Page => "PAGE",
        AutoNumberType::Footnote => "FOOTNOTE",
        AutoNumberType::Endnote => "ENDNOTE",
        AutoNumberType::Picture => "PICTURE",
        AutoNumberType::Table => "TABLE",
        AutoNumberType::Equation => "EQUATION",
    }
}

fn field_type_to_xml(field_type: FieldType) -> &'static str {
    match field_type {
        FieldType::Unknown => "UNKNOWN",
        FieldType::Date => "DATE",
        FieldType::DocDate => "DOC_DATE",
        FieldType::Path => "PATH",
        FieldType::Bookmark => "BOOKMARK",
        FieldType::MailMerge => "MAILMERGE",
        FieldType::CrossRef => "CROSSREF",
        FieldType::Formula => "FORMULA",
        FieldType::ClickHere => "CLICK_HERE",
        FieldType::Summary => "SUMMARY",
        FieldType::UserInfo => "USER_INFO",
        FieldType::Hyperlink => "HYPERLINK",
        FieldType::Memo => "MEMO",
        FieldType::PrivateInfoSecurity => "PRIVATE_INFO",
        FieldType::TableOfContents => "TABLE_OF_CONTENTS",
    }
}

fn page_number_position_to_xml(position: u8) -> &'static str {
    match position {
        0 => "NONE",
        1 => "TOP_LEFT",
        2 => "TOP_CENTER",
        3 => "TOP_RIGHT",
        4 => "BOTTOM_LEFT",
        5 => "BOTTOM_CENTER",
        6 => "BOTTOM_RIGHT",
        7 => "OUTSIDE_TOP",
        8 => "OUTSIDE_BOTTOM",
        9 => "INSIDE_TOP",
        10 => "INSIDE_BOTTOM",
        _ => "BOTTOM_CENTER",
    }
}

fn page_number_format_to_xml(format: u8) -> &'static str {
    match format {
        1 => "CIRCLE_DIGIT",
        2 => "ROMAN_CAPITAL",
        3 => "ROMAN_SMALL",
        4 => "LATIN_CAPITAL",
        5 => "LATIN_SMALL",
        6 => "HANGUL",
        7 => "HANJA",
        _ => "DIGIT",
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Document;
    use crate::model::paragraph::{CharShapeRef, FieldRange, Paragraph};

    #[test]
    fn test_serialize_hwpx_roundtrip_text() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());

        let paragraph = Paragraph {
            text: "Hello HWPX".to_string(),
            char_offsets: (0..10).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            para_shape_id: 0,
            style_id: 0,
            char_count: 11,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].paragraphs[0].text, "Hello HWPX");
    }

    #[test]
    fn test_analyze_hwpx_support_rejects_shapes() {
        let mut document = Document::default();
        let mut paragraph = Paragraph::new_empty();
        paragraph.controls.push(Control::Shape(Box::new(
            crate::model::shape::ShapeObject::Line(Default::default()),
        )));
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(!report.is_supported());
        assert!(report.blockers.iter().any(|blocker| blocker.contains("drawing shapes")));
    }

    #[test]
    fn test_serialize_hwpx_roundtrip_bullet_document() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape {
            head_type: HeadType::Bullet,
            numbering_id: 1,
            ..Default::default()
        });
        document.doc_info.bullets.push(Bullet {
            bullet_char: '◦',
            width_adjust: 12,
            text_distance: 50,
            ..Default::default()
        });

        let paragraph = Paragraph {
            text: "Bullet item".to_string(),
            char_offsets: (0..11).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            para_shape_id: 0,
            style_id: 0,
            char_count: 12,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        assert_eq!(parsed.doc_info.bullets.len(), 1);
        assert_eq!(parsed.doc_info.bullets[0].bullet_char, '◦');
        assert_eq!(parsed.sections[0].paragraphs[0].para_shape_id, 0);
        assert_eq!(parsed.doc_info.para_shapes[0].head_type, HeadType::Bullet);
        assert_eq!(parsed.doc_info.para_shapes[0].numbering_id, 1);
    }

    #[test]
    fn test_serialize_hwpx_roundtrip_field_ranges() {
        let mut document = Document::default();
        document.doc_info.font_faces = vec![Vec::new(); 7];
        document.doc_info.char_shapes.push(CharShape::default());
        document.doc_info.para_shapes.push(ParaShape::default());

        let paragraph = Paragraph {
            text: "inside".to_string(),
            char_offsets: (0..6).collect(),
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            field_ranges: vec![FieldRange {
                start_char_idx: 0,
                end_char_idx: 6,
                control_idx: 0,
            }],
            controls: vec![Control::Field(Field {
                field_type: FieldType::ClickHere,
                command: Field::build_clickhere_command("inside", "", "Sample"),
                properties: 1,
                field_id: 77,
                ctrl_id: 7,
                ctrl_data_name: Some("Sample".to_string()),
                ..Default::default()
            })],
            para_shape_id: 0,
            style_id: 0,
            char_count: 7,
            has_para_text: true,
            ..Default::default()
        };
        document.sections.push(Section {
            paragraphs: vec![paragraph],
            ..Default::default()
        });

        let report = analyze_hwpx_support(&document);
        assert!(report.is_supported(), "{:?}", report.blockers);

        let bytes = serialize_hwpx(&document).expect("serialize hwpx");
        let parsed = crate::parser::parse_document(&bytes).expect("parse hwpx");
        let para = &parsed.sections[0].paragraphs[0];
        assert_eq!(para.text, "inside");
        assert_eq!(para.field_ranges.len(), 1);
        assert_eq!(para.field_ranges[0].start_char_idx, 0);
        assert_eq!(para.field_ranges[0].end_char_idx, 6);
        assert!(matches!(para.controls[0], Control::Field(_)));
    }

    #[test]
    fn test_analyze_hwpx_support_uses_stable_issue_codes() {
        let mut document = Document::default();
        document.doc_info.extra_records.push(Default::default());

        let report = analyze_hwpx_support(&document);
        assert!(report.issues.iter().any(|issue| issue.code == "hwpx-docinfo-extra-records"));
    }
}
*/
