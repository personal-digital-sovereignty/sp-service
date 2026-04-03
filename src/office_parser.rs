use std::path::Path;
use std::fs::File;
use std::io::Read;

pub fn parse_file(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    let ext = p.extension().unwrap_or_default().to_string_lossy().to_lowercase();

    match ext.as_str() {
        "docx" => parse_docx(path),
        "pptx" | "odt" | "odp" => parse_ooxml_generic(path, ext.as_str()),
        "xlsx" | "ods" => parse_spreadsheet(path),
        "csv" => parse_csv(path),
        _ => {
            // Default text fallback
            std::fs::read_to_string(path).map_err(|e| e.to_string())
        }
    }
}

fn parse_docx(path: &str) -> Result<String, String> {
    // DOCX is a zip file, text is in word/document.xml
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;
    
    let mut xml_file = match archive.by_name("word/document.xml") {
        Ok(f) => f,
        Err(_) => return Err("Invalid DOCX format: missing word/document.xml".to_string()),
    };
    
    let mut xml_content = String::new();
    xml_file.read_to_string(&mut xml_content).map_err(|e| format!("Failed to read XML: {}", e))?;
    
    extract_text_from_xml(&xml_content, b"w:t")
}

fn parse_ooxml_generic(path: &str, ext: &str) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;
    
    let target_file = match ext {
        "pptx" => "ppt/slides/slide", // Requires iterating through slide*.xml, handled below
        "odt" | "odp" => "content.xml",
        _ => return Err("Unsupported OOXML variant".to_string()),
    };

    if ext == "pptx" {
        let mut full_text = String::new();
        // PPTX splits content across multiple slide files
        for i in 1..999 {
            let slide_name = format!("ppt/slides/slide{}.xml", i);
            if let Ok(mut xml_file) = archive.by_name(&slide_name) {
                let mut xml_content = String::new();
                if xml_file.read_to_string(&mut xml_content).is_ok() {
                    if let Ok(slide_text) = extract_text_from_xml(&xml_content, b"a:t") {
                        full_text.push_str(&slide_text);
                        full_text.push_str("\n\n---\n\n"); // Slide separator
                    }
                }
            } else {
                break; // No more slides
            }
        }
        return Ok(full_text);
    }

    // For ODT/ODP
    let mut xml_file = match archive.by_name(target_file) {
        Ok(f) => f,
        Err(_) => return Err(format!("Invalid file format: missing {}", target_file)),
    };
    
    let mut xml_content = String::new();
    xml_file.read_to_string(&mut xml_content).map_err(|e| format!("Failed to read XML: {}", e))?;
    
    extract_text_from_xml(&xml_content, b"text:p") // ODF uses text:p for paragraphs
}

