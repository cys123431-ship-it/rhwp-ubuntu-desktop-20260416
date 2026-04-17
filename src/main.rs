use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::Serialize;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--help") | Some("-h") => print_help(),
        Some("--version") | Some("-V") => println!("rhwp v{}", rhwp::version()),
        Some("export-svg") => export_svg(&args[2..]),
        Some("export-pdf") => export_pdf(&args[2..]),
        Some("info") => show_info(&args[2..]),
        Some("dump") => dump_controls(&args[2..]),
        Some("dump-pages") => dump_pages(&args[2..]),
        Some("diag") => diag_document(&args[2..]),
        Some("convert") => convert_hwp(&args[2..]),
        Some("convert-format") => convert_format(&args[2..]),
        Some("compat-generate-fixtures") => compat_generate_fixtures(&args[2..]),
        Some("dump-records") => dump_raw_records(&args[2..]),
        Some("test-shape") => test_shape_roundtrip(&args[2..]),
        Some("test-caption") => test_caption(&args[2..]),
        Some("gen-table") => gen_table(&args[2..]),
        Some("test-field") => test_field_roundtrip(&args[2..]),
        Some("ir-diff") => ir_diff(&args[2..]),
        Some("compat-report") => compat_report(&args[2..]),
        Some("compat-corpus") => compat_corpus(&args[2..]),
        Some("thumbnail") => extract_thumbnail(&args[2..]),
        _ => {
            println!("rhwp v{}", rhwp::version());
            println!("사용법: rhwp <명령> [옵션]");
            println!("'rhwp --help'로 자세한 사용법을 확인하세요.");
        }
    }
}

fn print_help() {
    println!("rhwp v{} - HWP 파일 뷰어", rhwp::version());
    println!();
    println!("사용법: rhwp <명령> [옵션]");
    println!();
    println!("명령:");
    println!("  export-svg <파일.hwp> [옵션]");
    println!("      HWP 파일을 SVG로 내보내기");
    println!();
    println!("      -o, --output <폴더>     출력 폴더 (기본: output/)");
    println!("      -p, --page <번호>       특정 페이지만 내보내기 (0부터 시작)");
    println!("      --show-para-marks       문단부호(↵/↓) 표시");
    println!("      --show-control-codes    조판부호 보이기 (문단부호 + 개체 마커 등)");
    println!("      --debug-overlay         디버그 오버레이 (문단/표 경계 + 인덱스 라벨)");
    println!("      --font-style            @font-face local() 참조 삽입 (폰트 데이터 미포함)");
    println!("      --embed-fonts           폰트 서브셋 임베딩 (사용 글자만 base64)");
    println!("      --embed-fonts=full      폰트 전체 임베딩 (base64)");
    println!("      --font-path <경로>      폰트 파일 탐색 경로 (여러 번 지정 가능)");
    println!();
    println!("  info <파일.hwp>");
    println!("      HWP 파일 정보 표시");
    println!();
    println!("  dump <파일.hwp> [--section <번호>] [--para <번호>]");
    println!("      문서 조판부호 구조 덤프 (디버깅용)");
    println!();
    println!("  dump-pages <파일.hwp> [-p <번호>]");
    println!("      페이지네이션 결과 덤프 (페이지별 문단/표 배치 목록)");
    println!();
    println!("  diag <파일.hwp>");
    println!("      문서 구조 진단 (번호/글머리표/개요 분석)");
    println!();
    println!("  convert <입력.hwp> <출력.hwp>");
    println!("      배포용(읽기전용) HWP를 편집 가능한 HWP로 변환");
    println!();
    println!("  convert-format <입력.hwp|입력.hwpx> <출력.hwp|출력.hwpx>");
    println!("      Save the parsed document to the format implied by the output extension");
    println!();
    println!("  ir-diff <파일A.hwpx> <파일B.hwp> [-s <구역>] [-p <문단>]");
    println!("      두 파일의 IR(중간표현) 비교 (HWPX↔HWP 불일치 검출)");
    println!();
    println!("  compat-report [--json] <file.hwp|file.hwpx>");
    println!("      Print structured compatibility and font substitution diagnostics");
    println!();
    println!("  compat-corpus [--json] [--emit-reports <dir>] <manifest.tsv>");
    println!("      Validate a corpus manifest with parse/save/reparse checks");
    println!();
    println!("  compat-generate-fixtures [output-dir]");
    println!("      Generate synthetic phase-1 and wave-2 HWPX fixtures for corpus validation");
    println!();
    println!("  thumbnail <파일.hwp> [옵션]");
    println!("      HWP 파일에서 썸네일(PrvImage) 추출");
    println!();
    println!("      -o, --output <파일>       출력 파일 경로 (기본: 입력명_thumb.png)");
    println!("      --base64                  base64 문자열을 stdout에 출력");
    println!("      --data-uri                data:image/... URI 형식으로 stdout에 출력");
    println!();
    println!("옵션:");
    println!("  -h, --help      도움말 표시");
    println!("  -V, --version   버전 표시");
}

fn export_svg(args: &[String]) {
    if args.is_empty() {
        eprintln!("오류: HWP 파일 경로를 지정해주세요.");
        eprintln!("사용법: rhwp export-svg <파일.hwp> [옵션] (rhwp --help 참조)");
        return;
    }

    let file_path = &args[0];
    let mut output_dir = "output".to_string();
    let mut target_page: Option<u32> = None;
    let mut show_para_marks = false;
    let mut show_control_codes = false;
    let mut debug_overlay = false;
    let mut font_embed_mode = rhwp::renderer::svg::FontEmbedMode::None;
    let mut font_paths: Vec<std::path::PathBuf> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("오류: --output 뒤에 폴더 경로가 필요합니다.");
                    return;
                }
            }
            "--page" | "-p" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u32>() {
                        Ok(n) => target_page = Some(n),
                        Err(_) => {
                            eprintln!("오류: 페이지 번호가 올바르지 않습니다.");
                            return;
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("오류: --page 뒤에 페이지 번호가 필요합니다.");
                    return;
                }
            }
            "--show-para-marks" => {
                show_para_marks = true;
                i += 1;
            }
            "--show-control-codes" => {
                show_control_codes = true;
                i += 1;
            }
            "--debug-overlay" => {
                debug_overlay = true;
                i += 1;
            }
            "--font-style" => {
                font_embed_mode = rhwp::renderer::svg::FontEmbedMode::Style;
                i += 1;
            }
            "--embed-fonts" => {
                font_embed_mode = rhwp::renderer::svg::FontEmbedMode::Subset;
                i += 1;
            }
            "--embed-fonts=full" => {
                font_embed_mode = rhwp::renderer::svg::FontEmbedMode::Full;
                i += 1;
            }
            "--font-path" => {
                if i + 1 < args.len() {
                    font_paths.push(std::path::PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("오류: --font-path 뒤에 경로가 필요합니다.");
                    return;
                }
            }
            _ => {
                eprintln!("알 수 없는 옵션: {}", args[i]);
                i += 1;
            }
        }
    }

    // 파일 읽기
    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    // 문서 로드
    let mut doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    if show_para_marks {
        doc.set_show_paragraph_marks(true);
    }
    if show_control_codes {
        doc.set_show_control_codes(true);
    }
    if debug_overlay {
        doc.set_debug_overlay(true);
    }

    let page_count = doc.page_count();
    println!("문서 로드 완료: {} ({}페이지)", file_path, page_count);

    // 출력 폴더 생성
    let output_path = Path::new(&output_dir);
    if !output_path.exists() {
        if let Err(e) = fs::create_dir_all(output_path) {
            eprintln!("오류: 출력 폴더를 생성할 수 없습니다 - {}: {}", output_dir, e);
            return;
        }
    }

    // 페이지 범위 결정
    let pages: Vec<u32> = match target_page {
        Some(p) => {
            if p >= page_count {
                eprintln!("오류: 페이지 번호가 범위를 벗어났습니다 (0~{})", page_count - 1);
                return;
            }
            vec![p]
        }
        None => (0..page_count).collect(),
    };

    // SVG 내보내기
    let file_stem = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");

    for page_num in &pages {
        let svg_result = if font_embed_mode != rhwp::renderer::svg::FontEmbedMode::None {
            doc.render_page_svg_with_fonts(*page_num, font_embed_mode, &font_paths)
        } else {
            doc.render_page_svg_native(*page_num)
        };
        match svg_result {
            Ok(svg) => {
                let svg_filename = if page_count == 1 {
                    format!("{}.svg", file_stem)
                } else {
                    format!("{}_{:03}.svg", file_stem, page_num + 1)
                };
                let svg_path = output_path.join(&svg_filename);

                match fs::write(&svg_path, &svg) {
                    Ok(_) => println!("  → {}", svg_path.display()),
                    Err(e) => eprintln!("오류: SVG 저장 실패 - {}: {}", svg_path.display(), e),
                }
            }
            Err(e) => {
                eprintln!("오류: 페이지 {} 렌더링 실패 - {:?}", page_num, e);
            }
        }
    }

    println!("내보내기 완료: {}개 SVG 파일 → {}/", pages.len(), output_dir);
}

