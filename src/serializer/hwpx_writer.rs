use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::model::control::{
    AutoNumber, AutoNumberType, Bookmark, Control, NewNumber, PageHide, PageNumberPos,
};
use crate::model::document::{Document, Section, SectionDef};
use crate::model::header_footer::HeaderFooterApply;
use crate::model::image::{ImageEffect, Picture};
use crate::model::page::{ColumnDef, ColumnDirection, ColumnType, PageDef};
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::model::shape::{
    Caption, CaptionDirection, CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertAlign,
    VertRelTo,
};
use crate::model::style::{
    Alignment, BorderFill, BorderLine, BorderLineType, CharShape, FillType, Font, HeadType,
    ImageFillMode, LineSpacingType, Numbering, ParaShape, TabDef, UnderlineType,
};
use crate::model::table::{Cell, Table, TablePageBreak, VerticalAlign};
use crate::document_core::helpers::find_control_text_positions;

use super::cfb_writer::SerializeError;

#[derive(Debug, Default, Clone)]
pub struct HwpxSupportReport {
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl HwpxSupportReport {
    fn block(&mut self, msg: impl Into<String>) {
        self.blockers.push(msg.into());
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    fn dedupe(&mut self) {
        self.blockers = dedupe_messages(&self.blockers);
        self.warnings = dedupe_messages(&self.warnings);
    }

    pub fn is_supported(&self) -> bool {
        self.blockers.is_empty()
    }
}

fn dedupe_messages(messages: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for message in messages {
        if seen.insert(message.clone()) {
            deduped.push(message.clone());
        }
    }
    deduped
}

pub fn analyze_hwpx_support(doc: &Document) -> HwpxSupportReport {
    let mut report = HwpxSupportReport::default();

    if doc.header.encrypted {
        report.block("Encrypted documents cannot be saved as HWPX yet.");
    }
    if doc.header.distribution {
        report.block("Distribution documents stay protected to avoid save corruption.");
    }
    if !doc.doc_info.extra_records.is_empty() {
        report.block("Extra DocInfo records are not preserved in HWPX saves yet.");
    }
    if !doc.doc_info.bullets.is_empty() {
        report.block("Bullet definitions are not written by the HWPX serializer yet.");
    }
    if !doc.extra_streams.is_empty() {
        report.block("Extra binary streams are not preserved in HWPX packages yet.");
    }
    if doc.preview.is_some() {
        report.warn("Preview streams are regenerated lazily and are not preserved in HWPX yet.");
    }

    for (section_idx, section) in doc.sections.iter().enumerate() {
        analyze_section(section, section_idx, &mut report);
    }

    report.dedupe();
    report
}

fn analyze_section(section: &Section, section_idx: usize, report: &mut HwpxSupportReport) {
    if section.section_def.page_border_fill.border_fill_id != 0 {
        report.block(format!(
            "Section {} uses page border/fill settings that are not written to HWPX yet.",
            section_idx
        ));
    }
    if !section.section_def.master_pages.is_empty() {
        report.block(format!(
            "Section {} uses master pages that are not written to HWPX yet.",
            section_idx
        ));
    }
    if !section.section_def.extra_page_border_fills.is_empty()
        || !section.section_def.extra_child_records.is_empty()
    {
        report.block(format!(
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
        report.block(format!(
            "Paragraph {} in section {} uses numbering restarts that are not written to HWPX yet.",
            para_idx, section_idx
        ));
    }
    if para.text.chars().any(|ch| matches!(ch, '\u{0003}' | '\u{0004}')) {
        report.block(format!(
            "Paragraph {} in section {} contains field markers that are not written to HWPX yet.",
            para_idx, section_idx
        ));
    }

    for control in &para.controls {
        analyze_control(control, section_idx, para_idx, report);
    }
}

fn analyze_control(
    control: &Control,
    section_idx: usize,
    para_idx: usize,
    report: &mut HwpxSupportReport,
) {
    match control {
        Control::SectionDef(_) => report.block(format!(
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
            if pic.caption.is_some() {
                report.block(format!(
                    "Paragraph {} in section {} uses picture captions that are not written to HWPX yet.",
                    para_idx, section_idx
                ));
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
        Control::Field(_) => report.block(format!(
            "Paragraph {} in section {} uses fields that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::HiddenComment(_) => report.block(format!(
            "Paragraph {} in section {} uses hidden comments that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Shape(_) => report.block(format!(
            "Paragraph {} in section {} uses drawing shapes that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Equation(_) => report.block(format!(
            "Paragraph {} in section {} uses equations that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Form(_) => report.block(format!(
            "Paragraph {} in section {} uses form controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Hyperlink(_) => report.block(format!(
            "Paragraph {} in section {} uses hyperlink controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Ruby(_) => report.block(format!(
            "Paragraph {} in section {} uses ruby annotations that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::CharOverlap(_) => report.block(format!(
            "Paragraph {} in section {} uses character-overlap controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
        Control::Unknown(_) => report.block(format!(
            "Paragraph {} in section {} contains unknown controls that are not written to HWPX yet.",
            para_idx, section_idx
        )),
    }
}

pub fn serialize_hwpx(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let report = analyze_hwpx_support(doc);
    if !report.is_supported() {
        return Err(SerializeError::CfbError(report.blockers.join(" ")));
    }

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    write_zip_entry(
        &mut writer,
        "Contents/content.hpf",
        &serialize_content_hpf(doc),
        options,
    )?;
    write_zip_entry(
        &mut writer,
        "Contents/header.xml",
        &serialize_header_xml(doc),
        options,
    )?;

    for (index, section) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{}.xml", index);
        let xml = serialize_section_xml(section)?;
        write_zip_entry(&mut writer, &path, &xml, options)?;
    }

    let mut written_bin_paths = BTreeSet::new();
    for content in &doc.bin_data_content {
        let path = format!("BinData/image{}.{}", content.id, content.extension);
        writer
            .start_file(path.as_str(), options)
            .map_err(|e| SerializeError::CfbError(e.to_string()))?;
        writer
            .write_all(&content.data)
            .map_err(|e| SerializeError::CfbError(e.to_string()))?;
        written_bin_paths.insert(path);
    }

    for info in &doc.doc_info.bin_data_list {
        if let Some(ext) = info.extension.as_ref() {
            let path = format!("BinData/image{}.{}", info.storage_id, ext);
            if written_bin_paths.contains(&path) {
                continue;
            }
            writer
                .start_file(path.as_str(), options)
                .map_err(|e| SerializeError::CfbError(e.to_string()))?;
        }
    }

    let cursor = writer
        .finish()
        .map_err(|e| SerializeError::CfbError(e.to_string()))?;
    Ok(cursor.into_inner())
}

fn write_zip_entry(
    writer: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    path: &str,
    content: &str,
    options: SimpleFileOptions,
) -> Result<(), SerializeError> {
    writer
        .start_file(path, options)
        .map_err(|e| SerializeError::CfbError(e.to_string()))?;
    writer
        .write_all(content.as_bytes())
        .map_err(|e| SerializeError::CfbError(e.to_string()))
}

fn serialize_content_hpf(doc: &Document) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(
        r#"<opf:package xmlns:opf="http://www.idpf.org/2007/opf/" version="3.0" unique-identifier="rhwp-doc" id="rhwp-doc">"#,
    );
    xml.push_str(r#"<opf:manifest>"#);
    xml.push_str(r#"<opf:item id="header" href="Contents/header.xml" media-type="application/xml"/>"#);

    for content in &doc.bin_data_content {
        let media_type = media_type_for_extension(&content.extension);
        xml.push_str(&format!(
            r#"<opf:item id="image{}" href="BinData/image{}.{}" media-type="{}" isEmbeded="1"/>"#,
            content.id,
            content.id,
            xml_escape_attr(&content.extension),
            media_type,
        ));
    }

    for index in 0..doc.sections.len().max(1) {
        xml.push_str(&format!(
            r#"<opf:item id="section{}" href="Contents/section{}.xml" media-type="application/xml"/>"#,
            index, index
        ));
    }

    xml.push_str(r#"</opf:manifest><opf:spine>"#);
    xml.push_str(r#"<opf:itemref idref="header" linear="yes"/>"#);
    for index in 0..doc.sections.len().max(1) {
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
        r#"<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">"#,
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
        controls_by_pos.entry(pos).or_default().push(ctrl_idx);
    }

    let mut cursor = 0usize;
    while cursor <= text_chars.len() {
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
            if skip_text_indices.contains(&end) || controls_by_pos.contains_key(&end) {
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
    let mut xml = String::new();
    xml.push_str(&format!(
        r#"<hp:pic zOrder="{}" textWrap="{}" instid="{}" groupLevel="{}">"#,
        picture.common.z_order,
        text_wrap_to_xml(picture.common.text_wrap),
        picture.common.instance_id,
        picture.shape_attr.group_level,
    ));
    xml.push_str(&format!(
        r#"<hp:sz width="{}" height="{}"/>"#,
        picture.common.width,
        picture.common.height,
    ));
    xml.push_str(&serialize_pos_xml(&picture.common));
    xml.push_str(&format!(
        r#"<hp:outMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        picture.common.margin.left,
        picture.common.margin.right,
        picture.common.margin.top,
        picture.common.margin.bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:inMargin left="{}" right="{}" top="{}" bottom="{}"/>"#,
        picture.padding.left,
        picture.padding.right,
        picture.padding.top,
        picture.padding.bottom,
    ));
    xml.push_str(&format!(
        r#"<hp:imgClip left="{}" right="{}" top="{}" bottom="{}"/>"#,
        picture.crop.left,
        picture.crop.right,
        picture.crop.top,
        picture.crop.bottom,
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

fn serialize_pos_xml(common: &CommonObjAttr) -> String {
    format!(
        r#"<hp:pos treatAsChar="{}" vertRelTo="{}" horzRelTo="{}" vertAlign="{}" horzAlign="{}" vertOffset="{}" horzOffset="{}"/>"#,
        bool_to_attr(common.treat_as_char),
        vert_rel_to_xml(common.vert_rel_to),
        horz_rel_to_xml(common.horz_rel_to),
        vert_align_to_xml(common.vert_align),
        horz_align_to_xml(common.horz_align),
        common.vertical_offset,
        common.horizontal_offset,
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
    for para in &caption.paragraphs {
        xml.push_str(&serialize_paragraph_xml(para, None, None)?);
    }
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

fn caption_direction_to_xml(direction: CaptionDirection) -> &'static str {
    match direction {
        CaptionDirection::Left => "LEFT",
        CaptionDirection::Right => "RIGHT",
        CaptionDirection::Top => "TOP",
        CaptionDirection::Bottom => "BOTTOM",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Document;
    use crate::model::paragraph::{CharShapeRef, Paragraph};

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
}
