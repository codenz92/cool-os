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
static LAST_PACED_FRAME_TICK: AtomicU64 = AtomicU64::new(0);
static ACTIVE_FRAME_UNTIL_TICK: AtomicU64 = AtomicU64::new(0);
static ACTIVE_FRAME_BOOSTS: AtomicU64 = AtomicU64::new(0);
static SESSION_LOCK_REQUEST: AtomicBool = AtomicBool::new(false);
static SCREENSHOT_REQUEST: Mutex<Option<String>> = Mutex::new(None);
static PENDING_USER_GUI_OWNER_CLEANUP: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static PENDING_STARTUP_COMMANDS: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());

const PASSIVE_FRAME_HZ: u64 = 36;
const ACTIVE_FRAME_HZ: u64 = 144;
const ACTIVE_FRAME_BOOST_MS: u64 = 750;

pub fn request_repaint() {
    boost_active_frame_pacing();
    REPAINT.store(true, Ordering::Relaxed);
}

pub fn request_cursor_repaint() {
    boost_active_frame_pacing();
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
    let paced_due = tick && paced_frame_due(now);
    let startup_due = startup_command_due_at(now);

    if full || startup_due || paced_due {
        mark_paced_frame(now);
        compositor::WM.lock().compose();
    } else if cursor {
        let mut wm = compositor::WM.lock();
        if !wm.compose_cursor_only() {
            mark_paced_frame(now);
            wm.compose();
        }
    }
}

fn boost_active_frame_pacing() {
    let now = crate::interrupts::ticks();
    let until = now.wrapping_add(crate::interrupts::ticks_for_millis(ACTIVE_FRAME_BOOST_MS));
    loop {
        let current = ACTIVE_FRAME_UNTIL_TICK.load(Ordering::Relaxed);
        if until <= current {
            break;
        }
        if ACTIVE_FRAME_UNTIL_TICK
            .compare_exchange(current, until, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            ACTIVE_FRAME_BOOSTS.fetch_add(1, Ordering::Relaxed);
            break;
        }
    }
}

fn paced_frame_due(now: u64) -> bool {
    let divisor = frame_budget_ticks_for_hz(target_frame_hz_at(now));
    let last = LAST_PACED_FRAME_TICK.load(Ordering::Relaxed);
    if now.wrapping_sub(last) >= divisor {
        LAST_PACED_FRAME_TICK.store(now, Ordering::Relaxed);
        true
    } else {
        false
    }
}

fn mark_paced_frame(now: u64) {
    LAST_PACED_FRAME_TICK.store(now, Ordering::Relaxed);
}

fn frame_budget_ticks_for_hz(hz: u64) -> u64 {
    let timer_hz = crate::interrupts::TIMER_HZ as u64;
    let hz = hz.max(1);
    ((timer_hz + hz - 1) / hz).max(1)
}

fn target_frame_hz_at(now: u64) -> u64 {
    if active_frame_boosted_at(now) {
        ACTIVE_FRAME_HZ
    } else {
        PASSIVE_FRAME_HZ
    }
}

fn active_frame_boosted_at(now: u64) -> bool {
    let until = ACTIVE_FRAME_UNTIL_TICK.load(Ordering::Relaxed);
    until != 0 && until >= now
}

fn startup_command_due_at(now: u64) -> bool {
    PENDING_STARTUP_COMMANDS
        .lock()
        .first()
        .map(|(_, ready_tick)| now.wrapping_sub(*ready_tick) < u64::MAX / 2)
        .unwrap_or(false)
}

pub fn passive_frame_hz() -> u64 {
    PASSIVE_FRAME_HZ
}

pub fn active_frame_hz() -> u64 {
    ACTIVE_FRAME_HZ
}

pub fn active_frame_boost_ms() -> u64 {
    ACTIVE_FRAME_BOOST_MS
}

pub fn active_frame_boosts() -> u64 {
    ACTIVE_FRAME_BOOSTS.load(Ordering::Relaxed)
}

pub fn target_frame_hz() -> u64 {
    target_frame_hz_at(crate::interrupts::ticks())
}

pub fn target_frame_budget_ticks() -> u64 {
    frame_budget_ticks_for_hz(target_frame_hz())
}

pub fn active_frame_boost_ms_left() -> u64 {
    let now = crate::interrupts::ticks();
    let until = ACTIVE_FRAME_UNTIL_TICK.load(Ordering::Relaxed);
    if until <= now {
        return 0;
    }
    let ticks_left = until - now;
    let timer_hz = crate::interrupts::TIMER_HZ as u64;
    (ticks_left.saturating_mul(1000) + timer_hz - 1) / timer_hz
}

pub fn frame_pacing_mode() -> &'static str {
    if active_frame_boost_ms_left() > 0 {
        "active"
    } else {
        "idle"
    }
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

pub fn user_gui_event_readiness(owner: usize, handle: u64) -> Option<u64> {
    compositor::WM.try_lock().map(|wm| {
        wm.user_gui_event_readiness(owner, handle)
            .unwrap_or(crate::evented::EVENT_ERROR)
    })
}

pub fn register_user_gui_event_waiter(owner: usize, handle: u64, task_id: usize) -> bool {
    compositor::WM
        .try_lock()
        .map(|mut wm| wm.register_user_gui_event_waiter(owner, handle, task_id))
        .unwrap_or(false)
}

pub fn unregister_user_gui_event_waiter(owner: usize, handle: u64, task_id: usize) {
    if let Some(mut wm) = compositor::WM.try_lock() {
        wm.unregister_user_gui_event_waiter(owner, handle, task_id);
    }
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

pub fn trim_browser_memory_pressure() -> usize {
    compositor::WM
        .try_lock()
        .map(|mut wm| wm.trim_browser_memory_pressure())
        .unwrap_or(0)
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
