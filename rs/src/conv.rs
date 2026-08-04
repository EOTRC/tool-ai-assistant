

use regex::Regex;
use std::fs;
use std::path::Path;

pub const CHUNK_SIZE: usize = 1000;
pub const CHUNK_OVERLAP: usize = 150;

pub const TEXT_EXTS: &[&str] = &[
    ".txt", ".md", ".markdown", ".csv", ".json", ".xml", ".html", ".htm", ".log", ".ini",
    ".cfg", ".yaml", ".yml", ".py", ".js", ".ts", ".jsx", ".tsx", ".css", ".jsonl", ".rst",
    ".tex", ".sql", ".sh", ".bat", ".ps1",
];
pub const IMAGE_EXTS: &[&str] = &[".png", ".jpg", ".jpeg", ".bmp", ".webp", ".gif"];
pub const OFFICE_EXTS: &[&str] = &[
    ".doc", ".docx", ".rtf", ".odt", ".xls", ".xlsx", ".ods", ".ppt", ".pptx", ".odp",
];

fn has_ext(path: &str, exts: &[&str]) -> bool {
    let e = Path::new(path)
        .extension()
        .map(|x| format!(".{}", x.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    exts.contains(&e.as_str())
}


fn read_text_file(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        return Ok(s.trim_start_matches('\u{feff}').to_string());
    }
    
    if let Some(s) = decode_cp1251(&bytes) {
        return Ok(s);
    }
    
    Ok(bytes.iter().map(|&b| b as char).collect())
}


fn decode_cp1251(bytes: &[u8]) -> Option<String> {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let c = match b {
            0x00..=0x7f => b as char,
            0x80 => 'Ђ',
            0x81 => 'Ѓ',
            0x82 => '‚',
            0x83 => 'ѓ',
            0x84 => '„',
            0x85 => '…',
            0x86 => '†',
            0x87 => '‡',
            0x88 => '€',
            0x89 => '‰',
            0x8a => 'Љ',
            0x8b => '‹',
            0x8c => 'Њ',
            0x8d => 'Ќ',
            0x8e => 'Ћ',
            0x8f => 'Џ',
            0x90 => 'ђ',
            0x91 => '‘',
            0x92 => '’',
            0x93 => '“',
            0x94 => '”',
            0x95 => '•',
            0x96 => '–',
            0x97 => '—',
            0x98 => '˜',
            0x99 => '™',
            0x9a => 'љ',
            0x9b => '›',
            0x9c => 'њ',
            0x9d => 'ќ',
            0x9e => 'ћ',
            0x9f => 'џ',
            0xa0 => '\u{a0}',
            0xa1 => 'Ў',
            0xa2 => 'ў',
            0xa3 => 'Ј',
            0xa4 => '¤',
            0xa5 => 'Ґ',
            0xa6 => '¦',
            0xa7 => '§',
            0xa8 => 'Ё',
            0xa9 => '©',
            0xaa => 'Є',
            0xab => '«',
            0xac => '¬',
            0xad => '\u{ad}',
            0xae => '®',
            0xaf => 'Ї',
            0xb0 => '°',
            0xb1 => '±',
            0xb2 => 'І',
            0xb3 => 'і',
            0xb4 => 'ґ',
            0xb5 => 'µ',
            0xb6 => '¶',
            0xb7 => '·',
            0xb8 => 'ё',
            0xb9 => '№',
            0xba => 'є',
            0xbb => '»',
            0xbc => 'ј',
            0xbd => 'Ѕ',
            0xbe => 'ѕ',
            0xbf => 'ї',
            0xc0..=0xff => char::from_u32(0x0410 + (b as u32 - 0xc0)).unwrap_or('?'),
        };
        out.push(c);
    }
    Some(out)
}


