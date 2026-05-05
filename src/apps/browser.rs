extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

use super::filemanager::FileManagerOpenRequest;
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
const BOOKMARKS_PATH: &str = "/CONFIG/BROWSER.CFG";
const DOWNLOADS_DIR: &str = "/Downloads";
const MAX_BOOKMARKS: usize = 32;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserLineKind {
    Text,
    Heading,
    Muted,
    Link,
    Image,
    Code,
    Error,
}

#[derive(Clone)]
struct BrowserLine {
    text: String,
    link: Option<String>,
    kind: BrowserLineKind,
}

#[derive(Clone)]
struct CachedPage {
    url: String,
    body: String,
    body_bytes: Vec<u8>,
    content_type: Option<String>,
}

pub struct BrowserApp {
    pub window: Window,
    address: String,
    status: String,
    title: String,
    lines: Vec<BrowserLine>,
    history: Vec<String>,
    bookmarks: Vec<String>,
    history_index: usize,
    scroll: usize,
    rows: usize,
    cols: usize,
    address_focused: bool,
    address_selected: bool,
    last_width: i32,
    last_height: i32,
    last_page: Option<CachedPage>,
    pending_open: Option<FileManagerOpenRequest>,
}

impl BrowserApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, BROWSER_W, BROWSER_H, "Web Browser");
        let mut app = Self {
            window,
            address: String::from("browser://home"),
            status: String::from("Ready"),
            title: String::from("New tab"),
            lines: welcome_lines(),
            history: Vec::new(),
            bookmarks: load_bookmarks(),
            history_index: 0,
            scroll: 0,
            rows: 0,
            cols: 0,
            address_focused: true,
            address_selected: true,
            last_width: BROWSER_W,
            last_height: BROWSER_H,
            last_page: None,
            pending_open: None,
        };
        app.render();
        app
    }

    pub fn open_url(x: i32, y: i32, url: &str) -> Self {
        let mut app = Self::new(x, y);
        app.navigate(url, true);
        app
    }

    pub fn handle_key(&mut self, c: char) {
        if self.address_focused {
            match c {
                '\n' | '\r' => {
                    let url = self.address.clone();
                    self.address_selected = false;
                    self.navigate(&url, true);
                }
                '\u{8}' | '\u{7f}' => {
                    if self.address_selected {
                        self.address.clear();
                        self.address_selected = false;
                    } else {
                        self.address.pop();
                    }
                    self.render();
                }
                _ if !c.is_control() && self.address.len() < 192 => {
                    if self.address_selected {
                        self.address.clear();
                        self.address_selected = false;
                    }
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
            'b' | 'B' => self.bookmark_current(),
            'd' | 'D' => self.save_current_page(false),
            'h' | 'H' => self.navigate("browser://history", true),
            'm' | 'M' => self.navigate("browser://bookmarks", true),
            'o' | 'O' => self.open_downloads_folder(),
            's' | 'S' => self.save_current_page(true),
            'g' | 'G' => {
                self.address_focused = true;
                self.address_selected = true;
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
                    self.address_selected = false;
                    self.navigate(&url, true);
                } else if lx >= ADDRESS_X && lx < search_x - 8 {
                    self.address_focused = true;
                    self.address_selected = true;
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
                    if self.open_file_url(&resolved) {
                        return;
                    }
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

    pub fn take_open_request(&mut self) -> Option<FileManagerOpenRequest> {
        self.pending_open.take()
    }

    fn navigate(&mut self, url: &str, add_history: bool) {
        let url = normalize_address_input(url);
        self.address = url.clone();
        if self.render_internal_page(&url, add_history) {
            return;
        }
        if self.open_file_url(&url) {
            if add_history {
                self.push_history(url);
            }
            return;
        }
        self.status = String::from("Loading...");
        self.title = String::from("Loading");
        self.lines = vec![BrowserLine {
            text: format!("Loading {}", url),
            link: None,
            kind: BrowserLineKind::Muted,
        }];
        self.scroll = 0;
        self.render();

        match parse_web_url(&url) {
            Ok((_scheme, host, path)) => match crate::net::web_get_response(&url) {
                Ok(response) => {
                    self.title = extract_title(&response.body).unwrap_or_else(|| host.clone());
                    self.address = response.final_url.clone();
                    let security = match response.tls_trust_root {
                        Some(root) => format!("  TLS root: {}", root),
                        None => String::new(),
                    };
                    self.status = if response.redirect_count > 0 {
                        format!(
                            "{}  {} redirect(s) -> {}{}",
                            response.status_line,
                            response.redirect_count,
                            response.final_url,
                            security
                        )
                    } else {
                        format!(
                            "{}  {}{} -> {}{}",
                            response.status_line,
                            response.host,
                            response.path,
                            crate::net::ipv4_string(response.resolved_addr),
                            security
                        )
                    };
                    self.last_page = Some(CachedPage {
                        url: response.final_url.clone(),
                        body: response.body.clone(),
                        body_bytes: response.body_bytes.clone(),
                        content_type: response.content_type.clone(),
                    });
                    self.lines = if is_image_content(response.content_type.as_deref()) {
                        image_response_lines(&response)
                    } else {
                        render_document(&response.final_url, &response.body, self.cols.max(48))
                    };
                    if self.lines.is_empty() {
                        self.lines.push(BrowserLine {
                            text: String::from("(empty response)"),
                            link: None,
                            kind: BrowserLineKind::Muted,
                        });
                    }
                    if add_history {
                        self.push_history(response.final_url);
                    }
                }
                Err(err) => {
                    self.title = String::from("Load failed");
                    self.status = format!("Network error: {}", err);
                    self.last_page = None;
                    self.lines = vec![
                        kind_line("Unable to load page", BrowserLineKind::Error),
                        line(&format!("{}{}", host, path)),
                        kind_line(err, BrowserLineKind::Muted),
                    ];
                }
            },
            Err(err) => {
                self.title = String::from("Unsupported URL");
                self.status = String::from(err);
                self.last_page = None;
                self.lines = vec![
                    BrowserLine {
                        text: String::from("Enter an http:// or https:// URL."),
                        link: None,
                        kind: BrowserLineKind::Text,
                    },
                    BrowserLine {
                        text: String::from("Try https://example.com/"),
                        link: Some(String::from("https://example.com/")),
                        kind: BrowserLineKind::Link,
                    },
                ];
            }
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
    }

    fn render_internal_page(&mut self, url: &str, add_history: bool) -> bool {
        if !url.starts_with("browser://") {
            return false;
        }
        self.scroll = 0;
        self.address_focused = false;
        self.address_selected = false;
        self.last_page = None;
        if add_history {
            self.push_history(String::from(url));
        }
        match url {
            "browser://home" => {
                self.title = String::from("Home");
                self.status = String::from("Ready");
                self.lines = welcome_lines();
            }
            "browser://history" => {
                self.title = String::from("History");
                self.status = format!("{} item(s)", self.history.len());
                self.lines = history_lines(&self.history);
            }
            "browser://bookmarks" => {
                self.title = String::from("Bookmarks");
                self.status = format!("{} bookmark(s)", self.bookmarks.len());
                self.lines = bookmark_lines(&self.bookmarks);
            }
            "browser://downloads" => {
                self.title = String::from("Downloads");
                self.status = String::from(DOWNLOADS_DIR);
                self.lines = downloads_lines();
            }
            _ if url.starts_with("browser://search?q=") => {
                let query = decode_query(&url["browser://search?q=".len()..]);
                self.title = String::from("Search");
                self.status = format!("Local search: {}", query);
                self.lines = self.search_lines(&query);
            }
            _ => {
                self.title = String::from("Browser");
                self.status = String::from("Unknown internal page");
                self.lines = vec![
                    line("Page not found"),
                    line(""),
                    link_line("Home", "browser://home"),
                ];
            }
        }
        self.render();
        true
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

    fn bookmark_current(&mut self) {
        let url = self.address.clone();
        if url.starts_with("browser://") {
            self.status = String::from("Internal pages are not bookmarked");
            self.render();
            return;
        }
        if !self.bookmarks.iter().any(|bookmark| bookmark == &url) {
            self.bookmarks.push(url.clone());
            if self.bookmarks.len() > MAX_BOOKMARKS {
                self.bookmarks.remove(0);
            }
            save_bookmarks(&self.bookmarks);
            self.status = format!("Bookmarked {}", url);
        } else {
            self.status = format!("Already bookmarked {}", url);
        }
        self.render();
    }

    fn save_current_page(&mut self, source: bool) {
        let Some(page) = self.last_page.clone() else {
            self.status = String::from("Nothing loaded to save");
            self.render();
            return;
        };
        let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
        let filename = download_filename(&page.url, page.content_type.as_deref(), source);
        let mut path = String::from(DOWNLOADS_DIR);
        path.push('/');
        path.push_str(&filename);
        let data = if source {
            response_body_text(&page.body)
                .unwrap_or(page.body.as_str())
                .as_bytes()
                .to_vec()
        } else {
            page.body_bytes
        };
        match crate::vfs::vfs_safe_write_file(&path, &data) {
            Ok(()) => {
                self.status = format!("Saved {}", path);
                self.lines = vec![
                    kind_line("Saved download", BrowserLineKind::Heading),
                    line(""),
                    link_line(&path, &file_url_for_path(&path)),
                    link_line("Open Downloads", "browser://downloads"),
                ];
            }
            Err(err) => {
                self.status = format!("Save failed: {}", err.as_str());
            }
        }
        self.render();
    }

    fn open_downloads_folder(&mut self) {
        let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
        self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(DOWNLOADS_DIR)));
        self.status = String::from("Opening Downloads in File Manager");
        self.render();
    }

    fn open_file_url(&mut self, url: &str) -> bool {
        let Some(path) = url.strip_prefix("file://") else {
            return false;
        };
        let path = if path.is_empty() { "/" } else { path };
        if crate::vfs::vfs_list_dir(path).is_some() {
            self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(path)));
            self.status = format!("Opening {}", path);
        } else if crate::vfs::vfs_read_file(path).is_some() {
            self.pending_open = Some(FileManagerOpenRequest::File(String::from(path)));
            self.status = format!("Opening {}", path);
        } else {
            self.title = String::from("File not found");
            self.status = format!("Missing {}", path);
            self.lines = vec![
                kind_line("File not found", BrowserLineKind::Error),
                kind_line(path, BrowserLineKind::Muted),
                link_line("Downloads", "browser://downloads"),
            ];
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
        true
    }

    fn search_lines(&self, query: &str) -> Vec<BrowserLine> {
        let mut out = vec![line("Search"), line("")];
        let query_lower = lowercase_ascii(query);
        let mut matches = 0usize;
        for url in self
            .history
            .iter()
            .chain(self.bookmarks.iter())
            .filter(|url| lowercase_ascii(url).contains(&query_lower))
        {
            out.push(link_line(url, url));
            matches += 1;
        }
        if matches == 0 {
            out.push(line("No local matches."));
            if looks_like_url(query) {
                out.push(line(""));
                out.push(link_line(
                    "Open as web URL",
                    &normalize_address_input(query),
                ));
            }
        }
        out
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
        let address_bg = if self.address_focused && self.address_selected {
            0x00_00_66_CC
        } else if self.address_focused {
            WHITE
        } else {
            0x00_E2_E8_EC
        };
        self.fill_rect(stride, addr_x, 10, addr_w, 24, address_bg);
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
        let address_text = if self.address_focused && self.address_selected {
            WHITE
        } else {
            TEXT
        };
        self.put_str(stride, addr_x + 8, 17, &address, address_text);
        self.draw_button(stride, search_x, 10, search_w, 24, "Search", true);

        let mut title = self.title.clone();
        truncate_chars(&mut title, width.saturating_sub(PAD_X * 2) / CHAR_W);
        self.put_str(stride, PAD_X, 40, &title, WHITE);

        let doc_y = TOOLBAR_H + 10;
        let doc_h = content_h.saturating_sub(TOOLBAR_H + STATUS_H + 18);
        self.fill_rect(
            stride,
            10,
            TOOLBAR_H + 6,
            width.saturating_sub(20),
            doc_h + 8,
            PAGE,
        );
        self.draw_rect(
            stride,
            10,
            TOOLBAR_H + 6,
            width.saturating_sub(20),
            doc_h + 8,
            0x00_BB_C6_CC,
        );

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
            let color = match line.kind {
                BrowserLineKind::Heading => 0x00_04_24_3A,
                BrowserLineKind::Muted => MUTED,
                BrowserLineKind::Link => LINK,
                BrowserLineKind::Image => 0x00_7A_3B_00,
                BrowserLineKind::Code => 0x00_22_33_33,
                BrowserLineKind::Error => 0x00_AA_20_20,
                BrowserLineKind::Text => {
                    if line.link.is_some() {
                        LINK
                    } else {
                        TEXT
                    }
                }
            };
            let mut text = line.text;
            truncate_chars(&mut text, self.cols);
            self.put_str(stride, PAD_X, y, &text, color);
            if line.kind == BrowserLineKind::Heading {
                self.put_str(stride, PAD_X + 1, y, &text, color);
            }
            if line.link.is_some() {
                self.fill_rect(
                    stride,
                    PAD_X,
                    y + 10,
                    text.len().min(self.cols) * CHAR_W,
                    1,
                    LINK,
                );
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
        self.draw_rect(
            stride,
            x,
            y,
            w,
            h,
            if enabled { BUTTON_HOT } else { BORDER },
        );
        let label_x = x + 6;
        let label_y = y + 8;
        self.put_str(
            stride,
            label_x,
            label_y,
            label,
            if enabled { WHITE } else { MUTED },
        );
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
        kind_line("coolOS Browser", BrowserLineKind::Heading),
        line(""),
        kind_line("Quick links", BrowserLineKind::Muted),
        line(""),
        link_line("Example Domain", "https://example.com/"),
        link_line("History", "browser://history"),
        link_line("Bookmarks", "browser://bookmarks"),
        link_line("Downloads", "browser://downloads"),
    ]
}

fn load_bookmarks() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(bytes) = crate::config_store::read(BOOKMARKS_PATH) {
        if let Ok(text) = core::str::from_utf8(&bytes) {
            for line in text.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                if key.trim() != "bookmark" {
                    continue;
                }
                let url = value.trim();
                if (url.starts_with("http://") || url.starts_with("https://"))
                    && !out.iter().any(|existing| existing == url)
                    && out.len() < MAX_BOOKMARKS
                {
                    out.push(String::from(url));
                }
            }
        }
    }
    if out.is_empty() {
        out.push(String::from("https://example.com/"));
    }
    out
}

fn save_bookmarks(bookmarks: &[String]) {
    let mut out = String::new();
    for bookmark in bookmarks.iter().take(MAX_BOOKMARKS) {
        if !(bookmark.starts_with("http://") || bookmark.starts_with("https://")) {
            continue;
        }
        out.push_str("bookmark=");
        out.push_str(bookmark);
        out.push('\n');
    }
    let _ = crate::config_store::safe_write(BOOKMARKS_PATH, out.as_bytes());
}

fn history_lines(history: &[String]) -> Vec<BrowserLine> {
    let mut out = vec![kind_line("History", BrowserLineKind::Heading), line("")];
    if history.is_empty() {
        out.push(kind_line("No pages visited yet.", BrowserLineKind::Muted));
        return out;
    }
    out.push(kind_line("Recently visited", BrowserLineKind::Muted));
    for url in history.iter().rev().take(32) {
        out.push(link_line(url, url));
    }
    out
}

fn bookmark_lines(bookmarks: &[String]) -> Vec<BrowserLine> {
    let mut out = vec![kind_line("Bookmarks", BrowserLineKind::Heading), line("")];
    if bookmarks.is_empty() {
        out.push(kind_line("No bookmarks yet.", BrowserLineKind::Muted));
        return out;
    }
    out.push(kind_line("Saved pages", BrowserLineKind::Muted));
    for url in bookmarks {
        out.push(link_line(url, url));
    }
    out
}

fn downloads_lines() -> Vec<BrowserLine> {
    let _ = crate::vfs::vfs_create_dir(DOWNLOADS_DIR);
    let mut out = vec![
        kind_line("Downloads", BrowserLineKind::Heading),
        line(""),
        link_line(
            "Open Downloads in File Manager",
            &file_url_for_path(DOWNLOADS_DIR),
        ),
        line(""),
    ];
    let Some(mut entries) = crate::vfs::vfs_list_dir(DOWNLOADS_DIR) else {
        out.push(kind_line(
            "Downloads folder unavailable.",
            BrowserLineKind::Error,
        ));
        return out;
    };
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    if entries.is_empty() {
        out.push(kind_line(
            "No downloaded files yet.",
            BrowserLineKind::Muted,
        ));
        return out;
    }
    out.push(kind_line("Files", BrowserLineKind::Muted));
    for entry in entries.into_iter().take(48) {
        let mut path = String::from(DOWNLOADS_DIR);
        path.push('/');
        path.push_str(&entry.name);
        let label = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            format!("{}  {} bytes", entry.name, entry.size)
        };
        out.push(link_line(&label, &file_url_for_path(&path)));
    }
    out
}

fn image_response_lines(response: &crate::net::HttpResponse) -> Vec<BrowserLine> {
    let mut out = vec![
        kind_line("Image", BrowserLineKind::Heading),
        kind_line(
            response.content_type.as_deref().unwrap_or("image/*"),
            BrowserLineKind::Muted,
        ),
        line(""),
    ];
    out.push(BrowserLine {
        text: format!("{} bytes received", response.body_bytes.len()),
        link: None,
        kind: BrowserLineKind::Image,
    });
    out.push(link_line("Image source URL", &response.final_url));
    out
}

fn is_image_content(content_type: Option<&str>) -> bool {
    content_type
        .map(|value| value.trim().to_ascii_lowercase().starts_with("image/"))
        .unwrap_or(false)
}

fn response_body_text(response: &str) -> Option<&str> {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .or_else(|| response.split_once("\n\n").map(|(_, body)| body))
}

fn file_url_for_path(path: &str) -> String {
    let mut out = String::from("file://");
    out.push_str(path);
    out
}

fn download_filename(url: &str, content_type: Option<&str>, source: bool) -> String {
    let (_scheme, host, path) = parse_web_url(url).unwrap_or_else(|_| {
        (
            String::from("web"),
            String::from("download"),
            String::from("/index"),
        )
    });
    let ext = if source {
        "html"
    } else {
        extension_for_content_type(content_type).unwrap_or_else(|| extension_from_path(&path))
    };
    let mut stem = String::new();
    stem.push_str(&sanitize_filename_part(&host));
    let leaf = path
        .split('?')
        .next()
        .unwrap_or("/")
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("index");
    stem.push('-');
    stem.push_str(&sanitize_filename_part(leaf));
    if stem.len() > 72 {
        stem.truncate(72);
    }
    if stem.ends_with('-') {
        stem.push_str("page");
    }
    stem.push('.');
    stem.push_str(ext);
    stem
}

fn extension_for_content_type(content_type: Option<&str>) -> Option<&'static str> {
    let value = content_type?.split(';').next()?.trim();
    if value.eq_ignore_ascii_case("text/html") {
        Some("html")
    } else if value.eq_ignore_ascii_case("text/plain") {
        Some("txt")
    } else if value.eq_ignore_ascii_case("image/png") {
        Some("png")
    } else if value.eq_ignore_ascii_case("image/jpeg") || value.eq_ignore_ascii_case("image/jpg") {
        Some("jpg")
    } else if value.eq_ignore_ascii_case("image/gif") {
        Some("gif")
    } else if value.eq_ignore_ascii_case("image/webp") {
        Some("webp")
    } else {
        None
    }
}