fn export_pdf(args: &[String]) {
    if args.is_empty() {
        eprintln!("오류: HWP 파일 경로를 지정해주세요.");
        eprintln!("사용법: rhwp export-pdf <파일.hwp> [-o 출력.pdf] [-p 페이지]");
        return;
    }

    let file_path = &args[0];
    let mut output_file = String::new();
    let mut target_page: Option<u32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_file = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("오류: --output 뒤에 파일 경로가 필요합니다.");
                    return;
                }
            }
            "--page" | "-p" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u32>() {
                        Ok(n) => target_page = Some(n),
                        Err(_) => {
                            eprintln!("오류: 페이지 번호가 올바르지 않습니다.");
                            return;
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("오류: --page 뒤에 페이지 번호가 필요합니다.");
                    return;
                }
            }
            _ => { i += 1; }
        }
    }

    // 기본 출력 파일명
    if output_file.is_empty() {
        let stem = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        output_file = format!("output/{}.pdf", stem);
    }

    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    let mut doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    let page_count = doc.page_count();
    println!("문서 로드 완료: {} ({}페이지)", file_path, page_count);

    // 출력 디렉토리 생성
    if let Some(parent) = Path::new(&output_file).parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }

    // 페이지 범위 결정
    let pages: Vec<u32> = match target_page {
        Some(p) => {
            if p >= page_count {
                eprintln!("오류: 페이지 번호가 범위를 벗어났습니다 (0~{})", page_count - 1);
                return;
            }
            vec![p]
        }
        None => (0..page_count).collect(),
    };

    // SVG 렌더링 → PDF 변환
    let mut svg_pages: Vec<String> = Vec::new();
    for page_num in &pages {
        match doc.render_page_svg(*page_num) {
            Ok(svg) => svg_pages.push(svg),
            Err(e) => {
                eprintln!("오류: 페이지 {} 렌더링 실패 - {:?}", page_num, e);
                return;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use rhwp::renderer::pdf;
        match pdf::svgs_to_pdf(&svg_pages) {
            Ok(pdf_bytes) => {
                match fs::write(&output_file, &pdf_bytes) {
                    Ok(_) => println!("  → {} ({}KB, {}페이지)", output_file, pdf_bytes.len() / 1024, svg_pages.len()),
                    Err(e) => eprintln!("오류: PDF 저장 실패 - {}", e),
                }
            }
            Err(e) => eprintln!("오류: PDF 변환 실패 - {}", e),
        }
    }

    println!("PDF 내보내기 완료");
}

fn show_info(args: &[String]) {
    if args.is_empty() {
        eprintln!("오류: HWP 파일 경로를 지정해주세요.");
        return;
    }

    let file_path = &args[0];

    // 파일 읽기
    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    let file_size = data.len();

    // HWP 파싱
    let doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    let document = doc.document();

    println!("파일: {}", file_path);
    println!("크기: {} bytes", file_size);
    println!(
        "버전: {}.{}.{}.{}",
        document.header.version.major,
        document.header.version.minor,
        document.header.version.build,
        document.header.version.revision,
    );
    println!("압축: {}", if document.header.compressed { "예" } else { "아니오" });
    println!("암호화: {}", if document.header.encrypted { "예" } else { "아니오" });
    println!("배포용: {}", if document.header.distribution { "예" } else { "아니오" });
    println!("구역 수: {}", document.sections.len());
    println!("페이지 수: {}", doc.page_count());

    // 용지 정보
    for (sec_idx, section) in document.sections.iter().enumerate() {
        let page_def = &section.section_def.page_def;
        let orientation = if page_def.landscape { "가로" } else { "세로" };
        println!("구역{} 용지: {}×{} HWPUNIT, 방향={} (여백: 좌{} 우{} 상{} 하{})",
            sec_idx,
            page_def.width, page_def.height, orientation,
            page_def.margin_left, page_def.margin_right,
            page_def.margin_top, page_def.margin_bottom,
        );
        println!("  머리말여백={} 꼬리말여백={} 제본여백={}",
            page_def.margin_header, page_def.margin_footer,
            page_def.margin_gutter);
        if section.section_def.hide_empty_line {
            println!("  빈 줄 감추기: 활성");
        }
    }

    // 폰트 목록
    let lang_names = ["한글", "영어", "한자", "일어", "기타", "기호", "사용자"];
    for (i, fonts) in document.doc_info.font_faces.iter().enumerate() {
        if !fonts.is_empty() {
            let name = if i < lang_names.len() { lang_names[i] } else { "기타" };
            let font_names: Vec<&str> = fonts.iter().map(|f| f.name.as_str()).collect();
            println!("폰트({}): {}", name, font_names.join(", "));
        }
    }

    // 스타일 목록
    if !document.doc_info.styles.is_empty() {
        let style_names: Vec<&str> = document.doc_info.styles.iter().map(|s| s.local_name.as_str()).collect();
        println!("스타일: {}", style_names.join(", "));
    }

    // 문단 통계
    let total_paras: usize = document.sections.iter().map(|s| s.paragraphs.len()).sum();
    println!("총 문단 수: {}", total_paras);

    // BinData 정보
    if !document.doc_info.bin_data_list.is_empty() {
        println!("BinData:");
        for (idx, bd) in document.doc_info.bin_data_list.iter().enumerate() {
            let type_str = match bd.data_type {
                rhwp::model::bin_data::BinDataType::Link => "Link",
                rhwp::model::bin_data::BinDataType::Embedding => "Embedding",
                rhwp::model::bin_data::BinDataType::Storage => "Storage",
            };
            let ext = bd.extension.as_deref().unwrap_or("?");
            // 로드된 데이터 크기 확인
            let loaded_size = document.bin_data_content
                .iter()
                .find(|c| c.id == bd.storage_id)
                .map(|c| c.data.len())
                .unwrap_or(0);
            println!("  [{}] {} (ID: {}, ext: {}, loaded: {} bytes)", idx, type_str, bd.storage_id, ext, loaded_size);
        }
    }

    // 테이블 및 그림 정보
    use rhwp::model::control::Control;
    let mut table_idx = 0;
    let mut picture_idx = 0;

    fn count_pictures(ctrl: &Control, picture_idx: &mut usize, location: &str) {
        match ctrl {
            Control::Picture(pic) => {
                *picture_idx += 1;
                println!(
                    "그림{} [{}]: bin_data_id={}, size={}×{}",
                    *picture_idx, location,
                    pic.image_attr.bin_data_id,
                    pic.common.width, pic.common.height,
                );
            }
            Control::Table(table) => {
                // 표 내부 셀의 문단에서도 그림 검색
                for (cell_idx, cell) in table.cells.iter().enumerate() {
                    for (cp_idx, cp) in cell.paragraphs.iter().enumerate() {
                        for cc in &cp.controls {
                            let loc = format!("{}→셀{}:문단{}", location, cell_idx, cp_idx);
                            count_pictures(cc, picture_idx, &loc);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for (sec_idx, section) in document.sections.iter().enumerate() {
        for (para_idx, para) in section.paragraphs.iter().enumerate() {
            for ctrl in &para.controls {
                let location = format!("구역{}:문단{}", sec_idx, para_idx);
                match ctrl {
                    Control::Table(table) => {
                        table_idx += 1;
                        let page_break_str = match table.page_break {
                            rhwp::model::table::TablePageBreak::None => "나누지 않음",
                            rhwp::model::table::TablePageBreak::CellBreak => "셀 단위 나눔",
                            rhwp::model::table::TablePageBreak::RowBreak => "나눔(행 단위)",
                        };
                        println!(
                            "표{} [{}]: {}행×{}열, 셀 {}개, 쪽나눔={} (attr=0x{:08x}), 제목반복={}",
                            table_idx, location,
                            table.row_count, table.col_count, table.cells.len(),
                            page_break_str, table.raw_table_record_attr, table.repeat_header,
                        );
                        count_pictures(ctrl, &mut picture_idx, &location);
                    }
                    Control::Picture(_) => {
                        count_pictures(ctrl, &mut picture_idx, &location);
                    }
                    Control::Shape(shape) => {
                        use rhwp::model::shape::ShapeObject;
                        let s = shape.as_ref();
                        let shape_type = s.shape_name();
                        let common = s.common();
                        let border_info = match shape.as_ref() {
                            ShapeObject::Rectangle(r) => format!(
                                ", border(color={:#010x}, width={}, attr={:#010x})",
                                r.drawing.border_line.color,
                                r.drawing.border_line.width,
                                r.drawing.border_line.attr,
                            ),
                            ShapeObject::Line(l) => format!(
                                ", border(color={:#010x}, width={}, attr={:#010x})",
                                l.drawing.border_line.color,
                                l.drawing.border_line.width,
                                l.drawing.border_line.attr,
                            ),
                            _ => String::new(),
                        };
                        println!(
                            "도형 [{}]: {}, size={}×{}, treat_as_char={}{}",
                            location, shape_type,
                            common.width, common.height,
                            common.treat_as_char,
                            border_info,
                        );
                        // 그룹 자식 상세 정보
                        if let ShapeObject::Group(g) = shape.as_ref() {
                            for (i, child) in g.children.iter().enumerate() {
                                let ctype = child.shape_name();
                                let cattr = child.shape_attr();
                                let eff_w = (cattr.current_width as f64 * cattr.render_sx) as i32;
                                let eff_h = (cattr.current_height as f64 * cattr.render_sy) as i32;
                                println!("  자식[{}]: {}, orig={}×{}, scale=({:.3},{:.3}), eff={}×{} at ({:.0},{:.0})",
                                    i, ctype,
                                    cattr.current_width, cattr.current_height,
                                    cattr.render_sx, cattr.render_sy,
                                    eff_w, eff_h,
                                    cattr.render_tx, cattr.render_ty);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// HWPUNIT(u32)을 mm로 변환
fn hu_to_mm(hu: u32) -> f64 {
    hu as f64 * 25.4 / 7200.0
}

/// HWPUNIT(i32)을 mm로 변환
fn hu_to_mm_i(hu: i32) -> f64 {
    hu as f64 * 25.4 / 7200.0
}

fn dump_pages(args: &[String]) {
    if args.is_empty() {
        eprintln!("사용법: rhwp dump-pages <파일.hwp> [-p <페이지번호>]");
        return;
    }

    let file_path = &args[0];
    let mut target_page: Option<u32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--page" | "-p" => {
                if i + 1 < args.len() {
                    target_page = args[i + 1].parse().ok();
                    i += 2;
                } else { i += 1; }
            }
            _ => { i += 1; }
        }
    }

    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    let doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    println!("문서 로드: {} ({}페이지)", file_path, doc.page_count());
    print!("{}", doc.dump_page_items(target_page));
}

fn dump_controls(args: &[String]) {
    if args.is_empty() {
        eprintln!("오류: HWP 파일 경로를 지정해주세요.");
        eprintln!("사용법: rhwp dump <파일.hwp> [--section <번호>] [--para <번호>]");
        return;
    }

    let file_path = &args[0];
    let mut filter_section: Option<usize> = None;
    let mut filter_para: Option<usize> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--section" | "-s" => {
                if i + 1 < args.len() {
                    filter_section = args[i + 1].parse().ok();
                    i += 2;
                } else { i += 1; }
            }
            "--para" | "-p" => {
                if i + 1 < args.len() {
                    filter_para = args[i + 1].parse().ok();
                    i += 2;
                } else { i += 1; }
            }
            _ => { i += 1; }
        }
    }

    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    let doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    let document = doc.document();

    // border_fill 상세 덤프 (필터 없을 때 전체, 필터 있을 때 관련 bf만)
    if filter_section.is_none() && filter_para.is_none() {
        for (i, bf) in document.doc_info.border_fills.iter().enumerate() {
            let fill = &bf.fill;
            let solid_info = fill.solid.as_ref().map(|s| format!("bg=#{:06X} pat_type={} pat_color=#{:06X}", s.background_color, s.pattern_type, s.pattern_color)).unwrap_or_default();
            let grad_info = if fill.gradient.is_some() { " gradient" } else { "" };
            let img_info = fill.image.as_ref().map(|img| format!(" image(bin_id={}, mode={:?})", img.bin_data_id, img.fill_mode)).unwrap_or_default();
            println!("  border_fill[{}] fill_type={:?} {}{}{}", i, fill.fill_type, solid_info, grad_info, img_info);
        }
    }

    use rhwp::model::control::Control;
    use rhwp::model::shape::{ShapeObject, VertRelTo, HorzRelTo, TextWrap};
    use rhwp::model::paragraph::ColumnBreakType;

    let vert_str = |v: &VertRelTo| -> &str {
        match v {
            VertRelTo::Paper => "용지",
            VertRelTo::Page => "쪽",
            VertRelTo::Para => "문단",
        }
    };
    let horz_str = |h: &HorzRelTo| -> &str {
        match h {
            HorzRelTo::Paper => "용지",
            HorzRelTo::Page => "쪽",
            HorzRelTo::Column => "단",
            HorzRelTo::Para => "문단",
        }
    };
    let wrap_str = |w: &TextWrap| -> &str {
        match w {
            TextWrap::Square => "어울림",
            TextWrap::Tight => "자리차지",
            TextWrap::Through => "글뒤로",
            TextWrap::TopAndBottom => "위아래",
            TextWrap::BehindText => "글뒤로",
            TextWrap::InFrontOfText => "글앞으로",
        }
    };
    let break_str = |b: &ColumnBreakType| -> &str {
        match b {
            ColumnBreakType::None => "",
            ColumnBreakType::Section => "[구역나누기]",
            ColumnBreakType::MultiColumn => "[다단나누기]",
            ColumnBreakType::Page => "[쪽나누기]",
            ColumnBreakType::Column => "[단나누기]",
        }
    };

    // 도형 공통 속성 출력 헬퍼
    let dump_common = |c: &rhwp::model::shape::CommonObjAttr, indent: &str| {
        println!("{}  크기: {:.1}mm × {:.1}mm ({}×{} HU)",
            indent, hu_to_mm(c.width), hu_to_mm(c.height), c.width, c.height);
        println!("{}  위치: 가로={} 오프셋={:.1}mm({}), 세로={} 오프셋={:.1}mm({})",
            indent, horz_str(&c.horz_rel_to),
            hu_to_mm(c.horizontal_offset), c.horizontal_offset,
            vert_str(&c.vert_rel_to),
            hu_to_mm(c.vertical_offset), c.vertical_offset);
        println!("{}  배치: {}, 글자처럼={}, z={}",
            indent, wrap_str(&c.text_wrap), c.treat_as_char, c.z_order);
    };

    // 도형 요소 속성 출력 헬퍼
    let dump_shape_attr = |sa: &rhwp::model::shape::ShapeComponentAttr, indent: &str| {
        let eff_w = (sa.current_width as f64 * sa.render_sx) as u32;
        let eff_h = (sa.current_height as f64 * sa.render_sy) as u32;
        println!("{}  요소: orig={}×{}, curr={}×{}, M=[{:.3},{:.3},{:.0}; {:.3},{:.3},{:.0}], offset=({},{}), eff={:.1}mm×{:.1}mm",
            indent, sa.original_width, sa.original_height,
            sa.current_width, sa.current_height,
            sa.render_sx, sa.render_b, sa.render_tx,
            sa.render_c, sa.render_sy, sa.render_ty,
            sa.offset_x, sa.offset_y,
            hu_to_mm(eff_w), hu_to_mm(eff_h));
        if sa.horz_flip || sa.vert_flip || sa.rotation_angle != 0 {
            println!("{}  변환: 뒤집기=({},{}), 회전={}",
                indent, sa.horz_flip, sa.vert_flip, sa.rotation_angle);
        }
    };

    // 재귀적 도형 덤프
    fn dump_shape(
        shape: &ShapeObject, indent: &str,
        dump_common_fn: &dyn Fn(&rhwp::model::shape::CommonObjAttr, &str),
        dump_sa_fn: &dyn Fn(&rhwp::model::shape::ShapeComponentAttr, &str),
    ) {
        match shape {
            ShapeObject::Line(s) => {
                println!("{}[직선] start=({},{}) end=({},{})",
                    indent, s.start.x, s.start.y, s.end.x, s.end.y);
                println!("{}  선: color={:#010x}, width={}, style={:#06x}",
                    indent, s.drawing.border_line.color, s.drawing.border_line.width, s.drawing.border_line.attr);
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
            }
            ShapeObject::Rectangle(s) => {
                println!("{}[사각형] round={}%", indent, s.round_rate);
                println!("{}  선: color={:#010x}, width={}, style={:#06x}",
                    indent, s.drawing.border_line.color, s.drawing.border_line.width, s.drawing.border_line.attr);
                println!("{}  채우기: {:?}{}", indent, s.drawing.fill.fill_type,
                    if let Some(ref img) = s.drawing.fill.image { format!(", image=bin_data_id={}, mode={:?}", img.bin_data_id, img.fill_mode) } else { String::new() });
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
                if let Some(tb) = &s.drawing.text_box {
                    println!("{}  글상자: list_attr={:#010x}, margins=({},{},{},{}), max_width={}, paras={}",
                        indent, tb.list_attr, tb.margin_left, tb.margin_right, tb.margin_top, tb.margin_bottom,
                        tb.max_width, tb.paragraphs.len());
                    for (tpi, tp) in tb.paragraphs.iter().enumerate() {
                        let text_preview = if tp.text.is_empty() {
                            "(빈)".to_string()
                        } else if tp.text.chars().count() > 60 {
                            let end = tp.text.char_indices().nth(60).map(|(i,_)|i).unwrap_or(tp.text.len());
                            format!("\"{}...\"", &tp.text[..end])
                        } else {
                            format!("\"{}\"", tp.text)
                        };
                        println!("{}    p[{}]: ps_id={}, cc={}, text={}, ls_count={}, ctrls={}",
                            indent, tpi, tp.para_shape_id, tp.char_count, text_preview,
                            tp.line_segs.len(), tp.controls.len());
                        for (li, ls) in tp.line_segs.iter().enumerate() {
                            println!("{}      ls[{}]: vpos={}, lh={}, th={}, bl={}, cs={}, sw={}",
                                indent, li, ls.vertical_pos, ls.line_height, ls.text_height,
                                ls.baseline_distance, ls.column_start, ls.segment_width);
                        }
                    }
                }
            }
            ShapeObject::Ellipse(s) => {
                println!("{}[타원]", indent);
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
            }
            ShapeObject::Arc(s) => {
                println!("{}[호]", indent);
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
            }
            ShapeObject::Polygon(s) => {
                println!("{}[다각형] points={}", indent, s.points.len());
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
                // 좌표 범위 출력
                if !s.points.is_empty() {
                    let min_x = s.points.iter().map(|p| p.x).min().unwrap();
                    let max_x = s.points.iter().map(|p| p.x).max().unwrap();
                    let min_y = s.points.iter().map(|p| p.y).min().unwrap();
                    let max_y = s.points.iter().map(|p| p.y).max().unwrap();
                    println!("{}  좌표범위: x=[{},{}], y=[{},{}]", indent, min_x, max_x, min_y, max_y);
                }
            }
            ShapeObject::Curve(s) => {
                println!("{}[곡선] points={}", indent, s.points.len());
                dump_common_fn(&s.common, indent);
                dump_sa_fn(&s.drawing.shape_attr, indent);
            }
            ShapeObject::Group(g) => {
                println!("{}[묶음] children={}", indent, g.children.len());
                dump_common_fn(&g.common, indent);
                dump_sa_fn(&g.shape_attr, indent);
                let child_indent = format!("{}  ", indent);
                for (ci, child) in g.children.iter().enumerate() {
                    print!("{}child[{}] ", child_indent, ci);
                    dump_shape(child, &child_indent, dump_common_fn, dump_sa_fn);
                }
            }
            ShapeObject::Picture(p) => {
                println!("{}[그림] bin_data_id={}", indent, p.image_attr.bin_data_id);
                dump_common_fn(&p.common, indent);
                dump_sa_fn(&p.shape_attr, indent);
            }
        }
    }

    for (sec_idx, section) in document.sections.iter().enumerate() {
        if let Some(fs) = filter_section {
            if sec_idx != fs { continue; }
        }

        let pd = &section.section_def.page_def;
        println!("=== 구역 {} ===", sec_idx);
        println!("  용지: {:.1}mm × {:.1}mm ({}×{} HU), {}",
            hu_to_mm(pd.width), hu_to_mm(pd.height), pd.width, pd.height,
            if pd.landscape { "가로" } else { "세로" });
        println!("  여백: 좌={:.1} 우={:.1} 상={:.1} 하={:.1} 머리말={:.1} 꼬리말={:.1} mm",
            hu_to_mm(pd.margin_left), hu_to_mm(pd.margin_right),
            hu_to_mm(pd.margin_top), hu_to_mm(pd.margin_bottom),
            hu_to_mm(pd.margin_header), hu_to_mm(pd.margin_footer));

        // 바탕쪽 정보
        if !section.section_def.master_pages.is_empty() {
            println!("  바탕쪽: {}개", section.section_def.master_pages.len());
            for (mi, mp) in section.section_def.master_pages.iter().enumerate() {
                println!("    [{}] {:?}, 문단 {}개, 영역 {}×{} HU, is_ext={}, overlap={}, ext_flags=0x{:04X}, text_ref={}, num_ref={}",
                    mi, mp.apply_to, mp.paragraphs.len(), mp.text_width, mp.text_height,
                    mp.is_extension, mp.overlap, mp.ext_flags, mp.text_ref, mp.num_ref);
                for (pi, para) in mp.paragraphs.iter().enumerate() {
                    println!("      p[{}]: cc={}, text=\"{}\"", pi, para.controls.len(),
                        if para.text.is_empty() { "(빈 문단)".to_string() } else { para.text.chars().take(30).collect::<String>() });
                    for (ci, ctrl) in para.controls.iter().enumerate() {
                        let ctrl_name = match ctrl {
                            Control::Table(t) => {
                                let cell_texts: Vec<String> = t.cells.iter().take(3)
                                    .map(|c| {
                                        c.paragraphs.iter()
                                            .map(|p| p.text.chars().take(20).collect::<String>())
                                            .collect::<Vec<_>>().join("|")
                                    })
                                    .collect();
                                format!("표({}x{}, tac={}, wrap={:?}, vert={:?}/{}, horz={:?}/{}, size={}x{}, cells=[{}])",
                                    t.row_count, t.col_count, t.common.treat_as_char,
                                    t.common.text_wrap, t.common.vert_rel_to, t.common.vertical_offset,
                                    t.common.horz_rel_to, t.common.horizontal_offset,
                                    t.common.width, t.common.height,
                                    cell_texts.join("; "))
                            },
                            Control::Shape(s) => {
                                let mut desc = format!("도형(ctrl_id=0x{:08X}, w={}, h={}, attr=0x{:08X}, wc={:?}, hc={:?})",
                                    s.common().ctrl_id, s.common().width, s.common().height,
                                    s.common().attr, s.common().width_criterion, s.common().height_criterion);
                                // TextBox 내용 출력
                                if let Some(tb) = s.drawing().and_then(|d| d.text_box.as_ref()) {
                                    desc += &format!(" 글상자({}문단)", tb.paragraphs.len());
                                    for (tpi, tp) in tb.paragraphs.iter().enumerate() {
                                        let tp_text: String = tp.text.chars().take(20).collect();
                                        desc += &format!("\n          tb_p[{}]: cc={} text=\"{}\"", tpi, tp.controls.len(), tp_text);
                                        for (tci, tc) in tp.controls.iter().enumerate() {
                                            let tc_name = match tc {
                                                Control::AutoNumber(an) => format!("자동번호({:?})", an.number_type),
                                                _ => format!("{:?}", std::mem::discriminant(tc)),
                                            };
                                            desc += &format!("\n            tb_ctrl[{}]: {}", tci, tc_name);
                                        }
                                    }
                                }
                                desc
                            }
                            Control::Picture(p) => format!("그림(bin_id={}, w={}, h={}, tac={})", p.image_attr.bin_data_id, p.common.width, p.common.height, p.common.treat_as_char),
                            Control::Header(_) => "머리말".to_string(),
                            Control::Footer(_) => "꼬리말".to_string(),
                            _ => format!("{:?}", std::mem::discriminant(ctrl)),
                        };
                        println!("        ctrl[{}]: {}", ci, ctrl_name);
                    }
                }
            }
        }
        if section.section_def.hide_master_page {
            println!("  바탕쪽 감추기: true");
        }

        for (para_idx, para) in section.paragraphs.iter().enumerate() {
            if let Some(fp) = filter_para {
                if para_idx != fp { continue; }
            }

            let text_preview = if para.text.is_empty() {
                "(빈 문단)".to_string()
            } else {
                let preview = if para.text.chars().count() > 50 {
                    let end = para.text.char_indices().nth(50).map(|(i,_)|i).unwrap_or(para.text.len());
                    format!("\"{}...\"", &para.text[..end])
                } else {
                    format!("\"{}\"", para.text)
                };
                preview
            };

            let break_info = break_str(&para.column_type);
            println!("\n--- 문단 {}.{} --- cc={}, text_len={}, controls={} {}",
                sec_idx, para_idx, para.char_count, para.text.chars().count(),
                para.controls.len(), break_info);
            println!("  텍스트: {}", text_preview);
            // char_shapes 출력
            if !para.char_shapes.is_empty() {
                let text_chars: Vec<char> = para.text.chars().collect();
                for (ci, cs) in para.char_shapes.iter().enumerate() {
                    let next_pos = para.char_shapes.get(ci + 1).map(|n| n.start_pos).unwrap_or(u32::MAX);
                    let char_at = text_chars.iter().enumerate()
                        .find(|(i, _)| {
                            if *i < para.char_offsets.len() { para.char_offsets[*i] >= cs.start_pos && para.char_offsets[*i] < next_pos }
                            else { false }
                        })
                        .map(|(_, c)| *c);
                    if let Some(chs) = document.doc_info.char_shapes.get(cs.char_shape_id as usize) {
                        let bold = (chs.attr & 0x02) != 0;
                        let spacing = chs.spacings[0]; // 한국어 자간
                        let ratio = chs.ratios[0]; // 한국어 장평
                        println!("  [CS] pos={} id={} bold={} spacing={}% ratio={}% char={:?}",
                            cs.start_pos, cs.char_shape_id, bold, spacing, ratio,
                            char_at.map(|c| c.to_string()).unwrap_or_default());
                    }
                }
            }
            if let Some(ps) = document.doc_info.para_shapes.get(para.para_shape_id as usize) {
                // 문단 모양 기본 정보 (항상 출력)
                println!("  [PS] ps_id={} align={:?} spacing: before={} after={} line={}/{:?}",
                    para.para_shape_id, ps.alignment,
                    ps.spacing_before, ps.spacing_after,
                    ps.line_spacing, ps.line_spacing_type);
                println!("       margins: left={} right={} indent={} border_fill_id={}",
                    ps.margin_left, ps.margin_right, ps.indent, ps.border_fill_id);
                if ps.border_fill_id > 0 {
                    println!("       border_spacing: left={} right={} top={} bottom={}",
                        ps.border_spacing[0], ps.border_spacing[1],
                        ps.border_spacing[2], ps.border_spacing[3]);
                }
                if ps.head_type != rhwp::model::style::HeadType::None {
                    println!("       head={:?} level={} num_id={} attr1=0x{:08X} attr2=0x{:08X} raw_extra={:?}",
                        ps.head_type, ps.para_level, ps.numbering_id, ps.attr1, ps.attr2,
                        &para.raw_header_extra);
                }
                {
                    let td_id = ps.tab_def_id;
                    if let Some(td) = document.doc_info.tab_defs.get(td_id as usize) {
                        let tabs_str: Vec<String> = td.tabs.iter().enumerate()
                            .map(|(i, t)| format!("tab[{}] pos={} ({:.1}mm) type={} fill={}",
                                i, t.position, hu_to_mm(t.position), t.tab_type, t.fill_type))
                            .collect();
                        println!("       tab_def_id={} auto_left={} auto_right={} tabs=[{}]",
                            td_id, td.auto_tab_left, td.auto_tab_right,
                            if tabs_str.is_empty() { "(없음)".to_string() } else { tabs_str.join(", ") });
                    } else {
                        println!("       tab_def_id={} (정의 없음)", td_id);
                    }
                }
            }
            // line_segs 출력
            if !para.line_segs.is_empty() {
                for (li, ls) in para.line_segs.iter().enumerate() {
                    println!("  ls[{}]: ts={}, vpos={}, lh={}, th={}, bl={}, ls={}, cs={}, sw={}, tag=0x{:08X}",
                        li, ls.text_start, ls.vertical_pos, ls.line_height, ls.text_height,
                        ls.baseline_distance, ls.line_spacing, ls.column_start, ls.segment_width, ls.tag);
                }
            }

            for (ctrl_idx, ctrl) in para.controls.iter().enumerate() {
                let prefix = format!("  [{}] ", ctrl_idx);
                match ctrl {
                    Control::ColumnDef(cd) => {
                        let ct = match cd.column_type {
                            rhwp::model::page::ColumnType::Normal => "일반",
                            rhwp::model::page::ColumnType::Distribute => "배분",
                            rhwp::model::page::ColumnType::Parallel => "병행",
                        };
                        println!("{}단정의: {}단, 유형={}, 간격={:.1}mm({}), 같은너비={}",
                            prefix, cd.column_count, ct,
                            hu_to_mm_i(cd.spacing as i32), cd.spacing, cd.same_width);
                        if !cd.widths.is_empty() {
                            // 비례값일 경우 body_width 기준으로 실제 mm 변환
                            let body_width_hu = {
                                let spd = &section.section_def.page_def;
                                let (pw, _) = if spd.landscape { (spd.height, spd.width) } else { (spd.width, spd.height) };
                                (pw - spd.margin_left - spd.margin_right - spd.margin_gutter) as f64
                            };
                            let total: f64 = if cd.proportional_widths {
                                cd.widths.iter().chain(cd.gaps.iter())
                                    .map(|&v| (v as u16) as f64).sum()
                            } else {
                                1.0
                            };
                            let cols_info: Vec<String> = cd.widths.iter().enumerate()
                                .map(|(i, w)| {
                                    let gap = cd.gaps.get(i).copied().unwrap_or(0);
                                    if cd.proportional_widths && total > 0.0 {
                                        let w_hu = (*w as u16) as f64 / total * body_width_hu;
                                        let g_hu = (gap as u16) as f64 / total * body_width_hu;
                                        format!("너비={:.1}mm 간격={:.1}mm", w_hu * 25.4 / 7200.0, g_hu * 25.4 / 7200.0)
                                    } else {
                                        format!("너비={:.1}mm 간격={:.1}mm", hu_to_mm_i(*w as i32), hu_to_mm_i(gap as i32))
                                    }
                                })
                                .collect();
                            println!("{}  단별: [{}]", prefix, cols_info.join(", "));
                        }
                        if cd.separator_type > 0 {
                            println!("{}  구분선: type={}, width={}, color={:#010x}",
                                prefix, cd.separator_type, cd.separator_width, cd.separator_color);
                        }
                    }
                    Control::SectionDef(sd) => {
                        let spd = &sd.page_def;
                        println!("{}구역정의: 용지 {:.1}×{:.1}mm, {}, flags=0x{:08X}",
                            prefix,
                            hu_to_mm(spd.width), hu_to_mm(spd.height),
                            if spd.landscape { "가로" } else { "세로" }, sd.flags);
                        if sd.hide_header || sd.hide_footer || sd.hide_master_page {
                            println!("{}  감추기: 머리말={} 꼬리말={} 바탕쪽={}",
                                prefix, sd.hide_header, sd.hide_footer, sd.hide_master_page);
                        }
                    }
                    Control::Table(table) => {
                        println!("{}표: {}행×{}열, 셀={}, 쪽나눔={:?} (attr=0x{:08x}), padding=({},{},{},{}), cs={}",
                            prefix, table.row_count, table.col_count,
                            table.cells.len(), table.page_break, table.raw_table_record_attr,
                            table.padding.left, table.padding.right, table.padding.top, table.padding.bottom,
                            table.cell_spacing);
                        if !table.zones.is_empty() {
                            for (zi, z) in table.zones.iter().enumerate() {
                                println!("{}  zone[{}] row={}..{} col={}..{} bf={}",
                                    prefix, zi, z.start_row, z.end_row, z.start_col, z.end_col, z.border_fill_id);
                            }
                        }
                        {
                            let c = &table.common;
                            println!("{}  [common] treat_as_char={}, wrap={}, vert={}({}={:.1}mm), horz={}({}={:.1}mm)",
                                prefix, c.treat_as_char, wrap_str(&c.text_wrap),
                                vert_str(&c.vert_rel_to), c.vertical_offset, hu_to_mm(c.vertical_offset),
                                horz_str(&c.horz_rel_to), c.horizontal_offset, hu_to_mm(c.horizontal_offset));
                            println!("{}  [common] size={}×{}({:.1}×{:.1}mm), valign={:?}, halign={:?}",
                                prefix, c.width, c.height, hu_to_mm(c.width), hu_to_mm(c.height),
                                c.vert_align, c.horz_align);
                            println!("{}  [outer_margin] left={:.1}mm({}) right={:.1}mm({}) top={:.1}mm({}) bottom={:.1}mm({})",
                                prefix,
                                hu_to_mm_i(table.outer_margin_left as i32), table.outer_margin_left,
                                hu_to_mm_i(table.outer_margin_right as i32), table.outer_margin_right,
                                hu_to_mm_i(table.outer_margin_top as i32), table.outer_margin_top,
                                hu_to_mm_i(table.outer_margin_bottom as i32), table.outer_margin_bottom);
                            if table.raw_ctrl_data.len() >= 20 {
                                println!("{}  [raw] {:02X?}", prefix, &table.raw_ctrl_data[..20.min(table.raw_ctrl_data.len())]);
                            }
                        }
                        // 셀 상세 출력
                        fn dump_table_deep(table: &rhwp::model::table::Table, indent: &str, depth: usize) {
                            for (ci, cell) in table.cells.iter().enumerate() {
                                let text_preview: String = cell.paragraphs.iter()
                                    .map(|p| p.text.chars().take(30).collect::<String>())
                                    .collect::<Vec<_>>().join("|");
                                println!("{}셀[{}] r={},c={} rs={},cs={} h={} w={} pad=({},{},{},{}) aim={} bf={} paras={} text=\"{}\"",
                                    indent, ci, cell.row, cell.col, cell.row_span, cell.col_span,
                                    cell.height, cell.width,
                                    cell.padding.left, cell.padding.right, cell.padding.top, cell.padding.bottom,
                                    cell.apply_inner_margin,
                                    cell.border_fill_id, cell.paragraphs.len(), text_preview);
                                if let Some(ref fname) = cell.field_name {
                                    println!("{}  field=\"{}\"", indent, fname);
                                }
                                // 셀 내 LINE_SEG 상세
                                for (pi, cp) in cell.paragraphs.iter().enumerate() {
                                    if !cp.line_segs.is_empty() || !cp.controls.is_empty() {
                                        let ls_info: Vec<String> = cp.line_segs.iter().enumerate()
                                            .map(|(li, ls)| format!("ls[{}] vpos={} lh={} ls={}", li, ls.vertical_pos, ls.line_height, ls.line_spacing))
                                            .collect();
                                        println!("{}  p[{}] ps_id={} ctrls={} text_len={} {}",
                                            indent, pi, cp.para_shape_id, cp.controls.len(),
                                            cp.text.len(), ls_info.join(", "));
                                    }
                                    // 셀 내부 컨트롤 상세
                                    for (ci, ctrl) in cp.controls.iter().enumerate() {
                                        match ctrl {
                                            Control::Picture(p) => {
                                                println!("{}    ctrl[{}] 그림: bin_id={}, w={} h={} ({:.1}×{:.1}mm), tac={}, wrap={:?}, vert={:?}(off={}), horz={:?}(off={})",
                                                    indent, ci, p.image_attr.bin_data_id,
                                                    p.common.width, p.common.height,
                                                    p.common.width as f64 / 7200.0 * 25.4,
                                                    p.common.height as f64 / 7200.0 * 25.4,
                                                    p.common.treat_as_char,
                                                    p.common.text_wrap, p.common.vert_rel_to, p.common.vertical_offset,
                                                    p.common.horz_rel_to, p.common.horizontal_offset);
                                            }
                                            Control::Shape(s) => {
                                                println!("{}    ctrl[{}] 도형: tac={}, wrap={:?}",
                                                    indent, ci, s.common().treat_as_char, s.common().text_wrap);
                                            }
                                            _ => {}
                                        }
                                    }
                                    // 내부 표 재귀
                                    if depth < 3 {
                                        for ctrl in &cp.controls {
                                            if let Control::Table(inner) = ctrl {
                                                println!("{}  p[{}] 내부표: {}행×{}열, 셀={}, cs={}, pad=({},{},{},{})",
                                                    indent, pi, inner.row_count, inner.col_count,
                                                    inner.cells.len(), inner.cell_spacing,
                                                    inner.padding.left, inner.padding.right, inner.padding.top, inner.padding.bottom);
                                                let next_indent = format!("{}    ", indent);
                                                dump_table_deep(inner, &next_indent, depth + 1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        dump_table_deep(table, &format!("{}  ", prefix), 0);
                    }
                    Control::Shape(shape) => {
                        print!("{}", prefix);
                        dump_shape(shape, "  ", &dump_common, &dump_shape_attr);
                    }
                    Control::Picture(pic) => {
                        let sa = &pic.shape_attr;
                        println!("{}그림: bin_id={}, common={}×{} ({:.1}×{:.1}mm), orig={}×{} ({:.1}×{:.1}mm), cur={}×{} ({:.1}×{:.1}mm), tac={}",
                            prefix, pic.image_attr.bin_data_id, pic.common.width, pic.common.height,
                            pic.common.width as f64 / 7200.0 * 25.4, pic.common.height as f64 / 7200.0 * 25.4,
                            sa.original_width, sa.original_height,
                            sa.original_width as f64 / 7200.0 * 25.4, sa.original_height as f64 / 7200.0 * 25.4,
                            sa.current_width, sa.current_height,
                            sa.current_width as f64 / 7200.0 * 25.4, sa.current_height as f64 / 7200.0 * 25.4,
                            pic.common.treat_as_char);
                        println!("{}  border_x={:?} border_y={:?} border_color=#{:06X} border_width={} ({:.2}mm) border_attr={:?}",
                            prefix, pic.border_x, pic.border_y,
                            pic.border_color, pic.border_width, pic.border_width as f64 / 7200.0 * 25.4,
                            pic.border_attr);
                        println!("{}  crop=({},{},{},{}) crop_mm=({:.2},{:.2},{:.2},{:.2})",
                            prefix, pic.crop.left, pic.crop.top, pic.crop.right, pic.crop.bottom,
                            pic.crop.left as f64 / 7200.0 * 25.4, pic.crop.top as f64 / 7200.0 * 25.4,
                            pic.crop.right as f64 / 7200.0 * 25.4, pic.crop.bottom as f64 / 7200.0 * 25.4);
                        dump_common(&pic.common, "  ");
                    }
                    Control::Header(h) => {
                        let text: String = h.paragraphs.iter()
                            .filter(|p| !p.text.is_empty())
                            .map(|p| p.text.clone())
                            .collect::<Vec<_>>()
                            .join(" ");
                        println!("{}머리말({:?}): paras={} \"{}\"", prefix, h.apply_to, h.paragraphs.len(), text);
                        for (hpi, hp) in h.paragraphs.iter().enumerate() {
                            if !hp.controls.is_empty() {
                                for (hci, hc) in hp.controls.iter().enumerate() {
                                    let cn = match hc {
                                        Control::AutoNumber(an) => format!("자동번호({:?})", an.number_type),
                                        Control::Shape(s) => {
                                            let c = s.common();
                                            let mut desc = format!("Shape horz={:?}/{} halign={:?} w={} h={}",
                                                c.horz_rel_to, c.horizontal_offset, c.horz_align, c.width, c.height);
                                            if let Some(tb) = s.drawing().and_then(|d| d.text_box.as_ref()) {
                                                let text: String = tb.paragraphs.iter()
                                                    .flat_map(|p| p.text.chars().take(20))
                                                    .collect();
                                                desc += &format!(" text={:?}", text);
                                            }
                                            desc
                                        }
                                        Control::Table(t) => {
                                            let mut desc = format!("표 {}행×{}열 셀={}", t.row_count, t.col_count, t.cells.len());
                                            for (si, cell) in t.cells.iter().enumerate() {
                                                let cell_text: String = cell.paragraphs.iter()
                                                    .flat_map(|p| p.text.chars().take(20))
                                                    .collect();
                                                desc += &format!("\n{}    셀[{}] text={:?}", prefix, si, cell_text);
                                                for (cpi, cp) in cell.paragraphs.iter().enumerate() {
                                                    for (cci, cc) in cp.controls.iter().enumerate() {
                                                        let ccn = match cc {
                                                            Control::AutoNumber(an) => format!("자동번호({:?})", an.number_type),
                                                            Control::Shape(s) => {
                                            let c = s.common();
                                            let mut d = format!("Shape vert={:?}/{} valign={:?} horz={:?}/{} halign={:?} w={} h={}",
                                                c.vert_rel_to, c.vertical_offset, c.vert_align,
                                                c.horz_rel_to, c.horizontal_offset, c.horz_align, c.width, c.height);
                                            if let Some(tb) = s.drawing().and_then(|dd| dd.text_box.as_ref()) {
                                                for (tpi, tp) in tb.paragraphs.iter().enumerate() {
                                                    let t: String = tp.text.chars().take(30).collect();
                                                    d += &format!(" tb_p[{}] ps_id={} text={:?}", tpi, tp.para_shape_id, t);
                                                }
                                            }
                                            d
                                        }
                                        _ => format!("{:?}", std::mem::discriminant(cc)),
                                                        };
                                                        desc += &format!("\n{}      p[{}]c[{}]: {}", prefix, cpi, cci, ccn);
                                                    }
                                                }
                                            }
                                            desc
                                        }
                                        Control::Picture(pic) => {
                                            let sa = &pic.shape_attr;
                                            format!("그림: bin_id={}, common={}×{} ({:.1}×{:.1}mm), orig={}×{} ({:.1}×{:.1}mm), cur={}×{} ({:.1}×{:.1}mm), tac={}, crop=({},{},{},{}) crop_mm=({:.2},{:.2},{:.2},{:.2})",
                                            pic.image_attr.bin_data_id, pic.common.width, pic.common.height,
                                            pic.common.width as f64 / 7200.0 * 25.4, pic.common.height as f64 / 7200.0 * 25.4,
                                            sa.original_width, sa.original_height,
                                            sa.original_width as f64 / 7200.0 * 25.4, sa.original_height as f64 / 7200.0 * 25.4,
                                            sa.current_width, sa.current_height,
                                            sa.current_width as f64 / 7200.0 * 25.4, sa.current_height as f64 / 7200.0 * 25.4,
                                            pic.common.treat_as_char,
                                            pic.crop.left, pic.crop.top, pic.crop.right, pic.crop.bottom,
                                            pic.crop.left as f64 / 7200.0 * 25.4, pic.crop.top as f64 / 7200.0 * 25.4,
                                            pic.crop.right as f64 / 7200.0 * 25.4, pic.crop.bottom as f64 / 7200.0 * 25.4)
                                        },
                                        _ => format!("{:?}", std::mem::discriminant(hc)),
                                    };
                                    let display = if cn.chars().count() > 30 {
                                        format!("{}...(truncated)", cn.chars().take(30).collect::<String>())
                                    } else {
                                        cn
                                    };
                                    println!("{}  hp[{}] ctrl[{}]: {}", prefix, hpi, hci, display);
                                }
                            }
                        }
                    }
                    Control::Footer(f) => {
                        let text: String = f.paragraphs.iter()
                            .filter(|p| !p.text.is_empty())
                            .map(|p| p.text.clone())
                            .collect::<Vec<_>>()
                            .join(" ");
                        println!("{}꼬리말({:?}): paras={} \"{}\"", prefix, f.apply_to, f.paragraphs.len(), text);
                        for (fpi, fp) in f.paragraphs.iter().enumerate() {
                            if !fp.controls.is_empty() {
                                for (fci, fc) in fp.controls.iter().enumerate() {
                                    let cn = match fc {
                                        Control::Picture(pic) => {
                                            let sa = &pic.shape_attr;
                                            format!("그림: bin_id={}, common={}×{} ({:.1}×{:.1}mm), orig={}×{} ({:.1}×{:.1}mm), cur={}×{} ({:.1}×{:.1}mm), tac={}, crop=({},{},{},{}) crop_mm=({:.2},{:.2},{:.2},{:.2})",
                                            pic.image_attr.bin_data_id, pic.common.width, pic.common.height,
                                            pic.common.width as f64 / 7200.0 * 25.4, pic.common.height as f64 / 7200.0 * 25.4,
                                            sa.original_width, sa.original_height,
                                            sa.original_width as f64 / 7200.0 * 25.4, sa.original_height as f64 / 7200.0 * 25.4,
                                            sa.current_width, sa.current_height,
                                            sa.current_width as f64 / 7200.0 * 25.4, sa.current_height as f64 / 7200.0 * 25.4,
                                            pic.common.treat_as_char,
                                            pic.crop.left, pic.crop.top, pic.crop.right, pic.crop.bottom,
                                            pic.crop.left as f64 / 7200.0 * 25.4, pic.crop.top as f64 / 7200.0 * 25.4,
                                            pic.crop.right as f64 / 7200.0 * 25.4, pic.crop.bottom as f64 / 7200.0 * 25.4)
                                        },
                                        _ => format!("{:?}", std::mem::discriminant(fc)),
                                    };
                                    println!("{}  fp[{}] ctrl[{}]: {}", prefix, fpi, fci, cn);
                                }
                            }
                        }
                    }
                    Control::Footnote(fn_) => {
                        println!("{}각주: paragraphs={}", prefix, fn_.paragraphs.len());
                    }
                    Control::Endnote(en) => {
                        println!("{}미주: paragraphs={}", prefix, en.paragraphs.len());
                    }
                    Control::AutoNumber(an) => {
                        println!("{}자동번호: type={:?}, number={}", prefix, an.number_type, an.number);
                    }
                    Control::NewNumber(nn) => {
                        println!("{}새번호: type={:?}, number={}", prefix, nn.number_type, nn.number);
                    }
                    Control::PageNumberPos(pn) => {
                        println!("{}쪽번호위치: format={}, pos={}", prefix, pn.format, pn.position);
                    }
                    Control::Bookmark(bm) => {
                        println!("{}책갈피: \"{}\"", prefix, bm.name);
                    }
                    Control::Hyperlink(hl) => {
                        println!("{}하이퍼링크: \"{}\"", prefix, hl.url);
                    }
                    Control::Ruby(r) => {
                        println!("{}덧말: \"{}\"", prefix, r.ruby_text);
                    }
                    Control::PageHide(ph) => {
                        println!("{}감추기: header={}, footer={}, master={}, border={}, fill={}, page_num={}",
                            prefix, ph.hide_header, ph.hide_footer, ph.hide_master_page, ph.hide_border, ph.hide_fill, ph.hide_page_num);
                    }
                    Control::HiddenComment(_) => {
                        println!("{}숨은설명", prefix);
                    }
                    Control::Field(f) => {
                        let name = f.field_name().unwrap_or("(이름없음)");
                        println!("{}필드: {:?} name=\"{}\" cmd=\"{}\"", prefix, f.field_type, name, f.command);
                    }
                    Control::CharOverlap(co) => {
                        println!("{}글자겹침: {:?}", prefix, co.chars);
                    }
                    Control::Equation(eq) => {
                        println!("{}수식: script=\"{}\" font_size={} font=\"{}\"",
                            prefix, eq.script, eq.font_size, eq.font_name);
                    }
                    Control::Form(f) => {
                        println!("{}양식개체: {:?} name=\"{}\" caption=\"{}\" {}x{}",
                            prefix, f.form_type, f.name, f.caption, f.width, f.height);
                    }
                    Control::Unknown(u) => {
                        println!("{}알수없음: ctrl_id={:#010x}", prefix, u.ctrl_id);
                    }
                }
            }
        }
    }

    println!("\n=== 완료: {} 구역, {} 문단 ===",
        document.sections.len(),
        document.sections.iter().map(|s| s.paragraphs.len()).sum::<usize>());
}

fn diag_document(args: &[String]) {
    if args.is_empty() {
        eprintln!("오류: HWP 파일 경로를 지정해주세요.");
        eprintln!("사용법: rhwp diag <파일.hwp>");
        return;
    }

    let file_path = &args[0];
    let data = match fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", file_path, e);
            return;
        }
    };

    let doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    let document = doc.document();
    use rhwp::model::style::HeadType;

    // === DocInfo 요약 ===
    println!("=== DocInfo 요약 ===");
    println!("  Numbering: {}개", document.doc_info.numberings.len());
    for (i, num) in document.doc_info.numberings.iter().enumerate() {
        let formats: Vec<String> = num.level_formats.iter()
            .enumerate()
            .filter(|(_, f)| !f.is_empty())
            .map(|(lv, f)| format!("L{}=\"{}\"", lv + 1, f))
            .collect();
        println!("    [{}] start={}, formats: {}", i, num.start_number, formats.join(", "));
    }

    println!("  Bullet: {}개", document.doc_info.bullets.len());
    for (i, bullet) in document.doc_info.bullets.iter().enumerate() {
        println!("    [{}] char='{}' (U+{:04X})", i, bullet.bullet_char, bullet.bullet_char as u32);
    }

    // === ParaShape head_type 분포 ===
    println!("\n=== ParaShape head_type 분포 ===");
    let mut count_none = 0u32;
    let mut count_outline = 0u32;
    let mut count_number = 0u32;
    let mut count_bullet = 0u32;
    for ps in &document.doc_info.para_shapes {
        match ps.head_type {
            HeadType::None => count_none += 1,
            HeadType::Outline => count_outline += 1,
            HeadType::Number => count_number += 1,
            HeadType::Bullet => count_bullet += 1,
        }
    }
    println!("  None: {}개, Outline: {}개, Number: {}개, Bullet: {}개",
        count_none, count_outline, count_number, count_bullet);

    // === SectionDef 개요번호 ===
    println!("\n=== SectionDef 개요번호 ===");
    for (sec_idx, section) in document.sections.iter().enumerate() {
        // SectionDef의 raw_ctrl_extra에서 바이트 14-15 추출 (outline_numbering_id)
        // 현재 outline_numbering_id 필드가 없으므로 파싱 전 상태에서는 raw_ctrl_extra 참조
        // 6단계에서 필드 추가 후 직접 참조로 변경 예정
        let sd = &section.section_def;
        let num_ref = if sd.outline_numbering_id > 0 {
            format!(" → Numbering[{}]", sd.outline_numbering_id - 1)
        } else {
            " (없음)".to_string()
        };
        println!("  구역{}: outline_numbering_id={}{}, flags={:#010x}",
            sec_idx, sd.outline_numbering_id, num_ref, sd.flags);
    }

    // === 비None head_type 문단 ===
    println!("\n=== 비None head_type 문단 ===");
    for (sec_idx, section) in document.sections.iter().enumerate() {
        for (para_idx, para) in section.paragraphs.iter().enumerate() {
            if let Some(ps) = document.doc_info.para_shapes.get(para.para_shape_id as usize) {
                if ps.head_type != HeadType::None {
                    let text_preview: String = para.text.chars().take(40).collect();
                    let text_display = if para.text.chars().count() > 40 {
                        format!("\"{}...\"", text_preview)
                    } else {
                        format!("\"{}\"", text_preview)
                    };
                    println!("  구역{}:문단{} head={:?} level={} num_id={} text={}",
                        sec_idx, para_idx,
                        ps.head_type, ps.para_level, ps.numbering_id,
                        text_display);
                }
            }
        }
    }
}

fn convert_hwp(args: &[String]) {
    if args.len() < 2 {
        eprintln!("오류: 입력 파일과 출력 파일 경로를 지정해주세요.");
        eprintln!("사용법: rhwp convert <입력.hwp> <출력.hwp>");
        return;
    }

    let input_path = &args[0];
    let output_path = &args[1];

    // 입력 파일 읽기
    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다 - {}: {}", input_path, e);
            return;
        }
    };

    // 문서 로드
    let mut doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: HWP 파싱 실패 - {}", e);
            return;
        }
    };

    let was_distribution = doc.document().header.distribution;
    if !was_distribution {
        println!("{}: 이미 편집 가능한 문서입니다.", input_path);
    }

    // 변환
    match doc.convert_to_editable_native() {
        Ok(_) => {
            if was_distribution {
                println!("배포용 → 편집 가능 변환 완료");
            }
        }
        Err(e) => {
            eprintln!("오류: 변환 실패 - {}", e);
            return;
        }
    }

    // 직렬화
    match doc.export_hwp_native() {
        Ok(bytes) => {
            match fs::write(output_path, &bytes) {
                Ok(_) => {
                    println!("저장 완료: {} ({}KB)", output_path, bytes.len() / 1024);
                }
                Err(e) => {
                    eprintln!("오류: 파일 저장 실패 - {}: {}", output_path, e);
                }
            }
        }
        Err(e) => {
            eprintln!("오류: 직렬화 실패 - {}", e);
        }
    }
}

fn convert_format(args: &[String]) {
    if args.len() < 2 {
        eprintln!("usage: rhwp convert-format <input.hwp|input.hwpx> <output.hwp|output.hwpx>");
        std::process::exit(1);
    }

    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let output_ext = output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let data = match fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: failed to read {}: {}", input_path.display(), error);
            std::process::exit(1);
        }
    };

    let core = match rhwp::document_core::DocumentCore::from_bytes(&data) {
        Ok(core) => core,
        Err(error) => {
            eprintln!("error: failed to parse {}: {}", input_path.display(), error);
            std::process::exit(1);
        }
    };

    let bytes = match output_ext.as_str() {
        "hwp" => core.export_hwp_native(),
        "hwpx" => core.export_hwpx_native(),
        _ => {
            eprintln!(
                "error: output extension must be .hwp or .hwpx, got {}",
                output_path.display()
            );
            std::process::exit(1);
        }
    };

    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "error: failed to save {} as {}: {}",
                input_path.display(),
                output_path.display(),
                error
            );
            std::process::exit(1);
        }
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("error: failed to create {}: {}", parent.display(), error);
                std::process::exit(1);
            }
        }
    }

    match fs::write(output_path, &bytes) {
        Ok(_) => println!(
            "saved {} -> {} ({} bytes)",
            input_path.display(),
            output_path.display(),
            bytes.len()
        ),
        Err(error) => {
            eprintln!("error: failed to write {}: {}", output_path.display(), error);
            std::process::exit(1);
        }
    }
}

fn dump_raw_records(args: &[String]) {
    if args.is_empty() {
        eprintln!("사용법: rhwp dump-records <파일.hwp>");
        return;
    }
    let data = match fs::read(&args[0]) {
        Ok(d) => d,
        Err(e) => { eprintln!("오류: {}", e); return; }
    };
    use rhwp::parser::cfb_reader::CfbReader;
    use rhwp::parser::record::Record;
    let mut cfb = match CfbReader::open(&data) {
        Ok(c) => c,
        Err(e) => { eprintln!("오류: {:?}", e); return; }
    };
    // FileHeader에서 압축 여부 확인
    let header = cfb.read_stream_raw("FileHeader").unwrap_or_default();
    let compressed = header.len() >= 40 && (header[36] & 0x01) != 0;
    let section = match cfb.read_body_text_section(0, compressed, false) {
        Ok(s) => s,
        Err(e) => { eprintln!("오류: {:?}", e); return; }
    };
    let records = match Record::read_all(&section) {
        Ok(r) => r,
        Err(e) => { eprintln!("오류: {:?}", e); return; }
    };
    let tag_name = |id: u16| -> &str {
        match id {
            66 => "PARA_HEADER", 67 => "PARA_TEXT", 68 => "PARA_CHAR_SHAPE",
            69 => "PARA_LINE_SEG", 70 => "PARA_RANGE_TAG", 71 => "CTRL_HEADER",
            72 => "LIST_HEADER", 73 => "PAGE_DEF", 74 => "FOOTNOTE_SHAPE",
            75 => "PAGE_BORDER_FILL", 76 => "SHAPE_COMPONENT", 77 => "TABLE",
            78 => "SC_LINE", 79 => "SC_RECT", 80 => "SC_ELLIPSE",
            81 => "SC_ARC", 82 => "SC_POLYGON", 83 => "SC_CURVE",
            85 => "SC_PICTURE", 86 => "SC_CONTAINER", 89 => "CTRL_DATA",
            _ => "?",
        }
    };
    for (i, rec) in records.iter().enumerate() {
        let indent = "  ".repeat(rec.level as usize);
        println!("[{:3}] {}tag={:<3} {:16} lv={} sz={}",
            i, indent, rec.tag_id, tag_name(rec.tag_id), rec.level, rec.data.len());
        // shape 관련 레코드만 hex 덤프
        if matches!(rec.tag_id, 71 | 72 | 76 | 79 | 85 | 89) {
            // 16바이트씩 나눠서 hex 출력
            for chunk in rec.data.chunks(16) {
                let hex: String = chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                println!("       {}  {}", indent, hex);
            }
        }
    }
}

fn test_shape_roundtrip(args: &[String]) {
    let input = if args.is_empty() { "saved/g555-s.hwp" } else { &args[0] };
    let output = if args.len() > 1 { &args[1] } else { "/tmp/test-shape-out.hwp" };

    let data = match fs::read(input) {
        Ok(d) => d,
        Err(e) => { eprintln!("입력 파일 읽기 오류: {}", e); return; }
    };

    let mut doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => { eprintln!("HWP 파싱 오류: {:?}", e); return; }
    };

    let _ = doc.convert_to_editable_native();

    // 글상자 생성 (9000 x 6750 HWPUNIT)
    let result = doc.create_shape_control_native(0, 0, 0, 9000, 6750, 0, 0, false, "InFrontOfText", "rectangle", false, false, &[]);
    match &result {
        Ok(r) => eprintln!("글상자 생성 성공: {}", r),
        Err(e) => { eprintln!("글상자 생성 실패: {:?}", e); return; }
    }

    match doc.export_hwp_native() {
        Ok(bytes) => {
            if let Err(e) = fs::write(output, &bytes) {
                eprintln!("파일 저장 오류: {}", e);
            } else {
                eprintln!("저장 완료: {} ({}KB)", output, bytes.len() / 1024);
            }
        }
        Err(e) => eprintln!("직렬화 오류: {:?}", e),
    }
}

/// 캡션 방향별 테스트: 4개 이미지에 각각 Bottom/Top/Left/Right 캡션을 설정하고 SVG 출력
fn test_caption(args: &[String]) {
    if args.is_empty() {
        eprintln!("사용법: rhwp test-caption <파일.hwp>");
        return;
    }

    let data = match fs::read(&args[0]) {
        Ok(d) => d,
        Err(e) => { eprintln!("파일 읽기 오류: {}", e); return; }
    };

    let mut doc = match rhwp::wasm_api::HwpDocument::from_bytes(&data) {
        Ok(d) => d,
        Err(e) => { eprintln!("파싱 오류: {}", e); return; }
    };

    // 문단 0: 컨트롤 2,3 / 문단 1: 컨트롤 0,1
    let pic_refs: [(usize, usize); 4] = [(0, 2), (0, 3), (1, 0), (1, 1)];

    // 4개 이미지에 각각 다른 캡션 방향 설정
    let directions = [
        ("Bottom", "Top"),
        ("Top", "Top"),
        ("Left", "Center"),
        ("Right", "Center"),
    ];

    for (i, ((para, ci), (dir, va))) in pic_refs.iter().zip(directions.iter()).enumerate() {
        let json = format!(
            r#"{{"hasCaption":true,"captionDirection":"{}","captionVertAlign":"{}","captionWidth":8504,"captionSpacing":850}}"#,
            dir, va
        );
        println!("[{}] para={}, ci={}, dir={}, va={}", i, para, ci, dir, va);
        match doc.set_picture_properties_native(0, *para, *ci, &json) {
            Ok(r) => println!("  결과: {}", r),
            Err(e) => println!("  오류: {:?}", e),
        }
    }

    // 캡션 상태 확인
    for (i, (para, ci)) in pic_refs.iter().enumerate() {
        let section = &doc.document().sections[0];
        let p = &section.paragraphs[*para];
        if let rhwp::model::control::Control::Picture(pic) = &p.controls[*ci] {
            println!("[{}] caption={:?}", i, pic.caption.as_ref().map(|c| {
                format!("dir={:?}, paras={}, text={:?}",
                    c.direction, c.paragraphs.len(),
                    c.paragraphs.first().map(|p| &p.text))
            }));
        }
    }

    // SVG 출력
    let output_dir = "output/caption-test";
    let _ = fs::create_dir_all(output_dir);
    let page_count = doc.page_count();
    println!("페이지 수: {}", page_count);
    for p in 0..page_count {
        let svg = doc.render_page_svg(p).expect("SVG 렌더링 오류");
        let path = format!("{}/caption-test-p{}.svg", output_dir, p);
        fs::write(&path, &svg).unwrap();
        println!("  → {}", path);
    }
    println!("완료");
}

fn gen_table(args: &[String]) {
    let rows: u16 = args.first().and_then(|s| s.parse().ok()).unwrap_or(1000);
    let cols: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let output = args.get(2).map(|s| s.as_str()).unwrap_or("output/gen_table.hwp");

    println!("{}행 × {}열 표 생성 중...", rows, cols);

    let mut core = rhwp::document_core::DocumentCore::new_empty();
    core.create_blank_document_native().expect("빈 문서 생성 실패");

    // 표 생성
    let result = core.create_table_native(0, 0, 0, rows, cols)
        .expect("표 생성 실패");
    println!("  표 생성: {}", result);

    // 결과에서 paraIdx 파싱
    let table_para_idx: usize = result.split("\"paraIdx\":").nth(1)
        .and_then(|s| s.split(&[',', '}'][..]).next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);
    println!("  표 문단 인덱스: {}", table_para_idx);

    // 배치 모드로 셀 내용 채우기
    core.begin_batch_native().expect("배치 시작 실패");

    let headers = ["번호", "이름", "부서", "직급", "연락처", "비고"];
    // 헤더 행
    for (ci, header) in headers.iter().enumerate().take(cols as usize) {
        let _ = core.insert_text_in_cell_native(0, table_para_idx, 0, ci, 0, 0, header);
    }

    // 데이터 행
    let departments = ["개발팀", "기획팀", "디자인팀", "영업팀", "인사팀", "재무팀"];
    let positions = ["사원", "대리", "과장", "차장", "부장"];
    for row in 1..rows as usize {
        for col in 0..cols as usize {
            let cell_idx = row * cols as usize + col;
            let text = match col {
                0 => format!("{}", row),
                1 => format!("홍길동{}", row),
                2 => departments[row % departments.len()].to_string(),
                3 => positions[row % positions.len()].to_string(),
                4 => format!("010-{:04}-{:04}", 1000 + row % 9000, 1000 + (row * 7) % 9000),
                5 => if row % 3 == 0 { "특이사항 없음".to_string() } else { String::new() },
                _ => format!("R{}C{}", row, col),
            };
            if !text.is_empty() {
                let _ = core.insert_text_in_cell_native(0, table_para_idx, 0, cell_idx, 0, 0, &text);
            }
        }
        if row % 100 == 0 {
            println!("  {} / {} 행 완료", row, rows);
        }
    }

    core.end_batch_native().expect("배치 종료 실패");
    println!("  셀 내용 입력 완료");

    // 저장
    let bytes = core.export_hwp_native().expect("HWP 내보내기 실패");
    let out_path = Path::new(output);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(out_path, bytes).expect("파일 저장 실패");
    println!("저장 완료: {} ({}행 × {}열)", output, rows, cols);
}

fn test_field_roundtrip(args: &[String]) {
    let input = args.first().map(|s| s.as_str()).unwrap_or("hwp_webctl/bsbc01_10_000.hwp");
    let output = args.get(1).map(|s| s.as_str()).unwrap_or("output/field_test.hwp");
    
    let data = std::fs::read(input).expect("파일 읽기 실패");
    let mut core = rhwp::document_core::DocumentCore::from_bytes(&data)
        .expect("문서 파싱 실패");
    
    // 1. 필드 목록 출력
    let fields = core.collect_all_fields();
    println!("=== 필드 목록 ({}개) ===", fields.len());
    for fi in &fields {
        let name = fi.field.field_name().unwrap_or("(이름없음)");
        println!("  {} = \"{}\"", name, fi.value);
    }
    
    // 2. 필드에 값 설정
    let test_data = [
        ("mbizNm", "청소년 자립지원사업"),
        ("newCtnuTxt", "계속"),
        ("chargerNm", "홍길동"),
        ("telno", "02-1234-5678"),
        ("sFisYear", "2026"),
        // 셀 필드
        ("bizPurps", "청소년 자립 역량 강화"),
        ("bizPrdTxt", "2026.01 ~ 2026.12"),
        ("insttNm", "시청 복지과"),
    ];
    
    println!("\n=== 필드 값 설정 ===");
    for (name, value) in &test_data {
        match core.set_field_value_by_name(name, value) {
            Ok(r) => println!("  ✓ {} = \"{}\" → {}", name, value, r),
            Err(e) => println!("  ✗ {} = \"{}\" → {}", name, value, e),
        }
    }
    
    // 3. 설정 후 확인
    println!("\n=== 설정 후 확인 ===");
    let fields2 = core.collect_all_fields();
    for fi in &fields2 {
        let name = fi.field.field_name().unwrap_or("(이름없음)");
        println!("  {} = \"{}\"", name, fi.value);
    }
    
    // 3.5 pi=0 문단 텍스트 직접 확인
    let para0 = &core.document().sections[0].paragraphs[0];

    // 4. 직렬화 → 저장
    let saved = core.export_hwp_native().expect("직렬화 실패");
    std::fs::write(output, &saved).expect("저장 실패");
    println!("\n저장: {} ({}바이트)", output, saved.len());
    
    // 5. 재로딩 → 필드 확인
    let mut core2 = rhwp::document_core::DocumentCore::from_bytes(&saved)
        .expect("재로딩 실패");
    let fields3 = core2.collect_all_fields();
    println!("\n=== 재로딩 후 확인 ===");
    for fi in &fields3 {
        let name = fi.field.field_name().unwrap_or("(이름없음)");
        println!("  {} = \"{}\"", name, fi.value);
    }
}

fn ir_diff(args: &[String]) {
    if args.len() < 2 {
        eprintln!("사용법: rhwp ir-diff <파일A> <파일B> [-s <구역>] [-p <문단>]");
        return;
    }

    let file_a = &args[0];
    let file_b = &args[1];
    let mut section_filter: Option<usize> = None;
    let mut para_filter: Option<usize> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-s" | "--section" if i + 1 < args.len() => {
                section_filter = args[i + 1].parse().ok();
                i += 2;
            }
            "-p" | "--para" if i + 1 < args.len() => {
                para_filter = args[i + 1].parse().ok();
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    let data_a = match fs::read(file_a) {
        Ok(d) => d,
        Err(e) => { eprintln!("오류: {} 읽기 실패: {}", file_a, e); return; }
    };
    let data_b = match fs::read(file_b) {
        Ok(d) => d,
        Err(e) => { eprintln!("오류: {} 읽기 실패: {}", file_b, e); return; }
    };

    let doc_a = match rhwp::parser::parse_document(&data_a) {
        Ok(d) => d,
        Err(e) => { eprintln!("오류: {} 파싱 실패: {:?}", file_a, e); return; }
    };
    let doc_b = match rhwp::parser::parse_document(&data_b) {
        Ok(d) => d,
        Err(e) => { eprintln!("오류: {} 파싱 실패: {:?}", file_b, e); return; }
    };

    let name_a = Path::new(file_a).file_name().unwrap_or_default().to_string_lossy();
    let name_b = Path::new(file_b).file_name().unwrap_or_default().to_string_lossy();
    println!("=== IR 비교: {} vs {} ===", name_a, name_b);

    // 구역 수 비교
    if doc_a.sections.len() != doc_b.sections.len() {
        println!("[차이] 구역 수: A={} vs B={}", doc_a.sections.len(), doc_b.sections.len());
    }

    let sec_count = doc_a.sections.len().min(doc_b.sections.len());
    let mut total_diffs = 0u32;

    for sec_idx in 0..sec_count {
        if let Some(sf) = section_filter {
            if sec_idx != sf { continue; }
        }

        let sec_a = &doc_a.sections[sec_idx];
        let sec_b = &doc_b.sections[sec_idx];

        if sec_a.paragraphs.len() != sec_b.paragraphs.len() {
            println!("[차이] 구역 {}: 문단 수 A={} vs B={}", sec_idx, sec_a.paragraphs.len(), sec_b.paragraphs.len());
            total_diffs += 1;
        }

        let para_count = sec_a.paragraphs.len().min(sec_b.paragraphs.len());
        for pi in 0..para_count {
            if let Some(pf) = para_filter {
                if pi != pf { continue; }
            }

            let pa = &sec_a.paragraphs[pi];
            let pb = &sec_b.paragraphs[pi];
            let mut diffs: Vec<String> = Vec::new();

            // 텍스트 비교
            if pa.text != pb.text {
                diffs.push(format!("text: A={:?} vs B={:?}",
                    pa.text.chars().take(30).collect::<String>(),
                    pb.text.chars().take(30).collect::<String>()));
            }

            // char_count 비교
            if pa.char_count != pb.char_count {
                diffs.push(format!("cc: A={} vs B={}", pa.char_count, pb.char_count));
            }

            // char_offsets 비교
            if pa.char_offsets != pb.char_offsets {
                let len_a = pa.char_offsets.len();
                let len_b = pb.char_offsets.len();
                if len_a != len_b {
                    diffs.push(format!("char_offsets len: A={} vs B={}", len_a, len_b));
                } else {
                    let first_diff = pa.char_offsets.iter().zip(pb.char_offsets.iter())
                        .enumerate()
                        .find(|(_, (a, b))| a != b);
                    if let Some((idx, (a, b))) = first_diff {
                        diffs.push(format!("char_offsets[{}]: A={} vs B={}", idx, a, b));
                    }
                }
            }

            // para_shape_id 비교
            if pa.para_shape_id != pb.para_shape_id {
                diffs.push(format!("ps_id: A={} vs B={}", pa.para_shape_id, pb.para_shape_id));
            }

            // tab_extended 비교
            if pa.tab_extended.len() != pb.tab_extended.len() {
                diffs.push(format!("tab_ext count: A={} vs B={}", pa.tab_extended.len(), pb.tab_extended.len()));
            } else {
                for (ti, (ta, tb)) in pa.tab_extended.iter().zip(pb.tab_extended.iter()).enumerate() {
                    if ta != tb {
                        diffs.push(format!("tab_ext[{}]: A={:?} vs B={:?}", ti, ta, tb));
                        break;
                    }
                }
            }

            // LINE_SEG 비교
            if pa.line_segs.len() != pb.line_segs.len() {
                diffs.push(format!("line_segs count: A={} vs B={}", pa.line_segs.len(), pb.line_segs.len()));
            } else {
                for (li, (la, lb)) in pa.line_segs.iter().zip(pb.line_segs.iter()).enumerate() {
                    if la.text_start != lb.text_start {
                        diffs.push(format!("ls[{}].ts: A={} vs B={}", li, la.text_start, lb.text_start));
                    }
                    if la.line_height != lb.line_height {
                        diffs.push(format!("ls[{}].lh: A={} vs B={}", li, la.line_height, lb.line_height));
                    }
                    if la.segment_width != lb.segment_width {
                        diffs.push(format!("ls[{}].sw: A={} vs B={}", li, la.segment_width, lb.segment_width));
                    }
                }
            }

            // 컨트롤 수 비교
            if pa.controls.len() != pb.controls.len() {
                diffs.push(format!("controls: A={} vs B={}", pa.controls.len(), pb.controls.len()));
            }

            // char_shapes 비교
            if pa.char_shapes.len() != pb.char_shapes.len() {
                diffs.push(format!("char_shapes count: A={} vs B={}", pa.char_shapes.len(), pb.char_shapes.len()));
            } else {
                for (ci, (ca, cb)) in pa.char_shapes.iter().zip(pb.char_shapes.iter()).enumerate() {
                    if ca.start_pos != cb.start_pos {
                        diffs.push(format!("cs[{}].pos: A={} vs B={}", ci, ca.start_pos, cb.start_pos));
                        break;
                    }
                    if ca.char_shape_id != cb.char_shape_id {
                        diffs.push(format!("cs[{}].id: A={} vs B={}", ci, ca.char_shape_id, cb.char_shape_id));
                        break;
                    }
                }
            }

            if !diffs.is_empty() {
                let text_preview: String = pa.text.chars().take(30).collect();
                println!("\n--- 문단 {}.{} --- \"{}\"", sec_idx, pi, text_preview);
                for d in &diffs {
                    println!("  [차이] {}", d);
                }
                total_diffs += diffs.len() as u32;
            }
        }
    }

    // doc_info 비교: ParaShape
    {
        let ps_a = &doc_a.doc_info.para_shapes;
        let ps_b = &doc_b.doc_info.para_shapes;
        if ps_a.len() != ps_b.len() {
            println!("\n[차이] ParaShape 수: A={} vs B={}", ps_a.len(), ps_b.len());
            total_diffs += 1;
        }
        let ps_count = ps_a.len().min(ps_b.len());
        for i in 0..ps_count {
            let a = &ps_a[i]; let b = &ps_b[i];
            let mut ps_diffs: Vec<String> = Vec::new();
            if a.margin_left != b.margin_left { ps_diffs.push(format!("ml: {}vs{}", a.margin_left, b.margin_left)); }
            if a.margin_right != b.margin_right { ps_diffs.push(format!("mr: {}vs{}", a.margin_right, b.margin_right)); }
            if a.indent != b.indent { ps_diffs.push(format!("indent: {}vs{}", a.indent, b.indent)); }
            if a.tab_def_id != b.tab_def_id { ps_diffs.push(format!("tab_def: {}vs{}", a.tab_def_id, b.tab_def_id)); }
            if a.spacing_before != b.spacing_before { ps_diffs.push(format!("sb: {}vs{}", a.spacing_before, b.spacing_before)); }
            if a.spacing_after != b.spacing_after { ps_diffs.push(format!("sa: {}vs{}", a.spacing_after, b.spacing_after)); }
            if a.line_spacing != b.line_spacing { ps_diffs.push(format!("ls: {}vs{}", a.line_spacing, b.line_spacing)); }
            if !ps_diffs.is_empty() {
                println!("  [PS {}] {}", i, ps_diffs.join(", "));
                total_diffs += ps_diffs.len() as u32;
            }
        }
    }

    // doc_info 비교: TabDef
    {
        let td_a = &doc_a.doc_info.tab_defs;
        let td_b = &doc_b.doc_info.tab_defs;
        if td_a.len() != td_b.len() {
            println!("\n[차이] TabDef 수: A={} vs B={}", td_a.len(), td_b.len());
            total_diffs += 1;
        }
        let td_count = td_a.len().min(td_b.len());
        for i in 0..td_count {
            let a = &td_a[i]; let b = &td_b[i];
            if a.tabs.len() != b.tabs.len() {
                println!("  [TD {}] 탭 수: A={} vs B={}", i, a.tabs.len(), b.tabs.len());
                total_diffs += 1;
            } else {
                for (ti, (ta, tb)) in a.tabs.iter().zip(b.tabs.iter()).enumerate() {
                    if ta.position != tb.position || ta.tab_type != tb.tab_type || ta.fill_type != tb.fill_type {
                        println!("  [TD {}][{}] pos: {}vs{}, type: {}vs{}, fill: {}vs{}",
                            i, ti, ta.position, tb.position, ta.tab_type, tb.tab_type, ta.fill_type, tb.fill_type);
                        total_diffs += 1;
                    }
                }
            }
        }
    }

    println!("\n=== 비교 완료: 차이 {} 건 ===", total_diffs);
}

fn extract_thumbnail(args: &[String]) {
    if args.is_empty() {
        eprintln!("사용법: rhwp thumbnail <파일.hwp> [옵션]");
        eprintln!("  -o, --output <파일>   출력 파일 경로");
        eprintln!("  --base64              base64 문자열 출력");
        eprintln!("  --data-uri            data:image/... URI 출력");
        std::process::exit(1);
    }

    let input_path = &args[0];
    let mut output_path: Option<String> = None;
    let mut mode = "file"; // "file", "base64", "data-uri"

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(args[i].clone());
                }
            }
            "--base64" => mode = "base64",
            "--data-uri" => mode = "data-uri",
            _ => {}
        }
        i += 1;
    }

    let data = match fs::read(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("오류: 파일을 읽을 수 없습니다: {} ({})", input_path, e);
            std::process::exit(1);
        }
    };

    let result = match rhwp::parser::extract_thumbnail_only(&data) {
        Some(r) => r,
        None => {
            eprintln!("오류: PrvImage 썸네일이 없습니다: {}", input_path);
            std::process::exit(1);
        }
    };

    let mime = match result.format.as_str() {
        "png" => "image/png",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    };

    match mode {
        "base64" => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&result.data);
            println!("{}", b64);
        }
        "data-uri" => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&result.data);
            println!("data:{};base64,{}", mime, b64);
        }
        _ => {
            // 파일 출력
            let out = output_path.unwrap_or_else(|| {
                let stem = Path::new(input_path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                let ext = &result.format;
                format!("output/{}_thumb.{}", stem, ext)
            });

            // 출력 디렉토리 생성
            if let Some(parent) = Path::new(&out).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).ok();
                }
            }

            match fs::write(&out, &result.data) {
                Ok(_) => {
                    println!("썸네일 추출 완료: {} ({}x{}, {} bytes, {})",
                        out, result.width, result.height, result.data.len(), result.format);
                }
                Err(e) => {
                    eprintln!("오류: 파일 저장 실패: {} ({})", out, e);
                    std::process::exit(1);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CorpusRoundtripExpectation {
    None,
    SaveReparse,
}

#[derive(Debug, Clone)]
struct CorpusEntry {
    relative_path: PathBuf,
    absolute_path: PathBuf,
    expected_edit_mode: Option<rhwp::document_core::DocumentEditMode>,
    expected_save_format: Option<rhwp::document_core::DocumentSourceFormat>,
    required_issue_codes: Vec<String>,
    roundtrip: CorpusRoundtripExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderSignature {
    page_count: u32,
    page_info_hashes: Vec<u64>,
    render_tree_hashes: Vec<u64>,
}

#[derive(Debug, Clone)]
struct CompatReportBundle {
    report: rhwp::document_core::CompatibilityReportData,
    fonts: rhwp::document_core::FontSubstitutionReportData,
    render: RenderSignature,
}

#[derive(Debug, Clone, Serialize)]
struct CompatReportJson<'a> {
    file: String,
    #[serde(rename = "sourceFormat")]
    source_format: &'a str,
    #[serde(rename = "preferredSaveFormat")]
    preferred_save_format: &'a str,
    #[serde(rename = "editMode")]
    edit_mode: &'a str,
    #[serde(rename = "pageCount")]
    page_count: u32,
    issues: Vec<CompatIssueJson<'a>>,
    #[serde(rename = "fontSubstitutions")]
    font_substitutions: Vec<CompatFontSubstitutionJson<'a>>,
    #[serde(rename = "renderSignature")]
    render_signature: CompatRenderSignatureJson<'a>,
}

#[derive(Debug, Clone, Serialize)]
struct CompatIssueJson<'a> {
    code: &'a str,
    severity: &'a str,
    message: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct CompatFontSubstitutionJson<'a> {
    lang: &'a str,
    original: &'a str,
    resolved: &'a str,
    substituted: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CompatRenderSignatureJson<'a> {
    #[serde(rename = "pageCount")]
    page_count: u32,
    #[serde(rename = "pageInfoHashes")]
    page_info_hashes: &'a [u64],
    #[serde(rename = "renderTreeHashes")]
    render_tree_hashes: &'a [u64],
}

#[derive(Debug, Clone, Serialize)]
struct CompatCorpusEntryJson {
    path: String,
    passed: bool,
    #[serde(rename = "sourceFormat", skip_serializing_if = "Option::is_none")]
    source_format: Option<String>,
    #[serde(rename = "preferredSaveFormat", skip_serializing_if = "Option::is_none")]
    preferred_save_format: Option<String>,
    #[serde(rename = "editMode", skip_serializing_if = "Option::is_none")]
    edit_mode: Option<String>,
    #[serde(rename = "issueCodes")]
    issue_codes: Vec<String>,
    #[serde(rename = "reportPath", skip_serializing_if = "Option::is_none")]
    report_path: Option<String>,
    problems: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CompatCorpusJson {
    manifest: String,
    #[serde(rename = "totalEntries")]
    total_entries: usize,
    failures: usize,
    passed: bool,
    entries: Vec<CompatCorpusEntryJson>,
}

fn compat_report(args: &[String]) {
    let mut json = false;
    let mut file_arg: Option<&str> = None;

    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                eprintln!("usage: rhwp compat-report [--json] <file.hwp|file.hwpx>");
                std::process::exit(1);
            }
            value => {
                if file_arg.replace(value).is_some() {
                    eprintln!("usage: rhwp compat-report [--json] <file.hwp|file.hwpx>");
                    std::process::exit(1);
                }
            }
        }
    }

    let file_path = match file_arg {
        Some(value) => Path::new(value),
        None => {
            eprintln!("usage: rhwp compat-report [--json] <file.hwp|file.hwpx>");
            std::process::exit(1);
        }
    };

    let bundle = match load_compat_report_bundle(file_path) {
        Ok(bundle) => bundle,
        Err(error) => {
            eprintln!("error: {}", error);
            std::process::exit(1);
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&compat_report_json(file_path, &bundle))
                .expect("serialize compat report json"),
        );
    } else {
        print_compat_report_text(file_path, &bundle);
    }
}

