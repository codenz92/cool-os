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
const SESSION_INTERNAL_URL: &str = "browser://session";
const MAX_BOOKMARKS: usize = 32;
const MAX_INLINE_PNG_PIXELS: usize = 1_048_576;
const MAX_HTML_INLINE_IMAGES: usize = 4;
const INLINE_IMAGE_MAX_H: usize = 168;
const INLINE_IMAGE_RESERVED_ROWS: usize = 14;
const CONTROL_H: usize = 24;
const CONTROL_GAP: usize = 10;
const BLOCK_GAP: usize = 6;
const MAX_DOM_NODES: usize = 768;
const MAX_DOM_ATTRS: usize = 8;
const MAX_FORM_CONTROLS: usize = 96;
const MAX_FORM_OPTIONS: usize = 16;
const MAX_FORM_VALUE: usize = 192;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct BrowserLineStyle {
    indent_px: usize,
    text_color: Option<u32>,
    background: Option<u32>,
}

impl BrowserLineStyle {
    fn is_default(self) -> bool {
        self == Self::default()
    }

    fn merged(self, other: Self) -> Self {
        Self {
            indent_px: self.indent_px.saturating_add(other.indent_px).min(160),
            text_color: other.text_color.or(self.text_color),
            background: other.background.or(self.background),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct ImageHint {
    width: Option<usize>,
    height: Option<usize>,
}

#[derive(Clone)]
enum BrowserControl {
    None,
    TextInput {
        label: String,
        value: String,
        chars: usize,
    },
    Button {
        label: String,
    },
    Checkbox {
        label: String,
        checked: bool,
    },
    Radio {
        label: String,
        checked: bool,
    },
    Select {
        label: String,
        value: String,
        options: usize,
    },
    TextArea {
        label: String,
        value: String,
        rows: usize,
    },
}

#[derive(Clone)]
struct BrowserLine {
    text: String,
    link: Option<String>,
    kind: BrowserLineKind,
    image_slot: Option<usize>,
    align: BrowserAlign,
    control: BrowserControl,
    style: BrowserLineStyle,
    image_hint: ImageHint,
    control_id: Option<usize>,
}

impl BrowserLine {
    fn new(text: String, link: Option<String>, kind: BrowserLineKind) -> Self {
        Self {
            text,
            link,
            kind,
            image_slot: None,
            align: BrowserAlign::Left,
            control: BrowserControl::None,
            style: BrowserLineStyle::default(),
            image_hint: ImageHint::default(),
            control_id: None,
        }
    }

    fn aligned(mut self, align: BrowserAlign) -> Self {
        self.align = align;
        self
    }

    fn with_control(mut self, control: BrowserControl) -> Self {
        self.control = control;
        self
    }

    fn styled(mut self, style: BrowserLineStyle) -> Self {
        self.style = style;
        self
    }

    fn with_image_hint(mut self, hint: ImageHint) -> Self {
        self.image_hint = hint;
        self
    }

    fn with_control_id(mut self, control_id: Option<usize>) -> Self {
        self.control_id = control_id;
        self
    }
}

struct BrowserHitBox {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    link: Option<String>,
    control_id: Option<usize>,
}

struct BrowserLayoutItem {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    text: String,
    link: Option<String>,
    kind: BrowserLineKind,
    control: BrowserControl,
    image_slot: Option<usize>,
    style: BrowserLineStyle,
    control_id: Option<usize>,
}

struct BrowserLayout {
    items: Vec<BrowserLayoutItem>,
    content_h: usize,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserFormMethod {
    Get,
    Post,
}

#[derive(Clone)]
struct BrowserFormState {
    action: String,
    method: BrowserFormMethod,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserFormControlKind {
    Text,
    Checkbox,
    Radio,
    Select,
    TextArea,
    Submit,
    Button,
    Reset,
    Hidden,
    Image,
}

#[derive(Clone)]
struct BrowserSelectOption {
    label: String,
    value: String,
}

#[derive(Clone)]
struct BrowserFormControlState {
    form_id: Option<usize>,
    kind: BrowserFormControlKind,
    name: String,
    label: String,
    value: String,
    default_value: String,
    checked: bool,
    default_checked: bool,
    disabled: bool,
    chars: usize,
    rows: usize,
    options: Vec<BrowserSelectOption>,
    selected: usize,
    default_selected: usize,
}

#[derive(Clone)]
struct BrowserDomAttr {
    name: String,
    value: String,
}

#[derive(Clone)]
enum BrowserDomNodeKind {
    Element {
        name: String,
        attrs: Vec<BrowserDomAttr>,
    },
    Text(String),
}

#[derive(Clone)]
struct BrowserDomNode {
    parent: Option<usize>,
    children: Vec<usize>,
    kind: BrowserDomNodeKind,
}

#[derive(Clone)]
struct BrowserDomDocument {
    nodes: Vec<BrowserDomNode>,
    root: usize,
}

#[derive(Clone)]
struct BrowserDocumentState {
    base_url: String,
    source: String,
    dom: BrowserDomDocument,
    forms: Vec<BrowserFormState>,
    controls: Vec<BrowserFormControlState>,
    focused_control: Option<usize>,
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
    hit_boxes: Vec<BrowserHitBox>,
    document: Option<BrowserDocumentState>,
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
            hit_boxes: Vec::new(),
            document: None,
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

        if self.handle_document_key(c) {
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
        if lx >= 0 && ly >= 0 {
            let lx = lx as usize;
            let ly = ly as usize;
            for hit in self.hit_boxes.iter().rev() {
                if lx >= hit.x
                    && lx < hit.x.saturating_add(hit.w)
                    && ly >= hit.y
                    && ly < hit.y.saturating_add(hit.h)
                {
                    if let Some(control_id) = hit.control_id {
                        self.activate_document_control(control_id);
                        return;
                    }
                    if let Some(link) = hit.link.as_ref() {
                        let resolved = resolve_url(&self.address, link);
                        if let Some(label) = browser_event_label(&resolved) {
                            self.status = format!("DOM event: {}", label);
                            self.render();
                            return;
                        }
                        if self.open_file_url(&resolved) {
                            return;
                        }
                        self.navigate(&resolved, true);
                        return;
                    }
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
        let expected = self.scroll as i32;
        if self.window.scroll.offset != expected {
            self.scroll = self.window.scroll.offset.max(0) as usize;
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
        self.document = None;
        self.lines = vec![BrowserLine::new(
            format!("Loading {}", url),
            None,
            BrowserLineKind::Muted,
        )];
        self.scroll = 0;
        self.render();

        match parse_web_url(&url) {
            Ok((_scheme, host, path)) => match crate::net::browser_get_response(&url) {
                Ok(response) => {
                    self.apply_web_response(response, add_history, "Loaded");
                }
                Err(err) => {
                    self.title = String::from("Load failed");
                    self.status = format!("Network error: {}", err);
                    self.last_page = None;
                    self.image_preview = None;
                    self.inline_images.clear();
                    self.document = None;
                    self.lines = network_error_lines(&url, &host, &path, err);
                }
            },
            Err(err) => {
                self.title = String::from("Unsupported URL");
                self.status = String::from(err);
                self.last_page = None;
                self.image_preview = None;
                self.inline_images.clear();
                self.document = None;
                self.lines = vec![
                    line("Enter an http:// or https:// URL."),
                    link_line("Try https://example.com/", "https://example.com/"),
                ];
            }
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
    }

    fn apply_web_response(
        &mut self,
        response: crate::net::HttpResponse,
        add_history: bool,
        success_label: &str,
    ) {
        self.title = extract_title(&response.body).unwrap_or_else(|| response.host.clone());
        self.address = response.final_url.clone();
        let security = match response.tls_trust_root {
            Some(root) => format!("  Secure: {}", root),
            None => String::new(),
        };
        self.status = if is_success_status(&response.status_line) {
            format!("{} {}{}", success_label, response.final_url, security)
        } else if response.redirect_count > 0 {
            format!(
                "{}  {} redirect(s) -> {}{}",
                response.status_line, response.redirect_count, response.final_url, security
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
            self.document = None;
            let preview_status = self.decode_image_preview(&response);
            image_response_lines(
                &response.final_url,
                response.content_type.as_deref(),
                response.body_bytes.len(),
                preview_status.as_deref(),
            )
        } else {
            self.image_preview = None;
            let images = self.set_html_document(&response.final_url, &response.body);
            if images > 0 {
                self.status.push_str(&format!("  images={}", images));
            }
            self.lines.clone()
        };
        if self.lines.is_empty() {
            self.lines.push(BrowserLine::new(
                String::from("(empty response)"),
                None,
                BrowserLineKind::Muted,
            ));
        }
        if response.session_cookies_stored > 0 {
            self.status
                .push_str(&format!("  cookies={}", response.session_cookies_stored));
        }
        if add_history {
            self.push_history(response.final_url);
        }
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
        self.document = None;
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
            SESSION_INTERNAL_URL => {
                self.title = String::from("Session");
                self.status = crate::browser_session::summary_line();
                self.lines = browser_session_lines();
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
                self.document = None;
            }
            Err(err) => {
                self.status = format!("Save failed: {}", err.as_str());
            }
        }
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
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
            if is_known_image_path(path) || looks_like_image_bytes(&bytes) {
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
            self.document = None;
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
        self.document = None;
        let content_type = image_content_type_for(path, &bytes).unwrap_or("image/*");
        self.last_page = Some(CachedPage {
            url: url.clone(),
            body: String::new(),
            body_bytes: bytes.clone(),
            content_type: Some(String::from(content_type)),
        });
        let preview_status = self.decode_image_preview_bytes(&bytes, Some(content_type), path);
        self.lines = image_response_lines(
            &url,
            Some(content_type),
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
        let images = self.set_html_document(&url, &body);
        if images > 0 {
            self.status.push_str(&format!("  images={}", images));
        }
        self.lines = if self.lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            self.lines.clone()
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

    fn set_html_document(&mut self, base_url: &str, body: &str) -> usize {
        self.document = Some(BrowserDocumentState::from_html(base_url, body));
        self.reflow_document()
    }

    fn reflow_document(&mut self) -> usize {
        let Some(document) = self.document.as_ref() else {
            return 0;
        };
        let mut lines = render_document_interactive(
            &document.base_url,
            &document.source,
            self.cols.max(48),
            document,
        );
        let images = self.attach_html_images(&mut lines);
        self.lines = if lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            lines
        };
        images
    }

    fn focused_control_id(&self) -> Option<usize> {
        self.document
            .as_ref()
            .and_then(|document| document.focused_control)
    }

    fn handle_document_key(&mut self, c: char) -> bool {
        if c == '\t' {
            if let Some(document) = self.document.as_mut() {
                if document.focus_next_control() {
                    self.status = String::from("Control focused");
                    self.render();
                    return true;
                }
            }
        }
        let Some(document) = self.document.as_mut() else {
            return false;
        };
        if document.focused_control.is_none() {
            return false;
        }
        if c == '\u{1b}' {
            document.focused_control = None;
            self.status = String::from("Control focus cleared");
            self.render();
            return true;
        }
        if c == '\n' || c == '\r' || c == ' ' {
            let Some(id) = document.focused_control else {
                return false;
            };
            let kind = document.controls.get(id).map(|control| control.kind);
            if matches!(kind, Some(BrowserFormControlKind::Text)) && (c == '\n' || c == '\r') {
                if let Some(submit_id) = document.default_submit_for(id) {
                    self.activate_document_control(submit_id);
                    return true;
                }
            }
            if matches!(
                kind,
                Some(
                    BrowserFormControlKind::Submit
                        | BrowserFormControlKind::Button
                        | BrowserFormControlKind::Reset
                        | BrowserFormControlKind::Image
                )
            ) {
                self.activate_document_control(id);
                return true;
            }
        }
        if document.edit_focused_control(c) {
            self.status = String::from("Control edited");
            self.reflow_document();
            self.render();
            return true;
        }
        false
    }

    fn activate_document_control(&mut self, control_id: usize) {
        let activation = {
            let Some(document) = self.document.as_mut() else {
                return;
            };
            document.activate_control(control_id)
        };
        match activation {
            BrowserControlActivation::Ignored => {
                self.status = String::from("Control unavailable");
                self.render();
            }
            BrowserControlActivation::Focused => {
                self.status = String::from("Control focused");
                self.render();
            }
            BrowserControlActivation::Changed => {
                self.status = String::from("Control changed");
                self.reflow_document();
                self.render();
            }
            BrowserControlActivation::Navigate(url) => {
                self.status = format!("Submitting {}", url);
                self.navigate(&url, true);
            }
            BrowserControlActivation::Post { url, body } => {
                self.submit_post_form(&url, &body);
            }
            BrowserControlActivation::DomEvent(label) => {
                self.status = format!("DOM event: {}", label);
                self.render();
            }
        }
    }

    fn submit_post_form(&mut self, url: &str, body: &str) {
        self.address = String::from(url);
        self.status = format!("Submitting POST {} bytes...", body.len());
        self.title = String::from("Submitting");
        self.image_preview = None;
        self.inline_images.clear();
        self.document = None;
        self.lines = vec![kind_line("Submitting form...", BrowserLineKind::Muted)];
        self.scroll = 0;
        self.render();

        match parse_web_url(url) {
            Ok((_scheme, host, path)) => {
                match crate::net::browser_post_response(
                    url,
                    body,
                    "application/x-www-form-urlencoded",
                ) {
                    Ok(response) => {
                        self.apply_web_response(response, true, "Submitted POST");
                    }
                    Err(err) => {
                        self.title = String::from("POST failed");
                        self.status = format!("Network error: {}", err);
                        self.last_page = None;
                        self.image_preview = None;
                        self.inline_images.clear();
                        self.document = None;
                        self.lines = network_error_lines(url, &host, &path, err);
                    }
                }
            }
            Err(err) => {
                self.title = String::from("Unsupported POST target");
                self.status = String::from(err);
                self.last_page = None;
                self.image_preview = None;
                self.inline_images.clear();
                self.document = None;
                self.lines = vec![
                    kind_line("POST form target is not a web URL", BrowserLineKind::Error),
                    kind_line(
                        "Use an http:// or https:// form action.",
                        BrowserLineKind::Muted,
                    ),
                    line(""),
                    kind_line("Target", BrowserLineKind::Muted),
                    line(url),
                    kind_line("Body", BrowserLineKind::Muted),
                    kind_line(body, BrowserLineKind::Code),
                ];
            }
        }
        self.address_focused = false;
        self.address_selected = false;
        self.render();
    }

    fn decode_image_preview(&mut self, response: &crate::net::HttpResponse) -> Option<String> {
        self.decode_image_preview_bytes(
            &response.body_bytes,
            response.content_type.as_deref(),
            &response.final_url,
        )
    }

    fn decode_image_preview_bytes(
        &mut self,
        bytes: &[u8],
        content_type: Option<&str>,
        url: &str,
    ) -> Option<String> {
        self.image_preview = None;
        if !is_png_content(content_type, url) {
            let meta = image_metadata_label(bytes, content_type, url)
                .unwrap_or_else(|| String::from("image dimensions unknown"));
            return Some(format!("Preview unavailable: {} (PNG decoder only)", meta));
        }
        match crate::png::decode_rgb8(bytes, MAX_INLINE_PNG_PIXELS) {
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
        let viewport_h = self.rows.saturating_mul(LINE_H) as i32;
        let max = self
            .window
            .scroll
            .content_h
            .saturating_sub(viewport_h)
            .max(0);
        let next =
            (self.scroll as i32 + delta.saturating_mul(LINE_H as i32)).clamp(0, max) as usize;
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

        let doc_w = width.saturating_sub(PAD_X * 2 + 28).max(1);
        self.rows = lines_h / LINE_H;
        self.cols = doc_w / CHAR_W;
        let layout = layout_browser_lines(&self.lines, &self.inline_images, doc_w);
        let max_scroll = layout.content_h.saturating_sub(lines_h);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        self.window.scroll.content_h = layout.content_h as i32;
        self.window.scroll.offset = self.scroll as i32;
        self.window.scroll.clamp(lines_h as i32);
        self.scroll = self.window.scroll.offset.max(0) as usize;
        self.hit_boxes.clear();

        let viewport_bottom = self.scroll.saturating_add(lines_h);
        for item in layout.items.into_iter() {
            if item.y.saturating_add(item.h) <= self.scroll || item.y >= viewport_bottom {
                continue;
            }
            if item.y < self.scroll || item.y.saturating_add(item.h) > viewport_bottom {
                continue;
            }
            let y = lines_y + item.y.saturating_sub(self.scroll);
            let x = PAD_X + item.x;
            if y >= doc_y.saturating_add(doc_h) {
                continue;
            }

            if let Some(slot) = item.image_slot {
                if let Some(image) = self
                    .inline_images
                    .get(slot)
                    .map(|inline| inline.image.clone())
                {
                    if let Some(bg) = item.style.background {
                        self.fill_rect(
                            stride,
                            x.saturating_sub(4),
                            y.saturating_sub(2),
                            item.w.saturating_add(8),
                            item.h.saturating_add(4),
                            bg,
                        );
                    }
                    draw_image_preview_pixels(
                        &mut self.window.buf,
                        width,
                        content_h,
                        stride,
                        x,
                        y,
                        item.w,
                        item.h,
                        &image,
                        false,
                    );
                }
            } else if matches!(item.control, BrowserControl::None) {
                if let Some(bg) = item.style.background {
                    self.fill_rect(
                        stride,
                        x.saturating_sub(4),
                        y.saturating_sub(2),
                        item.w.saturating_add(8),
                        item.h.saturating_add(4),
                        bg,
                    );
                }
                let color = item
                    .style
                    .text_color
                    .unwrap_or_else(|| color_for_line(item.kind, item.link.is_some()));
                let mut text = item.text.clone();
                truncate_chars(&mut text, item.w / CHAR_W);
                self.put_str(stride, x, y, &text, color);
                if item.kind == BrowserLineKind::Heading {
                    self.put_str(stride, x + 1, y, &text, color);
                }
                if item.link.is_some() {
                    self.fill_rect(stride, x, y + 10, text.len() * CHAR_W, 1, LINK);
                }
            } else {
                if let Some(bg) = item.style.background {
                    self.fill_rect(
                        stride,
                        x.saturating_sub(4),
                        y.saturating_sub(2),
                        item.w.saturating_add(8),
                        item.h.saturating_add(4),
                        bg,
                    );
                }
                self.draw_control(
                    stride,
                    x,
                    y,
                    item.w,
                    &item.control,
                    item.link.is_some() || item.control_id.is_some(),
                    item.control_id == self.focused_control_id(),
                );
            }

            if item.link.is_some() || item.control_id.is_some() {
                self.hit_boxes.push(BrowserHitBox {
                    x,
                    y,
                    w: item.w,
                    h: item.h.max(LINE_H),
                    link: item.link,
                    control_id: item.control_id,
                });
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

    fn draw_control(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        control: &BrowserControl,
        active: bool,
        focused: bool,
    ) {
        let border = if focused {
            BUTTON_HOT
        } else if active {
            BUTTON_HOT
        } else {
            0x00_96_A8_B4
        };
        match control {
            BrowserControl::TextInput { label, value, .. } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, WHITE);
                self.draw_rect(stride, x, y, w, CONTROL_H, border);
                let shown = if value.is_empty() { label } else { value };
                if !shown.is_empty() {
                    let mut text = shown.clone();
                    truncate_chars(&mut text, w.saturating_sub(14) / CHAR_W);
                    if focused && text.len() < w.saturating_sub(14) / CHAR_W {
                        text.push('_');
                    }
                    self.put_str(
                        stride,
                        x + 7,
                        y + 8,
                        &text,
                        if value.is_empty() { MUTED } else { TEXT },
                    );
                }
            }
            BrowserControl::Button { label } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, 0x00_E8_E8_E8);
                self.draw_rect(
                    stride,
                    x,
                    y,
                    w,
                    CONTROL_H,
                    if focused || active {
                        BUTTON_HOT
                    } else {
                        0x00_88_88_88
                    },
                );
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(12) / CHAR_W);
                let text_w = text.len().saturating_mul(CHAR_W);
                let tx = x + w.saturating_sub(text_w) / 2;
                self.put_str(stride, tx, y + 8, &text, TEXT);
            }
            BrowserControl::Checkbox { label, checked } => {
                self.fill_rect(stride, x, y + 5, 12, 12, WHITE);
                self.draw_rect(stride, x, y + 5, 12, 12, border);
                if *checked {
                    self.put_str(stride, x + 2, y + 6, "x", TEXT);
                }
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(18) / CHAR_W);
                self.put_str(stride, x + 18, y + 6, &text, TEXT);
            }
            BrowserControl::Radio { label, checked } => {
                self.fill_rect(stride, x, y + 5, 12, 12, WHITE);
                self.draw_rect(stride, x, y + 5, 12, 12, border);
                if *checked {
                    self.fill_rect(stride, x + 4, y + 9, 4, 4, TEXT);
                }
                let mut text = label.clone();
                truncate_chars(&mut text, w.saturating_sub(18) / CHAR_W);
                self.put_str(stride, x + 18, y + 6, &text, TEXT);
            }
            BrowserControl::Select {
                label,
                value,
                options,
            } => {
                self.fill_rect(stride, x, y, w, CONTROL_H, WHITE);
                self.draw_rect(stride, x, y, w, CONTROL_H, border);
                let mut text = if !value.is_empty() {
                    format!("{}: {}", label, value)
                } else if *options > 0 {
                    format!("{} ({} options)", label, options)
                } else {
                    label.clone()
                };
                truncate_chars(&mut text, w.saturating_sub(28) / CHAR_W);
                self.put_str(stride, x + 7, y + 8, &text, TEXT);
                self.put_str(stride, x + w.saturating_sub(18), y + 8, "v", MUTED);
            }
            BrowserControl::TextArea { label, value, rows } => {
                let h = CONTROL_H.saturating_add(rows.saturating_sub(1).min(5) * 10);
                self.fill_rect(stride, x, y, w, h, WHITE);
                self.draw_rect(stride, x, y, w, h, border);
                let mut text = if value.is_empty() {
                    label.clone()
                } else {
                    value.clone()
                };
                truncate_chars(&mut text, w.saturating_sub(14) / CHAR_W);
                self.put_str(
                    stride,
                    x + 7,
                    y + 8,
                    &text,
                    if value.is_empty() { MUTED } else { TEXT },
                );
            }
            BrowserControl::None => {}
        }
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
            true,
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

fn color_for_line(kind: BrowserLineKind, linked: bool) -> u32 {
    match kind {
        BrowserLineKind::Heading => 0x00_04_24_3A,
        BrowserLineKind::Muted => MUTED,
        BrowserLineKind::Link => LINK,
        BrowserLineKind::Image => 0x00_7A_3B_00,
        BrowserLineKind::Quote => 0x00_40_55_5D,
        BrowserLineKind::Code => 0x00_22_33_33,
        BrowserLineKind::Error => 0x00_AA_20_20,
        BrowserLineKind::Text => {
            if linked {
                LINK
            } else {
                TEXT
            }
        }
    }
}

fn layout_browser_lines(
    lines: &[BrowserLine],
    inline_images: &[InlineImage],
    doc_w: usize,
) -> BrowserLayout {
    let mut items = Vec::new();
    let mut y = 0usize;
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.kind == BrowserLineKind::Image
            && line.text.trim().is_empty()
            && line.image_slot.is_none()
        {
            i += 1;
            continue;
        }
        if line.text.trim().is_empty()
            && line.image_slot.is_none()
            && matches!(line.control, BrowserControl::None)
        {
            y = y.saturating_add(LINE_H);
            i += 1;
            continue;
        }
        if line.kind == BrowserLineKind::Link
            && !line.text.trim().is_empty()
            && line.image_slot.is_none()
            && matches!(line.control, BrowserControl::None)
        {
            let align = line.align;
            let mut group = Vec::new();
            let mut total_w = 0usize;
            let mut j = i;
            while let Some(next) = lines.get(j) {
                if next.text.trim().is_empty()
                    && next.image_slot.is_none()
                    && matches!(next.control, BrowserControl::None)
                    && !group.is_empty()
                {
                    j += 1;
                    continue;
                }
                if next.kind != BrowserLineKind::Link
                    || next.align != align
                    || next.image_slot.is_some()
                    || !matches!(next.control, BrowserControl::None)
                    || next.text.trim().is_empty()
                {
                    break;
                }
                if next.style != line.style {
                    break;
                }
                let available_w = doc_w.saturating_sub(next.style.indent_px).max(1);
                let w = text_pixel_width(&next.text).min(available_w);
                let candidate = if group.is_empty() {
                    w
                } else {
                    total_w.saturating_add(CONTROL_GAP + 8).saturating_add(w)
                };
                if candidate > doc_w && !group.is_empty() {
                    break;
                }
                total_w = candidate;
                group.push((j, w));
                j += 1;
            }
            if group.len() > 1 {
                let available_w = doc_w.saturating_sub(line.style.indent_px).max(1);
                let mut x =
                    line.style
                        .indent_px
                        .saturating_add(aligned_x(available_w, total_w, align));
                for (idx, w) in group {
                    let next = &lines[idx];
                    items.push(BrowserLayoutItem {
                        x,
                        y,
                        w,
                        h: LINE_H,
                        text: next.text.clone(),
                        link: next.link.clone(),
                        kind: next.kind,
                        control: BrowserControl::None,
                        image_slot: None,
                        style: next.style,
                        control_id: next.control_id,
                    });
                    x = x.saturating_add(w).saturating_add(CONTROL_GAP + 8);
                }
                y = y.saturating_add(LINE_H + BLOCK_GAP);
                i = j;
                continue;
            }
        }
        if matches!(line.control, BrowserControl::Button { .. }) {
            let align = line.align;
            let mut group = Vec::new();
            let mut total_w = 0usize;
            let mut j = i;
            while let Some(next) = lines.get(j) {
                if next.align != align
                    || next.style != line.style
                    || !matches!(next.control, BrowserControl::Button { .. })
                {
                    break;
                }
                let available_w = doc_w.saturating_sub(next.style.indent_px).max(1);
                let w = control_width(&next.control, available_w);
                let candidate = if group.is_empty() {
                    w
                } else {
                    total_w.saturating_add(CONTROL_GAP).saturating_add(w)
                };
                if candidate > doc_w && !group.is_empty() {
                    break;
                }
                total_w = candidate;
                group.push((j, w));
                j += 1;
            }
            let available_w = doc_w.saturating_sub(line.style.indent_px).max(1);
            let mut x = line
                .style
                .indent_px
                .saturating_add(aligned_x(available_w, total_w, align));
            for (idx, w) in group {
                let next = &lines[idx];
                items.push(BrowserLayoutItem {
                    x,
                    y,
                    w,
                    h: CONTROL_H,
                    text: next.text.clone(),
                    link: next.link.clone(),
                    kind: next.kind,
                    control: next.control.clone(),
                    image_slot: None,
                    style: next.style,
                    control_id: next.control_id,
                });
                x = x.saturating_add(w).saturating_add(CONTROL_GAP);
            }
            y = y.saturating_add(CONTROL_H + BLOCK_GAP);
            i = j;
            continue;
        }
        if !matches!(line.control, BrowserControl::None) {
            let available_w = doc_w.saturating_sub(line.style.indent_px).max(1);
            let w = control_width(&line.control, available_w);
            let h = control_height(&line.control);
            items.push(BrowserLayoutItem {
                x: line
                    .style
                    .indent_px
                    .saturating_add(aligned_x(available_w, w, line.align)),
                y,
                w,
                h,
                text: line.text.clone(),
                link: line.link.clone(),
                kind: line.kind,
                control: line.control.clone(),
                image_slot: None,
                style: line.style,
                control_id: line.control_id,
            });
            y = y.saturating_add(h + BLOCK_GAP);
            i += 1;
            continue;
        }
        if let Some(slot) = line.image_slot {
            if let Some(image) = inline_images.get(slot).map(|inline| &inline.image) {
                let max_w = doc_w.saturating_sub(line.style.indent_px).max(1);
                let (draw_w, draw_h) = scaled_image_size_with_hint(
                    image.width,
                    image.height,
                    line.image_hint,
                    max_w,
                    INLINE_IMAGE_MAX_H,
                );
                items.push(BrowserLayoutItem {
                    x: line
                        .style
                        .indent_px
                        .saturating_add(aligned_x(max_w, draw_w, line.align)),
                    y,
                    w: draw_w,
                    h: draw_h,
                    text: String::new(),
                    link: line.link.clone(),
                    kind: BrowserLineKind::Image,
                    control: BrowserControl::None,
                    image_slot: Some(slot),
                    style: line.style,
                    control_id: line.control_id,
                });
                y = y.saturating_add(draw_h + BLOCK_GAP);
                i += 1;
                continue;
            }
        }
        let available_w = doc_w.saturating_sub(line.style.indent_px).max(1);
        let w = text_pixel_width(&line.text).min(available_w);
        let h = if line.kind == BrowserLineKind::Heading {
            LINE_H + 2
        } else {
            LINE_H
        };
        items.push(BrowserLayoutItem {
            x: line
                .style
                .indent_px
                .saturating_add(aligned_x(available_w, w, line.align)),
            y,
            w,
            h,
            text: line.text.clone(),
            link: line.link.clone(),
            kind: line.kind,
            control: BrowserControl::None,
            image_slot: None,
            style: line.style,
            control_id: line.control_id,
        });
        y = y.saturating_add(h);
        i += 1;
    }
    BrowserLayout {
        items,
        content_h: y.saturating_add(BLOCK_GAP),
    }
}

fn aligned_x(doc_w: usize, item_w: usize, align: BrowserAlign) -> usize {
    match align {
        BrowserAlign::Left => 0,
        BrowserAlign::Center => doc_w.saturating_sub(item_w) / 2,
        BrowserAlign::Right => doc_w.saturating_sub(item_w),
    }
}

fn text_pixel_width(text: &str) -> usize {
    text.chars().count().saturating_mul(CHAR_W)
}

fn control_width(control: &BrowserControl, doc_w: usize) -> usize {
    let w = match control {
        BrowserControl::TextInput { chars, .. } => {
            (*chars).clamp(8, 72).saturating_mul(CHAR_W) + 18
        }
        BrowserControl::Button { label } => {
            text_pixel_width(label).saturating_add(24).clamp(74, 220)
        }
        BrowserControl::Checkbox { label, .. } | BrowserControl::Radio { label, .. } => {
            text_pixel_width(label).saturating_add(24).clamp(48, 260)
        }
        BrowserControl::Select { label, .. } | BrowserControl::TextArea { label, .. } => {
            text_pixel_width(label).saturating_add(34).clamp(120, 360)
        }
        BrowserControl::None => 0,
    };
    w.min(doc_w)
}

fn control_height(control: &BrowserControl) -> usize {
    match control {
        BrowserControl::TextArea { rows, .. } => {
            CONTROL_H.saturating_add(rows.saturating_sub(1).min(5) * 10)
        }
        _ => CONTROL_H,
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
    framed: bool,
) -> usize {
    if image.width == 0 || image.height == 0 || max_w < 8 || max_h < 8 {
        return 0;
    }
    let (mut draw_w, mut draw_h) = scaled_image_size(image.width, image.height, max_w, max_h);
    draw_w = draw_w.max(1);
    draw_h = draw_h.max(1);

    let (image_x, image_y, used_h) = if framed {
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
        (x + 4, y + 4, frame_h)
    } else {
        (x, y, draw_h)
    };

    for dy in 0..draw_h {
        let src_y = dy.saturating_mul(image.height) / draw_h;
        for dx in 0..draw_w {
            let src_x = dx.saturating_mul(image.width) / draw_w;
            let Some(&color) = image.pixels.get(src_y * image.width + src_x) else {
                continue;
            };
            let px = image_x + dx;
            let py = image_y + dy;
            let idx = py.saturating_mul(stride).saturating_add(px);
            if px < surface_w && py < surface_h && idx < buf.len() {
                buf[idx] = color;
            }
        }
    }
    used_h
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
        link_line("Session state", SESSION_INTERNAL_URL),
    ]
}

fn browser_session_lines() -> Vec<BrowserLine> {
    crate::browser_session::lines()
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == 0 {
                kind_line(&text, BrowserLineKind::Heading)
            } else if text.is_empty() {
                BrowserLine::new(String::new(), None, BrowserLineKind::Text)
            } else if text.starts_with("Cookie jar:")
                || text.starts_with("Storage:")
                || text == "No cookies stored."
            {
                kind_line(&text, BrowserLineKind::Muted)
            } else {
                line(&text)
            }
        })
        .collect()
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
    out.push(BrowserLine::new(
        format!("{} bytes received", byte_len),
        None,
        BrowserLineKind::Image,
    ));
    out.push(link_line("Image source URL", url));
    out
}

fn is_success_status(status_line: &str) -> bool {
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.as_bytes().first().copied())
        == Some(b'2')
}

fn is_image_content(content_type: Option<&str>) -> bool {
    content_type
        .map(|value| value.trim().to_ascii_lowercase().starts_with("image/"))
        .unwrap_or(false)
}

fn is_known_image_path(path: &str) -> bool {
    matches!(extension_from_path(path), "png" | "jpg" | "gif" | "webp")
}

fn looks_like_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
}

fn image_content_type_for(path: &str, bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") || extension_from_path(path) == "png" {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8") || extension_from_path(path) == "jpg" {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || extension_from_path(path) == "gif"
    {
        Some("image/gif")
    } else if (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
        || extension_from_path(path) == "webp"
    {
        Some("image/webp")
    } else {
        None
    }
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
    BrowserLine::new(
        String::new(),
        Some(String::from(url)),
        BrowserLineKind::Image,
    )
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

fn scaled_image_size_with_hint(
    image_w: usize,
    image_h: usize,
    hint: ImageHint,
    max_w: usize,
    max_h: usize,
) -> (usize, usize) {
    if image_w == 0 || image_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    let Some(mut draw_w) = hint.width else {
        let Some(mut draw_h) = hint.height else {
            return scaled_image_size(image_w, image_h, max_w, max_h);
        };
        draw_h = draw_h.clamp(1, max_h);
        let draw_w = image_w
            .saturating_mul(draw_h)
            .saturating_div(image_h)
            .max(1);
        return fit_box(draw_w, draw_h, max_w, max_h);
    };
    draw_w = draw_w.clamp(1, max_w);
    let draw_h = hint
        .height
        .unwrap_or_else(|| {
            image_h
                .saturating_mul(draw_w)
                .saturating_div(image_w)
                .max(1)
        })
        .clamp(1, max_h);
    fit_box(draw_w, draw_h, max_w, max_h)
}

fn fit_box(mut w: usize, mut h: usize, max_w: usize, max_h: usize) -> (usize, usize) {
    if w > max_w {
        h = h.saturating_mul(max_w).saturating_div(w).max(1);
        w = max_w;
    }
    if h > max_h {
        w = w.saturating_mul(max_h).saturating_div(h).max(1);
        h = max_h;
    }
    (w.min(max_w), h.min(max_h))
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

fn image_metadata_label(bytes: &[u8], content_type: Option<&str>, url: &str) -> Option<String> {
    let kind = image_kind_label(content_type, url, bytes);
    if let Some((w, h)) = png_dimensions(bytes)
        .or_else(|| gif_dimensions(bytes))
        .or_else(|| jpeg_dimensions(bytes))
        .or_else(|| webp_dimensions(bytes))
    {
        Some(format!("{} {}x{}", kind, w, h))
    } else if kind != "image" {
        Some(String::from(kind))
    } else {
        None
    }
}

fn image_kind_label(content_type: Option<&str>, url: &str, bytes: &[u8]) -> &'static str {
    let ct = content_type
        .and_then(|value| value.split(';').next())
        .unwrap_or("")
        .trim();
    if ct.eq_ignore_ascii_case("image/png") || bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "PNG"
    } else if ct.eq_ignore_ascii_case("image/jpeg")
        || ct.eq_ignore_ascii_case("image/jpg")
        || bytes.starts_with(b"\xff\xd8")
    {
        "JPEG"
    } else if ct.eq_ignore_ascii_case("image/gif")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
    {
        "GIF"
    } else if ct.eq_ignore_ascii_case("image/webp")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
    {
        "WebP"
    } else {
        match extension_from_path(url) {
            "png" => "PNG",
            "jpg" => "JPEG",
            "gif" => "GIF",
            "webp" => "WebP",
            _ => "image",
        }
    }
}

fn png_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let w = read_be_u32_local(bytes, 16)? as usize;
    let h = read_be_u32_local(bytes, 20)? as usize;
    if w == 0 || h == 0 {
        None
    } else {
        Some((w, h))
    }
}

fn gif_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 10 || !(bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return None;
    }
    let w = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let h = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    if w == 0 || h == 0 {
        None
    } else {
        Some((w, h))
    }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 4 || !bytes.starts_with(b"\xff\xd8") {
        return None;
    }
    let mut pos = 2usize;
    while pos + 9 < bytes.len() && pos < 4096 {
        if bytes[pos] != 0xff {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xff {
            pos += 1;
        }
        let marker = *bytes.get(pos)?;
        pos += 1;
        if matches!(marker, 0xd8 | 0xd9 | 0x01) {
            continue;
        }
        let len = read_be_u16_local(bytes, pos)? as usize;
        if len < 2 || pos + len > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let h = read_be_u16_local(bytes, pos + 3)? as usize;
            let w = read_be_u16_local(bytes, pos + 5)? as usize;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
            return None;
        }
        pos += len;
    }
    None
}

fn webp_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    if &bytes[12..16] == b"VP8X" && bytes.len() >= 30 {
        let w = 1 + read_le_u24_local(bytes, 24)? as usize;
        let h = 1 + read_le_u24_local(bytes, 27)? as usize;
        return Some((w, h));
    }
    None
}

fn read_be_u32_local(bytes: &[u8], pos: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *bytes.get(pos)?,
        *bytes.get(pos + 1)?,
        *bytes.get(pos + 2)?,
        *bytes.get(pos + 3)?,
    ]))
}

fn read_be_u16_local(bytes: &[u8], pos: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(pos)?, *bytes.get(pos + 1)?]))
}

