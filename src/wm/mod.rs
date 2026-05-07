/// Window manager — public interface.
pub mod compositor;
pub mod window;

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

static REPAINT: AtomicBool = AtomicBool::new(false);
static CURSOR_REPAINT: AtomicBool = AtomicBool::new(false);
static FRAME_TICK: AtomicBool = AtomicBool::new(false);
static LAST_PASSIVE_FRAME_TICK: AtomicU64 = AtomicU64::new(0);
static SESSION_LOCK_REQUEST: AtomicBool = AtomicBool::new(false);
static SCREENSHOT_REQUEST: Mutex<Option<String>> = Mutex::new(None);
static PENDING_USER_GUI_OWNER_CLEANUP: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static PENDING_STARTUP_COMMANDS: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());

const PASSIVE_FRAME_HZ: u64 = 36;

pub fn request_repaint() {
    REPAINT.store(true, Ordering::Relaxed);
}

pub fn request_cursor_repaint() {
    CURSOR_REPAINT.store(true, Ordering::Relaxed);
}

pub fn request_frame_tick() {
    FRAME_TICK.store(true, Ordering::Relaxed);
}

pub fn request_session_lock() {
    SESSION_LOCK_REQUEST.store(true, Ordering::Relaxed);
    request_repaint();
}

pub(crate) fn take_session_lock_request() -> bool {
    SESSION_LOCK_REQUEST.swap(false, Ordering::Relaxed)
}

pub fn compose_if_needed() {
    let full = REPAINT.swap(false, Ordering::Relaxed);
    let cursor = CURSOR_REPAINT.swap(false, Ordering::Relaxed);
    let tick = FRAME_TICK.swap(false, Ordering::Relaxed);
    let now = crate::interrupts::ticks();
    let passive_due = tick && passive_frame_due(now);
    let startup_pending = !PENDING_STARTUP_COMMANDS.lock().is_empty();

    if full || startup_pending || passive_due {
        compositor::WM.lock().compose();
    } else if cursor {
        let mut wm = compositor::WM.lock();
        if !wm.compose_cursor_only() {
            wm.compose();
        }
    }
}

fn passive_frame_due(now: u64) -> bool {
    let divisor = (crate::interrupts::TIMER_HZ as u64 / PASSIVE_FRAME_HZ).max(1);
    let last = LAST_PASSIVE_FRAME_TICK.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= divisor {
        LAST_PASSIVE_FRAME_TICK.store(now, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub fn passive_frame_hz() -> u64 {
    PASSIVE_FRAME_HZ
}

pub fn prepare() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        compositor::WM.initialize();
    });
}

pub fn init() {
    request_repaint();
}

pub fn queue_startup_command(command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }
    let mut pending = PENDING_STARTUP_COMMANDS.lock();
    if pending.len() >= 16 {
        pending.remove(0);
    }
    let delay_ms = 250 + pending.len() as u64 * 250;
    let ready_tick =
        crate::interrupts::ticks().wrapping_add(crate::interrupts::ticks_for_millis(delay_ms));
    pending.push((String::from(command), ready_tick));
    request_repaint();
}

pub fn queue_startup_command_immediate(command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }
    let mut pending = PENDING_STARTUP_COMMANDS.lock();
    if pending.len() >= 16 {
        pending.remove(0);
    }
    pending.push((String::from(command), crate::interrupts::ticks()));
    request_repaint();
}

pub(crate) fn take_startup_command() -> Option<String> {
    let mut pending = PENDING_STARTUP_COMMANDS.lock();
    let ready = pending
        .first()
        .map(|(_, ready_tick)| crate::interrupts::ticks().wrapping_sub(*ready_tick) < u64::MAX / 2)
        .unwrap_or(false);
    if ready {
        let command = pending.remove(0).0;
        if !pending.is_empty() {
            request_frame_tick();
        }
        Some(command)
    } else {
        if !pending.is_empty() {
            request_frame_tick();
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