fn compat_corpus(args: &[String]) {
    let mut json = false;
    let mut emit_reports: Option<PathBuf> = None;
    let mut manifest_arg: Option<&str> = None;
    let mut idx = 0usize;

    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--emit-reports" => {
                let Some(dir) = args.get(idx + 1) else {
                    eprintln!("usage: rhwp compat-corpus [--json] [--emit-reports <dir>] <manifest.tsv>");
                    std::process::exit(1);
                };
                emit_reports = Some(PathBuf::from(dir));
                idx += 2;
            }
            value if value.starts_with('-') => {
                eprintln!("usage: rhwp compat-corpus [--json] [--emit-reports <dir>] <manifest.tsv>");
                std::process::exit(1);
            }
            value => {
                if manifest_arg.replace(value).is_some() {
                    eprintln!("usage: rhwp compat-corpus [--json] [--emit-reports <dir>] <manifest.tsv>");
                    std::process::exit(1);
                }
                idx += 1;
            }
        }
    }

    let manifest_path = match manifest_arg {
        Some(value) => Path::new(value),
        None => {
            eprintln!("usage: rhwp compat-corpus [--json] [--emit-reports <dir>] <manifest.tsv>");
            std::process::exit(1);
        }
    };
    let entries = match load_corpus_manifest(manifest_path) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("error: {}", error);
            std::process::exit(1);
        }
    };

    if entries.is_empty() {
        eprintln!("error: corpus manifest has no entries: {}", manifest_path.display());
        std::process::exit(1);
    }

    if let Some(report_dir) = emit_reports.as_ref() {
        if let Err(error) = fs::create_dir_all(report_dir) {
            eprintln!("error: failed to create {}: {}", report_dir.display(), error);
            std::process::exit(1);
        }
    }

    if !json {
        println!(
            "compat-corpus: {} entries from {}",
            entries.len(),
            manifest_path.display()
        );
    }

    let mut failures = 0usize;
    let mut json_entries = Vec::new();
    for entry in entries {
        let mut problems = Vec::new();
        let mut source_format = None;
        let mut preferred_save_format = None;
        let mut edit_mode = None;
        let mut issue_codes = Vec::new();
        let mut report_path = None;

        let data = match fs::read(&entry.absolute_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                problems.push(format!("failed to read fixture: {}", error));
                failures += 1;
                if !json {
                    eprintln!(
                        "FAIL {}: failed to read fixture: {}",
                        entry.relative_path.display(),
                        error
                    );
                }
                json_entries.push(CompatCorpusEntryJson {
                    path: entry.relative_path.display().to_string(),
                    passed: false,
                    source_format,
                    preferred_save_format,
                    edit_mode,
                    issue_codes,
                    report_path,
                    problems,
                });
                continue;
            }
        };

        match load_compat_report_bundle(&entry.absolute_path) {
            Ok(bundle) => {
                let report = &bundle.report;
                source_format = Some(report.source_format.as_str().to_string());
                preferred_save_format = Some(report.preferred_save_format.as_str().to_string());
                edit_mode = Some(report.edit_mode.as_str().to_string());
                issue_codes = report
                    .issues
                    .iter()
                    .map(|issue| issue.code.to_string())
                    .collect::<Vec<_>>();

                let actual_codes = report
                    .issues
                    .iter()
                    .map(|issue| issue.code)
                    .collect::<Vec<_>>();

                if let Some(expected_edit_mode) = entry.expected_edit_mode {
                    if report.edit_mode != expected_edit_mode {
                        problems.push(format!(
                            "expected edit mode {}, got {}",
                            expected_edit_mode.as_str(),
                            report.edit_mode.as_str()
                        ));
                    }
                }

                if let Some(expected_save_format) = entry.expected_save_format {
                    if report.preferred_save_format != expected_save_format {
                        problems.push(format!(
                            "expected preferred save format {}, got {}",
                            expected_save_format.as_str(),
                            report.preferred_save_format.as_str()
                        ));
                    }
                }

                for required_code in &entry.required_issue_codes {
                    if !actual_codes.iter().any(|code| code == required_code) {
                        problems.push(format!("missing required issue code {}", required_code));
                    }
                }

                if entry.roundtrip == CorpusRoundtripExpectation::SaveReparse {
                    let core = match rhwp::document_core::DocumentCore::from_bytes(&data) {
                        Ok(core) => core,
                        Err(error) => {
                            problems.push(format!("failed to reconstruct core for roundtrip: {}", error));
                            failures += 1;
                            if !json {
                                eprintln!("FAIL {}", entry.relative_path.display());
                                for problem in &problems {
                                    eprintln!("  - {}", problem);
                                }
                            }
                            json_entries.push(CompatCorpusEntryJson {
                                path: entry.relative_path.display().to_string(),
                                passed: false,
                                source_format,
                                preferred_save_format,
                                edit_mode,
                                issue_codes,
                                report_path,
                                problems,
                            });
                            continue;
                        }
                    };

                    if let Err(error) = validate_roundtrip(&core, report, &bundle.render) {
                        problems.push(error);
                    }
                }

                if let Some(report_dir) = emit_reports.as_ref() {
                    match write_compat_report_json_file(report_dir, &entry.relative_path, &entry.absolute_path, &bundle) {
                        Ok(path) => {
                            report_path = Some(path.display().to_string());
                        }
                        Err(error) => {
                            problems.push(format!("failed to write report artifact: {}", error));
                        }
                    }
                }
            }
            Err(error) => {
                problems.push(error);
            }
        }

        if problems.is_empty() {
            if !json {
                println!(
                    "OK   {} ({}, {}, {} issues)",
                    entry.relative_path.display(),
                    source_format.as_deref().unwrap_or("unknown"),
                    edit_mode.as_deref().unwrap_or("unknown"),
                    issue_codes.len()
                );
            }
        } else {
            failures += 1;
            if !json {
                eprintln!("FAIL {}", entry.relative_path.display());
                for problem in &problems {
                    eprintln!("  - {}", problem);
                }
            }
        }

        json_entries.push(CompatCorpusEntryJson {
            path: entry.relative_path.display().to_string(),
            passed: problems.is_empty(),
            source_format,
            preferred_save_format,
            edit_mode,
            issue_codes,
            report_path,
            problems,
        });
    }

    let summary = CompatCorpusJson {
        manifest: manifest_path.display().to_string(),
        total_entries: json_entries.len(),
        failures,
        passed: failures == 0,
        entries: json_entries,
    };

    if failures > 0 {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("serialize compat corpus json"),
            );
        } else {
            eprintln!("compat-corpus: {} failing entries", failures);
        }
        std::process::exit(1);
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).expect("serialize compat corpus json"),
        );
    } else {
        println!("compat-corpus: all entries passed");
    }
}