fn extract_text_from_xml(xml: &str, target_tag: &[u8]) -> Result<String, String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut txt = String::new();
    let mut in_target = false;
    let mut buf = Vec::new();

    // Table state trackers for DOCX
    let mut in_table = false;
    let mut is_first_row = false;
    let mut columns = 0;

    // Formatting state trackers (DOCX)
    let mut is_bold = false;
    let mut is_italic = false;
    let mut is_underline = false;
    let mut heading_level = 0;
    let mut is_list = false;
    let mut new_paragraph = true;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == target_tag {
                    in_target = true;
                } else {
                    match name.as_ref() {
                        b"w:p" | b"a:p" | b"text:p" => {
                            heading_level = 0;
                            is_list = false;
                            new_paragraph = true;
                            if !in_table {
                                txt.push_str("\n");
                            } else {
                                txt.push_str(" ");
                            }
                        },
                        b"w:r" | b"a:r" => {
                            is_bold = false;
                            is_italic = false;
                            is_underline = false;
                        },
                        b"w:numPr" => {
                            is_list = true;
                        },
                        b"w:tbl" => {
                            in_table = true;
                            is_first_row = true;
                            txt.push_str("\n\n");
                        },
                        b"w:tr" => {
                            columns = 0;
                            txt.push_str("| ");
                        },
                        b"w:tc" => {
                            columns += 1;
                        },
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                        b"w:br" | b"a:br" => {
                            txt.push_str("\n");
                        },
                        b"w:b" | b"a:b" => is_bold = true,
                        b"w:i" | b"a:i" => is_italic = true,
                        b"w:u" | b"a:u" => is_underline = true,
                        b"w:pStyle" => {
                            for attr in e.attributes().filter_map(|a| a.ok()) {
                                if attr.key.as_ref() == b"w:val" {
                                    let val = String::from_utf8_lossy(&attr.value).to_lowercase();
                                    if val.starts_with("heading") || val.starts_with("ttulo") {
                                        let lvl_str = val.replace("heading", "").replace("ttulo", "");
                                        if let Ok(lvl) = lvl_str.parse::<usize>() {
                                            heading_level = lvl;
                                        }
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == target_tag {
                    in_target = false;
                } else {
                    match name.as_ref() {
                        b"w:p" | b"a:p" | b"text:p" => {
                            if !in_table {
                                txt.push_str("\n");
                            }
                            new_paragraph = false;
                        },
                        b"w:tbl" => {
                            in_table = false;
                            txt.push_str("\n\n");
                        },
                        b"w:tr" => {
                            txt.push_str("\n");
                            if is_first_row && in_table {
                                txt.push_str("|");
                                for _ in 0..columns {
                                    txt.push_str("---|");
                                }
                                txt.push_str("\n");
                                is_first_row = false;
                            }
                        },
                        b"w:tc" => {
                            txt.push_str(" | ");
                        },
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_target {
                    if let Ok(t) = e.unescape() {
                        let text = t.into_owned();
                        if !text.is_empty() {
                            if new_paragraph {
                                if heading_level > 0 && heading_level <= 6 {
                                    txt.push_str(&"#".repeat(heading_level));
                                    txt.push_str(" ");
                                } else if is_list {
                                    txt.push_str("- ");
                                }
                                new_paragraph = false;
                            }

                            if is_bold { txt.push_str("**"); }
                            if is_italic { txt.push_str("*"); }
                            if is_underline { txt.push_str("<u>"); }
                            
                            txt.push_str(&text);
                            
                            if is_underline { txt.push_str("</u>"); }
                            if is_italic { txt.push_str("*"); }
                            if is_bold { txt.push_str("**"); }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => (),
        }
        buf.clear();
    }

    Ok(txt.trim().to_string())
}

#[derive(Debug)]
struct SubGrid {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
}

fn find_chartable_subgrid(matrix: &[Vec<String>]) -> Option<SubGrid> {
    if matrix.is_empty() { return None; }
    
    // Scan for the first row that looks like numbers preceded by a label
    let mut best_grid: Option<SubGrid> = None;
    let mut best_size = 0;

    for i in 0..matrix.len() {
        if matrix[i].is_empty() { continue; }
        for j in 0..matrix[i].len() {
            // Is this cell a number? Wait, we want to find the top-left of the numeric block.
            // A numeric block typically has its first number at (start_row + 1, start_col + 1).
            // Meaning (start_row, start_col+1..) are series labels, and (start_row+1.., start_col) are category labels.
            
            // Let's assume (i, j) is the anchor: the top-left EMPTY or String cell before the headers.
            // So numeric data starts at (i+1, j+1).
            let data_r = i + 1;
            let data_c = j + 1;
            
            if data_r < matrix.len() && data_c < matrix[data_r].len() {
                if matrix[data_r][data_c].trim().parse::<f64>().is_ok() {
                    // Looks like numeric data! Let's trace how far it goes.
                    let mut max_r = data_r;
                    let mut max_c = data_c;
                    
                    // Trace width
                    while max_c < matrix[data_r].len() && matrix[data_r][max_c].trim().parse::<f64>().is_ok() {
                        max_c += 1;
                    }
                    
                    // Trace height
                    while max_r < matrix.len() {
                        let mut row_ok = true;
                        // Check if the whole width is numeric
                        for c in data_c..max_c {
                            if c >= matrix[max_r].len() || matrix[max_r][c].trim().parse::<f64>().is_err() {
                                row_ok = false;
                                break;
                            }
                        }
                        if !row_ok { break; }
                        max_r += 1;
                    }
                    
                    let rows = max_r - data_r;
                    let cols = max_c - data_c;
                    
                    if rows >= 1 && cols >= 1 {
                        let size = rows * cols;
                        if size > best_size {
                            best_size = size;
                            best_grid = Some(SubGrid {
                                row_start: i,
                                row_end: max_r - 1,
                                col_start: j,
                                col_end: max_c - 1,
                            });
                        }
                    }
                }
            }
        }
    }
    
    if best_size >= 2 { best_grid } else { None }
}

fn extract_subgrid_matrix(matrix: &[Vec<String>], grid: &SubGrid) -> Vec<Vec<String>> {
    let mut sub = Vec::new();
    for r in grid.row_start..=grid.row_end {
        if r < matrix.len() {
            let mut sub_row = Vec::new();
            for c in grid.col_start..=grid.col_end {
                if c < matrix[r].len() {
                    sub_row.push(matrix[r][c].clone());
                } else {
                    sub_row.push(String::new());
                }
            }
            sub.push(sub_row);
        }
    }
    sub
}

fn generate_svg_bar_chart(matrix: &[Vec<String>], title: &str) -> String {
    let num_categories = matrix.len() - 1;
    let num_series = matrix[0].len() - 1;
    
    let mut max_val: f64 = 0.0;
    for i in 1..=num_categories {
        for j in 1..=num_series {
            if j < matrix[i].len() {
                if let Ok(val) = matrix[i][j].trim().parse::<f64>() {
                    if val > max_val { max_val = val; }
                }
            }
        }
    }
    if max_val == 0.0 { max_val = 1.0; } 
    
    let width = 800;
    let height = 400;
    let pad_left = 60;
    let pad_bottom = 60;
    let pad_top = 60;
    let pad_right = 160; 
    
    let chart_w = width - pad_left - pad_right;
    let chart_h = height - pad_top - pad_bottom;
    
    let mut svg = String::new();
    svg.push_str(&format!(r##"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg" style="background:#0f172a; font-family:sans-serif; border-radius: 8px;">"##, width, height));
    
    svg.push_str(r##"<rect width="100%" height="100%" fill="#0f172a" rx="8" />"##);
    svg.push_str(&format!(r##"<text x="{}" y="35" fill="#f8fafc" font-size="20" font-weight="bold" text-anchor="middle">{}</text>"##, pad_left + chart_w/2, title));
    
    let num_ticks = 5;
    for t in 0..=num_ticks {
        let val = max_val * (t as f64) / (num_ticks as f64);
        let y = pad_top + chart_h - (chart_h * t / num_ticks);
        svg.push_str(&format!(r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#334155" stroke-dasharray="4" />"##, pad_left, y, pad_left + chart_w, y));
        svg.push_str(&format!(r##"<text x="{}" y="{}" fill="#94a3b8" font-size="12" text-anchor="end" dominant-baseline="middle">{}</text>"##, pad_left - 10, y, val.round()));
    }
    
    svg.push_str(&format!(r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#94a3b8" stroke-width="2" />"##, pad_left, pad_top + chart_h, pad_left + chart_w, pad_top + chart_h));
    
    let colors = ["#3b82f6", "#10b981", "#f59e0b", "#ec4899", "#8b5cf6", "#14b8a6", "#f43f5e", "#0ea5e9", "#eab308"];
    let group_width = chart_w / num_categories;
    let bar_width = (group_width - 20) / num_series;
    
    for i in 1..=num_categories {
        let group_x = pad_left + (i - 1) * group_width;
        let cat_name = &matrix[i][0];
        
        let center_x = group_x + (group_width / 2);
        svg.push_str(&format!(r##"<text x="{}" y="{}" fill="#cbd5e1" font-size="12" text-anchor="middle">{}</text>"##, center_x, pad_top + chart_h + 20, escape_xml(cat_name)));
        
        for j in 1..=num_series {
            if j < matrix[i].len() {
                if let Ok(val) = matrix[i][j].trim().parse::<f64>() {
                    let bar_h = ((val / max_val) * (chart_h as f64)) as usize;
                    let bar_x = group_x + 10 + (j - 1) * bar_width;
                    let bar_y = pad_top + chart_h - bar_h;
                    let color = colors[(j - 1) % colors.len()];
                    
                    svg.push_str(&format!(r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" rx="3" />"##, bar_x, bar_y, bar_width.saturating_sub(2), bar_h, color));
                }
            }
        }
    }
    
    let legend_x = pad_left + chart_w + 20;
    for j in 1..=num_series {
        let series_name = &matrix[0][j];
        let color = colors[(j - 1) % colors.len()];
        let leg_y = pad_top + (j - 1) * 25;
        
        svg.push_str(&format!(r##"<rect x="{}" y="{}" width="15" height="15" fill="{}" rx="3" />"##, legend_x, leg_y - 12, color));
        svg.push_str(&format!(r##"<text x="{}" y="{}" fill="#cbd5e1" font-size="12" dominant-baseline="middle">{}</text>"##, legend_x + 25, leg_y - 4, escape_xml(series_name)));
    }
    
    svg.push_str("</svg>");
    svg
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn parse_spreadsheet(path: &str) -> Result<String, String> {
    use calamine::{open_workbook_auto, Reader, Data};
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let mut workbook = open_workbook_auto(path).map_err(|e| format!("Failed to open workbook: {}", e))?;
    let mut result_md = String::new();

    let sheet_names = workbook.sheet_names().to_owned();

    for sheet in sheet_names {
        result_md.push_str(&format!("## Sheet: {}\n\n", sheet));
        if let Ok(range) = workbook.worksheet_range(&sheet) {
            let mut matrix: Vec<Vec<String>> = Vec::new();
            
            for row in range.rows() {
                let text_row: Vec<String> = row.iter().map(|c| match c {
                    Data::Empty => String::new(),
                    Data::String(s) => s.replace('\n', " "),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::DateTime(_) | Data::DateTimeIso(_) | Data::DurationIso(_) => format!("{}", c),
                    Data::Bool(b) => b.to_string(),
                    Data::Error(_) => String::new(),
                }).collect();
                matrix.push(text_row);
            }

            if let Some(subgrid) = find_chartable_subgrid(&matrix) {
                let chart_matrix = extract_subgrid_matrix(&matrix, &subgrid);
                let mut title = sheet.clone();
                
                // Attempt to find a title from rows above the chart
                for r in (0..subgrid.row_start).rev() {
                    let mut found = false;
                    for c in 0..matrix[r].len() {
                        let cell = matrix[r][c].trim();
                        if !cell.is_empty() {
                            title = cell.to_string();
                            found = true;
                            break;
                        }
                    }
                    if found { break; }
                }

                let svg = generate_svg_bar_chart(&chart_matrix, &title);
                let b64 = STANDARD.encode(svg.as_bytes());
                result_md.push_str(&format!(":::CHART_BASE64:{}:::\n\n", b64));
            }

            if !matrix.is_empty() {
                // Determine bounding box for meaningful data to print as a table
                let mut first_row = 0;
                while first_row < matrix.len() && matrix[first_row].iter().all(|c| c.trim().is_empty()) {
                    first_row += 1;
                }
                
                if first_row < matrix.len() {
                    let mut first_col = 0;
                    for c in 0..matrix[first_row].len() {
                        let mut empty_col = true;
                        for r in first_row..matrix.len() {
                            if c < matrix[r].len() && !matrix[r][c].trim().is_empty() {
                                empty_col = false;
                                break;
                            }
                        }
                        if empty_col { first_col += 1; } else { break; }
                    }
                    
                    let headers = &matrix[first_row];
                    let header_row = headers.iter().skip(first_col).map(|c| c.replace('|', "\\|")).collect::<Vec<String>>().join(" | ");
                    result_md.push_str(&format!("| {} |\n", header_row));
                    let sep_row = headers.iter().skip(first_col).map(|_| "---".to_string()).collect::<Vec<String>>().join(" | ");
                    result_md.push_str(&format!("| {} |\n", sep_row));

                    for r in matrix.iter().skip(first_row + 1) {
                        // skip entirely empty rows
                        if r.iter().all(|c| c.trim().is_empty()) { continue; }
                        let md_row = r.iter().skip(first_col).map(|c| c.replace('|', "\\|")).collect::<Vec<String>>().join(" | ");
                        result_md.push_str(&format!("| {} |\n", md_row));
                    }
                }
            }
            result_md.push_str("\n");
        }
    }

    Ok(result_md)
}

fn parse_csv(path: &str) -> Result<String, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to open CSV: {}", e))?;

    let mut result_md = String::new();

    if let Ok(headers) = rdr.headers() {
        let header_row = headers.iter().map(|s: &str| s.replace('|', "\\|")).collect::<Vec<String>>().join(" | ");
        result_md.push_str(&format!("| {} |\n", header_row));
        let sep_row = headers.iter().map(|_| "---".to_string()).collect::<Vec<String>>().join(" | ");
        result_md.push_str(&format!("| {} |\n", sep_row));
    }

    for result in rdr.records() {
        if let Ok(record) = result {
            let md_row = record.iter().map(|s: &str| s.replace('\n', " ").replace('|', "\\|")).collect::<Vec<String>>().join(" | ");
            result_md.push_str(&format!("| {} |\n", md_row));
        }
    }

    Ok(result_md)
}
