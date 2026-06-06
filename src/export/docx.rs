use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::document_core::DocumentCore;
use crate::error::HwpError;
use crate::model::control::Control;
use crate::model::header_footer::HeaderFooterApply;
use crate::model::image::Picture;
use crate::model::page::PageDef;
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::model::style::{Alignment, UnderlineType};
use crate::model::table::Table;

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL_PKG_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const CORE_PROPS_NS: &str =
    "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const DCTERMS_NS: &str = "http://purl.org/dc/terms/";
const DCMITYPE_NS: &str = "http://purl.org/dc/dcmitype/";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

struct DocxImagePart<'a> {
    id: u16,
    extension: String,
    content_type: &'static str,
    data: &'a [u8],
}

pub fn export_document(core: &DocumentCore) -> Result<Vec<u8>, HwpError> {
    let image_parts = collect_image_parts(core);
    let header_xml = build_header_footer_xml(core, true);
    let footer_xml = build_header_footer_xml(core, false);
    let document_xml = build_document_xml(core, header_xml.is_some(), footer_xml.is_some());
    let document_rels =
        build_document_relationships(header_xml.is_some(), footer_xml.is_some(), &image_parts);
    let content_types =
        build_content_types(header_xml.is_some(), footer_xml.is_some(), &image_parts);
    let core_props = build_core_properties(core);
    let app_props = build_app_properties();

    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_part(&mut writer, "[Content_Types].xml", &content_types, options)?;
        write_part(
            &mut writer,
            "_rels/.rels",
            &build_root_relationships(),
            options,
        )?;
        write_part(&mut writer, "docProps/core.xml", &core_props, options)?;
        write_part(&mut writer, "docProps/app.xml", &app_props, options)?;
        write_part(&mut writer, "word/document.xml", &document_xml, options)?;
        write_part(
            &mut writer,
            "word/_rels/document.xml.rels",
            &document_rels,
            options,
        )?;

        if let Some(header_xml) = header_xml {
            write_part(&mut writer, "word/header1.xml", &header_xml, options)?;
        }
        if let Some(footer_xml) = footer_xml {
            write_part(&mut writer, "word/footer1.xml", &footer_xml, options)?;
        }
        for image in &image_parts {
            write_binary_part(
                &mut writer,
                &format!("word/media/image{}.{}", image.id, image.extension),
                image.data,
                options,
            )?;
        }

        writer
            .finish()
            .map_err(|error| HwpError::RenderError(format!("finalize docx archive: {}", error)))?;
    }

    Ok(output.into_inner())
}

fn write_binary_part(
    writer: &mut zip::ZipWriter<&mut Cursor<Vec<u8>>>,
    path: &str,
    contents: &[u8],
    options: SimpleFileOptions,
) -> Result<(), HwpError> {
    writer
        .start_file(path, options)
        .map_err(|error| HwpError::RenderError(format!("start docx part {}: {}", path, error)))?;
    writer
        .write_all(contents)
        .map_err(|error| HwpError::RenderError(format!("write docx part {}: {}", path, error)))
}

fn write_part(
    writer: &mut zip::ZipWriter<&mut Cursor<Vec<u8>>>,
    path: &str,
    contents: &str,
    options: SimpleFileOptions,
) -> Result<(), HwpError> {
    writer
        .start_file(path, options)
        .map_err(|error| HwpError::RenderError(format!("start docx part {}: {}", path, error)))?;
    writer
        .write_all(contents.as_bytes())
        .map_err(|error| HwpError::RenderError(format!("write docx part {}: {}", path, error)))
}

fn build_header_footer_xml(core: &DocumentCore, header: bool) -> Option<String> {
    let paragraphs = find_header_footer_paragraphs(core, header)?;
    let root_name = if header { "w:hdr" } else { "w:ftr" };
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <{} xmlns:w=\"{}\">",
        root_name, WORD_NS
    );
    for para in paragraphs {
        xml.push_str(&paragraph_to_word_xml(core, para));
    }
    xml.push_str(&format!("</{}>", root_name));
    Some(xml)
}

