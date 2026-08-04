


use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;


fn api(host: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let url = format!("{}{}", host.trim_end_matches('/'), path);
    let resp = match body {
        Some(b) => ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(600))
            .send_json(&b),
        None => ureq::get(&url)
            .timeout(Duration::from_secs(600))
            .call(),
    };
    let resp = resp.map_err(|e| format!("Ollama недоступен: {}", e))?;
    resp.into_json().map_err(|e| format!("Ошибка ответа Ollama: {}", e))
}




pub fn generate(
    host: &str,
    model: &str,
    prompt: &str,
    images: Option<&[String]>,
    options: Value,
) -> Result<String, String> {
    let mut payload = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": options,
    });
    if let Some(imgs) = images {
        if !imgs.is_empty() {
            payload["images"] = serde_json::to_value(imgs).unwrap_or(Value::Null);
        }
    }
    let v = api(host, "/api/generate", Some(payload))?;
    let r = v.get("response").and_then(|x| x.as_str()).unwrap_or("");
    Ok(super::clean_util::clean_text(r))
}


pub fn embed(host: &str, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let payload = json!({"model": model, "input": texts});
    match api(host, "/api/embed", Some(payload)) {
        Ok(v) => {
            let arr = v.get("embeddings").and_then(|e| e.as_array()).ok_or("нет embeddings")?;
            Ok(arr
                .iter()
                .map(|e| {
                    e.as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .filter_map(|x| x.as_f64())
                        .map(|x| x as f32)
                        .collect()
                })
                .collect())
        }
        Err(_) => {
            let mut out = Vec::new();
            for t in texts {
                let p = json!({"model": model, "prompt": t});
                let v = api(host, "/api/embeddings", Some(p))?;
                let e = v.get("embedding").and_then(|x| x.as_array()).ok_or("нет embedding")?;
                out.push(
                    e.iter()
                        .filter_map(|x| x.as_f64())
                        .map(|x| x as f32)
                        .collect(),
                );
            }
            Ok(out)
        }
    }
}


pub fn embed_one(host: &str, model: &str, text: &str) -> Result<Vec<f32>, String> {
    embed(host, model, &[text.to_string()]).map(|mut v| v.pop().unwrap_or_default())
}


pub fn ping(host: &str) -> Result<String, String> {
    let v = api(host, "/api/version", None)?;
    Ok(v.get("version").and_then(|x| x.as_str()).unwrap_or("?").to_string())
}


pub fn pull(host: &str, model: &str) -> Result<(), String> {
    let url = format!("{}/api/pull", host.trim_end_matches('/'));
    let payload = json!({"model": model, "stream": true});
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(3600))
        .send_json(&payload)
        .map_err(|e| format!("Ollama недоступен: {}", e))?;
    let mut reader = BufReader::new(resp.into_reader());
    let mut line = String::new();
    let mut last_pct = -1i32;
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(t) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
            return Err(err.to_string());
        }
        if let Some(st) = v.get("status").and_then(|x| x.as_str()) {
            let total = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0);
            let completed = v.get("completed").and_then(|x| x.as_u64()).unwrap_or(0);
            if total > 0 {
                let pct = ((completed as f64 / total as f64) * 100.0) as i32;
                if pct != last_pct {
                    print!("\r  {} ... {}%", model, pct);
                    let _ = std::io::stdout().flush();
                    last_pct = pct;
                }
            } else {
                println!("  {}: {}", model, st);
            }
        }
    }
    println!();
    Ok(())
}
