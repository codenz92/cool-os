use super::*;

pub(super) fn draw_menu_chevron(s: &mut [u32], sw: usize, x: i32, y: i32, color: u32) {
    s_fill(s, sw, x, y, 1, 2, color);
    s_fill(s, sw, x + 1, y + 1, 1, 2, color);
    s_fill(s, sw, x + 2, y + 2, 1, 2, color);
    s_fill(s, sw, x + 1, y + 3, 1, 2, color);
    s_fill(s, sw, x, y + 4, 1, 2, color);
}

pub(super) fn draw_menu_check(s: &mut [u32], sw: usize, x: i32, y: i32, color: u32) {
    s_fill(s, sw, x, y + 3, 2, 2, color);
    s_fill(s, sw, x + 2, y + 5, 2, 2, color);
    s_fill(s, sw, x + 4, y + 3, 2, 2, color);
    s_fill(s, sw, x + 6, y + 1, 2, 2, color);
}

pub(super) fn s_put(s: &mut [u32], sw: usize, sh: usize, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 {
        let (x, y) = (x as usize, y as usize);
        if x < sw && (sh == usize::MAX || y < sh) && y * sw + x < s.len() {
            s[y * sw + x] = color;
        }
    }
}

pub(super) fn s_fill(s: &mut [u32], sw: usize, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    let x0 = (x.max(0) as usize).min(sw);
    let y0 = y.max(0) as usize;
    let x1 = ((x + w).max(0) as usize).min(sw);
    let y1 = ((y + h).max(0) as usize).min(sh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for row in y0..y1 {
        let base = row * sw;
        s[base + x0..base + x1].fill(color);
    }
}

/// Additive-alpha fill — darkens existing pixels by a fraction derived from `shadow`'s alpha byte.
#[inline(always)]
pub(super) fn s_fill_alpha(s: &mut [u32], sw: usize, x: i32, y: i32, w: i32, h: i32, shadow: u32) {
    // Scale darkening by the alpha embedded in bits [31:24] of the colour word.
    let amount = ((shadow >> 24) & 0xFF) as u32;
    if amount == 0 {
        return;
    }
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    let x0 = (x.max(0) as usize).min(sw);
    let y0 = y.max(0) as usize;
    let x1 = ((x + w).max(0) as usize).min(sw);
    let y1 = ((y + h).max(0) as usize).min(sh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for row in y0..y1 {
        for col in x0..x1 {
            let idx = row * sw + col;
            let p = s[idx];
            let r = ((p >> 16) & 0xFF).saturating_sub(amount);
            let g = ((p >> 8) & 0xFF).saturating_sub(amount);
            let b = (p & 0xFF).saturating_sub(amount);
            s[idx] = (r << 16) | (g << 8) | b;
        }
    }
}

/// Render text at raw 1× scale (8 × 8 px per glyph) — used for compact labels.
pub(super) fn s_draw_char_small(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    c: char,
    fg: u32,
    bg: u32,
) {
    let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    let large_text = crate::accessibility::snapshot().large_text;
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            let ink = byte & (1 << bit) != 0;
            let color = if ink { fg } else { bg };
            let px = x + bit as i32;
            let py = y + gy as i32;
            if px >= 0 && py >= 0 {
                let (px, py) = (px as usize, py as usize);
                if px < sw && py < sh {
                    s[py * sw + px] = color;
                    if large_text && ink && px + 1 < sw {
                        s[py * sw + px + 1] = fg;
                    }
                }
            }
        }
    }
}

pub(super) fn s_draw_str_small(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: u32,
    bg: u32,
    max_x: i32,
) {
    let mut cx = x;
    for c in text.chars() {
        if cx + 8 > max_x {
            break;
        }
        s_draw_char_small(s, sw, cx, y, c, fg, bg);
        cx += 8;
    }
}

pub(super) fn s_draw_char_small_transparent(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    c: char,
    fg: u32,
) {
    let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
    let sh = if sw > 0 { s.len() / sw } else { 0 };
    let large_text = crate::accessibility::snapshot().large_text;
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            if byte & (1 << bit) == 0 {
                continue;
            }
            let px = x + bit as i32;
            let py = y + gy as i32;
            if px >= 0 && py >= 0 {
                let (px, py) = (px as usize, py as usize);
                if px < sw && py < sh {
                    s[py * sw + px] = fg;
                    if large_text && px + 1 < sw {
                        s[py * sw + px + 1] = fg;
                    }
                }
            }
        }
    }
}

pub(super) fn s_draw_str_small_transparent(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: u32,
    max_x: i32,
) {
    let mut cx = x;
    for c in text.chars() {
        if cx + 8 > max_x {
            break;
        }
        s_draw_char_small_transparent(s, sw, cx, y, c, fg);
        cx += 8;
    }
}

