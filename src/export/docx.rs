use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::document_core::DocumentCore;
use crate::error::HwpError;
use crate::model::control::Control;
use crate::model::header_footer::HeaderFooterApply;
use crate::model::page::PageDef;
use crate::model::paragraph::{ColumnBreakType, Paragraph};

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

pub fn export_document(core: &DocumentCore) -> Result<Vec<u8>, HwpError> {
    let html = build_document_html(core);
    let header_xml = build_header_footer_xml(core, true);
    let footer_xml = build_header_footer_xml(core, false);
    let document_xml = build_document_xml(core, header_xml.is_some(), footer_xml.is_some());
    let document_rels = build_document_relationships(header_xml.is_some(), footer_xml.is_some());
    let content_types = build_content_types(header_xml.is_some(), footer_xml.is_some());
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
        write_part(&mut writer, "word/afchunk.html", &html, options)?;

        if let Some(header_xml) = header_xml {
            write_part(&mut writer, "word/header1.xml", &header_xml, options)?;
        }
        if let Some(footer_xml) = footer_xml {
            write_part(&mut writer, "word/footer1.xml", &footer_xml, options)?;
        }

        writer
            .finish()
            .map_err(|error| HwpError::RenderError(format!("finalize docx archive: {}", error)))?;
    }

    Ok(output.into_inner())
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

