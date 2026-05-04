extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

use crate::wm::window::{Window, TITLE_H};

pub const BROWSER_W: i32 = 760;
pub const BROWSER_H: i32 = 520;

const CHAR_W: usize = 8;
const LINE_H: usize = 14;
const TOOLBAR_H: usize = 54;
const STATUS_H: usize = 18;
const PAD_X: usize = 16;
const REFRESH_BUTTON_X: i32 = 82;
const REFRESH_BUTTON_W: i32 = 72;
const ADDRESS_X: i32 = 162;
const SEARCH_BUTTON_W: i32 = 72;

const BG: u32 = 0x00_03_06_10;
const PAGE: u32 = 0x00_F2_F6_F8;
const TEXT: u32 = 0x00_12_1A_22;
const LINK: u32 = 0x00_00_66_CC;
const MUTED: u32 = 0x00_5D_6B_78;
const BAR: u32 = 0x00_11_1D_2B;
const BORDER: u32 = 0x00_3D_6D_91;
const BUTTON: u32 = 0x00_23_45_60;
const BUTTON_DIM: u32 = 0x00_19_2A_38;
const BUTTON_HOT: u32 = 0x00_00_9C_DD;
const WHITE: u32 = 0x00_FF_FF_FF;

#[derive(Clone)]
struct BrowserLine {
    text: String,
    link: Option<String>,
}

pub struct BrowserApp {
    pub window: Window,
    address: String,
    status: String,
    title: String,
    lines: Vec<BrowserLine>,
    history: Vec<String>,
    history_index: usize,
    scroll: usize,
    rows: usize,
    cols: usize,
    address_focused: bool,
    last_width: i32,
    last_height: i32,
}