fn extension_from_path(path: &str) -> &'static str {
    let leaf = path.split('?').next().unwrap_or(path);
    if leaf.ends_with(".html") || leaf.ends_with(".htm") {
        "html"
    } else if leaf.ends_with(".txt") {
        "txt"
    } else if leaf.ends_with(".png") {
        "png"
    } else if leaf.ends_with(".jpg") || leaf.ends_with(".jpeg") {
        "jpg"
    } else if leaf.ends_with(".gif") {
        "gif"
    } else {
        "bin"
    }
}

fn sanitize_filename_part(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        let b = b.to_ascii_lowercase();
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else if matches!(b, b'.' | b'-' | b'_') {
            out.push(b as char);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.starts_with('.') || out.starts_with('-') {
        out.remove(0);
    }
    while out.ends_with('.') || out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        String::from("download")
    } else {
        out
    }
}

fn line(text: &str) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: None,
        kind: BrowserLineKind::Text,
    }
}

fn kind_line(text: &str, kind: BrowserLineKind) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: None,
        kind,
    }
}

fn link_line(text: &str, url: &str) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: Some(String::from(url)),
        kind: BrowserLineKind::Link,
    }
}

fn normalize_address_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("browser://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        String::from(trimmed)
    } else if looks_like_url(trimmed) {
        let mut out = String::from("https://");
        out.push_str(trimmed);
        out
    } else {
        let mut out = String::from("browser://search?q=");
        push_query_encoded(&mut out, trimmed);
        out
    }
}

fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("browser://")
        || trimmed.starts_with("file://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        String::from(trimmed)
    } else {
        let mut out = String::from("http://");
        out.push_str(trimmed);
        out
    }
}

fn looks_like_url(input: &str) -> bool {
    input.contains('.')
        || input.starts_with("localhost")
        || input.starts_with("10.")
        || input.starts_with("172.")
        || input.starts_with("192.168.")
}

fn push_query_encoded(out: &mut String, input: &str) {
    for b in input.bytes() {
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => out.push('+'),
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' => out.push(b as char),
            _ => {
                out.push('%');
                out.push(hex_digit((b >> 4) & 0x0f));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
}

fn decode_query(input: &str) -> String {
    let mut out = String::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push(((hi << 4) | lo) as char);
                i += 3;
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'A' + value - 10) as char,
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_web_url(url: &str) -> Result<(String, String, String), &'static str> {
    if let Some(rest) = url.strip_prefix("http://") {
        let (host, path) = parse_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = url.strip_prefix("https://") {
        let (host, path) = parse_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    Err("URL must start with http:// or https://")
}

fn parse_web_host_path(scheme: &str, rest: &str) -> Result<(String, String), &'static str> {
    let slash = rest.find('/').unwrap_or(rest.len());
    let mut host = rest[..slash].trim();
    if host.is_empty() {
        return Err("missing host");
    }
    if let Some((name, port)) = host.rsplit_once(':') {
        let expected_port = if scheme == "https" { "443" } else { "80" };
        if port != expected_port {
            return Err("only default web ports are supported");
        }
        host = name;
    }
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        "/"
    };
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
    let mut kind = BrowserLineKind::Text;
    let mut pending_prefix: Option<String> = None;
    let mut preformatted = false;
    let mut skip_until: Option<String> = None;
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(end_rel) = body[i..].find('>') {
                let tag = &body[i + 1..i + end_rel];
                let lower_tag = lowercase_ascii(tag.trim());
                if let Some(end_tag) = skip_until.as_ref() {
                    if lower_tag.starts_with(end_tag) {
                        skip_until = None;
                    }
                    i += end_rel + 1;
                    continue;
                }
                flush_text(
                    &mut out,
                    &mut text,
                    cols,
                    link.clone(),
                    kind,
                    pending_prefix.take(),
                );
                let lower_name = tag_name_of(&lower_tag);
                if lower_name == "script"
                    || lower_name == "style"
                    || lower_name == "head"
                    || (tag_is_hidden(&lower_tag) && !lower_tag.starts_with("input"))
                {
                    skip_until = Some(closing_tag_for(&lower_tag));
                    i += end_rel + 1;
                    continue;
                }
                handle_tag(
                    tag,
                    &mut out,
                    &mut link,
                    &mut kind,
                    &mut pending_prefix,
                    &mut preformatted,
                    base_url,
                    cols,
                );
                i += end_rel + 1;
                continue;
            }
        }
        if skip_until.is_none() {
            push_text_char(&mut text, bytes[i] as char, preformatted);
        }
        i += 1;
    }
    flush_text(&mut out, &mut text, cols, link, kind, pending_prefix);
    compact_lines(out)
}

