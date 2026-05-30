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
