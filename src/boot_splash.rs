extern crate alloc;

use core::{
    hint::spin_loop,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::framebuffer;

static DRAWN: AtomicBool = AtomicBool::new(false);
static LAST_PROGRESS_UNITS: AtomicUsize = AtomicUsize::new(0);

pub const BOOT_PROGRESS_TOTAL: usize = 24;

const PROGRESS_SUBSTEPS: usize = 4;
const FRAME_DELAY_SPINS: usize = 10_000;

const BG_TOP: u32 = 0x00_0B_10_1A;
const BG_BOTTOM: u32 = 0x00_05_0B_12;
const BG_SIDE: u32 = 0x00_16_18_25;
const GLOW: u32 = 0x00_2A_A7_A4;
const ACCENT: u32 = 0x00_2B_C8_E8;
const ACCENT_HOV: u32 = 0x00_7D_E7_F7;
const TITLE: u32 = 0x00_EB_F4_F8;
const BAR_BG: u32 = 0x00_12_1A_24;
const BAR_RIM: u32 = 0x00_2A_36_46;
const BAR_FILL: u32 = ACCENT;
const BAR_FILL_GLOW: u32 = ACCENT_HOV;

pub fn show(stage: &str, completed: usize, total: usize) {
    crate::boot_watchdog::record(stage, completed);
    crate::profiler::record_boot_stage(stage, completed);
    if !DRAWN.swap(true, Ordering::Relaxed) {
        draw_static();
    }

    let total_stages = total.max(1);
    let total_units = total_stages * PROGRESS_SUBSTEPS;
    let target_units = completed.min(total_stages) * PROGRESS_SUBSTEPS;
    let previous_units = LAST_PROGRESS_UNITS.swap(target_units, Ordering::Relaxed);

    if target_units > previous_units {
        for units in (previous_units + 1)..=target_units {
            draw_progress(stage, units, total_units, units);
            short_delay();
        }
    } else {
        draw_progress(stage, target_units, total_units, target_units);
    }
}

fn draw_static() {
    let w = framebuffer::width() as i32;
    let h = framebuffer::height() as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    let glow_cx = w / 2;
    let glow_cy = h / 2;
    let glow_rx = (w * 13 / 40).max(1);
    let glow_ry = (h * 7 / 20).max(1);

    for y in 0..h {
        let vertical = lerp_color(BG_TOP, BG_BOTTOM, y as usize, (h - 1).max(1) as usize);
        for x in 0..w {
            let edge = ((x - (w / 2)).abs() as i64 * 255 / (w / 2).max(1) as i64).min(255) as u8;
            let base = mix(vertical, BG_SIDE, edge / 6);
            let dx = (x - glow_cx).abs() as i64;
            let dy = (y - glow_cy).abs() as i64;
            let nx = dx * 255 / glow_rx as i64;
            let ny = dy * 255 / glow_ry as i64;
            let dist = (nx + ny).min(255) as u8;
            let glow = 255u8.saturating_sub(dist);
            let color = mix(base, GLOW, glow / 5);
            framebuffer::put_pixel(x as usize, y as usize, color);
        }
    }

    let logo_scale = 3;
    let title_scale = 4;
    let logo_size = 18 * logo_scale;
    let title_w = text_width_scaled_with_tracking("coolOS", title_scale as usize, 0);
    let title_h = 8 * title_scale;
    let lockup_gap = 24;
    let lockup_w = logo_size + lockup_gap + title_w;
    let lockup_x = (w - lockup_w) / 2;
    let lockup_y = h / 2 - 52;
    let title_x = lockup_x + logo_size + lockup_gap;
    let title_y = lockup_y + (logo_size - title_h) / 2;

    draw_logo_icon(lockup_x, lockup_y, logo_scale, BAR_FILL, BAR_FILL_GLOW);
    draw_str_scaled_with_tracking(title_x, title_y, "coolOS", TITLE, title_scale as usize, 0);
    draw_progress_line(0, BOOT_PROGRESS_TOTAL * PROGRESS_SUBSTEPS);
}

fn draw_progress(_stage: &str, completed_units: usize, total_units: usize, _phase: usize) {
    let w = framebuffer::width() as i32;
    let h = framebuffer::height() as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    draw_progress_line(completed_units, total_units);
}

fn draw_progress_line(completed_units: usize, total_units: usize) {
    let w = framebuffer::width() as i32;
    let h = framebuffer::height() as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    let bar_w = (w / 4).clamp(180, 360);
    let bar_h = 3;
    let bar_x = (w - bar_w) / 2;
    let bar_y = h / 2 + 46;

    fill_rect(bar_x - 1, bar_y - 1, bar_w + 2, bar_h + 2, BAR_RIM);
    fill_rect(bar_x, bar_y, bar_w, bar_h, BAR_BG);

    let fill = if total_units == 0 {
        bar_w
    } else {
        ((bar_w as i64 * completed_units.min(total_units) as i64) / total_units as i64) as i32
    };
    if fill > 0 {
        fill_rect(bar_x, bar_y, fill, bar_h, BAR_FILL);
        fill_rect(bar_x, bar_y, fill, 1, BAR_FILL_GLOW);

        let head_w = 10.min(fill);
        fill_rect(
            bar_x + fill - head_w,
            bar_y - 1,
            head_w,
            bar_h + 2,
            mix(BAR_FILL_GLOW, framebuffer::WHITE, 60),
        );
    }
}

fn draw_logo_icon(x: i32, y: i32, scale: i32, primary: u32, secondary: u32) {
    for rect in crate::branding::SNOWFLAKE_LOGO_RECTS.iter() {
        let color = if rect.highlight { secondary } else { primary };
        fill_rect(
            x + rect.x * scale,
            y + rect.y * scale,
            rect.w * scale,
            rect.h * scale,
            color,
        );
    }
}

fn fill_rect(x: i32, y: i32, w: i32, h: i32, color: u32) {
    let sw = framebuffer::width() as i32;
    let sh = framebuffer::height() as i32;
    if w <= 0 || h <= 0 || sw <= 0 || sh <= 0 {
        return;
    }
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(sw);
    let y1 = (y + h).min(sh);
    for py in y0..y1 {
        for px in x0..x1 {
            framebuffer::put_pixel(px as usize, py as usize, color);
        }
    }
}

fn draw_str_scaled_with_tracking(
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    scale: usize,
    tracking: i32,
) {
    let mut cx = x;
    for ch in text.chars() {
        draw_char_scaled(cx, y, ch, color, scale);
        cx += 8 * scale as i32 + tracking;
    }
}

fn text_width_scaled_with_tracking(text: &str, scale: usize, tracking: i32) -> i32 {
    let chars = text.chars().count() as i32;
    if chars == 0 {
        0
    } else {
        chars * (8 * scale as i32) + (chars - 1) * tracking
    }
}

fn draw_char_scaled(x: i32, y: i32, c: char, color: u32, scale: usize) {
    let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            if byte & (1 << bit) == 0 {
                continue;
            }
            let px = x + (bit * scale) as i32;
            let py = y + (gy * scale) as i32;
            fill_rect(px, py, scale as i32, scale as i32, color);
        }
    }
}