fn handle_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    link: &mut Option<String>,
    kind: &mut BrowserLineKind,
    pending_prefix: &mut Option<String>,
    preformatted: &mut bool,
    base_url: &str,
    _cols: usize,
) {
    let tag = tag.trim();
    let lower = lowercase_ascii(tag);
    if lower.starts_with("/a") {
        *link = None;
        return;
    }
    if lower.starts_with("/h") {
        *kind = BrowserLineKind::Text;
        push_blank_line(out);
        return;
    }
    if lower.starts_with("/pre") || lower.starts_with("/code") {
        *preformatted = false;
        *kind = BrowserLineKind::Text;
        push_blank_line(out);
        return;
    }
    if lower.starts_with("a ") || lower == "a" {
        if let Some(href) = attr_value(tag, "href") {
            *link = Some(resolve_url(base_url, &href));
        }
        return;
    }
    if lower.starts_with("h1") || lower.starts_with("h2") || lower.starts_with("h3") {
        push_blank_line(out);
        *kind = BrowserLineKind::Heading;
        return;
    }
    if lower.starts_with("pre") || lower.starts_with("code") {
        push_blank_line(out);
        *preformatted = true;
        *kind = BrowserLineKind::Code;
        return;
    }
    if lower.starts_with("img") {
        let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
        let label = attr_value(tag, "alt")
            .map(|alt| decode_entities(&alt))
            .filter(|alt| !alt.trim().is_empty())
            .unwrap_or_else(|| src.clone().unwrap_or_else(|| String::from("image")));
        let mut text = String::from("[image] ");
        text.push_str(&label);
        out.push(BrowserLine {
            text,
            link: src,
            kind: BrowserLineKind::Image,
        });
        return;
    }
    if lower.starts_with("li") {
        push_blank_line(out);
        *pending_prefix = Some(String::from("* "));
        return;
    }
    if lower.starts_with("br") {
        push_blank_line(out);
        return;
    }
    if lower.starts_with("/p")
        || lower.starts_with("p")
        || lower.starts_with("/div")
        || lower.starts_with("div")
        || lower.starts_with("/section")
        || lower.starts_with("section")
        || lower.starts_with("/article")
        || lower.starts_with("article")
        || lower.starts_with("/main")
        || lower.starts_with("main")
        || lower.starts_with("/tr")
        || lower.starts_with("tr")
        || lower.starts_with("/ul")
        || lower.starts_with("/ol")
        || lower.starts_with("ul")
        || lower.starts_with("ol")
    {
        push_blank_line(out);
    }
}

