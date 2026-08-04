use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

mod clean_util;
mod cmd_file;
mod conv;
mod ollama;
mod rag;
mod selftest;

use clean_util::{AnsiStrip, strip_ansi, strip_think};

const SETTINGS_FILE: &str = "settings.cfg";

const TOOL_COMMANDS: &[&str] = &[
    "help", "chat", "shell", "screen", "status", "models", "settings", "todo",
    "convert", "ask", "ask-file", "code", "summarize", "translate", "search",
    "index", "clip", "ask-image", "web", "alias", "selftest",
];

const SYSTEM_COMMANDS: &[&str] = &[
    "help", "dir", "copy", "del", "erase", "move", "ren", "rename", "mkdir", "md",
    "cd", "cls", "echo", "find", "findstr", "sort", "type", "ver", "date", "time",
    "path", "set", "start", "taskkill", "tasklist", "xcopy", "robocopy", "attrib",
    "tree", "where", "more", "pause", "color", "exit", "prompt", "title", "vol",
    "assoc", "ftype", "rd", "rmdir", "call", "goto", "shift", "pushd", "popd", "chcp",
];

const DELEGATE_COMMANDS: &[&str] = &[
    "convert", "ask", "ask-file", "code", "summarize", "translate", "search",
    "index", "clip", "ask-image",
];

const DANGEROUS: &[&str] = &[
    "remove-item", "rm -", "del /", "clear-content", "format-", "reg add", "reg delete",
    "set-itemproperty", "new-service", "set-mppreference", "stop-process", "taskkill",
    "shutdown", "restart-computer", "enable-bitlocker", "diskpart", "sc delete",
    "netsh firewall", "netsh advfirewall",
];


const NO_THINK_MARKERS: &[&str] = &["*no_think*", "/no_think", "no_thinking", "/no_thinking"];



const WEB_TRIGGERS: &[&str] = &[
    "найди", "найти", "узнай", "узнать", "поиск", "поищи", "погода", "прогноз",
    "новости", "новост", "актуальн", "свеж", "последн", "сейчас", "сегодня",
    "интернет", "веб", "web", "search", "курс", "курсы", "цена", "цену",
    "сколько стоит", "проверь", "проверить", "что нового", "билет", "расписание",
    "события", "происходит", "статистик", "рейтинг", "weather", "news", "price",
    "latest", "stock", "акций", "валют", "доллар", "евро",
];

#[derive(Clone, Copy, PartialEq)]
enum Resolution {
    Tool,
    System,
}

struct Settings {
    ollama_host: String,
    model: String,
    coder_model: String,
    vision_model: String,
    embed_model: String,
    language: String,
    
    think: bool,
    
    system_prompt: String,
    
    devices: HashMap<String, String>,
    aliases: HashMap<String, String>,
    defaults: HashMap<String, String>,
    raw: String,
    path: PathBuf,
}

#[derive(Clone)]
struct ToolCall {
    name: String,
    args: serde_json::Value,
}

fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Settings {
    fn load() -> Settings {
        let dir = exe_dir();
        let path = dir.join(SETTINGS_FILE);
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                let tmpl = include_str!("../settings.cfg.template");
                let _ = fs::write(&path, tmpl);
                tmpl.to_string()
            }
        };
        let mut s = Settings {
            ollama_host: "http://localhost:11434".into(),
            model: "qwen3:1.7b".into(),
            coder_model: "qwen2.5-coder:7b".into(),
            vision_model: "qwen2.5vl:3b".into(),
            embed_model: "nomic-embed-text".into(),
            language: "ru".into(),
            think: false,
            system_prompt: String::new(),
            devices: HashMap::new(),
            aliases: HashMap::new(),
            defaults: HashMap::new(),
            raw: content,
            path,
        };
        s.parse();
        s
    }

    fn parse(&mut self) {
        let mut section = String::new();
        for line in self.raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_lowercase();
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_lowercase();
                let val = line[eq + 1..].trim().to_string();
                match section.as_str() {
                    "general" => match key.as_str() {
                        "ollama_host" => self.ollama_host = val,
                        "model" => self.model = val,
                        "coder_model" => self.coder_model = val,
                        "vision_model" => self.vision_model = val,
                        "embed_model" => self.embed_model = val,
                        "language" => self.language = val,
                        "think" => {
                            self.think = val.to_lowercase() == "on" || val.to_lowercase() == "true" || val == "1";
                        }
                        "system_prompt" => self.system_prompt = val,
                        _ => {}
                    },
                    "devices" => {
                        self.devices.insert(key, val.to_lowercase());
                    }
                    "aliases" => {
                        self.aliases.insert(key, val);
                    }
                    "defaults" => {
                        self.defaults.insert(key, val.to_lowercase());
                    }
                    _ => {}
                }
            }
        }
    }

    fn save_default(&mut self, cmd: &str, choice: &str) -> Result<(), String> {
        self.defaults.insert(cmd.to_lowercase(), choice.to_lowercase());
        let mut lines: Vec<String> = self.raw.lines().map(|s| s.to_string()).collect();
        let mut defaults_idx: Option<usize> = None;
        let mut replaced = false;
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim();
            if t.starts_with('[') {
                if t.to_lowercase() == "[defaults]" {
                    defaults_idx = Some(i);
                } else if defaults_idx.is_some() && !replaced {
                    break;
                }
            }
            if defaults_idx.is_some() {
                if let Some(eq) = lines[i].find('=') {
                    let lt = lines[i].trim_start();
                    if !lt.starts_with('#') && !lt.starts_with(';')
                        && lines[i][..eq].trim().to_lowercase() == cmd.to_lowercase()
                    {
                        lines[i] = format!("{} = {}", cmd, choice);
                        replaced = true;
                    }
                }
            }
            i += 1;
        }
        if !replaced {
            let insert_at = match defaults_idx {
                Some(idx) => {
                    let mut j = idx + 1;
                    while j < lines.len() && !lines[j].trim().starts_with('[') {
                        j += 1;
                    }
                    j
                }
                None => {
                    lines.push(String::new());
                    lines.push("[defaults]".to_string());
                    lines.len()
                }
            };
            lines.insert(insert_at, format!("{} = {}", cmd, choice));
        }
        let out = lines.join("\n") + "\n";
        fs::write(&self.path, &out).map_err(|e| e.to_string())?;
        self.raw = out;
        Ok(())
    }

    
    fn save_general(&mut self, key: &str, value: &str) -> Result<(), String> {
        let mut lines: Vec<String> = self.raw.lines().map(|s| s.to_string()).collect();
        let mut in_general = false;
        let mut replaced = false;
        for i in 0..lines.len() {
            let t = lines[i].trim();
            if t.starts_with('[') {
                in_general = t[1..t.len() - 1].trim().eq_ignore_ascii_case("general");
                continue;
            }
            if in_general {
                if let Some(eq) = lines[i].find('=') {
                    if lines[i][..eq].trim().eq_ignore_ascii_case(key) {
                        lines[i] = format!("{} = {}", key, value);
                        replaced = true;
                    }
                }
            }
        }
        if !replaced {
            let mut insert_at = lines.len();
            for i in 0..lines.len() {
                let t = lines[i].trim();
                if t.starts_with('[') && !t[1..t.len() - 1].trim().eq_ignore_ascii_case("general") {
                    insert_at = i;
                    break;
                }
            }
            lines.insert(insert_at, format!("{} = {}", key, value));
        }
        let out = lines.join("\n") + "\n";
        fs::write(&self.path, &out).map_err(|e| e.to_string())?;
        self.raw = out;
        Ok(())
    }
}

fn base_host(s: &Settings) -> String {
    s.ollama_host.trim_end_matches('/').to_string()
}

fn api_get(host: &str, path: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", host, path);
    ureq::get(&url)
        .timeout(Duration::from_secs(10))
        .call()
        .map_err(|e| format!("Ollama недоступен: {}", e))?
        .into_json()
        .map_err(|e| e.to_string())
}

fn api_post(host: &str, path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", host, path);
    ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(300))
        .send_json(&body)
        .map_err(|e| format!("Ollama недоступен: {}", e))?
        .into_json()
        .map_err(|e| e.to_string())
}



