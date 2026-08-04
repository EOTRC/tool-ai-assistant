


use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::conv;
use crate::ollama;

fn walk_files(root: &Path, exts: &HashSet<String>, out: &mut Vec<std::path::PathBuf>) {
    if root.is_file() {
        if let Some(e) = root.extension() {
            let e = format!(".{}", e.to_string_lossy().to_lowercase());
            if exts.contains(&e) {
                out.push(root.to_path_buf());
            }
        }
        return;
    }
    if let Ok(rd) = fs::read_dir(root) {
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk_files(&p, exts, out);
            } else if let Some(e) = p.extension() {
                let e = format!(".{}", e.to_string_lossy().to_lowercase());
                if exts.contains(&e) {
                    out.push(p);
                }
            }
        }
    }
}

fn all_exts() -> HashSet<String> {
    let mut s = HashSet::new();
    for e in conv::TEXT_EXTS.iter().chain(conv::IMAGE_EXTS.iter()).chain(conv::OFFICE_EXTS.iter()) {
        s.insert(e.to_string());
    }
    s.insert(".pdf".to_string());
    s.insert(".epub".to_string());
    s
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..n {
        dot += a[i] as f64 * b[i] as f64;
        na += (a[i] as f64) * (a[i] as f64);
        nb += (b[i] as f64) * (b[i] as f64);
    }
    let denom = (na.sqrt()) * (nb.sqrt());
    if denom == 0.0 {
        0.0
    } else {
        (dot / denom) as f32
    }
}


pub fn index(host: &str, embed_model: &str, path: &str, index_file: &str) -> Result<(), String> {
    let mut files = Vec::new();
    walk_files(Path::new(path), &all_exts(), &mut files);
    if files.is_empty() {
        return Err("Нечего индексировать".to_string());
    }
    let mut entries: Vec<Value> = Vec::new();
    for (i, p) in files.iter().enumerate() {
        let text = match conv::convert(&p.to_string_lossy()) {
            Ok(t) => t,
            Err(e) => {
                println!("[!] {}: {}", p.display(), e);
                continue;
            }
        };
        let chunks = conv::chunk_text(&text, conv::CHUNK_SIZE, conv::CHUNK_OVERLAP);
        for (c, chunk) in chunks.iter().enumerate() {
            entries.push(json!({
                "file": p.to_string_lossy(),
                "chunk": c,
                "text": chunk,
            }));
        }
        println!("[{}/{}] {} ({} симв.)", i + 1, files.len(), p.file_name().unwrap_or_default().to_string_lossy(), text.len());
    }
    if entries.is_empty() {
        return Err("Нечего индексировать (ничего не прочиталось)".to_string());
    }
    let texts: Vec<String> = entries
        .iter()
        .filter_map(|e| e.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
        .collect();
    let vecs = ollama::embed(host, embed_model, &texts)?;
    for (e, v) in entries.iter_mut().zip(vecs.iter()) {
        e["vec"] = json!(v);
    }
    let data = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
    fs::write(index_file, data).map_err(|e| e.to_string())?;
    println!("Готово: {} чанков -> {}", entries.len(), index_file);
    Ok(())
}

fn load_index(index_file: &str) -> Result<Vec<Value>, String> {
    let raw = fs::read_to_string(index_file).map_err(|_| format!("Нет индекса {}", index_file))?;
    let entries: Vec<Value> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(entries)
}

fn score_top(host: &str, embed_model: &str, index_file: &str, query: &str, k: usize) -> Result<Vec<(f32, Value)>, String> {
    let entries = load_index(index_file)?;
    let qvec = ollama::embed_one(host, embed_model, query)?;
    let mut scored: Vec<(f32, Value)> = entries
        .iter()
        .filter_map(|e| {
            e.get("vec").and_then(|v| v.as_array()).map(|arr| {
                let vec: Vec<f32> = arr.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
                (cosine(&qvec, &vec), e.clone())
            })
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    Ok(scored)
}


pub fn search(host: &str, embed_model: &str, index_file: &str, query: &str, k: usize) -> Result<(), String> {
    let scored = score_top(host, embed_model, index_file, query, k)?;
    if scored.is_empty() {
        println!("Ничего не найдено.");
        return Ok(());
    }
    for (score, e) in scored {
        let file = e.get("file").and_then(|x| x.as_str()).unwrap_or("?");
        let chunk = e.get("chunk").and_then(|x| x.as_u64()).unwrap_or(0);
        let text = e.get("text").and_then(|x| x.as_str()).unwrap_or("");
        let snippet: String = text.chars().take(200).collect::<String>().replace('\n', " ");
        println!("[{:.2}] {} #{}", score, file, chunk);
        println!("    {}\n", snippet);
    }
    Ok(())
}


pub fn ask(
    host: &str,
    model: &str,
    embed_model: &str,
    index_file: &str,
    question: &str,
    k: usize,
    options: Value,
) -> Result<(), String> {
    let scored = score_top(host, embed_model, index_file, question, k)?;
    if scored.is_empty() {
        return Err("В индексе ничего не нашлось. Построй индекс: Tool index <папка>".to_string());
    }
    let mut ctx = Vec::new();
    for (_, e) in &scored {
        let file = e.get("file").and_then(|x| x.as_str()).unwrap_or("?");
        let chunk = e.get("chunk").and_then(|x| x.as_u64()).unwrap_or(0);
        let text = e.get("text").and_then(|x| x.as_str()).unwrap_or("");
        ctx.push(format!("[{} #{}]\n{}", file, chunk, text));
    }
    let context = ctx.join("\n\n---\n\n");
    let prompt = format!(
        "Ты — ассистент, отвечающий по материалам. Отвечай на языке вопроса. \
         Используй только приведённый контекст. Если ответа нет — так и скажи.\n\n\
         Контекст:\n{}\n\nВопрос: {}",
        context, question
    );
    let answer = ollama::generate(host, model, &prompt, None, options)?;
    println!("{}", answer);
    Ok(())
}
