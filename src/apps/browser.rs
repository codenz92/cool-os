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
const CACHE_INTERNAL_URL: &str = "browser://cache";
const JS_INTERNAL_URL: &str = "browser://js";
const STORAGE_INTERNAL_URL: &str = "browser://storage";
const COMPAT_INTERNAL_URL: &str = "browser://compat";
const MAX_BOOKMARKS: usize = 32;
const MAX_INLINE_PNG_PIXELS: usize = 1_048_576;
const MAX_HTML_INLINE_IMAGES: usize = 4;
const MAX_STYLESHEET_SUBRESOURCES: usize = 8;
const MAX_SCRIPT_SUBRESOURCES: usize = 8;
const MAX_SCRIPT_BYTES: usize = 64 * 1024;
const MAX_SCRIPT_STATEMENTS: usize = 192;
const MAX_SCRIPT_EVENT_HANDLERS: usize = 64;
const MAX_SCRIPT_RECURSION: usize = 4;
const MAX_SCRIPT_VARS: usize = 8;
const MAX_SCRIPT_FETCH_BYTES: usize = 64 * 1024;
const MAX_SESSION_STORAGE_ENTRIES: usize = 32;
const MAX_COMPAT_NOTES: usize = 8;
const MAX_BROWSER_CACHE_ENTRIES: usize = 16;
const MAX_BROWSER_RESOURCE_BYTES: usize = 512 * 1024;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CssLength {
    Px(usize),
    Percent(u8),
}