fn load_compat_report_bundle(file_path: &Path) -> Result<CompatReportBundle, String> {
    let data = fs::read(file_path)
        .map_err(|error| format!("failed to read {}: {}", file_path.display(), error))?;
    let core = rhwp::document_core::DocumentCore::from_bytes(&data)
        .map_err(|error| format!("failed to parse {}: {}", file_path.display(), error))?;
    let report = core.compatibility_report_data();
    let fonts = core.font_substitution_report_data();
    let render = capture_render_signature(&data)
        .map_err(|error| format!("failed to render {}: {}", file_path.display(), error))?;

    Ok(CompatReportBundle { report, fonts, render })
}

fn compat_report_json<'a>(
    file_path: &Path,
    bundle: &'a CompatReportBundle,
) -> CompatReportJson<'a> {
    CompatReportJson {
        file: file_path.display().to_string(),
        source_format: bundle.report.source_format.as_str(),
        preferred_save_format: bundle.report.preferred_save_format.as_str(),
        edit_mode: bundle.report.edit_mode.as_str(),
        page_count: bundle.render.page_count,
        issues: bundle
            .report
            .issues
            .iter()
            .map(|issue| CompatIssueJson {
                code: issue.code,
                severity: issue.severity,
                message: issue.message.as_str(),
            })
            .collect(),
        font_substitutions: bundle
            .fonts
            .items
            .iter()
            .map(|item| CompatFontSubstitutionJson {
                lang: item.lang.as_str(),
                original: item.original.as_str(),
                resolved: item.resolved.as_str(),
                substituted: item.substituted,
            })
            .collect(),
        render_signature: CompatRenderSignatureJson {
            page_count: bundle.render.page_count,
            page_info_hashes: &bundle.render.page_info_hashes,
            render_tree_hashes: &bundle.render.render_tree_hashes,
        },
    }
}