fn find_header_footer_paragraphs<'a>(
    core: &'a DocumentCore,
    header: bool,
) -> Option<&'a [Paragraph]> {
    for section in &core.document().sections {
        for para in &section.paragraphs {
            for control in &para.controls {
                match control {
                    Control::Header(item)
                        if header
                            && matches!(
                                item.apply_to,
                                HeaderFooterApply::Both | HeaderFooterApply::Odd
                            ) =>
                    {
                        if !item.paragraphs.is_empty() {
                            return Some(&item.paragraphs);
                        }
                    }
                    Control::Footer(item)
                        if !header
                            && matches!(
                                item.apply_to,
                                HeaderFooterApply::Both | HeaderFooterApply::Odd
                            ) =>
                    {
                        if !item.paragraphs.is_empty() {
                            return Some(&item.paragraphs);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn build_document_xml(core: &DocumentCore, has_header: bool, has_footer: bool) -> String {
    let section = core.document().sections.last();
    let page_def = section
        .map(|item| &item.section_def.page_def)
        .cloned()
        .unwrap_or_default();
    let sect_pr = build_section_properties(&page_def, has_header, has_footer);
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:document xmlns:w=\"{}\" xmlns:r=\"{}\" \
         xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
         xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
         xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
         <w:body>",
        WORD_NS, REL_NS
    );

    let mut has_content = false;
    for (section_index, section) in core.document().sections.iter().enumerate() {
        for para in &section.paragraphs {
            xml.push_str(&paragraph_to_word_xml(core, para));
            has_content = true;
            for control in &para.controls {
                xml.push_str(&control_to_word_xml(core, control));
            }
            if matches!(
                para.column_type,
                ColumnBreakType::Page | ColumnBreakType::Section
            ) {
                xml.push_str(page_break_xml());
            }
        }
        if section_index + 1 < core.document().sections.len() {
            xml.push_str(page_break_xml());
        }
    }
    if !has_content {
        xml.push_str("<w:p/>");
    }
    xml.push_str(&sect_pr);
    xml.push_str("</w:body></w:document>");
    xml
}

fn paragraph_to_word_xml(core: &DocumentCore, para: &Paragraph) -> String {
    let chars: Vec<char> = para.text.chars().collect();
    let mut xml = String::from("<w:p>");
    xml.push_str(&paragraph_properties_xml(core, para));

    for (start, end, style_id) in core.get_char_style_ranges(para, 0, chars.len()) {
        if start >= end || end > chars.len() {
            continue;
        }
        xml.push_str("<w:r>");
        xml.push_str(&run_properties_xml(core, style_id));
        append_word_text(&mut xml, &chars[start..end]);
        xml.push_str("</w:r>");
    }

    if !chars.is_empty() && !xml.contains("<w:r>") {
        xml.push_str("<w:r>");
        append_word_text(&mut xml, &chars);
        xml.push_str("</w:r>");
    }
    xml.push_str("</w:p>");
    xml
}

fn paragraph_properties_xml(core: &DocumentCore, para: &Paragraph) -> String {
    let Some(style) = core.styles.para_styles.get(para.para_shape_id as usize) else {
        return String::new();
    };

    let align = match style.alignment {
        Alignment::Left => "left",
        Alignment::Right => "right",
        Alignment::Center => "center",
        Alignment::Justify | Alignment::Distribute | Alignment::Split => "both",
    };
    let mut properties = format!("<w:jc w:val=\"{}\"/>", align);
    let left = px_to_twips(style.margin_left, core.dpi);
    let right = px_to_twips(style.margin_right, core.dpi);
    let indent = px_to_twips(style.indent, core.dpi);
    if left != 0 || right != 0 || indent != 0 {
        properties.push_str(&format!(
            "<w:ind w:left=\"{}\" w:right=\"{}\" w:firstLine=\"{}\"/>",
            left.max(0),
            right.max(0),
            indent.max(0)
        ));
    }
    let before = px_to_twips(style.spacing_before, core.dpi);
    let after = px_to_twips(style.spacing_after, core.dpi);
    if before != 0 || after != 0 {
        properties.push_str(&format!(
            "<w:spacing w:before=\"{}\" w:after=\"{}\"/>",
            before.max(0),
            after.max(0)
        ));
    }
    if style.keep_with_next {
        properties.push_str("<w:keepNext/>");
    }
    if style.keep_lines {
        properties.push_str("<w:keepLines/>");
    }
    if style.page_break_before {
        properties.push_str("<w:pageBreakBefore/>");
    }
    format!("<w:pPr>{}</w:pPr>", properties)
}

fn run_properties_xml(core: &DocumentCore, style_id: u32) -> String {
    let Some(style) = core.styles.char_styles.get(style_id as usize) else {
        return String::new();
    };

    let mut properties = String::new();
    if !style.font_family.is_empty() {
        let font = escape_xml_text(&style.font_family);
        properties.push_str(&format!(
            "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\" w:eastAsia=\"{}\"/>",
            font, font, font
        ));
    }
    let half_points = (style.font_size * 72.0 / core.dpi * 2.0).round().max(1.0) as u32;
    properties.push_str(&format!(
        "<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>",
        half_points, half_points
    ));
    if style.bold {
        properties.push_str("<w:b/>");
    }
    if style.italic {
        properties.push_str("<w:i/>");
    }
    if !matches!(style.underline, UnderlineType::None) {
        properties.push_str("<w:u w:val=\"single\"/>");
    }
    if style.strikethrough {
        properties.push_str("<w:strike/>");
    }
    if style.superscript {
        properties.push_str("<w:vertAlign w:val=\"superscript\"/>");
    } else if style.subscript {
        properties.push_str("<w:vertAlign w:val=\"subscript\"/>");
    }
    properties.push_str(&format!(
        "<w:color w:val=\"{}\"/>",
        color_ref_to_word_hex(style.text_color)
    ));
    format!("<w:rPr>{}</w:rPr>", properties)
}

fn append_word_text(xml: &mut String, chars: &[char]) {
    let mut text = String::new();
    let flush = |xml: &mut String, text: &mut String| {
        if text.is_empty() {
            return;
        }
        xml.push_str("<w:t xml:space=\"preserve\">");
        xml.push_str(&escape_xml_text(text));
        xml.push_str("</w:t>");
        text.clear();
    };

    for ch in chars {
        match ch {
            '\t' => {
                flush(xml, &mut text);
                xml.push_str("<w:tab/>");
            }
            '\n' | '\r' => {
                flush(xml, &mut text);
                xml.push_str("<w:br/>");
            }
            value if !value.is_control() => text.push(*value),
            _ => {}
        }
    }
    flush(xml, &mut text);
}

fn control_to_word_xml(core: &DocumentCore, control: &Control) -> String {
    match control {
        Control::Table(table) => table_to_word_xml(core, table),
        Control::Shape(shape) => crate::document_core::get_textbox_from_shape(shape)
            .map(|textbox| {
                textbox
                    .paragraphs
                    .iter()
                    .map(|para| paragraph_to_word_xml(core, para))
                    .collect::<String>()
            })
            .unwrap_or_default(),
        Control::Hyperlink(link) => {
            let text = if link.text.is_empty() {
                &link.url
            } else {
                &link.text
            };
            plain_word_paragraph(text)
        }
        Control::Footnote(note) => note_to_word_xml(core, "주석", note.number, &note.paragraphs),
        Control::Endnote(note) => note_to_word_xml(core, "미주", note.number, &note.paragraphs),
        Control::Equation(equation) => plain_word_paragraph(&format!(
            "수식: {}",
            if equation.script.is_empty() {
                "수식"
            } else {
                &equation.script
            }
        )),
        Control::Picture(picture) => {
            picture_to_word_xml(core, picture).unwrap_or_else(|| plain_word_paragraph("[그림]"))
        }
        Control::Ruby(ruby) if !ruby.ruby_text.is_empty() => plain_word_paragraph(&ruby.ruby_text),
        Control::CharOverlap(overlap) if !overlap.chars.is_empty() => {
            plain_word_paragraph(&overlap.chars.iter().collect::<String>())
        }
        Control::HiddenComment(comment) => comment
            .paragraphs
            .iter()
            .map(|para| paragraph_to_word_xml(core, para))
            .collect(),
        _ => String::new(),
    }
}

fn picture_to_word_xml(core: &DocumentCore, picture: &Picture) -> Option<String> {
    let bin_data_id = picture.image_attr.bin_data_id;
    if bin_data_id == 0 {
        return None;
    }
    let image = core
        .document
        .bin_data_content
        .get((bin_data_id - 1) as usize)?;
    image_content_type(&image.extension, &image.data)?;

    let width = (i64::from(picture.common.width).max(1) * 127).max(1);
    let height = (i64::from(picture.common.height).max(1) * 127).max(1);
    Some(format!(
        "<w:p><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
         <wp:extent cx=\"{width}\" cy=\"{height}\"/>\
         <wp:docPr id=\"{bin_data_id}\" name=\"Picture {bin_data_id}\"/>\
         <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
         <pic:pic><pic:nvPicPr><pic:cNvPr id=\"0\" name=\"Picture {bin_data_id}\"/>\
         <pic:cNvPicPr/></pic:nvPicPr><pic:blipFill>\
         <a:blip r:embed=\"rIdImage{bin_data_id}\"/><a:stretch><a:fillRect/></a:stretch>\
         </pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/>\
         <a:ext cx=\"{width}\" cy=\"{height}\"/></a:xfrm>\
         <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr>\
         </pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"
    ))
}

fn table_to_word_xml(core: &DocumentCore, table: &Table) -> String {
    let mut xml = String::from(
        "<w:tbl><w:tblPr><w:tblBorders>\
         <w:top w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         <w:left w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         <w:bottom w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         <w:right w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         <w:insideH w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         <w:insideV w:val=\"single\" w:sz=\"4\" w:color=\"auto\"/>\
         </w:tblBorders></w:tblPr>",
    );
    for row in 0..table.row_count {
        let mut cells: Vec<_> = table.cells.iter().filter(|cell| cell.row == row).collect();
        cells.sort_by_key(|cell| cell.col);
        if cells.is_empty() {
            continue;
        }
        xml.push_str("<w:tr>");
        for cell in cells {
            xml.push_str("<w:tc><w:tcPr>");
            if cell.width > 0 {
                xml.push_str(&format!(
                    "<w:tcW w:w=\"{}\" w:type=\"dxa\"/>",
                    hwpunit_to_twips(cell.width)
                ));
            }
            if cell.col_span > 1 {
                xml.push_str(&format!("<w:gridSpan w:val=\"{}\"/>", cell.col_span));
            }
            if cell.row_span > 1 {
                xml.push_str("<w:vMerge w:val=\"restart\"/>");
            }
            xml.push_str("</w:tcPr>");
            if cell.paragraphs.is_empty() {
                xml.push_str("<w:p/>");
            } else {
                for para in &cell.paragraphs {
                    xml.push_str(&paragraph_to_word_xml(core, para));
                    for control in &para.controls {
                        xml.push_str(&control_to_word_xml(core, control));
                    }
                }
            }
            xml.push_str("</w:tc>");
        }
        xml.push_str("</w:tr>");
    }
    xml.push_str("</w:tbl>");
    xml
}

fn note_to_word_xml(
    core: &DocumentCore,
    label: &str,
    number: u16,
    paragraphs: &[Paragraph],
) -> String {
    let mut xml = plain_word_paragraph(&format!("{} {}.", label, number));
    for para in paragraphs {
        xml.push_str(&paragraph_to_word_xml(core, para));
    }
    xml
}

fn plain_word_paragraph(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut xml = String::from("<w:p><w:r>");
    append_word_text(&mut xml, &chars);
    xml.push_str("</w:r></w:p>");
    xml
}

fn page_break_xml() -> &'static str {
    "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>"
}

fn px_to_twips(value: f64, dpi: f64) -> i32 {
    (value * 72.0 / dpi * 20.0).round() as i32
}

fn color_ref_to_word_hex(color: u32) -> String {
    let blue = (color >> 16) & 0xff;
    let green = (color >> 8) & 0xff;
    let red = color & 0xff;
    format!("{:02X}{:02X}{:02X}", red, green, blue)
}

fn build_section_properties(page_def: &PageDef, has_header: bool, has_footer: bool) -> String {
    let (page_width, page_height) = if page_def.landscape {
        (page_def.height, page_def.width)
    } else {
        (page_def.width, page_def.height)
    };
    let width = hwpunit_to_twips(page_width);
    let height = hwpunit_to_twips(page_height);
    let left = hwpunit_to_twips(page_def.margin_left + page_def.margin_gutter);
    let right = hwpunit_to_twips(page_def.margin_right);
    let top = hwpunit_to_twips(page_def.margin_header + page_def.margin_top);
    let bottom = hwpunit_to_twips(page_def.margin_footer + page_def.margin_bottom);
    let header = hwpunit_to_twips(page_def.margin_header);
    let footer = hwpunit_to_twips(page_def.margin_footer);
    let gutter = hwpunit_to_twips(page_def.margin_gutter);

    let mut xml = String::from("<w:sectPr>");
    if has_header {
        xml.push_str("<w:headerReference w:type=\"default\" r:id=\"rIdHeader1\"/>");
    }
    if has_footer {
        xml.push_str("<w:footerReference w:type=\"default\" r:id=\"rIdFooter1\"/>");
    }
    xml.push_str(&format!(
        "<w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"{}\" w:footer=\"{}\" w:gutter=\"{}\"/>",
        width, height, top, right, bottom, left, header, footer, gutter
    ));
    xml.push_str("</w:sectPr>");
    xml
}

fn build_content_types(
    has_header: bool,
    has_footer: bool,
    image_parts: &[DocxImagePart<'_>],
) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Types xmlns=\"{}\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/word/document.xml\" ContentType=\"{}\"/>\
         <Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
         <Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>",
        CONTENT_TYPES_NS, DOCX_MIME
    );
    if has_header {
        xml.push_str("<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>");
    }
    if has_footer {
        xml.push_str("<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>");
    }
    let mut image_extensions = Vec::new();
    for image in image_parts {
        if image_extensions.contains(&image.extension) {
            continue;
        }
        image_extensions.push(image.extension.clone());
        xml.push_str(&format!(
            "<Default Extension=\"{}\" ContentType=\"{}\"/>",
            escape_xml_text(&image.extension),
            image.content_type
        ));
    }
    xml.push_str("</Types>");
    xml
}

fn build_root_relationships() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Relationships xmlns=\"{}\">\
         <Relationship Id=\"rId1\" Type=\"{}/officeDocument\" Target=\"word/document.xml\"/>\
         <Relationship Id=\"rId2\" Type=\"{}/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
         <Relationship Id=\"rId3\" Type=\"{}/extended-properties\" Target=\"docProps/app.xml\"/>\
         </Relationships>",
        REL_PKG_NS, REL_NS, REL_PKG_NS, REL_NS
    )
}

fn build_document_relationships(
    has_header: bool,
    has_footer: bool,
    image_parts: &[DocxImagePart<'_>],
) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Relationships xmlns=\"{}\">",
        REL_PKG_NS
    );
    if has_header {
        xml.push_str(&format!(
            "<Relationship Id=\"rIdHeader1\" Type=\"{}/header\" Target=\"header1.xml\"/>",
            REL_NS
        ));
    }
    if has_footer {
        xml.push_str(&format!(
            "<Relationship Id=\"rIdFooter1\" Type=\"{}/footer\" Target=\"footer1.xml\"/>",
            REL_NS
        ));
    }
    for image in image_parts {
        xml.push_str(&format!(
            "<Relationship Id=\"rIdImage{}\" Type=\"{}/image\" Target=\"media/image{}.{}\"/>",
            image.id, REL_NS, image.id, image.extension
        ));
    }
    xml.push_str("</Relationships>");
    xml
}