fn ensure_ollama(s: &Settings) -> bool {
    let host = base_host(s);
    if api_get(&host, "/api/version").is_ok() {
        return true;
    }
    println!("Ollama не отвечает на {}. Пробую запустить 'ollama serve'...", host);
    let _ = Command::new("ollama").arg("serve").spawn();
    std::thread::sleep(Duration::from_secs(5));
    if api_get(&host, "/api/version").is_ok() {
        println!("Ollama запущен.");
        return true;
    }
    eprintln!(
        "Ollama недоступен ({}).\nПроверь: 1) сервер запущен ('ollama serve' или Ollama Desktop); 2) хост/порт в settings.cfg.",
        host
    );
    false
}



fn num_gpu_for(s: &Settings, model: &str) -> Option<i64> {
    let v = s
        .devices
        .get(model)
        .or(s.devices.get("default"))
        .map(|x| x.as_str())
        .unwrap_or("gpu");
    let v = v.trim().to_lowercase();
    if v == "cpu" {
        return Some(0);
    }
    if v == "gpu" {
        return None;
    }
    if let Some(rest) = v.strip_prefix("gpu:") {
        if let Ok(n) = rest.trim().parse::<i64>() {
            return Some(n.max(1));
        }
    }
    None
}


fn set_num_gpu(opts: &mut serde_json::Value, s: &Settings, model: &str) {
    if let Some(n) = num_gpu_for(s, model) {
        opts["num_gpu"] = serde_json::json!(n);
    }
}


fn system_content(s: &Settings) -> String {
    let mut c = format!(
        "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: {}.",
        s.language
    );
    if !s.system_prompt.is_empty() {
        c.push_str("\n\n");
        c.push_str(&s.system_prompt);
    }
    if !s.think {
        c.push_str("\n\nОтвечай сразу, без рассуждений. /no_think");
    }
    c
}


fn apply_no_think(think: bool, q: &str) -> String {
    if think {
        q.to_string()
    } else {
        format!("{} {}", NO_THINK_MARKERS[1], q)
    }
}

fn stream_chat(host: &str, body: serde_json::Value) -> Result<(String, Vec<ToolCall>), String> {
    let url = format!("{}/api/chat", host);
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(600))
        .send_json(&body)
        .map_err(|e| format!("Ollama недоступен: {}", e))?;
    let mut reader = BufReader::new(resp.into_reader());
    let mut out = String::new();
    let mut calls: Vec<ToolCall> = Vec::new();
    let mut line = String::new();
    let mut ansi = AnsiStrip::new();
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
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
            if let Some(msg) = v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                let mut clean = String::new();
                ansi.push(msg, &mut clean);
                print!("{}", clean);
                io::stdout().flush().ok();
                out.push_str(&clean);
            }
            if let Some(tcs) = v
                .get("message")
                .and_then(|m| m.get("tool_calls"))
                .and_then(|t| t.as_array())
            {
                for tc in tcs {
                    if let Some(f) = tc.get("function") {
                        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                        let args = f.get("arguments").cloned().unwrap_or(serde_json::Value::Null);
                        calls.push(ToolCall { name, args });
                    }
                }
            }
            if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                break;
            }
        }
    }
    println!();
    Ok((out, calls))
}

fn is_dangerous(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    DANGEROUS.iter().any(|d| c.contains(d))
}

fn resolve(first: &str, settings: &mut Settings) -> Resolution {
    let lower = first.to_lowercase();
    let is_tool = TOOL_COMMANDS.contains(&lower.as_str());
    let is_system = SYSTEM_COMMANDS.contains(&lower.as_str());
    if !(is_tool && is_system) {
        return Resolution::Tool;
    }
    if let Some(d) = settings.defaults.get(&lower) {
        return if d == "system" {
            Resolution::System
        } else {
            Resolution::Tool
        };
    }
    println!("Команда '{}' есть и в Tool, и в Windows. Как использовать?", first);
    println!("  1) Tool      2) Система      3) Всегда Tool      4) Всегда Система");
    print!("Выбор [1-4]: ");
    io::stdout().flush().ok();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    let choice: String = line
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .to_string();
    match choice.as_str() {
        "2" => Resolution::System,
        "3" => {
            let _ = settings.save_default(&lower, "tool");
            Resolution::Tool
        }
        "4" => {
            let _ = settings.save_default(&lower, "system");
            Resolution::System
        }
        _ => Resolution::Tool,
    }
}