impl CssLength {
    fn resolve_px(self, container_px: usize) -> usize {
        match self {
            Self::Px(px) => px,
            Self::Percent(percent) => container_px.saturating_mul(percent as usize) / 100,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserLineBoxPart {
    Single,
    First,
    Middle,
    Last,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CssPosition {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CssFloat {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CssListStyle {
    #[default]
    Disc,
    Circle,
    Square,
    Decimal,
    None,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct BrowserBoxStyle {
    margin_top: usize,
    margin_right: usize,
    margin_bottom: usize,
    margin_left: usize,
    padding_top: usize,
    padding_right: usize,
    padding_bottom: usize,
    padding_left: usize,
    border_width: usize,
    border_color: Option<u32>,
    width: Option<CssLength>,
    height: Option<usize>,
}

impl BrowserBoxStyle {
    fn has_layout(self) -> bool {
        self.margin_top > 0
            || self.margin_right > 0
            || self.margin_bottom > 0
            || self.margin_left > 0
            || self.padding_top > 0
            || self.padding_right > 0
            || self.padding_bottom > 0
            || self.padding_left > 0
            || self.border_width > 0
            || self.width.is_some()
            || self.height.is_some()
    }

    fn has_decoration(self, background: Option<u32>) -> bool {
        background.is_some()
            || self.border_width > 0
            || self.padding_top > 0
            || self.padding_right > 0
            || self.padding_bottom > 0
            || self.padding_left > 0
    }

    fn merged(self, other: Self) -> Self {
        Self {
            margin_top: if other.margin_top > 0 {
                other.margin_top
            } else {
                self.margin_top
            },
            margin_right: if other.margin_right > 0 {
                other.margin_right
            } else {
                self.margin_right
            },
            margin_bottom: if other.margin_bottom > 0 {
                other.margin_bottom
            } else {
                self.margin_bottom
            },
            margin_left: if other.margin_left > 0 {
                other.margin_left
            } else {
                self.margin_left
            },
            padding_top: if other.padding_top > 0 {
                other.padding_top
            } else {
                self.padding_top
            },
            padding_right: if other.padding_right > 0 {
                other.padding_right
            } else {
                self.padding_right
            },
            padding_bottom: if other.padding_bottom > 0 {
                other.padding_bottom
            } else {
                self.padding_bottom
            },
            padding_left: if other.padding_left > 0 {
                other.padding_left
            } else {
                self.padding_left
            },
            border_width: if other.border_width > 0 {
                other.border_width
            } else {
                self.border_width
            },
            border_color: other.border_color.or(self.border_color),
            width: other.width.or(self.width),
            height: other.height.or(self.height),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct BrowserLineStyle {
    indent_px: usize,
    text_color: Option<u32>,
    background: Option<u32>,
    box_style: BrowserBoxStyle,
    position: CssPosition,
    offset_top: Option<isize>,
    offset_right: Option<isize>,
    offset_bottom: Option<isize>,
    offset_left: Option<isize>,
    float_side: CssFloat,
    z_index: Option<i16>,
    list_style: CssListStyle,
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
            box_style: self.box_style.merged(other.box_style),
            position: if other.position != CssPosition::Static {
                other.position
            } else {
                self.position
            },
            offset_top: other.offset_top.or(self.offset_top),
            offset_right: other.offset_right.or(self.offset_right),
            offset_bottom: other.offset_bottom.or(self.offset_bottom),
            offset_left: other.offset_left.or(self.offset_left),
            float_side: if other.float_side != CssFloat::None {
                other.float_side
            } else {
                self.float_side
            },
            z_index: other.z_index.or(self.z_index),
            list_style: if other.list_style != CssListStyle::Disc {
                other.list_style
            } else {
                self.list_style
            },
        }
    }

    fn content_cols(self, container_cols: usize) -> usize {
        let container_px = container_cols.saturating_mul(CHAR_W);
        let horizontal = self
            .box_style
            .padding_left
            .saturating_add(self.box_style.padding_right)
            .saturating_add(self.box_style.border_width.saturating_mul(2))
            .saturating_add(self.box_style.margin_left)
            .saturating_add(self.box_style.margin_right);
        let width_px = if let Some(width) = self.box_style.width {
            width.resolve_px(container_px)
        } else {
            container_px.saturating_sub(horizontal)
        };
        (width_px / CHAR_W).clamp(8, container_cols.max(8))
    }

    fn visual_z(self) -> i16 {
        self.z_index.unwrap_or(0)
    }

    fn has_flow_effect(self) -> bool {
        self.position != CssPosition::Static
            || self.float_side != CssFloat::None
            || self.offset_top.is_some()
            || self.offset_right.is_some()
            || self.offset_bottom.is_some()
            || self.offset_left.is_some()
            || self.z_index.is_some()
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
    box_part: BrowserLineBoxPart,
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
            box_part: BrowserLineBoxPart::Single,
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

    fn with_box_part(mut self, part: BrowserLineBoxPart) -> Self {
        self.box_part = part;
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
    box_x: usize,
    box_y: usize,
    box_w: usize,
    box_h: usize,
    text: String,
    link: Option<String>,
    kind: BrowserLineKind,
    control: BrowserControl,
    image_slot: Option<usize>,
    style: BrowserLineStyle,
    control_id: Option<usize>,
    z_index: i16,
    source_order: usize,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserResourceKind {
    Stylesheet,
    Image,
    Script,
}

impl BrowserResourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Stylesheet => "css",
            Self::Image => "image",
            Self::Script => "script",
        }
    }
}

#[derive(Clone)]
struct BrowserCachedResource {
    requested_url: String,
    final_url: String,
    kind: BrowserResourceKind,
    content_type: Option<String>,
    bytes: Vec<u8>,
    created_tick: u64,
    last_used_tick: u64,
    hits: usize,
}

#[derive(Clone)]
struct BrowserFetchedResource {
    final_url: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
    cache_hit: bool,
}

#[derive(Default, Clone)]
struct BrowserSubresourceCache {
    entries: Vec<BrowserCachedResource>,
}

#[derive(Default, Clone, Copy)]
struct BrowserSubresourceStats {
    stylesheets_loaded: usize,
    stylesheets_failed: usize,
    images_loaded: usize,
    image_placeholders: usize,
    images_failed: usize,
    cache_hits: usize,
    cache_misses: usize,
}

impl BrowserSubresourceCache {
    fn lookup(
        &mut self,
        requested_url: &str,
        kind: BrowserResourceKind,
    ) -> Option<BrowserFetchedResource> {
        let now = crate::interrupts::ticks();
        let entry = self.entries.iter_mut().find(|entry| {
            entry.kind == kind
                && (entry.requested_url == requested_url || entry.final_url == requested_url)
        })?;
        entry.hits = entry.hits.saturating_add(1);
        entry.last_used_tick = now;
        Some(BrowserFetchedResource {
            final_url: entry.final_url.clone(),
            content_type: entry.content_type.clone(),
            bytes: entry.bytes.clone(),
            cache_hit: true,
        })
    }

    fn remember(
        &mut self,
        requested_url: &str,
        kind: BrowserResourceKind,
        final_url: &str,
        content_type: Option<String>,
        bytes: &[u8],
    ) {
        if bytes.is_empty() || bytes.len() > MAX_BROWSER_RESOURCE_BYTES {
            return;
        }
        if !crate::memory_pressure::admit_allocation(bytes.len(), "browser-cache") {
            return;
        }
        if let Some(pos) = self
            .entries
            .iter()
            .position(|entry| entry.kind == kind && entry.requested_url == requested_url)
        {
            self.entries.remove(pos);
        }
        while self.entries.len() >= MAX_BROWSER_CACHE_ENTRIES {
            let evict = self
                .entries
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.last_used_tick
                        .cmp(&b.last_used_tick)
                        .then(a.hits.cmp(&b.hits))
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.entries.remove(evict);
        }
        let now = crate::interrupts::ticks();
        self.entries.push(BrowserCachedResource {
            requested_url: String::from(requested_url),
            final_url: String::from(final_url),
            kind,
            content_type,
            bytes: bytes.to_vec(),
            created_tick: now,
            last_used_tick: now,
            hits: 0,
        });
    }

    fn entries(&self) -> &[BrowserCachedResource] {
        &self.entries
    }

    fn trim_memory_pressure(&mut self) -> usize {
        let bytes = self
            .entries
            .iter()
            .map(|entry| entry.bytes.len())
            .fold(0usize, |total, len| total.saturating_add(len));
        self.entries.clear();
        self.entries.shrink_to_fit();
        bytes
    }
}

impl BrowserSubresourceStats {
    fn note_cache(&mut self, hit: bool) {
        if hit {
            self.cache_hits = self.cache_hits.saturating_add(1);
        } else {
            self.cache_misses = self.cache_misses.saturating_add(1);
        }
    }

    fn has_activity(self) -> bool {
        self.stylesheets_loaded > 0
            || self.stylesheets_failed > 0
            || self.images_loaded > 0
            || self.image_placeholders > 0
            || self.images_failed > 0
            || self.cache_hits > 0
            || self.cache_misses > 0
    }
}

#[derive(Default, Clone, Copy)]
struct BrowserScriptStats {
    inline_scripts: usize,
    external_scripts: usize,
    external_failed: usize,
    handlers: usize,
    timers: usize,
    mutations: usize,
    storage_reads: usize,
    storage_writes: usize,
    cookie_reads: usize,
    cookie_writes: usize,
    fetches: usize,
    navigation_requests: usize,
    errors: usize,
    statements: usize,
}

impl BrowserScriptStats {
    fn has_activity(self) -> bool {
        self.inline_scripts > 0
            || self.external_scripts > 0
            || self.external_failed > 0
            || self.handlers > 0
            || self.timers > 0
            || self.mutations > 0
            || self.storage_reads > 0
            || self.storage_writes > 0
            || self.cookie_reads > 0
            || self.cookie_writes > 0
            || self.fetches > 0
            || self.navigation_requests > 0
            || self.errors > 0
            || self.statements > 0
    }
}

#[derive(Clone)]
struct BrowserScriptBundle {
    sources: Vec<String>,
    stats: BrowserScriptStats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserScriptEvent {
    Click,
    Change,
    Submit,
}

impl BrowserScriptEvent {
    fn from_attr(name: &str) -> Option<Self> {
        match name {
            "onclick" => Some(Self::Click),
            "onchange" => Some(Self::Change),
            "onsubmit" => Some(Self::Submit),
            _ => None,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match lowercase_ascii(name.trim()).as_str() {
            "click" => Some(Self::Click),
            "change" | "input" => Some(Self::Change),
            "submit" => Some(Self::Submit),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct BrowserScriptHandler {
    node_id: usize,
    event: BrowserScriptEvent,
    code: String,
}

#[derive(Clone)]
struct BrowserScriptVar {
    name: String,
    value: String,
}

#[derive(Clone)]
struct BrowserSessionStorageEntry {
    key: String,
    value: String,
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
    dom_node: Option<usize>,
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
    dom_node: Option<usize>,
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
    external_css: Vec<String>,
    dom: BrowserDomDocument,
    forms: Vec<BrowserFormState>,
    controls: Vec<BrowserFormControlState>,
    script_handlers: Vec<BrowserScriptHandler>,
    session_storage: Vec<BrowserSessionStorageEntry>,
    script_globals: Vec<BrowserScriptVar>,
    script_stats: BrowserScriptStats,
    pending_navigation: Option<String>,
    focused_control: Option<usize>,
}

#[derive(Clone)]
struct BrowserCompatState {
    mode: String,
    url: String,
    reason: String,
    notes: Vec<String>,
}

impl BrowserCompatState {
    fn native(url: &str) -> Self {
        Self {
            mode: String::from("native"),
            url: String::from(url),
            reason: String::from("Rendered by the built-in coolOS HTML/CSS pipeline."),
            notes: Vec::new(),
        }
    }

    fn google_search(url: &str) -> Self {
        let mut state = Self {
            mode: String::from("google-search"),
            url: String::from(url),
            reason: String::from(
                "Script-heavy Google markup was replaced with a native search compatibility shell.",
            ),
            notes: Vec::new(),
        };
        state.add_note("Raw Closure scripts are skipped before layout and DOM sync.");
        state.add_note("The shell submits real GET requests to https://www.google.com/search.");
        state.add_note(
            "Search-result ranking and interactive widgets still require a full JS engine.",
        );
        state
    }

    fn source(url: &str, content_type: Option<&str>) -> Self {
        let mut state = Self {
            mode: String::from("source"),
            url: String::from(url),
            reason: String::from("Non-HTML main resource shown as source instead of a document."),
            notes: Vec::new(),
        };
        if let Some(content_type) = content_type {
            state.add_note(&format!("Content-Type: {}", content_type));
        }
        state
    }

    fn add_note(&mut self, note: &str) {
        if self.notes.len() < MAX_COMPAT_NOTES {
            self.notes.push(String::from(note));
        }
    }
}

impl Default for BrowserCompatState {
    fn default() -> Self {
        Self::native("browser://home")
    }
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
    subresource_cache: BrowserSubresourceCache,
    subresource_stats: BrowserSubresourceStats,
    script_stats: BrowserScriptStats,
    bypass_subresource_cache: bool,
    image_preview: Option<crate::png::PngImage>,
    inline_images: Vec<InlineImage>,
    hit_boxes: Vec<BrowserHitBox>,
    document: Option<BrowserDocumentState>,
    compat_state: BrowserCompatState,
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
            subresource_cache: BrowserSubresourceCache::default(),
            subresource_stats: BrowserSubresourceStats::default(),
            script_stats: BrowserScriptStats::default(),
            bypass_subresource_cache: false,
            image_preview: None,
            inline_images: Vec::new(),
            hit_boxes: Vec::new(),
            document: None,
            compat_state: BrowserCompatState::default(),
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

    pub fn trim_memory_pressure(&mut self) -> usize {
        let mut bytes = self.subresource_cache.trim_memory_pressure();
        if let Some(page) = self.last_page.take() {
            bytes = bytes
                .saturating_add(page.body.len())
                .saturating_add(page.body_bytes.len());
        }
        if let Some(image) = self.image_preview.take() {
            bytes = bytes.saturating_add(image.pixels.len().saturating_mul(4));
        }
        for inline in self.inline_images.drain(..) {
            bytes = bytes.saturating_add(inline.image.pixels.len().saturating_mul(4));
        }
        bytes
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
            'r' => self.reload(),
            'R' => self.hard_reload(),
            'b' | 'B' => self.bookmark_current(),
            'c' | 'C' => self.navigate(CACHE_INTERNAL_URL, true),
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
        self.bypass_subresource_cache = false;
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
        let content_type = response.content_type.as_deref();
        self.lines = if is_image_content(content_type) {
            self.inline_images.clear();
            self.document = None;
            self.compat_state = BrowserCompatState::native(&response.final_url);
            let preview_status = self.decode_image_preview(&response);
            image_response_lines(
                &response.final_url,
                content_type,
                response.body_bytes.len(),
                preview_status.as_deref(),
            )
        } else if is_html_main_content(content_type, &response.final_url, &response.body_bytes) {
            self.image_preview = None;
            self.set_html_document(&response.final_url, &response.body);
            self.append_subresource_status();
            self.append_script_status();
            self.append_compat_status();
            self.lines.clone()
        } else {
            self.image_preview = None;
            self.inline_images.clear();
            self.document = None;
            self.subresource_stats = BrowserSubresourceStats::default();
            self.script_stats = BrowserScriptStats::default();
            self.compat_state = BrowserCompatState::source(&response.final_url, content_type);
            self.title = source_title_for_content(content_type, &response.final_url);
            self.append_compat_status();
            source_response_lines(
                &response.final_url,
                content_type,
                &response.body,
                response.body_bytes.len(),
                self.cols.max(48),
            )
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
        self.bypass_subresource_cache = false;
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
            CACHE_INTERNAL_URL => {
                self.title = String::from("Cache");
                self.status = format!(
                    "{} cached subresource(s)",
                    self.subresource_cache.entries().len()
                );
                self.lines = self.browser_cache_lines();
            }
            COMPAT_INTERNAL_URL => {
                self.title = String::from("Compatibility");
                self.status = format!("mode={}", self.compat_state.mode);
                self.lines = self.browser_compat_lines();
            }
            JS_INTERNAL_URL => {
                self.title = String::from("Scripts");
                self.status = script_stats_debug_line(self.script_stats);
                self.lines = self.browser_script_lines();
            }
            STORAGE_INTERNAL_URL => {
                self.title = String::from("Storage");
                self.status = crate::browser_storage::summary_line();
                self.lines = browser_storage_lines();
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

    fn hard_reload(&mut self) {
        self.bypass_subresource_cache = true;
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
        self.bypass_subresource_cache = false;
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
        self.bypass_subresource_cache = false;
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
        self.set_html_document(&url, &body);
        self.append_subresource_status();
        self.append_script_status();
        self.append_compat_status();
        self.lines = if self.lines.is_empty() {
            vec![kind_line("(empty document)", BrowserLineKind::Muted)]
        } else {
            self.lines.clone()
        };
        self.address_focused = false;
        self.address_selected = false;
        self.bypass_subresource_cache = false;
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

    fn browser_cache_lines(&self) -> Vec<BrowserLine> {
        let mut out = vec![
            kind_line("Browser Cache", BrowserLineKind::Heading),
            kind_line(
                "In-memory CSS, image, and script subresources for this Browser window.",
                BrowserLineKind::Muted,
            ),
            line(""),
        ];
        let stats = self.subresource_stats;
        out.push(kind_line(
            &format!(
                "last load: css={}/{} images={} placeholders={} failed={} cache={}/{}",
                stats.stylesheets_loaded,
                stats
                    .stylesheets_loaded
                    .saturating_add(stats.stylesheets_failed),
                stats.images_loaded,
                stats.image_placeholders,
                stats.images_failed,
                stats.cache_hits,
                stats.cache_hits.saturating_add(stats.cache_misses)
            ),
            BrowserLineKind::Muted,
        ));
        if self.subresource_cache.entries().is_empty() {
            out.push(kind_line(
                "No cached subresources yet.",
                BrowserLineKind::Muted,
            ));
            return out;
        }
        let now = crate::interrupts::ticks();
        for entry in self.subresource_cache.entries().iter().rev().take(32) {
            let age = now
                .saturating_sub(entry.created_tick)
                .saturating_div(crate::interrupts::TIMER_HZ as u64);
            let used = now
                .saturating_sub(entry.last_used_tick)
                .saturating_div(crate::interrupts::TIMER_HZ as u64);
            out.push(link_line(
                &format!(
                    "{}  {} bytes  hits={}  age={}s  used={}s  {}",
                    entry.kind.label(),
                    entry.bytes.len(),
                    entry.hits,
                    age,
                    used,
                    entry
                        .content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream")
                ),
                &entry.final_url,
            ));
            if entry.requested_url != entry.final_url {
                out.push(kind_line(
                    &format!("requested {}", entry.requested_url),
                    BrowserLineKind::Muted,
                ));
            }
        }
        out
    }

    fn browser_compat_lines(&self) -> Vec<BrowserLine> {
        let compat = &self.compat_state;
        let mut out = vec![
            kind_line("Browser Compatibility", BrowserLineKind::Heading),
            kind_line(
                "Current handling path for the last loaded main resource.",
                BrowserLineKind::Muted,
            ),
            line(""),
            line(&format!("Mode: {}", compat.mode)),
            line(&format!("URL: {}", compat.url)),
            line(&format!("Reason: {}", compat.reason)),
        ];
        if !compat.notes.is_empty() {
            out.push(line(""));
            out.push(kind_line("Notes", BrowserLineKind::Muted));
            for note in compat.notes.iter() {
                out.push(line(&format!("- {}", note)));
            }
        }
        out.push(line(""));
        out.push(link_line("Home", "browser://home"));
        out.push(link_line("Script diagnostics", JS_INTERNAL_URL));
        out.push(link_line("Cache state", CACHE_INTERNAL_URL));
        out
    }

    fn browser_script_lines(&self) -> Vec<BrowserLine> {
        let stats = self.script_stats;
        let mut out = vec![
            kind_line("Browser Scripts", BrowserLineKind::Heading),
            kind_line(
                "Bounded document scripts, event handlers, timers, and DOM mutations.",
                BrowserLineKind::Muted,
            ),
            line(""),
            kind_line(&script_stats_debug_line(stats), BrowserLineKind::Muted),
            line(""),
            line(&format!(
                "scripts: inline={} external={}/{}",
                stats.inline_scripts,
                stats.external_scripts,
                stats.external_scripts.saturating_add(stats.external_failed)
            )),
            line(&format!(
                "runtime: handlers={} timers={} statements={}",
                stats.handlers, stats.timers, stats.statements
            )),
            line(&format!(
                "dom: mutations={} errors={}",
                stats.mutations, stats.errors
            )),
            line(&format!(
                "web APIs: storage={}/{} cookies={}/{} fetch={} nav={}",
                stats.storage_reads,
                stats.storage_writes,
                stats.cookie_reads,
                stats.cookie_writes,
                stats.fetches,
                stats.navigation_requests
            )),
        ];
        if stats.errors > 0 {
            out.push(kind_line(
                "Unsupported script statements were skipped by the bounded runtime.",
                BrowserLineKind::Muted,
            ));
        }
        out
    }

    fn append_subresource_status(&mut self) {
        let stats = self.subresource_stats;
        if !stats.has_activity() {
            return;
        }
        let css_total = stats
            .stylesheets_loaded
            .saturating_add(stats.stylesheets_failed);
        if css_total > 0 {
            self.status
                .push_str(&format!("  css={}/{}", stats.stylesheets_loaded, css_total));
        }
        let image_total = stats
            .images_loaded
            .saturating_add(stats.image_placeholders)
            .saturating_add(stats.images_failed);
        if image_total > 0 {
            self.status.push_str(&format!(
                "  images={} placeholders={} failed={}",
                stats.images_loaded, stats.image_placeholders, stats.images_failed
            ));
        }
        let cache_total = stats.cache_hits.saturating_add(stats.cache_misses);
        if cache_total > 0 {
            self.status
                .push_str(&format!("  cache={}/{}", stats.cache_hits, cache_total));
        }
    }

    fn append_script_status(&mut self) {
        let stats = self.script_stats;
        if !stats.has_activity() {
            return;
        }
        let total = stats
            .inline_scripts
            .saturating_add(stats.external_scripts)
            .saturating_add(stats.external_failed);
        self.status.push_str(&format!(
            "  js={}/{} handlers={} timers={} mut={} api={} err={}",
            stats.inline_scripts.saturating_add(stats.external_scripts),
            total,
            stats.handlers,
            stats.timers,
            stats.mutations,
            stats
                .storage_reads
                .saturating_add(stats.storage_writes)
                .saturating_add(stats.cookie_reads)
                .saturating_add(stats.cookie_writes)
                .saturating_add(stats.fetches)
                .saturating_add(stats.navigation_requests),
            stats.errors
        ));
    }

    fn append_compat_status(&mut self) {
        if self.compat_state.mode != "native" {
            self.status
                .push_str(&format!("  compat={}", self.compat_state.mode));
        }
    }

    fn set_html_document(&mut self, base_url: &str, body: &str) -> usize {
        self.subresource_stats = BrowserSubresourceStats::default();
        self.script_stats = BrowserScriptStats::default();
        let body_text = response_body_text(body).unwrap_or(body);
        let effective_base = extract_base_href(body_text, base_url);
        let compat_body = google_search_compat_document(&effective_base, body_text);
        let render_body = compat_body.as_deref().unwrap_or(body_text);
        self.compat_state = if compat_body.is_some() {
            BrowserCompatState::google_search(&effective_base)
        } else {
            BrowserCompatState::native(&effective_base)
        };
        let external_css = load_document_stylesheets(
            &effective_base,
            render_body,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.bypass_subresource_cache,
        );
        let scripts = load_document_scripts(
            &effective_base,
            render_body,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.bypass_subresource_cache,
        );
        let document = BrowserDocumentState::from_html_with_external_css_and_scripts(
            &effective_base,
            render_body,
            external_css,
            scripts.sources,
            scripts.stats,
        );
        self.script_stats = document.script_stats;
        self.document = Some(document);
        let images = self.reflow_document();
        self.bypass_subresource_cache = false;
        images
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
        let images = attach_html_images_with_cache(
            &mut lines,
            &mut self.inline_images,
            &mut self.subresource_cache,
            &mut self.subresource_stats,
            self.cols.max(48),
            self.bypass_subresource_cache,
        );
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
        if let Some(document) = self.document.as_ref() {
            self.script_stats = document.script_stats;
        }
        let pending_navigation = self
            .document
            .as_mut()
            .and_then(|document| document.pending_navigation.take());
        if let Some(url) = pending_navigation {
            self.status = format!("Script navigating {}", url);
            self.navigate(&url, true);
            return;
        }
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
                self.status = if self.script_stats.mutations > 0 {
                    format!(
                        "Control changed  js mut={} err={}",
                        self.script_stats.mutations, self.script_stats.errors
                    )
                } else {
                    String::from("Control changed")
                };
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
            if item.box_y.saturating_add(item.box_h) <= self.scroll || item.box_y >= viewport_bottom
            {
                continue;
            }
            if item.box_y < self.scroll || item.box_y.saturating_add(item.box_h) > viewport_bottom {
                continue;
            }
            let y = lines_y + item.y.saturating_sub(self.scroll);
            let x = PAD_X + item.x;
            let box_x = PAD_X + item.box_x;
            let box_y = lines_y + item.box_y.saturating_sub(self.scroll);
            if y >= doc_y.saturating_add(doc_h) {
                continue;
            }
            self.draw_box_decoration(stride, box_x, box_y, item.box_w, item.box_h, item.style);

            if let Some(slot) = item.image_slot {
                if let Some(image) = self
                    .inline_images
                    .get(slot)
                    .map(|inline| inline.image.clone())
                {
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
                    x: box_x,
                    y: box_y,
                    w: item.box_w.max(item.w),
                    h: item.box_h.max(item.h).max(LINE_H),
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

    fn draw_box_decoration(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        style: BrowserLineStyle,
    ) {
        if w == 0 || h == 0 || !style.box_style.has_decoration(style.background) {
            return;
        }
        if let Some(bg) = style.background {
            self.fill_rect(stride, x, y, w, h, bg);
        }
        let border_w = style.box_style.border_width.min(8);
        let color = style.box_style.border_color.unwrap_or(BORDER);
        for inset in 0..border_w {
            if w <= inset.saturating_mul(2) || h <= inset.saturating_mul(2) {
                break;
            }
            self.draw_rect(
                stride,
                x + inset,
                y + inset,
                w.saturating_sub(inset * 2),
                h.saturating_sub(inset * 2),
                color,
            );
        }
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
    let mut max_bottom = 0usize;
    let mut active_floats = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        prune_active_floats(&mut active_floats, y);
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
            && !line.style.box_style.has_layout()
            && !line.style.has_flow_effect()
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
                    || next.style.has_flow_effect()
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
                        box_x: x,
                        box_y: y,
                        box_w: w,
                        box_h: LINE_H,
                        text: next.text.clone(),
                        link: next.link.clone(),
                        kind: next.kind,
                        control: BrowserControl::None,
                        image_slot: None,
                        style: next.style,
                        control_id: next.control_id,
                        z_index: next.style.visual_z(),
                        source_order: items.len(),
                    });
                    x = x.saturating_add(w).saturating_add(CONTROL_GAP + 8);
                }
                y = y.saturating_add(LINE_H + BLOCK_GAP);
                i = j;
                continue;
            }
        }
        if matches!(line.control, BrowserControl::Button { .. })
            && !line.style.box_style.has_layout()
            && !line.style.has_flow_effect()
        {
            let align = line.align;
            let mut group = Vec::new();
            let mut total_w = 0usize;
            let mut j = i;
            while let Some(next) = lines.get(j) {
                if next.align != align
                    || next.style != line.style
                    || next.style.has_flow_effect()
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
                    box_x: x,
                    box_y: y,
                    box_w: w,
                    box_h: CONTROL_H,
                    text: next.text.clone(),
                    link: next.link.clone(),
                    kind: next.kind,
                    control: next.control.clone(),
                    image_slot: None,
                    style: next.style,
                    control_id: next.control_id,
                    z_index: next.style.visual_z(),
                    source_order: items.len(),
                });
                x = x.saturating_add(w).saturating_add(CONTROL_GAP);
            }
            y = y.saturating_add(CONTROL_H + BLOCK_GAP);
            i = j;
            continue;
        }
        if !matches!(line.control, BrowserControl::None) {
            let available_w = doc_w.saturating_sub(line.style.indent_px).max(1);
            let natural_w = control_width(&line.control, available_w);
            let h = control_height(&line.control);
            let (flow_left, flow_right) =
                flow_reserve_for_line(&active_floats, y, doc_w, line.style.float_side);
            let placed =
                place_boxed_item_with_flow(line, doc_w, natural_w, h, flow_left, flow_right);
            let mut item = BrowserLayoutItem {
                x: placed.content_x,
                y: y.saturating_add(placed.content_y),
                w: placed.content_w,
                h: placed.content_h,
                box_x: placed.box_x,
                box_y: y.saturating_add(placed.box_y),
                box_w: placed.box_w,
                box_h: placed.box_h,
                text: line.text.clone(),
                link: line.link.clone(),
                kind: line.kind,
                control: line.control.clone(),
                image_slot: None,
                style: line.style,
                control_id: line.control_id,
                z_index: line.style.visual_z(),
                source_order: items.len(),
            };
            apply_css_position(&mut item, doc_w);
            max_bottom = max_bottom.max(item.box_y.saturating_add(item.box_h));
            if line.style.float_side != CssFloat::None {
                active_floats.push(ActiveBrowserFloat::from_item(line.style.float_side, &item));
            }
            items.push(item);
            if line_part_occupies_flow(line) {
                y = y.saturating_add(placed.outer_h + BLOCK_GAP);
            }
            i += 1;
            continue;
        }
        if let Some(slot) = line.image_slot {
            if let Some(image) = inline_images.get(slot).map(|inline| &inline.image) {
                let (flow_left, flow_right) =
                    flow_reserve_for_line(&active_floats, y, doc_w, line.style.float_side);
                let metrics = box_metrics(line.style.box_style, line.box_part);
                let chrome = metrics.horizontal_chrome();
                let max_w = doc_w
                    .saturating_sub(line.style.indent_px)
                    .saturating_sub(flow_left)
                    .saturating_sub(flow_right)
                    .saturating_sub(chrome)
                    .max(1);
                let (draw_w, draw_h) = scaled_image_size_with_hint(
                    image.width,
                    image.height,
                    line.image_hint,
                    max_w,
                    INLINE_IMAGE_MAX_H,
                );
                let placed =
                    place_boxed_item_with_flow(line, doc_w, draw_w, draw_h, flow_left, flow_right);
                let mut item = BrowserLayoutItem {
                    x: placed.content_x,
                    y: y.saturating_add(placed.content_y),
                    w: placed.content_w,
                    h: placed.content_h,
                    box_x: placed.box_x,
                    box_y: y.saturating_add(placed.box_y),
                    box_w: placed.box_w,
                    box_h: placed.box_h,
                    text: String::new(),
                    link: line.link.clone(),
                    kind: BrowserLineKind::Image,
                    control: BrowserControl::None,
                    image_slot: Some(slot),
                    style: line.style,
                    control_id: line.control_id,
                    z_index: line.style.visual_z(),
                    source_order: items.len(),
                };
                apply_css_position(&mut item, doc_w);
                max_bottom = max_bottom.max(item.box_y.saturating_add(item.box_h));
                if line.style.float_side != CssFloat::None {
                    active_floats.push(ActiveBrowserFloat::from_item(line.style.float_side, &item));
                }
                items.push(item);
                if line_part_occupies_flow(line) {
                    y = y.saturating_add(placed.outer_h + BLOCK_GAP);
                }
                i += 1;
                continue;
            }
        }
        let (flow_left, flow_right) =
            flow_reserve_for_line(&active_floats, y, doc_w, line.style.float_side);
        let available_w = content_available_width_with_flow(line, doc_w, flow_left, flow_right);
        let w = text_pixel_width(&line.text).min(available_w);
        let h = if line.kind == BrowserLineKind::Heading {
            LINE_H + 2
        } else {
            LINE_H
        };
        let placed = place_boxed_item_with_flow(line, doc_w, w, h, flow_left, flow_right);
        let mut item = BrowserLayoutItem {
            x: placed.content_x,
            y: y.saturating_add(placed.content_y),
            w: placed.content_w,
            h: placed.content_h,
            box_x: placed.box_x,
            box_y: y.saturating_add(placed.box_y),
            box_w: placed.box_w,
            box_h: placed.box_h,
            text: line.text.clone(),
            link: line.link.clone(),
            kind: line.kind,
            control: BrowserControl::None,
            image_slot: None,
            style: line.style,
            control_id: line.control_id,
            z_index: line.style.visual_z(),
            source_order: items.len(),
        };
        apply_css_position(&mut item, doc_w);
        max_bottom = max_bottom.max(item.box_y.saturating_add(item.box_h));
        if line.style.float_side != CssFloat::None {
            active_floats.push(ActiveBrowserFloat::from_item(line.style.float_side, &item));
        }
        items.push(item);
        if line_part_occupies_flow(line) {
            y = y.saturating_add(placed.outer_h);
        }
        i += 1;
    }
    items.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then(a.source_order.cmp(&b.source_order))
    });
    BrowserLayout {
        items,
        content_h: y.max(max_bottom).saturating_add(BLOCK_GAP),
    }
}

#[derive(Clone, Copy)]
struct BrowserBoxMetrics {
    margin_top: usize,
    margin_right: usize,
    margin_bottom: usize,
    margin_left: usize,
    padding_top: usize,
    padding_right: usize,
    padding_bottom: usize,
    padding_left: usize,
    border_top: usize,
    border_right: usize,
    border_bottom: usize,
    border_left: usize,
}

impl BrowserBoxMetrics {
    fn horizontal_chrome(self) -> usize {
        self.margin_left
            .saturating_add(self.margin_right)
            .saturating_add(self.padding_left)
            .saturating_add(self.padding_right)
            .saturating_add(self.border_left)
            .saturating_add(self.border_right)
    }
}

struct PlacedBrowserBox {
    content_x: usize,
    content_y: usize,
    content_w: usize,
    content_h: usize,
    box_x: usize,
    box_y: usize,
    box_w: usize,
    box_h: usize,
    outer_h: usize,
}

#[derive(Clone, Copy)]
struct ActiveBrowserFloat {
    side: CssFloat,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl ActiveBrowserFloat {
    fn from_item(side: CssFloat, item: &BrowserLayoutItem) -> Self {
        Self {
            side,
            x: item.box_x,
            y: item.box_y,
            w: item.box_w,
            h: item.box_h.saturating_add(BLOCK_GAP),
        }
    }

    fn overlaps_y(self, y: usize) -> bool {
        y >= self.y && y < self.y.saturating_add(self.h)
    }
}

fn box_metrics(style: BrowserBoxStyle, part: BrowserLineBoxPart) -> BrowserBoxMetrics {
    let border = style.border_width.min(8);
    let top = matches!(part, BrowserLineBoxPart::Single | BrowserLineBoxPart::First);
    let bottom = matches!(part, BrowserLineBoxPart::Single | BrowserLineBoxPart::Last);
    BrowserBoxMetrics {
        margin_top: if top { style.margin_top } else { 0 },
        margin_right: style.margin_right,
        margin_bottom: if bottom { style.margin_bottom } else { 0 },
        margin_left: style.margin_left,
        padding_top: if top { style.padding_top } else { 0 },
        padding_right: style.padding_right,
        padding_bottom: if bottom { style.padding_bottom } else { 0 },
        padding_left: style.padding_left,
        border_top: if top { border } else { 0 },
        border_right: border,
        border_bottom: if bottom { border } else { 0 },
        border_left: border,
    }
}

fn content_available_width_with_flow(
    line: &BrowserLine,
    doc_w: usize,
    flow_left: usize,
    flow_right: usize,
) -> usize {
    let metrics = box_metrics(line.style.box_style, line.box_part);
    let available = doc_w
        .saturating_sub(line.style.indent_px)
        .saturating_sub(flow_left)
        .saturating_sub(flow_right)
        .saturating_sub(metrics.horizontal_chrome())
        .max(1);
    line.style
        .box_style
        .width
        .map(|width| width.resolve_px(doc_w).min(available).max(1))
        .unwrap_or(available)
}

fn place_boxed_item_with_flow(
    line: &BrowserLine,
    doc_w: usize,
    natural_w: usize,
    natural_h: usize,
    flow_left: usize,
    flow_right: usize,
) -> PlacedBrowserBox {
    let metrics = box_metrics(line.style.box_style, line.box_part);
    let available_content_w = content_available_width_with_flow(line, doc_w, flow_left, flow_right);
    let specified_w = line
        .style
        .box_style
        .width
        .map(|width| width.resolve_px(doc_w).min(available_content_w).max(1));
    let content_w = specified_w.unwrap_or_else(|| natural_w.min(available_content_w).max(1));
    let content_h = if matches!(line.box_part, BrowserLineBoxPart::Single) {
        line.style
            .box_style
            .height
            .map(|height| height.max(natural_h))
            .unwrap_or(natural_h)
    } else {
        natural_h
    };
    let box_w = metrics
        .border_left
        .saturating_add(metrics.padding_left)
        .saturating_add(content_w)
        .saturating_add(metrics.padding_right)
        .saturating_add(metrics.border_right);
    let outer_w = metrics
        .margin_left
        .saturating_add(box_w)
        .saturating_add(metrics.margin_right);
    let align_space = doc_w
        .saturating_sub(line.style.indent_px)
        .saturating_sub(flow_left)
        .saturating_sub(flow_right)
        .max(1);
    let flow_origin = line.style.indent_px.saturating_add(flow_left);
    let outer_x = match line.style.float_side {
        CssFloat::Left => line.style.indent_px,
        CssFloat::Right => doc_w.saturating_sub(outer_w).max(line.style.indent_px),
        CssFloat::None => flow_origin.saturating_add(aligned_x(align_space, outer_w, line.align)),
    };
    let box_x = outer_x.saturating_add(metrics.margin_left);
    let box_y = metrics.margin_top;
    let content_x = box_x
        .saturating_add(metrics.border_left)
        .saturating_add(metrics.padding_left);
    let content_y = metrics
        .margin_top
        .saturating_add(metrics.border_top)
        .saturating_add(metrics.padding_top);
    let box_h = metrics
        .border_top
        .saturating_add(metrics.padding_top)
        .saturating_add(content_h)
        .saturating_add(metrics.padding_bottom)
        .saturating_add(metrics.border_bottom);
    let outer_h = metrics
        .margin_top
        .saturating_add(box_h)
        .saturating_add(metrics.margin_bottom);
    PlacedBrowserBox {
        content_x,
        content_y,
        content_w,
        content_h,
        box_x,
        box_y,
        box_w,
        box_h,
        outer_h,
    }
}

fn prune_active_floats(floats: &mut Vec<ActiveBrowserFloat>, y: usize) {
    floats.retain(|float_box| y < float_box.y.saturating_add(float_box.h));
}

fn flow_reserve_for_line(
    floats: &[ActiveBrowserFloat],
    y: usize,
    doc_w: usize,
    line_float: CssFloat,
) -> (usize, usize) {
    if line_float != CssFloat::None {
        return (0, 0);
    }
    let mut left = 0usize;
    let mut right = 0usize;
    for float_box in floats {
        if !float_box.overlaps_y(y) {
            continue;
        }
        match float_box.side {
            CssFloat::Left => {
                left = left.max(float_box.x.saturating_add(float_box.w).saturating_add(8));
            }
            CssFloat::Right => {
                right = right.max(
                    doc_w
                        .saturating_sub(float_box.x)
                        .saturating_add(8)
                        .min(doc_w),
                );
            }
            CssFloat::None => {}
        }
    }
    if left.saturating_add(right).saturating_add(CHAR_W * 8) > doc_w {
        (0, 0)
    } else {
        (left, right)
    }
}

fn line_part_occupies_flow(line: &BrowserLine) -> bool {
    !matches!(
        line.style.position,
        CssPosition::Absolute | CssPosition::Fixed
    ) && line.style.float_side == CssFloat::None
}

fn apply_css_position(item: &mut BrowserLayoutItem, doc_w: usize) {
    let style = item.style;
    let content_dx = item.x.saturating_sub(item.box_x);
    let content_dy = item.y.saturating_sub(item.box_y);
    match style.position {
        CssPosition::Static => {}
        CssPosition::Relative | CssPosition::Sticky => {
            let dx = style
                .offset_left
                .unwrap_or(0)
                .saturating_sub(style.offset_right.unwrap_or(0));
            let dy = style
                .offset_top
                .unwrap_or(0)
                .saturating_sub(style.offset_bottom.unwrap_or(0));
            item.box_x = offset_usize(item.box_x, dx);
            item.box_y = offset_usize(item.box_y, dy);
            item.x = offset_usize(item.x, dx);
            item.y = offset_usize(item.y, dy);
        }
        CssPosition::Absolute | CssPosition::Fixed => {
            if let Some(left) = style.offset_left {
                item.box_x = offset_usize(0, left);
            } else if let Some(right) = style.offset_right {
                item.box_x = offset_usize(doc_w.saturating_sub(item.box_w), -right);
            }
            if let Some(top) = style.offset_top {
                item.box_y = offset_usize(0, top);
            } else if let Some(bottom) = style.offset_bottom {
                item.box_y = offset_usize(item.box_y, -bottom);
            }
            item.x = item.box_x.saturating_add(content_dx);
            item.y = item.box_y.saturating_add(content_dy);
        }
    }
}

fn offset_usize(value: usize, delta: isize) -> usize {
    if delta >= 0 {
        value.saturating_add(delta as usize)
    } else {
        value.saturating_sub(delta.unsigned_abs())
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
        link_line("Cache state", CACHE_INTERNAL_URL),
        link_line("Script diagnostics", JS_INTERNAL_URL),
        link_line("Web storage", STORAGE_INTERNAL_URL),
        link_line("Compatibility", COMPAT_INTERNAL_URL),
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

fn browser_storage_lines() -> Vec<BrowserLine> {
    crate::browser_storage::lines()
        .into_iter()
        .enumerate()
        .map(|(idx, text)| {
            if idx == 0 {
                kind_line(&text, BrowserLineKind::Heading)
            } else if text.is_empty() {
                BrowserLine::new(String::new(), None, BrowserLineKind::Text)
            } else if text.starts_with("localStorage:")
                || text.starts_with("Storage:")
                || text.starts_with("sessionStorage")
                || text == "No localStorage entries stored."
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

fn is_html_main_content(content_type: Option<&str>, url: &str, bytes: &[u8]) -> bool {
    if let Some(value) = content_type.and_then(|value| value.split(';').next()) {
        let value = value.trim();
        if value.eq_ignore_ascii_case("text/html")
            || value.eq_ignore_ascii_case("application/xhtml+xml")
        {
            return true;
        }
        if value.eq_ignore_ascii_case("text/plain") {
            return looks_like_html_bytes(bytes);
        }
        return false;
    }
    extension_from_path(url).eq_ignore_ascii_case("html") || looks_like_html_bytes(bytes)
}

fn source_title_for_content(content_type: Option<&str>, url: &str) -> String {
    let mime = content_type
        .and_then(|value| value.split(';').next())
        .unwrap_or("")
        .trim();
    if is_main_script_content(content_type, url) {
        String::from("JavaScript Source")
    } else if mime.eq_ignore_ascii_case("text/css") || extension_from_path(url) == "css" {
        String::from("CSS Source")
    } else if mime.eq_ignore_ascii_case("application/json") || extension_from_path(url) == "json" {
        String::from("JSON Source")
    } else if mime.starts_with("text/") {
        String::from("Text Source")
    } else {
        String::from("Resource")
    }
}

fn source_response_lines(
    url: &str,
    content_type: Option<&str>,
    body: &str,
    byte_len: usize,
    cols: usize,
) -> Vec<BrowserLine> {
    let title = source_title_for_content(content_type, url);
    let mut out = vec![
        kind_line(&title, BrowserLineKind::Heading),
        kind_line(
            content_type.unwrap_or("application/octet-stream"),
            BrowserLineKind::Muted,
        ),
        kind_line(
            &format!("{} bytes received", byte_len),
            BrowserLineKind::Muted,
        ),
        link_line("Resource URL", url),
        line(""),
        kind_line("Source preview", BrowserLineKind::Muted),
    ];
    let mut shown = 0usize;
    for raw in body.lines() {
        if shown >= 28 {
            out.push(kind_line(
                "... preview truncated ...",
                BrowserLineKind::Muted,
            ));
            break;
        }
        let mut line_text = clean_inline_text(raw);
        if line_text.len() > cols {
            line_text = truncate_text_for_source(&line_text, cols);
        }
        out.push(kind_line(&line_text, BrowserLineKind::Code));
        shown += 1;
    }
    if shown == 0 {
        out.push(kind_line("(empty resource)", BrowserLineKind::Muted));
    }
    out
}

fn truncate_text_for_source(input: &str, max_len: usize) -> String {
    let mut out = String::new();
    for c in input.chars() {
        if out.len().saturating_add(c.len_utf8()).saturating_add(3) > max_len {
            out.push_str("...");
            return out;
        }
        out.push(c);
    }
    out
}

fn google_search_compat_document(base_url: &str, body: &str) -> Option<String> {
    let Ok((_scheme, host, path)) = parse_web_url(base_url) else {
        return None;
    };
    if !is_google_host(&host) {
        return None;
    }
    let path_only = path_without_query_fragment(&path);
    if !matches!(
        path_only.as_str(),
        "/" | "/webhp" | "/search" | "/imghp" | "/advanced_search"
    ) {
        return None;
    }
    let lower = lowercase_ascii(body);
    let looks_like_google = lower.contains("<title>google")
        || lower.contains("name=\"q\"")
        || lower.contains("name='q'")
        || lower.contains("name=q")
        || lower.contains("closure library authors")
        || lower.contains("this.gbar_");
    if !looks_like_google {
        return None;
    }
    Some(build_google_search_compat_document(
        google_query_from_url(base_url).as_deref().unwrap_or(""),
        path_only == "/search",
    ))
}

fn build_google_search_compat_document(query: &str, is_results_url: bool) -> String {
    let query_value = escape_html(query);
    let mut out = String::from(
        "<!doctype html><html><head><title>Google</title><style>\
body{font-family:sans-serif;background:#fff;color:#202124;text-align:center}\
.logo{font-size:48px;margin-top:36px;margin-bottom:18px;color:#4285f4}\
form{margin:0 auto 16px auto;width:70%;padding:10px;border:1px solid #dadce0;background:#fff}\
input{margin:4px;padding:6px;border:1px solid #dadce0}\
.note{color:#5f6368;font-size:12px}\
</style></head><body><h1 class=\"logo\">Google</h1>",
    );
    out.push_str("<form action=\"https://www.google.com/search\" method=\"get\">");
    out.push_str("<input type=\"search\" name=\"q\" value=\"");
    out.push_str(&query_value);
    out.push_str("\" placeholder=\"Search Google\">");
    out.push_str("<input type=\"submit\" name=\"btnG\" value=\"Google Search\">");
    out.push_str("</form>");
    if is_results_url && !query.is_empty() {
        out.push_str("<p>Search: ");
        out.push_str(&query_value);
        out.push_str("</p>");
        out.push_str(
            "<p class=\"note\">Results pages need a modern JavaScript and layout engine; \
this shell keeps search submission usable.</p>",
        );
    } else {
        out.push_str(
            "<p class=\"note\">Compatibility mode keeps the Google search form usable while \
coolOS grows a fuller browser engine.</p>",
        );
    }
    out.push_str("<p><a href=\"browser://compat\">Compatibility diagnostics</a></p>");
    out.push_str("</body></html>");
    out
}

fn is_google_host(host: &str) -> bool {
    let host = lowercase_ascii(host);
    host.starts_with("google.") || host.contains(".google.")
}

fn path_without_query_fragment(path: &str) -> String {
    let end = path
        .find('?')
        .or_else(|| path.find('#'))
        .unwrap_or(path.len());
    String::from(&path[..end])
}

fn google_query_from_url(url: &str) -> Option<String> {
    query_param_from_url(url, "q")
}

fn query_param_from_url(url: &str, wanted: &str) -> Option<String> {
    let start = url.find('?')?.saturating_add(1);
    let end = url[start..]
        .find('#')
        .map(|rel| start + rel)
        .unwrap_or(url.len());
    for pair in url[start..end].split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if decode_query(key) == wanted {
            return Some(decode_query(value));
        }
    }
    None
}

fn escape_html(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
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

fn load_document_stylesheets(
    base_url: &str,
    body: &str,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    for url in stylesheet_urls(base_url, body) {
        match fetch_subresource_with_cache(
            &url,
            BrowserResourceKind::Stylesheet,
            cache,
            stats,
            bypass_cache,
        ) {
            Ok(resource) => {
                if !is_stylesheet_content(resource.content_type.as_deref(), &resource.final_url) {
                    stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
                    continue;
                }
                let css = String::from_utf8_lossy(&resource.bytes).into_owned();
                if css.trim().is_empty() {
                    stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
                    continue;
                }
                stats.stylesheets_loaded = stats.stylesheets_loaded.saturating_add(1);
                out.push(css);
                if out.len() >= MAX_STYLESHEET_SUBRESOURCES {
                    break;
                }
            }
            Err(_) => {
                stats.stylesheets_failed = stats.stylesheets_failed.saturating_add(1);
            }
        }
    }
    out
}

fn load_document_scripts(
    base_url: &str,
    body: &str,
    cache: &mut BrowserSubresourceCache,
    subresource_stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> BrowserScriptBundle {
    let mut bundle = BrowserScriptBundle {
        sources: Vec::new(),
        stats: BrowserScriptStats::default(),
    };
    let lower = lowercase_ascii(body);
    let mut i = 0usize;
    while bundle.sources.len() < MAX_SCRIPT_SUBRESOURCES {
        let Some(rel) = lower[i..].find("<script") else {
            break;
        };
        let start = i + rel;
        let Some(tag_end_rel) = find_tag_end(&body[start..]) else {
            break;
        };
        let tag_end = start + tag_end_rel;
        let tag = &body[start + 1..tag_end];
        let content_start = tag_end + 1;
        let close_rel = lower[content_start..].find("</script");
        let content_end = close_rel
            .map(|rel| content_start + rel)
            .unwrap_or(content_start);
        let next_i = close_rel
            .and_then(|rel| {
                let close_start = content_start + rel;
                find_tag_end(&body[close_start..]).map(|close_end| close_start + close_end + 1)
            })
            .unwrap_or(content_end);

        if script_type_is_executable(tag) {
            if let Some(src) = attr_value(tag, "src") {
                let src = decode_entities(src.trim());
                let url = resolve_url(base_url, &src);
                if script_url_allowed(base_url, &url) {
                    match fetch_subresource_with_cache(
                        &url,
                        BrowserResourceKind::Script,
                        cache,
                        subresource_stats,
                        bypass_cache,
                    ) {
                        Ok(resource) => {
                            if is_script_content(
                                resource.content_type.as_deref(),
                                &resource.final_url,
                            ) && resource.bytes.len() <= MAX_SCRIPT_BYTES
                            {
                                bundle.stats.external_scripts =
                                    bundle.stats.external_scripts.saturating_add(1);
                                bundle
                                    .sources
                                    .push(String::from_utf8_lossy(&resource.bytes).into_owned());
                            } else {
                                bundle.stats.external_failed =
                                    bundle.stats.external_failed.saturating_add(1);
                            }
                        }
                        Err(_) => {
                            bundle.stats.external_failed =
                                bundle.stats.external_failed.saturating_add(1);
                        }
                    }
                } else {
                    bundle.stats.external_failed = bundle.stats.external_failed.saturating_add(1);
                }
            } else if content_end > content_start {
                let script = &body[content_start..content_end];
                if script.len() <= MAX_SCRIPT_BYTES {
                    bundle.stats.inline_scripts = bundle.stats.inline_scripts.saturating_add(1);
                    bundle.sources.push(String::from(script));
                } else {
                    bundle.stats.external_failed = bundle.stats.external_failed.saturating_add(1);
                }
            }
        }
        i = next_i;
    }
    bundle
}

fn stylesheet_urls(base_url: &str, body: &str) -> Vec<String> {
    let lower = lowercase_ascii(body);
    let mut out = Vec::new();
    let mut i = 0usize;
    while out.len() < MAX_STYLESHEET_SUBRESOURCES {
        let Some(rel) = lower[i..].find("<link") else {
            break;
        };
        let start = i + rel;
        let Some(end_rel) = find_tag_end(&body[start..]) else {
            break;
        };
        let tag = &body[start + 1..start + end_rel];
        let lower_tag = lowercase_ascii(tag.trim());
        if tag_name_of(&lower_tag) == "link"
            && link_rel_includes_stylesheet(tag)
            && !link_media_is_unsupported(tag)
        {
            if let Some(href) = attr_value(tag, "href") {
                let href = decode_entities(href.trim());
                if !href.is_empty() {
                    let url = resolve_url(base_url, &href);
                    if !out.iter().any(|existing| existing == &url) {
                        out.push(url);
                    }
                }
            }
        }
        i = start + end_rel + 1;
    }
    out
}

fn script_type_is_executable(tag: &str) -> bool {
    let Some(kind) = attr_value(tag, "type").or_else(|| attr_value(tag, "language")) else {
        return true;
    };
    let kind = lowercase_ascii(kind.trim());
    kind.is_empty()
        || kind == "javascript"
        || kind == "text/javascript"
        || kind == "application/javascript"
        || kind == "module"
        || kind == "text/ecmascript"
        || kind == "application/ecmascript"
}

fn script_url_allowed(base_url: &str, url: &str) -> bool {
    if url.starts_with("file://") {
        return base_url.starts_with("file://");
    }
    let Ok((scheme, host, _)) = parse_web_url(url) else {
        return false;
    };
    let Ok((base_scheme, base_host, _)) = parse_web_url(base_url) else {
        return false;
    };
    scheme == base_scheme && lowercase_ascii(&host) == lowercase_ascii(&base_host)
}

fn is_script_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("application/javascript")
                || value.eq_ignore_ascii_case("text/javascript")
                || value.eq_ignore_ascii_case("application/ecmascript")
                || value.eq_ignore_ascii_case("text/ecmascript")
                || value.eq_ignore_ascii_case("text/plain")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("js"))
}

fn is_main_script_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("application/javascript")
                || value.eq_ignore_ascii_case("text/javascript")
                || value.eq_ignore_ascii_case("application/ecmascript")
                || value.eq_ignore_ascii_case("text/ecmascript")
                || value.eq_ignore_ascii_case("application/x-javascript")
        })
        .unwrap_or_else(|| {
            let ext = extension_from_path(url);
            ext.eq_ignore_ascii_case("js") || ext.eq_ignore_ascii_case("mjs")
        })
}

fn link_rel_includes_stylesheet(tag: &str) -> bool {
    attr_value(tag, "rel")
        .map(|rel| {
            lowercase_ascii(&rel)
                .split_whitespace()
                .any(|part| part == "stylesheet")
        })
        .unwrap_or(false)
}

fn link_media_is_unsupported(tag: &str) -> bool {
    let Some(media) = attr_value(tag, "media") else {
        return false;
    };
    let media = lowercase_ascii(&media);
    !(media.trim().is_empty()
        || media
            .split(',')
            .any(|part| matches!(part.trim(), "all" | "screen")))
}

fn is_stylesheet_content(content_type: Option<&str>, url: &str) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            let value = value.trim();
            value.eq_ignore_ascii_case("text/css") || value.eq_ignore_ascii_case("text/plain")
        })
        .unwrap_or_else(|| extension_from_path(url).eq_ignore_ascii_case("css"))
}

fn attach_html_images_with_cache(
    lines: &mut Vec<BrowserLine>,
    inline_images: &mut Vec<InlineImage>,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    cols: usize,
    bypass_cache: bool,
) -> usize {
    inline_images.clear();
    let mut idx = 0usize;
    while idx < lines.len() && inline_images.len() < MAX_HTML_INLINE_IMAGES {
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
        match fetch_image_for_browser(&url, cache, stats, bypass_cache) {
            Ok(BrowserFetchedImage::Png {
                image,
                source_url,
                byte_len,
                cache_hit,
            }) => {
                let slot = inline_images.len();
                let rows = inline_image_reserved_rows_for(image.width, image.height, cols);
                lines[idx].image_slot = Some(slot);
                lines[idx].text = format!(
                    "[image] {}  {}x{}  {} bytes{}",
                    alt,
                    image.width,
                    image.height,
                    byte_len,
                    if cache_hit { " cached" } else { "" }
                );
                lines[idx].link = Some(source_url);
                inline_images.push(InlineImage { image });
                for _ in 1..rows {
                    lines.insert(idx + 1, inline_image_spacer(slot, &url));
                }
                idx += rows;
            }
            Ok(BrowserFetchedImage::Placeholder {
                label,
                source_url,
                byte_len,
                cache_hit,
            }) => {
                lines[idx].text = format!(
                    "[image] {}  {}  {} bytes  preview unavailable{}",
                    alt,
                    label,
                    byte_len,
                    if cache_hit { " cached" } else { "" }
                );
                lines[idx].link = Some(source_url);
                idx += 1;
            }
            Err(err) => {
                lines[idx].text = format!("{} ({})", lines[idx].text, err);
                idx += 1;
            }
        }
    }
    inline_images.len()
}

enum BrowserFetchedImage {
    Png {
        image: crate::png::PngImage,
        source_url: String,
        byte_len: usize,
        cache_hit: bool,
    },
    Placeholder {
        label: String,
        source_url: String,
        byte_len: usize,
        cache_hit: bool,
    },
}

fn fetch_image_for_browser(
    url: &str,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Result<BrowserFetchedImage, String> {
    let resource =
        fetch_subresource_with_cache(url, BrowserResourceKind::Image, cache, stats, bypass_cache)
            .map_err(String::from)?;
    let cache_hit = resource.cache_hit;
    if !is_image_content(resource.content_type.as_deref())
        && !looks_like_image_bytes(&resource.bytes)
        && !is_known_image_path(&resource.final_url)
    {
        stats.images_failed = stats.images_failed.saturating_add(1);
        return Err(String::from("preview skipped: response is not image data"));
    }
    if !is_png_content(resource.content_type.as_deref(), &resource.final_url) {
        let label = image_metadata_label(
            &resource.bytes,
            resource.content_type.as_deref(),
            &resource.final_url,
        )
        .unwrap_or_else(|| String::from("image metadata unavailable"));
        stats.image_placeholders = stats.image_placeholders.saturating_add(1);
        return Ok(BrowserFetchedImage::Placeholder {
            label,
            source_url: resource.final_url,
            byte_len: resource.bytes.len(),
            cache_hit,
        });
    }
    match crate::png::decode_rgb8(&resource.bytes, MAX_INLINE_PNG_PIXELS) {
        Ok(image) => {
            stats.images_loaded = stats.images_loaded.saturating_add(1);
            Ok(BrowserFetchedImage::Png {
                image,
                source_url: resource.final_url,
                byte_len: resource.bytes.len(),
                cache_hit,
            })
        }
        Err(err) => {
            stats.images_failed = stats.images_failed.saturating_add(1);
            Err(format!("PNG preview unavailable: {}", err))
        }
    }
}

fn fetch_subresource_with_cache(
    url: &str,
    kind: BrowserResourceKind,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Result<BrowserFetchedResource, &'static str> {
    if !bypass_cache {
        if let Some(resource) = cache.lookup(url, kind) {
            stats.note_cache(true);
            return Ok(resource);
        }
    }
    stats.note_cache(false);
    let resource = load_subresource_uncached(url, kind)?;
    cache.remember(
        url,
        kind,
        &resource.final_url,
        resource.content_type.clone(),
        &resource.bytes,
    );
    Ok(resource)
}

fn load_subresource_uncached(
    url: &str,
    kind: BrowserResourceKind,
) -> Result<BrowserFetchedResource, &'static str> {
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = crate::vfs::vfs_read_file(path).ok_or("subresource file missing")?;
        if bytes.len() > MAX_BROWSER_RESOURCE_BYTES {
            return Err("subresource too large");
        }
        let content_type = browser_resource_content_type(kind, path, &bytes);
        return Ok(BrowserFetchedResource {
            final_url: file_url_for_path(path),
            content_type,
            bytes,
            cache_hit: false,
        });
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("unsupported subresource URL");
    }
    let response = crate::net::browser_get_response(url)?;
    if response.body_bytes.len() > MAX_BROWSER_RESOURCE_BYTES {
        return Err("subresource too large");
    }
    Ok(BrowserFetchedResource {
        final_url: response.final_url,
        content_type: response.content_type,
        bytes: response.body_bytes,
        cache_hit: false,
    })
}

fn browser_resource_content_type(
    kind: BrowserResourceKind,
    path: &str,
    bytes: &[u8],
) -> Option<String> {
    match kind {
        BrowserResourceKind::Stylesheet => {
            if extension_from_path(path).eq_ignore_ascii_case("css") {
                Some(String::from("text/css"))
            } else {
                Some(String::from("text/plain"))
            }
        }
        BrowserResourceKind::Image => image_content_type_for(path, bytes).map(String::from),
        BrowserResourceKind::Script => {
            if extension_from_path(path).eq_ignore_ascii_case("js") {
                Some(String::from("application/javascript"))
            } else {
                Some(String::from("text/plain"))
            }
        }
    }
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
    } else if value.eq_ignore_ascii_case("text/css") {
        Some("css")
    } else if value.eq_ignore_ascii_case("application/javascript")
        || value.eq_ignore_ascii_case("text/javascript")
        || value.eq_ignore_ascii_case("application/ecmascript")
        || value.eq_ignore_ascii_case("text/ecmascript")
    {
        Some("js")
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
    } else if lower.ends_with(".css") {
        "css"
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "js"
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
    width: Option<CssLength>,
    height: Option<usize>,
    margin_top: Option<usize>,
    margin_right: Option<usize>,
    margin_bottom: Option<usize>,
    margin_left: Option<usize>,
    padding_top: Option<usize>,
    padding_right: Option<usize>,
    padding_bottom: Option<usize>,
    padding_left: Option<usize>,
    border_width: Option<usize>,
    border_color: Option<u32>,
    position: Option<CssPosition>,
    offset_top: Option<isize>,
    offset_right: Option<isize>,
    offset_bottom: Option<isize>,
    offset_left: Option<isize>,
    float_side: Option<CssFloat>,
    z_index: Option<i16>,
    list_style: Option<CssListStyle>,
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
    width: CssSlot<CssLength>,
    height: CssSlot<usize>,
    margin_top: CssSlot<usize>,
    margin_right: CssSlot<usize>,
    margin_bottom: CssSlot<usize>,
    margin_left: CssSlot<usize>,
    padding_top: CssSlot<usize>,
    padding_right: CssSlot<usize>,
    padding_bottom: CssSlot<usize>,
    padding_left: CssSlot<usize>,
    border_width: CssSlot<usize>,
    border_color: CssSlot<u32>,
    position: CssSlot<CssPosition>,
    offset_top: CssSlot<isize>,
    offset_right: CssSlot<isize>,
    offset_bottom: CssSlot<isize>,
    offset_left: CssSlot<isize>,
    float_side: CssSlot<CssFloat>,
    z_index: CssSlot<i16>,
    list_style: CssSlot<CssListStyle>,
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
        self.margin_top
            .apply(declarations.margin_top, specificity, order);
        self.margin_right
            .apply(declarations.margin_right, specificity, order);
        self.margin_bottom
            .apply(declarations.margin_bottom, specificity, order);
        self.margin_left
            .apply(declarations.margin_left, specificity, order);
        self.padding_top
            .apply(declarations.padding_top, specificity, order);
        self.padding_right
            .apply(declarations.padding_right, specificity, order);
        self.padding_bottom
            .apply(declarations.padding_bottom, specificity, order);
        self.padding_left
            .apply(declarations.padding_left, specificity, order);
        self.border_width
            .apply(declarations.border_width, specificity, order);
        self.border_color
            .apply(declarations.border_color, specificity, order);
        self.position
            .apply(declarations.position, specificity, order);
        self.offset_top
            .apply(declarations.offset_top, specificity, order);
        self.offset_right
            .apply(declarations.offset_right, specificity, order);
        self.offset_bottom
            .apply(declarations.offset_bottom, specificity, order);
        self.offset_left
            .apply(declarations.offset_left, specificity, order);
        self.float_side
            .apply(declarations.float_side, specificity, order);
        self.z_index.apply(declarations.z_index, specificity, order);
        self.list_style
            .apply(declarations.list_style, specificity, order);
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
    list_style: Option<CssListStyle>,
    preformatted: bool,
}

struct StyleHints {
    hidden_classes: Vec<String>,
    rules: Vec<CssRule>,
}

impl StyleHints {
    fn from_document_with_external_css(body: &str, external_css: &[String]) -> Self {
        let lower = lowercase_ascii(body);
        let mut hints = Self {
            hidden_classes: Vec::new(),
            rules: Vec::new(),
        };
        for css in external_css.iter().take(MAX_STYLESHEET_SUBRESOURCES) {
            let css = lowercase_ascii(css);
            collect_css_hints(&css, &mut hints);
        }
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
                box_style: BrowserBoxStyle {
                    margin_top: cascade.margin_top.value.unwrap_or(0).min(96),
                    margin_right: cascade.margin_right.value.unwrap_or(0).min(160),
                    margin_bottom: cascade.margin_bottom.value.unwrap_or(0).min(96),
                    margin_left: cascade.margin_left.value.unwrap_or(0).min(160),
                    padding_top: cascade.padding_top.value.unwrap_or(0).min(96),
                    padding_right: cascade.padding_right.value.unwrap_or(0).min(160),
                    padding_bottom: cascade.padding_bottom.value.unwrap_or(0).min(96),
                    padding_left: cascade.padding_left.value.unwrap_or(0).min(160),
                    border_width: cascade.border_width.value.unwrap_or(0).min(8),
                    border_color: cascade.border_color.value,
                    width: cascade.width.value,
                    height: cascade.height.value.map(|height| height.min(512)),
                },
                position: cascade.position.value.unwrap_or(CssPosition::Static),
                offset_top: cascade.offset_top.value.map(|value| value.clamp(-512, 512)),
                offset_right: cascade
                    .offset_right
                    .value
                    .map(|value| value.clamp(-512, 512)),
                offset_bottom: cascade
                    .offset_bottom
                    .value
                    .map(|value| value.clamp(-512, 512)),
                offset_left: cascade
                    .offset_left
                    .value
                    .map(|value| value.clamp(-512, 512)),
                float_side: cascade.float_side.value.unwrap_or(CssFloat::None),
                z_index: cascade.z_index.value.map(|value| value.clamp(-64, 64)),
                list_style: cascade.list_style.value.unwrap_or(CssListStyle::Disc),
            },
            width: match cascade.width.value {
                Some(CssLength::Px(width)) => Some(width),
                _ => None,
            },
            height: cascade.height.value,
            list_style: cascade.list_style.value,
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
                }
                if let Some([top, right, bottom, left]) = parse_css_box_lengths_px(&value) {
                    out.margin_top = Some(top);
                    out.margin_right = Some(right);
                    out.margin_bottom = Some(bottom);
                    out.margin_left = Some(left);
                }
            }
            "margin-top" => out.margin_top = parse_css_length_px(&value).or(out.margin_top),
            "margin-right" => out.margin_right = parse_css_length_px(&value).or(out.margin_right),
            "margin-bottom" => {
                out.margin_bottom = parse_css_length_px(&value).or(out.margin_bottom)
            }
            "margin-left" => {
                out.margin_left = parse_css_length_px(&value).or(out.margin_left);
            }
            "padding" => {
                if let Some([top, right, bottom, left]) = parse_css_box_lengths_px(&value) {
                    out.padding_top = Some(top);
                    out.padding_right = Some(right);
                    out.padding_bottom = Some(bottom);
                    out.padding_left = Some(left);
                }
            }
            "padding-top" => out.padding_top = parse_css_length_px(&value).or(out.padding_top),
            "padding-right" => {
                out.padding_right = parse_css_length_px(&value).or(out.padding_right)
            }
            "padding-bottom" => {
                out.padding_bottom = parse_css_length_px(&value).or(out.padding_bottom)
            }
            "padding-left" => {
                out.padding_left = parse_css_length_px(&value).or(out.padding_left);
            }
            "text-indent" => out.indent_px = parse_css_length_px(&value).or(out.indent_px),
            "color" => out.color = parse_css_color(&value).or(out.color),
            "background" | "background-color" => {
                out.background = parse_css_color(&value).or(out.background)
            }
            "width" | "max-width" => out.width = parse_css_length(&value).or(out.width),
            "height" | "max-height" => out.height = parse_css_length_px(&value).or(out.height),
            "border" => {
                if let Some(width) = first_css_length_px(&value) {
                    out.border_width = Some(width);
                }
                out.border_color = first_css_color(&value).or(out.border_color);
            }
            "border-width" => out.border_width = first_css_length_px(&value).or(out.border_width),
            "border-color" => out.border_color = first_css_color(&value).or(out.border_color),
            "border-style" => {
                if value != "none" && value != "hidden" && out.border_width.is_none() {
                    out.border_width = Some(1);
                }
            }
            "position" => {
                out.position = match value.as_str() {
                    "relative" => Some(CssPosition::Relative),
                    "absolute" => Some(CssPosition::Absolute),
                    "fixed" => Some(CssPosition::Fixed),
                    "sticky" => Some(CssPosition::Sticky),
                    "static" => Some(CssPosition::Static),
                    _ => out.position,
                };
            }
            "top" => out.offset_top = parse_css_signed_length_px(&value).or(out.offset_top),
            "right" => out.offset_right = parse_css_signed_length_px(&value).or(out.offset_right),
            "bottom" => {
                out.offset_bottom = parse_css_signed_length_px(&value).or(out.offset_bottom)
            }
            "left" => out.offset_left = parse_css_signed_length_px(&value).or(out.offset_left),
            "float" => {
                out.float_side = match value.as_str() {
                    "left" => Some(CssFloat::Left),
                    "right" => Some(CssFloat::Right),
                    "none" => Some(CssFloat::None),
                    _ => out.float_side,
                };
            }
            "z-index" => out.z_index = parse_css_integer_i16(&value).or(out.z_index),
            "list-style" | "list-style-type" => {
                out.list_style = parse_css_list_style(&value).or(out.list_style)
            }
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

fn parse_css_box_lengths_px(value: &str) -> Option<[usize; 4]> {
    let mut lengths = Vec::new();
    for part in value.split_whitespace().take(4) {
        if part == "auto" {
            lengths.push(0);
            continue;
        }
        lengths.push(parse_css_length_px(part)?);
    }
    match lengths.as_slice() {
        [a] => Some([*a, *a, *a, *a]),
        [a, b] => Some([*a, *b, *a, *b]),
        [a, b, c] => Some([*a, *b, *c, *b]),
        [a, b, c, d] => Some([*a, *b, *c, *d]),
        _ => None,
    }
}

fn first_css_length_px(value: &str) -> Option<usize> {
    for part in value.split_whitespace() {
        if let Some(length) = parse_css_length_px(part) {
            return Some(length);
        }
    }
    None
}

fn first_css_color(value: &str) -> Option<u32> {
    for part in value.split_whitespace() {
        if let Some(color) = parse_css_color(part) {
            return Some(color);
        }
    }
    None
}

fn parse_css_length(value: &str) -> Option<CssLength> {
    let value = value.trim();
    if value.is_empty() || value == "auto" {
        return None;
    }
    if let Some(percent) = value.strip_suffix('%') {
        return parse_css_number(percent).map(|number| CssLength::Percent(number.min(100) as u8));
    }
    parse_css_length_px(value).map(CssLength::Px)
}

fn parse_css_length_px(value: &str) -> Option<usize> {
    let value = value.trim();
    if value.is_empty() || value == "auto" || value.ends_with('%') {
        return None;
    }
    parse_css_number(value).map(|number| {
        if value.contains("em") || value.contains("rem") {
            number.saturating_mul(16).min(2048)
        } else {
            number.min(2048)
        }
    })
}

fn parse_css_signed_length_px(value: &str) -> Option<isize> {
    let value = value.trim();
    if value.is_empty() || value == "auto" || value.ends_with('%') {
        return None;
    }
    let negative = value.starts_with('-');
    let unsigned = value.trim_start_matches(|c| c == '+' || c == '-');
    let parsed = parse_css_length_px(unsigned)? as isize;
    Some(if negative { -parsed } else { parsed })
}

fn parse_css_integer_i16(value: &str) -> Option<i16> {
    let value = value.trim();
    if value == "auto" || value.is_empty() {
        return None;
    }
    let negative = value.starts_with('-');
    let unsigned = value.trim_start_matches(|c| c == '+' || c == '-');
    let mut number = 0i16;
    let mut saw_digit = false;
    for b in unsigned.bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        number = number.saturating_mul(10).saturating_add((b - b'0') as i16);
        saw_digit = true;
    }
    if !saw_digit {
        return None;
    }
    Some(if negative {
        number.saturating_neg()
    } else {
        number
    })
}

fn parse_css_list_style(value: &str) -> Option<CssListStyle> {
    for part in value.split_whitespace() {
        match part {
            "disc" => return Some(CssListStyle::Disc),
            "circle" => return Some(CssListStyle::Circle),
            "square" => return Some(CssListStyle::Square),
            "decimal" | "decimal-leading-zero" | "lower-roman" | "upper-roman" => {
                return Some(CssListStyle::Decimal)
            }
            "none" => return Some(CssListStyle::None),
            _ => {}
        }
    }
    None
}

fn parse_css_number(value: &str) -> Option<usize> {
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
    Some(number)
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
    render_document_core(base_url, response, cols, &[], None)
}

fn render_document_interactive(
    base_url: &str,
    response: &str,
    cols: usize,
    document: &BrowserDocumentState,
) -> Vec<BrowserLine> {
    let mut controls = BrowserRenderControls::new(document);
    render_document_core(
        base_url,
        response,
        cols,
        &document.external_css,
        Some(&mut controls),
    )
}

fn render_document_core(
    base_url: &str,
    response: &str,
    cols: usize,
    external_css: &[String],
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
    let style_hints = StyleHints::from_document_with_external_css(body, external_css);
    let lower_body = lowercase_ascii(body);
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
                let suppress_raw = is_raw_text_suppressed_element(lower_name);
                if suppress_raw
                    || ((tag_is_hidden(&lower_tag) || tag_style.hidden)
                        && !lower_tag.starts_with("input"))
                {
                    flush_flow_text(&mut out, &mut text, cols, &mut state);
                    if suppress_raw && !lower_tag.starts_with('/') {
                        i = skip_raw_text_element(body, &lower_body, i + end_rel + 1, lower_name);
                        continue;
                    } else if !is_void_element(lower_name) {
                        state.skip_until = Some(closing_tag_for(&lower_tag));
                    }
                    i += end_rel + 1;
                    continue;
                }
                if !lower_tag.starts_with('/') {
                    repair_html_before_start(&mut out, &mut text, cols, &mut state, lower_name);
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
    browser_style_debug_lines(render_document(base_url, response, cols))
}

fn browser_style_debug_lines(lines: Vec<BrowserLine>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| !line.text.is_empty())
        .map(|line| {
            let mut out = line.text;
            let visual_indent = line
                .style
                .indent_px
                .saturating_add(line.style.box_style.margin_left)
                .saturating_add(line.style.box_style.padding_left);
            if visual_indent > 0 {
                out.push_str(" [indent=");
                out.push_str(&format!("{}", visual_indent));
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
            push_box_style_debug(&mut out, line.style.box_style);
            push_flow_style_debug(&mut out, line.style);
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

fn push_box_style_debug(out: &mut String, style: BrowserBoxStyle) {
    if let Some(width) = style.width {
        out.push_str(" [box-w=");
        push_css_length_debug(out, width);
        out.push(']');
    }
    if let Some(height) = style.height {
        out.push_str(" [box-h=");
        out.push_str(&format!("{}", height));
        out.push(']');
    }
    if style.margin_top > 0
        || style.margin_right > 0
        || style.margin_bottom > 0
        || style.margin_left > 0
    {
        out.push_str(" [margin=");
        push_box_edges_debug(
            out,
            style.margin_top,
            style.margin_right,
            style.margin_bottom,
            style.margin_left,
        );
        out.push(']');
    }
    if style.padding_top > 0
        || style.padding_right > 0
        || style.padding_bottom > 0
        || style.padding_left > 0
    {
        out.push_str(" [pad=");
        push_box_edges_debug(
            out,
            style.padding_top,
            style.padding_right,
            style.padding_bottom,
            style.padding_left,
        );
        out.push(']');
    }
    if style.border_width > 0 {
        out.push_str(" [border=");
        out.push_str(&format!("{}", style.border_width));
        if let Some(color) = style.border_color {
            out.push(' ');
            push_hex_color(out, color);
        }
        out.push(']');
    }
}

fn push_flow_style_debug(out: &mut String, style: BrowserLineStyle) {
    if style.position != CssPosition::Static {
        out.push_str(" [pos=");
        push_position_debug(out, style.position);
        if let Some(left) = style.offset_left {
            out.push_str(" left=");
            out.push_str(&format!("{}", left));
        }
        if let Some(top) = style.offset_top {
            out.push_str(" top=");
            out.push_str(&format!("{}", top));
        }
        if let Some(right) = style.offset_right {
            out.push_str(" right=");
            out.push_str(&format!("{}", right));
        }
        if let Some(bottom) = style.offset_bottom {
            out.push_str(" bottom=");
            out.push_str(&format!("{}", bottom));
        }
        out.push(']');
    }
    if style.float_side != CssFloat::None {
        out.push_str(" [float=");
        push_float_debug(out, style.float_side);
        out.push(']');
    }
    if let Some(z_index) = style.z_index {
        out.push_str(" [z=");
        out.push_str(&format!("{}", z_index));
        out.push(']');
    }
    if style.list_style != CssListStyle::Disc {
        out.push_str(" [list=");
        push_list_style_debug(out, style.list_style);
        out.push(']');
    }
}

fn push_position_debug(out: &mut String, position: CssPosition) {
    out.push_str(match position {
        CssPosition::Static => "static",
        CssPosition::Relative => "relative",
        CssPosition::Absolute => "absolute",
        CssPosition::Fixed => "fixed",
        CssPosition::Sticky => "sticky",
    });
}

fn push_float_debug(out: &mut String, float_side: CssFloat) {
    out.push_str(match float_side {
        CssFloat::None => "none",
        CssFloat::Left => "left",
        CssFloat::Right => "right",
    });
}

fn push_list_style_debug(out: &mut String, list_style: CssListStyle) {
    out.push_str(match list_style {
        CssListStyle::Disc => "disc",
        CssListStyle::Circle => "circle",
        CssListStyle::Square => "square",
        CssListStyle::Decimal => "decimal",
        CssListStyle::None => "none",
    });
}

fn push_css_length_debug(out: &mut String, length: CssLength) {
    match length {
        CssLength::Px(px) => out.push_str(&format!("{}", px)),
        CssLength::Percent(percent) => {
            out.push_str(&format!("{}", percent));
            out.push('%');
        }
    }
}

fn push_box_edges_debug(out: &mut String, top: usize, right: usize, bottom: usize, left: usize) {
    out.push_str(&format!("{},{},{},{}", top, right, bottom, left));
}

pub fn render_document_box_debug_for_test(
    base_url: &str,
    response: &str,
    cols: usize,
    doc_w: usize,
) -> Vec<String> {
    let lines = render_document(base_url, response, cols);
    let layout = layout_browser_lines(&lines, &[], doc_w);
    layout
        .items
        .into_iter()
        .filter(|item| !item.text.is_empty())
        .map(|item| {
            let mut out = format!(
                "{} content={}x{} box={}x{} at {},{}",
                item.text, item.w, item.h, item.box_w, item.box_h, item.box_x, item.box_y
            );
            if item.style.position != CssPosition::Static {
                out.push_str(" pos=");
                push_position_debug(&mut out, item.style.position);
            }
            if item.style.float_side != CssFloat::None {
                out.push_str(" float=");
                push_float_debug(&mut out, item.style.float_side);
            }
            if item.z_index != 0 {
                out.push_str(" z=");
                out.push_str(&format!("{}", item.z_index));
            }
            out
        })
        .collect()
}

pub fn browser_subresource_debug_for_test(
    base_url: &str,
    response: &str,
    cols: usize,
) -> Vec<String> {
    let body = response_body_text(response).unwrap_or(response);
    let effective_base = extract_base_href(body, base_url);
    let mut cache = BrowserSubresourceCache::default();
    let mut stats = BrowserSubresourceStats::default();
    let external_css =
        load_document_stylesheets(&effective_base, body, &mut cache, &mut stats, false);
    let document =
        BrowserDocumentState::from_html_with_external_css(&effective_base, response, external_css);
    let mut first_lines =
        render_document_interactive(&document.base_url, &document.source, cols, &document);
    let mut inline_images = Vec::new();
    attach_html_images_with_cache(
        &mut first_lines,
        &mut inline_images,
        &mut cache,
        &mut stats,
        cols,
        false,
    );

    let second_css =
        load_document_stylesheets(&effective_base, body, &mut cache, &mut stats, false);
    let second_document =
        BrowserDocumentState::from_html_with_external_css(&effective_base, response, second_css);
    let mut second_lines = render_document_interactive(
        &second_document.base_url,
        &second_document.source,
        cols,
        &second_document,
    );
    let mut second_inline_images = Vec::new();
    attach_html_images_with_cache(
        &mut second_lines,
        &mut second_inline_images,
        &mut cache,
        &mut stats,
        cols,
        false,
    );

    let mut out = vec![format!(
        "stats css={}/{} images={} placeholders={} failed={} cache={}/{} entries={}",
        stats.stylesheets_loaded,
        stats
            .stylesheets_loaded
            .saturating_add(stats.stylesheets_failed),
        stats.images_loaded,
        stats.image_placeholders,
        stats.images_failed,
        stats.cache_hits,
        stats.cache_hits.saturating_add(stats.cache_misses),
        cache.entries().len()
    )];
    out.extend(browser_style_debug_lines(second_lines));
    out
}

pub fn browser_script_debug_for_test(base_url: &str, response: &str, cols: usize) -> Vec<String> {
    let body = response_body_text(response).unwrap_or(response);
    let effective_base = extract_base_href(body, base_url);
    let mut cache = BrowserSubresourceCache::default();
    let mut subresource_stats = BrowserSubresourceStats::default();
    let external_css = load_document_stylesheets(
        &effective_base,
        body,
        &mut cache,
        &mut subresource_stats,
        false,
    );
    let scripts = load_document_scripts(
        &effective_base,
        body,
        &mut cache,
        &mut subresource_stats,
        false,
    );
    let mut document = BrowserDocumentState::from_html_with_external_css_and_scripts(
        &effective_base,
        response,
        external_css,
        scripts.sources,
        scripts.stats,
    );
    let mut out = vec![script_stats_debug_line(document.script_stats)];
    out.extend(
        render_document_interactive(&document.base_url, &document.source, cols, &document)
            .into_iter()
            .filter(|line| !line.text.is_empty())
            .map(|line| line.text),
    );
    if let Some(run_id) = document
        .controls
        .iter()
        .position(|control| control.label == "Run")
    {
        let activation = document.activate_control(run_id);
        out.push(match activation {
            BrowserControlActivation::Changed => String::from("click=changed"),
            BrowserControlActivation::DomEvent(label) => format!("click=event {}", label),
            BrowserControlActivation::Navigate(url) => format!("click=navigate {}", url),
            BrowserControlActivation::Post { url, body } => {
                format!("click=post {} {}", url, body)
            }
            BrowserControlActivation::Focused => String::from("click=focused"),
            BrowserControlActivation::Ignored => String::from("click=ignored"),
        });
    }
    out.push(script_stats_debug_line(document.script_stats));
    if let Some(control) = document
        .controls
        .iter()
        .find(|control| control.name == "name")
    {
        out.push(format!("name={}", control.value));
    }
    if let Some(control) = document
        .controls
        .iter()
        .find(|control| control.name == "agree")
    {
        out.push(format!("agree={}", control.checked));
    }
    out.extend(
        render_document_interactive(&document.base_url, &document.source, cols, &document)
            .into_iter()
            .filter(|line| !line.text.is_empty())
            .map(|line| line.text),
    );
    out.push(
        document
            .submit_url_for_test("Send")
            .unwrap_or_else(|| String::from("submit missing")),
    );
    out
}

pub fn browser_web_api_debug_for_test(base_url: &str, response: &str, cols: usize) -> Vec<String> {
    let body = response_body_text(response).unwrap_or(response);
    let effective_base = extract_base_href(body, base_url);
    let mut cache = BrowserSubresourceCache::default();
    let mut subresource_stats = BrowserSubresourceStats::default();
    let external_css = load_document_stylesheets(
        &effective_base,
        body,
        &mut cache,
        &mut subresource_stats,
        false,
    );
    let scripts = load_document_scripts(
        &effective_base,
        body,
        &mut cache,
        &mut subresource_stats,
        false,
    );
    let document = BrowserDocumentState::from_html_with_external_css_and_scripts(
        &effective_base,
        response,
        external_css,
        scripts.sources,
        scripts.stats,
    );
    let mut out = vec![
        script_stats_debug_line(document.script_stats),
        format!("base={}", document.base_url),
        format!(
            "pending_nav={}",
            document
                .pending_navigation
                .clone()
                .unwrap_or_else(|| String::from("-"))
        ),
    ];
    out.extend(
        render_document_interactive(&document.base_url, &document.source, cols, &document)
            .into_iter()
            .filter(|line| !line.text.is_empty())
            .map(|line| line.text),
    );
    out
}

pub fn browser_compat_debug_for_test(base_url: &str, response: &str, cols: usize) -> Vec<String> {
    let body = response_body_text(response).unwrap_or(response);
    let effective_base = extract_base_href(body, base_url);
    if let Some(compat_body) = google_search_compat_document(&effective_base, body) {
        let document = BrowserDocumentState::from_html(&effective_base, &compat_body);
        let mut out = vec![
            String::from("mode=google-search"),
            format!("base={}", document.base_url),
        ];
        out.extend(
            render_document_interactive(&document.base_url, &document.source, cols, &document)
                .into_iter()
                .filter(|line| !line.text.is_empty())
                .map(|line| line.text),
        );
        return out;
    }
    let document = BrowserDocumentState::from_html(&effective_base, response);
    let mut out = vec![
        String::from("mode=native"),
        format!("base={}", document.base_url),
    ];
    out.extend(
        render_document_interactive(&document.base_url, &document.source, cols, &document)
            .into_iter()
            .filter(|line| !line.text.is_empty())
            .map(|line| line.text),
    );
    out
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

fn script_stats_debug_line(stats: BrowserScriptStats) -> String {
    format!(
        "js inline={} external={}/{} handlers={} timers={} mutations={} storage={}/{} cookies={}/{} fetch={} nav={} errors={} statements={}",
        stats.inline_scripts,
        stats.external_scripts,
        stats.external_scripts.saturating_add(stats.external_failed),
        stats.handlers,
        stats.timers,
        stats.mutations,
        stats.storage_reads,
        stats.storage_writes,
        stats.cookie_reads,
        stats.cookie_writes,
        stats.fetches,
        stats.navigation_requests,
        stats.errors,
        stats.statements
    )
}

fn split_script_statements(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && out.len() < MAX_SCRIPT_STATEMENTS {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => brace_depth = brace_depth.saturating_add(1),
            b'}' => brace_depth = brace_depth.saturating_sub(1),
            b';' if brace_depth == 0 => {
                out.push(String::from(input[start..i].trim()));
                start = i.saturating_add(1);
            }
            _ => {}
        }
        i += 1;
    }
    if start < input.len() && out.len() < MAX_SCRIPT_STATEMENTS {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            out.push(String::from(tail));
        }
    }
    out
}

fn script_statement_is_ignorable(statement: &str) -> bool {
    let trimmed = statement.trim();
    trimmed.is_empty()
        || trimmed == "\"use strict\""
        || trimmed == "'use strict'"
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || ((trimmed.starts_with("var ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const "))
            && !trimmed.contains('='))
}

fn compact_script_expr(input: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in input.chars() {
        if let Some(q) = quote {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            out.push(c);
        } else if !c.is_ascii_whitespace() {
            out.push(c);
        }
    }
    out
}

fn parse_add_event_listener(
    statement: &str,
) -> Option<(BrowserScriptTarget, BrowserScriptEvent, String)> {
    let marker = ".addEventListener";
    let pos = statement.find(marker)?;
    let target = parse_script_target(&statement[..pos])?;
    let args = &statement[pos + marker.len()..];
    let open = args.find('(')?;
    let args = &args[open + 1..];
    let (event_name, _) = parse_script_string_literal(args)?;
    let event = BrowserScriptEvent::from_name(&event_name)?;
    let body = extract_script_function_body(statement)?;
    Some((target, event, body))
}

fn split_script_assignment(statement: &str) -> Option<(&str, &str)> {
    let bytes = statement.as_bytes();
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'=' if depth == 0 => {
                let prev = i.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
                let next = bytes.get(i + 1).copied();
                if matches!(prev, Some(b'=' | b'!' | b'<' | b'>' | b'+') | Some(b'-'))
                    || matches!(next, Some(b'=' | b'>'))
                {
                    i += 1;
                    continue;
                }
                return Some((statement[..i].trim(), statement[i + 1..].trim()));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_script_assignment_left(
    left: &str,
) -> Option<(BrowserScriptTarget, BrowserScriptProperty)> {
    let compact = compact_script_expr(left);
    if let Some((target, property)) = parse_style_assignment_left(&compact) {
        return Some((target, BrowserScriptProperty::Style(property)));
    }
    for (suffix, property) in [
        (".textContent", BrowserScriptProperty::TextContent),
        (".innerText", BrowserScriptProperty::TextContent),
        (".className", BrowserScriptProperty::ClassName),
        (".value", BrowserScriptProperty::Value),
        (".checked", BrowserScriptProperty::Checked),
        (".disabled", BrowserScriptProperty::Disabled),
    ] {
        if let Some(target) = compact.strip_suffix(suffix) {
            return parse_script_target(target).map(|target| (target, property));
        }
    }
    None
}

fn parse_style_assignment_left(compact: &str) -> Option<(BrowserScriptTarget, String)> {
    let marker = ".style.";
    let pos = compact.rfind(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let property = css_property_from_js_name(&compact[pos + marker.len()..])?;
    Some((target, property))
}

fn parse_script_target(input: &str) -> Option<BrowserScriptTarget> {
    let compact = compact_script_expr(input);
    parse_script_call_string_arg(&compact, "document.getElementById")
        .map(BrowserScriptTarget::Id)
        .or_else(|| {
            parse_script_call_string_arg(&compact, "document.querySelector")
                .map(BrowserScriptTarget::Selector)
        })
        .or_else(|| parse_query_selector_all_target(&compact))
}

fn parse_query_selector_all_target(input: &str) -> Option<BrowserScriptTarget> {
    let close = input.find(")[")?;
    let target = &input[..close + 1];
    let selector = parse_script_call_string_arg(target, "document.querySelectorAll")?;
    let rest = &input[close + 2..];
    let end = rest.find(']')?;
    let index = rest[..end].parse::<usize>().ok()?;
    Some(BrowserScriptTarget::SelectorAll(selector, index))
}

fn parse_storage_call(compact: &str) -> Option<BrowserStorageCall> {
    let (area, rest) = strip_storage_area_prefix(compact)?;
    if let Some(args) = parse_method_args(rest, ".setItem") {
        let key = parse_script_string_literal(args.first()?.as_str())?.0;
        let value_expr = args.get(1)?.clone();
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::SetItem,
            key: Some(truncate_script_value(&key)),
            value_expr: Some(value_expr),
        });
    }
    if let Some(args) = parse_method_args(rest, ".removeItem") {
        let key = parse_script_string_literal(args.first()?.as_str())?.0;
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::RemoveItem,
            key: Some(truncate_script_value(&key)),
            value_expr: None,
        });
    }
    if rest == ".clear()" {
        return Some(BrowserStorageCall {
            area,
            method: BrowserStorageMethod::Clear,
            key: None,
            value_expr: None,
        });
    }
    None
}

fn parse_storage_get_item_expr(compact: &str) -> Option<(BrowserStorageArea, String)> {
    let (area, rest) = strip_storage_area_prefix(compact)?;
    let args = parse_method_args(rest, ".getItem")?;
    let key = parse_script_string_literal(args.first()?.as_str())?.0;
    Some((area, truncate_script_value(&key)))
}

fn strip_storage_area_prefix(input: &str) -> Option<(BrowserStorageArea, &str)> {
    for (prefix, area) in [
        ("localStorage", BrowserStorageArea::Local),
        ("window.localStorage", BrowserStorageArea::Local),
        ("sessionStorage", BrowserStorageArea::Session),
        ("window.sessionStorage", BrowserStorageArea::Session),
    ] {
        if let Some(rest) = input.strip_prefix(prefix) {
            return Some((area, rest));
        }
    }
    None
}

fn parse_class_list_call(compact: &str) -> Option<ClassListCall> {
    let marker = ".classList.";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let rest = &compact[pos + marker.len()..];
    for (name, op) in [
        ("add", ClassListOp::Add),
        ("remove", ClassListOp::Remove),
        ("toggle", ClassListOp::Toggle),
    ] {
        let Some(args) = parse_method_args(rest, name) else {
            continue;
        };
        let class_name = parse_script_string_literal(args.first()?.as_str())?.0;
        return Some(ClassListCall {
            target,
            op,
            class_name: truncate_script_value(&class_name),
        });
    }
    None
}

fn parse_attribute_call(compact: &str) -> Option<BrowserAttributeCall> {
    let marker = ".setAttribute";
    if let Some(pos) = compact.find(marker) {
        let target = parse_script_target(&compact[..pos])?;
        let args = parse_method_args(&compact[pos..], marker)?;
        let name = parse_script_string_literal(args.first()?.as_str())?.0;
        let value_expr = args.get(1)?.clone();
        return Some(BrowserAttributeCall {
            target,
            op: BrowserAttributeOp::Set,
            name: lowercase_ascii(&truncate_script_value(&name)),
            value_expr: Some(value_expr),
        });
    }
    let marker = ".removeAttribute";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let args = parse_method_args(&compact[pos..], marker)?;
    let name = parse_script_string_literal(args.first()?.as_str())?.0;
    Some(BrowserAttributeCall {
        target,
        op: BrowserAttributeOp::Remove,
        name: lowercase_ascii(&truncate_script_value(&name)),
        value_expr: None,
    })
}

fn parse_get_attribute_expr(compact: &str) -> Option<BrowserAttributeCall> {
    let marker = ".getAttribute";
    let pos = compact.find(marker)?;
    let target = parse_script_target(&compact[..pos])?;
    let args = parse_method_args(&compact[pos..], marker)?;
    let name = parse_script_string_literal(args.first()?.as_str())?.0;
    Some(BrowserAttributeCall {
        target,
        op: BrowserAttributeOp::Set,
        name: lowercase_ascii(&truncate_script_value(&name)),
        value_expr: None,
    })
}

fn parse_history_url_arg(compact: &str) -> Option<String> {
    let prefixes = [
        "history.pushState",
        "window.history.pushState",
        "history.replaceState",
        "window.history.replaceState",
        "location.assign",
        "window.location.assign",
        "location.replace",
        "window.location.replace",
    ];
    for prefix in prefixes {
        if let Some(args) = parse_method_args(compact, prefix) {
            if prefix.contains("history.") {
                return args.get(2).cloned();
            }
            return args.first().cloned();
        }
    }
    None
}

fn parse_fetch_request(compact: &str) -> Option<BrowserFetchRequest> {
    let args = if compact.starts_with("fetch(") {
        parse_method_args(compact, "fetch")?
    } else {
        parse_method_args(compact, "window.fetch")?
    };
    let url = parse_script_string_literal(args.first()?.as_str())?.0;
    let mut method = BrowserFetchMethod::Get;
    let mut body = String::new();
    if let Some(options) = args.get(1) {
        if parse_object_string_field(options, "method")
            .map(|value| value.eq_ignore_ascii_case("POST"))
            .unwrap_or(false)
        {
            method = BrowserFetchMethod::Post;
        }
        if let Some(value) = parse_object_string_field(options, "body") {
            body = truncate_script_value(&value);
        }
    }
    Some(BrowserFetchRequest {
        url: truncate_script_value(&url),
        method,
        body,
    })
}

fn parse_object_string_field(input: &str, key: &str) -> Option<String> {
    let compact = compact_script_expr(input);
    for quote in ["'", "\""] {
        let marker = format!("{}:{}", key, quote);
        if let Some(pos) = compact.find(&marker) {
            let value_start = pos + marker.len() - quote.len();
            return parse_script_string_literal(&compact[value_start..]).map(|(value, _)| value);
        }
    }
    None
}

fn load_fetch_post_uncached(url: &str, body: &str) -> Result<BrowserFetchedResource, &'static str> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("unsupported fetch POST URL");
    }
    let response =
        crate::net::browser_post_response(url, body, "application/x-www-form-urlencoded")?;
    Ok(BrowserFetchedResource {
        final_url: response.final_url,
        content_type: response.content_type,
        bytes: response.body_bytes,
        cache_hit: false,
    })
}

fn extract_fetch_text_callback_body(statement: &str) -> Option<String> {
    let compact = compact_script_expr(statement);
    let marker = ".text().then";
    let compact_pos = compact.find(marker)?;
    let mut seen_non_ws = 0usize;
    let mut original_pos = 0usize;
    for (idx, c) in statement.char_indices() {
        if !c.is_ascii_whitespace() {
            if seen_non_ws == compact_pos {
                original_pos = idx;
                break;
            }
            seen_non_ws += 1;
        }
    }
    extract_script_function_body(&statement[original_pos..])
}

fn parse_method_args(input: &str, method: &str) -> Option<Vec<String>> {
    let rest = input.strip_prefix(method)?;
    let rest = rest.strip_prefix('(')?;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 1usize;
    let bytes = rest.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(split_script_args(&rest[..idx]));
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn split_script_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let bytes = input.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                out.push(String::from(input[start..idx].trim()));
                start = idx.saturating_add(1);
            }
            _ => {}
        }
    }
    if start <= input.len() {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            out.push(String::from(tail));
        }
    }
    out
}

fn split_script_concat(input: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let bytes = input.as_bytes();
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' | b'[' | b'{' => depth = depth.saturating_add(1),
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'+' if depth == 0 => {
                out.push(String::from(input[start..idx].trim()));
                start = idx.saturating_add(1);
            }
            _ => {}
        }
    }
    if out.is_empty() {
        return None;
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        out.push(String::from(tail));
    }
    Some(out)
}

fn css_property_from_js_name(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() || input.len() > 48 {
        return None;
    }
    let mut out = String::new();
    for c in input.chars() {
        if c.is_ascii_uppercase() {
            if !out.is_empty() {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            out.push(c);
        } else {
            return None;
        }
    }
    Some(out)
}

fn valid_script_var_name(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && input.len() <= 32
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn set_style_declaration(style: &mut String, property: &str, value: &str) {
    let mut declarations: Vec<(String, String)> = Vec::new();
    let wanted = lowercase_ascii(property);
    let mut replaced = false;
    for part in style.split(';') {
        let Some((name, current)) = part.split_once(':') else {
            continue;
        };
        let name = lowercase_ascii(name.trim());
        if name.is_empty() {
            continue;
        }
        if name == wanted {
            declarations.push((name, String::from(value.trim())));
            replaced = true;
        } else {
            declarations.push((name, String::from(current.trim())));
        }
        if declarations.len() >= MAX_DOM_ATTRS {
            break;
        }
    }
    if !replaced && declarations.len() < MAX_DOM_ATTRS {
        declarations.push((wanted, String::from(value.trim())));
    }
    style.clear();
    for (idx, (name, value)) in declarations.iter().enumerate() {
        if idx > 0 {
            style.push(';');
        }
        style.push_str(name);
        style.push(':');
        style.push_str(value);
    }
}

fn style_declaration_value(style: &str, property: &str) -> Option<String> {
    let wanted = lowercase_ascii(property);
    for part in style.split(';') {
        let Some((name, value)) = part.split_once(':') else {
            continue;
        };
        if lowercase_ascii(name.trim()) == wanted {
            return Some(String::from(value.trim()));
        }
    }
    None
}

fn parse_script_call_string_arg(input: &str, name: &str) -> Option<String> {
    let rest = input.strip_prefix(name)?;
    let args = rest.strip_prefix('(')?.strip_suffix(')')?;
    parse_script_string_literal(args).map(|(value, _)| value)
}

fn parse_script_string_value(input: &str) -> Option<String> {
    parse_script_string_literal(input.trim()).map(|(value, _)| truncate_script_value(&value))
}

fn parse_script_bool_value(input: &str) -> Option<bool> {
    match lowercase_ascii(input.trim()).as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_script_string_literal(input: &str) -> Option<(String, usize)> {
    let input = input.trim_start();
    let bytes = input.as_bytes();
    let quote = *bytes.first()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == quote {
            return Some((out, i + 1));
        }
        if b == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'\'' => out.push('\''),
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                _ => out.push(next as char),
            }
            i += 2;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    None
}

fn extract_script_function_body(statement: &str) -> Option<String> {
    let bytes = statement.as_bytes();
    let mut open = None;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for (idx, b) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => {
                open = Some(idx);
                break;
            }
            _ => {}
        }
    }
    let open = open?;
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for idx in open..bytes.len() {
        let b = bytes[idx];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(String::from(&statement[open + 1..idx]));
                }
            }
            _ => {}
        }
    }
    None
}