impl BrowserApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, BROWSER_W, BROWSER_H, "Web Browser");
        let mut app = Self {
            window,
            address: String::from("http://example.com/"),
            status: String::from("Ready"),
            title: String::from("New tab"),
            lines: welcome_lines(),
            history: Vec::new(),
            history_index: 0,
            scroll: 0,
            rows: 0,
            cols: 0,
            address_focused: true,
            last_width: BROWSER_W,
            last_height: BROWSER_H,
        };
        app.render();
        app
    }

    pub fn handle_key(&mut self, c: char) {
        if self.address_focused {
            match c {
                '\n' | '\r' => {
                    let url = self.address.clone();
                    self.navigate(&url, true);
                }
                '\u{8}' | '\u{7f}' => {
                    self.address.pop();
                    self.render();
                }
                _ if !c.is_control() && self.address.len() < 192 => {
                    self.address.push(c);
                    self.render();
                }
                _ => {}
            }
            return;
        }

        match c {
            'j' | 'J' => self.scroll_by(1),
            'k' | 'K' => self.scroll_by(-1),
            'r' | 'R' => self.reload(),
            'g' | 'G' => {
                self.address_focused = true;
                self.render();
            }
            _ => {}
        }
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        if ly >= 10 && ly < 34 {
            if lx >= 14 && lx < 44 {
                self.back();
            } else if lx >= 48 && lx < 78 {
                self.forward();
            } else if lx >= REFRESH_BUTTON_X && lx < REFRESH_BUTTON_X + REFRESH_BUTTON_W {
                self.reload();
            } else {
                let search_x = self.window.width - (SEARCH_BUTTON_W + 16);
                if lx >= search_x && lx < search_x + SEARCH_BUTTON_W {
                    let url = self.address.clone();
                    self.navigate(&url, true);
                } else if lx >= ADDRESS_X && lx < search_x - 8 {
                    self.address_focused = true;
                    self.render();
                }
            }
            return;
        }

        self.address_focused = false;
        let doc_y0 = TOOLBAR_H as i32 + 10;
        if ly >= doc_y0 {
            let row = ((ly - doc_y0) as usize) / LINE_H;
            let idx = self.scroll + row;
            if let Some(line) = self.lines.get(idx) {
                if let Some(link) = line.link.clone() {
                    let resolved = resolve_url(&self.address, &link);
                    self.navigate(&resolved, true);
                    return;
                }
            }
        }
        self.render();
    }

    pub fn handle_scroll(&mut self, delta: i32) {
        self.scroll_by(delta.signum() * 3);
    }

    pub fn update(&mut self) {
        if self.window.width != self.last_width || self.window.height != self.last_height {
            self.last_width = self.window.width;
            self.last_height = self.window.height;
            self.render();
            return;
        }
        let expected = self.scroll as i32 * LINE_H as i32;
        if self.window.scroll.offset != expected {
            let max = self.lines.len().saturating_sub(self.rows);
            self.scroll = ((self.window.scroll.offset / LINE_H as i32) as usize).min(max);
            self.render();
        }
    }

    fn navigate(&mut self, url: &str, add_history: bool) {
        let url = normalize_url(url);
        self.address = url.clone();
        self.status = String::from("Loading...");
        self.title = String::from("Loading");
        self.lines = vec![BrowserLine {
            text: format!("Loading {}", url),
            link: None,
        }];
        self.scroll = 0;
        self.render();

        match parse_http_url(&url) {
            Ok((host, path)) => match crate::net::http_get_response(&host, &path) {
                Ok(response) => {
                    self.title = extract_title(&response.body).unwrap_or_else(|| host.clone());
                    self.status = format!(
                        "{}  {}{} -> {}",
                        response.status_line,
                        response.host,
                        response.path,
                        crate::net::ipv4_string(response.resolved_addr)
                    );
                    self.lines = render_document(&url, &response.body, self.cols.max(48));
                    if self.lines.is_empty() {
                        self.lines.push(BrowserLine {
                            text: String::from("(empty response)"),
                            link: None,
                        });
                    }
                    if add_history {
                        self.push_history(url);
                    }
                }
                Err(err) => {
                    self.title = String::from("Load failed");
                    self.status = format!("Network error: {}", err);
                    self.lines = vec![
                        BrowserLine {
                            text: String::from("Unable to load page"),
                            link: None,
                        },
                        BrowserLine {
                            text: format!("{}{}", host, path),
                            link: None,
                        },
                        BrowserLine {
                            text: String::from(err),
                            link: None,
                        },
                    ];
                }
            },
            Err(err) => {
                self.title = String::from("Unsupported URL");
                self.status = String::from(err);
                self.lines = vec![
                    BrowserLine {
                        text: String::from("Only plain HTTP URLs are supported in this milestone."),
                        link: None,
                    },
                    BrowserLine {
                        text: String::from("Try http://example.com/"),
                        link: Some(String::from("http://example.com/")),
                    },
                ];
            }
        }
        self.address_focused = false;
        self.render();
    }

    fn push_history(&mut self, url: String) {
        if self
            .history
            .get(self.history_index)
            .map(|current| current == &url)
            .unwrap_or(false)
        {
            return;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(url);
        self.history_index = self.history.len().saturating_sub(1);
    }

    fn back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(url) = self.history.get(self.history_index).cloned() {
                self.navigate(&url, false);
            }
        }
    }

    fn forward(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            if let Some(url) = self.history.get(self.history_index).cloned() {
                self.navigate(&url, false);
            }
        }
    }

    fn reload(&mut self) {
        let url = self.address.clone();
        self.navigate(&url, false);
    }

    fn scroll_by(&mut self, delta: i32) {
        let max = self.lines.len().saturating_sub(self.rows);
        let next = (self.scroll as i32 + delta).clamp(0, max as i32) as usize;
        if next != self.scroll {
            self.scroll = next;
            self.render();
        }
    }

    fn render(&mut self) {
        let width = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        if width == 0 || content_h == 0 {
            return;
        }
        let stride = width;
        for pixel in self.window.buf.iter_mut() {
            *pixel = BG;
        }
        self.fill_rect(stride, 0, 0, width, TOOLBAR_H, BAR);
        self.fill_rect(stride, 0, TOOLBAR_H - 1, width, 1, BORDER);
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(STATUS_H),
            width,
            STATUS_H,
            BAR,
        );
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(STATUS_H),
            width,
            1,
            BORDER,
        );

        self.draw_button(stride, 14, 10, 30, 24, "<", self.history_index > 0);
        self.draw_button(
            stride,
            48,
            10,
            30,
            24,
            ">",
            self.history_index + 1 < self.history.len(),
        );
        self.draw_button(
            stride,
            REFRESH_BUTTON_X as usize,
            10,
            REFRESH_BUTTON_W as usize,
            24,
            "Refresh",
            true,
        );

        let search_w = SEARCH_BUTTON_W as usize;
        let search_x = width.saturating_sub(search_w + 16);
        let addr_x = ADDRESS_X as usize;
        let addr_w = search_x.saturating_sub(addr_x + 8);
        self.fill_rect(
            stride,
            addr_x,
            10,
            addr_w,
            24,
            if self.address_focused { WHITE } else { 0x00_E2_E8_EC },
        );
        self.draw_rect(
            stride,
            addr_x,
            10,
            addr_w,
            24,
            if self.address_focused {
                BUTTON_HOT
            } else {
                BORDER
            },
        );
        let mut address = self.address.clone();
        truncate_chars(&mut address, addr_w.saturating_sub(14) / CHAR_W);
        self.put_str(stride, addr_x + 8, 17, &address, TEXT);
        self.draw_button(stride, search_x, 10, search_w, 24, "Search", true);

        let mut title = self.title.clone();
        truncate_chars(&mut title, width.saturating_sub(PAD_X * 2) / CHAR_W);
        self.put_str(stride, PAD_X, 40, &title, WHITE);

        let doc_y = TOOLBAR_H + 10;
        let doc_h = content_h.saturating_sub(TOOLBAR_H + STATUS_H + 18);
        self.fill_rect(stride, 10, TOOLBAR_H + 6, width.saturating_sub(20), doc_h + 8, PAGE);
        self.draw_rect(stride, 10, TOOLBAR_H + 6, width.saturating_sub(20), doc_h + 8, 0x00_BB_C6_CC);

        self.rows = doc_h / LINE_H;
        self.cols = width.saturating_sub(PAD_X * 2 + 28) / CHAR_W;
        self.window.scroll.content_h = (self.lines.len() * LINE_H) as i32;
        self.window.scroll.offset = self.scroll as i32 * LINE_H as i32;
        self.window.scroll.clamp((self.rows * LINE_H) as i32);

        for row in 0..self.rows {
            let idx = self.scroll + row;
            let Some(line) = self.lines.get(idx).cloned() else {
                break;
            };
            let y = doc_y + row * LINE_H;
            let color = if line.link.is_some() { LINK } else { TEXT };
            let mut text = line.text;
            truncate_chars(&mut text, self.cols);
            self.put_str(stride, PAD_X, y, &text, color);
            if line.link.is_some() {
                self.fill_rect(stride, PAD_X, y + 10, text.len().min(self.cols) * CHAR_W, 1, LINK);
            }
        }

        let mut status = self.status.clone();
        truncate_chars(&mut status, width.saturating_sub(PAD_X * 2) / CHAR_W);
        self.put_str(
            stride,
            PAD_X,
            content_h.saturating_sub(STATUS_H).saturating_add(5),
            &status,
            0x00_CC_DD_E8,
        );
        self.window.mark_dirty_all();
    }

    fn draw_button(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        label: &str,
        enabled: bool,
    ) {
        let bg = if enabled { BUTTON } else { BUTTON_DIM };
        self.fill_rect(stride, x, y, w, h, bg);
        self.draw_rect(stride, x, y, w, h, if enabled { BUTTON_HOT } else { BORDER });
        let label_x = x + 6;
        let label_y = y + 8;
        self.put_str(stride, label_x, label_y, label, if enabled { WHITE } else { MUTED });
    }

    fn fill_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let width = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        for row in y..(y + h).min(content_h) {
            let base = row * stride;
            for col in x..(x + w).min(width) {
                let idx = base + col;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = color;
                }
            }
        }
    }

    fn draw_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(stride, x, y, w, 1, color);
        self.fill_rect(stride, x, y + h - 1, w, 1, color);
        self.fill_rect(stride, x, y, 1, h, color);
        self.fill_rect(stride, x + w - 1, y, 1, h, color);
    }

    fn put_str(&mut self, stride: usize, px: usize, py: usize, s: &str, color: u32) {
        crate::font::draw_str(
            &mut self.window.buf,
            stride,
            px,
            py,
            s,
            color,
            None,
            crate::font::UI_FONT,
        );
    }
}

