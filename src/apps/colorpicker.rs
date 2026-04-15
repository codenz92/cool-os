/// Color Picker — shows all 16 VGA palette colours as clickable swatches.
/// Click a swatch to select it; the status bar shows its name and index.

use font8x8::UnicodeFonts;
use crate::framebuffer::{CHAR_W, WHITE, LIGHT_GRAY, DARK_GRAY};
use crate::wm::window::{Window, TITLE_H};

pub const PICKER_W: i32 = 148;
pub const PICKER_H: i32 = 100;

const SWATCH: i32  = 16;   // pixels per swatch (square)
const GAP:    i32  = 3;    // pixels between swatches
const STEP:   i32  = SWATCH + GAP;
const GRID_X: i32  = 10;   // content-area origin
const GRID_Y: i32  = 8;
const COLS:   i32  = 8;    // 8 wide × 2 tall = 16 colours

const COLORS: [(&str, u8); 16] = [
    ("Black",    0),  ("Blue",     1),  ("Green",    2),  ("Cyan",     3),
    ("Red",      4),  ("Magenta",  5),  ("Brown",    6),  ("Lt Gray",  7),
    ("Dk Gray",  8),  ("Lt Blue",  9),  ("Lt Green", 10), ("Lt Cyan",  11),
    ("Lt Red",   12), ("Pink",     13), ("Yellow",   14), ("White",    15),
];

pub struct ColorPickerApp {
    pub window:   Window,
    selected:     Option<usize>,
}

impl ColorPickerApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, PICKER_W, PICKER_H, "Color Picker");
        let mut app = ColorPickerApp { window, selected: None };
        app.render();
        app
    }

    /// Called when the user clicks in the content area
    /// `(lx, ly)` are coordinates relative to the content area top-left.
    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        let col = (lx - GRID_X) / STEP;
        let row = (ly - GRID_Y) / STEP;
        if col >= 0 && col < COLS && row >= 0 && row < 2 {
            let idx = (row * COLS + col) as usize;
            if idx < 16 {
                self.selected = Some(idx);
                self.render();
            }
        }
    }

    fn render(&mut self) {
        let stride = PICKER_W as usize;
        let content_h = (PICKER_H - TITLE_H) as usize;
        for b in self.window.buf.iter_mut() { *b = DARK_GRAY; }

        // Draw swatches.
        for i in 0..16usize {
            let col = (i % COLS as usize) as i32;
            let row = (i / COLS as usize) as i32;
            let x  = GRID_X + col * STEP;
            let y  = GRID_Y + row * STEP;
            let color = COLORS[i].1;
            let selected = self.selected == Some(i);

            // Selection highlight (1-px border in WHITE).
            if selected {
                fill_buf(&mut self.window.buf, stride, content_h,
                         x - 1, y - 1, SWATCH + 2, SWATCH + 2, WHITE);
            }
            fill_buf(&mut self.window.buf, stride, content_h, x, y, SWATCH, SWATCH, color);
        }

        // Status bar — show selected colour name.
        let status_y = (GRID_Y + 2 * STEP + GAP) as usize;
        let status_py = status_y;
        if let Some(idx) = self.selected {
            let (name, palette) = COLORS[idx];
            let mut line = [b' '; 20];
            // Compose "Name (N)"
            let mut pos = 0;
            for b in name.bytes() { if pos < 20 { line[pos] = b; pos += 1; } }
            let suffix = b" (xx)";
            // Write palette index
            let mut tmp = [0u8; 4];
            let s = usize_to_buf(palette as usize, &mut tmp);
            if pos < 20 { line[pos] = b' '; pos += 1; }
            if pos < 20 { line[pos] = b'('; pos += 1; }
            for &b in s { if pos < 20 { line[pos] = b; pos += 1; } }
            if pos < 20 { line[pos] = b')'; pos += 1; }
            let _ = suffix;

            for (ci, &b) in line[..pos].iter().enumerate() {
                let px = ci * CHAR_W;
                if px + CHAR_W > stride { break; }
                put_char(&mut self.window.buf, stride, px, status_py, b as char,
                         WHITE, DARK_GRAY);
            }
        } else {
            // Hint text.
            let hint = "Click a colour";
            for (ci, c) in hint.chars().enumerate() {
                let px = ci * CHAR_W;
                if px + CHAR_W > stride { break; }
                put_char(&mut self.window.buf, stride, px, status_py, c,
                         LIGHT_GRAY, DARK_GRAY);
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fill_buf(buf: &mut [u8], stride: usize, content_h: usize,
            x: i32, y: i32, w: i32, h: i32, color: u8)
{
    let x0 = (x.max(0) as usize).min(stride);
    let y0 = (y.max(0) as usize).min(content_h);
    let x1 = ((x + w).max(0) as usize).min(stride);
    let y1 = ((y + h).max(0) as usize).min(content_h);
    if x0 >= x1 || y0 >= y1 { return; }
    for row in y0..y1 {
        let base = row * stride;
        buf[base + x0..base + x1].fill(color);
    }
}

fn put_char(buf: &mut [u8], stride: usize, px: usize, py: usize,
            c: char, fg: u8, bg: u8)
{
    let glyph = font8x8::BASIC_FONTS
        .get(c)
        .unwrap_or_else(|| font8x8::BASIC_FONTS.get(' ').unwrap());
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..CHAR_W {
            let color = if byte & (1 << bit) != 0 { fg } else { bg };
            let idx = (py + gy) * stride + (px + bit);
            if idx < buf.len() { buf[idx] = color; }
        }
    }
}

fn usize_to_buf(mut n: usize, buf: &mut [u8; 4]) -> &[u8] {
    if n == 0 { buf[0] = b'0'; return &buf[..1]; }
    let mut i = 4usize;
    while n > 0 && i > 0 { i -= 1; buf[i] = b'0' + (n % 10) as u8; n /= 10; }
    &buf[i..]
}
