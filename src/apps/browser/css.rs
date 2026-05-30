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