fn read_le_u24_local(bytes: &[u8], pos: usize) -> Option<u32> {
    Some(
        (*bytes.get(pos)? as u32)
            | ((*bytes.get(pos + 1)? as u32) << 8)
            | ((*bytes.get(pos + 2)? as u32) << 16),
    )
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
    let response = crate::net::browser_get_response(url)?;
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
    BrowserLine::new(String::from(text), None, BrowserLineKind::Text)
}

fn kind_line(text: &str, kind: BrowserLineKind) -> BrowserLine {
    BrowserLine::new(String::from(text), None, kind)
}

fn link_line(text: &str, url: &str) -> BrowserLine {
    BrowserLine::new(
        String::from(text),
        Some(String::from(url)),
        BrowserLineKind::Link,
    )
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

fn browser_event_url(label: &str) -> String {
    let mut out = String::from("browser://event?label=");
    push_query_encoded(&mut out, label);
    out
}

fn browser_event_label(url: &str) -> Option<String> {
    let query = url.strip_prefix("browser://event?label=")?;
    Some(decode_query(query))
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CssDisplay {
    Default,
    None,
    Block,
    Inline,
    ListItem,
    Table,
    Flex,
    Grid,
}

impl Default for CssDisplay {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Clone, Copy, Default)]
struct CssDeclarations {
    display: Option<CssDisplay>,
    visibility_hidden: Option<bool>,
    align: Option<BrowserAlign>,
    indent_px: Option<usize>,
    color: Option<u32>,
    background: Option<u32>,
    width: Option<usize>,
    height: Option<usize>,
    preformatted: Option<bool>,
}

#[derive(Clone)]
struct CssRule {
    selector: String,
    declarations: CssDeclarations,
    specificity: u16,
    order: usize,
}

#[derive(Clone, Copy)]
struct CssSlot<T: Copy> {
    value: Option<T>,
    specificity: u16,
    order: usize,
}

impl<T: Copy> Default for CssSlot<T> {
    fn default() -> Self {
        Self {
            value: None,
            specificity: 0,
            order: 0,
        }
    }
}

impl<T: Copy> CssSlot<T> {
    fn apply(&mut self, value: Option<T>, specificity: u16, order: usize) {
        let Some(value) = value else {
            return;
        };
        if self.value.is_none()
            || specificity > self.specificity
            || (specificity == self.specificity && order >= self.order)
        {
            self.value = Some(value);
            self.specificity = specificity;
            self.order = order;
        }
    }
}

#[derive(Default)]
struct CssCascade {
    display: CssSlot<CssDisplay>,
    visibility_hidden: CssSlot<bool>,
    align: CssSlot<BrowserAlign>,
    indent_px: CssSlot<usize>,
    color: CssSlot<u32>,
    background: CssSlot<u32>,
    width: CssSlot<usize>,
    height: CssSlot<usize>,
    preformatted: CssSlot<bool>,
}

impl CssCascade {
    fn apply(&mut self, declarations: CssDeclarations, specificity: u16, order: usize) {
        self.display.apply(declarations.display, specificity, order);
        self.visibility_hidden
            .apply(declarations.visibility_hidden, specificity, order);
        self.align.apply(declarations.align, specificity, order);
        self.indent_px
            .apply(declarations.indent_px, specificity, order);
        self.color.apply(declarations.color, specificity, order);
        self.background
            .apply(declarations.background, specificity, order);
        self.width.apply(declarations.width, specificity, order);
        self.height.apply(declarations.height, specificity, order);
        self.preformatted
            .apply(declarations.preformatted, specificity, order);
    }
}

#[derive(Clone, Copy, Default)]
struct TagStyle {
    hidden: bool,
    display: CssDisplay,
    align: Option<BrowserAlign>,
    line: BrowserLineStyle,
    width: Option<usize>,
    height: Option<usize>,
    preformatted: bool,
}

struct StyleHints {
    hidden_classes: Vec<String>,
    rules: Vec<CssRule>,
}

impl StyleHints {
    fn from_document(body: &str) -> Self {
        let lower = lowercase_ascii(body);
        let mut hints = Self {
            hidden_classes: Vec::new(),
            rules: Vec::new(),
        };
        let mut i = 0usize;
        while let Some(rel) = lower[i..].find("<style") {
            let start = i + rel;
            let Some(tag_end) = find_tag_end(&lower[start..]) else {
                break;
            };
            let content_start = start + tag_end + 1;
            let Some(close_rel) = lower[content_start..].find("</style") else {
                break;
            };
            let content_end = content_start + close_rel;
            collect_css_hints(&lower[content_start..content_end], &mut hints);
            i = content_end + "</style".len();
        }
        hints
    }

    fn has_hidden_class(&self, tag: &str) -> bool {
        let Some(classes) = attr_value(tag, "class") else {
            return false;
        };
        classes.split_whitespace().any(|class| {
            let class = lowercase_ascii(class);
            contains_string(&self.hidden_classes, &class)
        })
    }

    fn computed_for_tag(&self, lower_tag: &str, name: &str) -> TagStyle {
        let mut cascade = CssCascade::default();
        for rule in &self.rules {
            if selector_matches_tag(&rule.selector, lower_tag, name) {
                cascade.apply(rule.declarations, rule.specificity, rule.order);
            }
        }
        if let Some(style) = attr_value(lower_tag, "style") {
            cascade.apply(parse_css_declarations(&style), 1000, usize::MAX);
        }
        let display = cascade.display.value.unwrap_or(CssDisplay::Default);
        let hidden = display == CssDisplay::None
            || cascade.visibility_hidden.value.unwrap_or(false)
            || self.has_hidden_class(lower_tag);
        TagStyle {
            hidden,
            display,
            align: cascade.align.value,
            line: BrowserLineStyle {
                indent_px: cascade.indent_px.value.unwrap_or(0).min(160),
                text_color: cascade.color.value,
                background: cascade.background.value,
            },
            width: cascade.width.value,
            height: cascade.height.value,
            preformatted: cascade.preformatted.value.unwrap_or(false),
        }
    }
}

fn collect_css_hints(css: &str, hints: &mut StyleHints) {
    let mut pos = 0usize;
    while let Some(open_rel) = css[pos..].find('{') {
        let open = pos + open_rel;
        let selectors = &css[pos..open];
        let Some(close_rel) = css[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + close_rel;
        let rules = &css[open + 1..close];
        if selectors.contains('@') {
            pos = close + 1;
            continue;
        }
        let declarations = parse_css_declarations(rules);
        if declarations.display == Some(CssDisplay::None)
            || declarations.visibility_hidden == Some(true)
        {
            collect_selector_classes(selectors, &mut hints.hidden_classes);
        }
        for selector in selectors.split(',') {
            let selector = selector.trim();
            if selector.is_empty() || selector.len() > 96 {
                continue;
            }
            if hints.rules.len() >= 192 {
                break;
            }
            hints.rules.push(CssRule {
                selector: String::from(selector),
                declarations,
                specificity: selector_specificity(selector),
                order: hints.rules.len(),
            });
        }
        pos = close + 1;
    }
}

fn collect_selector_classes(selectors: &str, out: &mut Vec<String>) {
    for selector in selectors.split(',') {
        let selector = selector.trim();
        let Some(rest) = selector.strip_prefix('.') else {
            continue;
        };
        let bytes = rest.as_bytes();
        let mut end = 0usize;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'-' | b'_'))
        {
            end += 1;
        }
        if end == 0 || !rest[end..].trim().is_empty() {
            continue;
        }
        push_unique_class(out, &rest[..end]);
    }
}

