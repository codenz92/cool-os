/// Window compositor — paints the desktop, then each window back-to-front,
/// then the cursor on top, all into a 64 KB shadow buffer.  At the end of
/// compose() the shadow is copied to the VGA framebuffer in one shot, which
/// eliminates visible tearing/flickering.

extern crate alloc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::framebuffer::{HEIGHT, WIDTH, WHITE, BLACK, LIGHT_GRAY, DARK_GRAY, BLUE, RED, CHAR_W};
use crate::wm::window::{Window, TITLE_H, CLOSE_W};

const FB: *mut u8 = 0xa0000 as *mut u8;

// ── Cursor shape ──────────────────────────────────────────────────────────────

const CURSOR_H: usize = 8;

/// Arrow pointing top-left, bit 7 = leftmost pixel.
const CURSOR_SHAPE: [u8; CURSOR_H] = [
    0b11111110,
    0b11111100,
    0b11111000,
    0b11110000,
    0b11111000,
    0b11001100,
    0b10000110,
    0b00000011,
];

// ── Shadow-buffer helpers ─────────────────────────────────────────────────────
// All rendering targets a `&mut [u8; WIDTH * HEIGHT]` slice.  Nothing writes
// to the VGA address until the final blit in `compose()`.

#[inline(always)]
fn s_put(shadow: &mut [u8], x: i32, y: i32, color: u8) {
    if x >= 0 && y >= 0 {
        let (x, y) = (x as usize, y as usize);
        if x < WIDTH && y < HEIGHT {
            shadow[y * WIDTH + x] = color;
        }
    }
}

fn s_fill(shadow: &mut [u8], x: i32, y: i32, w: i32, h: i32, color: u8) {
    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w).max(0) as usize).min(WIDTH);
    let y1 = ((y + h).max(0) as usize).min(HEIGHT);
    for row in y0..y1 {
        let base = row * WIDTH;
        shadow[base + x0..base + x1].fill(color);
    }
}

fn s_draw_char(shadow: &mut [u8], x: i32, y: i32, c: char, fg: u8, bg: u8) {
    use font8x8::UnicodeFonts;
    let glyph = font8x8::BASIC_FONTS
        .get(c)
        .unwrap_or_else(|| font8x8::BASIC_FONTS.get(' ').unwrap());
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            let color = if byte & (1 << bit) != 0 { fg } else { bg };
            s_put(shadow, x + bit as i32, y + gy as i32, color);
        }
    }
}

fn s_draw_str(shadow: &mut [u8], x: i32, y: i32, s: &str, fg: u8, bg: u8, max_x: i32) {
    let mut cx = x;
    for c in s.chars() {
        if cx + CHAR_W as i32 > max_x {
            break;
        }
        s_draw_char(shadow, cx, y, c, fg, bg);
        cx += CHAR_W as i32;
    }
}

// ── Drag state ────────────────────────────────────────────────────────────────

struct DragState {
    window: usize,
    off_x:  i32,
    off_y:  i32,
}

// ── Window manager ────────────────────────────────────────────────────────────

