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
const MAX_INLINE_PNG_PIXELS: usize = 1_048_576;
const MAX_HTML_INLINE_IMAGES: usize = 4;
const INLINE_IMAGE_MAX_H: usize = 168;
const INLINE_IMAGE_RESERVED_ROWS: usize = 14;

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
    Quote,
    Code,
    Error,
}

#[derive(Clone)]
struct BrowserLine {
    text: String,
    link: Option<String>,
    kind: BrowserLineKind,
    image_slot: Option<usize>,
}

#[derive(Clone)]
struct CachedPage {
    url: String,
    body: String,
    body_bytes: Vec<u8>,
    content_type: Option<String>,
}

#[derive(Clone)]
struct InlineImage {
    image: crate::png::PngImage,
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
    image_preview: Option<crate::png::PngImage>,
    inline_images: Vec<InlineImage>,
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
            image_preview: None,
            inline_images: Vec::new(),
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
        self.image_preview = None;
        self.inline_images.clear();
        self.lines = vec![BrowserLine {
            text: format!("Loading {}", url),
            link: None,
            kind: BrowserLineKind::Muted,
            image_slot: None,
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
                        self.inline_images.clear();
                        let preview_status = self.decode_image_preview(&response);
                        image_response_lines(
                            &response.final_url,
                            response.content_type.as_deref(),
                            response.body_bytes.len(),
                            preview_status.as_deref(),
                        )
                    } else {
                        self.image_preview = None;
                        let mut lines =
                            render_document(&response.final_url, &response.body, self.cols.max(48));
                        let images = self.attach_html_images(&mut lines);
                        if images > 0 {
                            self.status.push_str(&format!("  images={}", images));
                        }
                        lines
                    };
                    if self.lines.is_empty() {
                        self.lines.push(BrowserLine {
                            text: String::from("(empty response)"),
                            link: None,
                            kind: BrowserLineKind::Muted,
                            image_slot: None,
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
                    self.image_preview = None;
                    self.inline_images.clear();
                    self.lines = network_error_lines(&url, &host, &path, err);
                }
            },
            Err(err) => {
                self.title = String::from("Unsupported URL");
                self.status = String::from(err);
                self.last_page = None;
                self.image_preview = None;
                self.inline_images.clear();
                self.lines = vec![
                    BrowserLine {
                        text: String::from("Enter an http:// or https:// URL."),
                        link: None,
                        kind: BrowserLineKind::Text,
                        image_slot: None,
                    },
                    BrowserLine {
                        text: String::from("Try https://example.com/"),
                        link: Some(String::from("https://example.com/")),
                        kind: BrowserLineKind::Link,
                        image_slot: None,
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
        self.image_preview = None;
        self.inline_images.clear();
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
                    kind_line(&format!("{} bytes", data.len()), BrowserLineKind::Muted),
                    link_line("Open Downloads", "browser://downloads"),
                ];
            }
            Err(err) => {
                self.status = format!("Save failed: {}", err.as_str());
            }
        }
        self.image_preview = None;
        self.inline_images.clear();
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
        } else if let Some(bytes) = crate::vfs::vfs_read_file(path) {
            if is_png_content(None, path) || bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                self.show_local_image(path, bytes);
                return true;
            }
            if is_html_path(path) || looks_like_html_bytes(&bytes) {
                self.show_local_html(path, bytes);
                return true;
            }
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
            self.image_preview = None;
            self.inline_images.clear();
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
        true
    }

    fn show_local_image(&mut self, path: &str, bytes: Vec<u8>) {
        let url = file_url_for_path(path);
        self.address = url.clone();
        self.title = String::from("Image");
        self.status = format!("Local image {}", path);
        self.inline_images.clear();
        self.last_page = Some(CachedPage {
            url: url.clone(),
            body: String::new(),
            body_bytes: bytes.clone(),
            content_type: Some(String::from("image/png")),
        });
        let preview_status = match crate::png::decode_rgb8(&bytes, MAX_INLINE_PNG_PIXELS) {
            Ok(image) => {
                let status = format!("PNG preview {}x{}", image.width, image.height);
                self.image_preview = Some(image);
                Some(status)
            }
            Err(err) => {
                self.image_preview = None;
                Some(format!("PNG preview unavailable: {}", err))
            }
        };
        self.lines = image_response_lines(
            &url,
            Some("image/png"),
            bytes.len(),
            preview_status.as_deref(),
        );
        self.address_focused = false;
        self.address_selected = false;
        self.render();
    }

    fn show_local_html(&mut self, path: &str, bytes: Vec<u8>) {
        let url = file_url_for_path(path);
        let body = String::from_utf8_lossy(&bytes).into_owned();
        self.address = url.clone();
        self.title = extract_title(&body).unwrap_or_else(|| String::from("Local HTML"));
        self.status = format!("Local HTML {}", path);
        self.image_preview = None;
        self.inline_images.clear();
        self.last_page = Some(CachedPage {
            url: url.clone(),
            body: body.clone(),
            body_bytes: bytes,
            content_type: Some(String::from("text/html")),
        });
        let mut lines = render_document(&url, &body, self.cols.max(48));
        let images = self.attach_html_images(&mut lines);
        if images > 0 {
            self.status.push_str(&format!("  images={}", images));
        }
        self.lines = if lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            lines
        };
        self.address_focused = false;
        self.address_selected = false;
        self.render();
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

