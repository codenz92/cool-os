pub const BG_TOP: u32 = 0x00_0B_10_1A;
pub const BG_BOTTOM: u32 = 0x00_05_08_10;
pub const BG_DEEP: u32 = 0x00_03_06_0C;
pub const PANEL: u32 = 0x00_10_17_22;
pub const PANEL_ALT: u32 = 0x00_14_20_2C;
pub const PANEL_SOFT: u32 = 0x00_0C_13_1D;
pub const FIELD: u32 = 0x00_08_0E_16;
pub const BORDER: u32 = 0x00_2A_3A_4A;
pub const BORDER_SOFT: u32 = 0x00_1A_25_32;
pub const ACCENT: u32 = 0x00_2B_C8_E8;
pub const ACCENT_HOVER: u32 = 0x00_7D_E7_F7;
pub const ACCENT_ALT: u32 = 0x00_4C_DD_A1;
pub const SELECTION: u32 = 0x00_1A_2A_38;
pub const SELECTION_GLOW: u32 = 0x00_25_96_BA;
pub const TEXT: u32 = 0x00_E7_EF_F6;
pub const TEXT_MUTED: u32 = 0x00_90_A4_B8;
pub const TEXT_DIM: u32 = 0x00_6F_81_92;
pub const SUCCESS: u32 = 0x00_4C_DD_A1;
pub const WARNING: u32 = 0x00_F5_CB_6B;
pub const DANGER: u32 = 0x00_EA_6B_6B;

pub const CONTROL_FILL: u32 = PANEL_ALT;
pub const CONTROL_HOVER: u32 = SELECTION;
pub const CONTROL_PRESSED: u32 = PANEL_SOFT;
pub const CONTROL_DISABLED: u32 = CONTROL_PRESSED;
pub const DIVIDER: u32 = BORDER_SOFT;
pub const CARD_SURFACE: u32 = PANEL;
pub const CARD_HOVER: u32 = CONTROL_HOVER;
pub const INPUT_FOCUS: u32 = ACCENT;
pub const TEXT_ON_ACCENT: u32 = 0x00_02_08_10;
pub const STATUS_INFO: u32 = ACCENT_HOVER;
pub const STATUS_SUCCESS: u32 = SUCCESS;
pub const STATUS_WARNING: u32 = WARNING;
pub const STATUS_DANGER: u32 = DANGER;

pub fn mix_color(a: u32, b: u32, t: u32) -> u32 {
    let t = t.min(255);
    let inv = 255u32.saturating_sub(t);
    let ar = (a >> 16) & 0xFF;
    let ag = (a >> 8) & 0xFF;
    let ab = a & 0xFF;
    let br = (b >> 16) & 0xFF;
    let bg = (b >> 8) & 0xFF;
    let bb = b & 0xFF;
    let r = (ar * inv + br * t) / 255;
    let g = (ag * inv + bg * t) / 255;
    let blue = (ab * inv + bb * t) / 255;
    (r << 16) | (g << 8) | blue
}

pub fn app_background_at(row: usize, height: usize) -> u32 {
    let height = height.max(1);
    let top_span = (height / 3).max(1);
    let mid_span = (height / 3).max(1);
    if row < top_span {
        mix_color(
            BG_DEEP,
            BG_TOP,
            (row as u32).saturating_mul(255) / top_span as u32,
        )
    } else if row < top_span + mid_span {
        let pos = row.saturating_sub(top_span);
        mix_color(
            BG_TOP,
            FIELD,
            (pos as u32).saturating_mul(255) / mid_span as u32,
        )
    } else {
        let pos = row.saturating_sub(top_span + mid_span);
        let span = height.saturating_sub(top_span + mid_span).max(1);
        mix_color(
            FIELD,
            BG_BOTTOM,
            (pos as u32).saturating_mul(255) / span as u32,
        )
    }
}

pub fn fill_app_background(buf: &mut [u32], stride: usize, height: usize) {
    if stride == 0 {
        return;
    }
    for row in 0..height {
        let color = app_background_at(row, height);
        let start = row.saturating_mul(stride);
        if start >= buf.len() {
            break;
        }
        let end = start.saturating_add(stride).min(buf.len());
        for pixel in &mut buf[start..end] {
            *pixel = color;
        }
    }
}

pub fn fill_rect(
    buf: &mut [u32],
    stride: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if stride == 0 || w == 0 || h == 0 {
        return;
    }
    for row in y..y.saturating_add(h).min(height) {
        let start = row.saturating_mul(stride).saturating_add(x.min(stride));
        if start >= buf.len() {
            break;
        }
        let end = row
            .saturating_mul(stride)
            .saturating_add(x.saturating_add(w).min(stride))
            .min(buf.len());
        for pixel in &mut buf[start..end] {
            *pixel = color;
        }
    }
}

pub fn draw_rect_border(
    buf: &mut [u32],
    stride: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if w == 0 || h == 0 {
        return;
    }
    fill_rect(buf, stride, height, x, y, w, 1, color);
    fill_rect(buf, stride, height, x, y + h.saturating_sub(1), w, 1, color);
    fill_rect(buf, stride, height, x, y, 1, h, color);
    fill_rect(buf, stride, height, x + w.saturating_sub(1), y, 1, h, color);
}

pub fn draw_glass_panel(
    buf: &mut [u32],
    stride: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    accent: u32,
) {
    if w == 0 || h == 0 {
        return;
    }
    fill_rect(buf, stride, height, x, y, w, h, CARD_SURFACE);
    if h > 1 {
        fill_rect(buf, stride, height, x, y, w, 1, accent);
    }
    draw_rect_border(buf, stride, height, x, y, w, h, BORDER);
    if w > 2 && h > 2 {
        draw_rect_border(buf, stride, height, x + 1, y + 1, w - 2, h - 2, DIVIDER);
    }
}

pub fn draw_control(
    buf: &mut [u32],
    stride: usize,
    height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    active: bool,
) {
    let fill = if active { CONTROL_HOVER } else { CONTROL_FILL };
    let border = if active { INPUT_FOCUS } else { BORDER };
    fill_rect(buf, stride, height, x, y, w, h, fill);
    draw_rect_border(buf, stride, height, x, y, w, h, border);
    if h > 2 {
        fill_rect(
            buf,
            stride,
            height,
            x,
            y,
            w,
            1,
            if active { ACCENT_HOVER } else { ACCENT },
        );
    }
}