pub fn html_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = s[i..].find(';') {
                let ent = &s[i + 1..i + semi];
                let decoded: Option<String> = if let Some(hex) = ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).map(String::from)
                } else if let Some(dec) = ent.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32).map(String::from)
                } else {
                    let named = match ent {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        "nbsp" => " ",
                        "mdash" => "—",
                        "ndash" => "–",
                        "laquo" => "«",
                        "raquo" => "»",
                        "hellip" => "…",
                        _ => "",
                    };
                    if named.is_empty() {
                        None
                    } else {
                        Some(named.to_string())
                    }
                };
                match decoded {
                    Some(d) => {
                        out.push_str(&d);
                        i = i + semi + 1;
                        continue;
                    }
                    None => {}
                }
            }
        }
        out.push(s[i..].chars().next().unwrap());
        i += 1;
    }
    out
}


fn xml_tag_text(xml: &str, tags: &[&str]) -> String {
    let mut parts = Vec::new();
    for tag in tags {
        let re = Regex::new(&format!(r"<{}[^>]*>(.*?)</{}>", tag, tag)).unwrap();
        for cap in re.captures_iter(xml) {
            let t = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if !t.is_empty() {
                parts.push(html_unescape(t));
            }
        }
    }
    parts.join(" ")
}


fn xml_strip(xml: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    let t = re.replace_all(xml, " ");
    html_unescape(&t)
}

fn convert_docx(path: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
    let xml = read_zip_entry(&mut z, "word/document.xml")?;
    Ok(xml_tag_text(&xml, &["w:t"]))
}

fn convert_xlsx(path: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
    let names: Vec<String> = (0..z.len())
        .filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let shared: Vec<String> = if names.iter().any(|n| n == "xl/sharedStrings.xml") {
        let xml = read_zip_entry(&mut z, "xl/sharedStrings.xml")?;
        xml_tag_text(&xml, &["t"]).split(' ').map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };
    let mut sheet_names: Vec<String> = names
        .iter()
        .filter(|n| Regex::new(r"^xl/worksheets/sheet\d+\.xml$").unwrap().is_match(n))
        .cloned()
        .collect();
    sheet_names.sort();
    let cell_re = Regex::new(r#"<c[^>]*?t="s"[^>]*?>(?:<v>([^<]*)</v>)?</c>|<c[^>]*?>(?:<v>([^<]*)</v>)?</c>"#).unwrap();
    let mut rows = Vec::new();
    for name in &sheet_names {
        let xml = read_zip_entry(&mut z, name)?;
        let mut line = Vec::new();
        for cap in cell_re.captures_iter(&xml) {
            let v = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str()).unwrap_or("");
            if v.is_empty() {
                continue;
            }
            if let Ok(idx) = v.parse::<usize>() {
                if let Some(s) = shared.get(idx) {
                    line.push(s.clone());
                    continue;
                }
            }
            line.push(v.to_string());
        }
        if !line.is_empty() {
            rows.push(line.join(" | "));
        }
    }
    Ok(rows.join("\n"))
}

fn convert_pptx(path: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
    let names: Vec<String> = (0..z.len())
        .filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let mut slides: Vec<String> = names
        .iter()
        .filter(|n| Regex::new(r"^ppt/slides/slide\d+\.xml$").unwrap().is_match(n))
        .cloned()
        .collect();
    slides.sort();
    let mut out = Vec::new();
    for s in &slides {
        let xml = read_zip_entry(&mut z, s)?;
        let t = xml_tag_text(&xml, &["a:t"]);
        if !t.is_empty() {
            out.push(t);
        }
    }
    Ok(out.join("\n\n"))
}

fn convert_odt(path: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
    let xml = read_zip_entry(&mut z, "content.xml")?;
    Ok(xml_strip(&xml))
}

fn convert_epub(path: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
    let names: Vec<String> = (0..z.len())
        .filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let script_re = Regex::new(r"(?s)<script.*?</script>").unwrap();
    let style_re = Regex::new(r"(?s)<style.*?</style>").unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    let ws_re = Regex::new(r"\s+").unwrap();
    let mut parts = Vec::new();
    for name in names.iter().filter(|n| {
        let l = n.to_lowercase();
        l.ends_with(".xhtml") || l.ends_with(".html") || l.ends_with(".htm")
    }) {
        let mut data = read_zip_entry(&mut z, name)?;
        data = script_re.replace_all(&data, " ").to_string();
        data = style_re.replace_all(&data, " ").to_string();
        data = tag_re.replace_all(&data, " ").to_string();
        data = html_unescape(&data);
        data = ws_re.replace_all(&data, " ").to_string();
        let t = data.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    Ok(parts.join("\n\n"))
}

fn convert_pdf(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(t) => Ok(t),
        Err(e) => Err(format!("pdf: {}", e)),
    }
}



