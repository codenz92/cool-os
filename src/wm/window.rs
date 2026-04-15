/// A single window managed by the compositor.

extern crate alloc;
use alloc::vec::Vec;

/// Height of the title bar in pixels.
pub const TITLE_H: i32 = 10;
/// Width of the close button in pixels.
pub const CLOSE_W: i32 = 10;

pub struct Window {
    pub x:      i32,
    pub y:      i32,
    pub width:  i32,
    pub height: i32,
    pub title:  &'static str,
    /// Per-pixel content area back-buffer (width × (height - TITLE_H) bytes).
    pub buf:    Vec<u8>,
    pub dirty:  bool,
}

impl Window {
    /// Create a new window.  `height` includes the title bar.
    pub fn new(x: i32, y: i32, width: i32, height: i32, title: &'static str) -> Self {
        let content_h = (height - TITLE_H).max(0) as usize;
        let buf = Vec::from_iter(
            core::iter::repeat(crate::framebuffer::DARK_GRAY)
                .take(width as usize * content_h),
        );
        Window { x, y, width, height, title, buf, dirty: true }
    }

    /// Does the pixel point `(px, py)` fall inside the title bar?
    pub fn hit_title(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + TITLE_H
    }

    /// Does the pixel point fall on the close button (top-right corner of title bar)?
    pub fn hit_close(&self, px: i32, py: i32) -> bool {
        px >= self.x + self.width - CLOSE_W
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + TITLE_H
    }

    /// Does the pixel point fall anywhere inside the window (including chrome)?
    pub fn hit(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + self.height
    }
}