fn parse_css_declarations(input: &str) -> CssDeclarations {
    let mut out = CssDeclarations::default();
    for declaration in input.split(';') {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        let name = lowercase_ascii(name.trim());
        let value = lowercase_ascii(value.trim());
        match name.as_str() {
            "display" => {
                out.display = match value.as_str() {
                    "none" => Some(CssDisplay::None),
                    "block" => Some(CssDisplay::Block),
                    "inline" | "inline-block" => Some(CssDisplay::Inline),
                    "list-item" => Some(CssDisplay::ListItem),
                    "table" | "table-row" | "table-cell" => Some(CssDisplay::Table),
                    "flex" | "inline-flex" => Some(CssDisplay::Flex),
                    "grid" | "inline-grid" => Some(CssDisplay::Grid),
                    _ => out.display,
                };
            }
            "visibility" => {
                if value == "hidden" || value == "collapse" {
                    out.visibility_hidden = Some(true);
                }
            }
            "text-align" => out.align = parse_alignment(&value),
            "margin" => {
                if value.contains("auto") {
                    out.align = Some(BrowserAlign::Center);
                } else {
                    out.indent_px = first_css_length_px(&value).or(out.indent_px);
                }
            }
            "margin-left" | "padding-left" | "text-indent" => {
                out.indent_px = parse_css_length_px(&value).or(out.indent_px);
            }
            "color" => out.color = parse_css_color(&value).or(out.color),
            "background" | "background-color" => {
                out.background = parse_css_color(&value).or(out.background)
            }
            "width" | "max-width" => out.width = parse_css_length_px(&value).or(out.width),
            "height" | "max-height" => out.height = parse_css_length_px(&value).or(out.height),
            "white-space" => {
                if value == "pre" || value == "pre-wrap" || value == "break-spaces" {
                    out.preformatted = Some(true);
                }
            }
            _ => {}
        }
    }
    out
}