fn print_compat_report_text(file_path: &Path, bundle: &CompatReportBundle) {
    let report = &bundle.report;
    let fonts = &bundle.fonts;
    let render = &bundle.render;

    println!("file: {}", file_path.display());
    println!("sourceFormat: {}", report.source_format.as_str());
    println!("preferredSaveFormat: {}", report.preferred_save_format.as_str());
    println!("editMode: {}", report.edit_mode.as_str());
    println!("pageCount: {}", render.page_count);
    println!("issues:");
    if report.issues.is_empty() {
        println!("  - none");
    } else {
        for issue in &report.issues {
            println!("  - {} [{}] {}", issue.code, issue.severity, issue.message);
        }
    }
    println!("fontSubstitutions:");
    if fonts.items.is_empty() {
        println!("  - none");
    } else {
        for item in &fonts.items {
            println!(
                "  - {}: {} -> {}{}",
                item.lang,
                item.original,
                item.resolved,
                if item.substituted { " (substituted)" } else { "" }
            );
        }
    }
    println!("renderSignature:");
    println!("  pageInfoHashes: {:?}", render.page_info_hashes);
    println!("  renderTreeHashes: {:?}", render.render_tree_hashes);
}

fn write_compat_report_json_file(
    output_dir: &Path,
    relative_path: &Path,
    absolute_path: &Path,
    bundle: &CompatReportBundle,
) -> Result<PathBuf, String> {
    let output_path = output_dir.join(sanitized_report_file_name(relative_path));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {}", parent.display(), error))?;
    }

    let json = serde_json::to_vec_pretty(&compat_report_json(absolute_path, bundle))
        .map_err(|error| format!("failed to serialize report for {}: {}", relative_path.display(), error))?;
    fs::write(&output_path, json)
        .map_err(|error| format!("failed to write {}: {}", output_path.display(), error))?;
    Ok(output_path)
}