    fn decode_image_preview(&mut self, response: &crate::net::HttpResponse) -> Option<String> {
        self.image_preview = None;
        if !is_png_content(response.content_type.as_deref(), &response.final_url) {
            return Some(String::from("Preview unavailable: only PNG is supported"));
        }
        match crate::png::decode_rgb8(&response.body_bytes, MAX_INLINE_PNG_PIXELS) {
            Ok(image) => {
                let status = format!("PNG preview {}x{}", image.width, image.height);
                self.image_preview = Some(image);
                Some(status)
            }
            Err(err) => Some(format!("PNG preview unavailable: {}", err)),
        }
    }

    fn attach_html_images(&mut self, lines: &mut Vec<BrowserLine>) -> usize {
        self.inline_images.clear();
        let mut idx = 0usize;
        while idx < lines.len() && self.inline_images.len() < MAX_HTML_INLINE_IMAGES {
            let should_try = lines
                .get(idx)
                .map(|line| line.kind == BrowserLineKind::Image && line.link.is_some())
                .unwrap_or(false);
            if !should_try {
                idx += 1;
                continue;
            }
            let Some(url) = lines[idx].link.clone() else {
                idx += 1;
                continue;
            };
            let alt = image_alt_from_line(&lines[idx].text);
            match fetch_png_for_browser(&url) {
                Ok((image, source_url, byte_len)) => {
                    let slot = self.inline_images.len();
                    let rows = inline_image_reserved_rows_for(
                        image.width,
                        image.height,
                        self.cols.max(48),
                    );
                    lines[idx].image_slot = Some(slot);
                    lines[idx].text = format!(
                        "[image] {}  {}x{}  {} bytes",
                        alt, image.width, image.height, byte_len
                    );
                    lines[idx].link = Some(source_url);
                    self.inline_images.push(InlineImage { image });
                    for _ in 1..rows {
                        lines.insert(idx + 1, inline_image_spacer(slot, &url));
                    }
                    idx += rows;
                }
                Err(err) => {
                    lines[idx].text = format!("{} ({})", lines[idx].text, err);
                    idx += 1;
                }
            }
        }
        self.inline_images.len()
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

        let mut lines_y = doc_y;
        let mut lines_h = doc_h;
        if let Some(image) = self.image_preview.clone() {
            let preview_h = self.draw_image_preview(
                stride,
                PAD_X,
                doc_y,
                width.saturating_sub(PAD_X * 2),
                doc_h.saturating_sub(42).min(260),
                &image,
            );
            if preview_h > 0 {
                lines_y = lines_y.saturating_add(preview_h + 16);
                lines_h = doc_h.saturating_sub(preview_h + 16);
            }
        }

        self.rows = lines_h / LINE_H;
        self.cols = width.saturating_sub(PAD_X * 2 + 28) / CHAR_W;
        self.window.scroll.content_h = (self.lines.len() * LINE_H) as i32;
        self.window.scroll.offset = self.scroll as i32 * LINE_H as i32;
        self.window.scroll.clamp((self.rows * LINE_H) as i32);

        for row in 0..self.rows {
            let idx = self.scroll + row;
            let Some(line) = self.lines.get(idx).cloned() else {
                break;
            };
            let y = lines_y + row * LINE_H;
            let color = match line.kind {
                BrowserLineKind::Heading => 0x00_04_24_3A,
                BrowserLineKind::Muted => MUTED,
                BrowserLineKind::Link => LINK,
                BrowserLineKind::Image => 0x00_7A_3B_00,
                BrowserLineKind::Quote => 0x00_40_55_5D,
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
            let image_rendered = if let Some(slot) = line.image_slot {
                if let Some(image) = self.inline_images.get(slot).map(|inline| &inline.image) {
                    let available_h = (doc_y + doc_h)
                        .saturating_sub(y + LINE_H + 3)
                        .min(INLINE_IMAGE_MAX_H);
                    if available_h > 0 {
                        draw_image_preview_pixels(
                            &mut self.window.buf,
                            width,
                            content_h,
                            stride,
                            PAD_X,
                            y + LINE_H + 3,
                            width.saturating_sub(PAD_X * 2 + 28),
                            available_h,
                            image,
                        );
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !image_rendered {
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

    fn draw_image_preview(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        max_w: usize,
        max_h: usize,
        image: &crate::png::PngImage,
    ) -> usize {
        draw_image_preview_pixels(
            &mut self.window.buf,
            self.window.width.max(0) as usize,
            (self.window.height - TITLE_H).max(0) as usize,
            stride,
            x,
            y,
            max_w,
            max_h,
            image,
        )
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

fn draw_image_preview_pixels(
    buf: &mut [u32],
    surface_w: usize,
    surface_h: usize,
    stride: usize,
    x: usize,
    y: usize,
    max_w: usize,
    max_h: usize,
    image: &crate::png::PngImage,
) -> usize {
    if image.width == 0 || image.height == 0 || max_w < 8 || max_h < 8 {
        return 0;
    }
    let (mut draw_w, mut draw_h) = scaled_image_size(image.width, image.height, max_w, max_h);
    draw_w = draw_w.max(1);
    draw_h = draw_h.max(1);

    let frame_w = draw_w + 8;
    let frame_h = draw_h + 8;
    fill_pixels(
        buf,
        surface_w,
        surface_h,
        stride,
        x,
        y,
        frame_w,
        frame_h,
        0x00_E8_EF_F3,
    );
    draw_pixel_rect(
        buf,
        surface_w,
        surface_h,
        stride,
        x,
        y,
        frame_w,
        frame_h,
        0x00_91_A6_B5,
    );

    for dy in 0..draw_h {
        let src_y = dy.saturating_mul(image.height) / draw_h;
        for dx in 0..draw_w {
            let src_x = dx.saturating_mul(image.width) / draw_w;
            let Some(&color) = image.pixels.get(src_y * image.width + src_x) else {
                continue;
            };
            let px = x + 4 + dx;
            let py = y + 4 + dy;
            let idx = py.saturating_mul(stride).saturating_add(px);
            if px < surface_w && py < surface_h && idx < buf.len() {
                buf[idx] = color;
            }
        }
    }
    frame_h
}

fn fill_pixels(
    buf: &mut [u32],
    surface_w: usize,
    surface_h: usize,
    stride: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    for row in y..(y + h).min(surface_h) {
        let base = row.saturating_mul(stride);
        for col in x..(x + w).min(surface_w) {
            let idx = base.saturating_add(col);
            if idx < buf.len() {
                buf[idx] = color;
            }
        }
    }
}

fn draw_pixel_rect(
    buf: &mut [u32],
    surface_w: usize,
    surface_h: usize,
    stride: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if w == 0 || h == 0 {
        return;
    }
    fill_pixels(buf, surface_w, surface_h, stride, x, y, w, 1, color);
    fill_pixels(buf, surface_w, surface_h, stride, x, y + h - 1, w, 1, color);
    fill_pixels(buf, surface_w, surface_h, stride, x, y, 1, h, color);
    fill_pixels(buf, surface_w, surface_h, stride, x + w - 1, y, 1, h, color);
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
    let file_count = entries.iter().filter(|entry| !entry.is_dir).count();
    let total_bytes = entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .fold(0usize, |total, entry| {
            total.saturating_add(entry.size as usize)
        });
    out.push(kind_line(
        &format!("{} file(s), {} bytes", file_count, total_bytes),
        BrowserLineKind::Muted,
    ));
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

fn image_response_lines(
    url: &str,
    content_type: Option<&str>,
    byte_len: usize,
    preview_status: Option<&str>,
) -> Vec<BrowserLine> {
    let mut out = vec![
        kind_line("Image", BrowserLineKind::Heading),
        kind_line(content_type.unwrap_or("image/*"), BrowserLineKind::Muted),
        line(""),
    ];
    if let Some(status) = preview_status {
        out.push(kind_line(status, BrowserLineKind::Muted));
    }
    out.push(BrowserLine {
        text: format!("{} bytes received", byte_len),
        link: None,
        kind: BrowserLineKind::Image,
        image_slot: None,
    });
    out.push(link_line("Image source URL", url));
    out
}

fn is_image_content(content_type: Option<&str>) -> bool {
    content_type
        .map(|value| value.trim().to_ascii_lowercase().starts_with("image/"))
        .unwrap_or(false)
}

fn is_png_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("image/png")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("png"))
}

fn is_html_path(path: &str) -> bool {
    matches!(extension_from_path(path), "html")
}

fn looks_like_html_bytes(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(512);
    let sample = String::from_utf8_lossy(&bytes[..sample_len]);
    let lower = lowercase_ascii(&sample);
    lower.contains("<html") || lower.contains("<!doctype html") || lower.contains("<body")
}

fn inline_image_spacer(_slot: usize, url: &str) -> BrowserLine {
    BrowserLine {
        text: String::new(),
        link: Some(String::from(url)),
        kind: BrowserLineKind::Image,
        image_slot: None,
    }
}

fn inline_image_reserved_rows_for(width: usize, height: usize, cols: usize) -> usize {
    let max_w = cols.saturating_mul(CHAR_W).max(80);
    let (_draw_w, draw_h) = scaled_image_size(width, height, max_w, INLINE_IMAGE_MAX_H);
    (draw_h / LINE_H + 3).clamp(4, INLINE_IMAGE_RESERVED_ROWS)
}

fn scaled_image_size(image_w: usize, image_h: usize, max_w: usize, max_h: usize) -> (usize, usize) {
    if image_w == 0 || image_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    if image_w <= max_w && image_h <= max_h {
        let mut scale = 1usize;
        while scale < 16
            && image_w.saturating_mul(scale + 1) <= max_w
            && image_h.saturating_mul(scale + 1) <= max_h
            && image_w.saturating_mul(scale) < 320
            && image_h.saturating_mul(scale) < 220
        {
            scale += 1;
        }
        return (image_w.saturating_mul(scale), image_h.saturating_mul(scale));
    }
    let mut draw_w = image_w.min(max_w);
    let mut draw_h = image_h.saturating_mul(draw_w) / image_w;
    if draw_h > max_h {
        draw_h = max_h;
        draw_w = image_w.saturating_mul(draw_h) / image_h;
    }
    (draw_w.min(max_w), draw_h.min(max_h))
}

fn image_alt_from_line(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("[image]") {
        let label = rest.trim();
        if !label.is_empty() {
            return String::from(label);
        }
    }
    if let Some(rest) = text.strip_prefix("[image ") {
        if let Some((_, label)) = rest.split_once(']') {
            let label = label.trim();
            if !label.is_empty() {
                return String::from(label);
            }
        }
    }
    String::from("image")
}

fn fetch_png_for_browser(url: &str) -> Result<(crate::png::PngImage, String, usize), &'static str> {
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = crate::vfs::vfs_read_file(path).ok_or("image file missing")?;
        if !is_png_content(None, path) && !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("preview skipped: not PNG");
        }
        let image = crate::png::decode_rgb8(&bytes, MAX_INLINE_PNG_PIXELS)?;
        return Ok((image, file_url_for_path(path), bytes.len()));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("preview skipped: unsupported image URL");
    }
    if matches!(extension_from_path(url), "jpg" | "gif" | "webp") {
        return Err("preview skipped: not PNG");
    }
    let response = crate::net::web_get_response(url)?;
    if !is_png_content(response.content_type.as_deref(), &response.final_url) {
        return Err("preview skipped: not PNG");
    }
    let image = crate::png::decode_rgb8(&response.body_bytes, MAX_INLINE_PNG_PIXELS)?;
    Ok((image, response.final_url, response.body_bytes.len()))
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
    let lower = lowercase_ascii(leaf);
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "html"
    } else if lower.ends_with(".txt") {
        "txt"
    } else if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "jpg"
    } else if lower.ends_with(".gif") {
        "gif"
    } else if lower.ends_with(".webp") {
        "webp"
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
        image_slot: None,
    }
}

fn kind_line(text: &str, kind: BrowserLineKind) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: None,
        kind,
        image_slot: None,
    }
}

fn link_line(text: &str, url: &str) -> BrowserLine {
    BrowserLine {
        text: String::from(text),
        link: Some(String::from(url)),
        kind: BrowserLineKind::Link,
        image_slot: None,
    }
}

fn network_error_lines(url: &str, host: &str, path: &str, err: &str) -> Vec<BrowserLine> {
    let mut lines = vec![
        kind_line("Unable to load page", BrowserLineKind::Error),
        line(url),
        kind_line(err, BrowserLineKind::Muted),
        line(""),
    ];
    if err.contains("timeout") {
        lines.push(kind_line(
            "The connection timed out before the page finished loading.",
            BrowserLineKind::Muted,
        ));
    } else if err.contains("certificate") || err.contains("hostname") {
        lines.push(kind_line(
            "The TLS certificate could not be verified for this host.",
            BrowserLineKind::Muted,
        ));
    } else if err.contains("DNS") {
        lines.push(kind_line(
            "The hostname did not resolve through the configured DNS server.",
            BrowserLineKind::Muted,
        ));
    }
    lines.push(link_line("Retry", url));
    if path != "/" {
        let mut origin = if url.starts_with("http://") {
            String::from("http://")
        } else {
            String::from("https://")
        };
        origin.push_str(host);
        origin.push('/');
        lines.push(link_line("Open site root", &origin));
    }
    lines.push(link_line(
        "Open known-good HTTPS page",
        "https://example.com/",
    ));
    lines
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

fn extract_base_href(body: &str, fallback: &str) -> String {
    let head_end = {
        let lower = lowercase_ascii(body);
        lower.find("</head>").unwrap_or(body.len().min(8192))
    };
    let search = &body[..head_end];
    let lower_search = lowercase_ascii(search);
    let mut i = 0usize;
    while let Some(rel) = lower_search[i..].find("<base") {
        let abs = i + rel;
        if let Some(end) = lower_search[abs..].find('>') {
            let tag = &body[abs + 1..abs + end];
            if let Some(href) = attr_value(tag, "href") {
                let href = String::from(href.trim());
                if !href.is_empty() {
                    return href;
                }
            }
            i = abs + end + 1;
        } else {
            break;
        }
    }
    String::from(fallback)
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
    let effective_base = extract_base_href(body, base_url);
    let base_url: &str = &effective_base;
    let mut out = Vec::new();
    let mut text = String::new();
    let mut state = HtmlRenderState::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if body[i..].starts_with("<!--") {
                if let Some(end_rel) = body[i + 4..].find("-->") {
                    i += end_rel + 7;
                } else {
                    i += 4;
                }
                continue;
            }
            if let Some(end_rel) = find_tag_end(&body[i..]) {
                let tag = &body[i + 1..i + end_rel];
                let lower_tag = lowercase_ascii(tag.trim());
                if let Some(end_tag) = state.skip_until.as_ref() {
                    if lower_tag.starts_with(end_tag) {
                        state.skip_until = None;
                    }
                    i += end_rel + 1;
                    continue;
                }
                let lower_name = tag_name_of(&lower_tag);
                if lower_name == "script"
                    || lower_name == "style"
                    || lower_name == "noscript"
                    || lower_name == "svg"
                    || lower_name == "canvas"
                    || lower_name == "template"
                    || lower_name == "iframe"
                    || lower_name == "video"
                    || lower_name == "audio"
                    || lower_name == "object"
                    || lower_name == "embed"
                    || lower_name == "head"
                    || (tag_is_hidden(&lower_tag) && !lower_tag.starts_with("input"))
                {
                    flush_flow_text(&mut out, &mut text, cols, &mut state);
                    state.skip_until = Some(closing_tag_for(&lower_tag));
                    i += end_rel + 1;
                    continue;
                }
                handle_tag(tag, &mut out, &mut text, &mut state, base_url, cols);
                i += end_rel + 1;
                continue;
            }
        }
        if state.skip_until.is_none() {
            if state.in_table_cell {
                push_text_char(&mut state.table_cell_text, bytes[i] as char, false);
            } else {
                push_text_char(&mut text, bytes[i] as char, state.preformatted);
            }
        }
        i += 1;
    }
    if state.in_table_cell {
        finish_table_cell(&mut state);
    }
    if state.in_table {
        finish_table_row(&mut out, &mut state, cols);
    }
    flush_flow_text(&mut out, &mut text, cols, &mut state);
    compact_lines(out)
}