fn first_css_length_px(value: &str) -> Option<usize> {
    for part in value.split_whitespace() {
        if let Some(length) = parse_css_length_px(part) {
            return Some(length);
        }
    }
    None
}

fn parse_css_length_px(value: &str) -> Option<usize> {
    let value = value.trim();
    if value.is_empty() || value == "auto" || value.ends_with('%') {
        return None;
    }
    let mut number = 0usize;
    let mut saw_digit = false;
    let mut decimal = false;
    for b in value.bytes() {
        if b.is_ascii_digit() {
            if !decimal {
                number = number
                    .saturating_mul(10)
                    .saturating_add((b - b'0') as usize);
            }
            saw_digit = true;
        } else if b == b'.' {
            decimal = true;
        } else {
            break;
        }
    }
    if !saw_digit || number == 0 {
        return None;
    }
    if value.contains("em") || value.contains("rem") {
        Some(number.saturating_mul(16).min(2048))
    } else {
        Some(number.min(2048))
    }
}

fn parse_css_color(value: &str) -> Option<u32> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if let Some(args) = value.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let mut parts = args.split(',');
        let r = parse_css_color_component(parts.next()?)?;
        let g = parse_css_color_component(parts.next()?)?;
        let b = parse_css_color_component(parts.next()?)?;
        return Some(((r as u32) << 16) | ((g as u32) << 8) | b as u32);
    }
    match value {
        "black" => Some(0x00_00_00_00),
        "white" => Some(0x00_FF_FF_FF),
        "red" => Some(0x00_D0_22_22),
        "green" => Some(0x00_1E_8A_3D),
        "blue" => Some(0x00_00_4E_C4),
        "navy" => Some(0x00_00_20_60),
        "teal" => Some(0x00_00_7A_7A),
        "purple" => Some(0x00_72_34_A8),
        "gray" | "grey" => Some(0x00_70_70_70),
        "silver" => Some(0x00_C0_C0_C0),
        "maroon" => Some(0x00_80_20_20),
        "orange" => Some(0x00_D9_78_00),
        "yellow" => Some(0x00_D0_B8_00),
        "transparent" => None,
        _ => None,
    }
}

fn parse_hex_color(hex: &str) -> Option<u32> {
    let bytes = hex.as_bytes();
    if bytes.len() == 3 {
        let r = hex_value(bytes[0])?;
        let g = hex_value(bytes[1])?;
        let b = hex_value(bytes[2])?;
        return Some(((r as u32 * 17) << 16) | ((g as u32 * 17) << 8) | (b as u32 * 17));
    }
    if bytes.len() == 6 {
        let r = (hex_value(bytes[0])? << 4) | hex_value(bytes[1])?;
        let g = (hex_value(bytes[2])? << 4) | hex_value(bytes[3])?;
        let b = (hex_value(bytes[4])? << 4) | hex_value(bytes[5])?;
        return Some(((r as u32) << 16) | ((g as u32) << 8) | b as u32);
    }
    None
}

fn parse_css_color_component(value: &str) -> Option<u8> {
    let value = value.trim();
    if value.ends_with('%') {
        let pct = parse_dimension(value.trim_end_matches('%'))?.min(100);
        Some((pct.saturating_mul(255) / 100) as u8)
    } else {
        Some(parse_dimension(value)?.min(255) as u8)
    }
}

fn selector_matches_tag(selector: &str, lower_tag: &str, name: &str) -> bool {
    let mut last = "";
    for part in selector.split(|c: char| c.is_ascii_whitespace() || matches!(c, '>' | '+' | '~')) {
        if !part.trim().is_empty() {
            last = part.trim();
        }
    }
    if last.is_empty() {
        return false;
    }
    matches_compound_selector(last, lower_tag, name)
}

fn matches_compound_selector(selector: &str, lower_tag: &str, name: &str) -> bool {
    let selector = selector
        .split(':')
        .next()
        .unwrap_or(selector)
        .trim_matches('/');
    if selector == "*" {
        return true;
    }
    let bytes = selector.as_bytes();
    let mut pos = 0usize;
    let mut required_tag = "";
    while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-')) {
        pos += 1;
    }
    if pos > 0 {
        required_tag = &selector[..pos];
    }
    if !required_tag.is_empty() && required_tag != name {
        return false;
    }
    while pos < bytes.len() {
        match bytes[pos] {
            b'.' => {
                pos += 1;
                let start = pos;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
                {
                    pos += 1;
                }
                if start == pos || !tag_has_class(lower_tag, &selector[start..pos]) {
                    return false;
                }
            }
            b'#' => {
                pos += 1;
                let start = pos;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'_'))
                {
                    pos += 1;
                }
                if start == pos || !tag_has_id(lower_tag, &selector[start..pos]) {
                    return false;
                }
            }
            b'[' => {
                let Some(end) = selector[pos + 1..].find(']') else {
                    return false;
                };
                let attr = selector[pos + 1..pos + 1 + end]
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim();
                if attr.is_empty() || !has_attr(lower_tag, attr) {
                    return false;
                }
                pos += end + 2;
            }
            _ => return false,
        }
    }
    !required_tag.is_empty()
        || selector.starts_with('.')
        || selector.starts_with('#')
        || selector.starts_with('[')
}

