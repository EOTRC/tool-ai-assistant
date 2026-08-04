



use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::conv;
use crate::ollama;
use crate::rag;


fn split_flags(args: &[String], flags: &[&str]) -> (HashMap<String, String>, Vec<String>) {
    let mut map = HashMap::new();
    let mut pos = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a.starts_with("--") && flags.contains(&a.as_str()) {
            if let Some(v) = args.get(i + 1) {
                map.insert(a.clone(), v.clone());
                i += 2;
                continue;
            }
        }
        pos.push(a.clone());
        i += 1;
    }
    (map, pos)
}

fn options_for(s: &crate::Settings, model: &str) -> Value {
    let mut opts = json!({
        "num_ctx": 8192,
        "temperature": 0.1,
    });
    crate::set_num_gpu(&mut opts, s, model);
    opts
}


pub fn dispatch(s: &crate::Settings, cmd: &str, args: &[String]) {
    let r = match cmd {
        "convert" => convert_cmd(s, args),
        "index" => index_cmd(s, args),
        "ask" => ask_cmd(s, args),
        "ask-file" => ask_file_cmd(s, args),
        "code" => code_cmd(s, args),
        "ask-image" => ask_image_cmd(s, args),
        "summarize" => summarize_cmd(s, args),
        "translate" => translate_cmd(s, args),
        "search" => search_cmd(s, args),
        "clip" => clip_cmd(s, args),
        "screen" => screen_cmd(s, args),
        _ => {
            eprintln!("Неизвестная команда: {}", cmd);
            return;
        }
    };
    if let Err(e) = r {
        eprintln!("{}", e);
    }
}

fn convert_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let _ = s;
    if args.iter().any(|a| a == "--list" || a == "-l") {
        println!("Поддерживаемые форматы:");
        println!("  Текст: {}", conv::TEXT_EXTS.join(", "));
        println!("  Office: {}", conv::OFFICE_EXTS.join(", "));
        println!("  Прочее: pdf, epub");
        println!("  Изображения: {} (только через ask-image)", conv::IMAGE_EXTS.join(", "));
        return Ok(());
    }
    let (flags, pos) = split_flags(args, &["--out"]);
    let out = flags.get("--out").cloned();
    if pos.is_empty() {
        return Err("Укажи файл: Tool convert file.pdf [--out f.txt]".to_string());
    }
    for p in &pos {
        let text = conv::convert(p)?;
        match &out {
            Some(o) => {
                fs::write(o, &text).map_err(|e| e.to_string())?;
                println!("OK  {} -> {}  ({} симв.)", p, o, text.chars().count());
            }
            None => {
                println!("===== {} ({} симв.) =====", p, text.chars().count());
                println!("{}", text);
            }
        }
    }
    Ok(())
}

fn index_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (flags, pos) = split_flags(args, &["--index"]);
    let dir = pos.first().ok_or_else(|| {
        if cfg!(target_os = "windows") {
            "Укажи файл или папку: Tool index C:\\documents".to_string()
        } else {
            "Укажи файл или папку: Tool index ~/documents".to_string()
        }
    })?;
    let index_file = flags.get("--index").cloned().unwrap_or_else(|| "index.json".to_string());
    rag::index(&crate::base_host(s), &s.embed_model, dir, &index_file)
}

fn ask_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (flags, pos) = split_flags(args, &["--index", "--k"]);
    let q = pos.join(" ");
    if q.is_empty() {
        return Err("Укажи вопрос: Tool ask какой вывод в материалах".to_string());
    }
    let k = flags
        .get("--k")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);
    let index_file = flags.get("--index").cloned().unwrap_or_else(|| "index.json".to_string());
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    rag::ask(
        &crate::base_host(s),
        &s.model,
        &s.embed_model,
        &index_file,
        &q,
        k,
        options_for(s, &s.model),
    )
}