fn convert_old_binary(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let mut out = String::new();
    let mut i = 0;
    let mut cur = String::new();
    fn is_text16(b0: u8, b1: u8) -> bool {
        if b1 == 0 {
            let b = b0;
            b == b'\t' || b == b'\r' || b == b'\n' || (0x20..=0x7e).contains(&b)
        } else {
            let cp = (b0 as u32) | ((b1 as u32) << 8);
            (0x0410..=0x044f).contains(&cp)
                || (0x0400..=0x040f).contains(&cp)
                || (0x20..=0x7e).contains(&cp)
        }
    }
    while i + 1 < bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        if is_text16(b0, b1) {
            let cp = (b0 as u32) | ((b1 as u32) << 8);
            cur.push(char::from_u32(cp).unwrap_or(' '));
            i += 2;
        } else {
            if cur.len() >= 4 {
                out.push_str(&cur);
                out.push('\n');
            }
            cur.clear();
            i += 1;
        }
    }
    if cur.len() >= 4 {
        out.push_str(&cur);
        out.push('\n');
    }
    if out.trim().is_empty() {
        Err("Не удалось извлечь текст (старый бинарный формат)".to_string())
    } else {
        Ok(out)
    }
}


fn read_zip_entry(z: &mut zip::ZipArchive<fs::File>, name: &str) -> Result<String, String> {
    let mut f = z.by_name(name).map_err(|_| format!("в архиве нет {}", name))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut bytes).map_err(|e| e.to_string())?;
    read_text_file_bytes(&bytes)
}

fn read_text_file_bytes(bytes: &[u8]) -> Result<String, String> {
    if let Ok(s) = String::from_utf8(bytes.to_vec()) {
        return Ok(s.trim_start_matches('\u{feff}').to_string());
    }
    if let Some(s) = decode_cp1251(bytes) {
        return Ok(s);
    }
    Ok(bytes.iter().map(|&b| b as char).collect())
}


pub fn convert(path: &str) -> Result<String, String> {
    let ext = Path::new(path)
        .extension()
        .map(|x| format!(".{}", x.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    if has_ext(path, TEXT_EXTS) {
        return read_text_file(path);
    }
    if ext == ".pdf" {
        return convert_pdf(path);
    }
    if ext == ".docx" {
        return convert_docx(path);
    }
    if ext == ".xlsx" {
        return convert_xlsx(path);
    }
    if ext == ".pptx" {
        return convert_pptx(path);
    }
    if ext == ".epub" {
        return convert_epub(path);
    }
    if ext == ".odt" || ext == ".ods" || ext == ".odp" {
        return convert_odt(path);
    }
    if ext == ".doc" || ext == ".xls" || ext == ".ppt" {
        return convert_old_binary(path);
    }
    if has_ext(path, IMAGE_EXTS) {
        return Ok(format!("[изображение: {} — используй ask-image]", path));
    }
    Err(format!("Неподдерживаемый формат: {}", ext))
}


pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let re = Regex::new(r"\n{3,}").unwrap();
    let text = re.replace_all(text, "\n\n");
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= chunk_size {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let mut end = (start + chunk_size).min(chars.len());
        if end < chars.len() {
            
            if let Some(pos) = (start + chunk_size / 2..end).rev().find(|&i| chars[i] == '\n') {
                end = pos;
            }
        }
        let c: String = chars[start..end].iter().collect();
        chunks.push(c);
        let next = end.saturating_sub(overlap).max(start + chunk_size / 2);
        start = next;
    }
    chunks
}


pub fn is_cyrillic(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let cyr = letters
        .iter()
        .filter(|c| ('\u{0400}'..='\u{04ff}').contains(c))
        .count();
    (cyr as f64 / letters.len() as f64) > 0.3
}