pub struct WindowManager {
    pub windows: Vec<Window>,
    /// z_order[0] = back-most, z_order[last] = front-most.
    z_order:     Vec<usize>,
    focused:     Option<usize>,
    drag:        Option<DragState>,
    prev_left:   bool,
    /// Off-screen render target — one byte per pixel, row-major.
    shadow:      Vec<u8>,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows:   Vec::new(),
            z_order:   Vec::new(),
            focused:   None,
            drag:      None,
            prev_left: false,
            shadow:    alloc::vec![0u8; WIDTH * HEIGHT],
        }
    }

    pub fn add_window(&mut self, w: Window) {
        let idx = self.windows.len();
        self.windows.push(w);
        self.z_order.push(idx);
        self.focused = Some(idx);
    }

    /// Render one frame into the shadow buffer, then blit to VGA.
    pub fn compose(&mut self) {
        let (mx, my) = crate::mouse::pos();
        let (left, _right) = crate::mouse::buttons();
        let mx_i = mx as i32;
        let my_i = my as i32;

        // ── Mouse event handling ─────────────────────────────────────────────

        let just_pressed  = left  && !self.prev_left;
        let just_released = !left &&  self.prev_left;

        if just_pressed {
            if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
                let win_idx = self.z_order[z_pos];

                // Raise to front.
                self.z_order.remove(z_pos);
                self.z_order.push(win_idx);
                self.focused = Some(win_idx);

                let w = &self.windows[win_idx];

                if w.hit_close(mx_i, my_i) {
                    // Remove window and remap indices.
                    self.windows.remove(win_idx);
                    self.z_order.retain(|&i| i != win_idx);
                    for z in self.z_order.iter_mut() {
                        if *z > win_idx { *z -= 1; }
                    }
                    self.focused = self.z_order.last().copied();
                    self.drag    = None;
                } else if w.hit_title(mx_i, my_i) {
                    self.drag = Some(DragState {
                        window: win_idx,
                        off_x:  mx_i - w.x,
                        off_y:  my_i - w.y,
                    });
                }
            }
        }

        if just_released {
            self.drag = None;
        }

        if left {
            if let Some(ref d) = self.drag {
                let wi = d.window;
                if wi < self.windows.len() {
                    self.windows[wi].x = mx_i - d.off_x;
                    self.windows[wi].y = my_i - d.off_y;
                }
            }
        }

        self.prev_left = left;

        // ── Render into shadow buffer ────────────────────────────────────────

        let s = &mut self.shadow;

        // Desktop background.
        s.fill(DARK_GRAY);

        // Windows back-to-front.
        let z_snapshot: Vec<usize> = self.z_order.clone();
        for &wi in &z_snapshot {
            if wi < self.windows.len() {
                let focused = self.focused == Some(wi);
                let w = &self.windows[wi];
                Self::draw_window(s, w, focused);
            }
        }

        // Cursor on top.
        for (row, &byte) in CURSOR_SHAPE.iter().enumerate() {
            for bit in 0..8usize {
                if byte & (0x80 >> bit) != 0 {
                    s_put(s, mx as i32 + bit as i32, my as i32 + row as i32, WHITE);
                }
            }
        }

        // ── Blit shadow → VGA (one memcpy, no tearing) ──────────────────────
        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), FB, WIDTH * HEIGHT);
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn front_to_back_hit(&self, px: i32, py: i32) -> Option<usize> {
        for z_pos in (0..self.z_order.len()).rev() {
            let wi = self.z_order[z_pos];
            if wi < self.windows.len() && self.windows[wi].hit(px, py) {
                return Some(z_pos);
            }
        }
        None
    }

    fn draw_window(s: &mut [u8], w: &Window, focused: bool) {
        let title_color  = if focused { BLUE      } else { DARK_GRAY };
        let border_color = if focused { LIGHT_GRAY } else { DARK_GRAY };

        // 1-px border.
        s_fill(s, w.x - 1, w.y - 1, w.width + 2, w.height + 2, border_color);

        // Title bar.
        s_fill(s, w.x, w.y, w.width, TITLE_H, title_color);

        // Title text.
        let max_title_x = w.x + w.width - CLOSE_W - 1;
        s_draw_str(s, w.x + 2, w.y + 1, w.title, WHITE, title_color, max_title_x);

        // Close button.
        let cx = w.x + w.width - CLOSE_W;
        s_fill(s, cx, w.y, CLOSE_W, TITLE_H, RED);
        s_draw_str(s, cx + 2, w.y + 1, "x", WHITE, RED, cx + CLOSE_W);

        // Content area background.
        let content_y = w.y + TITLE_H;
        let content_h = w.height - TITLE_H;
        s_fill(s, w.x, content_y, w.width, content_h, BLACK);

        // Content back-buffer (per-pixel).
        let cw = w.width as usize;
        let ch = content_h.max(0) as usize;
        for row in 0..ch {
            for col in 0..cw {
                let color = w.buf[row * cw + col];
                s_put(s, w.x + col as i32, content_y + row as i32, color);
            }
        }
    }
}

lazy_static! {
    pub static ref WM: Mutex<WindowManager> = Mutex::new(WindowManager::new());
}
