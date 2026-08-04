use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const SETTINGS_FILE: &str = "settings.cfg";

const TOOL_COMMANDS: &[&str] = &[
    "help", "chat", "shell", "screen", "status", "models", "settings", "todo",
    "convert", "ask", "ask-file", "code", "summarize", "translate", "search",
    "index", "clip", "ask-image", "web", "alias", "selftest", "install",
];

fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_aliases() -> Vec<(String, String)> {
    let path = exe_dir().join(SETTINGS_FILE);
    let raw = match fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut section = String::new();
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            section = t[1..t.len() - 1].trim().to_lowercase();
            continue;
        }
        if section == "aliases" {
            if let Some(eq) = t.find('=') {
                out.push((t[..eq].trim().to_string(), t[eq + 1..].trim().to_string()));
            }
        }
    }
    out
}

fn save_aliases(aliases: &[(String, String)]) {
    let path = exe_dir().join(SETTINGS_FILE);
    let raw = fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let mut in_aliases = false;
    let mut had_aliases = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_aliases = t[1..t.len() - 1].trim().eq_ignore_ascii_case("aliases");
            if in_aliases {
                had_aliases = true;
            }
            lines.push(line.to_string());
            continue;
        }
        if in_aliases && t.contains('=') && !t.starts_with('#') && !t.starts_with(';') {
            continue;
        }
        lines.push(line.to_string());
    }
    if !had_aliases {
        if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.push(String::new());
        }
        lines.push("[aliases]".to_string());
    }
    for (k, v) in aliases {
        lines.push(format!("{} = {}", k, v));
    }
    let _ = fs::write(&path, lines.join("\n"));
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut inq = false;
    for c in s.chars() {
        match c {
            '"' => inq = !inq,
            ' ' if !inq => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn find_tool_binary() -> Option<PathBuf> {
    let dir = exe_dir();
    #[cfg(target_os = "windows")]
    let exact: &[&str] = &["Tool.exe", "tool.exe"];
    #[cfg(not(target_os = "windows"))]
    let exact: &[&str] = &["tool"];
    for name in exact {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    let cur = env::current_exe().ok();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let base = name.to_lowercase();
            if base == "toolcmd" || base == "toolcmd.exe" || base.starts_with("toolcmd-") {
                continue;
            }
            if base == "tool" || base == "tool.exe" || base.starts_with("tool-") {
                if cur.as_ref().map(|c| e.path() == *c).unwrap_or(false) {
                    continue;
                }
                if e.path().is_file() {
                    return Some(e.path());
                }
            }
        }
    }
    None
}

fn exec_tool(args: &[String]) {
    let exe = match find_tool_binary() {
        Some(p) => p,
        None => {
            eprintln!("Не найден бинарь Tool рядом с toolcmd. Положи их в одну папку.");
            return;
        }
    };
    match Command::new(&exe).args(args).status() {
        Ok(_) => {}
        Err(e) => eprintln!("Ошибка запуска Tool: {}", e),
    }
}


#[cfg(not(target_os = "windows"))]
fn detected_shell() -> String {
    let sh = env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());
    sh.split_whitespace().next().unwrap_or("/bin/sh").to_string()
}

fn run_shell(line: &str) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/c", line]).status()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(&detected_shell()).args(["-c", line]).status()
    }
}

fn run_line(line: String, aliases: &[(String, String)]) {
    let line = line
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .to_string();
    if line.is_empty() {
        return;
    }
    let first = line.split_whitespace().next().unwrap_or("").to_lowercase();
    if first == "tool" {
        let rest = line[4..].trim();
        let args = split_args(rest);
        exec_tool(&args);
        return;
    }
    if let Some((_, val)) = aliases.iter().find(|(k, _)| k.eq_ignore_ascii_case(&first)) {
        let rest = line[first.len()..].trim();
        let expanded = if rest.is_empty() {
            val.clone()
        } else {
            format!("{} {}", val, rest)
        };
        run_line(expanded, aliases);
        return;
    }
    if TOOL_COMMANDS.contains(&first.as_str()) {
        let args = split_args(&line);
        exec_tool(&args);
        return;
    }
    match run_shell(&line) {
        Ok(_) => {}
        Err(e) => eprintln!("Ошибка: {}", e),
    }
}