pub(super) fn draw_desktop_label(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: u32,
    max_x: i32,
) {
    let halo = 0x00_03_07_0D;
    let soft_halo = 0x00_0A_12_18;
    s_draw_str_small_transparent(s, sw, x + 1, y + 1, text, halo, max_x + 1);
    s_draw_str_small_transparent(s, sw, x, y + 1, text, soft_halo, max_x);
    s_draw_str_small_transparent(s, sw, x, y, text, fg, max_x);
}

pub(super) fn s_draw_str_scaled_with_tracking(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    scale: usize,
    tracking: i32,
) {
    let mut cx = x;
    for ch in text.chars() {
        s_draw_char_scaled(s, sw, cx, y, ch, color, scale);
        cx += 8 * scale as i32 + tracking;
    }
}

pub(super) fn s_text_width_scaled_with_tracking(text: &str, scale: usize, tracking: i32) -> i32 {
    let chars = text.chars().count() as i32;
    if chars == 0 {
        0
    } else {
        chars * (8 * scale as i32) + (chars - 1) * tracking
    }
}

pub(super) fn s_draw_char_scaled(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    c: char,
    color: u32,
    scale: usize,
) {
    let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            if byte & (1 << bit) == 0 {
                continue;
            }
            let px = x + (bit * scale) as i32;
            let py = y + (gy * scale) as i32;
            s_fill(s, sw, px, py, scale as i32, scale as i32, color);
        }
    }
}

/// Draw a 1-pixel-wide unfilled rectangle border.
pub(super) fn draw_rect_border(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
    s_fill(s, sw, x, y, w, 1, color); // top
    s_fill(s, sw, x, y + h - 1, w, 1, color); // bottom
    s_fill(s, sw, x, y, 1, h, color); // left
    s_fill(s, sw, x + w - 1, y, 1, h, color); // right
}

pub(super) fn draw_glass_panel_outline(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    accent: u32,
) {
    if w <= 1 || h <= 1 {
        return;
    }
    let outer = 0x00_3B_4C_5E;
    let inner = 0x00_18_25_32;
    let rim = 0x00_05_0A_10;
    draw_rect_border(s, sw, x, y, w, h, outer);
    if w > 2 && h > 2 {
        draw_rect_border(s, sw, x + 1, y + 1, w - 2, h - 2, inner);
        s_fill(s, sw, x + 1, y + h - 2, w - 2, 1, rim);
        s_fill(s, sw, x + w - 2, y + 1, 1, h - 2, rim);
    }
    s_fill(s, sw, x, y, w, 1, accent);
    s_fill(s, sw, x, y, 1, h, blend_color(accent, outer, 110));
}

pub(super) fn fill_vertical_gradient(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    top: u32,
    bottom: u32,
) {
    if w <= 0 || h <= 0 {
        return;
    }
    for row in 0..h {
        let t = if h > 1 {
            (row * 255 / (h - 1)) as u32
        } else {
            0
        };
        s_fill(s, sw, x, y + row, w, 1, blend_color(top, bottom, t));
    }
}

pub(super) fn draw_filled_circle(s: &mut [u32], sw: usize, cx: i32, cy: i32, r: i32, color: u32) {
    if r <= 0 {
        return;
    }
    for dy in -r..=r {
        let span_sq = r * r - dy * dy;
        let mut span = 0;
        while span * span <= span_sq {
            span += 1;
        }
        let span = span - 1;
        s_fill(s, sw, cx - span, cy + dy, span * 2 + 1, 1, color);
    }
}

pub(super) fn draw_icon_line(
    s: &mut [u32],
    sw: usize,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        s_put(s, sw, usize::MAX, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

pub(super) fn draw_icon_line_thick(
    s: &mut [u32],
    sw: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    draw_icon_line(s, sw, x0, y0, x1, y1, color);
    draw_icon_line(s, sw, x0 + 1, y0, x1 + 1, y1, color);
    draw_icon_line(s, sw, x0, y0 + 1, x1, y1 + 1, blend_color(color, BLACK, 28));
}

/// Blend two u32 colours: t=0 → a, t=255 → b.
#[inline(always)]
pub(super) fn blend_color(a: u32, b: u32, t: u32) -> u32 {
    let lerp = |ca: u32, cb: u32| -> u32 {
        if cb >= ca {
            (ca + (cb - ca) * t / 255).min(255)
        } else {
            ca - (ca - cb) * t / 255
        }
    };
    let r = lerp((a >> 16) & 0xFF, (b >> 16) & 0xFF);
    let g = lerp((a >> 8) & 0xFF, (b >> 8) & 0xFF);
    let bl = lerp(a & 0xFF, b & 0xFF);
    (r << 16) | (g << 8) | bl
}

pub(super) fn bilinear_u8(tl: u8, tr: u8, bl: u8, br: u8, tx: f32, ty: f32) -> u8 {
    let top = tl as f32 * (1.0 - tx) + tr as f32 * tx;
    let bot = bl as f32 * (1.0 - tx) + br as f32 * tx;
    (top * (1.0 - ty) + bot * ty) as u8
}
