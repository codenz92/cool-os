/// Window compositor — desktop, windows, taskbar, cursor, context menu.
/// All rendering targets a `Vec<u32>` shadow buffer; one blit per frame.
///
/// Visual theme: modern dark glass desktop
extern crate alloc;
use alloc::{boxed::Box, format, string::String, vec::Vec};
use core::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};
use spin::{Mutex, MutexGuard};

use crate::apps::{
    BrowserApp, ColorPickerApp, DisplaySettingsApp, FileManagerApp, FileManagerOpenRequest,
    PersonalizeApp, SysMonApp, SysMonRequest, TerminalApp, TextViewerApp, UserGuiApp, UtilityApp,
};
use crate::desktop_settings::{self, DesktopSortMode, WallpaperPreset};
use crate::framebuffer::{BLACK, WHITE};
use crate::keyboard::{Key, KeyInput};
use crate::wm::window::{Window, TITLE_H};

mod desktop;
mod dialogs;
mod greeter;
mod icons;
mod notifications;
mod primitives;
mod start_menu;
mod taskbar;

use self::desktop::*;
use self::dialogs::*;
use self::greeter::*;
use self::icons::*;
use self::notifications::*;
use self::primitives::*;
use self::start_menu::*;
use self::taskbar::*;

// ── Layout constants ──────────────────────────────────────────────────────────

const TASKBAR_H: i32 = 40; // Win11: 40px tall taskbar
const START_BTN_W: i32 = 40; // square start icon button + left gutter
const TASKBAR_CLOCK_W: i32 = 176; // tray + time readout + brand
const TASKBAR_TRAY_W: i32 = 70;
const SHOW_DESKTOP_W: i32 = 18;
const BUTTON_W: i32 = 160;
const WIN_BTN_W: i32 = crate::wm::window::WIN_BTN_W;
const SCROLLBAR_W: i32 = crate::wm::window::SCROLLBAR_W;
const RESIZE_HANDLE: i32 = crate::wm::window::RESIZE_HANDLE;
const EVENT_PACKET_SIZE: usize = 8;
const EVENT_KIND_KEY_CHAR: u8 = 1;
const EVENT_KIND_MOUSE_DOWN: u8 = 2;
const SNAP_EDGE_PX: i32 = 18;
const TASK_SWITCHER_MS: u64 = 1200;
const SESSION_PATH: &str = "/CONFIG/SESSION.CFG";
const SESSION_SAVE_MS: u64 = 1200;
const MAX_SESSION_WINDOWS: usize = 8;
const WORKSPACE_COUNT: usize = 4;
const USER_GUI_CLOSE_TIMEOUT_MS: u64 = 1500;
const TASKBAR_MENU_W: i32 = 152;
const TASKBAR_MENU_ROW_H: i32 = 24;
const TASKBAR_MENU_H: i32 = TASKBAR_MENU_ROW_H * 3 + 10;
const START_MENU_SECTION_H: i32 = 11;
const START_MENU_WIN7_W: i32 = 440;
const START_MENU_WIN7_H: i32 = 468;
const START_MENU_WIN7_MIN_W: i32 = 360;
const START_MENU_WIN7_MIN_H: i32 = 340;
const START_MENU_WIN7_RIGHT_W: i32 = 180;
const START_MENU_WIN7_BOTTOM_H: i32 = 46;
const START_MENU_WIN7_ROW_H: i32 = 32;
const START_MENU_WIN7_LINK_H: i32 = 34;
const START_POWER_MENU_W: i32 = 136;
const START_POWER_MENU_ROW_H: i32 = 24;
const START_POWER_MENU_PAD: i32 = 4;
const GREETER_PANEL_W: i32 = 460;
const GREETER_PANEL_H: i32 = 420;
const GREETER_FIELD_H: i32 = 30;
const GREETER_USER_ROW_H: i32 = 28;

static COMPOSITOR_FPS: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FRAME_TICKS_LAST: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FRAME_TICKS_PEAK: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_DAMAGE_ROWS: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_DAMAGE_PIXELS: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FRAMES: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_CURSOR_FAST_FRAMES: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_CURSOR_PIXELS_LAST: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FULL_FRAMES: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FRAME_BUDGET_TICKS: AtomicU64 = AtomicU64::new(0);
static COMPOSITOR_FRAME_BUDGET_MISSES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub struct CompositorStats {
    pub fps: u64,
    pub frame_ticks_last: u64,
    pub frame_ticks_peak: u64,
    pub damage_rows: u64,
    pub damage_pixels: u64,
    pub frames: u64,
    pub full_frames: u64,
    pub cursor_fast_frames: u64,
    pub cursor_pixels_last: u64,
    pub frame_budget_ticks: u64,
    pub frame_budget_misses: u64,
}

pub fn compositor_stats() -> CompositorStats {
    CompositorStats {
        fps: COMPOSITOR_FPS.load(Ordering::Relaxed),
        frame_ticks_last: COMPOSITOR_FRAME_TICKS_LAST.load(Ordering::Relaxed),
        frame_ticks_peak: COMPOSITOR_FRAME_TICKS_PEAK.load(Ordering::Relaxed),
        damage_rows: COMPOSITOR_DAMAGE_ROWS.load(Ordering::Relaxed),
        damage_pixels: COMPOSITOR_DAMAGE_PIXELS.load(Ordering::Relaxed),
        frames: COMPOSITOR_FRAMES.load(Ordering::Relaxed),
        full_frames: COMPOSITOR_FULL_FRAMES.load(Ordering::Relaxed),
        cursor_fast_frames: COMPOSITOR_CURSOR_FAST_FRAMES.load(Ordering::Relaxed),
        cursor_pixels_last: COMPOSITOR_CURSOR_PIXELS_LAST.load(Ordering::Relaxed),
        frame_budget_ticks: COMPOSITOR_FRAME_BUDGET_TICKS.load(Ordering::Relaxed),
        frame_budget_misses: COMPOSITOR_FRAME_BUDGET_MISSES.load(Ordering::Relaxed),
    }
}

pub fn compositor_lines() -> Vec<String> {
    let stats = compositor_stats();
    let (usb_keyboard, usb_pointer) = crate::usb::input_presence();
    alloc::vec![
        format!("fps={} frames={}", stats.fps, stats.frames),
        format!(
            "frame_ticks last={} peak={}",
            stats.frame_ticks_last, stats.frame_ticks_peak
        ),
        format!(
            "damage rows={} pixels={}",
            stats.damage_rows, stats.damage_pixels
        ),
        format!(
            "frame_source full={} cursor_fast={} passive_frame_hz={}",
            stats.full_frames,
            stats.cursor_fast_frames,
            crate::wm::passive_frame_hz()
        ),
        format!(
            "frame_pacing mode={} target_hz={} idle_hz={} active_hz={} boost_ms_left={} boost_ms={} boosts={}",
            crate::wm::frame_pacing_mode(),
            crate::wm::target_frame_hz(),
            crate::wm::passive_frame_hz(),
            crate::wm::active_frame_hz(),
            crate::wm::active_frame_boost_ms_left(),
            crate::wm::active_frame_boost_ms(),
            crate::wm::active_frame_boosts()
        ),
        format!(
            "frame_budget target_ticks={} misses={}",
            stats.frame_budget_ticks, stats.frame_budget_misses
        ),
        format!(
            "cursor_mode=overlay cursor_pixels_last={}",
            stats.cursor_pixels_last
        ),
        format!(
            "input usb_keyboard={} usb_pointer={} pointer_kind={}",
            usb_keyboard,
            usb_pointer,
            crate::usb::pointer_kind()
        ),
    ]
}

// ── Colors — modern dark glass desktop ────────────────────────────────────────

// Taskbar / shell
const ACCENT: u32 = 0x00_2B_C8_E8;
const ACCENT_HOV: u32 = 0x00_7D_E7_F7;

// Window chrome
const WIN_BAR_F: u32 = 0x00_13_1A_25;
const WIN_BAR_U: u32 = 0x00_0B_10_18;
const WIN_CONTENT: u32 = 0x00_0E_14_1E;
const WIN_BDR_F: u32 = 0x00_35_B8_D8;
const WIN_BDR_U: u32 = 0x00_2A_36_46;

// Window caption buttons
const CAP_NORMAL: u32 = 0x00_13_1A_25;
const CAP_HOV: u32 = 0x00_21_2D_3A;
const CLOSE_REST: u32 = 0x00_13_1A_25;
const CLOSE_HOV: u32 = 0x00_D9_4A_4A;

/// Sentinel stored in window content buffers to mean "render the window background here".
/// Apps that genuinely need to paint pure black should write `0x00_00_00_01` instead
/// (visually identical, but not intercepted by the compositor blit).
const WIN_TRANSPARENT: u32 = 0x00_00_00_00;

// Desktop wallpaper
const DESK_TL: u32 = 0x00_0B_10_1A;
const DESK_TR: u32 = 0x00_17_18_25;
const DESK_BL: u32 = 0x00_05_17_18;
const DESK_BR: u32 = 0x00_1A_12_1F;
const BLOOM_1: u32 = 0x00_2A_A7_A4;

// Splash/greeter
const GREETER_BG_TOP: u32 = 0x00_0E_13_1F;
const GREETER_BG_BOTTOM: u32 = 0x00_05_0B_12;
const GREETER_BLOOM: u32 = 0x00_2A_A7_A4;
const GREETER_PANEL_BG: u32 = 0x00_0D_15_21;
const GREETER_PANEL_BG_2: u32 = 0x00_09_10_1A;
const GREETER_TITLE: u32 = 0x00_EB_F4_F8;
const GREETER_FIELD_BG: u32 = 0x00_08_0E_16;

// Desktop icons
const ICON_TERM_ACC: u32 = 0x00_4C_DD_A1;
const ICON_MON_ACC: u32 = 0x00_54_C7_EB;
const ICON_TXT_ACC: u32 = 0x00_7C_A7_F8;
const ICON_COL_ACC: u32 = 0x00_C0_85_F5;

// ── Cursor ────────────────────────────────────────────────────────────────────

const CURSOR_H: usize = 12;
const CURSOR_W: usize = 16;
// Standard Windows arrow cursor (taller, more precise)
const CURSOR_SHAPE: [u16; CURSOR_H] = [
    0b1000000000000000,
    0b1100000000000000,
    0b1110000000000000,
    0b1111000000000000,
    0b1111100000000000,
    0b1111110000000000,
    0b1111111000000000,
    0b1111100000000000,
    0b1101100000000000,
    0b1000110000000000,
    0b0000110000000000,
    0b0000011000000000,
];

// Black outline mask (1-pixel rim)
const CURSOR_OUTLINE: [u16; CURSOR_H] = [
    0b1100000000000000,
    0b1110000000000000,
    0b1111000000000000,
    0b1111100000000000,
    0b1111110000000000,
    0b1111111000000000,
    0b1111111100000000,
    0b1111111000000000,
    0b1111110000000000,
    0b1100111000000000,
    0b0000111000000000,
    0b0000111100000000,
];

const CURSOR_RESIZE_SHAPE: [u16; CURSOR_H] = [
    0b0000000000000000,
    0b0000000000110000,
    0b0000000001111000,
    0b0000000011111100,
    0b0000000110110100,
    0b0000001100110000,
    0b0000011001100000,
    0b0000110011000000,
    0b0001101101100000,
    0b0011111101111000,
    0b0001111000110000,
    0b0000110000000000,
];

const CURSOR_RESIZE_OUTLINE: [u16; CURSOR_H] = [
    0b0000000000110000,
    0b0000000001111000,
    0b0000000011111100,
    0b0000000111111110,
    0b0000001111111110,
    0b0000011111111100,
    0b0000111111111000,
    0b0001111111110000,
    0b0011111111111000,
    0b0111111111111100,
    0b0011111111111000,
    0b0001111000110000,
];

fn month_abbrev(month: u8) -> &'static str {
    match month {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        12 => "DEC",
        _ => "---",
    }
}

#[allow(dead_code)]
fn push_two_digits(out: &mut String, value: u8) {
    out.push((b'0' + (value / 10)) as char);
    out.push((b'0' + (value % 10)) as char);
}

#[allow(dead_code)]
fn push_u16(out: &mut String, value: u16) {
    for &div in &[1000u16, 100, 10, 1] {
        out.push((b'0' + ((value / div) % 10) as u8) as char);
    }
}

fn push_u8_decimal(out: &mut String, value: u8) {
    if value >= 10 {
        out.push((b'0' + (value / 10)) as char);
    }
    out.push((b'0' + (value % 10)) as char);
}

fn taskbar_clock_lines(uptime_ticks: u64) -> (String, String) {
    if let Some(datetime) = crate::rtc::read_datetime() {
        let mut time = String::with_capacity(5);
        push_two_digits(&mut time, datetime.hour);
        time.push(':');
        push_two_digits(&mut time, datetime.minute);

        let mut date = String::with_capacity(10);
        push_u8_decimal(&mut date, datetime.month);
        date.push('/');
        push_u8_decimal(&mut date, datetime.day);
        date.push('/');
        push_u16(&mut date, datetime.year);
        return (time, date);
    }

    let secs = uptime_ticks / crate::interrupts::TIMER_HZ as u64;
    let h = ((secs / 3600) % 24) as u8;
    let m = ((secs / 60) % 60) as u8;
    let mut time = String::with_capacity(5);
    push_two_digits(&mut time, h);
    time.push(':');
    push_two_digits(&mut time, m);
    (time, String::from("--/--/----"))
}