fn selector_specificity(selector: &str) -> u16 {
    let mut ids = 0u16;
    let mut classes = 0u16;
    let mut tags = 0u16;
    for part in selector.split(|c: char| c.is_ascii_whitespace() || matches!(c, '>' | '+' | '~')) {
        let part = part.split(':').next().unwrap_or(part).trim();
        if part.is_empty() || part == "*" {
            continue;
        }
        let bytes = part.as_bytes();
        let mut pos = 0usize;
        if bytes
            .first()
            .map(|b| b.is_ascii_alphabetic())
            .unwrap_or(false)
        {
            tags = tags.saturating_add(1);
            while pos < bytes.len()
                && (bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-'))
            {
                pos += 1;
            }
        }
        while pos < bytes.len() {
            match bytes[pos] {
                b'#' => {
                    ids = ids.saturating_add(1);
                    pos += 1;
                }
                b'.' | b'[' => {
                    classes = classes.saturating_add(1);
                    pos += 1;
                }
                _ => pos += 1,
            }
        }
    }
    ids.saturating_mul(100)
        .saturating_add(classes.saturating_mul(10))
        .saturating_add(tags)
}

fn tag_has_class(lower_tag: &str, class: &str) -> bool {
    attr_value(lower_tag, "class")
        .map(|classes| classes.split_whitespace().any(|value| value == class))
        .unwrap_or(false)
}

fn tag_has_id(lower_tag: &str, id: &str) -> bool {
    attr_value(lower_tag, "id")
        .map(|value| lowercase_ascii(value.trim()) == id)
        .unwrap_or(false)
}

fn push_unique_class(out: &mut Vec<String>, class: &str) {
    if class.is_empty() || out.len() >= 96 || contains_string(out, class) {
        return;
    }
    out.push(String::from(class));
}

fn contains_string(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

struct BrowserRenderControls<'a> {
    document: &'a BrowserDocumentState,
    cursor: usize,
}

impl<'a> BrowserRenderControls<'a> {
    fn new(document: &'a BrowserDocumentState) -> Self {
        Self {
            document,
            cursor: 0,
        }
    }

    fn next(&mut self) -> Option<(usize, &'a BrowserFormControlState)> {
        let id = self.cursor;
        self.cursor = self.cursor.saturating_add(1);
        self.document.controls.get(id).map(|control| (id, control))
    }
}

fn render_document(base_url: &str, response: &str, cols: usize) -> Vec<BrowserLine> {
    render_document_core(base_url, response, cols, None)
}

fn render_document_interactive(
    base_url: &str,
    response: &str,
    cols: usize,
    document: &BrowserDocumentState,
) -> Vec<BrowserLine> {
    let mut controls = BrowserRenderControls::new(document);
    render_document_core(base_url, response, cols, Some(&mut controls))
}

fn render_document_core(
    base_url: &str,
    response: &str,
    cols: usize,
    mut controls: Option<&mut BrowserRenderControls<'_>>,
) -> Vec<BrowserLine> {
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
    let style_hints = StyleHints::from_document(body);
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
                let tag_style = style_hints.computed_for_tag(&lower_tag, lower_name);
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
                    || ((tag_is_hidden(&lower_tag) || tag_style.hidden)
                        && !lower_tag.starts_with("input"))
                {
                    flush_flow_text(&mut out, &mut text, cols, &mut state);
                    if !is_void_element(lower_name) {
                        state.skip_until = Some(closing_tag_for(&lower_tag));
                    }
                    i += end_rel + 1;
                    continue;
                }
                handle_tag(
                    tag,
                    &lower_tag,
                    &style_hints,
                    &mut out,
                    &mut text,
                    &mut state,
                    controls.as_deref_mut(),
                    base_url,
                    cols,
                );
                i += end_rel + 1;
                continue;
            }
        }
        if state.skip_until.is_none() && !state.suppresses_text() {
            if state.in_table_cell {
                push_text_char(&mut state.table_cell_text, bytes[i] as char, false);
            } else {
                push_text_char(&mut text, bytes[i] as char, state.is_preformatted());
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

pub fn render_document_style_debug_for_test(
    base_url: &str,
    response: &str,
    cols: usize,
) -> Vec<String> {
    render_document(base_url, response, cols)
        .into_iter()
        .filter(|line| !line.text.is_empty())
        .map(|line| {
            let mut out = line.text;
            if line.style.indent_px > 0 {
                out.push_str(" [indent=");
                out.push_str(&format!("{}", line.style.indent_px));
                out.push(']');
            }
            if let Some(color) = line.style.text_color {
                out.push_str(" [color=");
                push_hex_color(&mut out, color);
                out.push(']');
            }
            if let Some(background) = line.style.background {
                out.push_str(" [bg=");
                push_hex_color(&mut out, background);
                out.push(']');
            }
            if line.align == BrowserAlign::Center {
                out.push_str(" [align=center]");
            } else if line.align == BrowserAlign::Right {
                out.push_str(" [align=right]");
            }
            if let Some(link) = line.link {
                out.push_str(" -> ");
                out.push_str(&link);
            }
            out
        })
        .collect()
}

pub fn document_interaction_debug_for_test(base_url: &str, response: &str) -> Vec<String> {
    let mut document = BrowserDocumentState::from_html(base_url, response);
    let attr_count = document
        .dom
        .nodes
        .iter()
        .map(|node| match &node.kind {
            BrowserDomNodeKind::Element { attrs, .. } => attrs.len(),
            BrowserDomNodeKind::Text(_) => 0,
        })
        .fold(0usize, |total, count| total.saturating_add(count));
    let parent_links = document
        .dom
        .nodes
        .iter()
        .filter(|node| node.parent.is_some())
        .count();
    let mut out = vec![
        format!(
            "dom nodes={} root_children={} parents={} attrs={}",
            document.dom.nodes.len(),
            document
                .dom
                .nodes
                .get(document.dom.root)
                .map(|node| node.children.len())
                .unwrap_or(0),
            parent_links,
            attr_count
        ),
        format!(
            "dom has form={} input={} text={}",
            document.dom_has_element("form"),
            document.dom_has_element("input"),
            document.dom_text_contains("DOM backed form")
        ),
        format!(
            "forms={} controls={}",
            document.forms.len(),
            document.controls.len()
        ),
    ];
    let edited = document.set_control_value_for_test("q", "edited");
    let noted = document.set_control_value_for_test("notes", "phase 53 note");
    let toggled = document.toggle_control_for_test("safe");
    out.push(format!(
        "edited={} noted={} toggled={}",
        edited, noted, toggled
    ));
    out.push(
        document
            .submit_url_for_test("Go")
            .unwrap_or_else(|| String::from("GET missing")),
    );
    out.push(
        document
            .submit_url_for_test("Post")
            .unwrap_or_else(|| String::from("POST missing")),
    );
    out
}

fn push_hex_color(out: &mut String, color: u32) {
    out.push('#');
    for shift in [20u32, 16, 12, 8, 4, 0] {
        out.push(hex_digit(((color >> shift) & 0x0f) as u8));
    }
}

struct HtmlRenderState {
    link: Option<String>,
    kind: BrowserLineKind,
    pending_prefix: Option<String>,
    preformatted: bool,
    css_pre_stack: Vec<String>,
    suppress_text_stack: Vec<String>,
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
    form_fields: Vec<FormField>,
    cell_has_form_control: bool,
    cell_form_link: Option<String>,
    cell_link: Option<String>,
    cell_controls: Vec<(usize, BrowserControl)>,
    align_stack: Vec<(String, BrowserAlign)>,
    style_stack: Vec<(String, BrowserLineStyle)>,
    table_cell_align: BrowserAlign,
}

impl HtmlRenderState {
    fn new() -> Self {
        Self {
            link: None,
            kind: BrowserLineKind::Text,
            pending_prefix: None,
            preformatted: false,
            css_pre_stack: Vec::new(),
            suppress_text_stack: Vec::new(),
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
            form_fields: Vec::new(),
            cell_has_form_control: false,
            cell_form_link: None,
            cell_link: None,
            cell_controls: Vec::new(),
            align_stack: Vec::new(),
            style_stack: Vec::new(),
            table_cell_align: BrowserAlign::Left,
        }
    }

    fn current_align(&self) -> BrowserAlign {
        self.align_stack
            .last()
            .map(|(_, align)| *align)
            .unwrap_or(BrowserAlign::Left)
    }

    fn push_align(&mut self, name: &str, align: BrowserAlign) {
        self.align_stack.push((String::from(name), align));
    }

    fn pop_align(&mut self, name: &str) {
        if let Some(pos) = self
            .align_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.align_stack.truncate(pos);
        }
    }

    fn current_line_style(&self) -> BrowserLineStyle {
        let mut out = BrowserLineStyle::default();
        for (_, style) in &self.style_stack {
            out = out.merged(*style);
        }
        out
    }

    fn push_style(&mut self, name: &str, style: BrowserLineStyle) {
        if !style.is_default() {
            self.style_stack.push((String::from(name), style));
        }
    }

    fn pop_style(&mut self, name: &str) {
        if let Some(pos) = self
            .style_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.style_stack.truncate(pos);
        }
    }

    fn push_pre_style(&mut self, name: &str) {
        self.css_pre_stack.push(String::from(name));
    }

    fn pop_pre_style(&mut self, name: &str) {
        if let Some(pos) = self
            .css_pre_stack
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.css_pre_stack.truncate(pos);
        }
    }

    fn push_suppressed_text(&mut self, name: &str) {
        self.suppress_text_stack.push(String::from(name));
    }

    fn pop_suppressed_text(&mut self, name: &str) {
        if let Some(pos) = self
            .suppress_text_stack
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.suppress_text_stack.truncate(pos);
        }
    }

    fn suppresses_text(&self) -> bool {
        !self.suppress_text_stack.is_empty()
    }

    fn is_preformatted(&self) -> bool {
        self.preformatted || !self.css_pre_stack.is_empty()
    }
}

struct TableCell {
    text: String,
    header: bool,
    link: Option<String>,
    is_form_row: bool,
    align: BrowserAlign,
    controls: Vec<(usize, BrowserControl)>,
}

struct FormField {
    name: String,
    value: String,
}

impl BrowserDocumentState {
    fn from_html(base_url: &str, response: &str) -> Self {
        let body = response_body_text(response).unwrap_or(response);
        let effective_base = extract_base_href(body, base_url);
        let mut state = Self {
            base_url: effective_base,
            source: String::from(body),
            dom: BrowserDomDocument::new(),
            forms: Vec::new(),
            controls: Vec::new(),
            focused_control: None,
        };
        scan_dom_and_controls(body, &state.base_url.clone(), &mut state);
        state.finalize_select_values();
        state
    }

