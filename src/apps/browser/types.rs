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