fn sanitized_report_file_name(relative_path: &Path) -> String {
    let raw = relative_path
        .display()
        .to_string()
        .replace("../", "up__")
        .replace("..\\", "up__")
        .replace(['/', '\\', ':'], "__");
    format!("{raw}.json")
}

fn load_corpus_manifest(manifest_path: &Path) -> Result<Vec<CorpusEntry>, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("failed to read {}: {}", manifest_path.display(), error))?;
    let base_dir = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut entries = Vec::new();

    for (line_idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let columns = line.split('\t').map(str::trim).collect::<Vec<_>>();
        let relative_path = columns
            .first()
            .ok_or_else(|| format!("{}:{} missing path column", manifest_path.display(), line_idx + 1))?;
        if relative_path.is_empty() {
            return Err(format!(
                "{}:{} path column cannot be empty",
                manifest_path.display(),
                line_idx + 1
            ));
        }

        let expected_edit_mode = parse_expected_edit_mode(
            columns.get(1).copied().unwrap_or_default(),
            manifest_path,
            line_idx + 1,
        )?;
        let expected_save_format = parse_expected_save_format(
            columns.get(2).copied().unwrap_or_default(),
            manifest_path,
            line_idx + 1,
        )?;
        let required_issue_codes = parse_required_issue_codes(
            columns.get(3).copied().unwrap_or_default(),
        );
        let roundtrip = parse_roundtrip_expectation(
            columns.get(4).copied().unwrap_or_default(),
            expected_edit_mode,
            manifest_path,
            line_idx + 1,
        )?;

        let relative_path = PathBuf::from(relative_path);
        let absolute_path = if relative_path.is_absolute() {
            relative_path.clone()
        } else {
            base_dir.join(&relative_path)
        };

        entries.push(CorpusEntry {
            relative_path,
            absolute_path,
            expected_edit_mode,
            expected_save_format,
            required_issue_codes,
            roundtrip,
        });
    }

    Ok(entries)
}