fn run_shell(line: &str) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/c", line]).status()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("/bin/sh").args(["-c", line]).status()
    }
}

fn run_system(args: &[String]) {
    let line = args
        .iter()
        .map(|a| if a.contains(' ') { format!("\"{}\"", a) } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ");
    match run_shell(&line) {
        Ok(s) => std::process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("Ошибка запуска: {}", e);
            std::process::exit(1);
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = hexval(bytes[i + 1]);
            let l = hexval(bytes[i + 2]);
            if let (Some(h), Some(l)) = (h, l) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
}

fn web_search(query: &str) -> Result<String, String> {
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencode(query));
    let resp = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36")
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|e| format!("Ошибка запроса к DuckDuckGo: {}", e))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let re_link = Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    let re_snip = Regex::new(r#"<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#).unwrap();
    let re_tag = Regex::new(r"<[^>]+>").unwrap();
    let strip = |s: &str| html_unescape(&re_tag.replace_all(s, "").into_owned());
    let links: Vec<(String, String)> = re_link
        .captures_iter(&resp)
        .map(|c| {
            let href = c.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let title = strip(c.get(2).map(|m| m.as_str()).unwrap_or(""));
            (href, title)
        })
        .collect();
    let snips: Vec<String> = re_snip
        .captures_iter(&resp)
        .map(|c| strip(c.get(1).map(|m| m.as_str()).unwrap_or("")))
        .collect();
    if links.is_empty() {
        return Ok("(поиск не дал результатов)".to_string());
    }
    let mut out = String::new();
    for (i, (href, title)) in links.iter().take(6).enumerate() {
        let real = if href.contains("uddg=") {
            let start = href.find("uddg=").map(|p| p + 5).unwrap_or(0);
            let end = href[start..].find('&').map(|p| start + p).unwrap_or(href.len());
            urldecode(&href[start..end])
        } else {
            href.clone()
        };
        out.push_str(&format!("{}. {}\n   {}\n", i + 1, title, real));
        if let Some(sn) = snips.get(i) {
            if !sn.is_empty() {
                out.push_str(&format!("   {}\n", sn));
            }
        }
    }
    Ok(out)
}

fn web_search_tool() -> serde_json::Value {
    serde_json::json!([{
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Поиск в интернете. Вызывай, когда пользователь просит актуальную информацию, новости, погоду, данные из интернета или то, чего ты не знаешь. Передай короткий поисковый запрос.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Поисковый запрос"}
                },
                "required": ["query"]
            }
        }
    }])
}



fn chat_body(s: &Settings, messages: Vec<serde_json::Value>, with_tools: bool) -> serde_json::Value {
    let mut opts = serde_json::json!({
        "num_ctx": 8192,
        "temperature": 0.1,
    });
    set_num_gpu(&mut opts, s, &s.model);
    let mut body = serde_json::json!({
        "model": s.model,
        "messages": messages,
        "stream": true,
        "options": opts,
    });
    if with_tools {
        body["tools"] = web_search_tool();
    }
    body
}




fn needs_tools(messages: &[serde_json::Value]) -> bool {
    if messages
        .iter()
        .any(|m| m.get("role").and_then(|r| r.as_str()) == Some("tool"))
    {
        return true;
    }
    for m in messages.iter().rev() {
        if m.get("role").and_then(|r| r.as_str()) == Some("user") {
            let text = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let lower = text.to_lowercase();
            return WEB_TRIGGERS.iter().any(|t| lower.contains(t));
        }
    }
    false
}