pub fn render_document_debug_for_test(base_url: &str, response: &str, cols: usize) -> Vec<String> {
    render_document(base_url, response, cols)
        .into_iter()
        .filter(|line| !line.text.is_empty())
        .map(|line| {
            if let Some(link) = line.link {
                format!("{} -> {}", line.text, link)
            } else {
                line.text
            }
        })
        .collect()
}

struct HtmlRenderState {
    link: Option<String>,
    kind: BrowserLineKind,
    pending_prefix: Option<String>,
    preformatted: bool,
    skip_until: Option<String>,
    quote_depth: usize,
    list_depth: usize,
    ordered_stack: Vec<usize>,
    in_table: bool,
    in_table_cell: bool,
    table_cell_is_header: bool,
    table_cell_text: String,
    table_row: Vec<TableCell>,
    form_action: Option<String>,
    cell_has_form_control: bool,
    cell_form_link: Option<String>,
}

impl HtmlRenderState {
    fn new() -> Self {
        Self {
            link: None,
            kind: BrowserLineKind::Text,
            pending_prefix: None,
            preformatted: false,
            skip_until: None,
            quote_depth: 0,
            list_depth: 0,
            ordered_stack: Vec::new(),
            in_table: false,
            in_table_cell: false,
            table_cell_is_header: false,
            table_cell_text: String::new(),
            table_row: Vec::new(),
            form_action: None,
            cell_has_form_control: false,
            cell_form_link: None,
        }
    }
}