fn ask_file_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (_, pos) = split_flags(args, &[]);
    let file = pos.first().ok_or("Укажи файл: Tool ask-file отчёт.docx сделай резюме")?;
    let task = pos.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    if task.is_empty() {
        return Err("Укажи задание: Tool ask-file отчёт.docx сделай резюме".to_string());
    }
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let text = conv::convert(file)?;
    let text = truncate(&text, 40000, "Файл большой, обработаны первые 40000 симв.");
    let prompt = format!("Файл:\n{}\n\nЗадание: {}", text, task);
    let answer = ollama::generate(&crate::base_host(s), &s.model, &prompt, None, options_for(s, &s.model))?;
    println!("{}", answer);
    Ok(())
}

fn code_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (_, pos) = split_flags(args, &[]);
    let file = pos.first().ok_or("Укажи файл: Tool code main.py найди баги")?;
    let task = pos.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    if task.is_empty() {
        return Err("Укажи задание: Tool code main.py найди баги".to_string());
    }
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let text = conv::convert(file)?;
    let text = truncate(&text, 40000, "Файл большой, обработаны первые 40000 симв.");
    let prompt = format!("Файл:\n{}\n\nЗадание: {}", text, task);
    let answer = ollama::generate(&crate::base_host(s), &s.coder_model, &prompt, None, options_for(s, &s.coder_model))?;
    println!("{}", answer);
    Ok(())
}

fn ask_image_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (_, pos) = split_flags(args, &[]);
    let file = pos.first().ok_or("Укажи файл: Tool ask-image photo.png что на фото")?;
    let q = pos.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    if q.is_empty() {
        return Err("Укажи вопрос: Tool ask-image photo.png что на фото".to_string());
    }
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let b64 = image_b64(file)?;
    vision_ask(s, &s.vision_model, &q, b64)
}

fn summarize_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (_, pos) = split_flags(args, &[]);
    if pos.is_empty() {
        return Err(if cfg!(target_os = "windows") {
            "Укажи файлы или папки: Tool summarize C:\\documents".to_string()
        } else {
            "Укажи файлы или папки: Tool summarize ~/documents".to_string()
        });
    }
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let mut parts = Vec::new();
    for p in &pos {
        let path = Path::new(p);
        if path.is_dir() {
            let mut files = Vec::new();
            collect_files(path, &mut files);
            for f in files {
                match conv::convert(&f.to_string_lossy()) {
                    Ok(t) => parts.push(format!("[{}]\n{}", f.display(), t)),
                    Err(e) => parts.push(format!("[{}] (ошибка: {})", f.display(), e)),
                }
            }
        } else {
            match conv::convert(p) {
                Ok(t) => parts.push(format!("[{}]\n{}", p, t)),
                Err(e) => parts.push(format!("[{}] (ошибка: {})", p, e)),
            }
        }
    }
    let mut text = parts.join("\n\n");
    if text.trim().is_empty() {
        return Err("Ничего не удалось прочитать".to_string());
    }
    text = truncate(&text, 50000, "...(обрезано)");
    let prompt = format!("Сделай структурированное резюме материалов: заголовки, ключевые факты, выводы.\n\n{}", text);
    let answer = ollama::generate(&crate::base_host(s), &s.model, &prompt, None, options_for(s, &s.model))?;
    println!("{}", answer);
    Ok(())
}

fn translate_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (flags, pos) = split_flags(args, &["--file", "--to"]);
    let text = if let Some(f) = flags.get("--file") {
        conv::convert(f)?
    } else {
        pos.join(" ")
    };
    let text = text.trim();
    if text.is_empty() {
        return Err("Укажи текст или --file".to_string());
    }
    let text = truncate(text, 40000, "");
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let target = flags
        .get("--to")
        .cloned()
        .unwrap_or_else(|| {
            if conv::is_cyrillic(&text) {
                "английский".to_string()
            } else {
                "русский".to_string()
            }
        });
    let prompt = format!("Переведи текст на {}. Верни только перевод.\n\n{}", target, text);
    let answer = ollama::generate(&crate::base_host(s), &s.model, &prompt, None, options_for(s, &s.model))?;
    println!("{}", answer);
    Ok(())
}

