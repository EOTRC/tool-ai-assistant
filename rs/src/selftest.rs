



use std::time::Instant;

use crate::clean_util;
use crate::conv;
use crate::ollama;
use crate::Settings;

pub fn run(s: &Settings) {
    let mut ok = 0usize;
    let mut fail = 0usize;

    macro_rules! check {
        ($name:expr, $r:expr) => {{
            match $r {
                Ok(v) => {
                    println!("[OK]   {} — {}", $name, v);
                    ok += 1;
                }
                Err(e) => {
                    println!("[FAIL] {} — {}", $name, e);
                    fail += 1;
                }
            }
        }};
    }

    println!("=== Tool selftest ===");
    println!(
        "Сборка: {} {} · ОС: {} · Host ollama: {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_NAME"),
        std::env::consts::OS,
        s.ollama_host
    );
    let gpu = match crate::num_gpu_for(s, "default") {
        Some(n) => n.to_string(),
        None => "auto".to_string(),
    };
    println!(
        "Модели: chat={} embed={} vision={} coder={} · GPU: {}",
        s.model, s.embed_model, s.vision_model, s.coder_model, gpu
    );

    
    let ansi_in = "\u{1b}[31mкрасный\u{1b}[0m и \u{1b}[1;32mзелёный\u{1b}[0m";
    let ansi_out = clean_util::strip_ansi(ansi_in);
    check!(
        "ANSI-очистка",
        if ansi_out == "красный и зелёный" {
            Ok("работает")
        } else {
            Err(format!("искажение: {:?}", ansi_out))
        }
    );
    let think_in = "<think>внутренние рассуждения</think>Пользователю";
    let think_out = clean_util::strip_think(think_in);
    check!(
        "strip_think",
        if think_out == "Пользователю" {
            Ok("работает")
        } else {
            Err(format!("искажение: {:?}", think_out))
        }
    );

    
    let tmp = std::env::temp_dir().join("tool_selftest");
    let _ = std::fs::create_dir_all(&tmp);
    let utf8_f = tmp.join("t_utf8.txt");
    std::fs::write(&utf8_f, "\u{feff}Привет, мир! Тест UTF-8 с BOM.").unwrap();
    check!(
        "convert txt+BOM",
        conv::convert(&utf8_f.to_string_lossy()).map(|t| {
            if t.starts_with("Привет") {
                "BOM обрезан".to_string()
            } else {
                format!("не так: {:?}", t)
            }
        })
    );
    let cp1251_f = tmp.join("t_cp1251.txt");
    let mut b = vec![0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2]; 
    b.extend_from_slice(b", mir!");
    std::fs::write(&cp1251_f, &b).unwrap();
    check!(
        "convert cp1251",
        conv::convert(&cp1251_f.to_string_lossy()).map(|t| {
            if t.starts_with("Привет") {
                "декодировано".to_string()
            } else {
                format!("не так: {:?}", t)
            }
        })
    );
    check!(
        "chunk_text",
        Ok::<String, String>(conv::chunk_text(&"абв ".repeat(300), 1000, 150).len().to_string())
    );
    check!(
        "is_cyrillic",
        if conv::is_cyrillic("Привет мир") && !conv::is_cyrillic("hello world") {
            Ok("работает")
        } else {
            Err("неверно определяет кириллицу".to_string())
        }
    );
    check!(
        "html_unescape",
        if conv::html_unescape("a &lt;b&gt; &amp; &laquo;c&raquo;") == "a <b> & «c»" {
            Ok("работает")
        } else {
            Err("неверно".to_string())
        }
    );

    
    match ollama::ping(&crate::base_host(s)) {
        Ok(v) => {
            println!("[OK]   ollama ping — версия {}", v);
            ok += 1;
        }
        Err(e) => {
            println!("[FAIL] ollama ping — {}", e);
            fail += 1;
            println!("\nДальнейшие сетевые проверки пропущены.");
            finish(ok, fail);
            return;
        }
    }

    
    match crate::api_get(&crate::base_host(s), "/api/tags") {
        Ok(v) => {
            let names: Vec<String> = v
                .get("models")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|n| n.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            println!(
                "[OK]   модели в ollama — {}",
                if names.is_empty() { "(пусто)".to_string() } else { names.join(", ") }
            );
            for m in [&s.model, &s.embed_model, &s.vision_model, &s.coder_model] {
                if !names.iter().any(|n| n == m || n.starts_with(&format!("{}:", m))) {
                    println!("[WARN] модель из settings.cfg не в списке ollama: {}", m);
                }
            }
            ok += 1;
        }
        Err(e) => {
            println!("[FAIL] список моделей — {}", e);
            fail += 1;
        }
    }

    
    check!(
        "embedding размер",
        ollama::embed_one(&crate::base_host(s), &s.embed_model, "тест").map(|v| v.len().to_string())
    );
    let mut opts = serde_json::json!({ "num_ctx": 2048, "temperature": 0.1 });
    if let Some(n) = crate::num_gpu_for(s, &s.model) {
        opts["num_gpu"] = serde_json::json!(n);
    }
    let body = serde_json::json!({
        "model": s.model,
        "prompt": "Ответь одним словом: ок",
        "stream": false,
        "options": opts
    });
    let start = Instant::now();
    match crate::api_post(&crate::base_host(s), "/api/generate", body) {
        Ok(v) => {
            let elapsed = start.elapsed();
            let toks = v.get("eval_count").and_then(|x| x.as_u64()).unwrap_or(0);
            let secs = elapsed.as_secs_f64().max(0.001);
            println!(
                "[OK]   скорость {} — {} ток. за {:.1}с = {:.0} ток/с, ответ: {}",
                s.model,
                toks,
                secs,
                toks as f64 / secs,
                v.get("response").and_then(|x| x.as_str()).unwrap_or("").trim().chars().take(40).collect::<String>()
            );
            ok += 1;
        }
        Err(e) => {
            println!("[FAIL] скорость {} — {}", s.model, e);
            fail += 1;
        }
    }

    finish(ok, fail);
}

fn finish(ok: usize, fail: usize) {
    println!("\nИтог: {} проверок OK, {} ошибок.", ok, fail);
    if fail > 0 {
        std::process::exit(1);
    }
}