fn parse_expected_edit_mode(
    value: &str,
    manifest_path: &Path,
    line_number: usize,
) -> Result<Option<rhwp::document_core::DocumentEditMode>, String> {
    match value {
        "" => Ok(None),
        "editable-safe" => Ok(Some(rhwp::document_core::DocumentEditMode::EditableSafe)),
        "protected-view" => Ok(Some(rhwp::document_core::DocumentEditMode::ProtectedView)),
        other => Err(format!(
            "{}:{} invalid edit mode: {}",
            manifest_path.display(),
            line_number,
            other
        )),
    }
}

fn parse_expected_save_format(
    value: &str,
    manifest_path: &Path,
    line_number: usize,
) -> Result<Option<rhwp::document_core::DocumentSourceFormat>, String> {
    match value {
        "" => Ok(None),
        "hwp" => Ok(Some(rhwp::document_core::DocumentSourceFormat::Hwp)),
        "hwpx" => Ok(Some(rhwp::document_core::DocumentSourceFormat::Hwpx)),
        "unknown" => Ok(Some(rhwp::document_core::DocumentSourceFormat::Unknown)),
        other => Err(format!(
            "{}:{} invalid save format: {}",
            manifest_path.display(),
            line_number,
            other
        )),
    }
}

fn parse_required_issue_codes(value: &str) -> Vec<String> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Vec::new();
    }

    value
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_roundtrip_expectation(
    value: &str,
    expected_edit_mode: Option<rhwp::document_core::DocumentEditMode>,
    manifest_path: &Path,
    line_number: usize,
) -> Result<CorpusRoundtripExpectation, String> {
    match value {
        "" => Ok(if expected_edit_mode
            == Some(rhwp::document_core::DocumentEditMode::EditableSafe)
        {
            CorpusRoundtripExpectation::SaveReparse
        } else {
            CorpusRoundtripExpectation::None
        }),
        "save-reparse" => Ok(CorpusRoundtripExpectation::SaveReparse),
        "none" => Ok(CorpusRoundtripExpectation::None),
        other => Err(format!(
            "{}:{} invalid roundtrip mode: {}",
            manifest_path.display(),
            line_number,
            other
        )),
    }
}

fn validate_roundtrip(
    core: &rhwp::document_core::DocumentCore,
    report: &rhwp::document_core::CompatibilityReportData,
    original_signature: &RenderSignature,
) -> Result<(), String> {
    if report.edit_mode != rhwp::document_core::DocumentEditMode::EditableSafe {
        return Err(format!(
            "roundtrip requested but edit mode is {}",
            report.edit_mode.as_str()
        ));
    }

    let save_format = match report.preferred_save_format {
        rhwp::document_core::DocumentSourceFormat::Unknown => rhwp::document_core::DocumentSourceFormat::Hwp,
        format => format,
    };

    let saved = match save_format {
        rhwp::document_core::DocumentSourceFormat::Hwp => core.export_hwp_native(),
        rhwp::document_core::DocumentSourceFormat::Hwpx => core.export_hwpx_native(),
        rhwp::document_core::DocumentSourceFormat::Unknown => unreachable!(),
    }
    .map_err(|error| format!("save failed in {} mode: {}", save_format.as_str(), error))?;

    let reparsed_core = rhwp::document_core::DocumentCore::from_bytes(&saved)
        .map_err(|error| format!("reparse after save failed: {}", error))?;
    let reparsed_report = reparsed_core.compatibility_report_data();
    if reparsed_report.edit_mode != rhwp::document_core::DocumentEditMode::EditableSafe {
        return Err(format!(
            "reparsed document became {} after {} save",
            reparsed_report.edit_mode.as_str(),
            save_format.as_str()
        ));
    }

    let reparsed_signature = capture_render_signature(&saved)
        .map_err(|error| format!("failed to capture reparsed render signature: {}", error))?;
    if original_signature.page_count != reparsed_signature.page_count {
        return Err(format!(
            "page count changed after {} save: {} -> {}",
            save_format.as_str(),
            original_signature.page_count,
            reparsed_signature.page_count
        ));
    }
    if original_signature.page_info_hashes != reparsed_signature.page_info_hashes {
        return Err(format!(
            "page info signature changed after {} save",
            save_format.as_str()
        ));
    }
    if original_signature.render_tree_hashes != reparsed_signature.render_tree_hashes {
        return Err(format!(
            "render tree signature changed after {} save",
            save_format.as_str()
        ));
    }

    Ok(())
}

fn capture_render_signature(data: &[u8]) -> Result<RenderSignature, String> {
    let doc = rhwp::wasm_api::HwpDocument::from_bytes(data)
        .map_err(|error| error.to_string())?;
    let page_count = doc.page_count();
    let page_limit = page_count.min(3);
    let mut page_info_hashes = Vec::new();
    let mut render_tree_hashes = Vec::new();

    for page_idx in 0..page_limit {
        let page_info = doc
            .get_page_info(page_idx)
            .map_err(|error| js_error_to_string(error))?;
        page_info_hashes.push(hash_text(&page_info));

        let render_tree = doc
            .get_page_render_tree(page_idx)
            .map_err(|error| js_error_to_string(error))?;
        render_tree_hashes.push(hash_text(&render_tree));
    }

    Ok(RenderSignature {
        page_count,
        page_info_hashes,
        render_tree_hashes,
    })
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn js_error_to_string(error: wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| format!("{:?}", error))
}

fn compat_generate_fixtures(args: &[String]) {
    let output_dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("compatibility-corpus/fixtures"));

    if let Err(error) = fs::create_dir_all(&output_dir) {
        eprintln!("error: failed to create {}: {}", output_dir.display(), error);
        std::process::exit(1);
    }

    let document_fixtures = vec![
        ("phase1-basic-text.hwpx", build_basic_text_fixture()),
        ("phase1-number-bullet.hwpx", build_bullet_fixture()),
        ("phase1-clickhere-field.hwpx", build_clickhere_field_fixture()),
        ("phase1-note-pair.hwpx", build_note_pair_fixture()),
        ("phase1-header-footer.hwpx", build_header_footer_fixture()),
        ("basic-shape.hwpx", build_basic_shape_fixture()),
        ("textbox-in-shape.hwpx", build_textbox_shape_fixture()),
        ("picture-caption.hwpx", build_picture_caption_fixture()),
        ("shape-group.hwpx", build_shape_group_fixture()),
    ];

    for (file_name, document) in document_fixtures {
        let output_path = output_dir.join(file_name);
        let bytes = match rhwp::serializer::serialize_hwpx(&document) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("error: failed to serialize {}: {}", output_path.display(), error);
                std::process::exit(1);
            }
        };

        if let Err(error) = fs::write(&output_path, &bytes) {
            eprintln!("error: failed to write {}: {}", output_path.display(), error);
            std::process::exit(1);
        }

        println!("generated {} ({} bytes)", output_path.display(), bytes.len());
    }

    let byte_fixtures = vec![
        (
            "unsupported-shape-clean-section.hwpx",
            build_unsupported_shape_fixture_bytes(),
        ),
        (
            "unsupported-shape-dirty-section.hwpx",
            build_unsupported_shape_fixture_bytes(),
        ),
    ];

    for (file_name, bytes) in byte_fixtures {
        let output_path = output_dir.join(file_name);
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("error: failed to build {}: {}", output_path.display(), error);
                std::process::exit(1);
            }
        };

        if let Err(error) = fs::write(&output_path, &bytes) {
            eprintln!("error: failed to write {}: {}", output_path.display(), error);
            std::process::exit(1);
        }

        println!("generated {} ({} bytes)", output_path.display(), bytes.len());
    }
}

fn build_basic_text_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    document
        .sections
        .push(fixture_section(vec![fixture_paragraph("Hello HWPX phase 1", 0)]));
    document
}

fn build_bullet_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    document.doc_info.para_shapes[0] = rhwp::model::style::ParaShape {
        head_type: rhwp::model::style::HeadType::Bullet,
        numbering_id: 1,
        ..Default::default()
    };
    document.doc_info.bullets.push(rhwp::model::style::Bullet {
        bullet_char: '*',
        width_adjust: 12,
        text_distance: 50,
        ..Default::default()
    });
    document
        .sections
        .push(fixture_section(vec![fixture_paragraph("Bullet item", 0)]));
    document
}