fn search_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    let (flags, pos) = split_flags(args, &["--index", "--k"]);
    let q = pos.join(" ");
    if q.is_empty() {
        return Err("Укажи запрос: Tool search погода в москве".to_string());
    }
    let k = flags
        .get("--k")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);
    let index_file = flags.get("--index").cloned().unwrap_or_else(|| "index.json".to_string());
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    rag::search(&crate::base_host(s), &s.embed_model, &index_file, &q, k)
}

fn clip_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("клипборд: {}", e))?;
    let text = cb.get_text().unwrap_or_default();
    if text.trim().is_empty() {
        return Err("Буфер обмена пуст".to_string());
    }
    let task = args.join(" ");
    let task = if task.is_empty() {
        "Сделай краткое резюме.".to_string()
    } else {
        task
    };
    let prompt = format!("Буфер обмена:\n{}\n\nЗадание: {}", text, task);
    let answer = ollama::generate(&crate::base_host(s), &s.model, &prompt, None, options_for(s, &s.model))?;
    println!("{}", answer);
    Ok(())
}

fn screen_cmd(s: &crate::Settings, args: &[String]) -> Result<(), String> {
    if !crate::ensure_ollama(s) {
        return Ok(());
    }
    let q = args.join(" ");
    let q = if q.is_empty() {
        "Что сейчас на экране? Кратко опиши.".to_string()
    } else {
        q
    };
    let png = capture_screen()?;
    let b64 = image_b64(&png.to_string_lossy())?;
    let r = vision_ask(s, &s.vision_model, &q, b64);
    let _ = fs::remove_file(&png);
    r
}




fn vision_ask(s: &crate::Settings, model: &str, q: &str, b64: String) -> Result<(), String> {
    let host = crate::base_host(s);
    let opts = options_for(s, model);
    let mut answer = ollama::generate(&host, model, q, Some(&[b64.clone()]), opts.clone())?;
    if is_degenerate(&answer) {
        let _ = Command::new("ollama").args(["stop", model]).status();
        thread::sleep(Duration::from_secs(2));
        answer = ollama::generate(&host, model, q, Some(&[b64]), opts)?;
    }
    if is_degenerate(&answer) {
        return Err(format!(
            "Модель {} вернула испорченный ответ. Попробуй ещё раз или перезапусти ollama.",
            model
        ));
    }
    println!("{}", answer);
    Ok(())
}


fn is_degenerate(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return true;
    }
    if t.chars().count() < 10 {
        return false;
    }
    let c = t.chars().next().unwrap();
    t.chars().all(|x| x == c || x.is_whitespace())
}


fn capture_screen() -> Result<std::path::PathBuf, String> {
    let screens = screenshots::Screen::all().map_err(|e| format!("скриншот: {}", e))?;
    if screens.is_empty() {
        return Err("Экранов не найдено".to_string());
    }
    let img = screens[0].capture().map_err(|e| format!("скриншот: {}", e))?;
    let path = std::env::temp_dir().join("tool_screen.png");
    img.save(&path).map_err(|e| format!("сохранение: {}", e))?;
    Ok(path)
}


fn image_b64(path: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

fn truncate(text: &str, max: usize, msg: &str) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    if !msg.is_empty() {
        println!("{}", msg);
    }
    text.chars().take(max).collect()
}

fn collect_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                collect_files(&p, out);
            } else if let Some(e) = p.extension() {
                let e = format!(".{}", e.to_string_lossy().to_lowercase());
                let is_text = conv::TEXT_EXTS.contains(&e.as_str())
                    || conv::OFFICE_EXTS.contains(&e.as_str())
                    || e == ".pdf"
                    || e == ".epub";
                if is_text {
                    out.push(p);
                }
            }
        }
    }
}