fn chat_round(
    s: &Settings,
    messages: &mut Vec<serde_json::Value>,
    think: bool,
) -> Result<bool, String> {
    let with_tools = needs_tools(messages);
    let body = chat_body(s, messages.clone(), with_tools);
    let (text, calls) = stream_chat(&base_host(s), body)?;
    if calls.is_empty() {
        let clean = strip_think(&text);
        messages.push(serde_json::json!({"role": "assistant", "content": clean}));
        return Ok(true);
    }
    for call in calls {
        if call.name == "web_search" {
            let q = call.args.get("query").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !q.is_empty() {
                println!("[поиск в интернете: {}]", q);
                match web_search(&q) {
                    Ok(res) => messages.push(serde_json::json!({"role": "tool", "content": res})),
                    Err(e) => messages.push(serde_json::json!({"role": "tool", "content": format!("Ошибка поиска: {}", e)})),
                }
            }
        }
    }
    let _ = think;
    Ok(false)
}

fn print_help() {
    println!("Tool {} — локальный ИИ-ассистент (cmd-обёртка).", "0.2.0");
    println!("Обычные команды Windows работают как в cmd. Свои команды:");
    println!();
    println!("  ИИ (модель из settings.cfg, по умолчанию qwen3:1.7b):");
    println!("    Tool chat                      диалог со стримингом (exit/пока — выход)");
    println!("    Tool chat --file doc.txt       диалог с учётом файла");
    println!("    Tool chat --once \"вопрос\"      один ответ");
    println!("    Tool chat --once \"q\" --think   один ответ с думаньем");
    println!("    В чате: /think on | off        включить/выключить думанье");
    println!("    Tool shell \"показать место\"    натуральный язык -> команда PowerShell");
    println!("    Tool shell \"...\" --run         выполнить команду (опасные требуют --force)");
    println!("    Tool web \"что искать\"          поиск в интернете");
    println!("    Tool screen \"что на экране\"    скриншот + описание (qwen2.5vl)");
    println!("    Tool status                    проверка Ollama");
    println!("    Tool models                    список моделей");
    println!();
    println!("  Файлы и документы (на чистом Rust):");
    println!("    Tool convert file.pdf --out f.txt    файл -> текст");
    println!("    Tool ask \"вопрос по материалам\"       вопрос по индексу (RAG)");
    println!("    Tool index C:\\documents               построить индекс");
    println!("    Tool ask-file file.docx \"резюме\"      задание по файлу");
    println!("    Tool code main.py \"найди баги\"        работа с кодом");
    println!("    Tool summarize C:\\documents           резюме файлов/папок");
    println!("    Tool translate \"Hello\" --to русский   перевод");
    println!("    Tool search \"температура чая\"         поиск по индексу");
    println!("    Tool clip \"переведи\"                 буфер обмена");
    println!("    Tool ask-image photo.png \"что это\"    вопрос по картинке");
    println!();
    println!("  Служебные:");
    println!("    Tool help            справка");
    println!("    Tool settings        открыть settings.cfg в блокноте");
    println!("    Tool settings view   показать настройки");
    println!("    Tool alias           список алиасов");
    println!("    Tool selftest        самодиагностика (всё в одном)");
    println!("    Tool todo            показать TODO/как это работает");
    println!();
    println!("  toolcmd — консоль, где команды Tool работают без префикса,");
    println!("  есть алиасы (alias имя=команда) и обычные cmd-команды.");
}

fn print_todo(settings: &Settings) {
    let mut active = false;
    for line in settings.raw.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            active = t[1..t.len() - 1].trim().eq_ignore_ascii_case("todo");
            continue;
        }
        if active {
            println!("{}", line);
        }
    }
}

fn cmd_status(s: &Settings) {
    let host = base_host(s);
    match api_get(&host, "/api/version") {
        Ok(v) => {
            let ver = v.get("version").and_then(|x| x.as_str()).unwrap_or("?");
            println!("Ollama: OK ({})", ver);
        }
        Err(e) => {
            println!("{} (запусти 'ollama serve')", e);
            return;
        }
    }
    cmd_models(s);
}

fn cmd_models(s: &Settings) {
    let host = base_host(s);
    match api_get(&host, "/api/tags") {
        Ok(v) => {
            if let Some(models) = v.get("models").and_then(|m| m.as_array()) {
                for m in models {
                    let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let size = m.get("size").and_then(|x| x.as_u64()).unwrap_or(0) as f64 / 1e9;
                    println!("{:<38} {:>6.1} GB", name, size);
                }
            }
        }
        Err(e) => println!("{}", e),
    }
}