    fn dom_has_element(&self, wanted: &str) -> bool {
        self.dom.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                BrowserDomNodeKind::Element { name, attrs }
                    if name == wanted
                        || attrs
                            .iter()
                            .any(|attr| attr.name == "id" && attr.value == wanted)
            )
        })
    }

    fn dom_text_contains(&self, needle: &str) -> bool {
        self.dom.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                BrowserDomNodeKind::Text(text) if text.contains(needle)
            )
        })
    }

    fn focus_control(&mut self, id: usize) -> bool {
        if self.controls.get(id).map(|c| c.disabled).unwrap_or(true) {
            return false;
        }
        self.focused_control = Some(id);
        true
    }

    fn focus_next_control(&mut self) -> bool {
        if self.controls.is_empty() {
            self.focused_control = None;
            return false;
        }
        let start = self
            .focused_control
            .map(|id| id.saturating_add(1))
            .unwrap_or(0);
        for offset in 0..self.controls.len() {
            let idx = (start + offset) % self.controls.len();
            if self.controls[idx].is_focusable() {
                self.focused_control = Some(idx);
                return true;
            }
        }
        false
    }

    fn edit_focused_control(&mut self, c: char) -> bool {
        let Some(id) = self.focused_control else {
            return false;
        };
        let Some(control) = self.controls.get_mut(id) else {
            return false;
        };
        match control.kind {
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => match c {
                '\u{8}' | '\u{7f}' => {
                    control.value.pop();
                    true
                }
                '\n' | '\r' if control.kind == BrowserFormControlKind::TextArea => {
                    if control.value.len() < MAX_FORM_VALUE {
                        control.value.push('\n');
                    }
                    true
                }
                _ if !c.is_control() && control.value.len() < MAX_FORM_VALUE => {
                    control.value.push(c);
                    true
                }
                _ => false,
            },
            BrowserFormControlKind::Checkbox if c == ' ' || c == '\n' || c == '\r' => {
                control.checked = !control.checked;
                true
            }
            BrowserFormControlKind::Radio if c == ' ' || c == '\n' || c == '\r' => {
                self.set_radio_checked(id);
                true
            }
            BrowserFormControlKind::Select if c == ' ' || c == '\n' || c == '\r' => {
                self.select_next_option(id);
                true
            }
            _ => false,
        }
    }

    fn activate_control(&mut self, id: usize) -> BrowserControlActivation {
        if !self.focus_control(id) {
            return BrowserControlActivation::Ignored;
        }
        let Some(kind) = self.controls.get(id).map(|control| control.kind) else {
            return BrowserControlActivation::Ignored;
        };
        match kind {
            BrowserFormControlKind::Checkbox => {
                if let Some(control) = self.controls.get_mut(id) {
                    control.checked = !control.checked;
                }
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Radio => {
                self.set_radio_checked(id);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Select => {
                self.select_next_option(id);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Submit | BrowserFormControlKind::Image => self
                .submission_for(id)
                .unwrap_or(BrowserControlActivation::Ignored),
            BrowserFormControlKind::Button => {
                let label = self.controls[id].label.clone();
                BrowserControlActivation::DomEvent(label)
            }
            BrowserFormControlKind::Reset => {
                self.reset_form_for_control(id);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => {
                BrowserControlActivation::Focused
            }
            BrowserFormControlKind::Hidden => BrowserControlActivation::Ignored,
        }
    }

    fn set_control_value_for_test(&mut self, name: &str, value: &str) -> bool {
        let Some(control) = self
            .controls
            .iter_mut()
            .find(|control| control.name == name && control.kind.accepts_text())
        else {
            return false;
        };
        control.value.clear();
        for c in value.chars().take(MAX_FORM_VALUE) {
            control.value.push(c);
        }
        true
    }

    fn toggle_control_for_test(&mut self, name: &str) -> bool {
        let Some(id) = self.controls.iter().position(|control| {
            control.name == name && control.kind == BrowserFormControlKind::Checkbox
        }) else {
            return false;
        };
        matches!(self.activate_control(id), BrowserControlActivation::Changed)
    }

    fn submit_url_for_test(&self, label: &str) -> Option<String> {
        let id = self
            .controls
            .iter()
            .position(|control| control.label == label && control.kind.can_submit())?;
        match self.submission_for(id)? {
            BrowserControlActivation::Navigate(url) => Some(url),
            BrowserControlActivation::Post { url, body } => {
                let mut out = String::from("POST ");
                out.push_str(&url);
                out.push_str(" body=");
                out.push_str(&body);
                Some(out)
            }
            _ => None,
        }
    }

    fn default_submit_for(&self, control_id: usize) -> Option<usize> {
        let form_id = self.controls.get(control_id)?.form_id?;
        self.controls
            .iter()
            .enumerate()
            .find(|(_, control)| {
                control.form_id == Some(form_id) && control.kind.can_submit() && !control.disabled
            })
            .map(|(id, _)| id)
    }

    fn submission_for(&self, submit_id: usize) -> Option<BrowserControlActivation> {
        let submit = self.controls.get(submit_id)?;
        let form_id = submit.form_id?;
        let form = self.forms.get(form_id)?;
        let body = self.encoded_form_body(form_id, Some(submit_id));
        match form.method {
            BrowserFormMethod::Get => {
                let mut url = form.action.clone();
                if !body.is_empty() {
                    url.push(if url.contains('?') { '&' } else { '?' });
                    url.push_str(&body);
                }
                Some(BrowserControlActivation::Navigate(url))
            }
            BrowserFormMethod::Post => Some(BrowserControlActivation::Post {
                url: form.action.clone(),
                body,
            }),
        }
    }

    fn encoded_form_body(&self, form_id: usize, submit_id: Option<usize>) -> String {
        let mut out = String::new();
        let mut wrote = false;
        for (idx, control) in self.controls.iter().enumerate() {
            if control.form_id != Some(form_id) || !control.successful(Some(idx) == submit_id) {
                continue;
            }
            if wrote {
                out.push('&');
            }
            push_query_encoded(&mut out, &control.name);
            out.push('=');
            push_query_encoded(&mut out, &control.submit_value());
            wrote = true;
        }
        out
    }

    fn set_radio_checked(&mut self, id: usize) {
        let Some(target) = self.controls.get(id).cloned() else {
            return;
        };
        for control in &mut self.controls {
            if control.kind == BrowserFormControlKind::Radio
                && control.form_id == target.form_id
                && !target.name.is_empty()
                && control.name == target.name
            {
                control.checked = false;
            }
        }
        if let Some(control) = self.controls.get_mut(id) {
            control.checked = true;
        }
    }

    fn select_next_option(&mut self, id: usize) {
        let Some(control) = self.controls.get_mut(id) else {
            return;
        };
        if control.options.is_empty() {
            return;
        }
        control.selected = (control.selected + 1) % control.options.len();
        control.value = control.options[control.selected].value.clone();
    }

    fn reset_form_for_control(&mut self, id: usize) {
        let Some(form_id) = self.controls.get(id).and_then(|control| control.form_id) else {
            return;
        };
        for control in &mut self.controls {
            if control.form_id == Some(form_id) {
                control.reset_to_default();
            }
        }
    }

    fn finalize_select_values(&mut self) {
        for control in &mut self.controls {
            if control.kind == BrowserFormControlKind::Select && !control.options.is_empty() {
                control.selected = control.selected.min(control.options.len() - 1);
                control.value = control.options[control.selected].value.clone();
            }
        }
    }
}

impl BrowserDomDocument {
    fn new() -> Self {
        Self {
            nodes: vec![BrowserDomNode {
                parent: None,
                children: Vec::new(),
                kind: BrowserDomNodeKind::Element {
                    name: String::from("document"),
                    attrs: Vec::new(),
                },
            }],
            root: 0,
        }
    }

    fn push_element(&mut self, parent: usize, name: &str, attrs: Vec<BrowserDomAttr>) -> usize {
        self.push_node(
            parent,
            BrowserDomNodeKind::Element {
                name: String::from(name),
                attrs,
            },
        )
    }

    fn push_text(&mut self, parent: usize, text: String) {
        if clean_inline_text(&text).is_empty() {
            return;
        }
        self.push_node(parent, BrowserDomNodeKind::Text(text));
    }

    fn push_node(&mut self, parent: usize, kind: BrowserDomNodeKind) -> usize {
        if self.nodes.len() >= MAX_DOM_NODES {
            return parent;
        }
        let idx = self.nodes.len();
        self.nodes.push(BrowserDomNode {
            parent: Some(parent),
            children: Vec::new(),
            kind,
        });
        if let Some(parent) = self.nodes.get_mut(parent) {
            parent.children.push(idx);
        }
        idx
    }
}

impl BrowserFormControlKind {
    fn accepts_text(self) -> bool {
        matches!(self, Self::Text | Self::TextArea)
    }

    fn can_submit(self) -> bool {
        matches!(self, Self::Submit | Self::Image)
    }
}

impl BrowserFormControlState {
    fn is_focusable(&self) -> bool {
        !self.disabled
            && !matches!(
                self.kind,
                BrowserFormControlKind::Hidden | BrowserFormControlKind::Image
            )
    }

    fn successful(&self, is_submitter: bool) -> bool {
        if self.disabled || self.name.is_empty() {
            return false;
        }
        match self.kind {
            BrowserFormControlKind::Hidden
            | BrowserFormControlKind::Text
            | BrowserFormControlKind::TextArea
            | BrowserFormControlKind::Select => true,
            BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio => self.checked,
            BrowserFormControlKind::Submit | BrowserFormControlKind::Image => is_submitter,
            BrowserFormControlKind::Button | BrowserFormControlKind::Reset => false,
        }
    }

    fn submit_value(&self) -> String {
        if self.kind == BrowserFormControlKind::Checkbox
            || self.kind == BrowserFormControlKind::Radio
        {
            if self.value.is_empty() {
                String::from("on")
            } else {
                self.value.clone()
            }
        } else {
            self.value.clone()
        }
    }

    fn reset_to_default(&mut self) {
        match self.kind {
            BrowserFormControlKind::Text | BrowserFormControlKind::TextArea => {
                self.value = self.default_value.clone();
            }
            BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio => {
                self.checked = self.default_checked;
            }
            BrowserFormControlKind::Select => {
                self.selected = self
                    .default_selected
                    .min(self.options.len().saturating_sub(1));
                if let Some(option) = self.options.first() {
                    self.value = self
                        .options
                        .get(self.selected)
                        .unwrap_or(option)
                        .value
                        .clone();
                }
            }
            _ => {}
        }
    }
}

enum BrowserControlActivation {
    Ignored,
    Focused,
    Changed,
    Navigate(String),
    Post { url: String, body: String },
    DomEvent(String),
}

struct PendingOption {
    control_id: usize,
    value: Option<String>,
    label: String,
    selected: bool,
}

fn scan_dom_and_controls(body: &str, base_url: &str, document: &mut BrowserDocumentState) {
    let mut stack = vec![document.dom.root];
    let mut names = vec![String::from("document")];
    let mut form_stack: Vec<usize> = Vec::new();
    let mut text = String::new();
    let mut active_textarea: Option<usize> = None;
    let mut active_button: Option<(usize, String)> = None;
    let mut active_select: Option<usize> = None;
    let mut pending_option: Option<PendingOption> = None;
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
            flush_dom_text(document, &stack, &mut text);
            if let Some(end_rel) = find_tag_end(&body[i..]) {
                let tag = body[i + 1..i + end_rel].trim();
                let lower_tag = lowercase_ascii(tag);
                let name = tag_name_of(&lower_tag);
                let closing = lower_tag.starts_with('/');
                if closing {
                    match name {
                        "form" => {
                            form_stack.pop();
                        }
                        "textarea" => {
                            if let Some(control_id) = active_textarea {
                                finalize_textarea_value(document, control_id);
                            }
                            active_textarea = None;
                        }
                        "button" => {
                            finalize_button_text(document, &mut active_button);
                        }
                        "select" => {
                            active_select = None;
                        }
                        "option" => {
                            finish_pending_option(document, &mut pending_option);
                        }
                        _ => {}
                    }
                    pop_dom_stack(&mut stack, &mut names, name);
                    i += end_rel + 1;
                    continue;
                }

                let parent = *stack.last().unwrap_or(&document.dom.root);
                let attrs = parse_dom_attrs(tag);
                let node = document.dom.push_element(parent, name, attrs);
                let self_closing = lower_tag.ends_with('/') || is_void_element(name);
                if !self_closing {
                    stack.push(node);
                    names.push(String::from(name));
                }

                match name {
                    "form" => {
                        if document.forms.len() < MAX_FORM_CONTROLS {
                            let id = document.forms.len();
                            document.forms.push(BrowserFormState {
                                action: form_action_url_any(tag, base_url),
                                method: form_method_for_tag(tag),
                            });
                            form_stack.push(id);
                        }
                    }
                    "input" => {
                        push_document_input_control(document, tag, form_stack.last().copied());
                    }
                    "button" => {
                        active_button =
                            push_document_button_control(document, tag, form_stack.last().copied())
                                .map(|control_id| (control_id, String::new()));
                    }
                    "select" => {
                        active_select =
                            push_document_select_control(document, tag, form_stack.last().copied());
                    }
                    "textarea" => {
                        active_textarea = push_document_textarea_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                        );
                    }
                    "option" => {
                        finish_pending_option(document, &mut pending_option);
                        if let Some(control_id) = active_select {
                            pending_option = Some(PendingOption {
                                control_id,
                                value: attr_value(tag, "value")
                                    .map(|value| clean_inline_text(&decode_entities(&value))),
                                label: String::new(),
                                selected: has_attr(tag, "selected"),
                            });
                        }
                    }
                    _ => {}
                }
                i += end_rel + 1;
                continue;
            }
        }
        let c = bytes[i] as char;
        text.push(c);
        if let Some(control_id) = active_textarea {
            if let Some(control) = document.controls.get_mut(control_id) {
                if control.value.len() < MAX_FORM_VALUE {
                    control.value.push(c);
                    control.default_value.push(c);
                }
            }
        }
        if let Some((_, text)) = active_button.as_mut() {
            if text.len() < MAX_FORM_VALUE {
                text.push(c);
            }
        }
        if let Some(option) = pending_option.as_mut() {
            option.label.push(c);
        }
        i += 1;
    }
    flush_dom_text(document, &stack, &mut text);
    finalize_button_text(document, &mut active_button);
    finish_pending_option(document, &mut pending_option);
}

fn flush_dom_text(document: &mut BrowserDocumentState, stack: &[usize], text: &mut String) {
    if text.is_empty() {
        return;
    }
    let parent = *stack.last().unwrap_or(&document.dom.root);
    document.dom.push_text(parent, decode_entities(text));
    text.clear();
}

fn pop_dom_stack(stack: &mut Vec<usize>, names: &mut Vec<String>, name: &str) {
    if stack.len() <= 1 {
        return;
    }
    if let Some(pos) = names.iter().rposition(|entry| entry == name) {
        let keep = pos.max(1);
        stack.truncate(keep);
        names.truncate(keep);
    }
}

fn parse_dom_attrs(tag: &str) -> Vec<BrowserDomAttr> {
    let mut out = Vec::new();
    let bytes = tag.as_bytes();
    let mut pos = tag_name_of(&lowercase_ascii(tag)).len();
    while pos < bytes.len() && out.len() < MAX_DOM_ATTRS {
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
        let name = lowercase_ascii(&tag[start..pos]);
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let mut value = String::new();
        if bytes.get(pos) == Some(&b'=') {
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
                let quote = bytes[pos];
                pos += 1;
                let value_start = pos;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                value = decode_entities(&tag[value_start..pos]);
                pos = pos.saturating_add(1);
            } else {
                let value_start = pos;
                while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                    pos += 1;
                }
                value = decode_entities(&tag[value_start..pos]);
            }
        }
        out.push(BrowserDomAttr { name, value });
    }
    out
}

fn form_method_for_tag(tag: &str) -> BrowserFormMethod {
    attr_value(tag, "method")
        .map(|method| {
            if method.eq_ignore_ascii_case("post") {
                BrowserFormMethod::Post
            } else {
                BrowserFormMethod::Get
            }
        })
        .unwrap_or(BrowserFormMethod::Get)
}

fn form_action_url_any(tag: &str, base_url: &str) -> String {
    let action = attr_value(tag, "action").unwrap_or_else(|| String::from(base_url));
    if action.trim().is_empty() {
        String::from(base_url)
    } else {
        resolve_url(base_url, &action)
    }
}

fn push_document_input_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
) -> Option<usize> {
    let input_type = lowercase_ascii(
        attr_value(tag, "type")
            .unwrap_or_else(|| String::from("text"))
            .trim(),
    );
    let kind = match input_type.as_str() {
        "hidden" => BrowserFormControlKind::Hidden,
        "checkbox" => BrowserFormControlKind::Checkbox,
        "radio" => BrowserFormControlKind::Radio,
        "submit" => BrowserFormControlKind::Submit,
        "button" => BrowserFormControlKind::Button,
        "reset" => BrowserFormControlKind::Reset,
        "image" => BrowserFormControlKind::Image,
        _ => BrowserFormControlKind::Text,
    };
    let label = if kind == BrowserFormControlKind::Text {
        input_field_label(tag, &input_type)
    } else {
        form_control_label(tag, &input_type)
    };
    let value = input_value(tag);
    let checked = has_attr(tag, "checked");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            kind,
            name: control_name(tag),
            label,
            value: value.clone(),
            default_value: value,
            checked,
            default_checked: checked,
            disabled: has_attr(tag, "disabled"),
            chars: input_size_chars(tag),
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_button_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
) -> Option<usize> {
    let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
    let kind = if button_type.eq_ignore_ascii_case("button") {
        BrowserFormControlKind::Button
    } else if button_type.eq_ignore_ascii_case("reset") {
        BrowserFormControlKind::Reset
    } else {
        BrowserFormControlKind::Submit
    };
    let label = form_control_label(tag, "button");
    let value = attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(|| label.clone());
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            kind,
            name: control_name(tag),
            label,
            value: value.clone(),
            default_value: value,
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: 12,
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_select_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "select");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            kind: BrowserFormControlKind::Select,
            name: control_name(tag),
            label,
            value: String::new(),
            default_value: String::new(),
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: 20,
            rows: 1,
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_textarea_control(
    document: &mut BrowserDocumentState,
    tag: &str,
    form_id: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "textarea");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            kind: BrowserFormControlKind::TextArea,
            name: control_name(tag),
            label,
            value: String::new(),
            default_value: String::new(),
            checked: false,
            default_checked: false,
            disabled: has_attr(tag, "disabled"),
            chars: input_size_chars(tag),
            rows: attr_value(tag, "rows")
                .and_then(|value| parse_dimension(&value))
                .unwrap_or(3)
                .clamp(2, 8),
            options: Vec::new(),
            selected: 0,
            default_selected: 0,
        },
    )
}

fn push_document_control(
    document: &mut BrowserDocumentState,
    control: BrowserFormControlState,
) -> Option<usize> {
    if document.controls.len() >= MAX_FORM_CONTROLS {
        return None;
    }
    let id = document.controls.len();
    document.controls.push(control);
    Some(id)
}

fn finish_pending_option(document: &mut BrowserDocumentState, pending: &mut Option<PendingOption>) {
    let Some(option) = pending.take() else {
        return;
    };
    let Some(control) = document.controls.get_mut(option.control_id) else {
        return;
    };
    if control.options.len() >= MAX_FORM_OPTIONS {
        return;
    }
    let label = clean_inline_text(&decode_entities(&option.label));
    let label = if label.is_empty() {
        option
            .value
            .clone()
            .unwrap_or_else(|| String::from("option"))
    } else {
        label
    };
    let value = option.value.unwrap_or_else(|| label.clone());
    if option.selected {
        control.selected = control.options.len();
        control.default_selected = control.selected;
    }
    control.options.push(BrowserSelectOption { label, value });
}

