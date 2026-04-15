/// Window compositor — paints the desktop, then each window back-to-front,
/// then the cursor on top.  Called from the main loop whenever REPAINT is set.

extern crate alloc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::framebuffer::{self, HEIGHT, WIDTH, WHITE, BLACK, LIGHT_GRAY, DARK_GRAY, BLUE, RED};
use crate::wm::window::{Window, TITLE_H, CLOSE_W};

// ── Cursor shape ──────────────────────────────────────────────────────────────

const CURSOR_W: usize = 8;
const CURSOR_H: usize = 8;

/// 1-bit mask — bit 7 is the leftmost pixel. Arrow pointing top-left.
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

// ── Drag state ────────────────────────────────────────────────────────────────

struct DragState {
    /// Index into WindowManager::windows of the dragged window.
    window: usize,
    /// Cursor offset from window top-left at the time the drag started.
    off_x: i32,
    off_y: i32,
}

// ── Window manager ────────────────────────────────────────────────────────────

pub struct WindowManager {
    pub windows:  Vec<Window>,
    /// z-order: windows[z_order[0]] is back-most, windows[z_order[last]] is front-most.
    z_order:      Vec<usize>,
    focused:      Option<usize>,
    drag:         Option<DragState>,
    /// Last known mouse position — used to redraw cursor without re-querying.
    last_mx:      usize,
    last_my:      usize,
    /// Whether the left button was down on the previous frame.
    prev_left:    bool,
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows:   Vec::new(),
            z_order:   Vec::new(),
            focused:   None,
            drag:      None,
            last_mx:   WIDTH  / 2,
            last_my:   HEIGHT / 2,
            prev_left: false,
        }
    }

    pub fn add_window(&mut self, w: Window) {
        let idx = self.windows.len();
        self.windows.push(w);
        self.z_order.push(idx);
        self.focused = Some(idx);
    }

    /// Full composite: desktop → windows back-to-front → cursor.
    pub fn compose(&mut self) {
        let (mx, my) = crate::mouse::pos();
        let (left, _right) = crate::mouse::buttons();
        let mx_i = mx as i32;
        let my_i = my as i32;

        // ── Handle mouse events ──────────────────────────────────────────────

        let left_just_pressed  = left && !self.prev_left;
        let left_just_released = !left && self.prev_left;

        if left_just_pressed {
            // Hit-test from front to back.
            let hit = self.front_to_back_hit(mx_i, my_i);

            if let Some(z_pos) = hit {
                let win_idx = self.z_order[z_pos];

                // Raise to front.
                self.z_order.remove(z_pos);
                self.z_order.push(win_idx);
                self.focused = Some(win_idx);

                let w = &self.windows[win_idx];

                if w.hit_close(mx_i, my_i) {
                    // Close button: remove window.
                    self.windows.remove(win_idx);
                    self.z_order.retain(|&i| i != win_idx);
                    // Remap indices above win_idx.
                    for z in self.z_order.iter_mut() {
                        if *z > win_idx { *z -= 1; }
                    }
                    self.focused = self.z_order.last().copied();
                    self.drag = None;
                } else if w.hit_title(mx_i, my_i) {
                    // Start drag.
                    self.drag = Some(DragState {
                        window: win_idx,
                        off_x: mx_i - w.x,
                        off_y: my_i - w.y,
                    });
                }
            }
        }

        if left_just_released {
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
        self.last_mx   = mx;
        self.last_my   = my;

        // ── Paint ────────────────────────────────────────────────────────────

        // Desktop background.
        framebuffer::clear(DARK_GRAY);

        // Windows back-to-front.
        for &wi in &self.z_order {
            if wi < self.windows.len() {
                let is_focused = self.focused == Some(wi);
                Self::draw_window(&self.windows[wi], is_focused);
            }
        }

        // Cursor on top.
        for (row, &byte) in CURSOR_SHAPE.iter().enumerate() {
            for bit in 0..8usize {
                if byte & (0x80 >> bit) != 0 {
                    framebuffer::put_pixel(mx + bit, my + row, WHITE);
                }
            }
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Returns the z-order position (index into z_order vec, highest = front)
    /// of the front-most window under `(px, py)`, or None.
    fn front_to_back_hit(&self, px: i32, py: i32) -> Option<usize> {
        for z_pos in (0..self.z_order.len()).rev() {
            let wi = self.z_order[z_pos];
            if wi < self.windows.len() && self.windows[wi].hit(px, py) {
                return Some(z_pos);
            }
        }
        None
    }

    fn draw_window(w: &Window, focused: bool) {
        let title_color = if focused { BLUE } else { DARK_GRAY };
        let border_color = if focused { LIGHT_GRAY } else { DARK_GRAY };

        // Border (1px).
        framebuffer::fill_rect_clipped(w.x - 1, w.y - 1, w.width + 2, w.height + 2, border_color);

        // Title bar.
        framebuffer::fill_rect_clipped(w.x, w.y, w.width, TITLE_H, title_color);

        // Title text.
        let text_x = (w.x + 2).max(0) as usize;
        let text_y = (w.y + 1).max(0) as usize;
        let max_x  = ((w.x + w.width - CLOSE_W - 1).max(0) as usize).min(WIDTH);
        framebuffer::draw_str_px_clipped(text_x, text_y, w.title, WHITE, title_color, max_x);

        // Close button.
        let cx = w.x + w.width - CLOSE_W;
        framebuffer::fill_rect_clipped(cx, w.y, CLOSE_W, TITLE_H, RED);
        let close_tx = (cx + 2).max(0) as usize;
        let close_ty = (w.y + 1).max(0) as usize;
        framebuffer::draw_str_px_clipped(close_tx, close_ty, "x", WHITE, RED, WIDTH);

        // Content area.
        let content_y = w.y + TITLE_H;
        let content_h = w.height - TITLE_H;
        framebuffer::fill_rect_clipped(w.x, content_y, w.width, content_h, BLACK);

        // Content back-buffer.
        let cw = w.width as usize;
        let ch = content_h.max(0) as usize;
        for row in 0..ch {
            for col in 0..cw {
                let color = w.buf[row * cw + col];
                let px = w.x + col as i32;
                let py = content_y + row as i32;
                if px >= 0 && py >= 0 {
                    framebuffer::put_pixel(px as usize, py as usize, color);
                }
            }
        }
    }
}

lazy_static! {
    pub static ref WM: Mutex<WindowManager> = Mutex::new(WindowManager::new());
}