fn push_blank_line(out: &mut Vec<BrowserLine>) {
    if out.last().map(|line| !line.text.is_empty()).unwrap_or(true) {
        out.push(BrowserLine {
            text: String::new(),
            link: None,
            kind: BrowserLineKind::Text,
        });
    }
}

fn tag_is_hidden(lower_tag: &str) -> bool {
    lower_tag.contains("hidden")
        || lower_tag.contains("display:none")
        || lower_tag.contains("display: none")
}

fn tag_name_of(lower_tag: &str) -> &str {
    lower_tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
}

fn closing_tag_for(lower_tag: &str) -> String {
    let mut out = String::from("/");
    out.push_str(tag_name_of(lower_tag));
    out
}

fn flush_text(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
    prefix: Option<String>,
) {
    let trimmed = if kind == BrowserLineKind::Code {
        text.trim_matches('\n')
    } else {
        text.trim()
    };
    let mut decoded = decode_entities(trimmed);
    text.clear();
    if decoded.is_empty() {
        return;
    }
    if let Some(prefix) = prefix {
        decoded.insert_str(0, &prefix);
    }
    out.extend(wrap_plain_text_kind(&decoded, cols, link, kind));
}

fn wrap_plain_text(text: &str, cols: usize, link: Option<String>) -> Vec<BrowserLine> {
    wrap_plain_text_kind(text, cols, link, BrowserLineKind::Text)
}