fn collect_image_parts(core: &DocumentCore) -> Vec<DocxImagePart<'_>> {
    core.document
        .bin_data_content
        .iter()
        .enumerate()
        .filter_map(|(index, image)| {
            let (extension, content_type) = image_content_type(&image.extension, &image.data)?;
            Some(DocxImagePart {
                id: (index + 1) as u16,
                extension,
                content_type,
                data: &image.data,
            })
        })
        .collect()
}

fn image_content_type(extension: &str, data: &[u8]) -> Option<(String, &'static str)> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    let normalized = match extension.as_str() {
        "jpeg" => "jpg",
        "tif" => "tiff",
        known @ ("jpg" | "png" | "gif" | "bmp" | "tiff" | "wmf" | "emf") => known,
        _ if data.starts_with(&[0x89, b'P', b'N', b'G']) => "png",
        _ if data.starts_with(&[0xff, 0xd8, 0xff]) => "jpg",
        _ if data.starts_with(b"GIF8") => "gif",
        _ if data.starts_with(b"BM") => "bmp",
        _ if data.starts_with(&[0x49, 0x49, 0x2a, 0x00])
            || data.starts_with(&[0x4d, 0x4d, 0x00, 0x2a]) =>
        {
            "tiff"
        }
        _ => return None,
    };
    let content_type = match normalized {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" => "image/tiff",
        "wmf" => "image/x-wmf",
        "emf" => "image/x-emf",
        _ => return None,
    };
    Some((normalized.to_string(), content_type))
}

