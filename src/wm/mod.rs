/// Window manager — public interface.
pub mod compositor;
pub mod window;

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

static REPAINT: AtomicBool = AtomicBool::new(false);
static SCREENSHOT_REQUEST: Mutex<Option<String>> = Mutex::new(None);
static PENDING_USER_GUI_OWNER_CLEANUP: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static PENDING_STARTUP_COMMAND: Mutex<Option<(String, u64)>> = Mutex::new(None);

pub fn request_repaint() {
    REPAINT.store(true, Ordering::Relaxed);
}

pub fn compose_if_needed() {
    if REPAINT.swap(false, Ordering::Relaxed) {
        compositor::WM.lock().compose();
    }
}

pub fn prepare() {
    drop(compositor::WM.lock());
}

pub fn init() {
    request_repaint();
}

pub fn queue_startup_command(command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }
    let ready_tick =
        crate::interrupts::ticks().wrapping_add(crate::interrupts::ticks_for_millis(250));
    *PENDING_STARTUP_COMMAND.lock() = Some((String::from(command), ready_tick));
    request_repaint();
}

pub(crate) fn take_startup_command() -> Option<String> {
    let mut pending = PENDING_STARTUP_COMMAND.lock();
    let ready = pending
        .as_ref()
        .map(|(_, ready_tick)| crate::interrupts::ticks().wrapping_sub(*ready_tick) < u64::MAX / 2)
        .unwrap_or(false);
    if ready {
        pending.take().map(|(command, _)| command)
    } else {
        if pending.is_some() {
            request_repaint();
        }
        None
    }
}

pub fn request_focused_screenshot(path: &str) {
    *SCREENSHOT_REQUEST.lock() = Some(String::from(path));
    request_repaint();
}

pub(crate) fn take_screenshot_request() -> Option<String> {
    SCREENSHOT_REQUEST.lock().take()
}

pub fn user_gui_open(owner: usize, title: &str, width: u16, height: u16) -> u64 {
    compositor::WM
        .try_lock()
        .map(|mut wm| wm.open_user_gui(owner, title, width, height))
        .unwrap_or(u64::MAX)
}

pub fn user_gui_present(owner: usize, handle: u64, pixels: &[u8]) -> bool {
    compositor::WM
        .try_lock()
        .map(|mut wm| wm.present_user_gui(owner, handle, pixels))
        .unwrap_or(false)
}

pub fn user_gui_poll_event(owner: usize, handle: u64, out: &mut [u8]) -> Option<usize> {
    compositor::WM
        .try_lock()
        .and_then(|mut wm| wm.poll_user_gui_event(owner, handle, out))
}

pub fn user_gui_close(owner: usize, handle: u64) -> bool {
    compositor::WM
        .try_lock()
        .map(|mut wm| wm.close_user_gui(owner, handle))
        .unwrap_or(false)
}

pub fn close_user_gui_windows_for_owner(owner: usize) {
    if let Some(mut wm) = compositor::WM.try_lock() {
        wm.close_user_gui_windows_for_owner(owner);
    } else {
        queue_user_gui_owner_cleanup(owner);
    }
}

pub(crate) fn drain_user_gui_owner_cleanup(wm: &mut compositor::WindowManager) {
    let owners = x86_64::instructions::interrupts::without_interrupts(|| {
        let mut pending = PENDING_USER_GUI_OWNER_CLEANUP.lock();
        core::mem::take(&mut *pending)
    });

    for owner in owners {
        wm.close_user_gui_windows_for_owner(owner);
    }
}

fn queue_user_gui_owner_cleanup(owner: usize) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut pending = PENDING_USER_GUI_OWNER_CLEANUP.lock();
        if !pending.iter().any(|&queued| queued == owner) {
            pending.push(owner);
        }
    });
    request_repaint();
}
