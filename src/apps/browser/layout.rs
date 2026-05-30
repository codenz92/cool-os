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