fn finalize_textarea_value(document: &mut BrowserDocumentState, control_id: usize) {
    let Some(control) = document.controls.get_mut(control_id) else {
        return;
    };
    if control.kind != BrowserFormControlKind::TextArea {
        return;
    }
    control.value = decode_entities(&control.value);
    control.default_value = control.value.clone();
    if control.value.len() > MAX_FORM_VALUE {
        let mut trimmed = String::new();
        for c in control.value.chars() {
            if trimmed.len().saturating_add(c.len_utf8()) > MAX_FORM_VALUE {
                break;
            }
            trimmed.push(c);
        }
        control.value = trimmed;
        control.default_value = control.value.clone();
    }
}

fn finalize_button_text(
    document: &mut BrowserDocumentState,
    active_button: &mut Option<(usize, String)>,
) {
    let Some((control_id, text)) = active_button.take() else {
        return;
    };
    let label = clean_inline_text(&decode_entities(&text));
    if label.is_empty() {
        return;
    }
    let Some(control) = document.controls.get_mut(control_id) else {
        return;
    };
    if control.label == "button" {
        control.label = label.clone();
    }
    if control.default_value == "button" {
        control.value = label.clone();
        control.default_value = label;
    }
}

fn control_name(tag: &str) -> String {
    attr_value(tag, "name")
        .map(|name| clean_inline_text(&decode_entities(&name)))
        .unwrap_or_else(String::new)
}

fn is_inline_tag(name: &str) -> bool {
    matches!(
        name,
        "span"
            | "strong"
            | "em"
            | "b"
            | "i"
            | "small"
            | "big"
            | "sub"
            | "sup"
            | "s"
            | "u"
            | "del"
            | "ins"
            | "mark"
            | "cite"
            | "abbr"
            | "time"
            | "var"
            | "samp"
            | "kbd"
            | "wbr"
            | "bdi"
            | "bdo"
            | "data"
            | "q"
            | "dfn"
            | "label"
            | "output"
            | "meter"
            | "progress"
            | "nobr"
            | "font"
            | "tt"
            | "acronym"
            | "strike"
            | "blink"
            | "marquee"
    )
}

fn tag_alignment(tag: &str, name: &str) -> Option<BrowserAlign> {
    if name == "center" {
        return Some(BrowserAlign::Center);
    }
    if let Some(value) = attr_value(tag, "align") {
        return parse_alignment(&value);
    }
    if let Some(style) = attr_value(tag, "style") {
        let style = lowercase_ascii(&style);
        if style.contains("text-align:center") || style.contains("text-align: center") {
            return Some(BrowserAlign::Center);
        }
        if style.contains("text-align:right") || style.contains("text-align: right") {
            return Some(BrowserAlign::Right);
        }
        if style.contains("text-align:left") || style.contains("text-align: left") {
            return Some(BrowserAlign::Left);
        }
        if style.contains("margin:") && style.contains("auto") {
            return Some(BrowserAlign::Center);
        }
    }
    None
}

fn parse_alignment(value: &str) -> Option<BrowserAlign> {
    let value = lowercase_ascii(value.trim());
    match value.as_str() {
        "center" | "middle" => Some(BrowserAlign::Center),
        "right" => Some(BrowserAlign::Right),
        "left" => Some(BrowserAlign::Left),
        _ => None,
    }
}

fn handle_tag(
    tag: &str,
    lower_tag: &str,
    styles: &StyleHints,
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
    base_url: &str,
    cols: usize,
) {
    let tag = tag.trim();
    let name = tag_name_of(lower_tag);
    let closing = lower_tag.starts_with('/');
    let tag_style = styles.computed_for_tag(lower_tag, name);
    let mut controls = controls;
    if closing {
        state.pop_suppressed_text(name);
    }

    if state.in_table_cell {
        handle_table_cell_tag(
            tag,
            out,
            state,
            controls.as_deref_mut(),
            base_url,
            cols,
            name,
            closing,
        );
        return;
    }

    if is_inline_tag(name) {
        let has_inline_effect = tag_style.align.is_some()
            || !tag_style.line.is_default()
            || tag_style.preformatted
            || tag_style.display == CssDisplay::Block;
        if !has_inline_effect {
            return;
        }
        flush_flow_text(out, text, cols, state);
        if closing {
            state.pop_align(name);
            state.pop_style(name);
            state.pop_pre_style(name);
        } else if !is_void_element(name) {
            if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
                state.push_align(name, align);
            }
            state.push_style(name, tag_style.line);
            if tag_style.preformatted {
                state.push_pre_style(name);
            }
        }
        return;
    }

    flush_flow_text(out, text, cols, state);
    if closing {
        state.pop_align(name);
        state.pop_style(name);
        state.pop_pre_style(name);
    } else if !is_void_element(name) {
        if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
            state.push_align(name, align);
        }
    }
    if !closing && !is_void_element(name) {
        state.push_style(name, tag_style.line);
        if tag_style.preformatted {
            state.push_pre_style(name);
        }
    }

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
                state.form_fields.clear();
                push_blank_line(out);
            } else {
                state.form_action = form_action_url(tag, base_url);
                state.form_fields.clear();
                push_blank_line(out);
            }
        }
        "input" => {
            if !closing {
                push_input_line(out, tag, state, controls.as_deref_mut());
            }
        }
        "button" => {
            if !closing {
                let interactive = controls.is_some();
                push_button_line(out, tag, state, controls.as_deref_mut());
                if interactive {
                    state.push_suppressed_text(name);
                }
            }
        }
        "select" => {
            if !closing {
                push_named_control_line(out, "select", tag, state, controls.as_deref_mut());
                state.push_suppressed_text(name);
            }
        }
        "textarea" => {
            if !closing {
                push_named_control_line(out, "textarea", tag, state, controls.as_deref_mut());
                state.push_suppressed_text(name);
            }
        }
        "img" => push_image_line(out, tag, base_url, state, tag_style),
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
            state.table_cell_align =
                tag_alignment(tag, name).unwrap_or_else(|| state.current_align());
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

fn record_input_field(state: &mut HtmlRenderState, tag: &str, input_type: &str) {
    if state.form_action.is_none() || has_attr(tag, "disabled") || state.form_fields.len() >= 64 {
        return;
    }
    if matches!(input_type, "submit" | "button" | "reset" | "image" | "file") {
        return;
    }
    if matches!(input_type, "checkbox" | "radio") && !has_attr(tag, "checked") {
        return;
    }
    let Some(name) = attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
    else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let value = if matches!(input_type, "checkbox" | "radio") {
        attr_value(tag, "value")
            .map(|value| clean_inline_text(&decode_entities(&value)))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| String::from("on"))
    } else {
        input_value(tag)
    };
    state.form_fields.push(FormField { name, value });
}

fn record_named_form_field(state: &mut HtmlRenderState, tag: &str) {
    if state.form_action.is_none() || has_attr(tag, "disabled") || state.form_fields.len() >= 64 {
        return;
    }
    let Some(name) = attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
    else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let value = attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(String::new);
    state.form_fields.push(FormField { name, value });
}

fn form_submit_url(state: &HtmlRenderState, submit_tag: Option<&str>) -> Option<String> {
    let mut out = state.form_action.clone()?;
    let mut wrote = out.contains('?');
    for field in &state.form_fields {
        append_query_param(&mut out, &mut wrote, &field.name, &field.value);
    }
    if let Some(tag) = submit_tag {
        if !has_attr(tag, "disabled") {
            if let Some(name) =
                attr_value(tag, "name").map(|name| clean_inline_text(&decode_entities(&name)))
            {
                if !name.is_empty() {
                    let value = attr_value(tag, "value")
                        .map(|value| clean_inline_text(&decode_entities(&value)))
                        .unwrap_or_else(String::new);
                    append_query_param(&mut out, &mut wrote, &name, &value);
                }
            }
        }
    }
    Some(out)
}

fn append_query_param(out: &mut String, wrote: &mut bool, name: &str, value: &str) {
    out.push(if *wrote { '&' } else { '?' });
    push_query_encoded(out, name);
    out.push('=');
    push_query_encoded(out, value);
    *wrote = true;
}

fn push_input_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
) {
    let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
    let input_type = lowercase_ascii(input_type.trim());
    let interactive = controls.and_then(|controls| controls.next());
    if input_type == "hidden" {
        if interactive.is_none() {
            record_input_field(state, tag, &input_type);
        }
        return;
    }
    let mut text = String::new();
    let mut link = None;
    let control;
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    match input_type.as_str() {
        "submit" | "button" | "reset" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, &input_type));
            text.push_str("[button] ");
            text.push_str(&label);
            if live.is_none() {
                link = if input_type == "submit" {
                    form_submit_url(state, Some(tag))
                } else {
                    Some(browser_event_url(&label))
                };
            }
            control = BrowserControl::Button { label };
        }
        "checkbox" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[checkbox] ");
            text.push_str(&label);
            let checked = live
                .map(|control| control.checked)
                .unwrap_or_else(|| has_attr(tag, "checked"));
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::Checkbox { label, checked };
        }
        "radio" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[radio] ");
            text.push_str(&label);
            let checked = live
                .map(|control| control.checked)
                .unwrap_or_else(|| has_attr(tag, "checked"));
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::Radio { label, checked };
        }
        "search" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[search] ");
            text.push_str(&label);
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::TextInput {
                label: live
                    .map(|control| control.label.clone())
                    .unwrap_or_else(|| input_control_label(tag, &label)),
                value: live
                    .map(|control| control.value.clone())
                    .unwrap_or_else(|| input_value(tag)),
                chars: live
                    .map(|control| control.chars)
                    .unwrap_or_else(|| input_size_chars(tag)),
            };
        }
        "image" => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, &input_type));
            text.push_str("[image button] ");
            text.push_str(&label);
            link = if live.is_none() {
                form_submit_url(state, Some(tag))
            } else {
                None
            };
            control = BrowserControl::Button { label };
        }
        _ => {
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| input_field_label(tag, &input_type));
            text.push_str("[input] ");
            text.push_str(&label);
            if live.is_none() {
                record_input_field(state, tag, &input_type);
            }
            control = BrowserControl::TextInput {
                label: live
                    .map(|control| control.label.clone())
                    .unwrap_or_else(|| input_control_label(tag, &label)),
                value: live
                    .map(|control| control.value.clone())
                    .unwrap_or_else(|| input_value(tag)),
                chars: live
                    .map(|control| control.chars)
                    .unwrap_or_else(|| input_size_chars(tag)),
            };
        }
    }
    out.push(
        BrowserLine::new(
            text,
            link.clone(),
            line_kind_for_link(&link, BrowserLineKind::Code),
        )
        .aligned(state.current_align())
        .styled(state.current_line_style())
        .with_control(control)
        .with_control_id(control_id),
    );
}

fn push_button_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    state: &HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
) {
    let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
    let interactive = controls.and_then(|controls| controls.next());
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    let label = live
        .map(|control| control.label.clone())
        .unwrap_or_else(|| form_control_label(tag, "button"));
    let link = if live.is_some() {
        None
    } else if button_type.eq_ignore_ascii_case("submit") {
        form_submit_url(state, Some(tag))
    } else {
        Some(browser_event_url(&label))
    };
    let has_form_value = attr_value(tag, "value").is_some()
        || attr_value(tag, "name").is_some()
        || attr_value(tag, "id").is_some();
    if link.is_none() && control_id.is_none() && !has_form_value {
        return;
    }
    out.push(
        BrowserLine::new(
            format!("[button] {}", label),
            link.clone(),
            line_kind_for_link(&link, BrowserLineKind::Code),
        )
        .aligned(state.current_align())
        .styled(state.current_line_style())
        .with_control(BrowserControl::Button { label })
        .with_control_id(control_id),
    );
}

fn push_named_control_line(
    out: &mut Vec<BrowserLine>,
    control: &str,
    tag: &str,
    state: &mut HtmlRenderState,
    controls: Option<&mut BrowserRenderControls<'_>>,
) {
    let interactive = controls.and_then(|controls| controls.next());
    let control_id = interactive.map(|(id, _)| id);
    let live = interactive.map(|(_, control)| control);
    let label = live
        .map(|control| control.label.clone())
        .unwrap_or_else(|| form_control_label(tag, control));
    let visual = if control == "textarea" {
        BrowserControl::TextArea {
            label: label.clone(),
            value: live
                .map(|control| control.value.clone())
                .unwrap_or_else(String::new),
            rows: live.map(|control| control.rows).unwrap_or_else(|| {
                attr_value(tag, "rows")
                    .and_then(|value| parse_dimension(&value))
                    .unwrap_or(3)
                    .clamp(2, 8)
            }),
        }
    } else {
        BrowserControl::Select {
            label: label.clone(),
            value: live.map(select_display_value).unwrap_or_else(String::new),
            options: live.map(|control| control.options.len()).unwrap_or(0),
        }
    };
    if live.is_none() {
        record_named_form_field(state, tag);
    }
    out.push(
        BrowserLine::new(
            format!("[{}] {}", control, label),
            None,
            BrowserLineKind::Code,
        )
        .aligned(state.current_align())
        .styled(state.current_line_style())
        .with_control(visual)
        .with_control_id(control_id),
    );
}

fn select_display_value(control: &BrowserFormControlState) -> String {
    control
        .options
        .get(control.selected)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| control.value.clone())
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

// For text/search/email fields, prefer name/placeholder over aria-label
// so the field shows its identity, not the descriptive label shared with a button.
fn input_field_label(tag: &str, fallback: &str) -> String {
    for attr in ["placeholder", "name", "id", "aria-label", "title"] {
        if let Some(value) = attr_value(tag, attr) {
            let decoded = clean_inline_text(&decode_entities(&value));
            if !decoded.is_empty() {
                return decoded;
            }
        }
    }
    String::from(fallback)
}

fn input_control_label(tag: &str, _fallback: &str) -> String {
    attr_value(tag, "placeholder")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(String::new)
}

fn input_value(tag: &str) -> String {
    attr_value(tag, "value")
        .map(|value| clean_inline_text(&decode_entities(&value)))
        .unwrap_or_else(String::new)
}