fn main() {
    let mut aliases = load_aliases();
    if cfg!(target_os = "windows") {
        println!("ToolCmd — консоль. Команды Tool работают без префикса, остальное — в cmd.");
    } else {
        println!("ToolCmd — консоль. Команды Tool работают без префикса, остальное — в sh.");
    }
    println!("Справка с примерами: help    Выход: exit    Алиасы: alias имя=команда");
    loop {
        let dir = env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
        print!("{}> ", dir);
        io::stdout().flush().ok();
        let mut buffer = String::new();
        match io::stdin().read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let line = buffer
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .to_string();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if matches!(lower.as_str(), "exit" | "quit") {
            break;
        }
        if lower == "cls" || lower == "clear" {
            run_shell(if cfg!(target_os = "windows") { "cls" } else { "clear" }).ok();
            continue;
        }
        if lower == "help" {
            println!("Tool — локальный ИИ-ассистент. Команды вводятся без кавычек,");
            println!("фраза считается до конца строки (или до флага --...).");
            println!();
            println!("  Общение и поиск:");
            println!("    chat                      диалог с ИИ (exit/пока — выход)");
            println!("    chat --once найди погоду   один ответ, сразу");
            println!("    web температура в москве    поиск в интернете (не ИИ)");
            println!("    status / models            проверка Ollama / список моделей");
            println!("    install ollama|models     установить Ollama или ИИ-модели");
            println!();
            println!("  Файлы и документы:");
            println!("    convert file.pdf --out f.txt    файл в текст");
            if cfg!(target_os = "windows") {
                println!("    index C:\\documents               построить поиск по папке");
            } else {
                println!("    index ~/documents                построить поиск по папке");
            }
            println!("    search погода в москве           поиск по индексу");
            println!("    ask какие выводы в материалах    вопрос по индексу (RAG)");
            println!("    ask-file отчёт.docx сделай резюме");
            println!("    code main.py найди баги");
            if cfg!(target_os = "windows") {
                println!("    summarize C:\\documents           резюме папки");
            } else {
                println!("    summarize ~/documents            резюме папки");
            }
            println!("    translate hello world --to русский");
            println!("    clip сделай резюме               текст из буфера обмена");
            println!("    screen                           что на экране (скриншот)");
            println!();
            println!("  Служебные: settings (настройки), alias (алиасы), exit");
            if cfg!(target_os = "windows") {
                println!("  Обычные команды Windows (dir, cd, del...) работают как в cmd.");
            } else {
                println!("  Обычные команды системы (ls, cd, pwd...) работают как в sh.");
            }
            continue;
        }
        if lower == "aliases" {
            if aliases.is_empty() {
                println!("Алиасов нет.");
            }
            for (k, v) in &aliases {
                println!("{} = {}", k, v);
            }
            continue;
        }
        if lower.starts_with("alias ") {
            let rest = &line[6..];
            if let Some(eq) = rest.find('=') {
                let name = rest[..eq].trim().to_lowercase();
                let val = rest[eq + 1..].trim().to_string();
                if !name.is_empty() {
                    aliases.retain(|(k, _)| k != &name);
                    if !val.is_empty() {
                        aliases.push((name, val));
                    }
                    save_aliases(&aliases);
                    println!("Алиас сохранён в settings.cfg [aliases].");
                }
            } else {
                println!("Формат: alias имя=команда");
            }
            continue;
        }
        if lower.starts_with("cd") {
            let p = &line[3..].trim();
            if p.is_empty() {
                if let Ok(d) = env::current_dir() {
                    println!("{}", d.display());
                }
                continue;
            }
            if env::set_current_dir(p).is_err() {
                eprintln!("Не удалось перейти в '{}'", p);
            }
            continue;
        }
        run_line(line, &aliases);
    }
    println!();
}