fn truncate_script_value(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        if out.len().saturating_add(c.len_utf8()) > MAX_FORM_VALUE {
            break;
        }
        out.push(c);
    }
    out
}

fn push_html_text_escaped(out: &mut String, input: &str) {
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn push_html_attr_escaped(out: &mut String, input: &str) {
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
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
    list_style_stack: Vec<(String, CssListStyle)>,
    table_cell_align: BrowserAlign,
    open_elements: Vec<String>,
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
            list_style_stack: Vec::new(),
            table_cell_align: BrowserAlign::Left,
            open_elements: Vec::new(),
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

    fn current_list_style(&self) -> CssListStyle {
        self.list_style_stack
            .last()
            .map(|(_, style)| *style)
            .unwrap_or_else(|| {
                if self.ordered_stack.is_empty() {
                    CssListStyle::Disc
                } else {
                    CssListStyle::Decimal
                }
            })
    }

    fn push_list_style(&mut self, name: &str, style: CssListStyle) {
        self.list_style_stack.push((String::from(name), style));
    }

    fn pop_list_style(&mut self, name: &str) {
        if let Some(pos) = self
            .list_style_stack
            .iter()
            .rposition(|(tag_name, _)| tag_name == name)
        {
            self.list_style_stack.truncate(pos);
        }
    }

    fn push_open_element(&mut self, name: &str) {
        if self.open_elements.len() < 64 {
            self.open_elements.push(String::from(name));
        }
    }

    fn pop_open_element(&mut self, name: &str) {
        if let Some(pos) = self
            .open_elements
            .iter()
            .rposition(|tag_name| tag_name == name)
        {
            self.open_elements.truncate(pos);
        }
    }

    fn has_open_element(&self, name: &str) -> bool {
        self.open_elements.iter().any(|tag_name| tag_name == name)
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
        Self::from_html_with_external_css(base_url, response, Vec::new())
    }

    fn from_html_with_external_css(
        base_url: &str,
        response: &str,
        external_css: Vec<String>,
    ) -> Self {
        Self::from_html_with_external_css_and_scripts(
            base_url,
            response,
            external_css,
            Vec::new(),
            BrowserScriptStats::default(),
        )
    }

    fn from_html_with_external_css_and_scripts(
        base_url: &str,
        response: &str,
        external_css: Vec<String>,
        scripts: Vec<String>,
        script_stats: BrowserScriptStats,
    ) -> Self {
        let body = response_body_text(response).unwrap_or(response);
        let effective_base = extract_base_href(body, base_url);
        let mut state = Self {
            base_url: effective_base,
            source: String::from(body),
            external_css,
            dom: BrowserDomDocument::new(),
            forms: Vec::new(),
            controls: Vec::new(),
            script_handlers: Vec::new(),
            session_storage: Vec::new(),
            script_globals: Vec::new(),
            script_stats,
            pending_navigation: None,
            focused_control: None,
        };
        scan_dom_and_controls(body, &state.base_url.clone(), &mut state);
        state.finalize_select_values();
        state.collect_inline_event_handlers();
        state.execute_script_sources(&scripts);
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
        let changed = match control.kind {
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
        };
        if changed {
            match self.controls.get(id).map(|control| control.kind) {
                Some(BrowserFormControlKind::Checkbox | BrowserFormControlKind::Radio) => {
                    self.sync_control_dom_checked(id);
                }
                Some(_) => self.sync_control_dom_value(id),
                None => {}
            }
            self.sync_source_from_dom();
        }
        changed
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
                self.sync_control_dom_checked(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Radio => {
                self.set_radio_checked(id);
                self.sync_control_dom_checked(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Select => {
                self.select_next_option(id);
                self.sync_control_dom_value(id);
                let _ = self.run_control_event(id, BrowserScriptEvent::Change);
                BrowserControlActivation::Changed
            }
            BrowserFormControlKind::Submit | BrowserFormControlKind::Image => {
                let _ = self.run_control_event(id, BrowserScriptEvent::Click);
                let _ = self.run_control_event(id, BrowserScriptEvent::Submit);
                self.submission_for(id)
                    .unwrap_or(BrowserControlActivation::Ignored)
            }
            BrowserFormControlKind::Button => {
                let mutated = self.run_control_event(id, BrowserScriptEvent::Click);
                if mutated {
                    return BrowserControlActivation::Changed;
                }
                let label = self.controls[id].label.clone();
                BrowserControlActivation::DomEvent(label)
            }
            BrowserFormControlKind::Reset => {
                let _ = self.run_control_event(id, BrowserScriptEvent::Click);
                self.reset_form_for_control(id);
                self.sync_source_from_dom();
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
        let mut affected = Vec::new();
        for (idx, control) in self.controls.iter_mut().enumerate() {
            if control.kind == BrowserFormControlKind::Radio
                && control.form_id == target.form_id
                && !target.name.is_empty()
                && control.name == target.name
            {
                control.checked = false;
                affected.push(idx);
            }
        }
        if let Some(control) = self.controls.get_mut(id) {
            control.checked = true;
            if !affected.iter().any(|idx| *idx == id) {
                affected.push(id);
            }
        }
        for idx in affected {
            self.sync_control_dom_checked(idx);
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

    fn collect_inline_event_handlers(&mut self) {
        let mut handlers = Vec::new();
        for (node_id, node) in self.dom.nodes.iter().enumerate() {
            let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                continue;
            };
            for attr in attrs {
                let Some(event) = BrowserScriptEvent::from_attr(&attr.name) else {
                    continue;
                };
                if attr.value.trim().is_empty() || handlers.len() >= MAX_SCRIPT_EVENT_HANDLERS {
                    continue;
                }
                handlers.push(BrowserScriptHandler {
                    node_id,
                    event,
                    code: attr.value.clone(),
                });
            }
        }
        for handler in handlers {
            self.add_script_handler(handler.node_id, handler.event, handler.code);
        }
    }

    fn add_script_handler(&mut self, node_id: usize, event: BrowserScriptEvent, code: String) {
        if self.script_handlers.len() >= MAX_SCRIPT_EVENT_HANDLERS {
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
            return;
        }
        self.script_handlers.push(BrowserScriptHandler {
            node_id,
            event,
            code,
        });
        self.script_stats.handlers = self.script_stats.handlers.saturating_add(1);
    }

    fn execute_script_sources(&mut self, scripts: &[String]) {
        let before = self.script_stats.mutations;
        for script in scripts.iter().take(MAX_SCRIPT_SUBRESOURCES) {
            self.execute_script(script, 0);
        }
        if self.script_stats.mutations != before {
            self.sync_source_from_dom();
        }
    }

    fn execute_script(&mut self, code: &str, depth: usize) {
        self.execute_script_with_vars(code, depth, &[]);
    }

    fn execute_script_with_vars(&mut self, code: &str, depth: usize, vars: &[BrowserScriptVar]) {
        if depth > MAX_SCRIPT_RECURSION {
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
            return;
        }
        let statements = split_script_statements(code);
        for statement in statements.into_iter().take(MAX_SCRIPT_STATEMENTS) {
            if self.script_stats.statements >= MAX_SCRIPT_STATEMENTS {
                self.script_stats.errors = self.script_stats.errors.saturating_add(1);
                break;
            }
            let statement = statement.trim();
            if statement.is_empty() || script_statement_is_ignorable(statement) {
                continue;
            }
            self.script_stats.statements = self.script_stats.statements.saturating_add(1);
            if self.execute_script_statement(statement, depth, vars) {
                continue;
            }
            self.script_stats.errors = self.script_stats.errors.saturating_add(1);
        }
    }

    fn execute_script_statement(
        &mut self,
        statement: &str,
        depth: usize,
        vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(statement);
        if compact.starts_with("setTimeout(") || compact.starts_with("window.setTimeout(") {
            let Some(body) = extract_script_function_body(statement) else {
                return false;
            };
            self.script_stats.timers = self.script_stats.timers.saturating_add(1);
            self.execute_script_with_vars(&body, depth.saturating_add(1), vars);
            return true;
        }
        if self.execute_var_assignment(statement, vars) {
            return true;
        }
        if self.execute_fetch_statement(statement, depth, vars) {
            return true;
        }
        if self.execute_storage_statement(statement, vars) {
            return true;
        }
        if self.execute_class_list_statement(statement) {
            return true;
        }
        if self.execute_attribute_statement(statement, vars) {
            return true;
        }
        if self.execute_history_statement(statement, vars) {
            return true;
        }
        if let Some((target, event, body)) = parse_add_event_listener(statement) {
            let Some(node_id) = self.resolve_script_target(&target) else {
                return false;
            };
            self.add_script_handler(node_id, event, body);
            return true;
        }
        if let Some((left, right)) = split_script_assignment(statement) {
            if self.apply_global_script_assignment(left, right, vars) {
                return true;
            }
            let Some((target, property)) = parse_script_assignment_left(left) else {
                return false;
            };
            let Some(node_id) = self.resolve_script_target(&target) else {
                return false;
            };
            return self.apply_script_property(node_id, property, right, vars);
        }
        false
    }

    fn execute_var_assignment(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let trimmed = statement.trim();
        let rest = if let Some(rest) = trimmed.strip_prefix("var ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("let ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("const ") {
            rest
        } else {
            return false;
        };
        let Some((name, value_expr)) = split_script_assignment(rest) else {
            return false;
        };
        let name = name.trim();
        if !valid_script_var_name(name) {
            return false;
        }
        let Some(value) = self.script_string_value(value_expr, vars) else {
            return false;
        };
        self.set_script_global(name, &value);
        true
    }

    fn execute_fetch_statement(
        &mut self,
        statement: &str,
        depth: usize,
        _vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(statement);
        if !(compact.starts_with("fetch(") || compact.starts_with("window.fetch(")) {
            return false;
        }
        let Some(request) = parse_fetch_request(&compact) else {
            return false;
        };
        let url = resolve_url(&self.base_url, &request.url);
        if !script_url_allowed(&self.base_url, &url) {
            return false;
        }
        let resource = match request.method {
            BrowserFetchMethod::Get => load_subresource_uncached(&url, BrowserResourceKind::Script),
            BrowserFetchMethod::Post => load_fetch_post_uncached(&url, &request.body),
        };
        let Ok(resource) = resource else {
            return false;
        };
        if resource.bytes.len() > MAX_SCRIPT_FETCH_BYTES {
            return false;
        }
        let body = truncate_script_value(&String::from_utf8_lossy(&resource.bytes));
        self.script_stats.fetches = self.script_stats.fetches.saturating_add(1);
        if let Some(callback) = extract_fetch_text_callback_body(statement)
            .or_else(|| extract_script_function_body(statement))
        {
            let vars = vec![
                BrowserScriptVar {
                    name: String::from("text"),
                    value: body.clone(),
                },
                BrowserScriptVar {
                    name: String::from("body"),
                    value: body.clone(),
                },
                BrowserScriptVar {
                    name: String::from("responseText"),
                    value: body,
                },
            ];
            self.execute_script_with_vars(&callback, depth.saturating_add(1), &vars);
        }
        true
    }

    fn execute_storage_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_storage_call(&compact) else {
            return false;
        };
        match call.method {
            BrowserStorageMethod::SetItem => {
                let Some(key) = call.key else {
                    return false;
                };
                let Some(value_expr) = call.value_expr else {
                    return false;
                };
                let Some(value) = self.script_string_value(&value_expr, vars) else {
                    return false;
                };
                if self.set_script_storage(call.area, &key, &value) {
                    self.script_stats.storage_writes =
                        self.script_stats.storage_writes.saturating_add(1);
                    return true;
                }
            }
            BrowserStorageMethod::RemoveItem => {
                let Some(key) = call.key else {
                    return false;
                };
                if self.remove_script_storage(call.area, &key) {
                    self.script_stats.storage_writes =
                        self.script_stats.storage_writes.saturating_add(1);
                    return true;
                }
            }
            BrowserStorageMethod::Clear => {
                let changed = self.clear_script_storage(call.area);
                self.script_stats.storage_writes = self
                    .script_stats
                    .storage_writes
                    .saturating_add(changed.max(1));
                return true;
            }
        }
        false
    }

    fn execute_class_list_statement(&mut self, statement: &str) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_class_list_call(&compact) else {
            return false;
        };
        let Some(node_id) = self.resolve_script_target(&call.target) else {
            return false;
        };
        if self.mutate_node_class_list(node_id, &call.class_name, call.op) {
            self.note_script_mutation();
            return true;
        }
        false
    }

    fn execute_attribute_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(call) = parse_attribute_call(&compact) else {
            return false;
        };
        let Some(node_id) = self.resolve_script_target(&call.target) else {
            return false;
        };
        match call.op {
            BrowserAttributeOp::Set => {
                let Some(value_expr) = call.value_expr else {
                    return false;
                };
                let Some(value) = self.script_string_value(&value_expr, vars) else {
                    return false;
                };
                if self.set_node_attribute_from_script(node_id, &call.name, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserAttributeOp::Remove => {
                if self.remove_node_attribute_from_script(node_id, &call.name) {
                    self.note_script_mutation();
                    return true;
                }
            }
        }
        false
    }

    fn execute_history_statement(&mut self, statement: &str, vars: &[BrowserScriptVar]) -> bool {
        let compact = compact_script_expr(statement);
        let Some(url_expr) = parse_history_url_arg(&compact) else {
            return false;
        };
        let Some(url) = self.script_string_value(&url_expr, vars) else {
            return false;
        };
        let url = resolve_url(&self.base_url, &url);
        self.base_url = url;
        self.script_stats.navigation_requests =
            self.script_stats.navigation_requests.saturating_add(1);
        true
    }

    fn apply_global_script_assignment(
        &mut self,
        left: &str,
        right: &str,
        vars: &[BrowserScriptVar],
    ) -> bool {
        let compact = compact_script_expr(left);
        if compact == "document.cookie" {
            let Some(cookie) = self.script_string_value(right, vars) else {
                return false;
            };
            let Some((scheme, host, path)) = self.cookie_context() else {
                return false;
            };
            if crate::browser_session::set_document_cookie_for_context(
                &scheme, &host, &path, &cookie,
            ) {
                self.script_stats.cookie_writes = self.script_stats.cookie_writes.saturating_add(1);
                return true;
            }
            return false;
        }
        if matches!(
            compact.as_str(),
            "location.href" | "window.location.href" | "document.location.href" | "window.location"
        ) {
            let Some(url) = self.script_string_value(right, vars) else {
                return false;
            };
            self.pending_navigation = Some(resolve_url(&self.base_url, &url));
            self.script_stats.navigation_requests =
                self.script_stats.navigation_requests.saturating_add(1);
            return true;
        }
        false
    }

    fn resolve_script_target(&self, target: &BrowserScriptTarget) -> Option<usize> {
        match target {
            BrowserScriptTarget::Id(id) => self.find_node_by_id(id),
            BrowserScriptTarget::Selector(selector) => self.query_selector(selector),
            BrowserScriptTarget::SelectorAll(selector, index) => {
                self.query_selector_all(selector).get(*index).copied()
            }
        }
    }

    fn find_node_by_id(&self, wanted: &str) -> Option<usize> {
        self.dom
            .nodes
            .iter()
            .enumerate()
            .find_map(|(node_id, node)| {
                let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                    return None;
                };
                attrs
                    .iter()
                    .any(|attr| attr.name == "id" && attr.value == wanted)
                    .then_some(node_id)
            })
    }

    fn query_selector(&self, selector: &str) -> Option<usize> {
        self.query_selector_all(selector).into_iter().next()
    }

    fn query_selector_all(&self, selector: &str) -> Vec<usize> {
        let selector = selector.trim();
        if selector.is_empty() {
            return Vec::new();
        }
        if let Some(id) = selector.strip_prefix('#') {
            return self.find_node_by_id(id).into_iter().collect();
        }
        if let Some(class) = selector.strip_prefix('.') {
            return self
                .dom
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(node_id, node)| {
                    let BrowserDomNodeKind::Element { attrs, .. } = &node.kind else {
                        return None;
                    };
                    attrs
                        .iter()
                        .find(|attr| attr.name == "class")
                        .map(|attr| attr.value.split_whitespace().any(|value| value == class))
                        .unwrap_or(false)
                        .then_some(node_id)
                })
                .collect();
        }
        let wanted = lowercase_ascii(selector);
        self.dom
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(node_id, node)| {
                matches!(
                    &node.kind,
                    BrowserDomNodeKind::Element { name, .. } if name == &wanted
                )
                .then_some(node_id)
            })
            .collect()
    }

    fn apply_script_property(
        &mut self,
        node_id: usize,
        property: BrowserScriptProperty,
        value: &str,
        vars: &[BrowserScriptVar],
    ) -> bool {
        match property {
            BrowserScriptProperty::TextContent => {
                let Some(text) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_text_content(node_id, &text) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::ClassName => {
                let Some(class_name) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_attr(node_id, "class", &class_name) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Value => {
                let Some(value) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_value(node_id, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Checked => {
                let Some(checked) = parse_script_bool_value(value) else {
                    return false;
                };
                if self.set_node_checked(node_id, checked) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Disabled => {
                let Some(disabled) = parse_script_bool_value(value) else {
                    return false;
                };
                if self.set_node_disabled(node_id, disabled) {
                    self.note_script_mutation();
                    return true;
                }
            }
            BrowserScriptProperty::Style(property) => {
                let Some(value) = self.script_string_value(value, vars) else {
                    return false;
                };
                if self.set_node_style_property(node_id, &property, &value) {
                    self.note_script_mutation();
                    return true;
                }
            }
        }
        false
    }

    fn script_string_value(&mut self, input: &str, vars: &[BrowserScriptVar]) -> Option<String> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        if let Some(parts) = split_script_concat(input) {
            let mut out = String::new();
            for part in parts {
                let value = self.script_string_value(&part, vars)?;
                if out.len().saturating_add(value.len()) > MAX_FORM_VALUE {
                    let remaining = MAX_FORM_VALUE.saturating_sub(out.len());
                    out.push_str(&value[..value.len().min(remaining)]);
                    break;
                }
                out.push_str(&value);
            }
            return Some(out);
        }
        if let Some(value) = parse_script_string_value(input) {
            return Some(value);
        }
        let compact = compact_script_expr(input);
        if compact == "null" || compact == "undefined" {
            return Some(String::new());
        }
        if compact == "document.cookie" {
            let Some((scheme, host, path)) = self.cookie_context() else {
                return Some(String::new());
            };
            self.script_stats.cookie_reads = self.script_stats.cookie_reads.saturating_add(1);
            return Some(crate::browser_session::document_cookie_for_context(
                &scheme, &host, &path,
            ));
        }
        if matches!(
            compact.as_str(),
            "location.href" | "window.location.href" | "document.location.href"
        ) {
            return Some(self.base_url.clone());
        }
        if matches!(
            compact.as_str(),
            "location.search" | "window.location.search" | "document.location.search"
        ) {
            return Some(location_search(&self.base_url));
        }
        if let Some(value) = self.storage_get_from_expr(&compact) {
            return Some(value);
        }
        if let Some(value) = self.node_get_attribute_from_expr(&compact) {
            return Some(value);
        }
        if let Some(value) = self.node_property_value_from_expr(&compact) {
            return Some(value);
        }
        for var in vars.iter().take(MAX_SCRIPT_VARS) {
            if var.name == compact {
                return Some(truncate_script_value(&var.value));
            }
        }
        for var in self.script_globals.iter().take(MAX_SCRIPT_VARS) {
            if var.name == compact {
                return Some(truncate_script_value(&var.value));
            }
        }
        None
    }

    fn set_script_global(&mut self, name: &str, value: &str) {
        if let Some(var) = self.script_globals.iter_mut().find(|var| var.name == name) {
            var.value = truncate_script_value(value);
            return;
        }
        if self.script_globals.len() >= MAX_SCRIPT_VARS {
            self.script_globals.remove(0);
        }
        self.script_globals.push(BrowserScriptVar {
            name: String::from(name),
            value: truncate_script_value(value),
        });
    }

    fn storage_get_from_expr(&mut self, compact: &str) -> Option<String> {
        let (area, key) = parse_storage_get_item_expr(compact)?;
        let value = self
            .get_script_storage(area, &key)
            .unwrap_or_else(String::new);
        self.script_stats.storage_reads = self.script_stats.storage_reads.saturating_add(1);
        Some(value)
    }

    fn node_get_attribute_from_expr(&self, compact: &str) -> Option<String> {
        let call = parse_get_attribute_expr(compact)?;
        let node_id = self.resolve_script_target(&call.target)?;
        self.node_attr_value(node_id, &call.name)
            .or_else(|| Some(String::new()))
    }

    fn node_property_value_from_expr(&self, compact: &str) -> Option<String> {
        let (target, property) = parse_script_assignment_left(compact)?;
        let node_id = self.resolve_script_target(&target)?;
        match property {
            BrowserScriptProperty::TextContent => Some(self.node_text_content(node_id)),
            BrowserScriptProperty::ClassName => self
                .node_attr_value(node_id, "class")
                .or_else(|| Some(String::new())),
            BrowserScriptProperty::Value => Some(self.node_value(node_id)),
            BrowserScriptProperty::Checked => Some(
                self.control_for_node(node_id)
                    .and_then(|control_id| self.controls.get(control_id))
                    .map(|control| if control.checked { "true" } else { "false" })
                    .unwrap_or("false")
                    .into(),
            ),
            BrowserScriptProperty::Disabled => Some(
                self.control_for_node(node_id)
                    .and_then(|control_id| self.controls.get(control_id))
                    .map(|control| if control.disabled { "true" } else { "false" })
                    .unwrap_or("false")
                    .into(),
            ),
            BrowserScriptProperty::Style(property) => Some(
                self.node_style_property(node_id, &property)
                    .unwrap_or_else(String::new),
            ),
        }
    }

    fn cookie_context(&self) -> Option<(String, String, String)> {
        parse_web_url(&self.base_url).ok()
    }

    fn note_script_mutation(&mut self) {
        self.script_stats.mutations = self.script_stats.mutations.saturating_add(1);
    }

    fn set_node_text_content(&mut self, node_id: usize, text: &str) -> bool {
        if node_id >= self.dom.nodes.len() {
            return false;
        }
        let text = truncate_script_value(text);
        let first_text_child = self.dom.nodes[node_id]
            .children
            .iter()
            .copied()
            .find(|child| {
                matches!(
                    self.dom.nodes.get(*child).map(|node| &node.kind),
                    Some(BrowserDomNodeKind::Text(_))
                )
            });
        if let Some(child) = first_text_child {
            if let Some(BrowserDomNode {
                kind: BrowserDomNodeKind::Text(existing),
                ..
            }) = self.dom.nodes.get_mut(child)
            {
                *existing = text;
            }
            if let Some(node) = self.dom.nodes.get_mut(node_id) {
                node.children.clear();
                node.children.push(child);
            }
            return true;
        }
        if self.dom.nodes.len() >= MAX_DOM_NODES {
            return false;
        }
        if let Some(node) = self.dom.nodes.get_mut(node_id) {
            node.children.clear();
        }
        self.dom.push_text_raw(node_id, text);
        true
    }

    fn node_text_content(&self, node_id: usize) -> String {
        let mut out = String::new();
        self.push_node_text_content(node_id, &mut out);
        truncate_script_value(&out)
    }

    fn push_node_text_content(&self, node_id: usize, out: &mut String) {
        let Some(node) = self.dom.nodes.get(node_id) else {
            return;
        };
        match &node.kind {
            BrowserDomNodeKind::Text(text) => out.push_str(text),
            BrowserDomNodeKind::Element { .. } => {
                for child in &node.children {
                    self.push_node_text_content(*child, out);
                    if out.len() >= MAX_FORM_VALUE {
                        out.truncate(MAX_FORM_VALUE);
                        break;
                    }
                }
            }
        }
    }

    fn set_node_value(&mut self, node_id: usize, value: &str) -> bool {
        let value = truncate_script_value(value);
        if let Some(control_id) = self.control_for_node(node_id) {
            self.set_control_value(control_id, &value);
            return true;
        }
        self.set_node_attr(node_id, "value", &value)
    }

    fn node_value(&self, node_id: usize) -> String {
        if let Some(control_id) = self.control_for_node(node_id) {
            return self
                .controls
                .get(control_id)
                .map(|control| control.value.clone())
                .unwrap_or_else(String::new);
        }
        self.node_attr_value(node_id, "value")
            .unwrap_or_else(String::new)
    }

    fn set_node_checked(&mut self, node_id: usize, checked: bool) -> bool {
        let Some(control_id) = self.control_for_node(node_id) else {
            if checked {
                return self.set_node_attr(node_id, "checked", "");
            }
            return self.remove_node_attr(node_id, "checked");
        };
        if checked
            && self
                .controls
                .get(control_id)
                .map(|control| control.kind == BrowserFormControlKind::Radio)
                .unwrap_or(false)
        {
            self.set_radio_checked(control_id);
        } else if let Some(control) = self.controls.get_mut(control_id) {
            control.checked = checked;
        }
        self.sync_control_dom_checked(control_id);
        true
    }

    fn set_node_disabled(&mut self, node_id: usize, disabled: bool) -> bool {
        if let Some(control_id) = self.control_for_node(node_id) {
            if let Some(control) = self.controls.get_mut(control_id) {
                control.disabled = disabled;
            }
        }
        if disabled {
            self.set_node_attr(node_id, "disabled", "")
        } else {
            self.remove_node_attr(node_id, "disabled")
        }
    }

    fn set_node_attr(&mut self, node_id: usize, name: &str, value: &str) -> bool {
        let Some(BrowserDomNode {
            kind: BrowserDomNodeKind::Element { attrs, .. },
            ..
        }) = self.dom.nodes.get_mut(node_id)
        else {
            return false;
        };
        if let Some(attr) = attrs.iter_mut().find(|attr| attr.name == name) {
            attr.value = String::from(value);
            return true;
        }
        if attrs.len() >= MAX_DOM_ATTRS {
            return false;
        }
        attrs.push(BrowserDomAttr {
            name: String::from(name),
            value: String::from(value),
        });
        true
    }

    fn set_node_attribute_from_script(&mut self, node_id: usize, name: &str, value: &str) -> bool {
        let name = lowercase_ascii(name.trim());
        match name.as_str() {
            "class" => self.set_node_attr(node_id, "class", &truncate_script_value(value)),
            "style" => self.set_node_attr(node_id, "style", &truncate_script_value(value)),
            "value" => self.set_node_value(node_id, value),
            "checked" => self.set_node_checked(node_id, true),
            "disabled" => self.set_node_disabled(node_id, true),
            _ => self.set_node_attr(node_id, &name, &truncate_script_value(value)),
        }
    }

    fn remove_node_attr(&mut self, node_id: usize, name: &str) -> bool {
        let Some(BrowserDomNode {
            kind: BrowserDomNodeKind::Element { attrs, .. },
            ..
        }) = self.dom.nodes.get_mut(node_id)
        else {
            return false;
        };
        if let Some(pos) = attrs.iter().position(|attr| attr.name == name) {
            attrs.remove(pos);
        }
        true
    }

    fn remove_node_attribute_from_script(&mut self, node_id: usize, name: &str) -> bool {
        let name = lowercase_ascii(name.trim());
        match name.as_str() {
            "checked" => self.set_node_checked(node_id, false),
            "disabled" => self.set_node_disabled(node_id, false),
            _ => self.remove_node_attr(node_id, &name),
        }
    }

    fn node_attr_value(&self, node_id: usize, name: &str) -> Option<String> {
        let BrowserDomNodeKind::Element { attrs, .. } = &self.dom.nodes.get(node_id)?.kind else {
            return None;
        };
        attrs
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value.clone())
    }

    fn set_node_style_property(&mut self, node_id: usize, property: &str, value: &str) -> bool {
        let mut style = self
            .node_attr_value(node_id, "style")
            .unwrap_or_else(String::new);
        set_style_declaration(&mut style, property, &truncate_script_value(value));
        self.set_node_attr(node_id, "style", &style)
    }

    fn node_style_property(&self, node_id: usize, property: &str) -> Option<String> {
        let style = self.node_attr_value(node_id, "style")?;
        style_declaration_value(&style, property)
    }

    fn mutate_node_class_list(
        &mut self,
        node_id: usize,
        class_name: &str,
        op: ClassListOp,
    ) -> bool {
        if class_name.trim().is_empty() {
            return false;
        }
        let class_name = truncate_script_value(class_name.trim());
        let current = self
            .node_attr_value(node_id, "class")
            .unwrap_or_else(String::new);
        let mut classes: Vec<String> = current
            .split_whitespace()
            .map(String::from)
            .take(16)
            .collect();
        let exists = classes.iter().any(|class| class == &class_name);
        match op {
            ClassListOp::Add => {
                if !exists {
                    classes.push(class_name);
                }
            }
            ClassListOp::Remove => {
                classes.retain(|class| class != &class_name);
            }
            ClassListOp::Toggle => {
                if exists {
                    classes.retain(|class| class != &class_name);
                } else {
                    classes.push(class_name);
                }
            }
        }
        self.set_node_attr(node_id, "class", &classes.join(" "))
    }

    fn control_for_node(&self, node_id: usize) -> Option<usize> {
        self.controls
            .iter()
            .position(|control| control.dom_node == Some(node_id))
    }

    fn set_control_value(&mut self, control_id: usize, value: &str) {
        let Some(control) = self.controls.get_mut(control_id) else {
            return;
        };
        control.value = truncate_script_value(value);
        if control.kind == BrowserFormControlKind::Select {
            if let Some(pos) = control
                .options
                .iter()
                .position(|option| option.value == control.value || option.label == control.value)
            {
                control.selected = pos;
            }
        }
        self.sync_control_dom_value(control_id);
    }

    fn sync_control_dom_value(&mut self, control_id: usize) {
        let Some(control) = self.controls.get(control_id).cloned() else {
            return;
        };
        let Some(node_id) = control.dom_node else {
            return;
        };
        match control.kind {
            BrowserFormControlKind::Text
            | BrowserFormControlKind::Hidden
            | BrowserFormControlKind::Submit
            | BrowserFormControlKind::Button
            | BrowserFormControlKind::Reset
            | BrowserFormControlKind::Image
            | BrowserFormControlKind::Checkbox
            | BrowserFormControlKind::Radio => {
                let _ = self.set_node_attr(node_id, "value", &control.value);
            }
            BrowserFormControlKind::TextArea => {
                let _ = self.set_node_text_content(node_id, &control.value);
            }
            BrowserFormControlKind::Select => {
                for (idx, option_node) in self
                    .option_nodes_for_select(node_id)
                    .into_iter()
                    .enumerate()
                {
                    if idx == control.selected {
                        let _ = self.set_node_attr(option_node, "selected", "");
                    } else {
                        let _ = self.remove_node_attr(option_node, "selected");
                    }
                }
            }
        }
    }

    fn sync_control_dom_checked(&mut self, control_id: usize) {
        let Some(control) = self.controls.get(control_id).cloned() else {
            return;
        };
        let Some(node_id) = control.dom_node else {
            return;
        };
        if control.checked {
            let _ = self.set_node_attr(node_id, "checked", "");
        } else {
            let _ = self.remove_node_attr(node_id, "checked");
        }
    }

    fn option_nodes_for_select(&self, select_node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let Some(node) = self.dom.nodes.get(select_node) else {
            return out;
        };
        for child in &node.children {
            if matches!(
                self.dom.nodes.get(*child).map(|node| &node.kind),
                Some(BrowserDomNodeKind::Element { name, .. }) if name == "option"
            ) {
                out.push(*child);
            }
        }
        out
    }

    fn get_script_storage(&self, area: BrowserStorageArea, key: &str) -> Option<String> {
        match area {
            BrowserStorageArea::Local => {
                crate::browser_storage::local_get(&storage_origin_for_url(&self.base_url)?, key)
            }
            BrowserStorageArea::Session => self
                .session_storage
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| entry.value.clone()),
        }
    }

    fn set_script_storage(&mut self, area: BrowserStorageArea, key: &str, value: &str) -> bool {
        match area {
            BrowserStorageArea::Local => {
                let Some(origin) = storage_origin_for_url(&self.base_url) else {
                    return false;
                };
                crate::browser_storage::local_set(&origin, key, value)
            }
            BrowserStorageArea::Session => {
                if key.is_empty() || key.len() > 64 || value.len() > MAX_FORM_VALUE {
                    return false;
                }
                if let Some(entry) = self
                    .session_storage
                    .iter_mut()
                    .find(|entry| entry.key == key)
                {
                    entry.value = truncate_script_value(value);
                    return true;
                }
                if self.session_storage.len() >= MAX_SESSION_STORAGE_ENTRIES {
                    self.session_storage.remove(0);
                }
                self.session_storage.push(BrowserSessionStorageEntry {
                    key: String::from(key),
                    value: truncate_script_value(value),
                });
                true
            }
        }
    }

    fn remove_script_storage(&mut self, area: BrowserStorageArea, key: &str) -> bool {
        match area {
            BrowserStorageArea::Local => {
                let Some(origin) = storage_origin_for_url(&self.base_url) else {
                    return false;
                };
                crate::browser_storage::local_remove(&origin, key)
            }
            BrowserStorageArea::Session => {
                let Some(pos) = self
                    .session_storage
                    .iter()
                    .position(|entry| entry.key == key)
                else {
                    return false;
                };
                self.session_storage.remove(pos);
                true
            }
        }
    }

    fn clear_script_storage(&mut self, area: BrowserStorageArea) -> usize {
        match area {
            BrowserStorageArea::Local => storage_origin_for_url(&self.base_url)
                .map(|origin| crate::browser_storage::local_clear(&origin))
                .unwrap_or(0),
            BrowserStorageArea::Session => {
                let removed = self.session_storage.len();
                self.session_storage.clear();
                removed
            }
        }
    }

    fn run_control_event(&mut self, control_id: usize, event: BrowserScriptEvent) -> bool {
        let before = self.script_stats.mutations;
        if let Some(node_id) = self
            .controls
            .get(control_id)
            .and_then(|control| control.dom_node)
        {
            self.run_event_handlers(node_id, event);
        }
        if event == BrowserScriptEvent::Submit {
            if let Some(form_node) = self
                .controls
                .get(control_id)
                .and_then(|control| control.form_id)
                .and_then(|form_id| self.forms.get(form_id))
                .and_then(|form| form.dom_node)
            {
                self.run_event_handlers(form_node, event);
            }
        }
        if self.script_stats.mutations != before {
            self.sync_source_from_dom();
            true
        } else {
            false
        }
    }

    fn run_event_handlers(&mut self, node_id: usize, event: BrowserScriptEvent) {
        let handlers: Vec<String> = self
            .script_handlers
            .iter()
            .filter(|handler| handler.node_id == node_id && handler.event == event)
            .map(|handler| handler.code.clone())
            .collect();
        for code in handlers {
            self.execute_script(&code, 1);
        }
    }

    fn sync_source_from_dom(&mut self) {
        self.source = self.dom.to_html();
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

    fn push_text_raw(&mut self, parent: usize, text: String) {
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

    fn to_html(&self) -> String {
        let mut out = String::new();
        if let Some(root) = self.nodes.get(self.root) {
            for child in &root.children {
                self.push_node_html(*child, &mut out);
            }
        }
        out
    }

    fn push_node_html(&self, node_id: usize, out: &mut String) {
        let Some(node) = self.nodes.get(node_id) else {
            return;
        };
        match &node.kind {
            BrowserDomNodeKind::Text(text) => push_html_text_escaped(out, text),
            BrowserDomNodeKind::Element { name, attrs } => {
                out.push('<');
                out.push_str(name);
                for attr in attrs.iter().take(MAX_DOM_ATTRS) {
                    out.push(' ');
                    out.push_str(&attr.name);
                    if !attr.value.is_empty() {
                        out.push_str("=\"");
                        push_html_attr_escaped(out, &attr.value);
                        out.push('"');
                    }
                }
                out.push('>');
                if is_void_element(name) {
                    return;
                }
                for child in &node.children {
                    self.push_node_html(*child, out);
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
        }
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

enum BrowserScriptTarget {
    Id(String),
    Selector(String),
    SelectorAll(String, usize),
}

#[derive(Clone)]
enum BrowserScriptProperty {
    TextContent,
    ClassName,
    Value,
    Checked,
    Disabled,
    Style(String),
}

#[derive(Clone, Copy)]
enum BrowserStorageArea {
    Local,
    Session,
}

#[derive(Clone, Copy)]
enum BrowserStorageMethod {
    SetItem,
    RemoveItem,
    Clear,
}

struct BrowserStorageCall {
    area: BrowserStorageArea,
    method: BrowserStorageMethod,
    key: Option<String>,
    value_expr: Option<String>,
}

#[derive(Clone, Copy)]
enum ClassListOp {
    Add,
    Remove,
    Toggle,
}

struct ClassListCall {
    target: BrowserScriptTarget,
    op: ClassListOp,
    class_name: String,
}

#[derive(Clone, Copy)]
enum BrowserAttributeOp {
    Set,
    Remove,
}

struct BrowserAttributeCall {
    target: BrowserScriptTarget,
    op: BrowserAttributeOp,
    name: String,
    value_expr: Option<String>,
}

#[derive(Clone, Copy)]
enum BrowserFetchMethod {
    Get,
    Post,
}

struct BrowserFetchRequest {
    url: String,
    method: BrowserFetchMethod,
    body: String,
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
    let lower_body = lowercase_ascii(body);
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

                if is_raw_text_suppressed_element(name) {
                    i = skip_raw_text_element(body, &lower_body, i + end_rel + 1, name);
                    continue;
                }

                repair_dom_before_start(document, &mut stack, &mut names, name);
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
                                dom_node: Some(node),
                            });
                            form_stack.push(id);
                        }
                    }
                    "input" => {
                        push_document_input_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        );
                    }
                    "button" => {
                        active_button = push_document_button_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        )
                        .map(|control_id| (control_id, String::new()));
                    }
                    "select" => {
                        active_select = push_document_select_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
                        );
                    }
                    "textarea" => {
                        active_textarea = push_document_textarea_control(
                            document,
                            tag,
                            form_stack.last().copied(),
                            Some(node),
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

fn repair_dom_before_start(
    document: &mut BrowserDocumentState,
    stack: &mut Vec<usize>,
    names: &mut Vec<String>,
    name: &str,
) {
    if matches!(name, "td" | "th") {
        while names
            .last()
            .map(|open| open == "td" || open == "th")
            .unwrap_or(false)
        {
            stack.pop();
            names.pop();
        }
    }
    if name == "tr" {
        while names
            .last()
            .map(|open| open == "td" || open == "th")
            .unwrap_or(false)
        {
            stack.pop();
            names.pop();
        }
        while names.last().map(|open| open == "tr").unwrap_or(false) {
            stack.pop();
            names.pop();
        }
    }
    if name == "li" && names.iter().any(|open| open == "li") {
        pop_dom_stack(stack, names, "li");
    }
    if is_block_boundary(name) && names.iter().any(|open| open == "p") {
        pop_dom_stack(stack, names, "p");
    }
    if stack.is_empty() {
        stack.push(document.dom.root);
        names.push(String::from("document"));
    }
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
    dom_node: Option<usize>,
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
            dom_node,
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
    dom_node: Option<usize>,
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
            dom_node,
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
    dom_node: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "select");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
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
    dom_node: Option<usize>,
) -> Option<usize> {
    let label = form_control_label(tag, "textarea");
    push_document_control(
        document,
        BrowserFormControlState {
            form_id,
            dom_node,
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
    let fallback_value =
        control.default_value == control.label || control.default_value == "button";
    control.label = label.clone();
    if fallback_value {
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

fn repair_html_before_start(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
    name: &str,
) {
    if matches!(name, "td" | "th") && state.in_table_cell {
        finish_table_cell(state);
    }
    if name == "tr" {
        if state.in_table_cell {
            finish_table_cell(state);
        }
        if state.in_table && !state.table_row.is_empty() {
            finish_table_row(out, state, cols);
        }
    }
    if name == "li" && state.has_open_element("li") {
        close_implicit_element(out, text, cols, state, "li");
    }
    if is_block_boundary(name) && state.has_open_element("p") {
        close_implicit_element(out, text, cols, state, "p");
    }
}

fn close_implicit_element(
    out: &mut Vec<BrowserLine>,
    text: &mut String,
    cols: usize,
    state: &mut HtmlRenderState,
    name: &str,
) {
    flush_flow_text(out, text, cols, state);
    state.pop_align(name);
    state.pop_style(name);
    state.pop_list_style(name);
    state.pop_pre_style(name);
    state.pop_suppressed_text(name);
    state.pop_open_element(name);
    if matches!(name, "p" | "li") {
        push_blank_line(out);
    }
}

fn is_block_boundary(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
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
        state.pop_open_element(name);
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
            state.pop_list_style(name);
            state.pop_pre_style(name);
        } else if !is_void_element(name) {
            state.push_open_element(name);
            if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
                state.push_align(name, align);
            }
            state.push_style(name, tag_style.line);
            if let Some(list_style) = tag_style.list_style {
                state.push_list_style(name, list_style);
            }
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
        state.pop_list_style(name);
        state.pop_pre_style(name);
    } else if !is_void_element(name) {
        state.push_open_element(name);
        if let Some(align) = tag_style.align.or_else(|| tag_alignment(tag, name)) {
            state.push_align(name, align);
        }
    }
    if !closing && !is_void_element(name) {
        state.push_style(name, tag_style.line);
        if let Some(list_style) = tag_style.list_style {
            state.push_list_style(name, list_style);
        }
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
                push_input_line(out, tag, state, controls.as_deref_mut(), tag_style);
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
    tag_style: TagStyle,
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
        .styled(state.current_line_style().merged(tag_style.line))
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
    match state.current_list_style() {
        CssListStyle::Decimal => {
            if let Some(next) = state.ordered_stack.last_mut() {
                out.push_str(&format!("{}. ", *next));
                *next = next.saturating_add(1);
            } else {
                out.push_str("1. ");
            }
        }
        CssListStyle::Circle => out.push_str("o "),
        CssListStyle::Square => out.push_str("- "),
        CssListStyle::None => {}
        CssListStyle::Disc => {
            if let Some(next) = state.ordered_stack.last_mut() {
                out.push_str(&format!("{}. ", *next));
                *next = next.saturating_add(1);
            } else {
                out.push_str("* ");
            }
        }
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
    state.pop_style("td");
    state.pop_style("th");
    state.pop_list_style("td");
    state.pop_list_style("th");
    state.pop_pre_style("td");
    state.pop_pre_style("th");
    state.pop_open_element("td");
    state.pop_open_element("th");
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
    let widths = table_column_widths(cells, cols);
    let mut out = String::from("|");
    for (idx, cell) in cells.iter().enumerate() {
        out.push(' ');
        push_truncated_padded(
            &mut out,
            &cell.text,
            widths
                .get(idx)
                .copied()
                .unwrap_or_else(|| table_cell_width(cells.len(), cols)),
        );
        out.push(' ');
        out.push('|');
    }
    out
}

fn format_table_separator(cells: &[TableCell], cols: usize) -> String {
    let widths = table_column_widths(cells, cols);
    let mut out = String::from("+");
    for idx in 0..cells.len() {
        let width = widths
            .get(idx)
            .copied()
            .unwrap_or_else(|| table_cell_width(cells.len(), cols));
        for _ in 0..(width + 2) {
            out.push('-');
        }
        out.push('+');
    }
    out
}

fn table_column_widths(cells: &[TableCell], cols: usize) -> Vec<usize> {
    if cells.is_empty() {
        return Vec::new();
    }
    let chrome = cells.len().saturating_mul(3).saturating_add(1);
    let budget = cols
        .saturating_sub(chrome)
        .max(cells.len().saturating_mul(8));
    let mut widths = Vec::new();
    let mut used = 0usize;
    for cell in cells {
        let wanted = clean_inline_text(&cell.text).chars().count().clamp(8, 32);
        widths.push(wanted);
        used = used.saturating_add(wanted);
    }
    while used > budget {
        let Some((idx, width)) = widths
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, width)| (idx, *width))
        else {
            break;
        };
        if width <= 8 {
            break;
        }
        widths[idx] = widths[idx].saturating_sub(1);
        used = used.saturating_sub(1);
    }
    while used < budget {
        let Some((idx, width)) = widths
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, width)| (idx, *width))
        else {
            break;
        };
        if width >= 32 {
            break;
        }
        widths[idx] = widths[idx].saturating_add(1);
        used = used.saturating_add(1);
    }
    widths
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

fn is_raw_text_suppressed_element(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "noscript"
            | "svg"
            | "canvas"
            | "template"
            | "iframe"
            | "video"
            | "audio"
            | "object"
            | "embed"
            | "head"
    )
}

fn skip_raw_text_element(body: &str, lower_body: &str, content_start: usize, name: &str) -> usize {
    if is_void_element(name) {
        return content_start;
    }
    let mut close = String::from("</");
    close.push_str(name);
    if let Some(close_rel) = lower_body[content_start..].find(&close) {
        let close_start = content_start + close_rel;
        find_tag_end(&body[close_start..])
            .map(|close_end| close_start + close_end + 1)
            .unwrap_or(body.len())
    } else {
        body.len()
    }
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
    let cols = style.content_cols(cols.clamp(20, 120)).clamp(8, 120);
    let mut chunks = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if word.len() > cols {
            if !line.is_empty() {
                chunks.push(line);
                line = String::new();
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.len() >= cols {
                    chunks.push(chunk);
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
            chunks.push(line);
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        chunks.push(line);
    }
    let total = chunks.len();
    let mut out = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        let part = if total <= 1 || !style.box_style.has_layout() {
            BrowserLineBoxPart::Single
        } else if idx == 0 {
            BrowserLineBoxPart::First
        } else if idx + 1 == total {
            BrowserLineBoxPart::Last
        } else {
            BrowserLineBoxPart::Middle
        };
        out.push(
            BrowserLine::new(chunk, link.clone(), line_kind_for_link(&link, kind))
                .aligned(align)
                .styled(style)
                .with_box_part(part),
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

fn storage_origin_for_url(url: &str) -> Option<String> {
    if url.starts_with("file://") {
        return Some(String::from("file://"));
    }
    let Ok((scheme, host, _)) = parse_web_url(url) else {
        return None;
    };
    let mut out = scheme;
    out.push_str("://");
    out.push_str(&lowercase_ascii(&host));
    Some(out)
}

fn location_search(url: &str) -> String {
    let query_start = url.find('?');
    let Some(start) = query_start else {
        return String::new();
    };
    let end = url[start..]
        .find('#')
        .map(|rel| start + rel)
        .unwrap_or(url.len());
    String::from(&url[start..end])
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