fn wrap_plain_text_kind(
    text: &str,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
) -> Vec<BrowserLine> {
    let cols = cols.clamp(20, 120);
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if word.len() > cols {
            if !line.is_empty() {
                out.push(BrowserLine {
                    text: line,
                    link: link.clone(),
                    kind: line_kind_for_link(&link, kind),
                });
                line = String::new();
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.len() >= cols {
                    out.push(BrowserLine {
                        text: chunk,
                        link: link.clone(),
                        kind: line_kind_for_link(&link, kind),
                    });
                    chunk = String::new();
                }
                chunk.push(c);
            }
            if !chunk.is_empty() {
                line = chunk;
            }
            continue;
        }
        let extra = if line.is_empty() { 0 } else { 1 };
        if line.len() + word.len() + extra > cols && !line.is_empty() {
            out.push(BrowserLine {
                text: line,
                link: link.clone(),
                kind: line_kind_for_link(&link, kind),
            });
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        let kind = line_kind_for_link(&link, kind);
        out.push(BrowserLine {
            text: line,
            link,
            kind,
        });
    }
    out
}

fn line_kind_for_link(link: &Option<String>, fallback: BrowserLineKind) -> BrowserLineKind {
    if link.is_some() {
        BrowserLineKind::Link
    } else {
        fallback
    }
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

fn push_text_char(out: &mut String, c: char, preformatted: bool) {
    if preformatted && (c == '\n' || c == '\r') {
        if !out.ends_with('\n') {
            out.push('\n');
        }
    } else if c == '\n' || c == '\r' || c == '\t' {
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
            if input[i..].starts_with("&#x") || input[i..].starts_with("&#X") {
                if let Some(end) = input[i + 3..].find(';') {
                    if let Some(value) = parse_entity_number(&input[i + 3..i + 3 + end], 16) {
                        out.push(value as char);
                        i += end + 4;
                        continue;
                    }
                }
            }
            if input[i..].starts_with("&#") {
                if let Some(end) = input[i + 2..].find(';') {
                    if let Some(value) = parse_entity_number(&input[i + 2..i + 2 + end], 10) {
                        out.push(value as char);
                        i += end + 3;
                        continue;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_entity_number(input: &str, radix: u32) -> Option<u8> {
    let mut value = 0u32;
    for b in input.bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u32,
            b'a'..=b'f' => (b - b'a' + 10) as u32,
            b'A'..=b'F' => (b - b'A' + 10) as u32,
            _ => return None,
        };
        if digit >= radix {
            return None;
        }
        value = value.checked_mul(radix)?.checked_add(digit)?;
        if value > 0xff {
            return None;
        }
    }
    Some(value as u8)
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
    if href.starts_with("browser://")
        || href.starts_with("file://")
        || href.starts_with("http://")
        || href.starts_with("https://")
    {
        return String::from(href);
    }
    let Ok((scheme, host, path)) = parse_web_url(base) else {
        return normalize_url(href);
    };
    if href.starts_with('/') {
        let mut out = scheme;
        out.push_str("://");
        out.push_str(&host);
        out.push_str(href);
        return out;
    }
    let mut dir = path;
    if let Some(pos) = dir.rfind('/') {
        dir.truncate(pos + 1);
    }
    let mut out = scheme;
    out.push_str("://");
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