fn build_core_properties(core: &DocumentCore) -> String {
    let title = if core.file_name.is_empty() {
        "Geulbit X 문서"
    } else {
        core.file_name.as_str()
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\" xmlns:dcterms=\"{}\" xmlns:dcmitype=\"{}\" xmlns:xsi=\"{}\">\
         <dc:title>{}</dc:title>\
         <dc:creator>Geulbit X</dc:creator>\
         <cp:lastModifiedBy>Geulbit X</cp:lastModifiedBy>\
         <dcterms:created xsi:type=\"dcterms:W3CDTF\">2026-04-22T00:00:00Z</dcterms:created>\
         <dcterms:modified xsi:type=\"dcterms:W3CDTF\">2026-04-22T00:00:00Z</dcterms:modified>\
         </cp:coreProperties>",
        CORE_PROPS_NS,
        DC_NS,
        DCTERMS_NS,
        DCMITYPE_NS,
        XSI_NS,
        escape_xml_text(title),
    )
}

fn build_app_properties() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
     <Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\" \
     xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\">\
     <Application>Geulbit X</Application><DocSecurity>0</DocSecurity><ScaleCrop>false</ScaleCrop>\
     <SharedDoc>false</SharedDoc><HyperlinksChanged>false</HyperlinksChanged><AppVersion>0.1</AppVersion>\
     </Properties>"
        .to_string()
}