fn input_size_chars(tag: &str) -> usize {
    attr_value(tag, "size")
        .and_then(|value| parse_dimension(&value))
        .unwrap_or(28)
        .clamp(8, 72)
}

fn handle_table_cell_tag(
    tag: &str,
    out: &mut Vec<BrowserLine>,
    state: &mut HtmlRenderState,
    mut controls: Option<&mut BrowserRenderControls<'_>>,
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
        "a" => {
            if !closing {
                if let Some(href) = attr_value(tag, "href") {
                    state.cell_link = Some(resolve_url(base_url, &decode_entities(&href)));
                }
            }
        }
        "input" if !closing => {
            let input_type = attr_value(tag, "type").unwrap_or_else(|| String::from("text"));
            let input_type = lowercase_ascii(input_type.trim());
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            if input_type == "hidden" {
                if live.is_none() {
                    record_input_field(state, tag, &input_type);
                }
                return;
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            match input_type.as_str() {
                "submit" | "button" | "reset" | "image" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| form_control_label(tag, &input_type));
                    if live.is_none() && state.cell_form_link.is_none() {
                        state.cell_form_link = if input_type == "submit" || input_type == "image" {
                            form_submit_url(state, Some(tag))
                        } else {
                            Some(browser_event_url(&label))
                        };
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Button {
                                label: label.clone(),
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[btn:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                "checkbox" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let checked = live
                        .map(|control| control.checked)
                        .unwrap_or_else(|| has_attr(tag, "checked"));
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Checkbox {
                                label: label.clone(),
                                checked,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[checkbox:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                "radio" => {
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let checked = live
                        .map(|control| control.checked)
                        .unwrap_or_else(|| has_attr(tag, "checked"));
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::Radio {
                                label: label.clone(),
                                checked,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[radio:");
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
                _ => {
                    if live.is_none() {
                        record_input_field(state, tag, &input_type);
                    }
                    let label = live
                        .map(|control| control.label.clone())
                        .unwrap_or_else(|| input_field_label(tag, &input_type));
                    let chars = live
                        .map(|control| control.chars)
                        .unwrap_or_else(|| input_size_chars(tag));
                    if let Some(id) = control_id {
                        state.cell_controls.push((
                            id,
                            BrowserControl::TextInput {
                                label: live
                                    .map(|control| control.label.clone())
                                    .unwrap_or_else(|| input_control_label(tag, &label)),
                                value: live
                                    .map(|control| control.value.clone())
                                    .unwrap_or_else(|| input_value(tag)),
                                chars,
                            },
                        ));
                    }
                    state.table_cell_text.push_str("[field:");
                    state.table_cell_text.push_str(&format!("{}", chars));
                    state.table_cell_text.push(':');
                    state.table_cell_text.push_str(&label);
                    state.table_cell_text.push(']');
                }
            }
        }
        "button" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "button"));
            if live.is_none() && state.cell_form_link.is_none() {
                let button_type = attr_value(tag, "type").unwrap_or_else(|| String::from("submit"));
                state.cell_form_link = if button_type.eq_ignore_ascii_case("submit") {
                    form_submit_url(state, Some(tag))
                } else {
                    Some(browser_event_url(&label))
                };
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::Button {
                        label: label.clone(),
                    },
                ));
            }
            state.table_cell_text.push_str("[btn:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "select" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "select"));
            if live.is_none() {
                record_named_form_field(state, tag);
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::Select {
                        label: label.clone(),
                        value: live.map(select_display_value).unwrap_or_else(String::new),
                        options: live.map(|control| control.options.len()).unwrap_or(0),
                    },
                ));
            }
            state.table_cell_text.push_str("[select:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        "textarea" if !closing => {
            let interactive = controls.as_deref_mut().and_then(|controls| controls.next());
            let control_id = interactive.map(|(id, _)| id);
            let live = interactive.map(|(_, control)| control);
            let label = live
                .map(|control| control.label.clone())
                .unwrap_or_else(|| form_control_label(tag, "textarea"));
            if live.is_none() {
                record_named_form_field(state, tag);
            }
            state.cell_has_form_control = true;
            if !state.table_cell_text.is_empty() && !state.table_cell_text.ends_with(' ') {
                state.table_cell_text.push(' ');
            }
            if let Some(id) = control_id {
                state.cell_controls.push((
                    id,
                    BrowserControl::TextArea {
                        label: label.clone(),
                        value: live
                            .map(|control| control.value.clone())
                            .unwrap_or_else(String::new),
                        rows: live.map(|control| control.rows).unwrap_or(3),
                    },
                ));
            }
            state.table_cell_text.push_str("[textarea:");
            state.table_cell_text.push_str(&label);
            state.table_cell_text.push(']');
        }
        _ => {}
    }
}

fn push_image_line(
    out: &mut Vec<BrowserLine>,
    tag: &str,
    base_url: &str,
    state: &HtmlRenderState,
    tag_style: TagStyle,
) {
    let src = attr_value(tag, "src").map(|src| resolve_url(base_url, &src));
    let label = attr_value(tag, "alt")
        .map(|alt| decode_entities(&alt))
        .filter(|alt| !alt.trim().is_empty())
        .unwrap_or_else(|| src.clone().unwrap_or_else(|| String::from("image")));
    let hint = image_hint_for_tag(tag, tag_style);
    let mut text = String::from("[image");
    if let Some(size) = image_size_label_for_hint(hint) {
        text.push(' ');
        text.push_str(&size);
    }
    text.push_str("] ");
    text.push_str(&label);
    out.push(
        BrowserLine::new(text, src, BrowserLineKind::Image)
            .aligned(state.current_align())
            .styled(state.current_line_style().merged(tag_style.line))
            .with_image_hint(hint),
    );
}

fn image_size_label(tag: &str) -> Option<String> {
    image_size_label_for_hint(image_hint_for_tag(tag, TagStyle::default()))
}

fn image_hint_for_tag(tag: &str, tag_style: TagStyle) -> ImageHint {
    ImageHint {
        width: tag_style
            .width
            .or_else(|| attr_value(tag, "width").and_then(|value| parse_dimension(&value))),
        height: tag_style
            .height
            .or_else(|| attr_value(tag, "height").and_then(|value| parse_dimension(&value))),
    }
}

fn image_size_label_for_hint(hint: ImageHint) -> Option<String> {
    match (hint.width, hint.height) {
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

fn push_form_cell_lines(out: &mut Vec<BrowserLine>, cell: TableCell) {
    let align = cell.align;
    let link = cell.link;
    let text = cell.text;
    let mut controls = cell.controls.into_iter();
    let mut i = 0usize;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'[' {
            if let Some(end) = text[i..].find(']') {
                let part = &text[i..i + end + 1];
                if let Some(label) = part.strip_prefix("[btn:").and_then(|s| s.strip_suffix(']')) {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Button {
                            label: String::from(label),
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[button] {}", label),
                            link.clone(),
                            line_kind_for_link(&link, BrowserLineKind::Code),
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[checkbox:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Checkbox {
                            label: String::from(label),
                            checked: false,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[checkbox] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[radio:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Radio {
                            label: String::from(label),
                            checked: false,
                        }
                    });
                    out.push(
                        BrowserLine::new(format!("[radio] {}", label), None, BrowserLineKind::Code)
                            .aligned(align)
                            .with_control(visual)
                            .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[select:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::Select {
                            label: String::from(label),
                            value: String::new(),
                            options: 0,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[select] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(label) = part
                    .strip_prefix("[textarea:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::TextArea {
                            label: String::from(label),
                            value: String::new(),
                            rows: 3,
                        }
                    });
                    out.push(
                        BrowserLine::new(
                            format!("[textarea] {}", label),
                            None,
                            BrowserLineKind::Code,
                        )
                        .aligned(align)
                        .with_control(visual)
                        .with_control_id(control_id),
                    );
                } else if let Some(rest) = part
                    .strip_prefix("[field:")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let (chars, label) = rest
                        .split_once(':')
                        .map(|(chars, label)| {
                            (parse_dimension(chars).unwrap_or(28).clamp(8, 72), label)
                        })
                        .unwrap_or((28, rest));
                    let interactive = controls.next();
                    let control_id = interactive.as_ref().map(|(id, _)| *id);
                    let visual = interactive.map(|(_, control)| control).unwrap_or_else(|| {
                        BrowserControl::TextInput {
                            label: String::from(label),
                            value: String::new(),
                            chars,
                        }
                    });
                    out.push(
                        BrowserLine::new(format!("[input] {}", label), None, BrowserLineKind::Code)
                            .aligned(align)
                            .with_control(visual)
                            .with_control_id(control_id),
                    );
                } else {
                    out.push(
                        BrowserLine::new(String::from(part), None, BrowserLineKind::Code)
                            .aligned(align),
                    );
                }
                i += end + 1;
            } else {
                break;
            }
        } else {
            let end = text[i..].find('[').map(|p| i + p).unwrap_or(text.len());
            let plain = text[i..end].trim();
            if !plain.is_empty() {
                out.push(
                    BrowserLine::new(String::from(plain), None, BrowserLineKind::Text)
                        .aligned(align),
                );
            }
            i = end;
        }
    }
}

fn finish_table_cell(state: &mut HtmlRenderState) {
    if !state.in_table_cell {
        return;
    }
    let text = clean_inline_text(&decode_entities(&state.table_cell_text));
    let link = if state.cell_has_form_control {
        state.cell_form_link.take()
    } else {
        state.cell_link.take()
    };
    state.table_row.push(TableCell {
        text,
        header: state.table_cell_is_header,
        link,
        is_form_row: state.cell_has_form_control,
        align: state.table_cell_align,
        controls: core::mem::take(&mut state.cell_controls),
    });
    state.table_cell_text.clear();
    state.table_cell_is_header = false;
    state.in_table_cell = false;
    state.cell_has_form_control = false;
    state.cell_form_link = None;
    state.cell_link = None;
    state.cell_controls.clear();
    state.pop_align("td");
    state.pop_align("th");
    state.table_cell_align = state.current_align();
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
            if cell.is_form_row {
                push_form_cell_lines(out, cell);
            } else {
                let kind = if cell.link.is_some() {
                    BrowserLineKind::Link
                } else {
                    BrowserLineKind::Text
                };
                out.push(BrowserLine::new(cell.text, cell.link, kind).aligned(cell.align));
            }
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
        out.push(line(""));
    }
}

fn tag_is_hidden(lower_tag: &str) -> bool {
    let name = tag_name_of(lower_tag);
    let attrs = lower_tag[name.len()..].trim();
    // "hidden" as a standalone boolean attribute, not aria-hidden or data-hidden
    let has_hidden_attr = attrs
        .split(|c: char| c.is_ascii_whitespace())
        .any(|token| token == "hidden" || token.starts_with("hidden="));
    let has_hidden_class = attr_value(lower_tag, "class")
        .map(|classes| {
            classes.split_whitespace().any(|class| {
                matches!(
                    class,
                    "hidden" | "visually-hidden" | "sr-only" | "screen-reader-text"
                )
            })
        })
        .unwrap_or(false);
    has_hidden_attr
        || has_hidden_class
        || lower_tag.contains("display:none")
        || lower_tag.contains("display: none")
        || lower_tag.contains("visibility:hidden")
        || lower_tag.contains("visibility: hidden")
}

fn tag_name_of(lower_tag: &str) -> &str {
    lower_tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
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
    flush_text(
        out,
        text,
        cols,
        state.link.clone(),
        kind,
        prefix,
        state.current_align(),
        state.current_line_style(),
    );
}

fn flush_text(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
    prefix: Option<String>,
    align: BrowserAlign,
    style: BrowserLineStyle,
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
    out.extend(wrap_plain_text_kind(
        &decoded, cols, link, kind, align, style,
    ));
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
    wrap_plain_text_kind(
        text,
        cols,
        link,
        BrowserLineKind::Text,
        BrowserAlign::Left,
        BrowserLineStyle::default(),
    )
}

fn wrap_plain_text_kind(
    text: &str,
    cols: usize,
    link: Option<String>,
    kind: BrowserLineKind,
    align: BrowserAlign,
    style: BrowserLineStyle,
) -> Vec<BrowserLine> {
    let cols = cols.clamp(20, 120);
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if word.len() > cols {
            if !line.is_empty() {
                out.push(
                    BrowserLine::new(line, link.clone(), line_kind_for_link(&link, kind))
                        .aligned(align)
                        .styled(style),
                );
                line = String::new();
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.len() >= cols {
                    out.push(
                        BrowserLine::new(chunk, link.clone(), line_kind_for_link(&link, kind))
                            .aligned(align)
                            .styled(style),
                    );
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
            out.push(
                BrowserLine::new(line, link.clone(), line_kind_for_link(&link, kind))
                    .aligned(align)
                    .styled(style),
            );
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        let kind = line_kind_for_link(&link, kind);
        out.push(
            BrowserLine::new(line, link, kind)
                .aligned(align)
                .styled(style),
        );
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

fn is_separator_only(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    t.chars()
        .all(|c| matches!(c, '-' | '|' | '·' | '•' | '/' | '\\' | '_'))
}

fn compact_lines(lines: Vec<BrowserLine>) -> Vec<BrowserLine> {
    let mut out = Vec::new();
    let mut last_blank = false;
    for line in lines {
        let blank = line.text.trim().is_empty();
        if blank && last_blank {
            continue;
        }
        // Drop separator-only lines (e.g. lone "-" between footer links) with no link
        if line.link.is_none() && is_separator_only(&line.text) {
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

fn has_attr(tag: &str, name: &str) -> bool {
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
        if ascii_bytes_eq_ignore_case(&bytes[start..pos], name_bytes) {
            return true;
        }
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'=') {
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if matches!(bytes.get(pos), Some(b'"' | b'\'')) {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                pos = pos.saturating_add(1);
            } else {
                while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' {
                    pos += 1;
                }
            }
        }
    }
    false
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
