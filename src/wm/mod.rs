/// Window manager — public interface.

pub mod compositor;
pub mod window;

use core::sync::atomic::{AtomicBool, Ordering};

/// Set by mouse/timer interrupts; cleared by the main loop after each compose().
static REPAINT: AtomicBool = AtomicBool::new(false);

pub fn request_repaint() {
    REPAINT.store(true, Ordering::Relaxed);
}

pub fn needs_repaint() -> bool {
    REPAINT.swap(false, Ordering::Relaxed)
}

/// Compose one frame if a repaint was requested.
pub fn compose_if_needed() {
    if REPAINT.swap(false, Ordering::Relaxed) {
        compositor::WM.lock().compose();
    }
}

/// Add a window to the global WM (call after heap is ready).
pub fn add_window(w: window::Window) {
    compositor::WM.lock().add_window(w);
}

/// Force the first full paint on boot.
pub fn init() {
    request_repaint();
}