fn lerp_color(a: u32, b: u32, num: usize, den: usize) -> u32 {
    let den = den.max(1) as u32;
    let num = num.min(den as usize) as u32;
    let inv = den - num;
    let ar = (a >> 16) & 0xFF;
    let ag = (a >> 8) & 0xFF;
    let ab = a & 0xFF;
    let br = (b >> 16) & 0xFF;
    let bg = (b >> 8) & 0xFF;
    let bb = b & 0xFF;
    (((ar * inv + br * num) / den) << 16)
        | (((ag * inv + bg * num) / den) << 8)
        | ((ab * inv + bb * num) / den)
}

fn mix(base: u32, accent: u32, alpha: u8) -> u32 {
    let inv = 255u32 - alpha as u32;
    let alpha = alpha as u32;
    let br = (base >> 16) & 0xFF;
    let bg = (base >> 8) & 0xFF;
    let bb = base & 0xFF;
    let ar = (accent >> 16) & 0xFF;
    let ag = (accent >> 8) & 0xFF;
    let ab = accent & 0xFF;
    (((br * inv + ar * alpha) / 255) << 16)
        | (((bg * inv + ag * alpha) / 255) << 8)
        | ((bb * inv + ab * alpha) / 255)
}

fn short_delay() {
    for _ in 0..FRAME_DELAY_SPINS {
        spin_loop();
    }
}