fn cmd_chat(s: &mut Settings, args: &[String]) {
    if !ensure_ollama(s) {
        return;
    }
    let mut think = s.think;
    let mut file_text: Option<String> = None;
    let mut idx = 0;
    if args.first().map(|a| a.as_str() == "--file").unwrap_or(false) {
        if let Some(f) = args.get(1) {
            match fs::read_to_string(f) {
                Ok(t) => file_text = Some(t),
                Err(e) => println!("Не удалось прочитать файл: {}", e),
            }
            idx = 2;
        }
    }
    if args.get(idx).map(|a| a.as_str() == "--once").unwrap_or(false) {
        let q = args.iter().skip(idx + 1).cloned().collect::<Vec<_>>().join(" ");
        let q = q.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        if q.is_empty() {
            eprintln!("Для --once укажи вопрос (можно без кавычек).");
            return;
        }
        if args.contains(&"--think".to_string()) {
            think = true;
        }
        if args.contains(&"--no-think".to_string()) {
            think = false;
        }
        let mut messages = base_messages(s, think, file_text.as_deref());
        let user_q = apply_no_think(think, q);
        messages.push(serde_json::json!({"role": "user", "content": user_q}));
        let mut rounds = 0;
        loop {
            match chat_round(s, &mut messages, think) {
                Ok(true) => break,
                Ok(false) => {
                    rounds += 1;
                    if rounds >= 4 {
                        println!("\n(превышен лимит вызовов инструментов)");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            }
        }
        return;
    }
    println!("Чат с {} ({}). 'exit'/'пока' — выход. /think on|off — думанье.", s.model, base_host(s));
    let mut messages = base_messages(s, think, file_text.as_deref());
    let mut buffer = String::new();
    loop {
        print!("Ты> ");
        io::stdout().flush().ok();
        buffer.clear();
        match io::stdin().read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let q = buffer.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        if q.is_empty() {
            continue;
        }
        let lower = q.to_lowercase();
        if matches!(lower.as_str(), "exit" | "quit" | "пока" | "стоп" | "выход") {
            break;
        }
        if lower == "/think" || lower == "/think on" || lower == "think on" {
            think = true;
            s.think = true;
            let _ = s.save_general("think", "on");
            println!("[думанье включено]");
            continue;
        }
        if lower == "/think off" || lower == "/no_think" || lower == "think off" || lower == "/no-think" {
            think = false;
            s.think = false;
            let _ = s.save_general("think", "off");
            println!("[думанье выключено]");
            continue;
        }
        let user_q = apply_no_think(think, q);
        messages.push(serde_json::json!({"role": "user", "content": user_q}));
        let mut rounds = 0;
        loop {
            match chat_round(s, &mut messages, think) {
                Ok(true) => break,
                Ok(false) => {
                    rounds += 1;
                    if rounds >= 4 {
                        println!("\n(превышен лимит вызовов инструментов)");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            }
        }
    }
}


fn base_messages(s: &Settings, _think: bool, file_text: Option<&str>) -> Vec<serde_json::Value> {
    let mut messages = vec![serde_json::json!({"role": "system", "content": system_content(s)})];
    if let Some(t) = file_text {
        messages.push(serde_json::json!({"role": "user", "content": format!("Файл:\n{}", t)}));
    }
    messages
}

fn cmd_shell(s: &Settings, args: &[String]) {
    if !ensure_ollama(s) {
        return;
    }
    let run = args.iter().any(|a| a == "--run");
    let force = args.iter().any(|a| a == "--force");
    let task: Vec<String> = args
        .iter()
        .filter(|a| a.as_str() != "--run" && a.as_str() != "--force")
        .cloned()
        .collect();
    let task = task.join(" ");
    if task.is_empty() {
        eprintln!("Задача не указана. Пример: Tool shell \"показать свободное место на диске C\"");
        return;
    }
    #[cfg(target_os = "windows")]
    let shell_name = "PowerShell для Windows 11";
    #[cfg(not(target_os = "windows"))]
    let shell_name = "Bash (Linux/macOS)";
    let mut sys = format!(
        "Ты генератор команд {}. Отвечай ОДНОЙ командой, \
         без пояснений, без markdown. Пути с пробелами бери в одинарные кавычки. \
         Если нужно несколько шагов — объедини через ';'. /no_think",
        shell_name
    );
    if !s.system_prompt.is_empty() {
        sys.push_str("\n\n");
        sys.push_str(&s.system_prompt);
    }
    let user_q = if s.think { task.clone() } else { format!("{} {}", NO_THINK_MARKERS[1], task) };
    let mut opts = serde_json::json!({
        "num_ctx": 4096,
        "temperature": 0.2,
        "num_predict": 400,
    });
    set_num_gpu(&mut opts, s, &s.model);
    let body = serde_json::json!({
        "model": s.model,
        "messages": [
            {"role": "system", "content": sys},
            {"role": "user", "content": user_q}
        ],
        "stream": true,
        "options": opts
    });
    match stream_chat(&base_host(s), body) {
        Ok((text, _)) => {
            let clean = strip_think(&text);
            let clean = strip_ansi(&clean);
            if clean.is_empty() {
                println!("(модель не вернула команду)");
                return;
            }
            println!("\n{}", clean);
            if run {
                if is_dangerous(&clean) && !force {
                    println!("\n[!] Команда содержит опасные операции. Для запуска добавь --force");
                    return;
                }
                println!("\nЗапуск: {}", clean);
                match run_shell(&clean) {
                    Ok(st) => println!("(код возврата: {})", st.code().unwrap_or(0)),
                    Err(e) => eprintln!("Ошибка запуска: {}", e),
                }
            }
        }
        Err(e) => eprintln!("{}", e),
    }
}

fn cmd_web(args: &[String]) {
    let q = args.join(" ");
    if q.is_empty() {
        eprintln!("Укажи запрос: Tool web \"что искать\"");
        return;
    }
    match web_search(&q) {
        Ok(r) => println!("{}", r),
        Err(e) => eprintln!("{}", e),
    }
}

fn cmd_alias(s: &Settings) {
    if s.aliases.is_empty() {
        println!("Алиасов нет. Добавь в settings.cfg -> [aliases] или используй toolcmd (команда alias).");
        return;
    }
    let mut v: Vec<(&String, &String)> = s.aliases.iter().collect();
    v.sort_by_key(|a| a.0);
    for (k, val) in v {
        println!("{} = {}", k, val);
    }
}

fn cmd_settings(s: &Settings, args: &[String]) {
    if args.first().map(|a| a.as_str() == "view").unwrap_or(false) {
        print!("{}", s.raw);
        return;
    }
    match open_settings(&s.path) {
        Ok(_) => println!("Открыт {}", s.path.display()),
        Err(e) => eprintln!("Не удалось открыть настройки: {}", e),
    }
}



fn open_settings(path: &std::path::Path) -> std::io::Result<std::process::Child> {
    #[cfg(target_os = "windows")]
    {
        Command::new("notepad").arg(path).spawn()
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-e").arg(path).spawn()
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut settings = Settings::load();

    if args.is_empty() {
        print_help();
        return;
    }
    if args[0] == "-h" || args[0] == "--help" || args[0] == "/?" {
        print_help();
        return;
    }
    if args[0] == "--version" {
        println!("Tool 0.2.0");
        return;
    }

    let first = args[0].to_lowercase();
    match resolve(&first, &mut settings) {
        Resolution::System => run_system(&args),
        Resolution::Tool => match first.as_str() {
            "help" => print_help(),
            "chat" => cmd_chat(&mut settings, &args[1..]),
            "shell" => cmd_shell(&settings, &args[1..]),
            "screen" => cmd_file::dispatch(&settings, "screen", &args[1..]),
            "web" => cmd_web(&args[1..]),
            "selftest" => selftest::run(&settings),
            "alias" => cmd_alias(&settings),
            "status" => cmd_status(&settings),
            "models" => cmd_models(&settings),
            "settings" | "config" => cmd_settings(&settings, &args[1..]),
            "todo" => print_todo(&settings),
            c if DELEGATE_COMMANDS.contains(&c) => cmd_file::dispatch(&settings, &c, &args[1..]),
            _ => run_system(&args),
        },
    }
}
