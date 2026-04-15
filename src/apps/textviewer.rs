/// Text Viewer — scrollable read-only display of a static document.
/// Press 'j' to scroll down, 'k' to scroll up.

use font8x8::UnicodeFonts;
use crate::framebuffer::{CHAR_W, CHAR_H, WHITE, DARK_GRAY, LIGHT_GRAY, YELLOW};
use crate::wm::window::{Window, TITLE_H};

pub const VIEWER_W: i32 = 160;
pub const VIEWER_H: i32 = 120;

const ABOUT: &[&str] = &[
    " coolOS v1.4",
    " Bare-metal OS in Rust",
    "",
    " == Phases ==",
    " 1. Pixel framebuffer",
    " 2. PS/2 mouse driver",
    " 3. Window manager",
    " 4. Desktop shell",
    " 5. Applications",
    "",
    " == Commands ==",
    " help    - list commands",
    " echo    - print text",
    " info    - CPU + heap",
    " uptime  - tick count",
    " clear   - clear term",
    " reboot  - restart OS",
    "",
    " == Controls ==",
    " j / k   scroll dn/up",
    " Drag title bar: move",
    " x button: close win",
    " Right-click: new app",
    "",
    " github.com/codenz92",
    "   /cool_os",
];

pub struct TextViewerApp {
    pub window: Window,
    scroll:     usize,
    rows:       usize,
    cols:       usize,
}

impl TextViewerApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, VIEWER_W, VIEWER_H, "About coolOS");
        let content_h = (VIEWER_H - TITLE_H) as usize;
        let mut app = TextViewerApp {
            window,
            scroll: 0,
            rows:   content_h / CHAR_H,
            cols:   VIEWER_W as usize / CHAR_W,
        };
        app.render();
        app
    }

    pub fn handle_key(&mut self, c: char) {
        match c {
            'j' | 'J' => {
                if self.scroll + self.rows < ABOUT.len() {
                    self.scroll += 1;
                    self.render();
                }
            }
            'k' | 'K' => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                    self.render();
                }
            }
            _ => {}
        }
    }

    fn render(&mut self) {
        let stride = VIEWER_W as usize;
        for b in self.window.buf.iter_mut() { *b = DARK_GRAY; }

        for screen_row in 0..self.rows {
            let doc_row = self.scroll + screen_row;
            if doc_row >= ABOUT.len() { break; }
            let line = ABOUT[doc_row];
            let py = screen_row * CHAR_H;
            for (ci, c) in line.chars().enumerate() {
                if ci >= self.cols { break; }
                let px = ci * CHAR_W;
                let fg = if line.starts_with(" ==") { YELLOW }
                         else if line.starts_with(" //") || line.starts_with("  ") { LIGHT_GRAY }
                         else { WHITE };
                Self::put_char(&mut self.window.buf, stride, px, py, c, fg);
            }
        }

        // Scroll hint bar at top and bottom.
        let top_color = if self.scroll > 0 { LIGHT_GRAY } else { DARK_GRAY };
        let bot_color = if self.scroll + self.rows < ABOUT.len() { LIGHT_GRAY } else { DARK_GRAY };
        let hint_row = (self.rows - 1) * CHAR_H;
        for px in 0..stride {
            if self.window.buf[px] != DARK_GRAY {
                self.window.buf[px] = top_color;
            }
            let idx = hint_row * stride + px;
            if idx < self.window.buf.len() {
                self.window.buf[idx] = bot_color;
            }
        }
    }

    fn put_char(buf: &mut [u8], stride: usize, px: usize, py: usize, c: char, fg: u8) {
        let glyph = font8x8::BASIC_FONTS
            .get(c)
            .unwrap_or_else(|| font8x8::BASIC_FONTS.get(' ').unwrap());
        for (gy, &byte) in glyph.iter().enumerate() {
            for bit in 0..CHAR_W {
                if byte & (1 << bit) != 0 {
                    let idx = (py + gy) * stride + (px + bit);
                    if idx < buf.len() { buf[idx] = fg; }
                }
            }
        }
    }
}
