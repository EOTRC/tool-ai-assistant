




pub struct AnsiStrip {
    leftover: String,
}

enum AnsiMode {
    None,
    Esc,
    Csi,
    Osc,
}

impl AnsiStrip {
    pub fn new() -> Self {
        AnsiStrip {
            leftover: String::new(),
        }
    }

    
    pub fn push(&mut self, chunk: &str, out: &mut String) {
        let mut s = std::mem::take(&mut self.leftover);
        s.push_str(chunk);
        let mut mode = AnsiMode::None;
        let mut buf = String::new();
        for c in s.chars() {
            match mode {
                AnsiMode::None => {
                    if c == '\u{1b}' {
                        mode = AnsiMode::Esc;
                        buf.clear();
                        buf.push(c);
                    } else {
                        out.push(c);
                    }
                }
                AnsiMode::Esc => {
                    buf.push(c);
                    if c == '[' {
                        mode = AnsiMode::Csi;
                    } else if c == ']' {
                        mode = AnsiMode::Osc;
                    } else {
                        mode = AnsiMode::None;
                    }
                }
                AnsiMode::Csi => {
                    buf.push(c);
                    let b = c as u32;
                    if (0x40..=0x7e).contains(&b) {
                        mode = AnsiMode::None;
                    }
                }
                AnsiMode::Osc => {
                    if c == '\u{07}' || c == '\u{1b}' {
                        mode = AnsiMode::None;
                    } else {
                        buf.push(c);
                    }
                }
            }
        }
        if let AnsiMode::None = mode {
            self.leftover.clear();
        } else {
            self.leftover = buf;
        }
    }
}


pub fn strip_ansi(s: &str) -> String {
    let mut strip = AnsiStrip::new();
    let mut out = String::new();
    strip.push(s, &mut out);
    out
}



pub fn strip_think(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</think>") {
            Some(end) => rest = &rest[start + end + 8..],
            None => return out.trim().to_string(),
        }
    }
    if let Some(end) = rest.find("</think>") {
        let before = &rest[..end];
        let after = &rest[end + 8..];
        if before.trim_start().starts_with("Хорошо") || !before.contains(':') {
            rest = after;
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}


pub fn clean_text(s: &str) -> String {
    strip_ansi(&strip_think(s))
}