fn build_document_html(core: &DocumentCore) -> String {
    let mut html = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
         <style>\
         body{font-family:'Malgun Gothic','Apple SD Gothic Neo','Nanum Gothic',sans-serif;margin:0;}\
         section{page-break-after:always;padding:0 0 12pt 0;}\
         section:last-child{page-break-after:auto;}\
         p{margin:0;}\
         table{border-collapse:collapse;}\
         img{max-width:100%;}\
         .rhwp-note{font-size:9pt;color:#555;margin-top:4pt;}\
         .rhwp-unsupported{font-size:9pt;color:#666;border:1px solid #ddd;padding:6pt;margin:4pt 0;}\
         </style></head><body>",
    );

    for (section_index, section) in core.document().sections.iter().enumerate() {
        html.push_str("<section>");
        for para in &section.paragraphs {
            html.push_str(&paragraph_with_controls_to_html(core, para));
            if matches!(
                para.column_type,
                ColumnBreakType::Page | ColumnBreakType::Section
            ) {
                html.push_str("<div style=\"page-break-after:always\"></div>");
            }
        }

        let footnotes = collect_notes_html(core, section, true);
        if !footnotes.is_empty() {
            html.push_str("<hr><div class=\"rhwp-note\">");
            html.push_str(&footnotes);
            html.push_str("</div>");
        }

        let endnotes = collect_notes_html(core, section, false);
        if !endnotes.is_empty() {
            html.push_str("<hr><div class=\"rhwp-note\">");
            html.push_str(&endnotes);
            html.push_str("</div>");
        }

        html.push_str("</section>");
        if section_index + 1 < core.document().sections.len() {
            html.push_str("<div style=\"page-break-after:always\"></div>");
        }
    }

    html.push_str("</body></html>");
    html
}

fn paragraph_with_controls_to_html(core: &DocumentCore, para: &Paragraph) -> String {
    let mut html = String::new();
    let text_html = core.paragraph_to_html(para, None, None);
    if !text_html.is_empty() {
        html.push_str(&text_html);
    }

    for control in &para.controls {
        match control {
            Control::SectionDef(_)
            | Control::ColumnDef(_)
            | Control::Header(_)
            | Control::Footer(_) => {}
            Control::Bookmark(bookmark) => {
                if !bookmark.name.is_empty() {
                    html.push_str(&format!(
                        "<a id=\"{}\"></a>",
                        escape_html_attr(&bookmark.name)
                    ));
                }
            }
            Control::Hyperlink(link) => {
                let text = if link.text.is_empty() {
                    &link.url
                } else {
                    &link.text
                };
                html.push_str(&format!(
                    "<p><a href=\"{}\">{}</a></p>",
                    escape_html_attr(&link.url),
                    escape_html_text(text),
                ));
            }
            Control::Footnote(footnote) => {
                html.push_str(&format!(
                    "<div class=\"rhwp-note\">주석 {}. {}</div>",
                    footnote.number,
                    note_paragraphs_plain_text(&footnote.paragraphs),
                ));
            }
            Control::Endnote(endnote) => {
                html.push_str(&format!(
                    "<div class=\"rhwp-note\">미주 {}. {}</div>",
                    endnote.number,
                    note_paragraphs_plain_text(&endnote.paragraphs),
                ));
            }
            Control::Equation(equation) => {
                let script = if equation.script.is_empty() {
                    "수식"
                } else {
                    &equation.script
                };
                html.push_str(&format!(
                    "<div class=\"rhwp-unsupported\" data-rhwp-control=\"equation\">수식: {}</div>",
                    escape_html_text(script),
                ));
            }
            Control::Shape(shape) => {
                if let Some(textbox) = crate::document_core::get_textbox_from_shape(shape) {
                    for textbox_para in &textbox.paragraphs {
                        html.push_str(&core.paragraph_to_html(textbox_para, None, None));
                    }
                } else {
                    html.push_str(
                        "<div class=\"rhwp-unsupported\" data-rhwp-control=\"shape\">지원되지 않는 도형은 이미지 대체 없이 내보냈습니다.</div>",
                    );
                }
            }
            _ => html.push_str(&core.control_to_html(control)),
        }
    }

    html
}

fn collect_notes_html(
    core: &DocumentCore,
    section: &crate::model::document::Section,
    footnotes: bool,
) -> String {
    let mut html = String::new();
    for para in &section.paragraphs {
        for control in &para.controls {
            match control {
                Control::Footnote(note) if footnotes => {
                    html.push_str(&format!(
                        "<div class=\"rhwp-note\"><div>{}.</div>",
                        note.number
                    ));
                    for note_para in &note.paragraphs {
                        html.push_str(&core.paragraph_to_html(note_para, None, None));
                    }
                    html.push_str("</div>");
                }
                Control::Endnote(note) if !footnotes => {
                    html.push_str(&format!(
                        "<div class=\"rhwp-note\"><div>{}.</div>",
                        note.number
                    ));
                    for note_para in &note.paragraphs {
                        html.push_str(&core.paragraph_to_html(note_para, None, None));
                    }
                    html.push_str("</div>");
                }
                _ => {}
            }
        }
    }
    html
}

fn note_paragraphs_plain_text(paragraphs: &[Paragraph]) -> String {
    paragraphs
        .iter()
        .map(|para| escape_html_text(para.text.trim()))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
        let text = para.text.trim();
        if text.is_empty() {
            xml.push_str("<w:p/>");
            continue;
        }
        xml.push_str("<w:p><w:r><w:t xml:space=\"preserve\">");
        xml.push_str(&escape_xml_text(text));
        xml.push_str("</w:t></w:r></w:p>");
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
    let section = core.document().sections.first();
    let page_def = section
        .map(|item| &item.section_def.page_def)
        .cloned()
        .unwrap_or_default();
    let sect_pr = build_section_properties(&page_def, has_header, has_footer);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:document xmlns:w=\"{}\" xmlns:r=\"{}\">\
         <w:body><w:altChunk r:id=\"htmlChunk\"/>{}</w:body></w:document>",
        WORD_NS, REL_NS, sect_pr
    )
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

fn build_content_types(has_header: bool, has_footer: bool) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Types xmlns=\"{}\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Default Extension=\"html\" ContentType=\"text/html\"/>\
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

fn build_document_relationships(has_header: bool, has_footer: bool) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <Relationships xmlns=\"{}\">\
         <Relationship Id=\"htmlChunk\" Type=\"{}/aFChunk\" Target=\"afchunk.html\"/>",
        REL_PKG_NS, REL_NS
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
    xml.push_str("</Relationships>");
    xml
}

fn build_core_properties(core: &DocumentCore) -> String {
    let title = if core.file_name.is_empty() {
        "rhwp 문서"
    } else {
        core.file_name.as_str()
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\" xmlns:dcterms=\"{}\" xmlns:dcmitype=\"{}\" xmlns:xsi=\"{}\">\
         <dc:title>{}</dc:title>\
         <dc:creator>rhwp</dc:creator>\
         <cp:lastModifiedBy>rhwp</cp:lastModifiedBy>\
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
     <Application>rhwp</Application><DocSecurity>0</DocSecurity><ScaleCrop>false</ScaleCrop>\
     <SharedDoc>false</SharedDoc><HyperlinksChanged>false</HyperlinksChanged><AppVersion>0.1</AppVersion>\
     </Properties>"
        .to_string()
}

fn hwpunit_to_twips(value: u32) -> u32 {
    value / 5
}

fn escape_html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(text: &str) -> String {
    escape_html_text(text).replace('"', "&quot;")
}

fn escape_xml_text(text: &str) -> String {
    escape_html_attr(text).replace('\'', "&apos;")
}