fn build_clickhere_field_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let text = "inside";
    let text_len = text.chars().count();
    let paragraph = rhwp::model::paragraph::Paragraph {
        text: text.to_string(),
        char_offsets: fixture_char_offsets(text),
        char_shapes: vec![rhwp::model::paragraph::CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        field_ranges: vec![rhwp::model::paragraph::FieldRange {
            start_char_idx: 0,
            end_char_idx: text_len,
            control_idx: 0,
        }],
        controls: vec![rhwp::model::control::Control::Field(
            rhwp::model::control::Field {
                field_type: rhwp::model::control::FieldType::ClickHere,
                command: rhwp::model::control::Field::build_clickhere_command(
                    "inside",
                    "",
                    "Sample",
                ),
                properties: 1,
                field_id: 77,
                ctrl_id: 7,
                ctrl_data_name: Some("Sample".to_string()),
                ..Default::default()
            },
        )],
        para_shape_id: 0,
        style_id: 0,
        char_count: text_len as u32 + 1,
        has_para_text: true,
        ..Default::default()
    };
    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_note_pair_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    document.doc_properties.footnote_start_num = 1;
    document.doc_properties.endnote_start_num = 1;

    let mut paragraph = fixture_paragraph("Body with notes", 0);
    paragraph.controls.push(rhwp::model::control::Control::Footnote(Box::new(
        rhwp::model::footnote::Footnote {
            number: 1,
            paragraphs: vec![fixture_paragraph("Footnote body", 0)],
        },
    )));
    paragraph.controls.push(rhwp::model::control::Control::Endnote(Box::new(
        rhwp::model::footnote::Endnote {
            number: 1,
            paragraphs: vec![fixture_paragraph("Endnote body", 0)],
        },
    )));

    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_header_footer_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let mut paragraph = fixture_paragraph("Body content", 0);
    paragraph.controls.push(rhwp::model::control::Control::Header(Box::new(
        rhwp::model::header_footer::Header {
            apply_to: rhwp::model::header_footer::HeaderFooterApply::Both,
            paragraphs: vec![fixture_paragraph("Header text", 0)],
            ..Default::default()
        },
    )));
    paragraph.controls.push(rhwp::model::control::Control::Footer(Box::new(
        rhwp::model::header_footer::Footer {
            apply_to: rhwp::model::header_footer::HeaderFooterApply::Both,
            paragraphs: vec![fixture_paragraph("Footer text", 0)],
            ..Default::default()
        },
    )));

    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_basic_shape_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let paragraph = fixture_shape_paragraph(vec![
        rhwp::model::control::Control::Shape(Box::new(rhwp::model::shape::ShapeObject::Line(
            fixture_line_shape(0x4100_0001),
        ))),
        rhwp::model::control::Control::Shape(Box::new(
            rhwp::model::shape::ShapeObject::Rectangle(fixture_rectangle_shape(
                0x4100_0002,
                None,
                None,
            )),
        )),
        rhwp::model::control::Control::Shape(Box::new(rhwp::model::shape::ShapeObject::Ellipse(
            fixture_ellipse_shape(0x4100_0003),
        ))),
        rhwp::model::control::Control::Shape(Box::new(rhwp::model::shape::ShapeObject::Arc(
            fixture_arc_shape(0x4100_0004),
        ))),
        rhwp::model::control::Control::Shape(Box::new(
            rhwp::model::shape::ShapeObject::Polygon(fixture_polygon_shape(0x4100_0005)),
        )),
    ]);
    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_textbox_shape_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let text_box = rhwp::model::shape::TextBox {
        list_attr: 0x20,
        vertical_align: rhwp::model::table::VerticalAlign::Top,
        margin_left: 283,
        margin_right: 283,
        margin_top: 283,
        margin_bottom: 283,
        max_width: 9600,
        raw_list_header_extra: vec![0; 13],
        paragraphs: vec![fixture_paragraph("Shape TextBox", 0)],
    };
    let paragraph = fixture_shape_paragraph(vec![rhwp::model::control::Control::Shape(Box::new(
        rhwp::model::shape::ShapeObject::Rectangle(fixture_rectangle_shape(
            0x4100_0006,
            Some(text_box),
            None,
        )),
    ))]);
    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_picture_caption_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let paragraph = fixture_shape_paragraph(vec![rhwp::model::control::Control::Picture(Box::new(
        fixture_picture(
            0x4100_0007,
            Some(fixture_caption("Picture caption")),
        ),
    ))]);
    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_shape_group_fixture() -> rhwp::model::document::Document {
    let mut document = new_fixture_document();
    let mut child_rect = rhwp::model::shape::ShapeObject::Rectangle(fixture_rectangle_shape(
        0x4100_0011,
        Some(rhwp::model::shape::TextBox {
            list_attr: 0x20,
            vertical_align: rhwp::model::table::VerticalAlign::Top,
            margin_left: 200,
            margin_right: 200,
            margin_top: 200,
            margin_bottom: 200,
            max_width: 6400,
            raw_list_header_extra: vec![0; 13],
            paragraphs: vec![fixture_paragraph("Grouped box", 0)],
        }),
        None,
    ));
    child_rect.common_mut().horizontal_offset = 400;
    child_rect.common_mut().vertical_offset = 300;
    match &mut child_rect {
        rhwp::model::shape::ShapeObject::Rectangle(rect) => {
            rect.drawing.shape_attr.offset_x = 400;
            rect.drawing.shape_attr.offset_y = 300;
            rect.drawing.shape_attr.group_level = 1;
        }
        _ => unreachable!(),
    }

    let mut child_picture = rhwp::model::shape::ShapeObject::Picture(Box::new(fixture_picture(
        0x4100_0012,
        Some(fixture_caption("Grouped picture")),
    )));
    child_picture.common_mut().horizontal_offset = 7600;
    child_picture.common_mut().vertical_offset = 1200;
    if let rhwp::model::shape::ShapeObject::Picture(picture) = &mut child_picture {
        picture.shape_attr.offset_x = 7600;
        picture.shape_attr.offset_y = 1200;
        picture.shape_attr.group_level = 1;
    }

    let group = rhwp::model::shape::GroupShape {
        common: fixture_shape_common(0x2463_6f6e, 18000, 9600, 0x4100_0010, 0, 0, 1),
        shape_attr: fixture_shape_component(0x2463_6f6e, 18000, 9600, 1),
        children: vec![child_rect, child_picture],
        caption: Some(fixture_caption("Wave 2 group")),
    };

    let paragraph = fixture_shape_paragraph(vec![rhwp::model::control::Control::Shape(Box::new(
        rhwp::model::shape::ShapeObject::Group(group),
    ))]);
    document.sections.push(fixture_section(vec![paragraph]));
    document
}

fn build_unsupported_shape_fixture_bytes() -> Result<Vec<u8>, String> {
    let base = rhwp::serializer::serialize_hwpx(&build_basic_shape_fixture())
        .map_err(|error| format!("serialize base unsupported-shape fixture: {}", error))?;
    rewrite_hwpx_section_xml(&base, |xml| {
        let updated = xml
            .replacen("<hp:line ", "<hp:connectLine ", 1)
            .replacen("</hp:line>", "</hp:connectLine>", 1);
        if updated == xml {
            Err("failed to inject unsupported connectLine payload".to_string())
        } else {
            Ok(updated)
        }
    })
}

fn rewrite_hwpx_section_xml<F>(bytes: &[u8], transform: F) -> Result<Vec<u8>, String>
where
    F: FnOnce(String) -> Result<String, String>,
{
    use std::io::{Cursor, Read, Write};

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("open hwpx archive: {}", error))?;
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut transformed = Some(transform);
        for index in 0..archive.len() {
            let mut file = archive
                .by_index(index)
                .map_err(|error| format!("read hwpx entry {}: {}", index, error))?;
            let name = file.name().to_string();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(|error| format!("read hwpx entry {} contents: {}", name, error))?;

            writer
                .start_file(name.clone(), options)
                .map_err(|error| format!("start rewritten entry {}: {}", name, error))?;
            if name == "Contents/section0.xml" {
                let xml = String::from_utf8(contents)
                    .map_err(|error| format!("decode {} as utf-8: {}", name, error))?;
                let updated = transformed
                    .take()
                    .expect("section transform used once")(xml)?;
                writer
                    .write_all(updated.as_bytes())
                    .map_err(|error| format!("write transformed {}: {}", name, error))?;
            } else {
                writer
                    .write_all(&contents)
                    .map_err(|error| format!("copy {}: {}", name, error))?;
            }
        }

        writer
            .finish()
            .map_err(|error| format!("finalize rewritten hwpx archive: {}", error))?;
    }

    Ok(output.into_inner())
}

fn fixture_shape_paragraph(
    controls: Vec<rhwp::model::control::Control>,
) -> rhwp::model::paragraph::Paragraph {
    rhwp::model::paragraph::Paragraph {
        char_count: 1,
        controls,
        ..rhwp::model::paragraph::Paragraph::new_empty()
    }
}

fn fixture_shape_common(
    ctrl_id: u32,
    width: u32,
    height: u32,
    instance_id: u32,
    horizontal_offset: u32,
    vertical_offset: u32,
    z_order: i32,
) -> rhwp::model::shape::CommonObjAttr {
    rhwp::model::shape::CommonObjAttr {
        ctrl_id,
        width,
        height,
        instance_id,
        horizontal_offset,
        vertical_offset,
        z_order,
        margin: rhwp::model::Padding::default(),
        treat_as_char: false,
        vert_rel_to: rhwp::model::shape::VertRelTo::Paper,
        vert_align: rhwp::model::shape::VertAlign::Top,
        horz_rel_to: rhwp::model::shape::HorzRelTo::Paper,
        horz_align: rhwp::model::shape::HorzAlign::Left,
        text_wrap: rhwp::model::shape::TextWrap::Square,
        width_criterion: rhwp::model::shape::SizeCriterion::Absolute,
        height_criterion: rhwp::model::shape::SizeCriterion::Absolute,
        description: format!("Wave 2 fixture {:08x}", instance_id),
        ..Default::default()
    }
}

fn fixture_shape_component(
    ctrl_id: u32,
    width: u32,
    height: u32,
    group_level: u16,
) -> rhwp::model::shape::ShapeComponentAttr {
    rhwp::model::shape::ShapeComponentAttr {
        ctrl_id,
        is_two_ctrl_id: true,
        original_width: width,
        original_height: height,
        current_width: width,
        current_height: height,
        group_level,
        local_file_version: 1,
        rotation_center: rhwp::model::Point {
            x: (width / 2) as i32,
            y: (height / 2) as i32,
        },
        render_sx: 1.0,
        render_sy: 1.0,
        ..Default::default()
    }
}

fn fixture_border_line() -> rhwp::model::style::ShapeBorderLine {
    rhwp::model::style::ShapeBorderLine {
        color: 0x000000,
        width: 33,
        attr: 0xD1000041,
        outline_style: 0,
    }
}

fn fixture_solid_fill() -> rhwp::model::style::Fill {
    rhwp::model::style::Fill {
        fill_type: rhwp::model::style::FillType::Solid,
        solid: Some(rhwp::model::style::SolidFill {
            background_color: 0x00F6F6F6,
            pattern_color: 0,
            pattern_type: -1,
        }),
        gradient: None,
        image: None,
        alpha: 0,
    }
}

fn fixture_drawing_attr(
    ctrl_id: u32,
    width: u32,
    height: u32,
    instance_id: u32,
    text_box: Option<rhwp::model::shape::TextBox>,
    caption: Option<rhwp::model::shape::Caption>,
) -> rhwp::model::shape::DrawingObjAttr {
    rhwp::model::shape::DrawingObjAttr {
        shape_attr: fixture_shape_component(ctrl_id, width, height, 0),
        border_line: fixture_border_line(),
        fill: fixture_solid_fill(),
        inst_id: (instance_id & 0x3FFF_FFFF) + 1,
        text_box,
        caption,
        ..Default::default()
    }
}

fn fixture_line_shape(instance_id: u32) -> rhwp::model::shape::LineShape {
    let width = 14400;
    let height = 7200;
    rhwp::model::shape::LineShape {
        common: fixture_shape_common(0x246c_696e, width, height, instance_id, 1200, 1200, 1),
        drawing: fixture_drawing_attr(0x246c_696e, width, height, instance_id, None, None),
        start: rhwp::model::Point { x: 0, y: 0 },
        end: rhwp::model::Point {
            x: width as i32,
            y: height as i32,
        },
        started_right_or_bottom: false,
        connector: None,
    }
}

fn fixture_rectangle_shape(
    instance_id: u32,
    text_box: Option<rhwp::model::shape::TextBox>,
    caption: Option<rhwp::model::shape::Caption>,
) -> rhwp::model::shape::RectangleShape {
    let width = 9600;
    let height = 6400;
    rhwp::model::shape::RectangleShape {
        common: fixture_shape_common(0x2472_6563, width, height, instance_id, 1800, 1800, 1),
        drawing: fixture_drawing_attr(0x2472_6563, width, height, instance_id, text_box, caption),
        round_rate: 20,
        x_coords: [0, width as i32, width as i32, 0],
        y_coords: [0, 0, height as i32, height as i32],
    }
}

fn fixture_ellipse_shape(instance_id: u32) -> rhwp::model::shape::EllipseShape {
    let width = 8800;
    let height = 6200;
    rhwp::model::shape::EllipseShape {
        common: fixture_shape_common(0x2465_6c6c, width, height, instance_id, 2200, 2200, 1),
        drawing: fixture_drawing_attr(0x2465_6c6c, width, height, instance_id, None, None),
        attr: 0,
        center: rhwp::model::Point {
            x: (width / 2) as i32,
            y: (height / 2) as i32,
        },
        axis1: rhwp::model::Point {
            x: width as i32,
            y: (height / 2) as i32,
        },
        axis2: rhwp::model::Point {
            x: (width / 2) as i32,
            y: height as i32,
        },
        start1: rhwp::model::Point {
            x: width as i32,
            y: (height / 2) as i32,
        },
        end1: rhwp::model::Point {
            x: width as i32,
            y: (height / 2) as i32,
        },
        start2: rhwp::model::Point {
            x: (width / 2) as i32,
            y: height as i32,
        },
        end2: rhwp::model::Point {
            x: (width / 2) as i32,
            y: height as i32,
        },
    }
}

fn fixture_arc_shape(instance_id: u32) -> rhwp::model::shape::ArcShape {
    let width = 9000;
    let height = 5600;
    rhwp::model::shape::ArcShape {
        common: fixture_shape_common(0x2461_7263, width, height, instance_id, 2600, 2600, 1),
        drawing: fixture_drawing_attr(0x2461_7263, width, height, instance_id, None, None),
        arc_type: 0,
        center: rhwp::model::Point {
            x: (width / 2) as i32,
            y: (height / 2) as i32,
        },
        axis1: rhwp::model::Point {
            x: width as i32,
            y: (height / 2) as i32,
        },
        axis2: rhwp::model::Point {
            x: (width / 2) as i32,
            y: height as i32,
        },
    }
}

fn fixture_polygon_shape(instance_id: u32) -> rhwp::model::shape::PolygonShape {
    let width = 9200;
    let height = 7000;
    rhwp::model::shape::PolygonShape {
        common: fixture_shape_common(0x2470_6f6c, width, height, instance_id, 3000, 3000, 1),
        drawing: fixture_drawing_attr(0x2470_6f6c, width, height, instance_id, None, None),
        points: vec![
            rhwp::model::Point { x: width as i32 / 2, y: 0 },
            rhwp::model::Point { x: width as i32, y: height as i32 / 3 },
            rhwp::model::Point { x: (width as i32 * 3) / 4, y: height as i32 },
            rhwp::model::Point { x: width as i32 / 4, y: height as i32 },
            rhwp::model::Point { x: 0, y: height as i32 / 3 },
        ],
    }
}

fn fixture_picture(
    instance_id: u32,
    caption: Option<rhwp::model::shape::Caption>,
) -> rhwp::model::image::Picture {
    let width = 7800;
    let height = 5200;
    rhwp::model::image::Picture {
        common: fixture_shape_common(0x2470_6963, width, height, instance_id, 1500, 1500, 1),
        shape_attr: fixture_shape_component(0x2470_6963, width, height, 0),
        border_color: 0,
        border_width: 33,
        border_attr: fixture_border_line(),
        border_x: [0, width as i32, width as i32, 0],
        border_y: [0, 0, height as i32, height as i32],
        crop: rhwp::model::image::CropInfo {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        },
        padding: rhwp::model::Padding::default(),
        image_attr: rhwp::model::image::ImageAttr {
            brightness: 0,
            contrast: 0,
            effect: rhwp::model::image::ImageEffect::RealPic,
            bin_data_id: 0,
        },
        border_opacity: 0,
        instance_id,
        raw_picture_extra: Vec::new(),
        caption,
    }
}

fn fixture_caption(text: &str) -> rhwp::model::shape::Caption {
    rhwp::model::shape::Caption {
        direction: rhwp::model::shape::CaptionDirection::Bottom,
        vert_align: rhwp::model::shape::CaptionVertAlign::Top,
        width: 9000,
        spacing: 180,
        max_width: 9000,
        include_margin: false,
        paragraphs: vec![fixture_paragraph(text, 0)],
    }
}

fn new_fixture_document() -> rhwp::model::document::Document {
    let mut document = rhwp::model::document::Document::default();
    document.doc_info.font_faces = vec![Vec::new(); 7];
    document.doc_info.char_shapes.push(rhwp::model::style::CharShape::default());
    document
        .doc_info
        .para_shapes
        .push(rhwp::model::style::ParaShape::default());
    document
}

fn fixture_section(paragraphs: Vec<rhwp::model::paragraph::Paragraph>) -> rhwp::model::document::Section {
    rhwp::model::document::Section {
        section_def: rhwp::model::document::SectionDef {
            page_def: fixture_page_def(),
            ..Default::default()
        },
        paragraphs,
        ..Default::default()
    }
}

fn fixture_page_def() -> rhwp::model::page::PageDef {
    rhwp::model::page::PageDef {
        width: 59528,
        height: 84188,
        margin_left: 8504,
        margin_right: 8504,
        margin_top: 5669,
        margin_bottom: 4252,
        margin_header: 4252,
        margin_footer: 4252,
        margin_gutter: 0,
        ..Default::default()
    }
}

fn fixture_paragraph(text: &str, para_shape_id: u16) -> rhwp::model::paragraph::Paragraph {
    let text_len = text.chars().count();
    rhwp::model::paragraph::Paragraph {
        text: text.to_string(),
        char_offsets: fixture_char_offsets(text),
        char_shapes: vec![rhwp::model::paragraph::CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        para_shape_id,
        style_id: 0,
        char_count: text_len as u32 + 1,
        has_para_text: true,
        ..Default::default()
    }
}

fn fixture_char_offsets(text: &str) -> Vec<u32> {
    let mut offsets = Vec::new();
    let mut current = 0u32;
    for ch in text.chars() {
        offsets.push(current);
        current += if (ch as u32) > 0xFFFF { 2 } else { 1 };
    }
    offsets
}