fn welcome_lines() -> Vec<BrowserLine> {
    vec![
        line("coolOS Browser"),
        line(""),
        line("Quick links"),
        line(""),
        link_line("Example Domain", "http://example.com/"),
    ]
}

fn line(text: &str) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: None,
    }
}

fn link_line(text: &str, url: &str) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: Some(String::from(url)),
    }
}

fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        String::from(trimmed)
    } else {
        let mut out = String::from("http://");
        out.push_str(trimmed);
        out
    }
}

fn parse_http_url(url: &str) -> Result<(String, String), &'static str> {
    if url.starts_with("https://") {
        return Err("HTTPS is not available until TLS lands");
    }
    let rest = url.strip_prefix("http://").ok_or("URL must start with http://")?;
    let slash = rest.find('/').unwrap_or(rest.len());
    let host = rest[..slash].trim();
    if host.is_empty() {
        return Err("missing host");
    }
    let path = if slash < rest.len() { &rest[slash..] } else { "/" };
    Ok((String::from(host), String::from(path)))
}

fn render_document(base_url: &str, response: &str, cols: usize) -> Vec<BrowserLine> {
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .or_else(|| response.split_once("\n\n").map(|(_, body)| body))
        .unwrap_or(response);
    if !body.contains('<') {
        return wrap_plain_text(body, cols, None);
    }
    let mut out = Vec::new();
    let mut text = String::new();
    let mut link: Option<String> = None;
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            flush_text(&mut out, &mut text, cols, link.clone());
            if let Some(end_rel) = body[i..].find('>') {
                let tag = &body[i + 1..i + end_rel];
                handle_tag(tag, &mut out, &mut link, base_url);
                i += end_rel + 1;
                continue;
            }
        }
        push_text_char(&mut text, bytes[i] as char);
        i += 1;
    }
    flush_text(&mut out, &mut text, cols, link);
    compact_lines(out)
}