fn push_usize_bytes(out: &mut Vec<u8>, mut value: usize) {
    if value == 0 {
        out.push(b'0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for idx in (0..len).rev() {
        out.push(digits[idx]);
    }
}

fn merge_spans(a: (usize, usize), b: (usize, usize)) -> (usize, usize) {
    match (a.0 < a.1, b.0 < b.1) {
        (false, false) => (0, 0),
        (true, false) => a,
        (false, true) => b,
        (true, true) => (a.0.min(b.0), a.1.max(b.1)),
    }
}

#[allow(dead_code)]
fn start_menu_banner_clock(uptime_ticks: u64) -> (String, String) {
    if let Some(datetime) = crate::rtc::read_datetime() {
        let mut time = String::with_capacity(5);
        push_two_digits(&mut time, datetime.hour);
        time.push(':');
        push_two_digits(&mut time, datetime.minute);

        let mut date = String::with_capacity(11);
        push_two_digits(&mut date, datetime.day);
        date.push(' ');
        date.push_str(month_abbrev(datetime.month));
        date.push(' ');
        push_u16(&mut date, datetime.year);
        return (time, date);
    }

    let secs = uptime_ticks / crate::interrupts::TIMER_HZ as u64;
    let h = ((secs / 3600) % 24) as u8;
    let m = ((secs / 60) % 60) as u8;

    let mut time = String::with_capacity(5);
    push_two_digits(&mut time, h);
    time.push(':');
    push_two_digits(&mut time, m);

    (time, String::from("RTC offline"))
}

// ── Drag state ────────────────────────────────────────────────────────────────

struct DragState {
    window: usize,
    off_x: i32,
    off_y: i32,
}

struct ResizeState {
    window: usize,
    start_w: i32,
    start_h: i32,
    start_mx: i32,
    start_my: i32,
}

struct ScrollDragState {
    window: usize,
    start_offset: i32,
    start_my: i32,
    content_h: i32,
    view_h: i32,
    track_h: i32,
}

struct FileDragState {
    source_window: usize,
    paths: Vec<String>,
    cut: bool,
}

struct DesktopIconDragState {
    icon: usize,
    start_mx: i32,
    start_my: i32,
    start_x: i32,
    start_y: i32,
    cur_x: i32,
    cur_y: i32,
    moved: bool,
}

#[derive(Clone, Copy)]
enum SnapTarget {
    Left,
    Right,
    Bottom,
    Maximize,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub enum AppWindow {
    Terminal(TerminalApp),
    SysMon(SysMonApp),
    TextViewer(TextViewerApp),
    Browser(BrowserApp),
    ColorPicker(ColorPickerApp),
    DisplaySettings(DisplaySettingsApp),
    Personalize(PersonalizeApp),
    FileManager(FileManagerApp),
    Utility(UtilityApp),
    UserGui(UserGuiApp),
}

impl AppWindow {
    pub fn window(&self) -> &Window {
        match self {
            AppWindow::Terminal(t) => &t.window,
            AppWindow::SysMon(s) => &s.window,
            AppWindow::TextViewer(v) => &v.window,
            AppWindow::Browser(b) => &b.window,
            AppWindow::ColorPicker(c) => &c.window,
            AppWindow::DisplaySettings(d) => &d.window,
            AppWindow::Personalize(p) => &p.window,
            AppWindow::FileManager(f) => &f.window,
            AppWindow::Utility(u) => &u.window,
            AppWindow::UserGui(g) => &g.window,
        }
    }
    pub fn window_mut(&mut self) -> &mut Window {
        match self {
            AppWindow::Terminal(t) => &mut t.window,
            AppWindow::SysMon(s) => &mut s.window,
            AppWindow::TextViewer(v) => &mut v.window,
            AppWindow::Browser(b) => &mut b.window,
            AppWindow::ColorPicker(c) => &mut c.window,
            AppWindow::DisplaySettings(d) => &mut d.window,
            AppWindow::Personalize(p) => &mut p.window,
            AppWindow::FileManager(f) => &mut f.window,
            AppWindow::Utility(u) => &mut u.window,
            AppWindow::UserGui(g) => &mut g.window,
        }
    }
    pub fn handle_key(&mut self, c: char) {
        match self {
            AppWindow::Terminal(t) => t.handle_key(c),
            AppWindow::SysMon(s) => s.handle_key(c),
            AppWindow::TextViewer(v) => v.handle_key(c),
            AppWindow::Browser(b) => b.handle_key(c),
            AppWindow::FileManager(f) => f.handle_key(c),
            AppWindow::Utility(u) => u.handle_key(c),
            AppWindow::UserGui(g) => g.handle_key(c),
            _ => {}
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn handle_key_input(&mut self, input: KeyInput) {
        match self {
            AppWindow::Terminal(t) => t.handle_key_input(input),
            _ => {
                if let Some(c) = input.legacy_char() {
                    self.handle_key(c);
                    return;
                }
            }
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        match self {
            AppWindow::SysMon(s) => s.handle_click(lx, ly),
            AppWindow::ColorPicker(cp) => cp.handle_click(lx, ly),
            AppWindow::DisplaySettings(ds) => ds.handle_click(lx, ly),
            AppWindow::FileManager(fm) => fm.handle_click(lx, ly),
            AppWindow::Personalize(p) => p.handle_click(lx, ly),
            AppWindow::Browser(b) => b.handle_click(lx, ly),
            AppWindow::Utility(u) => u.handle_click(lx, ly),
            AppWindow::UserGui(g) => g.handle_click(lx, ly),
            _ => {}
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn handle_secondary_click(&mut self, lx: i32, ly: i32) {
        if let AppWindow::FileManager(fm) = self {
            fm.handle_secondary_click(lx, ly);
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn handle_dbl_click(&mut self, lx: i32, ly: i32) {
        if let AppWindow::FileManager(fm) = self {
            fm.handle_dbl_click(lx, ly);
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn begin_file_drag(&mut self, lx: i32, ly: i32) -> Option<Vec<String>> {
        match self {
            AppWindow::FileManager(fm) => fm.drag_paths_at(lx, ly),
            _ => None,
        }
    }
    pub fn take_sysmon_request(&mut self) -> Option<SysMonRequest> {
        match self {
            AppWindow::SysMon(sysmon) => sysmon.take_request(),
            _ => None,
        }
    }
    pub fn drop_file_paths(&mut self, paths: Vec<String>, cut: bool) -> bool {
        match self {
            AppWindow::FileManager(fm) => fm.drop_paths(paths, cut),
            _ => false,
        }
    }
    pub fn take_open_request(&mut self) -> Option<FileManagerOpenRequest> {
        match self {
            AppWindow::Browser(browser) => browser.take_open_request(),
            AppWindow::FileManager(fm) => fm.take_open_request(),
            AppWindow::Utility(u) => u.take_open_request(),
            _ => None,
        }
    }
    pub fn take_browser_request(&mut self) -> Option<String> {
        match self {
            AppWindow::Terminal(term) => term.take_browser_request(),
            _ => None,
        }
    }
    pub fn handle_scroll(&mut self, delta: i32) {
        match self {
            AppWindow::TextViewer(v) => v.handle_scroll(delta),
            AppWindow::FileManager(f) => f.handle_scroll(delta),
            AppWindow::Browser(b) => b.handle_scroll(delta),
            AppWindow::Utility(u) => u.handle_scroll(delta),
            AppWindow::UserGui(g) => g.handle_scroll(delta),
            _ => {}
        }
        self.window_mut().mark_dirty_all();
    }
    pub fn update(&mut self) {
        match self {
            AppWindow::Terminal(t) => t.update(),
            AppWindow::SysMon(s) => s.update(),
            AppWindow::TextViewer(v) => v.update(),
            AppWindow::Browser(b) => b.update(),
            AppWindow::ColorPicker(c) => c.update(),
            AppWindow::DisplaySettings(d) => d.update(),
            AppWindow::FileManager(f) => f.update(),
            AppWindow::Personalize(p) => p.update(),
            AppWindow::Utility(u) => u.update(),
            AppWindow::UserGui(g) => g.update(),
        }
    }
    pub fn request_close(&mut self) -> bool {
        match self {
            AppWindow::UserGui(g) => {
                g.request_close();
                false
            }
            _ => true,
        }
    }
    pub fn user_gui_close_timeout_owner(&self, now: u64, timeout_ticks: u64) -> Option<usize> {
        match self {
            AppWindow::UserGui(g) if g.close_timed_out(now, timeout_ticks) => Some(g.owner()),
            _ => None,
        }
    }
    pub fn is_minimized(&self) -> bool {
        self.window().minimized
    }
}

// ── Window manager ────────────────────────────────────────────────────────────

pub struct WindowManager {
    pub windows: Vec<AppWindow>,
    window_workspaces: Vec<usize>,
    z_order: Vec<usize>,
    focused: Option<usize>,
    current_workspace: usize,
    next_user_gui_handle: u64,
    key_sink_fd: Option<usize>,
    key_sink_window: Option<usize>,
    drag: Option<DragState>,
    resize: Option<ResizeState>,
    scroll_drag: Option<ScrollDragState>,
    file_drag: Option<FileDragState>,
    prev_left: bool,
    prev_right: bool,
    context_menu: Option<ContextMenu>,
    desktop_show_icons: bool,
    desktop_compact_spacing: bool,
    desktop_sort: DesktopSortMode,
    icon_selected: Option<usize>,
    desktop_multi_selected: Vec<usize>,
    pressed_icon: Option<usize>,
    desktop_icon_drag: Option<DesktopIconDragState>,
    desktop_select_drag: Option<(i32, i32)>,
    start_menu_open: bool,
    start_power_menu_open: bool,
    start_search: StartSearchState,
    start_menu_pinned: Vec<String>,
    start_menu_entries: Vec<StartMenuEntry>,
    notification_center_open: bool,
    taskbar_menu: Option<TaskbarMenu>,
    dialog: Option<ShellDialog>,
    session_locked: bool,
    greeter_user: String,
    greeter_password: String,
    greeter_focus: GreeterFocus,
    greeter_message: String,
    greeter_error: bool,
    greeter_attempts: u32,
    first_boot_owner: String,
    first_boot_password: String,
    first_boot_confirm: String,
    first_boot_device: String,
    first_boot_focus: FirstBootFocus,
    first_boot_message: String,
    first_boot_error: bool,
    session_ready: bool,
    session_dirty: bool,
    last_session_save_tick: u64,
    last_click_tick: u64,
    last_click_window: Option<usize>,
    last_click_x: i32,
    last_click_y: i32,
    task_switcher_until_tick: u64,
    task_switcher_query: String,
    fps_window_start_tick: u64,
    fps_window_frames: u64,
    frame_ticks_peak: u64,
    /// Shadow buffer — screen_width × screen_height u32 pixels.
    shadow: Vec<u32>,
    prev_shadow: Vec<u32>,
    damage_spans: Vec<(usize, usize)>,
    reported_damage_spans: Vec<(usize, usize)>,
    damage_rows_last: usize,
    damage_pixels_last: usize,
    damage_frames: u64,
    full_damage_next: bool,
    cursor_drawn: bool,
    cursor_hw_x: i32,
    cursor_hw_y: i32,
    shadow_width: usize,
    shadow_height: usize,
    blit_scratch: Vec<u8>,
    /// Pre-baked wallpaper pixels — computed once in new(), blitted each frame.
    wallpaper: Vec<u32>,
    wallpaper_preset: WallpaperPreset,
}

impl WindowManager {
    #[inline(always)]
    pub fn new_boxed() -> Box<Self> {
        desktop_settings::load_from_disk();
        let settings = desktop_settings::snapshot();
        let w = crate::framebuffer::width();
        let h = crate::framebuffer::height();
        let taskbar_y = h - TASKBAR_H as usize;
        let wallpaper = build_wallpaper(w, taskbar_y, settings.wallpaper, true);
        crate::boot_splash::show(
            "allocating render buffer",
            13,
            crate::boot_splash::BOOT_PROGRESS_TOTAL,
        );
        let shadow = alloc::vec![0u32; w * h];
        crate::boot_watchdog::record("finalizing shell", 14);
        crate::profiler::record_boot_stage("finalizing shell", 14);
        let prev_shadow = alloc::vec![u32::MAX; w * h];
        let damage_spans = alloc::vec![(0usize, w); h];
        let reported_damage_spans = alloc::vec![(0usize, 0usize); h];

        let wm = Box::new(WindowManager {
            windows: Vec::new(),
            window_workspaces: Vec::new(),
            z_order: Vec::new(),
            focused: None,
            current_workspace: 0,
            next_user_gui_handle: 1,
            key_sink_fd: None,
            key_sink_window: None,
            drag: None,
            resize: None,
            scroll_drag: None,
            file_drag: None,
            prev_left: false,
            prev_right: false,
            context_menu: None,
            desktop_show_icons: settings.show_icons,
            desktop_compact_spacing: settings.compact_spacing,
            desktop_sort: settings.sort_mode,
            icon_selected: None,
            desktop_multi_selected: Vec::new(),
            pressed_icon: None,
            desktop_icon_drag: None,
            desktop_select_drag: None,
            start_menu_open: false,
            start_power_menu_open: false,
            start_search: StartSearchState {
                query: String::new(),
                focused: false,
                selected: 0,
                show_all: false,
            },
            start_menu_pinned: Vec::new(),
            start_menu_entries: Vec::new(),
            notification_center_open: false,
            taskbar_menu: None,
            dialog: None,
            session_locked: true,
            greeter_user: default_login_user_name(),
            greeter_password: String::new(),
            greeter_focus: GreeterFocus::Password,
            greeter_message: String::new(),
            greeter_error: false,
            greeter_attempts: 0,
            first_boot_owner: String::new(),
            first_boot_password: String::new(),
            first_boot_confirm: String::new(),
            first_boot_device: String::new(),
            first_boot_focus: FirstBootFocus::Owner,
            first_boot_message: String::new(),
            first_boot_error: false,
            session_ready: true,
            session_dirty: false,
            last_session_save_tick: 0,
            last_click_tick: 0,
            last_click_window: None,
            last_click_x: 0,
            last_click_y: 0,
            task_switcher_until_tick: 0,
            task_switcher_query: String::new(),
            fps_window_start_tick: 0,
            fps_window_frames: 0,
            frame_ticks_peak: 0,
            shadow,
            prev_shadow,
            damage_spans,
            reported_damage_spans,
            damage_rows_last: 0,
            damage_pixels_last: 0,
            damage_frames: 0,
            full_damage_next: true,
            cursor_drawn: false,
            cursor_hw_x: 0,
            cursor_hw_y: 0,
            shadow_width: w,
            shadow_height: h,
            blit_scratch: alloc::vec![0u8; w * 3],
            wallpaper,
            wallpaper_preset: settings.wallpaper,
        });
        wm
    }

    pub fn add_window(&mut self, w: AppWindow) {
        let idx = self.windows.len();
        self.windows.push(w);
        self.window_workspaces.push(self.current_workspace);
        self.z_order.push(idx);
        self.focused = Some(idx);
        self.notify_session_changed();
    }

    pub fn open_user_gui(&mut self, owner: usize, title: &str, width: u16, height: u16) -> u64 {
        let handle = self.next_user_gui_handle;
        self.next_user_gui_handle = self.next_user_gui_handle.wrapping_add(1).max(1);
        let title = user_gui_window_title(title);

        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let off = self.windows.len() as i32 * 18;
        let wx = (40 + off).min(self.shadow_width as i32 - width as i32 - 24);
        let wy = (48 + off).min(taskbar_y - height as i32 - TITLE_H - 16);
        let x = wx.max(8);
        let y = wy.max(8);
        self.add_window(AppWindow::UserGui(UserGuiApp::new(
            owner, handle, x, y, width, height, title,
        )));
        crate::app_lifecycle::record_app(title);
        crate::app_lifecycle::record_user_window(owner, title, handle);
        crate::notifications::push_transient(title, "window opened from ring 3");
        crate::wm::request_repaint();
        handle
    }

    pub fn present_user_gui(&mut self, owner: usize, handle: u64, pixels: &[u8]) -> bool {
        let Some(app) = self.find_user_gui_mut(owner, handle) else {
            return false;
        };
        if app.present(pixels) {
            crate::wm::request_repaint();
            true
        } else {
            false
        }
    }

    pub fn poll_user_gui_event(
        &mut self,
        owner: usize,
        handle: u64,
        out: &mut [u8],
    ) -> Option<usize> {
        self.find_user_gui_mut(owner, handle)?.poll_event(out)
    }

    pub fn user_gui_event_readiness(&self, owner: usize, handle: u64) -> Option<u64> {
        let idx = self.find_user_gui_index(owner, handle)?;
        match self.windows.get(idx) {
            Some(AppWindow::UserGui(app)) if app.has_pending_event() => {
                Some(crate::evented::EVENT_READ)
            }
            Some(AppWindow::UserGui(_)) => Some(0),
            _ => None,
        }
    }

    pub fn register_user_gui_event_waiter(
        &mut self,
        owner: usize,
        handle: u64,
        task_id: usize,
    ) -> bool {
        let Some(app) = self.find_user_gui_mut(owner, handle) else {
            return false;
        };
        app.register_event_waiter(task_id);
        true
    }

    pub fn unregister_user_gui_event_waiter(&mut self, owner: usize, handle: u64, task_id: usize) {
        if let Some(app) = self.find_user_gui_mut(owner, handle) {
            app.unregister_event_waiter(task_id);
        }
    }

    pub fn close_user_gui(&mut self, owner: usize, handle: u64) -> bool {
        let Some(idx) = self.find_user_gui_index(owner, handle) else {
            return false;
        };
        self.close_window(idx);
        crate::wm::request_repaint();
        true
    }

    pub fn close_user_gui_windows_for_owner(&mut self, owner: usize) {
        let mut idx = 0usize;
        let mut closed = 0usize;
        while idx < self.windows.len() {
            let owned =
                matches!(&self.windows[idx], AppWindow::UserGui(app) if app.owner() == owner);
            if owned {
                self.close_window(idx);
                closed += 1;
            } else {
                idx += 1;
            }
        }
        if closed > 0 {
            crate::wm::request_repaint();
        }
    }

    pub fn trim_browser_memory_pressure(&mut self) -> usize {
        let mut bytes = 0usize;
        for window in self.windows.iter_mut() {
            if let AppWindow::Browser(browser) = window {
                bytes = bytes.saturating_add(browser.trim_memory_pressure());
            }
        }
        if bytes > 0 {
            crate::wm::request_repaint();
        }
        bytes
    }

    fn find_user_gui_index(&self, owner: usize, handle: u64) -> Option<usize> {
        self.windows.iter().position(|window| {
            matches!(window, AppWindow::UserGui(app) if app.owner() == owner && app.handle() == handle)
        })
    }

    fn find_user_gui_mut(&mut self, owner: usize, handle: u64) -> Option<&mut UserGuiApp> {
        let idx = self.find_user_gui_index(owner, handle)?;
        match self.windows.get_mut(idx) {
            Some(AppWindow::UserGui(app)) => Some(app),
            _ => None,
        }
    }

    fn notify_session_changed(&mut self) {
        if self.session_ready {
            self.session_dirty = true;
        }
    }

    fn maybe_save_session(&mut self, ticks: u64) {
        if !self.session_ready || !self.session_dirty {
            return;
        }
        let interval = crate::interrupts::ticks_for_millis(SESSION_SAVE_MS);
        if self.last_session_save_tick != 0
            && ticks.wrapping_sub(self.last_session_save_tick) < interval
        {
            return;
        }
        self.save_session();
        self.session_dirty = false;
        self.last_session_save_tick = ticks;
    }

    fn clear_session_chrome(&mut self) {
        self.start_menu_open = false;
        self.start_power_menu_open = false;
        self.reset_start_search();
        self.notification_center_open = false;
        self.taskbar_menu = None;
        self.context_menu = None;
        self.dialog = None;
        self.task_switcher_until_tick = 0;
        self.task_switcher_query.clear();
        self.drag = None;
        self.resize = None;
        self.scroll_drag = None;
        self.file_drag = None;
        self.desktop_icon_drag = None;
        self.desktop_select_drag = None;
        self.pressed_icon = None;
        self.stop_key_sink();
    }

    fn lock_session(&mut self) {
        self.clear_session_chrome();
        self.session_locked = true;
        self.greeter_user = default_login_user_name();
        self.greeter_password.clear();
        self.greeter_focus = GreeterFocus::Password;
        self.greeter_message = String::from("Session locked");
        self.greeter_error = false;
        self.full_damage_next = true;
        crate::wm::request_repaint();
        crate::event_bus::emit("security", "lock", &self.greeter_user);
        crate::println!("[session] locked");
    }

    fn logout_session(&mut self) {
        let user = crate::security::logout();
        self.minimize_all_windows();
        self.clear_session_chrome();
        self.session_locked = true;
        self.greeter_user = user.name.clone();
        self.greeter_password.clear();
        self.greeter_focus = GreeterFocus::Password;
        self.greeter_message = String::from("Signed out");
        self.greeter_error = false;
        self.full_damage_next = true;
        crate::wm::request_repaint();
        crate::println!("[session] logout {}", user.name);
    }

    fn login_with_credentials(&mut self, name: &str, password: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            self.greeter_message = String::from("Enter a user name");
            self.greeter_error = true;
            self.greeter_focus = GreeterFocus::User;
            crate::wm::request_repaint();
            return false;
        }
        match crate::security::login(trimmed, password) {
            Ok(user) => {
                self.session_locked = false;
                self.greeter_user = user.name.clone();
                self.greeter_password.clear();
                self.greeter_focus = GreeterFocus::Password;
                self.greeter_message = format!("Welcome, {}", user.name);
                self.greeter_error = false;
                self.greeter_attempts = 0;
                self.full_damage_next = true;
                crate::notifications::push("Session", "signed in");
                crate::wm::request_repaint();
                crate::println!("[session] login {} uid={}", user.name, user.uid);
                true
            }
            Err(err) => {
                self.greeter_attempts = self.greeter_attempts.saturating_add(1);
                self.greeter_password.clear();
                self.greeter_focus = GreeterFocus::Password;
                self.greeter_message = format!("Login failed: {}", err.as_str());
                self.greeter_error = true;
                crate::wm::request_repaint();
                crate::println!("[session] login failed {}: {}", trimmed, err.as_str());
                false
            }
        }
    }

    fn try_greeter_login(&mut self) -> bool {
        let user = self.greeter_user.clone();
        let password = self.greeter_password.clone();
        self.login_with_credentials(&user, &password)
    }

    fn first_boot_required(&self) -> bool {
        crate::security::first_run_required() && !crate::fw_cfg::smoke_mode()
    }

    fn complete_first_boot_setup(&mut self) -> bool {
        let owner = String::from(self.first_boot_owner.trim());
        let password = self.first_boot_password.clone();
        let confirm = self.first_boot_confirm.clone();
        let device = if self.first_boot_device.trim().is_empty() {
            String::from("coolOS")
        } else {
            String::from(self.first_boot_device.trim())
        };

        if owner.is_empty() {
            self.first_boot_message = String::from("Enter an owner name");
            self.first_boot_focus = FirstBootFocus::Owner;
            self.first_boot_error = true;
            crate::wm::request_repaint();
            return false;
        }
        if password != confirm {
            self.first_boot_message = String::from("Passwords do not match");
            self.first_boot_password.clear();
            self.first_boot_confirm.clear();
            self.first_boot_focus = FirstBootFocus::Password;
            self.first_boot_error = true;
            crate::wm::request_repaint();
            return false;
        }

        match crate::security::complete_first_run_admin(&owner, &password) {
            Ok(user) => {
                if crate::security::mark_first_boot_complete(&user.name, &device).is_err() {
                    crate::println!("[install] first boot state write failed");
                }
                self.session_locked = false;
                self.greeter_user = user.name.clone();
                self.greeter_password.clear();
                self.greeter_focus = GreeterFocus::Password;
                self.greeter_message = format!("Welcome, {}", user.name);
                self.greeter_error = false;
                self.greeter_attempts = 0;
                self.first_boot_password.clear();
                self.first_boot_confirm.clear();
                self.first_boot_message.clear();
                self.first_boot_error = false;
                self.full_damage_next = true;
                crate::notifications::push("Setup complete", "owner account created");
                crate::wm::request_repaint();
                crate::println!(
                    "[install] first boot complete user={} device={}",
                    user.name,
                    device
                );
                crate::println!("[session] login {} uid={}", user.name, user.uid);
                true
            }
            Err(err) => {
                self.first_boot_message = format!("Setup failed: {}", err.as_str());
                self.first_boot_error = true;
                match err {
                    crate::security::AccountError::PasswordTooShort => {
                        self.first_boot_password.clear();
                        self.first_boot_confirm.clear();
                        self.first_boot_focus = FirstBootFocus::Password;
                    }
                    crate::security::AccountError::InvalidName
                    | crate::security::AccountError::DuplicateUser
                    | crate::security::AccountError::ProtectedUser => {
                        self.first_boot_focus = FirstBootFocus::Owner;
                    }
                    _ => {}
                }
                crate::wm::request_repaint();
                crate::println!("[install] first boot failed: {}", err.as_str());
                false
            }
        }
    }

    fn handle_first_boot_input(&mut self, input: KeyInput) {
        if input.has_ctrl() || input.has_alt() {
            return;
        }
        match input.key {
            Key::Tab => {
                let step = if input.modifiers & crate::keyboard::MOD_SHIFT != 0 {
                    -1
                } else {
                    1
                };
                self.cycle_first_boot_focus(step);
                self.first_boot_error = false;
                crate::wm::request_repaint();
            }
            Key::Enter => {
                self.advance_or_complete_first_boot();
            }
            Key::Backspace => {
                self.pop_first_boot_char();
                self.first_boot_error = false;
                crate::wm::request_repaint();
            }
            Key::Escape => {
                self.handle_first_boot_escape();
                crate::wm::request_repaint();
            }
            Key::ArrowUp => {
                self.cycle_first_boot_focus(-1);
                crate::wm::request_repaint();
            }
            Key::ArrowDown => {
                self.cycle_first_boot_focus(1);
                crate::wm::request_repaint();
            }
            Key::Space => self.push_first_boot_char(' '),
            Key::Character(c) => self.push_first_boot_char(c),
            _ => {}
        }
    }

    fn advance_or_complete_first_boot(&mut self) {
        match self.first_boot_focus {
            FirstBootFocus::Owner if !self.first_boot_owner.trim().is_empty() => {
                self.first_boot_focus = FirstBootFocus::Password;
                self.first_boot_error = false;
                crate::wm::request_repaint();
            }
            FirstBootFocus::Password if !self.first_boot_password.is_empty() => {
                self.first_boot_focus = FirstBootFocus::Confirm;
                self.first_boot_error = false;
                crate::wm::request_repaint();
            }
            FirstBootFocus::Confirm if !self.first_boot_confirm.is_empty() => {
                self.first_boot_focus = FirstBootFocus::Device;
                self.first_boot_error = false;
                crate::wm::request_repaint();
            }
            _ => {
                self.complete_first_boot_setup();
            }
        }
    }

    fn cycle_first_boot_focus(&mut self, step: i32) {
        let current = match self.first_boot_focus {
            FirstBootFocus::Owner => 0,
            FirstBootFocus::Password => 1,
            FirstBootFocus::Confirm => 2,
            FirstBootFocus::Device => 3,
        };
        self.first_boot_focus = match (current + step).rem_euclid(4) {
            0 => FirstBootFocus::Owner,
            1 => FirstBootFocus::Password,
            2 => FirstBootFocus::Confirm,
            _ => FirstBootFocus::Device,
        };
    }

    fn pop_first_boot_char(&mut self) {
        match self.first_boot_focus {
            FirstBootFocus::Owner => {
                self.first_boot_owner.pop();
            }
            FirstBootFocus::Password => {
                self.first_boot_password.pop();
            }
            FirstBootFocus::Confirm => {
                self.first_boot_confirm.pop();
            }
            FirstBootFocus::Device => {
                self.first_boot_device.pop();
            }
        }
    }

    fn handle_first_boot_escape(&mut self) {
        if !self.first_boot_message.is_empty() {
            self.first_boot_message.clear();
            self.first_boot_error = false;
            return;
        }
        match self.first_boot_focus {
            FirstBootFocus::Owner => {
                self.first_boot_message = String::from("Setup is required before sign in");
                self.first_boot_error = false;
            }
            FirstBootFocus::Password if !self.first_boot_password.is_empty() => {
                self.first_boot_password.clear();
            }
            FirstBootFocus::Confirm if !self.first_boot_confirm.is_empty() => {
                self.first_boot_confirm.clear();
            }
            FirstBootFocus::Device if !self.first_boot_device.is_empty() => {
                self.first_boot_device.clear();
            }
            _ => self.cycle_first_boot_focus(-1),
        }
    }

    fn push_first_boot_char(&mut self, c: char) {
        if c < ' ' || c == '\u{7f}' || c == ':' || c == '\n' || c == '\r' {
            return;
        }
        match self.first_boot_focus {
            FirstBootFocus::Owner => {
                if (c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    && self.first_boot_owner.chars().count() < 16
                {
                    self.first_boot_owner.push(c);
                }
            }
            FirstBootFocus::Password => {
                if self.first_boot_password.chars().count() < 64 {
                    self.first_boot_password.push(c);
                }
            }
            FirstBootFocus::Confirm => {
                if self.first_boot_confirm.chars().count() < 64 {
                    self.first_boot_confirm.push(c);
                }
            }
            FirstBootFocus::Device => {
                if (c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ')
                    && self.first_boot_device.chars().count() < 24
                {
                    self.first_boot_device.push(c);
                }
            }
        }
        self.first_boot_error = false;
        crate::wm::request_repaint();
    }

    fn run_locked_startup_command(&mut self, command: &str) -> bool {
        let mut words = command.split_whitespace();
        match words.next() {
            Some("login") | Some("su") => {
                let (Some(user), Some(password)) = (words.next(), words.next()) else {
                    self.greeter_message = String::from("Startup login missing credentials");
                    self.greeter_error = true;
                    crate::println!("[session] startup login missing credentials");
                    return true;
                };
                self.login_with_credentials(user, password);
                true
            }
            _ => false,
        }
    }

    fn select_greeter_user(&mut self, name: &str) {
        self.greeter_user.clear();
        self.greeter_user.push_str(name);
        self.greeter_password.clear();
        self.greeter_focus = GreeterFocus::Password;
        self.greeter_message = format!("{} selected", name);
        self.greeter_error = false;
        crate::wm::request_repaint();
    }

    fn cycle_greeter_user(&mut self, step: i32) {
        let users: Vec<_> = crate::security::users()
            .into_iter()
            .filter(|user| user.login_enabled)
            .collect();
        if users.is_empty() {
            return;
        }
        let current = users
            .iter()
            .position(|user| user.name.eq_ignore_ascii_case(&self.greeter_user))
            .unwrap_or(0);
        let len = users.len() as i32;
        let next = (current as i32 + step).rem_euclid(len) as usize;
        self.select_greeter_user(&users[next].name);
    }

    fn handle_greeter_input(&mut self, input: KeyInput) {
        if self.first_boot_required() {
            self.handle_first_boot_input(input);
            return;
        }
        if input.has_ctrl() || input.has_alt() {
            return;
        }
        match input.key {
            Key::Tab => {
                if input.modifiers & crate::keyboard::MOD_SHIFT != 0 {
                    self.greeter_focus = if self.greeter_focus == GreeterFocus::User {
                        GreeterFocus::Password
                    } else {
                        GreeterFocus::User
                    };
                } else {
                    self.greeter_focus = if self.greeter_focus == GreeterFocus::Password {
                        GreeterFocus::User
                    } else {
                        GreeterFocus::Password
                    };
                }
                self.greeter_error = false;
                crate::wm::request_repaint();
            }
            Key::Enter => {
                self.try_greeter_login();
            }
            Key::Backspace => {
                match self.greeter_focus {
                    GreeterFocus::User => {
                        self.greeter_user.pop();
                    }
                    GreeterFocus::Password => {
                        self.greeter_password.pop();
                    }
                }
                self.greeter_error = false;
                crate::wm::request_repaint();
            }
            Key::Escape => {
                self.greeter_password.clear();
                self.greeter_message = String::from("Session locked");
                self.greeter_error = false;
                self.greeter_focus = GreeterFocus::Password;
                crate::wm::request_repaint();
            }
            Key::ArrowUp => self.cycle_greeter_user(-1),
            Key::ArrowDown => self.cycle_greeter_user(1),
            Key::Space => self.push_greeter_char(' '),
            Key::Character(c) => self.push_greeter_char(c),
            _ => {}
        }
    }

    fn push_greeter_char(&mut self, c: char) {
        if c < ' ' || c == '\u{7f}' {
            return;
        }
        match self.greeter_focus {
            GreeterFocus::User => {
                if self.greeter_user.chars().count() < 32 {
                    self.greeter_user.push(c);
                }
            }
            GreeterFocus::Password => {
                if self.greeter_password.chars().count() < 64 {
                    self.greeter_password.push(c);
                }
            }
        }
        self.greeter_error = false;
        crate::wm::request_repaint();
    }

    fn handle_greeter_click(&mut self, mx: i32, my: i32, sw: i32, taskbar_y: i32) {
        if self.first_boot_required() {
            self.handle_first_boot_click(mx, my, sw, taskbar_y);
            return;
        }
        let layout = greeter_layout(sw, taskbar_y);
        if rect_contains(
            layout.user_x,
            layout.user_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.greeter_focus = GreeterFocus::User;
            self.greeter_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.user_x,
            layout.pass_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.greeter_focus = GreeterFocus::Password;
            self.greeter_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.user_x,
            layout.button_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.try_greeter_login();
            return;
        }

        let mut row = 0i32;
        for user in crate::security::users()
            .into_iter()
            .filter(|user| user.login_enabled)
        {
            let y = layout.users_y + row * GREETER_USER_ROW_H;
            if y + GREETER_USER_ROW_H > layout.panel_y + layout.panel_h - 8 {
                break;
            }
            if rect_contains(
                layout.user_x,
                y,
                layout.row_w,
                GREETER_USER_ROW_H - 3,
                mx,
                my,
            ) {
                self.select_greeter_user(&user.name);
                return;
            }
            row += 1;
            if row >= 4 {
                break;
            }
        }
    }

    fn handle_first_boot_click(&mut self, mx: i32, my: i32, sw: i32, taskbar_y: i32) {
        let layout = first_boot_layout(sw, taskbar_y);
        if rect_contains(
            layout.field_x,
            layout.owner_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.first_boot_focus = FirstBootFocus::Owner;
            self.first_boot_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.field_x,
            layout.pass_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.first_boot_focus = FirstBootFocus::Password;
            self.first_boot_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.field_x,
            layout.confirm_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.first_boot_focus = FirstBootFocus::Confirm;
            self.first_boot_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.field_x,
            layout.device_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.first_boot_focus = FirstBootFocus::Device;
            self.first_boot_error = false;
            crate::wm::request_repaint();
            return;
        }
        if rect_contains(
            layout.field_x,
            layout.button_y,
            layout.field_w,
            GREETER_FIELD_H,
            mx,
            my,
        ) {
            self.complete_first_boot_setup();
        }
    }

    fn save_session(&self) {
        let mut data = String::new();
        for (idx, window) in self.windows.iter().enumerate().take(MAX_SESSION_WINDOWS) {
            let win = window.window();
            crate::app_lifecycle::remember_geometry(win.title, win.x, win.y, win.width, win.height);
            data.push_str(win.title);
            data.push('|');
            push_i32_decimal(&mut data, win.x);
            data.push('|');
            push_i32_decimal(&mut data, win.y);
            data.push('|');
            push_i32_decimal(&mut data, win.width);
            data.push('|');
            push_i32_decimal(&mut data, win.height);
            data.push('|');
            if let AppWindow::FileManager(fm) = window {
                data.push_str(fm.current_path());
            }
            data.push('|');
            push_decimal(&mut data, self.window_workspace(idx) as u64);
            data.push('\n');
        }
        let _ = crate::config_store::safe_write(SESSION_PATH, data.as_bytes());
    }

    fn restore_session(&mut self) {
        let Some(bytes) = crate::config_store::read(SESSION_PATH) else {
            return;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            return;
        };

        let mut restored = 0usize;
        for line in text.lines() {
            if restored >= MAX_SESSION_WINDOWS {
                break;
            }
            let mut parts = line.split('|');
            let title = parts.next().unwrap_or("");
            let Some(x) = parts.next().and_then(parse_i32_field) else {
                continue;
            };
            let Some(y) = parts.next().and_then(parse_i32_field) else {
                continue;
            };
            let Some(width) = parts.next().and_then(parse_i32_field) else {
                continue;
            };
            let Some(height) = parts.next().and_then(parse_i32_field) else {
                continue;
            };
            let extra = parts.next().unwrap_or("");
            let workspace = parts
                .next()
                .and_then(parse_usize_field)
                .unwrap_or(0)
                .min(WORKSPACE_COUNT - 1);
            let before = self.windows.len();
            let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
            let screen_w = self.shadow_width as i32;
            let x = x.clamp(-screen_w + 80, screen_w - 80);
            let y = y.clamp(0, taskbar_y.saturating_sub(40));
            let width = width.clamp(180, self.shadow_width as i32);
            let height = height.clamp(TITLE_H + 80, taskbar_y.max(TITLE_H + 80));

            match canonical_app_title(title) {
                "File Manager" => {
                    let dir = if extra.is_empty() { "/" } else { extra };
                    self.launch_file_manager_at(dir, x, y);
                }
                "Terminal" | "System Monitor" | "Diagnostics" | "Text Viewer" | "Text Editor"
                | "Notes" | "Trash Bin" | "Screenshot" | "Web Browser" | "Color Picker"
                | "Display Settings" | "Accounts" | "Personalize" => self.launch_app(title, x, y),
                _ => {}
            }

            if self.windows.len() > before {
                let idx = self.windows.len() - 1;
                self.windows[idx]
                    .window_mut()
                    .set_bounds(x, y, width, height);
                if let Some(slot) = self.window_workspaces.get_mut(idx) {
                    *slot = workspace;
                }
                restored += 1;
            }
        }
        if restored > 0 {
            self.current_workspace = 0;
            self.focused = self.top_visible_window();
            crate::klog::log_owned(format!("desktop restored {} window(s)", restored));
        }
    }

    fn sync_desktop_settings(&mut self) {
        let settings = desktop_settings::snapshot();
        self.desktop_show_icons = settings.show_icons;
        self.desktop_compact_spacing = settings.compact_spacing;
        self.desktop_sort = settings.sort_mode;
        if !self.desktop_show_icons {
            self.icon_selected = None;
            self.desktop_multi_selected.clear();
            self.pressed_icon = None;
            self.desktop_icon_drag = None;
            self.desktop_select_drag = None;
        }
        if self.wallpaper_preset != settings.wallpaper {
            let taskbar_y = self.shadow_height.saturating_sub(TASKBAR_H as usize);
            self.wallpaper =
                build_wallpaper(self.shadow_width, taskbar_y, settings.wallpaper, false);
            self.wallpaper_preset = settings.wallpaper;
            self.full_damage_next = true;
        }
    }

    fn refresh_desktop_state(&mut self) {
        self.sync_desktop_settings();
        let taskbar_y = self.shadow_height.saturating_sub(TASKBAR_H as usize);
        self.wallpaper =
            build_wallpaper(self.shadow_width, taskbar_y, self.wallpaper_preset, false);
        self.full_damage_next = true;
        self.icon_selected = None;
        self.desktop_multi_selected.clear();
        self.pressed_icon = None;
        self.desktop_icon_drag = None;
        self.desktop_select_drag = None;
        for window in self.windows.iter_mut() {
            if let AppWindow::FileManager(fm) = window {
                fm.refresh_current_dir();
            }
        }
    }

    fn launch_app(&mut self, name: &str, wx: i32, wy: i32) {
        let before = self.windows.len();
        let title = canonical_app_title(name);
        if let Some(meta) = crate::app_metadata::app_by_name(title) {
            if !crate::packages::is_installed(meta.id) {
                self.show_error_dialog("App unavailable", "package is not installed");
                return;
            }
        }
        if let Some(permission) = crate::security::app_permission_for(title) {
            crate::notifications::push(
                "App permissions",
                &format!("{} requests {}", title, permission),
            );
        }
        match title {
            "Terminal" => self.add_window(AppWindow::Terminal(TerminalApp::new(wx, wy))),
            "System Monitor" => self.add_window(AppWindow::SysMon(SysMonApp::new(wx, wy))),
            "Diagnostics" => self.add_window(AppWindow::TextViewer(
                TextViewerApp::diagnostics_viewer(wx, wy),
            )),
            "Text Viewer" => self.add_window(AppWindow::TextViewer(TextViewerApp::new(wx, wy))),
            "Text Editor" => {
                if !self.spawn_user_gui_app("Text Editor", "/bin/editor") {
                    self.add_window(AppWindow::Utility(UtilityApp::text_editor(wx, wy)));
                }
            }
            "Notes" => {
                if !self.spawn_user_gui_app("Notes", "/bin/notes") {
                    self.add_window(AppWindow::Utility(UtilityApp::notes(wx, wy)));
                }
            }
            "Trash Bin" => {
                if !self.spawn_user_gui_app("Trash Bin", "/bin/trash") {
                    self.add_window(AppWindow::Utility(UtilityApp::trash_bin(wx, wy)));
                }
            }
            "Screenshot" => {
                if !self.spawn_user_gui_app("Screenshot", "/bin/screenshot") {
                    self.add_window(AppWindow::Utility(UtilityApp::screenshot(wx, wy)));
                }
            }
            "Web Browser" => self.add_window(AppWindow::Browser(BrowserApp::new(wx, wy))),
            "Color Picker" => self.add_window(AppWindow::ColorPicker(ColorPickerApp::new(wx, wy))),
            "Display Settings" => {
                self.add_window(AppWindow::DisplaySettings(DisplaySettingsApp::new(wx, wy)))
            }
            "Accounts" => self.add_window(AppWindow::DisplaySettings(
                DisplaySettingsApp::with_page(wx, wy, "accounts"),
            )),
            "File Manager" => self.launch_file_manager_at("/", wx, wy),
            "Personalize" => self.add_window(AppWindow::Personalize(PersonalizeApp::new(wx, wy))),
            "Crash Viewer" => {
                self.add_window(AppWindow::TextViewer(TextViewerApp::crash_viewer(wx, wy)))
            }
            "Log Viewer" => {
                self.add_window(AppWindow::TextViewer(TextViewerApp::log_viewer(wx, wy)))
            }
            "Boot Profiler" => self.add_window(AppWindow::TextViewer(
                TextViewerApp::profiler_viewer(wx, wy),
            )),
            "Welcome" => self.add_window(AppWindow::TextViewer(TextViewerApp::welcome(wx, wy))),
            "GUI Demo" => {
                match self.spawn_user_gui_app_with_args_result("GUI Demo", "/bin/guidemo", &[]) {
                    Ok(pid) => {
                        crate::app_lifecycle::record_process_start(pid, "GUI Demo", "/bin/guidemo");
                        crate::app_lifecycle::record_app("GUI Demo");
                        crate::notifications::push_transient("GUI Demo", "spawned /bin/guidemo");
                    }
                    Err(err) => self.show_error_dialog("GUI Demo failed", err.as_str()),
                }
            }
            "Process Demo" => match crate::elf::spawn_elf_process_with_args("/bin/procdemo", &[]) {
                Ok(pid) => {
                    let job = crate::jobs::start_process("Process Demo", "/bin/procdemo", pid);
                    crate::app_lifecycle::record_app("Process Demo");
                    crate::notifications::push_transient(
                        "Process Demo",
                        &format!("pid {} job {}", pid, job),
                    );
                }
                Err(err) => self.show_error_dialog("Process Demo failed", err.as_str()),
            },
            _ => self.launch_manifest_app(title),
        }
        if self.windows.len() > before {
            crate::app_lifecycle::record_app(title);
            self.apply_remembered_geometry(self.windows.len() - 1);
        }
    }

    fn launch_manifest_app(&mut self, name: &str) {
        let Some(manifest) = crate::app_metadata::installed_manifest_by_id_or_command(name) else {
            return;
        };
        if crate::app_metadata::is_builtin_id(&manifest.id) {
            return;
        }
        match crate::packages::launch(&manifest.id, &[]) {
            Ok(launch) => {
                crate::notifications::push_transient(&launch.name, &launch.exec_path);
            }
            Err(err) => self.show_error_dialog("Package launch failed", err),
        }
    }

    fn spawn_user_gui_app(&mut self, name: &'static str, path: &'static str) -> bool {
        self.spawn_user_gui_app_with_args(name, path, &[])
    }

    fn spawn_user_gui_app_with_args(&mut self, name: &str, path: &str, args: &[&str]) -> bool {
        self.spawn_user_gui_app_with_args_result(name, path, args)
            .is_ok()
    }

    fn spawn_user_gui_app_with_args_result(
        &mut self,
        name: &str,
        path: &str,
        args: &[&str],
    ) -> Result<usize, crate::elf::ExecError> {
        let permission = crate::security::app_permission_for(name);
        let result = if let Some(permission) = permission {
            let credentials = crate::security::package_credentials(&permission);
            crate::elf::spawn_elf_process_with_credentials(path, args, credentials)
        } else {
            crate::elf::spawn_elf_process_with_args(path, args)
        };
        match result {
            Ok(pid) => {
                crate::notifications::push_transient(name, path);
                Ok(pid)
            }
            Err(err) => {
                if !matches!(err, crate::elf::ExecError::SchedulerBusy) {
                    crate::notifications::push_transient(name, err.as_str());
                }
                Err(err)
            }
        }
    }

    fn apply_remembered_geometry(&mut self, win_idx: usize) {
        if win_idx >= self.windows.len() {
            return;
        }
        let title = self.windows[win_idx].window().title;
        let Some(geometry) = crate::app_lifecycle::geometry_for(title) else {
            return;
        };
        self.windows[win_idx]
            .window_mut()
            .set_bounds(geometry.x, geometry.y, geometry.w, geometry.h);
    }

    fn window_workspace(&self, win_idx: usize) -> usize {
        self.window_workspaces
            .get(win_idx)
            .copied()
            .unwrap_or(0)
            .min(WORKSPACE_COUNT - 1)
    }

    fn is_window_on_current_workspace(&self, win_idx: usize) -> bool {
        self.window_workspace(win_idx) == self.current_workspace
    }

    fn top_visible_window(&self) -> Option<usize> {
        self.z_order.iter().rev().copied().find(|&idx| {
            idx < self.windows.len()
                && self.is_window_on_current_workspace(idx)
                && !self.windows[idx].is_minimized()
        })
    }

    fn switch_workspace(&mut self, workspace: usize) -> bool {
        let workspace = workspace.min(WORKSPACE_COUNT - 1);
        if self.current_workspace == workspace {
            return false;
        }
        self.current_workspace = workspace;
        self.focused = self.top_visible_window();
        self.drag = None;
        self.resize = None;
        self.scroll_drag = None;
        self.file_drag = None;
        self.desktop_icon_drag = None;
        self.desktop_select_drag = None;
        self.context_menu = None;
        self.start_menu_open = false;
        self.start_power_menu_open = false;
        self.taskbar_menu = None;
        self.notification_center_open = false;
        true
    }

    fn launch_file_manager_at(&mut self, dir: &str, wx: i32, wy: i32) {
        crate::app_lifecycle::record_app("File Manager");
        let app = if dir == "/" {
            FileManagerApp::new(wx, wy)
        } else {
            FileManagerApp::new_at_path(wx, wy, dir)
        };
        self.add_window(AppWindow::FileManager(app));
    }

    fn focus_window(&mut self, win_idx: usize) {
        if win_idx >= self.windows.len() {
            return;
        }
        self.current_workspace = self.window_workspace(win_idx);
        if self.windows[win_idx].is_minimized() {
            self.windows[win_idx].window_mut().restore();
        }
        if let Some(z_pos) = self.z_order.iter().position(|&i| i == win_idx) {
            self.z_order.remove(z_pos);
        }
        self.z_order.push(win_idx);
        self.focused = Some(win_idx);
    }

    fn reset_start_search(&mut self) {
        self.start_search.query.clear();
        self.start_search.focused = false;
        self.start_search.selected = 0;
        self.start_search.show_all = false;
    }

    fn close_start_menu(&mut self) {
        self.start_menu_open = false;
        self.start_power_menu_open = false;
        self.reset_start_search();
    }

    fn open_start_menu_search(&mut self) {
        self.refresh_start_menu_cache();
        self.start_menu_open = true;
        self.start_power_menu_open = false;
        self.start_search.query.clear();
        self.start_search.focused = true;
        self.start_search.selected = 0;
        self.start_search.show_all = false;
        self.context_menu = None;
        self.taskbar_menu = None;
        self.notification_center_open = false;
    }

    fn toggle_start_menu(&mut self) {
        if self.start_menu_open {
            self.close_start_menu();
        } else {
            self.refresh_start_menu_cache();
            self.start_menu_open = true;
            self.reset_start_search();
        }
        self.start_power_menu_open = false;
        self.context_menu = None;
        self.taskbar_menu = None;
        self.notification_center_open = false;
    }

    fn refresh_start_menu_cache(&mut self) {
        self.start_menu_pinned = crate::app_lifecycle::pinned_apps();
        self.start_menu_entries = build_start_menu_entries();
    }

    fn handle_start_menu_input(&mut self, input: KeyInput) -> bool {
        if !self.start_menu_open {
            return false;
        }

        match input.key {
            Key::Escape => {
                if !self.start_search.query.is_empty() || self.start_search.show_all {
                    self.start_search.query.clear();
                    self.start_search.selected = 0;
                    self.start_search.show_all = false;
                    self.start_search.focused = true;
                } else {
                    self.close_start_menu();
                }
                return true;
            }
            Key::Backspace
                if self.start_search.focused && !input.has_ctrl() && !input.has_alt() =>
            {
                self.start_search.query.pop();
                self.start_search.selected = 0;
                self.start_search.show_all = false;
                return true;
            }
            Key::ArrowUp if self.start_search.show_all || !self.start_search.query.is_empty() => {
                self.start_search.selected = self.start_search.selected.saturating_sub(1);
                return true;
            }
            Key::ArrowDown if self.start_search.show_all || !self.start_search.query.is_empty() => {
                let count = start_menu_results(&self.start_search).len();
                if count > 0 {
                    self.start_search.selected = (self.start_search.selected + 1).min(count - 1);
                }
                return true;
            }
            Key::Enter if self.start_search.show_all || !self.start_search.query.is_empty() => {
                self.activate_start_search_selection();
                return true;
            }
            Key::Space if !input.has_ctrl() && !input.has_alt() => {
                self.start_search.focused = true;
                self.start_search.show_all = false;
                self.start_search.query.push(' ');
                self.start_search.selected = 0;
                return true;
            }
            Key::Character(c) if !input.has_ctrl() && !input.has_alt() => {
                if c >= ' ' && c != '\u{7f}' {
                    self.start_search.focused = true;
                    self.start_search.show_all = false;
                    self.start_search.query.push(c);
                    self.start_search.selected = 0;
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn activate_start_search_selection(&mut self) {
        let results = start_menu_results(&self.start_search);
        if results.is_empty() {
            return;
        }
        let entry = results[self.start_search.selected.min(results.len() - 1)].clone();
        let query = self.start_search.query.clone();
        self.close_start_menu();
        if !query.trim().is_empty() {
            crate::app_lifecycle::record_search(&query);
        }
        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let off = self.windows.len() as i32 * 16;
        let wx = (16 + off).min(self.shadow_width as i32 - 220);
        let wy = (16 + off).min(taskbar_y - 120);
        self.activate_start_search_kind(&entry.kind, wx, wy);
    }

    fn activate_start_search_kind(&mut self, kind: &StartSearchKind, wx: i32, wy: i32) {
        match kind {
            StartSearchKind::App(app) => self.launch_app(app, wx, wy),
            StartSearchKind::Path(path) => {
                self.open_associated_path(path, wx, wy);
            }
            StartSearchKind::Command(command) => {
                self.run_terminal_command(command);
            }
            StartSearchKind::Inline(action) => self.run_start_inline_action(action, wx, wy),
        }
    }

    fn run_start_inline_action(&mut self, action: &str, wx: i32, wy: i32) {
        if action == "refresh-index" {
            crate::search_index::refresh();
            crate::notifications::push("Search", "desktop index refreshed");
        } else if action == "restore-session" {
            self.restore_session();
            crate::notifications::push("Session", "restore requested");
        } else if action == "restart-desktop" {
            self.refresh_desktop_state();
            crate::notifications::push("Desktop", "shell state refreshed");
        } else if action == "test-crash-dialog" {
            self.show_crash_dialog(
                "App launch failed",
                "diagnostic crash dialog preview",
                Some("Diagnostics"),
            );
        } else if action == "lock" {
            self.lock_session();
        } else if action == "logout" {
            self.logout_session();
        } else if action == "sleep" {
            match crate::acpi::sleep() {
                Ok(()) => crate::notifications::push("Power", "sleep requested"),
                Err(err) => self.show_error_dialog("Sleep unavailable", err),
            }
        } else if action == "shutdown" {
            match crate::acpi::shutdown() {
                Ok(()) => crate::notifications::push("Power", "shutdown requested"),
                Err(err) => self.show_error_dialog("Shutdown unavailable", err),
            }
        } else if action == "reboot" || action == "restart" {
            crate::notifications::push("Power", "reboot requested");
            crate::acpi::reboot();
        } else if let Some(url) = action.strip_prefix("browser-url:") {
            self.add_window(AppWindow::Browser(BrowserApp::open_url(wx, wy, url)));
            crate::app_lifecycle::record_app("Web Browser");
        } else if let Some(page) = action.strip_prefix("settings:") {
            self.add_window(AppWindow::DisplaySettings(DisplaySettingsApp::with_page(
                wx, wy, page,
            )));
            crate::app_lifecycle::record_app("Display Settings");
        } else if let Some(category) = action.strip_prefix("category:") {
            self.start_menu_open = true;
            self.start_search.query.clear();
            self.start_search.query.push_str(category);
            self.start_search.focused = true;
            self.start_search.selected = 0;
            self.start_search.show_all = false;
        } else if let Some(query) = action.strip_prefix("search:") {
            self.start_menu_open = true;
            self.start_search.query = String::from(query);
            self.start_search.focused = true;
            self.start_search.selected = 0;
            self.start_search.show_all = false;
        }
    }

    fn activate_start_item(&mut self, item: &str, wx: i32, wy: i32) {
        let kind = start_item_kind(item);
        self.activate_start_search_kind(&kind, wx, wy);
    }

    #[allow(dead_code)]
    fn activate_start_menu_quick_action(&mut self, action: &str, wx: i32, wy: i32) {
        if let Some(app) = action.strip_prefix("app:") {
            self.launch_app(app, wx, wy);
        } else if let Some(path) = action.strip_prefix("path:") {
            self.launch_file_manager_at(path, wx, wy);
        } else {
            self.run_start_inline_action(action, wx, wy);
        }
    }

    fn activate_win7_start_action(&mut self, action: Win7StartAction, wx: i32, wy: i32) {
        match action {
            Win7StartAction::App(app) => self.launch_app(app, wx, wy),
            Win7StartAction::Path(path) => self.launch_file_manager_at(path, wx, wy),
        }
    }

    fn quick_launch_pinned(&mut self, slot: usize) -> bool {
        let pinned = crate::app_lifecycle::pinned_apps();
        let Some(item) = pinned.get(slot) else {
            return false;
        };
        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let off = self.windows.len() as i32 * 16;
        let wx = (16 + off).min(self.shadow_width as i32 - 220);
        let wy = (16 + off).min(taskbar_y - 120);
        self.activate_start_item(item, wx, wy);
        true
    }

    fn open_associated_path(&mut self, path: &str, wx: i32, wy: i32) {
        let info = crate::vfs::inspect_path(path);
        if info.kind == crate::vfs::PathKind::Directory {
            self.launch_file_manager_at(path, wx, wy);
            return;
        }
        crate::app_lifecycle::record_file(path);
        match crate::app_metadata::association_for(path, false) {
            crate::app_metadata::Association::Executable => {
                if let Err(err) = crate::elf::spawn_elf_process(path) {
                    let body = err.as_str();
                    self.print_to_terminal("exec failed: ");
                    self.print_to_terminal(body);
                    self.print_to_terminal("\n");
                    self.show_crash_dialog("App launch failed", body, Some(path));
                }
            }
            crate::app_metadata::Association::AppShortcut(app) => self.launch_app(&app, wx, wy),
            crate::app_metadata::Association::Text | crate::app_metadata::Association::Unknown => {
                self.open_text_path_with_editor(path, wx, wy);
            }
            crate::app_metadata::Association::Directory => {
                self.launch_file_manager_at(path, wx, wy)
            }
        }
    }

    fn open_text_path_with_editor(&mut self, path: &str, wx: i32, wy: i32) {
        crate::app_lifecycle::record_file(path);
        if self.spawn_user_gui_app_with_args("Text Editor", "/bin/editor", &[path]) {
            crate::notifications::push_transient("Opening in Text Editor", path);
            return;
        }
        match TextViewerApp::open_file(wx, wy, path) {
            Ok(viewer) => self.add_window(AppWindow::TextViewer(viewer)),
            Err(err) => {
                self.print_to_terminal("open failed: ");
                self.print_to_terminal(err);
                self.print_to_terminal("\n");
                self.show_error_dialog("Open failed", err);
            }
        }
    }

    fn print_to_terminal(&mut self, msg: &str) {
        if let Some(term) = self.windows.iter_mut().find_map(|w| match w {
            AppWindow::Terminal(t) => Some(t),
            _ => None,
        }) {
            term.print_str(msg);
        }
    }

    fn show_error_dialog(&mut self, title: &str, body: &str) {
        self.dialog = Some(ShellDialog {
            title: String::from(title),
            body: String::from(body),
            kind: ShellDialogKind::Error,
            restart_target: None,
        });
        crate::notifications::push(title, body);
        self.close_start_menu();
        self.context_menu = None;
        self.taskbar_menu = None;
    }

    fn show_crash_dialog(&mut self, title: &str, body: &str, restart_target: Option<&str>) {
        self.dialog = Some(ShellDialog {
            title: String::from(title),
            body: String::from(body),
            kind: ShellDialogKind::Crash,
            restart_target: restart_target.map(String::from),
        });
        crate::notifications::push(title, body);
        self.close_start_menu();
        self.context_menu = None;
        self.taskbar_menu = None;
    }

    fn run_startup_exec_command(&mut self, command: &str) -> bool {
        let mut words = command.split_whitespace();
        if words.next() != Some("exec") {
            return false;
        }
        let Some(path) = words.next() else {
            return false;
        };
        let args: Vec<&str> = words.collect();
        let result = match path {
            "/bin/editor" => {
                self.spawn_user_gui_app_with_args_result("Text Editor", "/bin/editor", &args)
            }
            "/bin/notes" => self.spawn_user_gui_app_with_args_result("Notes", "/bin/notes", &args),
            "/bin/trash" => {
                self.spawn_user_gui_app_with_args_result("Trash Bin", "/bin/trash", &args)
            }
            "/bin/screenshot" => {
                self.spawn_user_gui_app_with_args_result("Screenshot", "/bin/screenshot", &args)
            }
            _ => return false,
        };
        if matches!(result, Err(crate::elf::ExecError::SchedulerBusy)) {
            crate::wm::queue_startup_command(command);
        }
        true
    }

    pub fn run_terminal_command(&mut self, command: &str) {
        let mut idx = self
            .windows
            .iter()
            .position(|w| matches!(w, AppWindow::Terminal(_)));
        if idx.is_none() {
            let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
            let off = self.windows.len() as i32 * 16;
            let wx = (16 + off).min(self.shadow_width as i32 - 220);
            let wy = (16 + off).min(taskbar_y - 120);
            self.launch_app("Terminal", wx, wy);
            idx = self.windows.len().checked_sub(1);
        }
        if let Some(win_idx) = idx {
            self.focus_window(win_idx);
            if let AppWindow::Terminal(term) = &mut self.windows[win_idx] {
                if term.is_busy() {
                    crate::wm::queue_startup_command(command);
                    return;
                }
                term.execute_command(command);
            }
        }
    }

    fn launch_browser_url(&mut self, url: &str) {
        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let off = self.windows.len() as i32 * 16;
        let wx = (32 + off).min(self.shadow_width as i32 - 260).max(8);
        let wy = (24 + off).min(taskbar_y - 180).max(8);
        self.add_window(AppWindow::Browser(BrowserApp::open_url(wx, wy, url)));
        crate::println!("[browser] open {}", url);
        crate::app_lifecycle::record_app("Web Browser");
    }

    fn toggle_notification_center(&mut self) {
        self.notification_center_open = !self.notification_center_open;
        if self.notification_center_open {
            crate::notifications::mark_all_read();
            self.close_start_menu();
            self.context_menu = None;
            self.taskbar_menu = None;
        }
    }

    fn handle_notification_center_click(
        &mut self,
        mx: i32,
        my: i32,
        sw: i32,
        taskbar_y: i32,
    ) -> bool {
        if !self.notification_center_open {
            return false;
        }
        let layout = notification_center_layout(sw, taskbar_y);
        if !layout.contains(mx, my) {
            return false;
        }

        let rows = layout.max_rows();
        let notes = crate::notifications::latest(rows);
        if layout.clear_contains(mx, my) && !notes.is_empty() {
            crate::notifications::clear();
            return true;
        }

        for (row, note) in notes.iter().rev().enumerate() {
            if layout.dismiss_contains(row, mx, my) {
                let _ = crate::notifications::dismiss(note.id);
                return true;
            }
        }

        true
    }

    fn consume_window_open_request(&mut self, win_idx: usize, sw: usize, taskbar_y: i32) {
        let open_request = self
            .windows
            .get_mut(win_idx)
            .and_then(AppWindow::take_open_request);
        let mut handled = false;

        if let Some(request) = open_request {
            handled = true;
            match request {
                FileManagerOpenRequest::Dir(path) => {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (20 + off).min(sw as i32 - 220);
                    let wy = (20 + off).min(taskbar_y - 120);
                    self.launch_file_manager_at(&path, wx, wy);
                }
                FileManagerOpenRequest::File(path) => {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (20 + off).min(sw as i32 - 220);
                    let wy = (20 + off).min(taskbar_y - 120);
                    self.open_text_path_with_editor(&path, wx, wy);
                }
                FileManagerOpenRequest::ViewFile(path) => {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (20 + off).min(sw as i32 - 220);
                    let wy = (20 + off).min(taskbar_y - 120);
                    crate::app_lifecycle::record_file(&path);
                    match TextViewerApp::open_file(wx, wy, &path) {
                        Ok(viewer) => self.add_window(AppWindow::TextViewer(viewer)),
                        Err(err) => self.show_error_dialog("Open failed", err),
                    }
                }
                FileManagerOpenRequest::Exec(path) => {
                    crate::app_lifecycle::record_file(&path);
                    if let Err(err) = crate::elf::spawn_elf_process(&path) {
                        let body = err.as_str();
                        if let Some(term) = self.windows.iter_mut().find_map(|w| match w {
                            AppWindow::Terminal(t) => Some(t),
                            _ => None,
                        }) {
                            term.print_str("exec failed: ");
                            term.print_str(body);
                            term.print_char('\n');
                        }
                        self.show_crash_dialog("App launch failed", body, Some(&path));
                    }
                }
                FileManagerOpenRequest::App(app) => {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (10 + off).min(sw as i32 - 220);
                    let wy = (10 + off).min(taskbar_y - 120);
                    self.launch_app(&app, wx, wy);
                }
            }
        }

        if win_idx < self.windows.len() {
            if let Some(request) = self.windows[win_idx].take_sysmon_request() {
                handled = true;
                self.handle_sysmon_request(request, sw, taskbar_y);
            }
        }

        if handled {
            crate::wm::request_repaint();
        }
    }

    fn handle_sysmon_request(&mut self, request: SysMonRequest, sw: usize, taskbar_y: i32) {
        match request {
            SysMonRequest::ClosePid(pid) => {
                if let Some(idx) = self.windows.iter().position(
                    |window| matches!(window, AppWindow::UserGui(app) if app.owner() == pid),
                ) {
                    self.request_or_close_window(idx);
                    crate::notifications::push_transient(
                        "App close requested",
                        &format!("pid {}", pid),
                    );
                } else {
                    crate::notifications::push_transient(
                        "App close unavailable",
                        &format!("pid {} has no GUI window", pid),
                    );
                }
            }
            SysMonRequest::KillPid(pid) => match crate::scheduler::kill_task(pid, 130) {
                Ok(()) => {
                    crate::notifications::push_transient("App killed", &format!("pid {}", pid))
                }
                Err(err) => crate::notifications::push_transient("Kill failed", err.as_str()),
            },
            SysMonRequest::OpenPath(path) => {
                let off = self.windows.len() as i32 * 16;
                let wx = (20 + off).min(sw as i32 - 220);
                let wy = (20 + off).min(taskbar_y - 120);
                self.launch_file_manager_at(parent_path(&path), wx, wy);
            }
        }
    }

    fn desktop_icons(&self) -> Vec<DesktopIcon> {
        if !self.desktop_show_icons {
            return Vec::new();
        }

        let mut specs = DESKTOP_ICON_SPECS.to_vec();
        match self.desktop_sort {
            DesktopSortMode::Default => {}
            DesktopSortMode::Name => {
                specs.sort_by(|a, b| a.label.cmp(b.label));
            }
            DesktopSortMode::Type => {
                specs.sort_by(|a, b| {
                    a.type_rank
                        .cmp(&b.type_rank)
                        .then_with(|| a.label.cmp(b.label))
                });
            }
        }

        let step_x = if self.desktop_compact_spacing {
            140
        } else {
            168
        };
        let step_y = if self.desktop_compact_spacing { 88 } else { 98 };
        let cols = 3i32;

        specs
            .into_iter()
            .enumerate()
            .map(|(i, spec)| DesktopIcon {
                x: self
                    .desktop_icon_drag
                    .as_ref()
                    .filter(|drag| drag.icon == i)
                    .map(|drag| drag.cur_x)
                    .or_else(|| desktop_settings::icon_position(spec.label).map(|pos| pos.0))
                    .unwrap_or(20 + (i as i32 % cols) * step_x),
                y: self
                    .desktop_icon_drag
                    .as_ref()
                    .filter(|drag| drag.icon == i)
                    .map(|drag| drag.cur_y)
                    .or_else(|| desktop_settings::icon_position(spec.label).map(|pos| pos.1))
                    .unwrap_or(20 + (i as i32 / cols) * step_y),
                label: spec.label,
                app: spec.app,
            })
            .collect()
    }

    fn desktop_icon_hit(&self, px: i32, py: i32) -> Option<usize> {
        self.desktop_icons()
            .iter()
            .position(|icon| icon.hit(px, py))
    }

    fn taskbar_button_hit(&self, px: i32, py: i32, sw: i32, taskbar_y: i32) -> Option<usize> {
        let show_desktop_x = sw - TASKBAR_CLOCK_W - SHOW_DESKTOP_W - 8;
        let taskbar_btn_x0 = START_BTN_W + 8;
        if px < taskbar_btn_x0 || px >= show_desktop_x - 6 {
            return None;
        }
        if py < taskbar_y + 2 || py >= taskbar_y + TASKBAR_H {
            return None;
        }
        let slot = ((px - taskbar_btn_x0) / (BUTTON_W + 6)) as usize;
        let bx = taskbar_btn_x0 + slot as i32 * (BUTTON_W + 6);
        if px >= bx + BUTTON_W {
            return None;
        }
        let mut current_slot = 0usize;
        for win_idx in 0..self.windows.len() {
            if !self.is_window_on_current_workspace(win_idx) {
                continue;
            }
            if current_slot == slot {
                return Some(win_idx);
            }
            current_slot += 1;
        }
        None
    }

    fn open_taskbar_menu(&mut self, win_idx: usize, mx: i32, taskbar_y: i32, sw: i32) {
        if win_idx >= self.windows.len() {
            return;
        }
        let x = mx.min(sw - TASKBAR_MENU_W - 4).max(4);
        let y = (taskbar_y - TASKBAR_MENU_H - 4).max(0);
        self.taskbar_menu = Some(TaskbarMenu {
            window: win_idx,
            x,
            y,
        });
        self.start_menu_open = false;
        self.start_power_menu_open = false;
        self.reset_start_search();
        self.context_menu = None;
    }

    fn handle_taskbar_menu_click(&mut self, mx: i32, my: i32) -> bool {
        let Some(menu) = self.taskbar_menu.take() else {
            return false;
        };
        if mx < menu.x
            || mx >= menu.x + TASKBAR_MENU_W
            || my < menu.y
            || my >= menu.y + TASKBAR_MENU_H
        {
            return false;
        }
        if menu.window >= self.windows.len() {
            return true;
        }
        let row_y = menu.y + 5;
        let row = ((my - row_y) / TASKBAR_MENU_ROW_H).clamp(0, 2);
        match row {
            0 => {
                if self.windows[menu.window].is_minimized() {
                    self.windows[menu.window].window_mut().restore();
                    self.focus_window(menu.window);
                } else {
                    self.windows[menu.window].window_mut().minimize();
                    if self.focused == Some(menu.window) {
                        self.focused = self.top_visible_window();
                    }
                }
            }
            1 => {
                self.snap_window(menu.window, SnapTarget::Maximize);
            }
            _ => {
                self.request_or_close_window(menu.window);
            }
        }
        true
    }

    fn open_context_menu(&mut self, mx: i32, my: i32, sw: i32, taskbar_y: i32) {
        let menu_h = ctx_menu_height(DESKTOP_CONTEXT_MENU);
        let cx = mx.min(sw - CTX_W).max(0);
        let cy = my.min(taskbar_y - menu_h).max(0);
        self.context_menu = Some(ContextMenu {
            x: cx,
            y: cy,
            submenu: None,
        });
    }

    fn update_context_menu_hover(&mut self, mx: i32, my: i32, sw: i32, taskbar_y: i32) {
        let Some(cm) = self.context_menu.as_mut() else {
            return;
        };

        if let Some(idx) = ctx_menu_hit_index(DESKTOP_CONTEXT_MENU, cm.x, cm.y, CTX_W, mx, my) {
            if let ContextEntryKind::Submenu(submenu) = DESKTOP_CONTEXT_MENU[idx].kind {
                cm.submenu = Some(submenu);
            } else {
                cm.submenu = None;
            }
            return;
        }

        if let Some(submenu) = cm.submenu {
            let (sub_x, sub_y, sub_w, sub_h) = ctx_submenu_rect(cm.x, cm.y, submenu, sw, taskbar_y);
            if mx < sub_x || mx >= sub_x + sub_w || my < sub_y || my >= sub_y + sub_h {
                cm.submenu = None;
            }
        }
    }

    fn handle_context_menu_click(&mut self, mx: i32, my: i32, sw: i32, taskbar_y: i32) -> bool {
        let Some(ref cm) = self.context_menu else {
            return false;
        };
        let cm_x = cm.x;
        let cm_y = cm.y;
        let cm_submenu = cm.submenu;

        if let Some(submenu) = cm_submenu {
            let (sub_x, sub_y, sub_w, sub_h) = ctx_submenu_rect(cm_x, cm_y, submenu, sw, taskbar_y);
            let entries = ctx_submenu_entries(submenu);
            if let Some(idx) = ctx_menu_hit_index(entries, sub_x, sub_y, sub_w, mx, my) {
                let entry = entries[idx];
                if entry.enabled {
                    if let ContextEntryKind::Action(cmd) = entry.kind {
                        self.context_menu = None;
                        self.run_context_command(cmd, sw, taskbar_y);
                    }
                }
                return true;
            }
            if mx >= sub_x && mx < sub_x + sub_w && my >= sub_y && my < sub_y + sub_h {
                return true;
            }
        }

        if let Some(idx) = ctx_menu_hit_index(DESKTOP_CONTEXT_MENU, cm_x, cm_y, CTX_W, mx, my) {
            let entry = DESKTOP_CONTEXT_MENU[idx];
            match entry.kind {
                ContextEntryKind::Action(cmd) if entry.enabled => {
                    self.context_menu = None;
                    self.run_context_command(cmd, sw, taskbar_y);
                }
                ContextEntryKind::Submenu(submenu) if entry.enabled => {
                    if let Some(cm_mut) = self.context_menu.as_mut() {
                        cm_mut.submenu = Some(submenu);
                    }
                }
                _ => {}
            }
            return true;
        }

        let main_h = ctx_menu_height(DESKTOP_CONTEXT_MENU);
        if mx >= cm_x && mx < cm_x + CTX_W && my >= cm_y && my < cm_y + main_h {
            return true;
        }

        self.context_menu = None;
        false
    }

    fn run_context_command(&mut self, cmd: DesktopContextCommand, sw: i32, taskbar_y: i32) {
        match cmd {
            DesktopContextCommand::ToggleDesktopIcons => {
                desktop_settings::set_show_icons(!self.desktop_show_icons);
                self.sync_desktop_settings();
            }
            DesktopContextCommand::ToggleCompactSpacing => {
                desktop_settings::set_compact_spacing(!self.desktop_compact_spacing);
                self.sync_desktop_settings();
            }
            DesktopContextCommand::SortByName => {
                desktop_settings::set_sort_mode(DesktopSortMode::Name);
                self.sync_desktop_settings();
            }
            DesktopContextCommand::SortByType => {
                desktop_settings::set_sort_mode(DesktopSortMode::Type);
                self.sync_desktop_settings();
            }
            DesktopContextCommand::Refresh => self.refresh_desktop_state(),
            DesktopContextCommand::CreateFolder => {
                if let Ok(path) = create_root_item("DIR", None, true) {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (10 + off).min(sw - 640);
                    let wy = (10 + off).min(taskbar_y - 120);
                    self.launch_app("File Manager", wx, wy);
                    if let Some(AppWindow::FileManager(fm)) = self.windows.last_mut() {
                        fm.load_dir("/");
                    }
                    if let Some(term) = self.windows.iter_mut().find_map(|w| match w {
                        AppWindow::Terminal(t) => Some(t),
                        _ => None,
                    }) {
                        term.print_str("created ");
                        term.print_str(&path);
                        term.print_char('\n');
                    }
                }
            }
            DesktopContextCommand::CreateTextDocument => {
                if let Ok(path) = create_root_item("FILE", Some("TXT"), false) {
                    let off = self.windows.len() as i32 * 16;
                    let wx = (10 + off).min(sw - 640);
                    let wy = (10 + off).min(taskbar_y - 120);
                    self.launch_app("File Manager", wx, wy);
                    if let Some(AppWindow::FileManager(fm)) = self.windows.last_mut() {
                        fm.load_dir("/");
                    }
                    if let Some(term) = self.windows.iter_mut().find_map(|w| match w {
                        AppWindow::Terminal(t) => Some(t),
                        _ => None,
                    }) {
                        term.print_str("created ");
                        term.print_str(&path);
                        term.print_char('\n');
                    }
                }
            }
            DesktopContextCommand::DisplaySettings => {
                let off = self.windows.len() as i32 * 16;
                let wx = (10 + off).min(sw - crate::apps::displaysettings::DISPLAY_SETTINGS_W);
                let wy =
                    (10 + off).min(taskbar_y - crate::apps::displaysettings::DISPLAY_SETTINGS_H);
                self.launch_app("Display Settings", wx, wy);
            }
            DesktopContextCommand::Personalize => {
                let off = self.windows.len() as i32 * 16;
                let wx = (10 + off).min(sw - crate::apps::personalize::PERSONALIZE_W);
                let wy = (10 + off).min(taskbar_y - crate::apps::personalize::PERSONALIZE_H);
                self.launch_app("Personalize", wx, wy);
            }
        }
        crate::wm::request_repaint();
    }

    fn toggle_show_desktop(&mut self) {
        let any_visible = self
            .windows
            .iter()
            .enumerate()
            .any(|(idx, w)| self.is_window_on_current_workspace(idx) && !w.window().minimized);
        if any_visible {
            let current_workspace = self.current_workspace;
            for (idx, w) in self.windows.iter_mut().enumerate() {
                if self
                    .window_workspaces
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .min(WORKSPACE_COUNT - 1)
                    == current_workspace
                {
                    w.window_mut().minimize();
                }
            }
            self.focused = None;
        } else {
            let current_workspace = self.current_workspace;
            for (idx, w) in self.windows.iter_mut().enumerate() {
                if self
                    .window_workspaces
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .min(WORKSPACE_COUNT - 1)
                    == current_workspace
                {
                    w.window_mut().restore();
                }
            }
            self.focused = self.top_visible_window();
        }
    }

    fn minimize_all_windows(&mut self) {
        let current_workspace = self.current_workspace;
        for (idx, w) in self.windows.iter_mut().enumerate() {
            if self
                .window_workspaces
                .get(idx)
                .copied()
                .unwrap_or(0)
                .min(WORKSPACE_COUNT - 1)
                == current_workspace
            {
                w.window_mut().minimize();
            }
        }
        self.focused = None;
    }

    fn show_task_switcher(&mut self) {
        self.task_switcher_until_tick =
            crate::interrupts::ticks() + crate::interrupts::ticks_for_millis(TASK_SWITCHER_MS);
        self.task_switcher_query.clear();
    }

    fn snap_focused_window(&mut self, target: SnapTarget) -> bool {
        let Some(idx) = self.focused else {
            return false;
        };
        self.snap_window(idx, target)
    }

    fn snap_window(&mut self, win_idx: usize, target: SnapTarget) -> bool {
        if win_idx >= self.windows.len() {
            return false;
        }

        let sw = self.shadow_width as i32;
        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let half_w = (sw / 2).max(160);
        let half_h = (taskbar_y / 2).max(TITLE_H + 80);
        let (x, y, w, h) = match target {
            SnapTarget::Left => (0, 0, half_w, taskbar_y),
            SnapTarget::Right => (sw - half_w, 0, half_w, taskbar_y),
            SnapTarget::Maximize => (0, 0, sw, taskbar_y),
            SnapTarget::Bottom => (0, taskbar_y - half_h, sw, half_h),
            SnapTarget::TopLeft => (0, 0, half_w, half_h),
            SnapTarget::TopRight => (sw - half_w, 0, half_w, half_h),
            SnapTarget::BottomLeft => (0, taskbar_y - half_h, half_w, half_h),
            SnapTarget::BottomRight => (sw - half_w, taskbar_y - half_h, half_w, half_h),
        };

        self.windows[win_idx].window_mut().set_bounds(x, y, w, h);
        if let Some(z_pos) = self.z_order.iter().position(|&i| i == win_idx) {
            self.z_order.remove(z_pos);
            self.z_order.push(win_idx);
        }
        self.focused = Some(win_idx);
        self.context_menu = None;
        self.start_menu_open = false;
        self.start_power_menu_open = false;
        self.notify_session_changed();
        true
    }

    fn snap_dragged_window_on_release(&mut self, win_idx: usize) -> bool {
        if win_idx >= self.windows.len() {
            return false;
        }
        let w = self.windows[win_idx].window();
        let sw = self.shadow_width as i32;
        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        let target = if w.y <= SNAP_EDGE_PX {
            Some(SnapTarget::Maximize)
        } else if w.x <= SNAP_EDGE_PX {
            Some(SnapTarget::Left)
        } else if w.x + w.width >= sw - SNAP_EDGE_PX {
            Some(SnapTarget::Right)
        } else if w.y + w.height >= taskbar_y - SNAP_EDGE_PX {
            Some(SnapTarget::Bottom)
        } else {
            None
        };
        if let Some(target) = target {
            self.snap_window(win_idx, target)
        } else {
            false
        }
    }

    pub fn stop_key_sink(&mut self) {
        if let Some(fd) = self.key_sink_fd.take() {
            crate::vfs::vfs_close(fd);
        }
        self.key_sink_window = None;
    }

    pub fn handle_key(&mut self, c: char) {
        if let (Some(fd), Some(target)) = (self.key_sink_fd, self.key_sink_window) {
            if self.focused != Some(target) {
                if let Some(idx) = self.focused {
                    if idx < self.windows.len() {
                        self.windows[idx].handle_key(c);
                        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
                        self.consume_window_open_request(idx, self.shadow_width, taskbar_y);
                        self.repair_focus_if_hidden();
                        crate::wm::request_repaint();
                    }
                }
                return;
            }

            if c == '~' {
                self.stop_key_sink();
                if target < self.windows.len() {
                    if let AppWindow::Terminal(ref mut t) = self.windows[target] {
                        t.print_str("\n[keydemo closed]\n> ");
                    }
                }
                crate::wm::request_repaint();
                return;
            }

            let packet = key_event_packet(c);
            let n = crate::vfs::vfs_write(fd, &packet);
            if n != EVENT_PACKET_SIZE {
                self.stop_key_sink();
                if target < self.windows.len() {
                    if let AppWindow::Terminal(ref mut t) = self.windows[target] {
                        t.print_str("\n[keydemo pipe error]\n> ");
                    }
                }
                crate::wm::request_repaint();
            }
            return;
        }

        if let Some(idx) = self.focused {
            if idx < self.windows.len() {
                self.windows[idx].handle_key(c);
                let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
                self.consume_window_open_request(idx, self.shadow_width, taskbar_y);
                self.repair_focus_if_hidden();
                crate::wm::request_repaint();
            }
        }
    }

    pub fn handle_key_input(&mut self, input: KeyInput) {
        if self.session_locked {
            self.handle_greeter_input(input);
            crate::wm::request_repaint();
            return;
        }
        if self.task_switcher_until_tick > crate::interrupts::ticks()
            && self.handle_task_switcher_input(input)
        {
            crate::wm::request_repaint();
            return;
        }
        if self.start_menu_open && self.handle_start_menu_input(input) {
            crate::wm::request_repaint();
            return;
        }
        if self.handle_global_shortcut(input) {
            crate::wm::request_repaint();
            return;
        }
        if self.key_sink_fd.is_some() {
            if let Some(c) = input.legacy_char() {
                self.handle_key(c);
            }
            return;
        }
        if let Some(idx) = self.focused {
            if idx < self.windows.len() {
                self.windows[idx].handle_key_input(input);
                let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
                self.consume_window_open_request(idx, self.shadow_width, taskbar_y);
                self.repair_focus_if_hidden();
                crate::wm::request_repaint();
            }
        }
    }

    fn handle_task_switcher_input(&mut self, input: KeyInput) -> bool {
        match input.key {
            Key::Escape => {
                self.task_switcher_until_tick = 0;
                self.task_switcher_query.clear();
                true
            }
            Key::Enter => {
                self.task_switcher_until_tick = 0;
                true
            }
            Key::Backspace if !input.has_alt() && !input.has_ctrl() => {
                self.task_switcher_query.pop();
                self.focus_first_switcher_match();
                true
            }
            Key::Character(c) if !input.has_alt() && !input.has_ctrl() => {
                if c >= ' ' && c != '\u{7f}' {
                    self.task_switcher_query.push(c);
                    self.focus_first_switcher_match();
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn focus_first_switcher_match(&mut self) {
        if self.task_switcher_query.is_empty() {
            return;
        }
        let query = self.task_switcher_query.to_ascii_lowercase();
        if let Some(idx) = self.z_order.iter().rev().copied().find(|&idx| {
            idx < self.windows.len()
                && self.is_window_on_current_workspace(idx)
                && !self.windows[idx].is_minimized()
                && self.windows[idx]
                    .window()
                    .title
                    .to_ascii_lowercase()
                    .contains(&query)
        }) {
            self.focus_window(idx);
        }
    }

    fn handle_global_shortcut(&mut self, input: KeyInput) -> bool {
        if crate::shortcuts::matches(crate::shortcuts::Action::StartSearch, input) {
            self.open_start_menu_search();
            return true;
        }
        if crate::shortcuts::matches(crate::shortcuts::Action::Notifications, input) {
            self.toggle_notification_center();
            return true;
        }
        match input.key {
            Key::Tab if input.has_alt() => {
                self.focus_previous_window();
                self.show_task_switcher();
                true
            }
            Key::PageUp if input.has_ctrl() && input.has_alt() => {
                let next = if self.current_workspace == 0 {
                    WORKSPACE_COUNT - 1
                } else {
                    self.current_workspace - 1
                };
                self.switch_workspace(next);
                true
            }
            Key::PageDown if input.has_ctrl() && input.has_alt() => {
                self.switch_workspace((self.current_workspace + 1) % WORKSPACE_COUNT);
                true
            }
            Key::ArrowLeft if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::Left)
            }
            Key::ArrowRight if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::Right)
            }
            Key::ArrowUp if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::Maximize)
            }
            Key::ArrowDown if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::Bottom)
            }
            Key::Character('1') if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::TopLeft)
            }
            Key::Character('2') if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::TopRight)
            }
            Key::Character('3') if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::BottomLeft)
            }
            Key::Character('4') if input.has_ctrl() && input.has_alt() => {
                self.snap_focused_window(SnapTarget::BottomRight)
            }
            Key::F4 if input.has_alt() => {
                if let Some(idx) = self.focused {
                    self.request_or_close_window(idx);
                    true
                } else {
                    false
                }
            }
            Key::F5 => {
                self.refresh_desktop_state();
                true
            }
            Key::F2
                if !input.has_ctrl()
                    && !input.has_alt()
                    && self.focused.is_none()
                    && self.icon_selected.is_some() =>
            {
                self.show_error_dialog(
                    "Rename shortcut",
                    "Desktop shortcut names are loaded from /APPS package metadata.",
                );
                true
            }
            Key::Escape if input.has_ctrl() => {
                self.toggle_start_menu();
                true
            }
            Key::Character('w') | Key::Character('W') if input.has_ctrl() => {
                if let Some(idx) = self.focused {
                    self.request_or_close_window(idx);
                    true
                } else {
                    false
                }
            }
            Key::Character('r') | Key::Character('R') if input.has_ctrl() => {
                self.refresh_desktop_state();
                true
            }
            Key::Character('f') | Key::Character('F') if input.has_ctrl() => {
                let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
                let off = self.windows.len() as i32 * 16;
                let wx = (10 + off).min(self.shadow_width as i32 - 220);
                let wy = (10 + off).min(taskbar_y - 120);
                self.launch_file_manager_at("/", wx, wy);
                self.start_menu_open = false;
                self.start_power_menu_open = false;
                true
            }
            Key::Character('n') | Key::Character('N') if input.has_ctrl() => {
                let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
                let off = self.windows.len() as i32 * 16;
                let wx = (10 + off).min(self.shadow_width as i32 - 220);
                let wy = (10 + off).min(taskbar_y - 120);
                self.launch_app("Terminal", wx, wy);
                self.start_menu_open = false;
                self.start_power_menu_open = false;
                true
            }
            Key::Character(c) if input.has_ctrl() && !input.has_alt() => {
                if let Some(slot) = ctrl_number_slot(c) {
                    self.quick_launch_pinned(slot)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn focus_previous_window(&mut self) {
        if self.z_order.is_empty() {
            self.focused = None;
            return;
        }
        let start = self
            .focused
            .and_then(|focused| self.z_order.iter().position(|&idx| idx == focused))
            .unwrap_or_else(|| self.z_order.len().saturating_sub(1));

        let mut pos = start;
        for _ in 0..self.z_order.len() {
            pos = if pos == 0 {
                self.z_order.len() - 1
            } else {
                pos - 1
            };
            let candidate = self.z_order[pos];
            if candidate < self.windows.len()
                && self.is_window_on_current_workspace(candidate)
                && !self.windows[candidate].window().minimized
            {
                self.z_order.remove(pos);
                self.z_order.push(candidate);
                self.focused = Some(candidate);
                self.context_menu = None;
                self.start_menu_open = false;
                self.start_power_menu_open = false;
                return;
            }
        }
        self.focused = None;
    }

    fn repair_focus_if_hidden(&mut self) {
        let needs_repair = match self.focused {
            Some(idx) => {
                idx >= self.windows.len()
                    || !self.is_window_on_current_workspace(idx)
                    || self.windows[idx].is_minimized()
            }
            None => false,
        };
        if needs_repair {
            self.focused = self.top_visible_window();
        }
    }

    fn close_window(&mut self, win_idx: usize) {
        if win_idx >= self.windows.len() {
            return;
        }
        if self.key_sink_window == Some(win_idx) {
            self.stop_key_sink();
        } else if let Some(target) = self.key_sink_window {
            if target > win_idx {
                self.key_sink_window = Some(target - 1);
            }
        }
        self.windows.remove(win_idx);
        if win_idx < self.window_workspaces.len() {
            self.window_workspaces.remove(win_idx);
        }
        self.z_order.retain(|&i| i != win_idx);
        for z in self.z_order.iter_mut() {
            if *z > win_idx {
                *z -= 1;
            }
        }
        self.focused = self.top_visible_window();
        self.drag = None;
        self.resize = None;
        self.scroll_drag = None;
        self.file_drag = None;
        self.context_menu = None;
        self.taskbar_menu = None;
        self.notify_session_changed();
    }

    fn reap_unresponsive_user_gui_closes(&mut self, now: u64) {
        let timeout_ticks = crate::interrupts::ticks_for_millis(USER_GUI_CLOSE_TIMEOUT_MS);
        let owners: Vec<usize> = self
            .windows
            .iter()
            .filter_map(|window| window.user_gui_close_timeout_owner(now, timeout_ticks))
            .collect();
        for owner in owners {
            if crate::scheduler::kill_task(owner, 143).is_ok() {
                crate::notifications::push_transient(
                    "App close timeout",
                    &format!("pid {} did not acknowledge close", owner),
                );
            }
        }
    }

    fn request_or_close_window(&mut self, win_idx: usize) {
        if win_idx >= self.windows.len() {
            return;
        }
        if self.windows[win_idx].request_close() {
            self.close_window(win_idx);
        } else {
            crate::wm::request_repaint();
        }
    }

    /// Repaint only the hardware cursor overlay when the base scene is unchanged.
    pub fn compose_cursor_only(&mut self) -> bool {
        let frame_start_tick = crate::interrupts::ticks();
        let (mx, my) = crate::mouse::pos();
        let (left, right) = crate::mouse::buttons();
        let mx_i = mx as i32;
        let my_i = my as i32;
        if !self.cursor_fast_path_allowed(mx_i, my_i, left, right) {
            return false;
        }

        let resize = self.cursor_resize_hover_at(mx_i, my_i);
        let pixels = self.restore_cursor_backing() + self.draw_cursor_overlay(mx_i, my_i, resize);
        COMPOSITOR_CURSOR_FAST_FRAMES.fetch_add(1, Ordering::Relaxed);
        COMPOSITOR_CURSOR_PIXELS_LAST.store(pixels as u64, Ordering::Relaxed);
        COMPOSITOR_FRAME_TICKS_LAST.store(
            crate::interrupts::ticks().wrapping_sub(frame_start_tick),
            Ordering::Relaxed,
        );
        true
    }

    fn cursor_fast_path_allowed(&self, mx_i: i32, my_i: i32, left: bool, right: bool) -> bool {
        if !self.cursor_drawn
            || self.full_damage_next
            || left
            || right
            || self.session_locked
            || self.start_menu_open
            || self.start_power_menu_open
            || self.context_menu.is_some()
            || self.taskbar_menu.is_some()
            || self.notification_center_open
            || self.dialog.is_some()
            || self.drag.is_some()
            || self.resize.is_some()
            || self.scroll_drag.is_some()
            || self.file_drag.is_some()
            || self.desktop_select_drag.is_some()
            || self.desktop_icon_drag.is_some()
            || self.task_switcher_until_tick > crate::interrupts::ticks()
        {
            return false;
        }

        let taskbar_y = self.shadow_height as i32 - TASKBAR_H;
        if my_i >= taskbar_y {
            return false;
        }

        if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
            let wi = self.z_order[z_pos];
            if wi >= self.windows.len() {
                return false;
            }
            let w = self.windows[wi].window();
            if w.hit_title(mx_i, my_i)
                || w.hit_close(mx_i, my_i)
                || w.hit_minimize(mx_i, my_i)
                || w.hit_maximize(mx_i, my_i)
                || w.hit_scrollbar(mx_i, my_i)
            {
                return false;
            }
        }

        true
    }

    fn cursor_resize_hover_at(&self, mx_i: i32, my_i: i32) -> bool {
        !self.session_locked
            && (self.resize.is_some()
                || self
                    .front_to_back_hit(mx_i, my_i)
                    .map(|z_pos| {
                        let wi = self.z_order[z_pos];
                        wi < self.windows.len() && self.windows[wi].window().hit_resize(mx_i, my_i)
                    })
                    .unwrap_or(false))
    }

    /// Full composite frame into shadow, then blit to hardware framebuffer.
    pub fn compose(&mut self) {
        let frame_start_tick = crate::interrupts::ticks();
        crate::wm::drain_user_gui_owner_cleanup(self);
        if crate::wm::take_session_lock_request() {
            self.lock_session();
        }
        let mut startup_drained = 0usize;
        while let Some(command) = crate::wm::take_startup_command() {
            if self.session_locked {
                if !self.run_locked_startup_command(&command) {
                    crate::wm::queue_startup_command(&command);
                    break;
                }
            } else if !self.run_startup_exec_command(&command) {
                self.run_terminal_command(&command);
            }
            startup_drained += 1;
            if startup_drained >= 16 {
                break;
            }
        }
        if crate::wm::take_session_lock_request() {
            self.lock_session();
        }

        // Drain buffered keystrokes.
        while let Some(input) = crate::keyboard::pop_input() {
            self.handle_key_input(input);
        }

        // Drain syscall write() output into the first terminal window.
        while let Some(b) = crate::syscall::pop_output_byte() {
            for w in self.windows.iter_mut() {
                if let AppWindow::Terminal(ref mut t) = w {
                    t.print_char(b as char);
                    break;
                }
            }
        }

        let mut browser_requests: Vec<String> = Vec::new();
        for w in self.windows.iter_mut() {
            if let Some(url) = w.take_browser_request() {
                browser_requests.push(url);
            }
        }
        for url in browser_requests {
            self.launch_browser_url(&url);
        }

        // Consume deferred terminal requests to install a compositor-owned key sink.
        for (idx, w) in self.windows.iter_mut().enumerate() {
            if let AppWindow::Terminal(t) = w {
                if let Some(fd) = t.take_pending_key_sink() {
                    if self.key_sink_fd.is_none() {
                        self.key_sink_fd = Some(fd);
                        self.key_sink_window = Some(idx);
                    } else {
                        crate::vfs::vfs_close(fd);
                        t.print_str("keydemo unavailable: input sink busy\n> ");
                    }
                }
            }
        }

        if let Some(path) = crate::wm::take_screenshot_request() {
            match self.save_focused_screenshot(&path) {
                Ok(()) => crate::notifications::push("Screenshot saved", &path),
                Err(err) => crate::notifications::push("Screenshot failed", err),
            }
        }

        let sw = self.shadow_width;
        let sh = self.shadow_height;
        let taskbar_y = sh as i32 - TASKBAR_H;

        // Snapshot the real boot tick count for time-based UI.
        let uptime_ticks = crate::interrupts::ticks();

        let (mx, my) = crate::mouse::pos();
        let (left, right) = crate::mouse::buttons();
        let mx_i = mx as i32;
        let my_i = my as i32;

        let raw_left_pressed = left && !self.prev_left;
        let raw_right_pressed = right && !self.prev_right;
        if self.session_locked {
            if raw_left_pressed {
                self.handle_greeter_click(mx_i, my_i, sw as i32, taskbar_y);
            }
            if raw_left_pressed || raw_right_pressed {
                crate::wm::request_repaint();
            }
        }
        let left_pressed = raw_left_pressed && !self.session_locked;
        let left_released = !left && self.prev_left && !self.session_locked;
        let right_pressed = raw_right_pressed && !self.session_locked;
        let mut left_press_consumed = false;

        // ── Input ─────────────────────────────────────────────────────────────

        if left_pressed && self.dialog.is_some() {
            self.handle_dialog_click(mx_i, my_i, sw as i32, taskbar_y);
            left_press_consumed = true;
            crate::wm::request_repaint();
        }

        // Start button click — flush left, full height
        let taskbar_click = left_pressed && my_i >= taskbar_y && mx_i < START_BTN_W;
        if taskbar_click {
            self.toggle_start_menu();
            left_press_consumed = true;
            crate::wm::request_repaint();
        }

        if left_pressed {
            let had_taskbar_menu = self.taskbar_menu.is_some();
            if self.handle_taskbar_menu_click(mx_i, my_i) || had_taskbar_menu {
                left_press_consumed = true;
                crate::wm::request_repaint();
            }
        }

        if left_pressed
            && !left_press_consumed
            && self.handle_notification_center_click(mx_i, my_i, sw as i32, taskbar_y)
        {
            left_press_consumed = true;
            crate::wm::request_repaint();
        }

        if right_pressed && my_i >= taskbar_y {
            if let Some(btn_idx) = self.taskbar_button_hit(mx_i, my_i, sw as i32, taskbar_y) {
                self.open_taskbar_menu(btn_idx, mx_i, taskbar_y, sw as i32);
                crate::wm::request_repaint();
            }
        } else if right_pressed && my_i < taskbar_y {
            if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
                let win_idx = self.z_order[z_pos];
                self.z_order.remove(z_pos);
                self.z_order.push(win_idx);
                self.focused = Some(win_idx);
                self.context_menu = None;
                let lx = mx_i - self.windows[win_idx].window().x;
                let ly = my_i - (self.windows[win_idx].window().y + TITLE_H);
                self.windows[win_idx].handle_secondary_click(lx, ly);
                self.consume_window_open_request(win_idx, sw, taskbar_y);
                crate::wm::request_repaint();
            } else {
                self.open_context_menu(mx_i, my_i, sw as i32, taskbar_y);
                crate::wm::request_repaint();
            }
        }

        if self.context_menu.is_some() {
            self.update_context_menu_hover(mx_i, my_i, sw as i32, taskbar_y);
        }

        if left_pressed {
            if left_press_consumed {
                // Already handled by shell chrome such as Start or a taskbar jump list.
            } else if self.context_menu.is_some() {
                left_press_consumed = true;
                let _ = self.handle_context_menu_click(mx_i, my_i, sw as i32, taskbar_y);
            } else {
                if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
                    left_press_consumed = true;
                    let win_idx = self.z_order[z_pos];
                    self.z_order.remove(z_pos);
                    self.z_order.push(win_idx);
                    self.focused = Some(win_idx);

                    let hit_close = self.windows[win_idx].window().hit_close(mx_i, my_i);
                    let hit_minimize = self.windows[win_idx].window().hit_minimize(mx_i, my_i);
                    let hit_maximize = self.windows[win_idx].window().hit_maximize(mx_i, my_i);
                    let hit_title = self.windows[win_idx].window().hit_title(mx_i, my_i);
                    let hit_resize = self.windows[win_idx].window().hit_resize(mx_i, my_i);
                    let hit_scrollbar = self.windows[win_idx].window().hit_scrollbar(mx_i, my_i);

                    if hit_close {
                        self.request_or_close_window(win_idx);
                        crate::wm::request_repaint();
                    } else if hit_minimize {
                        self.windows[win_idx].window_mut().minimize();
                        crate::wm::request_repaint();
                    } else if hit_maximize {
                        let sw = self.shadow_width as i32;
                        let sh = self.shadow_height as i32;
                        self.windows[win_idx].window_mut().maximize(sw, sh);
                        self.notify_session_changed();
                        crate::wm::request_repaint();
                    } else if hit_title {
                        self.drag = Some(DragState {
                            window: win_idx,
                            off_x: mx_i - self.windows[win_idx].window().x,
                            off_y: my_i - self.windows[win_idx].window().y,
                        });
                    } else if hit_resize {
                        let w = self.windows[win_idx].window();
                        self.resize = Some(ResizeState {
                            window: win_idx,
                            start_w: w.width,
                            start_h: w.height,
                            start_mx: mx_i,
                            start_my: my_i,
                        });
                    } else if hit_scrollbar {
                        let w = self.windows[win_idx].window();
                        let view_h = (w.height - TITLE_H).max(0);
                        self.scroll_drag = Some(ScrollDragState {
                            window: win_idx,
                            start_offset: w.scroll.offset,
                            start_my: my_i,
                            content_h: w.scroll.content_h,
                            view_h,
                            track_h: view_h,
                        });
                    } else {
                        let lx = mx_i - self.windows[win_idx].window().x;
                        let ly = my_i - (self.windows[win_idx].window().y + TITLE_H);
                        if self.key_sink_fd.is_some() && self.key_sink_window == Some(win_idx) {
                            let fd = self.key_sink_fd.unwrap();
                            let packet = mouse_event_packet(1, lx, ly);
                            if crate::vfs::vfs_write(fd, &packet) != EVENT_PACKET_SIZE {
                                self.stop_key_sink();
                                if win_idx < self.windows.len() {
                                    if let AppWindow::Terminal(ref mut t) = self.windows[win_idx] {
                                        t.print_str("\n[keydemo pipe error]\n> ");
                                    }
                                }
                            }
                        }
                        let file_drag_paths = self.windows[win_idx].begin_file_drag(lx, ly);
                        self.windows[win_idx].handle_click(lx, ly);
                        if let Some(paths) = file_drag_paths {
                            self.file_drag = Some(FileDragState {
                                source_window: win_idx,
                                paths,
                                cut: false,
                            });
                        }
                        self.consume_window_open_request(win_idx, sw, taskbar_y);
                        self.repair_focus_if_hidden();
                        let is_double_click = self.last_click_window == Some(win_idx)
                            && uptime_ticks.wrapping_sub(self.last_click_tick)
                                <= crate::interrupts::ticks_for_millis(500)
                            && (self.last_click_x - lx).abs() <= 6
                            && (self.last_click_y - ly).abs() <= 6;
                        if is_double_click {
                            self.windows[win_idx].handle_dbl_click(lx, ly);
                            self.consume_window_open_request(win_idx, sw, taskbar_y);
                        }
                        self.last_click_tick = uptime_ticks;
                        self.last_click_window = Some(win_idx);
                        self.last_click_x = lx;
                        self.last_click_y = ly;
                    }
                }

                if my_i >= taskbar_y {
                    left_press_consumed = true;
                    let tray = taskbar_tray_layout(sw as i32, taskbar_y);
                    if tray.clock_contains(mx_i, my_i) {
                        self.toggle_notification_center();
                        crate::wm::request_repaint();
                    } else if tray.icons_contains(mx_i, my_i) {
                        let off = self.windows.len() as i32 * 16;
                        let wx = (10 + off).min(sw as i32 - 540);
                        let wy = (10 + off).min(taskbar_y - 310);
                        self.launch_app("System Monitor", wx, wy);
                        crate::wm::request_repaint();
                    }
                    let show_desktop_x = sw as i32 - TASKBAR_CLOCK_W - SHOW_DESKTOP_W - 8;
                    if mx_i >= show_desktop_x && mx_i < show_desktop_x + SHOW_DESKTOP_W {
                        self.toggle_show_desktop();
                        crate::wm::request_repaint();
                    } else {
                        if let Some(btn_idx) =
                            self.taskbar_button_hit(mx_i, my_i, sw as i32, taskbar_y)
                        {
                            self.focus_window(btn_idx);
                            crate::wm::request_repaint();
                        }
                    }
                }
            }
        }

        if left_released {
            let session_changed = self.drag.is_some() || self.resize.is_some();
            let drag_window = self.drag.as_ref().map(|d| d.window);
            if let Some(win_idx) = drag_window {
                if self.snap_dragged_window_on_release(win_idx) {
                    crate::wm::request_repaint();
                }
            }
            self.drag = None;
            self.resize = None;
            self.scroll_drag = None;
            if let Some(file_drag) = self.file_drag.take() {
                if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
                    let target = self.z_order[z_pos];
                    if target != file_drag.source_window && target < self.windows.len() {
                        let count = file_drag.paths.len();
                        let cut = file_drag.cut;
                        let paths = file_drag.paths;
                        if self.windows[target].drop_file_paths(paths, cut) {
                            crate::notifications::push(
                                "File drop",
                                if count == 1 {
                                    "copied 1 item"
                                } else {
                                    "copied selected items"
                                },
                            );
                            crate::wm::request_repaint();
                        }
                    }
                }
            }
            if session_changed {
                self.notify_session_changed();
            }
        }

        if left {
            if let Some(ref d) = self.drag {
                let wi = d.window;
                if wi < self.windows.len() {
                    let w = self.windows[wi].window_mut();
                    w.x = mx_i - d.off_x;
                    w.y = my_i - d.off_y;
                }
            }
            if let Some(ref rs) = self.resize {
                let wi = rs.window;
                if wi < self.windows.len() {
                    let new_w = rs.start_w + mx_i - rs.start_mx;
                    let new_h = rs.start_h + my_i - rs.start_my;
                    self.windows[wi].window_mut().resize_to(new_w, new_h);
                    crate::wm::request_repaint();
                }
            }
            if let Some(ref sd) = self.scroll_drag {
                let wi = sd.window;
                if wi < self.windows.len() {
                    let delta = my_i - sd.start_my;
                    let max_off = (sd.content_h - sd.view_h).max(1);
                    let track_h = sd.track_h.max(1);
                    let new_off = sd.start_offset + delta * max_off / track_h;
                    self.windows[wi].window_mut().scroll.offset = new_off.clamp(0, max_off);
                    crate::wm::request_repaint();
                }
            }
        }

        self.prev_left = left;
        self.prev_right = right;

        // Start menu item click.
        if left_pressed && self.start_menu_open {
            let layout = win7_start_menu_layout(sw as i32, taskbar_y);
            if layout.contains(mx_i, my_i)
                || (self.start_power_menu_open && {
                    let (power_x, power_y, power_w, power_h) = start_power_menu_rect(
                        layout.shutdown_x,
                        layout.shutdown_y,
                        layout.shutdown_w,
                        layout.menu_x,
                        layout.menu_y,
                        layout.menu_w,
                    );
                    rect_contains(power_x, power_y, power_w, power_h, mx_i, my_i)
                })
            {
                left_press_consumed = true;
                let off = self.windows.len() as i32 * 16;
                let wx = (10 + off).min(sw as i32 - 220);
                let wy = (10 + off).min(taskbar_y - 120);
                let (power_x, power_y, power_w, power_h) = start_power_menu_rect(
                    layout.shutdown_x,
                    layout.shutdown_y,
                    layout.shutdown_w,
                    layout.menu_x,
                    layout.menu_y,
                    layout.menu_w,
                );
                if self.start_power_menu_open
                    && rect_contains(power_x, power_y, power_w, power_h, mx_i, my_i)
                {
                    if let Some(action) =
                        start_power_action_at(mx_i, my_i, power_x, power_y, power_w)
                    {
                        self.run_start_inline_action(action, wx, wy);
                        self.close_start_menu();
                    }
                    crate::wm::request_repaint();
                } else if let Some(result) =
                    start_menu_result_at(layout, &self.start_search, mx_i, my_i)
                {
                    let query = self.start_search.query.clone();
                    self.close_start_menu();
                    if !query.trim().is_empty() {
                        crate::app_lifecycle::record_search(&query);
                    }
                    self.activate_start_search_kind(&result.kind, wx, wy);
                    crate::wm::request_repaint();
                } else if rect_contains(
                    layout.all_x,
                    layout.all_y,
                    layout.all_w,
                    layout.all_h,
                    mx_i,
                    my_i,
                ) {
                    self.start_search.query.clear();
                    self.start_search.selected = 0;
                    self.start_search.focused = false;
                    self.start_search.show_all = true;
                    self.start_power_menu_open = false;
                    crate::wm::request_repaint();
                } else if rect_contains(
                    layout.search_x,
                    layout.search_y,
                    layout.search_w,
                    layout.search_h,
                    mx_i,
                    my_i,
                ) {
                    self.start_search.focused = true;
                    self.start_search.selected = 0;
                    self.start_power_menu_open = false;
                    crate::wm::request_repaint();
                } else if rect_contains(
                    layout.shutdown_arrow_x,
                    layout.shutdown_y,
                    layout.shutdown_arrow_w,
                    layout.shutdown_h,
                    mx_i,
                    my_i,
                ) {
                    self.start_power_menu_open = !self.start_power_menu_open;
                    crate::wm::request_repaint();
                } else if rect_contains(
                    layout.shutdown_x,
                    layout.shutdown_y,
                    layout.shutdown_w,
                    layout.shutdown_h,
                    mx_i,
                    my_i,
                ) {
                    self.run_start_inline_action("shutdown", wx, wy);
                    self.close_start_menu();
                    crate::wm::request_repaint();
                } else if let Some(action) = win7_start_right_action_at(layout, mx_i, my_i) {
                    self.activate_win7_start_action(action, wx, wy);
                    self.close_start_menu();
                    crate::wm::request_repaint();
                } else {
                    self.start_power_menu_open = false;
                    crate::wm::request_repaint();
                }
            }
        }

        // Desktop icon click.
        if left_pressed {
            let icon_hit = self.desktop_icon_hit(mx_i, my_i);
            let desktop_hit = !left_press_consumed
                && my_i < taskbar_y
                && self.context_menu.is_none()
                && !self.start_menu_open;
            self.pressed_icon = if desktop_hit { icon_hit } else { None };
            if let Some(i) = self.pressed_icon {
                self.focused = None;
                let icons = self.desktop_icons();
                if let Some(icon) = icons.get(i) {
                    self.desktop_icon_drag = Some(DesktopIconDragState {
                        icon: i,
                        start_mx: mx_i,
                        start_my: my_i,
                        start_x: icon.x,
                        start_y: icon.y,
                        cur_x: icon.x,
                        cur_y: icon.y,
                        moved: false,
                    });
                }
                if crate::keyboard::current_modifiers() & crate::keyboard::MOD_CTRL != 0 {
                    if let Some(pos) = self.desktop_multi_selected.iter().position(|&idx| idx == i)
                    {
                        self.desktop_multi_selected.remove(pos);
                    } else {
                        self.desktop_multi_selected.push(i);
                    }
                    self.icon_selected = Some(i);
                } else {
                    self.desktop_multi_selected.clear();
                    self.desktop_multi_selected.push(i);
                    self.icon_selected = Some(i);
                }
                self.context_menu = None;
                crate::wm::request_repaint();
            } else if desktop_hit {
                self.focused = None;
                self.desktop_select_drag = Some((mx_i, my_i));
                self.icon_selected = None;
                self.desktop_multi_selected.clear();
                self.context_menu = None;
                crate::wm::request_repaint();
            }
        }

        if left && self.desktop_select_drag.is_some() {
            crate::wm::request_repaint();
        }

        if left {
            if let Some(drag) = self.desktop_icon_drag.as_mut() {
                let dx = mx_i - drag.start_mx;
                let dy = my_i - drag.start_my;
                if dx.abs() > 4 || dy.abs() > 4 {
                    drag.moved = true;
                }
                if drag.moved {
                    drag.cur_x = (drag.start_x + dx).clamp(4, sw as i32 - ICON_SIZE - 4);
                    drag.cur_y =
                        (drag.start_y + dy).clamp(4, taskbar_y - ICON_SIZE - ICON_LABEL_H - 4);
                    crate::wm::request_repaint();
                }
            }
        }

        if left_released {
            let dragged_icon = self.desktop_icon_drag.take();
            if let Some(drag) = dragged_icon.as_ref() {
                if drag.moved {
                    if let Some(icon) = self.desktop_icons().get(drag.icon) {
                        let _ =
                            desktop_settings::set_icon_position(icon.label, drag.cur_x, drag.cur_y);
                    }
                    crate::wm::request_repaint();
                }
            }
            if let Some((sx, sy)) = self.desktop_select_drag.take() {
                let dx = (mx_i - sx).abs();
                let dy = (my_i - sy).abs();
                self.desktop_multi_selected.clear();
                if dx > 4 || dy > 4 {
                    let x0 = sx.min(mx_i);
                    let x1 = sx.max(mx_i);
                    let y0 = sy.min(my_i);
                    let y1 = sy.max(my_i);
                    for (idx, icon) in self.desktop_icons().iter().enumerate() {
                        let cx = icon.x + ICON_SIZE / 2;
                        let cy = icon.y + ICON_SIZE / 2;
                        if cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1 {
                            self.desktop_multi_selected.push(idx);
                        }
                    }
                    self.icon_selected = self.desktop_multi_selected.last().copied();
                } else {
                    self.icon_selected = None;
                }
                crate::wm::request_repaint();
            }
            if let Some(icon_idx) = self.pressed_icon.take() {
                let icon_was_dragged = dragged_icon
                    .as_ref()
                    .map(|drag| drag.moved && drag.icon == icon_idx)
                    .unwrap_or(false);
                if self.desktop_icon_hit(mx_i, my_i) == Some(icon_idx)
                    && my_i < taskbar_y
                    && self.front_to_back_hit(mx_i, my_i).is_none()
                    && self.context_menu.is_none()
                    && !self.start_menu_open
                    && self.desktop_multi_selected.len() <= 1
                    && !icon_was_dragged
                {
                    let icons = self.desktop_icons();
                    let icon = &icons[icon_idx];
                    let off = self.windows.len() as i32 * 16;
                    let wx = (10 + off).min(sw as i32 - 200);
                    let wy = (10 + off).min(taskbar_y - 80);
                    self.launch_app(icon.app, wx, wy);
                    crate::wm::request_repaint();
                }
            }
        }

        // ── Scroll wheel ─────────────────────────────────────────────────────
        let wheel_delta = crate::mouse::scroll_delta();
        if wheel_delta != 0 && !self.session_locked {
            if let Some(z_pos) = self.front_to_back_hit(mx_i, my_i) {
                let win_idx = self.z_order[z_pos];
                self.windows[win_idx].handle_scroll(wheel_delta);
                crate::wm::request_repaint();
            }
        }

        self.sync_desktop_settings();
        self.maybe_save_session(uptime_ticks);

        // ── Render ────────────────────────────────────────────────────────────
        for w in self.windows.iter_mut() {
            w.update();
        }
        self.reap_unresponsive_user_gui_closes(uptime_ticks);
        self.collect_app_dirty_spans(sw, sh);

        // Blit wallpaper before taking the exclusive &mut shadow borrow,
        // so the compiler sees two separate borrows of self.shadow / self.wallpaper.
        {
            let desk_pixels = taskbar_y as usize * sw;
            self.shadow[..desk_pixels].copy_from_slice(&self.wallpaper[..desk_pixels]);
        }
        let resize_hover = !self.session_locked
            && (self.resize.is_some()
                || self
                    .front_to_back_hit(mx_i, my_i)
                    .map(|z_pos| {
                        let wi = self.z_order[z_pos];
                        wi < self.windows.len() && self.windows[wi].window().hit_resize(mx_i, my_i)
                    })
                    .unwrap_or(false));
        let desktop_icons = self.desktop_icons();
        let hovered_taskbar_window = if self.session_locked {
            None
        } else {
            self.taskbar_button_hit(mx_i, my_i, sw as i32, taskbar_y)
        };
        let current_workspace = self.current_workspace;
        let first_boot_active = self.session_locked && self.first_boot_required();
        let first_boot_snapshot = if first_boot_active {
            Some((
                self.first_boot_owner.clone(),
                self.first_boot_password.chars().count(),
                self.first_boot_confirm.chars().count(),
                self.first_boot_device.clone(),
                self.first_boot_focus,
                self.first_boot_message.clone(),
                self.first_boot_error,
            ))
        } else {
            None
        };
        let greeter_snapshot = if self.session_locked && !first_boot_active {
            Some((
                self.greeter_user.clone(),
                self.greeter_password.chars().count(),
                self.greeter_focus,
                self.greeter_message.clone(),
                self.greeter_error,
                self.greeter_attempts,
            ))
        } else {
            None
        };
        {
            let s: &mut [u32] = self.shadow.as_mut_slice();

            // ── Desktop icons — drawn BEFORE windows so windows can cover them ────
            for (i, icon) in desktop_icons.iter().enumerate() {
                let selected =
                    self.icon_selected == Some(i) || self.desktop_multi_selected.contains(&i);
                let hot = mx_i >= icon.x
                    && mx_i < icon.x + ICON_SIZE
                    && my_i >= icon.y
                    && my_i < icon.y + ICON_SIZE;
                let app_open = self
                    .windows
                    .iter()
                    .any(|w| w.window().title == canonical_app_title(icon.app));

                let app_title = canonical_app_title(icon.app);
                let icon_kind = desktop_icon_kind(app_title);
                let icon_acc = desktop_icon_accent(icon_kind);

                if selected || hot {
                    draw_desktop_icon_plate(s, sw, icon.x, icon.y, selected, icon_acc);
                }
                draw_desktop_app_icon(s, sw, icon.x + 6, icon.y + 5, icon_kind);

                if app_open {
                    s_fill(
                        s,
                        sw,
                        icon.x + 18,
                        icon.y + ICON_SIZE - 3,
                        ICON_SIZE - 36,
                        2,
                        blend_color(icon_acc, 0x00_EA_FC_FF, 84),
                    );
                    s_fill(s, sw, icon.x + 24, icon.y + ICON_SIZE, 4, 1, icon_acc);
                }

                // Label below icon — plain centred text, no box
                let label_y = icon.y + ICON_SIZE + 8;
                let label_w = icon.label.len() as i32 * 8;
                let label_x = (icon.x + (ICON_SIZE - label_w) / 2).max(1);
                let label_fg = if selected {
                    0x00_E7_EF_F6
                } else if app_open {
                    blend_color(icon_acc, 0x00_DD_FF_FF, 118)
                } else if hot {
                    0x00_D4_E2_EB
                } else {
                    0x00_A4_B4_C2
                };
                draw_desktop_label(
                    s,
                    sw,
                    label_x,
                    label_y,
                    icon.label,
                    label_fg,
                    label_x + label_w + 4,
                );
            }

            if let Some((sx, sy)) = self.desktop_select_drag {
                let x0 = sx.min(mx_i);
                let x1 = sx.max(mx_i);
                let y0 = sy.min(my_i);
                let y1 = sy.max(my_i);
                if x1 - x0 > 3 && y1 - y0 > 3 {
                    s_fill_alpha(s, sw, x0, y0, x1 - x0, y1 - y0, 0x28_00_BB_FF);
                    draw_rect_border(s, sw, x0, y0, x1 - x0, y1 - y0, ACCENT);
                }
            }

            // ── Windows — drawn AFTER icons so they appear in front ───────────────
            let z: Vec<usize> = self.z_order.clone();
            for &wi in &z {
                if wi < self.windows.len()
                    && self
                        .window_workspaces
                        .get(wi)
                        .copied()
                        .unwrap_or(0)
                        .min(WORKSPACE_COUNT - 1)
                        == current_workspace
                {
                    let win = self.windows[wi].window();
                    if !win.minimized {
                        let focused = self.focused == Some(wi);
                        Self::draw_window(s, sw, win, focused, mx_i, my_i);
                    }
                }
            }

            // ── Taskbar — glass panel ────────────────────────────────────────────
            {
                let t0 = taskbar_y as usize;
                let t1 = (t0 + TASKBAR_H as usize).min(s.len() / sw);
                for row in t0..t1 {
                    for col in 0..sw {
                        let p = s[row * sw + col];
                        let r = (((p >> 16) & 0xFF) * 18 / 100).saturating_add(15).min(255);
                        let g = (((p >> 8) & 0xFF) * 18 / 100).saturating_add(19).min(255);
                        let b = ((p & 0xFF) * 20 / 100).saturating_add(25).min(255);
                        s[row * sw + col] = (r << 16) | (g << 8) | b;
                    }
                }
            }
            s_fill(s, sw, 0, taskbar_y, sw as i32, 1, ACCENT);
            s_fill(s, sw, 0, taskbar_y + 1, sw as i32, 1, 0x00_2C_3F_50);

            // ── Start button — minimal Start control ────────────────────────────
            let start_hot = mx_i >= 0
                && mx_i < START_BTN_W
                && my_i >= taskbar_y
                && my_i < taskbar_y + TASKBAR_H;
            let start_pressed = left && start_hot;

            let start_cell_x = 0i32;
            let start_cell_w = START_BTN_W;
            let underline_y = taskbar_y + TASKBAR_H - 3;
            let active_underline_x = start_cell_x + 5;
            let active_underline_w = start_cell_w - 10;
            if self.start_menu_open {
                s_fill(
                    s,
                    sw,
                    active_underline_x,
                    underline_y,
                    active_underline_w,
                    2,
                    ACCENT_HOV,
                );
            } else if start_pressed {
                s_fill(
                    s,
                    sw,
                    start_cell_x + 13,
                    underline_y + 1,
                    start_cell_w - 26,
                    1,
                    blend_color(ACCENT, BLACK, 96),
                );
            } else if start_hot {
                s_fill(
                    s,
                    sw,
                    active_underline_x,
                    underline_y,
                    active_underline_w,
                    2,
                    ACCENT_HOV,
                );
            }

            let tile_x = start_cell_x + (start_cell_w - 18) / 2;
            let tile_y = taskbar_y + (TASKBAR_H - 18) / 2;
            let icon_primary = if self.start_menu_open {
                WHITE
            } else if start_pressed {
                blend_color(ACCENT_HOV, BLACK, 104)
            } else if start_hot {
                blend_color(ACCENT_HOV, WHITE, 52)
            } else {
                ACCENT_HOV
            };
            let icon_secondary = if self.start_menu_open {
                blend_color(WHITE, ACCENT_HOV, 70)
            } else if start_pressed {
                blend_color(ACCENT_HOV, WHITE, 30)
            } else if start_hot {
                blend_color(ACCENT_HOV, WHITE, 130)
            } else {
                blend_color(ACCENT_HOV, WHITE, 104)
            };
            // coolOS snowflake mark.
            draw_snowflake_logo(s, sw, tile_x, tile_y, 1, icon_primary, icon_secondary);

            // ── Start menu — Windows 7-style two-column search surface ─────────
            if self.start_menu_open {
                draw_win7_start_menu(
                    s,
                    sw,
                    taskbar_y,
                    mx_i,
                    my_i,
                    self.start_power_menu_open,
                    &self.start_search,
                );
            }

            // ── Taskbar window tabs — icon-first strip ───────────────────────────
            let taskbar_btn_x0 = START_BTN_W + 8;
            let show_desktop_x = sw as i32 - TASKBAR_CLOCK_W - SHOW_DESKTOP_W - 8;
            let mut taskbar_slot = 0usize;
            for i in 0..self.windows.len() {
                if self
                    .window_workspaces
                    .get(i)
                    .copied()
                    .unwrap_or(0)
                    .min(WORKSPACE_COUNT - 1)
                    != current_workspace
                {
                    continue;
                }
                let bx = taskbar_btn_x0 + taskbar_slot as i32 * (BUTTON_W + 6);
                if bx + BUTTON_W > show_desktop_x - 6 {
                    break;
                }
                taskbar_slot += 1;

                let focused = self.focused == Some(i);
                let minimized = self.windows[i].is_minimized();
                let hovered = hovered_taskbar_window == Some(i);
                let title = self.windows[i].window().title;
                let accent = window_accent(title);

                let bh = TASKBAR_H - 4;
                let by = taskbar_y + 2;
                let bg = if focused {
                    0x00_1F_32_42
                } else if hovered {
                    0x00_18_25_32
                } else {
                    0x00_00_00_00
                };

                if focused || hovered {
                    s_fill(s, sw, bx, by, BUTTON_W, bh, bg);
                }
                if focused {
                    // 3px bottom accent bar
                    s_fill(s, sw, bx, taskbar_y + TASKBAR_H - 3, BUTTON_W, 3, accent);
                    // Subtle top glow line
                    s_fill(
                        s,
                        sw,
                        bx,
                        taskbar_y + 2,
                        BUTTON_W,
                        1,
                        blend_color(accent, BLACK, 190),
                    );
                } else if hovered {
                    s_fill(
                        s,
                        sw,
                        bx,
                        taskbar_y + TASKBAR_H - 2,
                        BUTTON_W,
                        1,
                        0x00_3B_4C_5E,
                    );
                }
                if minimized {
                    s_fill(
                        s,
                        sw,
                        bx + BUTTON_W / 2 - 4,
                        taskbar_y + TASKBAR_H - 4,
                        8,
                        2,
                        0x00_69_86_9C,
                    );
                }

                let icon_x = bx + 10;
                let icon_y = by + (bh - 16) / 2;
                draw_shell_app_icon(s, sw, icon_x, icon_y, 16, desktop_icon_kind(title));

                let trunc = if title.len() > 14 {
                    &title[..14]
                } else {
                    title
                };
                let text_w = trunc.len() as i32 * 8;
                let text_x = icon_x + 24;
                s_draw_str_small(
                    s,
                    sw,
                    text_x,
                    by + (bh - 8) / 2,
                    trunc,
                    if focused {
                        0x00_E7_EF_F6
                    } else if hovered {
                        0x00_B5_C7_D4
                    } else {
                        0x00_7B_8D_9D
                    },
                    bg,
                    (text_x + text_w).min(bx + BUTTON_W - 8),
                );
            }

            if let Some(idx) = hovered_taskbar_window {
                if idx < self.windows.len() {
                    let slot = self
                        .windows
                        .iter()
                        .enumerate()
                        .filter(|(win_idx, _)| {
                            self.window_workspaces
                                .get(*win_idx)
                                .copied()
                                .unwrap_or(0)
                                .min(WORKSPACE_COUNT - 1)
                                == current_workspace
                        })
                        .position(|(win_idx, _)| win_idx == idx)
                        .unwrap_or(0);
                    let bx = taskbar_btn_x0 + slot as i32 * (BUTTON_W + 6);
                    draw_taskbar_preview(s, sw, taskbar_y, bx, &self.windows[idx]);
                }
            }

            // ── Clock / system tray ──────────────────────────────────────────────
            draw_taskbar_tray(s, sw, taskbar_y, uptime_ticks, mx_i, my_i);

            // ── Context menu ──────────────────────────────────────────────────────
            if let Some(ref cm) = self.context_menu {
                draw_desktop_context_menu(
                    s,
                    sw,
                    cm,
                    mx_i,
                    my_i,
                    self.desktop_show_icons,
                    self.desktop_compact_spacing,
                    self.desktop_sort,
                    sw as i32,
                    taskbar_y,
                );
            }

            if let Some(ref menu) = self.taskbar_menu {
                draw_taskbar_menu(s, sw, menu, &self.windows, mx_i, my_i);
            }

            if self.notification_center_open {
                draw_notification_center(s, sw, taskbar_y, mx_i, my_i);
            } else {
                draw_notification_toasts(s, sw, taskbar_y, uptime_ticks);
            }

            if self.task_switcher_until_tick > uptime_ticks {
                draw_task_switcher_overlay(
                    s,
                    sw,
                    taskbar_y,
                    &self.windows,
                    &self.window_workspaces,
                    &self.z_order,
                    self.focused,
                    self.current_workspace,
                    &self.task_switcher_query,
                );
            }

            if let Some(ref dialog) = self.dialog {
                draw_shell_dialog(s, sw, taskbar_y, dialog);
            }

            if let Some(ref file_drag) = self.file_drag {
                draw_file_drag_badge(s, sw, mx_i + 16, my_i + 18, file_drag.paths.len());
            }

            if let Some((owner, password_len, confirm_len, device, focus, message, error)) =
                &first_boot_snapshot
            {
                draw_first_boot_overlay(
                    s,
                    sw,
                    taskbar_y,
                    owner,
                    *password_len,
                    *confirm_len,
                    device,
                    *focus,
                    message,
                    *error,
                    mx_i,
                    my_i,
                );
            } else if let Some((user, password_len, focus, message, error, attempts)) =
                &greeter_snapshot
            {
                draw_greeter_overlay(
                    s,
                    sw,
                    taskbar_y,
                    user,
                    *password_len,
                    *focus,
                    message,
                    *error,
                    *attempts,
                    mx_i,
                    my_i,
                    uptime_ticks,
                );
            }
        } // end shadow borrow — rendering done

        self.compute_damage_spans(sw, sh);

        self.blit_damage_spans_to_hw(sh);
        let cursor_pixels =
            self.restore_cursor_backing() + self.draw_cursor_overlay(mx_i, my_i, resize_hover);
        COMPOSITOR_CURSOR_PIXELS_LAST.store(cursor_pixels as u64, Ordering::Relaxed);
        self.update_compositor_telemetry(frame_start_tick);
    }

    fn blit_damage_spans_to_hw(&mut self, sh: usize) -> usize {
        let mut pixels = 0usize;
        for row in 0..sh {
            let (x0, x1) = self.damage_spans[row];
            if x0 >= x1 {
                continue;
            }
            pixels += self.blit_shadow_span_to_hw(row, x0, x1);
        }
        pixels
    }

    fn restore_cursor_backing(&mut self) -> usize {
        if !self.cursor_drawn {
            return 0;
        }
        self.blit_shadow_rect_to_hw(
            self.cursor_hw_x,
            self.cursor_hw_y,
            CURSOR_W as i32,
            CURSOR_H as i32,
        )
    }

    fn blit_shadow_rect_to_hw(&mut self, x: i32, y: i32, w: i32, h: i32) -> usize {
        let sw = self.shadow_width;
        let sh = self.shadow_height;
        let x0 = x.clamp(0, sw as i32) as usize;
        let y0 = y.clamp(0, sh as i32) as usize;
        let x1 = (x + w).clamp(0, sw as i32) as usize;
        let y1 = (y + h).clamp(0, sh as i32) as usize;
        if x0 >= x1 || y0 >= y1 {
            return 0;
        }
        let mut pixels = 0usize;
        for row in y0..y1 {
            pixels += self.blit_shadow_span_to_hw(row, x0, x1);
        }
        pixels
    }

    fn blit_shadow_span_to_hw(&mut self, row: usize, x0: usize, x1: usize) -> usize {
        let sw = self.shadow_width;
        let sh = self.shadow_height;
        if row >= sh || x0 >= x1 || x1 > sw {
            return 0;
        }
        let hw_base = crate::framebuffer::base();
        if hw_base == 0 {
            return 0;
        }
        let hw_stride = crate::framebuffer::stride();
        let hw_bpp = crate::framebuffer::bpp();
        let hw_fmt = crate::framebuffer::fmt();
        let is_rgb = hw_fmt == crate::framebuffer::PixFmt::Rgb;
        match hw_bpp {
            4 => {
                let src = &self.shadow[row * sw + x0..row * sw + x1];
                let row_base = hw_base + (row * hw_stride * 4) as u64;
                let dst = row_base as *mut u32;
                if !is_rgb {
                    unsafe {
                        core::ptr::copy_nonoverlapping(src.as_ptr(), dst.add(x0), x1 - x0);
                    }
                } else {
                    for col in x0..x1 {
                        let c = self.shadow[row * sw + col];
                        let hw = ((c & 0xFF) << 16) | (c & 0x00FF00) | (c >> 16 & 0xFF);
                        unsafe {
                            dst.add(col).write_volatile(hw);
                        }
                    }
                }
            }
            3 => {
                let src = &self.shadow[row * sw + x0..row * sw + x1];
                let row_bytes = (x1 - x0) * 3;
                if self.blit_scratch.len() < row_bytes {
                    self.blit_scratch.resize(row_bytes, 0);
                }
                let scratch = &mut self.blit_scratch[..row_bytes];
                if !is_rgb {
                    for (col, c) in src.iter().copied().enumerate() {
                        scratch[col * 3] = c as u8;
                        scratch[col * 3 + 1] = (c >> 8) as u8;
                        scratch[col * 3 + 2] = (c >> 16) as u8;
                    }
                } else {
                    for (col, c) in src.iter().copied().enumerate() {
                        scratch[col * 3] = (c >> 16) as u8;
                        scratch[col * 3 + 1] = (c >> 8) as u8;
                        scratch[col * 3 + 2] = c as u8;
                    }
                }
                let row_base = hw_base + (row * hw_stride * 3 + x0 * 3) as u64;
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        scratch.as_ptr(),
                        row_base as *mut u8,
                        row_bytes,
                    );
                }
            }
            _ => return 0,
        }
        x1 - x0
    }

    fn draw_cursor_overlay(&mut self, x: i32, y: i32, resize: bool) -> usize {
        if crate::framebuffer::base() == 0 {
            self.cursor_drawn = false;
            return 0;
        }
        let (outline, shape) = if resize {
            (&CURSOR_RESIZE_OUTLINE, &CURSOR_RESIZE_SHAPE)
        } else {
            (&CURSOR_OUTLINE, &CURSOR_SHAPE)
        };
        let mut pixels = 0usize;
        for row in 0..CURSOR_H {
            for bit in 0..CURSOR_W {
                let mask = 0x8000u16 >> bit;
                let color = if shape[row] & mask != 0 {
                    Some(WHITE)
                } else if outline[row] & mask != 0 {
                    Some(BLACK)
                } else {
                    None
                };
                if let Some(color) = color {
                    if self.write_hw_pixel(x + bit as i32, y + row as i32, color) {
                        pixels += 1;
                    }
                }
            }
        }
        self.cursor_drawn = true;
        self.cursor_hw_x = x;
        self.cursor_hw_y = y;
        pixels
    }

    fn write_hw_pixel(&self, x: i32, y: i32, color: u32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.shadow_width || y >= self.shadow_height {
            return false;
        }
        let hw_base = crate::framebuffer::base();
        if hw_base == 0 {
            return false;
        }
        let hw_stride = crate::framebuffer::stride();
        let is_rgb = crate::framebuffer::fmt() == crate::framebuffer::PixFmt::Rgb;
        match crate::framebuffer::bpp() {
            4 => {
                let hw = if is_rgb {
                    ((color & 0xFF) << 16) | (color & 0x00FF00) | (color >> 16 & 0xFF)
                } else {
                    color
                };
                let ptr = (hw_base + (y * hw_stride * 4 + x * 4) as u64) as *mut u32;
                unsafe {
                    ptr.write_volatile(hw);
                }
                true
            }
            3 => {
                let ptr = (hw_base + (y * hw_stride * 3 + x * 3) as u64) as *mut u8;
                let (r, g, b) = ((color >> 16) as u8, (color >> 8) as u8, color as u8);
                unsafe {
                    if is_rgb {
                        ptr.add(0).write_volatile(r);
                        ptr.add(1).write_volatile(g);
                        ptr.add(2).write_volatile(b);
                    } else {
                        ptr.add(0).write_volatile(b);
                        ptr.add(1).write_volatile(g);
                        ptr.add(2).write_volatile(r);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn update_compositor_telemetry(&mut self, frame_start_tick: u64) {
        let now = crate::interrupts::ticks();
        let frame_ticks = now.wrapping_sub(frame_start_tick);
        let budget_ticks = crate::wm::target_frame_budget_ticks();
        self.frame_ticks_peak = self.frame_ticks_peak.max(frame_ticks);
        self.fps_window_frames = self.fps_window_frames.saturating_add(1);
        COMPOSITOR_FULL_FRAMES.fetch_add(1, Ordering::Relaxed);
        COMPOSITOR_FRAME_BUDGET_TICKS.store(budget_ticks, Ordering::Relaxed);
        if frame_ticks > budget_ticks {
            COMPOSITOR_FRAME_BUDGET_MISSES.fetch_add(1, Ordering::Relaxed);
        }
        if self.fps_window_start_tick == 0 {
            self.fps_window_start_tick = now;
        }
        let elapsed = now.wrapping_sub(self.fps_window_start_tick);
        if elapsed >= crate::interrupts::TIMER_HZ as u64 {
            let fps = self
                .fps_window_frames
                .saturating_mul(crate::interrupts::TIMER_HZ as u64)
                / elapsed.max(1);
            COMPOSITOR_FPS.store(fps, Ordering::Relaxed);
            COMPOSITOR_FRAME_TICKS_PEAK.store(self.frame_ticks_peak, Ordering::Relaxed);
            self.fps_window_frames = 0;
            self.fps_window_start_tick = now;
            self.frame_ticks_peak = 0;
        }
        COMPOSITOR_FRAME_TICKS_LAST.store(frame_ticks, Ordering::Relaxed);
        COMPOSITOR_DAMAGE_ROWS.store(self.damage_rows_last as u64, Ordering::Relaxed);
        COMPOSITOR_DAMAGE_PIXELS.store(self.damage_pixels_last as u64, Ordering::Relaxed);
        COMPOSITOR_FRAMES.store(self.damage_frames, Ordering::Relaxed);
    }

    fn compute_damage_spans(&mut self, sw: usize, sh: usize) {
        let total = sw.saturating_mul(sh);
        let track_prev = self.prev_shadow.len() == total;
        if !track_prev {
            self.full_damage_next = true;
        }
        if self.damage_spans.len() != sh {
            self.damage_spans.resize(sh, (0, 0));
            self.full_damage_next = true;
        }
        if self.reported_damage_spans.len() != sh {
            self.reported_damage_spans.resize(sh, (0, 0));
        }

        let mut rows = 0usize;
        let mut pixels = 0usize;
        for row in 0..sh {
            let start_idx = row * sw;
            let end_idx = start_idx + sw;
            let span = if self.full_damage_next {
                (0, sw)
            } else {
                let cur = &self.shadow[start_idx..end_idx];
                let prev = &self.prev_shadow[start_idx..end_idx];
                let mut first = sw;
                let mut last = 0usize;
                for col in 0..sw {
                    if cur[col] != prev[col] {
                        if first == sw {
                            first = col;
                        }
                        last = col + 1;
                    }
                }
                if first == sw {
                    (0, 0)
                } else {
                    (first, last)
                }
            };
            let reported = self.reported_damage_spans[row];
            let span = merge_spans(span, reported);
            self.damage_spans[row] = span;
            if span.0 < span.1 {
                rows += 1;
                pixels += span.1 - span.0;
                if track_prev {
                    self.prev_shadow[start_idx..end_idx]
                        .copy_from_slice(&self.shadow[start_idx..end_idx]);
                }
            }
        }

        self.full_damage_next = false;
        self.damage_rows_last = rows;
        self.damage_pixels_last = pixels;
        self.damage_frames = self.damage_frames.saturating_add(1);
        if rows > 0 && self.damage_frames % 60 == 0 {
            crate::profiler::record(
                "compositor",
                "damage",
                &format!("rows={} pixels={}", rows, pixels),
            );
        }
    }

    fn collect_app_dirty_spans(&mut self, sw: usize, sh: usize) {
        if self.reported_damage_spans.len() != sh {
            self.reported_damage_spans.resize(sh, (0, 0));
        }
        for span in self.reported_damage_spans.iter_mut() {
            *span = (0, 0);
        }
        let current_workspace = self.current_workspace;
        for (idx, app) in self.windows.iter_mut().enumerate() {
            if self
                .window_workspaces
                .get(idx)
                .copied()
                .unwrap_or(0)
                .min(WORKSPACE_COUNT - 1)
                != current_workspace
                || app.window().minimized
            {
                continue;
            }
            let win_x = app.window().x;
            let win_y = app.window().y + TITLE_H;
            let rects = app.window_mut().take_dirty_regions();
            for rect in rects {
                let x0 = (win_x + rect.x).clamp(0, sw as i32) as usize;
                let x1 = (win_x + rect.x + rect.w).clamp(0, sw as i32) as usize;
                let y0 = (win_y + rect.y).clamp(0, sh as i32) as usize;
                let y1 = (win_y + rect.y + rect.h).clamp(0, sh as i32) as usize;
                if x0 >= x1 || y0 >= y1 {
                    continue;
                }
                for row in y0..y1 {
                    self.reported_damage_spans[row] =
                        merge_spans(self.reported_damage_spans[row], (x0, x1));
                }
            }
        }
    }

    fn save_focused_screenshot(&self, path: &str) -> Result<(), &'static str> {
        let visible_focused = self.focused.and_then(|idx| {
            self.windows
                .get(idx)
                .filter(|app| self.is_window_on_current_workspace(idx) && !app.is_minimized())
                .map(|_| idx)
        });
        let Some(win_idx) = visible_focused.or_else(|| self.top_visible_window()) else {
            return Err("no focused window");
        };
        let Some(app) = self.windows.get(win_idx) else {
            return Err("focused window missing");
        };
        let win = app.window();
        let width = win.width.max(1) as usize;
        let height = (win.height - TITLE_H).max(1) as usize;
        if win.buf.len() < width.saturating_mul(height) {
            return Err("window buffer incomplete");
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"P6\n");
        push_usize_bytes(&mut bytes, width);
        bytes.push(b' ');
        push_usize_bytes(&mut bytes, height);
        bytes.extend_from_slice(b"\n255\n");
        for pixel in win.buf.iter().take(width * height) {
            bytes.push(((pixel >> 16) & 0xFF) as u8);
            bytes.push(((pixel >> 8) & 0xFF) as u8);
            bytes.push((pixel & 0xFF) as u8);
        }

        let _ = crate::vfs::vfs_kernel_create_dir("/LOGS");
        match crate::vfs::vfs_kernel_create_file(path) {
            Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
            Err(err) => return Err(err.as_str()),
        }
        crate::vfs::vfs_kernel_write_file(path, &bytes).map_err(|err| err.as_str())
    }

    fn handle_dialog_click(&mut self, px: i32, py: i32, sw: i32, taskbar_y: i32) {
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        let (x, y, w, h) = shell_dialog_rect(sw, taskbar_y, &dialog);
        let button_y = y + h - 34;
        let button_h = 22;

        if dialog.kind == ShellDialogKind::Crash
            && py >= button_y
            && py < button_y + button_h
            && px >= x + 18
            && px < x + w - 18
        {
            let view_x = x + 18;
            let restart_x = view_x + 104;
            let copy_x = restart_x + 104;
            if px >= view_x && px < view_x + 94 {
                self.dialog = None;
                let off = self.windows.len() as i32 * 16;
                self.launch_app("Crash Viewer", x + off / 2, y + off / 2);
                return;
            }
            if px >= restart_x && px < restart_x + 94 {
                self.dialog = None;
                if let Some(target) = dialog.restart_target.as_ref() {
                    if target.starts_with('/') {
                        match crate::elf::spawn_elf_process(target) {
                            Ok(_) => {
                                crate::crashdump::record_restart(target);
                                crate::notifications::push("App restarted", target);
                            }
                            Err(err) => crate::notifications::push("Restart failed", err.as_str()),
                        }
                    } else {
                        crate::crashdump::record_restart(target);
                        self.launch_app(target, x + 18, y + 18);
                    }
                } else {
                    crate::notifications::push("Restart unavailable", "no app target recorded");
                }
                return;
            }
            if px >= copy_x && px < copy_x + 94 {
                crate::clipboard::set_text(&dialog.body);
                crate::notifications::push("Crash details copied", "details copied to clipboard");
                return;
            }
        }

        self.dialog = None;
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn front_to_back_hit(&self, px: i32, py: i32) -> Option<usize> {
        for z_pos in (0..self.z_order.len()).rev() {
            let wi = self.z_order[z_pos];
            if wi < self.windows.len()
                && self.is_window_on_current_workspace(wi)
                && !self.windows[wi].window().minimized
                && self.windows[wi].window().hit(px, py)
            {
                return Some(z_pos);
            }
        }
        None
    }

    /// Draw a single window — Windows 11 Dark Mode chrome.
    fn draw_window(
        s: &mut [u32],
        sw: usize,
        w: &Window,
        focused: bool,
        cursor_x: i32,
        cursor_y: i32,
    ) {
        // ── Drop shadow ───────────────────────────────────────────────────────
        const SHADOW_R: i32 = 10;
        for d in 1..=SHADOW_R {
            let t = (SHADOW_R - d + 1) as u32;
            let alpha = (t * t * 2 / SHADOW_R as u32).min(38);
            let shadow_col = alpha << 24;
            let sx = w.x + w.width + d - 1;
            let sy = w.y + w.height + d - 1;
            s_fill_alpha(s, sw, sx, w.y + d, 1, w.height, shadow_col);
            s_fill_alpha(s, sw, w.x + d, sy, w.width, 1, shadow_col);
            s_fill_alpha(s, sw, w.x - d, w.y + d, 1, w.height, shadow_col);
            s_fill_alpha(s, sw, w.x + d, w.y - d, w.width, 1, shadow_col);
        }

        if focused {
            let outer_glow = blend_color(ACCENT, BLACK, 176);
            draw_rect_border(
                s,
                sw,
                w.x - 2,
                w.y - 2,
                w.width + 4,
                w.height + 4,
                outer_glow,
            );
            draw_rect_border(
                s,
                sw,
                w.x - 1,
                w.y - 1,
                w.width + 2,
                w.height + 2,
                WIN_BDR_F,
            );
        }

        // ── Title bar ────────────────────────────────────────────────────────
        let title_bg = if focused { WIN_BAR_F } else { WIN_BAR_U };
        let title_top = if focused {
            blend_color(WIN_BAR_F, WHITE, 12)
        } else {
            blend_color(WIN_BAR_U, WHITE, 6)
        };
        let title_mid = if focused { WIN_BAR_F } else { WIN_BAR_U };
        for row in 0..TITLE_H {
            let t = (row * 255 / TITLE_H.max(1)) as u32;
            let shade = if t < 128 {
                blend_color(title_top, title_mid, t * 2)
            } else {
                blend_color(title_mid, title_bg, (t - 128) * 2)
            };
            s_fill(s, sw, w.x, w.y + row, w.width, 1, shade);
        }

        // ── Window border ─────────────────────────────────────────────────────
        let bord = if focused { WIN_BDR_F } else { WIN_BDR_U };
        let bord_inner = if focused {
            blend_color(WIN_BDR_F, WHITE, 42)
        } else {
            blend_color(WIN_BDR_U, WHITE, 18)
        };
        s_fill(s, sw, w.x - 1, w.y - 1, w.width + 2, 1, bord); // top outer
        s_fill(s, sw, w.x, w.y, w.width, 1, bord_inner); // top inner shine
        s_fill(s, sw, w.x - 1, w.y + w.height, w.width + 2, 1, bord); // bottom
        s_fill(s, sw, w.x - 1, w.y, 1, w.height, bord); // left
        s_fill(s, sw, w.x + w.width, w.y, 1, w.height, bord); // right

        // ── Title icon + text ─────────────────────────────────────────────────
        let title_content_y = w.y + 3;
        let title_content_h = TITLE_H - 3;
        let icon_x = w.x + 8;
        let icon_y = title_content_y + (title_content_h - 18) / 2;
        draw_shell_app_icon(s, sw, icon_x, icon_y, 18, desktop_icon_kind(w.title));

        let max_title_x = w.x + w.width - WIN_BTN_W * 3 - 10;
        let title_fg = if focused {
            0x00_E7_EF_F6
        } else {
            0x00_7F_8D_9B
        };
        s_draw_str_small(
            s,
            sw,
            w.x + 34,
            title_content_y + (title_content_h - 8) / 2,
            w.title,
            title_fg,
            title_bg,
            max_title_x,
        );

        // ── Caption buttons ──────────────────────────────────────────────────
        let btn_y = w.y + 1;
        let btn_h = TITLE_H - 2;
        let cap_glyph_mid_y = btn_y + 3 + (btn_h - 3) / 2;

        // Hover detection — only fire when cursor is over this window's title row
        let in_btn_row = cursor_y >= btn_y && cursor_y < btn_y + btn_h;
        let min_x = w.x + w.width - WIN_BTN_W * 3;
        let max_x = w.x + w.width - WIN_BTN_W * 2;
        let cls_x = w.x + w.width - WIN_BTN_W;
        let hover_min = in_btn_row && cursor_x >= min_x && cursor_x < min_x + WIN_BTN_W;
        let hover_max = in_btn_row && cursor_x >= max_x && cursor_x < max_x + WIN_BTN_W;
        let hover_close = in_btn_row && cursor_x >= cls_x && cursor_x < cls_x + WIN_BTN_W;

        // Minimize  ─
        s_fill(
            s,
            sw,
            min_x,
            btn_y,
            WIN_BTN_W,
            btn_h,
            if hover_min { CAP_HOV } else { CAP_NORMAL },
        );
        let min_glyph = if hover_min { WHITE } else { 0x00_90_A4_B8 };
        s_fill(
            s,
            sw,
            min_x + WIN_BTN_W / 2 - 4,
            cap_glyph_mid_y,
            8,
            1,
            min_glyph,
        );

        // Maximize  □
        s_fill(
            s,
            sw,
            max_x,
            btn_y,
            WIN_BTN_W,
            btn_h,
            if hover_max { CAP_HOV } else { CAP_NORMAL },
        );
        let max_glyph = if hover_max { WHITE } else { 0x00_90_A4_B8 };
        s_fill(
            s,
            sw,
            max_x + WIN_BTN_W / 2 - 4,
            cap_glyph_mid_y - 4,
            8,
            1,
            max_glyph,
        );
        s_fill(
            s,
            sw,
            max_x + WIN_BTN_W / 2 - 4,
            cap_glyph_mid_y + 3,
            8,
            1,
            max_glyph,
        );
        s_fill(
            s,
            sw,
            max_x + WIN_BTN_W / 2 - 4,
            cap_glyph_mid_y - 4,
            1,
            8,
            max_glyph,
        );
        s_fill(
            s,
            sw,
            max_x + WIN_BTN_W / 2 + 3,
            cap_glyph_mid_y - 4,
            1,
            8,
            max_glyph,
        );

        // Close  ✕ — pixel diagonals
        let cx_c = cls_x + WIN_BTN_W / 2;
        let cy_c = cap_glyph_mid_y;
        let sh_wnd = s.len() / sw;
        s_fill(
            s,
            sw,
            cls_x,
            btn_y,
            WIN_BTN_W,
            btn_h,
            if hover_close { CLOSE_HOV } else { CLOSE_REST },
        );
        let cls_glyph = if hover_close { WHITE } else { 0x00_F4_77_77 };
        for i in -3i32..=3 {
            s_put(s, sw, sh_wnd, cx_c + i, cy_c + i, cls_glyph);
            s_put(s, sw, sh_wnd, cx_c + i + 1, cy_c + i, cls_glyph);
            s_put(s, sw, sh_wnd, cx_c + i, cy_c - i, cls_glyph);
            s_put(s, sw, sh_wnd, cx_c + i + 1, cy_c - i, cls_glyph);
        }

        // Accent rail drawn last so it runs end-to-end over the caption buttons too.
        let rail = if focused {
            ACCENT
        } else {
            blend_color(ACCENT, WIN_BAR_U, 82)
        };
        s_fill(s, sw, w.x, w.y, w.width, if focused { 2 } else { 1 }, rail);
        s_fill(
            s,
            sw,
            w.x,
            w.y + if focused { 2 } else { 1 },
            w.width,
            1,
            blend_color(rail, title_bg, 150),
        );
        if focused {
            s_fill(
                s,
                sw,
                w.x,
                w.y + TITLE_H - 1,
                w.width,
                1,
                blend_color(WIN_BDR_F, WIN_BAR_F, 112),
            );
        }

        // ── Content area ──────────────────────────────────────────────────────
        let content_y = w.y + TITLE_H;
        let content_h = (w.height - TITLE_H).max(0) as usize;
        let cw = w.width as usize;
        let sh = s.len() / sw;
        let dst_x0 = w.x.max(0) as usize;
        let dst_x1 = (w.x + w.width).min(sw as i32).max(0) as usize;
        let dst_y0 = content_y.max(0) as usize;
        let dst_y1 = (content_y + content_h as i32).min(sh as i32).max(0) as usize;

        if dst_x0 < dst_x1 && dst_y0 < dst_y1 {
            let src_x0 = (dst_x0 as i32 - w.x) as usize;
            let src_y0 = (dst_y0 as i32 - content_y) as usize;
            let visible_w = dst_x1 - dst_x0;

            for dst_y in dst_y0..dst_y1 {
                let src_row = src_y0 + (dst_y - dst_y0);
                let row_start = src_row * cw + src_x0;
                let src = &w.buf[row_start..row_start + visible_w];
                let dst = &mut s[dst_y * sw + dst_x0..dst_y * sw + dst_x1];

                if !src.contains(&WIN_TRANSPARENT) {
                    dst.copy_from_slice(src);
                    continue;
                }

                for (out, &pixel) in dst.iter_mut().zip(src.iter()) {
                    let base = if pixel == WIN_TRANSPARENT {
                        WIN_CONTENT
                    } else {
                        pixel
                    };
                    *out = base;
                }
            }
        }

        // ── Scrollbar ─────────────────────────────────────────────────────────
        let view_h = content_h as i32;
        if w.scroll.needs_scrollbar(view_h) {
            let sb_x = w.x + w.width - SCROLLBAR_W;
            let track_h = view_h;
            s_fill(s, sw, sb_x, content_y, SCROLLBAR_W, track_h, 0x00_0B_11_19);
            s_fill(s, sw, sb_x, content_y, 1, track_h, 0x00_23_2F_3D);
            let (thumb_y, thumb_h) = w.scroll.thumb_rect(view_h, track_h);
            let thumb_col = if focused {
                0x00_38_5A_6E
            } else {
                0x00_28_35_46
            };
            let thumb_highlight = if focused {
                0x00_5C_AE_C3
            } else {
                0x00_44_55_66
            };
            s_fill(
                s,
                sw,
                sb_x + 2,
                content_y + thumb_y,
                SCROLLBAR_W - 4,
                thumb_h,
                thumb_col,
            );
            s_fill(
                s,
                sw,
                sb_x + 2,
                content_y + thumb_y,
                SCROLLBAR_W - 4,
                1,
                thumb_highlight,
            );
            s_fill(
                s,
                sw,
                sb_x + 2,
                content_y + thumb_y + thumb_h - 1,
                SCROLLBAR_W - 4,
                1,
                blend_color(thumb_col, BLACK, 80),
            );
        }

        // ── Resize handle — diagonal dot-grip in bottom-right corner ──────────
        {
            let hx = w.x + w.width - RESIZE_HANDLE;
            let hy = w.y + w.height - RESIZE_HANDLE;
            let gc = if focused {
                0x00_5C_AE_C3
            } else {
                0x00_35_43_54
            };
            let gc_dim = if focused {
                0x00_2E_5C_70
            } else {
                0x00_1A_25_32
            };
            s_fill(s, sw, hx + 7, hy + 7, 2, 2, gc);
            s_fill(s, sw, hx + 5, hy + 7, 2, 2, gc_dim);
            s_fill(s, sw, hx + 7, hy + 5, 2, 2, gc_dim);
            s_fill(s, sw, hx + 3, hy + 7, 2, 2, gc_dim);
            s_fill(s, sw, hx + 7, hy + 3, 2, 2, gc_dim);
        }
    }
}

fn key_event_packet(c: char) -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    let mut utf8 = [0u8; 4];
    let encoded = c.encode_utf8(&mut utf8);
    packet[0] = EVENT_KIND_KEY_CHAR;
    packet[1] = encoded.len() as u8;
    packet[2..2 + encoded.len()].copy_from_slice(encoded.as_bytes());
    packet
}

fn mouse_event_packet(buttons: u8, lx: i32, ly: i32) -> [u8; EVENT_PACKET_SIZE] {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    let x = lx.clamp(0, u16::MAX as i32) as u16;
    let y = ly.clamp(0, u16::MAX as i32) as u16;
    packet[0] = EVENT_KIND_MOUSE_DOWN;
    packet[1] = buttons;
    packet[2..4].copy_from_slice(&x.to_le_bytes());
    packet[4..6].copy_from_slice(&y.to_le_bytes());
    packet
}

fn ctrl_number_slot(c: char) -> Option<usize> {
    match c {
        '1' => Some(0),
        '2' => Some(1),
        '3' => Some(2),
        '4' => Some(3),
        '5' => Some(4),
        '6' => Some(5),
        '7' => Some(6),
        '8' => Some(7),
        '9' => Some(8),
        '0' => Some(9),
        _ => None,
    }
}

fn draw_win7_start_menu(
    s: &mut [u32],
    sw: usize,
    taskbar_y: i32,
    mx: i32,
    my: i32,
    power_open: bool,
    start_search: &StartSearchState,
) {
    let layout = win7_start_menu_layout(sw as i32, taskbar_y);
    let bottom_y = layout.menu_y + layout.menu_h - START_MENU_WIN7_BOTTOM_H;
    let frame = 0x00_2A_3A_4A;
    let left_bg = 0x00_10_17_22;
    let left_alt = 0x00_14_20_2C;
    let right_bg = 0x00_0D_20_30;
    let row_hot = 0x00_1A_2A_38;
    let text = 0x00_E7_EF_F6;
    let muted = 0x00_90_A4_B8;

    s_fill(
        s,
        sw,
        layout.menu_x,
        layout.menu_y,
        layout.left_w,
        layout.menu_h,
        left_bg,
    );
    s_fill(
        s,
        sw,
        layout.right_x,
        layout.menu_y,
        layout.right_w,
        layout.menu_h,
        right_bg,
    );
    s_fill(
        s,
        sw,
        layout.menu_x + 1,
        bottom_y,
        layout.menu_w - 2,
        START_MENU_WIN7_BOTTOM_H - 1,
        0x00_0A_10_18,
    );
    s_fill(
        s,
        sw,
        layout.menu_x + layout.left_w,
        layout.menu_y + 3,
        1,
        layout.menu_h - 4,
        frame,
    );
    s_fill(
        s,
        sw,
        layout.menu_x + 1,
        bottom_y,
        layout.menu_w - 2,
        1,
        frame,
    );
    s_fill(
        s,
        sw,
        layout.menu_x,
        layout.menu_y,
        layout.menu_w,
        2,
        ACCENT,
    );
    s_fill(
        s,
        sw,
        layout.menu_x + 1,
        layout.menu_y + 2,
        layout.menu_w - 2,
        1,
        blend_color(ACCENT, left_bg, 150),
    );
    draw_rect_border(
        s,
        sw,
        layout.menu_x,
        layout.menu_y,
        layout.menu_w,
        layout.menu_h,
        0x00_3B_4C_5E,
    );
    draw_rect_border(
        s,
        sw,
        layout.menu_x + 1,
        layout.menu_y + 1,
        layout.menu_w - 2,
        layout.menu_h - 2,
        0x00_18_25_32,
    );
    s_fill(
        s,
        sw,
        layout.menu_x,
        layout.menu_y,
        layout.menu_w,
        2,
        ACCENT,
    );
    s_fill(
        s,
        sw,
        layout.menu_x + 1,
        layout.menu_y + 2,
        layout.menu_w - 2,
        1,
        blend_color(ACCENT, left_bg, 150),
    );
    draw_glass_panel_outline(
        s,
        sw,
        layout.menu_x,
        layout.menu_y,
        layout.menu_w,
        layout.menu_h,
        ACCENT,
    );

    let results = start_menu_results(start_search);
    let visible_rows = start_menu_visible_rows(layout, results.len());
    let search_active = start_search.show_all || !start_search.query.trim().is_empty();
    if results.is_empty() {
        s_draw_str_small(
            s,
            sw,
            layout.list_x + 12,
            layout.list_y + 14,
            "No matching apps or files",
            muted,
            left_bg,
            layout.list_x + layout.list_w - 8,
        );
    }
    for (idx, result) in results.iter().take(visible_rows).enumerate() {
        let y = layout.list_y + idx as i32 * layout.row_h;
        let hot = rect_contains(layout.list_x, y, layout.list_w, layout.row_h, mx, my);
        let selected = search_active && idx == start_search.selected.min(results.len() - 1);
        let row_bg = if selected {
            0x00_1B_32_42
        } else if hot {
            row_hot
        } else {
            left_bg
        };
        if hot || selected {
            s_fill(
                s,
                sw,
                layout.list_x,
                y,
                layout.list_w,
                layout.row_h - 1,
                row_bg,
            );
            s_fill(
                s,
                sw,
                layout.list_x,
                y + 7,
                3,
                layout.row_h - 14,
                if selected { ACCENT_HOV } else { ACCENT },
            );
        } else if idx > 0 {
            s_fill(
                s,
                sw,
                layout.list_x + 34,
                y,
                layout.list_w - 42,
                1,
                0x00_1A_25_32,
            );
        }

        let icon_x = layout.list_x + 8;
        let icon_y = y + (layout.row_h - 24) / 2;
        draw_start_menu_app_icon(s, sw, icon_x, icon_y, start_search_icon_kind(result));
        s_draw_str_small(
            s,
            sw,
            icon_x + 34,
            y + 12,
            &result.label,
            if hot || selected { WHITE } else { text },
            row_bg,
            layout.menu_x + layout.left_w - 12,
        );
    }

    let all_hot = rect_contains(
        layout.all_x,
        layout.all_y,
        layout.all_w,
        layout.all_h,
        mx,
        my,
    );
    let all_bg = if all_hot { row_hot } else { left_bg };
    s_fill(
        s,
        sw,
        layout.all_x,
        layout.all_y - 1,
        layout.all_w,
        1,
        frame,
    );
    if all_hot {
        s_fill(
            s,
            sw,
            layout.all_x,
            layout.all_y,
            layout.all_w,
            layout.all_h,
            all_bg,
        );
    }
    let chevron_x = layout.all_x + 12;
    let chevron_y = layout.all_y + layout.all_h / 2;
    let chevron = if all_hot { WHITE } else { muted };
    s_fill(s, sw, chevron_x, chevron_y - 5, 1, 10, chevron);
    s_fill(s, sw, chevron_x + 1, chevron_y - 4, 1, 8, chevron);
    s_fill(s, sw, chevron_x + 2, chevron_y - 3, 1, 6, chevron);
    s_draw_str_small(
        s,
        sw,
        layout.all_x + 30,
        layout.all_y + 12,
        "All Programs",
        if all_hot { WHITE } else { text },
        all_bg,
        layout.all_x + layout.all_w - 8,
    );

    let search_hot = rect_contains(
        layout.search_x,
        layout.search_y,
        layout.search_w,
        layout.search_h,
        mx,
        my,
    );
    let search_bg = if search_hot {
        0x00_12_1C_28
    } else {
        0x00_06_0B_12
    };
    s_fill(
        s,
        sw,
        layout.search_x,
        layout.search_y,
        layout.search_w,
        layout.search_h,
        search_bg,
    );
    draw_rect_border(
        s,
        sw,
        layout.search_x,
        layout.search_y,
        layout.search_w,
        layout.search_h,
        if start_search.focused || search_hot {
            ACCENT
        } else {
            frame
        },
    );
    let sg_x = layout.search_x + layout.search_w - 20;
    let sg_y = layout.search_y + 8;
    let sg_col = if start_search.focused || search_hot {
        ACCENT
    } else {
        muted
    };
    s_fill(s, sw, sg_x + 1, sg_y, 5, 1, sg_col);
    s_fill(s, sw, sg_x, sg_y + 1, 1, 5, sg_col);
    s_fill(s, sw, sg_x + 6, sg_y + 1, 1, 5, sg_col);
    s_fill(s, sw, sg_x + 1, sg_y + 6, 5, 1, sg_col);
    s_fill(s, sw, sg_x + 6, sg_y + 6, 1, 1, sg_col);
    s_fill(s, sw, sg_x + 7, sg_y + 7, 1, 1, sg_col);
    s_fill(s, sw, sg_x + 8, sg_y + 8, 1, 1, sg_col);
    let search_text = if start_search.query.is_empty() {
        "Search programs and files"
    } else {
        &start_search.query
    };
    s_draw_str_small(
        s,
        sw,
        layout.search_x + 8,
        layout.search_y + 9,
        search_text,
        if start_search.query.is_empty() {
            muted
        } else {
            WHITE
        },
        search_bg,
        sg_x - 4,
    );
    if start_search.focused {
        let cursor_x = (layout.search_x + 8 + start_search.query.len() as i32 * 8).min(sg_x - 6);
        s_fill(
            s,
            sw,
            cursor_x,
            layout.search_y + 8,
            1,
            layout.search_h - 15,
            ACCENT_HOV,
        );
        s_fill(
            s,
            sw,
            layout.search_x + 1,
            layout.search_y + layout.search_h - 2,
            layout.search_w - 2,
            1,
            ACCENT,
        );
    }

    s_fill(
        s,
        sw,
        layout.right_x + 1,
        layout.menu_y + 2,
        layout.right_w - 2,
        28,
        0x00_14_2A_3E,
    );
    s_fill(
        s,
        sw,
        layout.avatar_x,
        layout.avatar_y,
        layout.avatar_w,
        layout.avatar_h,
        left_alt,
    );
    draw_glass_panel_outline(
        s,
        sw,
        layout.avatar_x,
        layout.avatar_y,
        layout.avatar_w,
        layout.avatar_h,
        ACCENT,
    );
    draw_rect_border(
        s,
        sw,
        layout.avatar_x + 2,
        layout.avatar_y + 2,
        layout.avatar_w - 4,
        layout.avatar_h - 4,
        0x00_C8_F7_FF,
    );
    draw_snowflake_logo(
        s,
        sw,
        layout.avatar_x + 19,
        layout.avatar_y + 19,
        1,
        WHITE,
        ACCENT_HOV,
    );

    let user = crate::security::current_user();
    for idx in 0..(win7_start_right_links().len() + 1) {
        let y = layout.links_y + idx as i32 * layout.link_h;
        if y + layout.link_h > bottom_y - 8 {
            break;
        }
        let hot = rect_contains(layout.links_x, y, layout.links_w, layout.link_h, mx, my);
        let row_bg = if hot { 0x00_18_2B_3D } else { right_bg };
        if hot {
            s_fill(
                s,
                sw,
                layout.links_x,
                y,
                layout.links_w,
                layout.link_h - 1,
                row_bg,
            );
            s_fill(s, sw, layout.links_x, y + 8, 2, layout.link_h - 16, ACCENT);
        }
        let label: &str = if idx == 0 {
            &user.name
        } else {
            win7_start_right_links()[idx - 1].label
        };
        s_draw_str_small(
            s,
            sw,
            layout.links_x + 6,
            y + 13,
            label,
            if hot { WHITE } else { text },
            row_bg,
            layout.links_x + layout.links_w - 2,
        );
        if idx == 3 || idx == 4 {
            s_fill(
                s,
                sw,
                layout.links_x + 4,
                y + layout.link_h - 1,
                layout.links_w - 8,
                1,
                0x00_2A_3A_4A,
            );
        }
    }

    let shutdown_hot = rect_contains(
        layout.shutdown_x,
        layout.shutdown_y,
        layout.shutdown_w,
        layout.shutdown_h,
        mx,
        my,
    );
    let arrow_hot = rect_contains(
        layout.shutdown_arrow_x,
        layout.shutdown_y,
        layout.shutdown_arrow_w,
        layout.shutdown_h,
        mx,
        my,
    );
    let shutdown_bg = if power_open || shutdown_hot {
        0x00_22_32_42
    } else {
        0x00_14_20_2C
    };
    s_fill(
        s,
        sw,
        layout.shutdown_x,
        layout.shutdown_y,
        layout.shutdown_w,
        layout.shutdown_h,
        shutdown_bg,
    );
    if arrow_hot || power_open {
        s_fill(
            s,
            sw,
            layout.shutdown_arrow_x,
            layout.shutdown_y,
            layout.shutdown_arrow_w,
            layout.shutdown_h,
            0x00_2B_44_5A,
        );
    }
    draw_rect_border(
        s,
        sw,
        layout.shutdown_x,
        layout.shutdown_y,
        layout.shutdown_w,
        layout.shutdown_h,
        frame,
    );
    s_fill(
        s,
        sw,
        layout.shutdown_arrow_x,
        layout.shutdown_y + 1,
        1,
        layout.shutdown_h - 2,
        frame,
    );
    s_draw_str_small(
        s,
        sw,
        layout.shutdown_x + 8,
        layout.shutdown_y + 7,
        "Shut down",
        WHITE,
        shutdown_bg,
        layout.shutdown_arrow_x - 4,
    );
    let caret = if arrow_hot || power_open {
        WHITE
    } else {
        muted
    };
    let caret_x = layout.shutdown_arrow_x + 8;
    let caret_y = layout.shutdown_y + 9;
    s_fill(s, sw, caret_x, caret_y, 5, 1, caret);
    s_fill(s, sw, caret_x + 1, caret_y + 1, 3, 1, caret);
    s_fill(s, sw, caret_x + 2, caret_y + 2, 1, 1, caret);

    if power_open {
        let (power_x, power_y, power_w, power_h) = start_power_menu_rect(
            layout.shutdown_x,
            layout.shutdown_y,
            layout.shutdown_w,
            layout.menu_x,
            layout.menu_y,
            layout.menu_w,
        );
        draw_start_power_menu(s, sw, power_x, power_y, power_w, power_h, mx, my);
    }
}

#[allow(dead_code)]
fn draw_start_menu_quick_actions(
    s: &mut [u32],
    sw: usize,
    banner_x: i32,
    banner_y: i32,
    banner_w: i32,
    rel_mx: i32,
    rel_my: i32,
) {
    for (idx, action) in start_menu_quick_actions().iter().enumerate() {
        let (x, y, w, h) = start_menu_quick_action_rect(idx, banner_w);
        let hot = rel_mx >= x && rel_mx < x + w && rel_my >= y && rel_my < y + h;
        let bg = if hot { 0x00_00_18_34 } else { 0x00_00_07_18 };
        let ax = banner_x + x;
        let ay = banner_y + y;
        s_fill(s, sw, ax, ay, w, h, bg);
        draw_rect_border(
            s,
            sw,
            ax,
            ay,
            w,
            h,
            if hot {
                action.accent
            } else {
                blend_color(action.accent, 0x00_00_07_18, 80)
            },
        );
        s_fill(s, sw, ax, ay, w, 2, action.accent);
        s_draw_str_small(
            s,
            sw,
            ax + 6,
            ay + 7,
            action.glyph,
            action.accent,
            bg,
            ax + 24,
        );
        s_draw_str_small(
            s,
            sw,
            ax + 28,
            ay + 7,
            action.label,
            if hot { WHITE } else { 0x00_AA_DD_FF },
            bg,
            ax + w - 6,
        );
    }
}

fn draw_start_power_menu(
    s: &mut [u32],
    sw: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    mx: i32,
    my: i32,
) {
    let bg = 0x00_00_07_18;
    s_fill(s, sw, x, y, w, h, bg);
    s_fill(s, sw, x, y, w, 3, 0x00_FF_DD_55);
    draw_glass_panel_outline(s, sw, x, y, w, h, 0x00_FF_DD_55);

    for (idx, action) in start_power_actions().iter().enumerate() {
        let row_y = y + START_POWER_MENU_PAD + idx as i32 * START_POWER_MENU_ROW_H;
        let hot =
            mx >= x + 3 && mx < x + w - 3 && my >= row_y && my < row_y + START_POWER_MENU_ROW_H;
        let row_bg = if hot { 0x00_00_18_34 } else { bg };
        if hot {
            s_fill(s, sw, x + 3, row_y, w - 6, START_POWER_MENU_ROW_H, row_bg);
            s_fill(
                s,
                sw,
                x + 4,
                row_y + 5,
                2,
                START_POWER_MENU_ROW_H - 10,
                action.accent,
            );
        }
        s_draw_str_small(
            s,
            sw,
            x + 12,
            row_y + 8,
            action.glyph,
            action.accent,
            row_bg,
            x + 30,
        );
        s_draw_str_small(
            s,
            sw,
            x + 36,
            row_y + 8,
            action.label,
            if hot { WHITE } else { 0x00_AA_DD_FF },
            row_bg,
            x + w - 10,
        );
    }
}

fn file_name(path: &str) -> String {
    String::from(path.rsplit('/').next().unwrap_or(path))
}

fn parse_i32_field(value: &str) -> Option<i32> {
    value.parse::<i32>().ok()
}

fn parse_usize_field(value: &str) -> Option<usize> {
    value.parse::<usize>().ok()
}

fn default_login_user_name() -> String {
    let current = crate::security::current_user();
    if current.login_enabled {
        return current.name;
    }
    for user in crate::security::users() {
        if user.login_enabled {
            return user.name;
        }
    }
    String::from("root")
}

fn push_i32_decimal(out: &mut String, n: i32) {
    if n < 0 {
        out.push('-');
        push_decimal(out, n.unsigned_abs() as u64);
    } else {
        push_decimal(out, n as u64);
    }
}

pub struct WindowManagerCell {
    inner: Mutex<Option<Box<WindowManager>>>,
}

pub struct WindowManagerGuard<'a>(MutexGuard<'a, Option<Box<WindowManager>>>);

impl WindowManagerCell {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn initialize(&self) {
        drop(self.lock());
    }

    pub fn lock(&self) -> WindowManagerGuard<'_> {
        let mut guard = self.inner.lock();
        if guard.is_none() {
            *guard = Some(WindowManager::new_boxed());
        }
        WindowManagerGuard(guard)
    }

    pub fn try_lock(&self) -> Option<WindowManagerGuard<'_>> {
        let mut guard = self.inner.try_lock()?;
        if guard.is_none() {
            *guard = Some(WindowManager::new_boxed());
        }
        Some(WindowManagerGuard(guard))
    }
}

impl Deref for WindowManagerGuard<'_> {
    type Target = WindowManager;

    fn deref(&self) -> &Self::Target {
        self.0.as_deref().expect("window manager initialized")
    }
}

impl DerefMut for WindowManagerGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_deref_mut().expect("window manager initialized")
    }
}

pub static WM: WindowManagerCell = WindowManagerCell::new();