fn hwpunit_to_twips(value: u32) -> u32 {
    value / 5
}

fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn read_part(bytes: &[u8], path: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("open docx");
        let mut file = archive.by_name(path).expect("find docx part");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("read docx part");
        contents
    }

    #[test]
    fn docx_body_uses_native_wordprocessingml() {
        let mut core = DocumentCore::new_empty();
        core.create_blank_document_native()
            .expect("create blank document");
        core.insert_text_native(0, 0, 0, "Geulbit X 문서 sS")
            .expect("insert text");

        let bytes = export_document(&core).expect("export docx");
        let document_xml = read_part(&bytes, "word/document.xml");
        assert!(document_xml.contains("Geulbit X 문서 sS"));
        assert!(!document_xml.contains("altChunk"));

        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).expect("open docx");
        assert!(archive.by_name("word/afchunk.html").is_err());
    }

    #[test]
    fn docx_preserves_text_from_real_hwp() {
        let core = DocumentCore::from_bytes(include_bytes!("../../samples/re-01-hangul-only.hwp"))
            .expect("parse sample hwp");
        let expected = core
            .document
            .sections
            .iter()
            .flat_map(|section| &section.paragraphs)
            .map(|para| para.text.trim())
            .find(|text| !text.is_empty())
            .expect("sample text");

        let bytes = export_document(&core).expect("export sample docx");
        let document_xml = read_part(&bytes, "word/document.xml");
        assert!(document_xml.contains(&escape_xml_text(expected)));
    }

    #[test]
    fn docx_embeds_images_from_hwpx() {
        let core = DocumentCore::from_bytes(include_bytes!("../../samples/tac-img-02.hwpx"))
            .expect("parse image sample");
        let bytes = export_document(&core).expect("export image docx");
        let document_xml = read_part(&bytes, "word/document.xml");
        assert!(document_xml.contains("<w:drawing>"));
        assert!(document_xml.contains("r:embed=\"rIdImage"));

        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("open docx");
        let has_media = (0..archive.len()).any(|index| {
            archive
                .by_index(index)
                .map(|file| file.name().starts_with("word/media/image"))
                .unwrap_or(false)
        });
        assert!(has_media);
    }
}