struct TableCell {
    text: String,
    header: bool,
    link: Option<String>,
    is_form_row: bool,
}

fn is_inline_tag(name: &str) -> bool {
    matches!(
        name,
        "span" | "strong" | "em" | "b" | "i" | "small" | "big" | "sub" | "sup" | "s" | "u"
            | "del" | "ins" | "mark" | "cite" | "abbr" | "time" | "var" | "samp" | "kbd"
            | "wbr" | "bdi" | "bdo" | "data" | "q" | "dfn" | "label" | "output" | "meter"
            | "progress" | "nobr" | "font" | "tt" | "acronym" | "strike" | "blink"
            | "marquee"
    )
}

fn handle_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    state: &mut HtmlRenderState,
    base_url: &str,
    cols: usize,
) {
    let tag = tag.trim();
    let lower = lowercase_ascii(tag);
    let name = tag_name_of(&lower);
    let closing = lower.starts_with('/');

    if state.in_table_cell {
        handle_table_cell_tag(tag, out, state, base_url, cols, name, closing);
        return;
    }

    if is_inline_tag(name) {
        return;
    }

    flush_flow_text(out, text, cols, state);

    match name {
        "a" => {
            if closing {
                state.link = None;
            } else if let Some(href) = attr_value(tag, "href") {
                state.link = Some(resolve_url(base_url, &decode_entities(&href)));
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if closing {
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.kind = BrowserLineKind::Heading;
            }
        }
        "pre" => {
            if closing {
                state.preformatted = false;
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.preformatted = true;
                state.kind = BrowserLineKind::Code;
            }
        }
        "code" => {
            state.kind = if closing {
                BrowserLineKind::Text
            } else {
                BrowserLineKind::Code
            };
        }
        "blockquote" => {
            if closing {
                state.quote_depth = state.quote_depth.saturating_sub(1);
                state.kind = BrowserLineKind::Text;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.quote_depth = state.quote_depth.saturating_add(1);
                state.kind = BrowserLineKind::Quote;
            }
        }
        "ul" => {
            if closing {
                state.list_depth = state.list_depth.saturating_sub(1);
            } else {
                state.list_depth = state.list_depth.saturating_add(1);
            }
            push_blank_line(out);
        }
        "ol" => {
            if closing {
                state.ordered_stack.pop();
                state.list_depth = state.list_depth.saturating_sub(1);
            } else {
                state.ordered_stack.push(1);
                state.list_depth = state.list_depth.saturating_add(1);
            }
            push_blank_line(out);
        }
        "li" => {
            push_blank_line(out);
            state.pending_prefix = Some(list_prefix(state));
        }
        "form" => {
            if closing {
                state.form_action = None;
                push_blank_line(out);
            } else {
                state.form_action = form_action_url(tag, base_url);
                push_blank_line(out);
            }
        }
        "input" => {
            if !closing {
                push_input_line(out, tag, state);
            }
        }
        "button" => {
            if !closing {
                push_button_line(out, tag, state);
            }
        }
        "select" => {
            if !closing {
                push_named_control_line(out, "select", tag);
            }
        }
        "textarea" => {
            if !closing {
                push_named_control_line(out, "textarea", tag);
            }
        }
        "img" => push_image_line(out, tag, base_url),
        "table" => {
            if closing {
                finish_table_row(out, state, cols);
                state.in_table = false;
                push_blank_line(out);
            } else {
                push_blank_line(out);
                state.in_table = true;
                state.table_row.clear();
            }
        }
        "tr" => {
            if closing {
                finish_table_row(out, state, cols);
            } else {
                state.in_table = true;
                state.table_row.clear();
            }
        }
        "td" | "th" => {
            state.in_table = true;
            state.in_table_cell = true;
            state.table_cell_is_header = name == "th";
            state.table_cell_text.clear();
        }
        "thead" | "tbody" | "tfoot" | "colgroup" | "col" | "caption" => {}
        "br" => push_blank_line(out),
        "hr" => out.push(kind_line(&rule_line(cols), BrowserLineKind::Muted)),
        "p" | "div" | "section" | "article" | "main" | "aside" | "header" | "footer" | "nav"
        | "figure" | "figcaption" | "address" | "dl" | "dt" | "dd" | "center" => {
            push_blank_line(out);
        }
        _ => {}
    }
}

fn form_action_url(tag: &str, base_url: &str) -> Option<String> {
    let method = attr_value(tag, "method").unwrap_or_else(|| String::from("get"));
    if method.eq_ignore_ascii_case("post") {
        return None;
    }
    let action = attr_value(tag, "action").unwrap_or_else(|| String::from(base_url));
    if action.trim().is_empty() {
        Some(String::from(base_url))
    } else {
        Some(resolve_url(base_url, &action))
    }
}

fn push_input_line(out: &mut Vec<BrowserLine>, tag: &str, state: &HtmlRenderState) {
    let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
    let input_type = lowercase_ascii(input_type.trim());
    if input_type == "hidden" {
        return;
    }
    let label = form_control_label(tag, &input_type);
    let mut text = String::new();
    let mut link = None;
    match input_type.as_str() {
        "submit" | "button" | "reset" => {
            text.push_str("[button] ");
            text.push_str(&label);
            if input_type == "submit" {
                link = state.form_action.clone();
            }
        }
        "checkbox" => {
            text.push_str("[checkbox] ");
            text.push_str(&label);
        }
        "radio" => {
            text.push_str("[radio] ");
            text.push_str(&label);
        }
        "search" => {
            text.push_str("[search] ");
            text.push_str(&label);
        }
        "image" => {
            text.push_str("[image button] ");
            text.push_str(&label);
            link = state.form_action.clone();
        }
        _ => {
            text.push_str("[input] ");
            text.push_str(&label);
        }
    }
    out.push(BrowserLine {
        text,
        kind: line_kind_for_link(&link, BrowserLineKind::Code),
        link,
        image_slot: None,
    });
}

fn push_button_line(out: &mut Vec<BrowserLine>, tag: &str, state: &HtmlRenderState) {
    if attr_value(tag, "aria-label").is_none()
        && attr_value(tag, "value").is_none()
        && attr_value(tag, "name").is_none()
        && attr_value(tag, "id").is_none()
    {
        return;
    }
    let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
    let label = form_control_label(tag, "button");
    let link = if button_type.eq_ignore_ascii_case("submit") {
        state.form_action.clone()
    } else {
        None
    };
    out.push(BrowserLine {
        text: format!("[button] {}", label),
        kind: line_kind_for_link(&link, BrowserLineKind::Code),
        link,
        image_slot: None,
    });
}

fn push_named_control_line(out: &mut Vec<BrowserLine>, control: &str, tag: &str) {
    let label = form_control_label(tag, control);
    out.push(kind_line(
        &format!("[{}] {}", control, label),
        BrowserLineKind::Code,
    ));
}

fn form_control_label(tag: &str, fallback: &str) -> String {
    for attr in ["aria-label", "placeholder", "value", "title", "name", "id"] {
        if let Some(value) = attr_value(tag, attr) {
            let decoded = clean_inline_text(&decode_entities(&value));
            if !decoded.is_empty() {
                return decoded;
            }
        }
    }
    String::from(fallback)
}

fn handle_table_cell_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    state: &mut HtmlRenderState,
    base_url: &str,
    cols: usize,
    name: &str,
    closing: bool,
) {
    match name {
        "td" | "th" if closing => finish_table_cell(state),
        "tr" if closing => {
            finish_table_cell(state);
            finish_table_row(out, state, cols);
        }
        "table" if closing => {
            finish_table_cell(state);
            finish_table_row(out, state, cols);
            state.in_table = false;
            push_blank_line(out);
        }
        "br" => {
            if !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
        }
        "img" => {
            let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
            let label = attr_value(tag, "alt")
                .map(|alt| decode_entities(&alt))
                .filter(|alt| !alt.trim().is_empty())
                .unwrap_or_else(|| src.unwrap_or_else(|| String::from("image")));
            if !state.table_cell_text.ends_with(' ') && !state.table_cell_text.is_empty() {
                state.table_cell_text.push(' ');
            }
            state.table_cell_text.push_str("[image");
            if let Some(size) = image_size_label(tag) {
                state.table_cell_text.push(' ');
                state.table_cell_text.push_str(&size);
            }
            state.table_cell_text.push(' ');
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "input" if !closing => {
            let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
            let input_type = lowercase_ascii(input_type.trim());
            if input_type == "hidden" {
                return;
            }
            let label = form_control_label(tag, &input_type);
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            match input_type.as_str() {
                "submit" | "button" => {
                    if state.cell_form_link.is_none() {
                        state.cell_form_link = state.form_action.clone();
                    }
                    state.table_cell_text.push_str("[btn:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                _ => {
                    state.table_cell_text.push('[');
                    state.table_cell_text.push_str(&input_type);
                    if !label.is_empty() && label != input_type {
                        state.table_cell_text.push(':');
                        state.table_cell_text.push_str(&label);
                    }
                    state.table_cell_text.push(']');
                }
            }
        }
        _ => {}
    }
}

fn push_image_line(out: &mut Vec<BrowserLine>, tag: &str, base_url: &str) {
    let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
    let label = attr_value(tag, "alt")
        .map(|alt| decode_entities(&alt))
        .filter(|alt| !alt.trim().is_empty())
        .unwrap_or_else(|| src.clone().unwrap_or_else(|| String::from("image")));
    let mut text = String::from("[image");
    if let Some(size) = image_size_label(tag) {
        text.push(' ');
        text.push_str(&size);
    }
    text.push_str("] ");
    text.push_str(&label);
    out.push(BrowserLine {
        text,
        link: src,
        kind: BrowserLineKind::Image,
        image_slot: None,
    });
}

fn image_size_label(tag: &str) -> Option<String> {
    let width = attr_value(tag, "width").and_then(|value| parse_dimension(&value));
    let height = attr_value(tag, "height").and_then(|value| parse_dimension(&value));
    match (width, height) {
        (Some(width), Some(height)) => Some(format!("{}x{}", width, height)),
        (Some(width), None) => Some(format!("{}w", width)),
        (None, Some(height)) => Some(format!("{}h", height)),
        (None, None) => None,
    }
}

fn parse_dimension(value: &str) -> Option<usize> {
    let mut out = 0usize;
    let mut saw_digit = false;
    for b in value.trim().bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        out = out.saturating_mul(10).saturating_add((b - b'0') as usize);
    }
    if saw_digit && out > 0 && out <= 10_000 {
        Some(out)
    } else {
        None
    }
}

fn list_prefix(state: &mut HtmlRenderState) -> String {
    let mut out = String::new();
    for _ in 1..state.list_depth {
        out.push_str("  ");
    }
    if let Some(next) = state.ordered_stack.last_mut() {
        out.push_str(&format!("{}. ", *next));
        *next = next.saturating_add(1);
    } else {
        out.push_str("* ");
    }
    out
}

fn rule_line(cols: usize) -> String {
    let mut out = String::new();
    for _ in 0..cols.clamp(20, 80) {
        out.push('-');
    }
    out
}

fn finish_table_cell(state: &mut HtmlRenderState) {
    if !state.in_table_cell {
        return;
    }
    let text = clean_inline_text(&decode_entities(&state.table_cell_text));
    state.table_row.push(TableCell {
        text,
        header: state.table_cell_is_header,
        link: state.cell_form_link.take(),
        is_form_row: state.cell_has_form_control,
    });
    state.table_cell_text.clear();
    state.table_cell_is_header = false;
    state.in_table_cell = false;
    state.cell_has_form_control = false;
}

fn finish_table_row(out: &mut Vec<BrowserLine>, state: &mut HtmlRenderState, cols: usize) {
    if state.table_row.is_empty() {
        return;
    }
    let has_form = state.table_row.iter().any(|c| c.is_form_row);
    if has_form {
        for cell in state.table_row.drain(..) {
            if cell.text.is_empty() {
                continue;
            }
            let kind = if cell.link.is_some() {
                BrowserLineKind::Link
            } else {
                BrowserLineKind::Code
            };
            out.push(BrowserLine {
                text: cell.text,
                link: cell.link,
                kind,
                image_slot: None,
            });
        }
        return;
    }
    let header = state.table_row.iter().any(|cell| cell.header);
    let row = format_table_row(&state.table_row, cols);
    out.push(kind_line(
        &row,
        if header {
            BrowserLineKind::Heading
        } else {
            BrowserLineKind::Code
        },
    ));
    if header {
        out.push(kind_line(
            &format_table_separator(&state.table_row, cols),
            BrowserLineKind::Muted,
        ));
    }
    state.table_row.clear();
}

fn format_table_row(cells: &[TableCell], cols: usize) -> String {
    let width = table_cell_width(cells.len(), cols);
    let mut out = String::from("|");
    for cell in cells {
        out.push(' ');
        push_truncated_padded(&mut out, &cell.text, width);
        out.push(' ');
        out.push('|');
    }
    out
}

fn format_table_separator(cells: &[TableCell], cols: usize) -> String {
    let width = table_cell_width(cells.len(), cols);
    let mut out = String::from("+");
    for _ in cells {
        for _ in 0..(width + 2) {
            out.push('-');
        }
        out.push('+');
    }
    out
}

fn table_cell_width(cell_count: usize, cols: usize) -> usize {
    if cell_count == 0 {
        return cols.clamp(8, 40);
    }
    let chrome = cell_count.saturating_mul(3).saturating_add(1);
    cols.saturating_sub(chrome)
        .saturating_div(cell_count)
        .clamp(8, 32)
}

fn push_truncated_padded(out: &mut String, input: &str, width: usize) {
    let mut written = 0usize;
    for c in input.chars().take(width) {
        out.push(c);
        written += 1;
    }
    if input.chars().count() > width && width > 0 {
        out.pop();
        out.push('>');
    }
    while written < width {
        out.push(' ');
        written += 1;
    }
}

fn push_blank_line(out: &mut Vec<BrowserLine>) {
    if out.last().map(|line| !line.text.is_empty()).unwrap_or(true) {
        out.push(BrowserLine {
            text: String::new(),
            link: None,
            kind: BrowserLineKind::Text,
            image_slot: None,
        });
    }
}

fn tag_is_hidden(lower_tag: &str) -> bool {
    let name = tag_name_of(lower_tag);
    let attrs = lower_tag[name.len()..].trim();
    // "hidden" as a standalone boolean attribute, not aria-hidden or data-hidden
    let has_hidden_attr = attrs.split(|c: char| c.is_ascii_whitespace()).any(|token| {
        token == "hidden" || token.starts_with("hidden=")
    });
    has_hidden_attr
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

fn flush_flow_text(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
) {
    let mut prefix = state.pending_prefix.take();
    if state.quote_depth > 0 && state.kind != BrowserLineKind::Code {
        let mut quote = String::new();
        for _ in 0..state.quote_depth.min(3) {
            quote.push_str("> ");
        }
        if let Some(existing) = prefix {
            quote.push_str(&existing);
        }
        prefix = Some(quote);
    }
    let kind =
        if state.quote_depth > 0 && state.kind == BrowserLineKind::Text && state.link.is_none() {
            BrowserLineKind::Quote
        } else {
            state.kind
        };
    flush_text(out, text, cols, state.link.clone(), kind, prefix);
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

fn clean_inline_text(input: &str) -> String {
    let mut out = String::new();
    for word in input.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
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
                    image_slot: None,
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
                        image_slot: None,
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
                image_slot: None,
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
            image_slot: None,
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
            if input[i..].starts_with("&nbsp;") {
                out.push(' ');
                i += 6;
                continue;
            }
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
                        out.push(value);
                        i += end + 4;
                        continue;
                    }
                }
            }
            if input[i..].starts_with("&#") {
                if let Some(end) = input[i + 2..].find(';') {
                    if let Some(value) = parse_entity_number(&input[i + 2..i + 2 + end], 10) {
                        out.push(value);
                        i += end + 3;
                        continue;
                    }
                }
            }
            // generic named entity fallback (&copy; &mdash; etc.)
            if let Some(semi) = input[i + 1..].find(';') {
                if semi < 24 {
                    let name = &input[i + 1..i + 1 + semi];
                    if name.bytes().all(|b| b.is_ascii_alphabetic()) {
                        if let Some(s) = named_entity_str(name) {
                            out.push_str(s);
                            i += semi + 2;
                            continue;
                        }
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn named_entity_str(name: &str) -> Option<&'static str> {
    match name {
        "copy" => Some("(c)"),
        "reg" => Some("(R)"),
        "trade" => Some("(TM)"),
        "apos" => Some("'"),
        "lsquo" | "rsquo" | "sbquo" => Some("'"),
        "ldquo" | "rdquo" | "bdquo" => Some("\""),
        "ndash" | "minus" => Some("-"),
        "mdash" => Some("--"),
        "hellip" => Some("..."),
        "bull" | "middot" => Some("*"),
        "laquo" => Some("<<"),
        "raquo" => Some(">>"),
        "euro" => Some("EUR"),
        "pound" => Some("GBP"),
        "yen" => Some("JPY"),
        "cent" => Some("c"),
        "deg" => Some("deg"),
        "times" => Some("x"),
        "divide" => Some("/"),
        "plusmn" => Some("+/-"),
        "rarr" | "rArr" => Some("->"),
        "larr" | "lArr" => Some("<-"),
        "harr" | "hArr" => Some("<->"),
        "uarr" => Some("^"),
        "darr" => Some("v"),
        "frac12" => Some("1/2"),
        "frac14" => Some("1/4"),
        "frac34" => Some("3/4"),
        "sup2" => Some("^2"),
        "sup3" => Some("^3"),
        "alpha" => Some("alpha"),
        "beta" => Some("beta"),
        "gamma" => Some("gamma"),
        "pi" => Some("pi"),
        "infin" => Some("inf"),
        "ne" => Some("!="),
        "le" => Some("<="),
        "ge" => Some(">="),
        "and" => Some("&&"),
        "or" => Some("||"),
        _ => None,
    }
}

fn parse_entity_number(input: &str, radix: u32) -> Option<char> {
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
        if value > 0x10_ffff {
            return None;
        }
    }
    char::from_u32(value)
}

fn extract_title(response: &str) -> Option<String> {
    let lower = lowercase_ascii(response);
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    Some(decode_entities(response[start..end].trim()))
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let name_bytes = name.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() {
        while pos < bytes.len()
            && (bytes[pos].is_ascii_whitespace() || matches!(bytes[pos], b'/' | b'<'))
        {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
        {
            pos += 1;
        }
        if start == pos {
            pos = pos.saturating_add(1);
            continue;
        }
        let key = &bytes[start..pos];
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b'=') {
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let value = if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
            let quote = bytes[pos];
            pos += 1;
            let value_start = pos;
            while pos < bytes.len() && bytes[pos] != quote {
                pos += 1;
            }
            String::from(&tag[value_start..pos])
        } else {
            let value_start = pos;
            while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                pos += 1;
            }
            String::from(&tag[value_start..pos])
        };
        if ascii_bytes_eq_ignore_case(key, name_bytes) {
            return Some(value);
        }
    }
    None
}

fn resolve_url(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.starts_with("browser://")
        || href.starts_with("file://")
        || href.starts_with("http://")
        || href.starts_with("https://")
    {
        return String::from(href);
    }
    if href.starts_with("//") {
        let scheme = if base.starts_with("https://") {
            "https:"
        } else if base.starts_with("http://") {
            "http:"
        } else {
            "https:"
        };
        let mut out = String::from(scheme);
        out.push_str(href);
        return out;
    }
    if href.starts_with('#') {
        let mut out = String::from(base);
        if let Some(hash) = out.find('#') {
            out.truncate(hash);
        }
        out.push_str(href);
        return out;
    }
    if let Some(path) = base.strip_prefix("file://") {
        let resolved = if href.starts_with('/') {
            crate::vfs::normalize_path(href)
        } else {
            let dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
            let mut joined = String::new();
            if dir.is_empty() {
                joined.push('/');
            } else {
                joined.push_str(dir);
                joined.push('/');
            }
            joined.push_str(href);
            crate::vfs::normalize_path(&joined)
        };
        return file_url_for_path(&resolved);
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

fn find_tag_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => return Some(i),
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
            }
            b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn lowercase_ascii(input: &str) -> String {
    input
        .bytes()
        .map(|b| if b.is_ascii_uppercase() { b + 32 } else { b } as char)
        .collect()
}

fn ascii_bytes_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(l, r)| l.to_ascii_lowercase() == r.to_ascii_lowercase())
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