fn handle_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    link: &mut Option<String>,
    base_url: &str,
) {
    let tag = tag.trim();
    let lower = lowercase_ascii(tag);
    if lower.starts_with("script") || lower.starts_with("style") {
        return;
    }
    if lower.starts_with("/a") {
        *link = None;
        return;
    }
    if lower.starts_with("a ") || lower == "a" {
        if let Some(href) = attr_value(tag, "href") {
            *link = Some(resolve_url(base_url, &href));
        }
        return;
    }
    if lower.starts_with("br")
        || lower.starts_with("/p")
        || lower.starts_with("p")
        || lower.starts_with("/div")
        || lower.starts_with("div")
        || lower.starts_with("/h")
        || lower.starts_with("h1")
        || lower.starts_with("h2")
        || lower.starts_with("h3")
        || lower.starts_with("li")
    {
        if out.last().map(|line| !line.text.is_empty()).unwrap_or(true) {
            out.push(BrowserLine {
                text: String::new(),
                link: None,
            });
        }
    }
}

fn flush_text(out: &mut Vec<BrowserLine>, text: &mut String, cols: usize, link: Option<String>) {
    let decoded = decode_entities(text.trim());
    text.clear();
    if decoded.is_empty() {
        return;
    }
    out.extend(wrap_plain_text(&decoded, cols, link));
}

fn wrap_plain_text(text: &str, cols: usize, link: Option<String>) -> Vec<BrowserLine> {
    let cols = cols.clamp(20, 120);
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let extra = if line.is_empty() { 0 } else { 1 };
        if line.len() + word.len() + extra > cols && !line.is_empty() {
            out.push(BrowserLine {
                text: line,
                link: link.clone(),
            });
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(BrowserLine { text: line, link });
    }
    out
}

fn compact_lines(lines: Vec<BrowserLine>) -> Vec<BrowserLine> {
    let mut out = Vec::new();
    let mut last_blank = false;
    for line in lines {
        let blank = line.text.trim().is_empty();
        if blank && last_blank {
            continue;
        }
        last_blank = blank;
        out.push(line);
    }
    out
}

fn push_text_char(out: &mut String, c: char) {
    if c == '\n' || c == '\r' || c == '\t' {
        if !out.ends_with(' ') {
            out.push(' ');
        }
    } else {
        out.push(c);
    }
}

fn decode_entities(input: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if input[i..].starts_with("&amp;") {
                out.push('&');
                i += 5;
                continue;
            }
            if input[i..].starts_with("&lt;") {
                out.push('<');
                i += 4;
                continue;
            }
            if input[i..].starts_with("&gt;") {
                out.push('>');
                i += 4;
                continue;
            }
            if input[i..].starts_with("&quot;") {
                out.push('"');
                i += 6;
                continue;
            }
            if input[i..].starts_with("&#39;") {
                out.push('\'');
                i += 5;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn extract_title(response: &str) -> Option<String> {
    let lower = lowercase_ascii(response);
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    Some(decode_entities(response[start..end].trim()))
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let lower = lowercase_ascii(tag);
    let needle = {
        let mut n = String::from(name);
        n.push('=');
        n
    };
    let pos = lower.find(&needle)? + needle.len();
    let bytes = tag.as_bytes();
    if pos >= bytes.len() {
        return None;
    }
    let quote = bytes[pos];
    if quote == b'"' || quote == b'\'' {
        let rest = &tag[pos + 1..];
        let end = rest.find(quote as char)?;
        Some(String::from(&rest[..end]))
    } else {
        let rest = &tag[pos..];
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        Some(String::from(&rest[..end]))
    }
}

fn resolve_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return String::from(href);
    }
    let Ok((host, path)) = parse_http_url(base) else {
        return normalize_url(href);
    };
    if href.starts_with('/') {
        let mut out = String::from("http://");
        out.push_str(&host);
        out.push_str(href);
        return out;
    }
    let mut dir = path;
    if let Some(pos) = dir.rfind('/') {
        dir.truncate(pos + 1);
    }
    let mut out = String::from("http://");
    out.push_str(&host);
    out.push_str(&dir);
    out.push_str(href);
    out
}

fn lowercase_ascii(input: &str) -> String {
    input
        .bytes()
        .map(|b| if b.is_ascii_uppercase() { b + 32 } else { b } as char)
        .collect()
}

fn truncate_chars(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    let mut out = String::new();
    for c in s.chars().take(max.saturating_sub(1)) {
        out.push(c);
    }
    out.push('>');
    *s = out;
}
