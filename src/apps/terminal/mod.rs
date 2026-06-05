/// Terminal app — renders a shell into a window's pixel back-buffer.
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::apps::theme;
use crate::keyboard::{Key, KeyInput};
use crate::wm::window::{Window, TITLE_H};

pub const TERM_W: i32 = 640;
pub const TERM_H: i32 = 440;

const CHAR_W: usize = 8;
const CHAR_H: usize = 8;
const LINE_H: usize = 12;
const GLYPH_Y_INSET: usize = 1;
const TERM_PAD_X: usize = 14;
const TERM_PAD_Y: usize = 10;

const TERM_BG_TOP: u32 = theme::BG_DEEP;
const TERM_BG_MID: u32 = theme::FIELD;
const TERM_BG_BOTTOM: u32 = theme::BG_BOTTOM;
const FG_OUTPUT: u32 = theme::TEXT;
const FG_PROMPT: u32 = theme::ACCENT_ALT;
const FG_INPUT: u32 = theme::TEXT;
const FG_ACCENT: u32 = theme::ACCENT;
const FG_DIM: u32 = theme::TEXT_MUTED;
const FG_ERROR: u32 = theme::DANGER;
const FG_DIR: u32 = theme::ACCENT_HOVER;
const FG_WARN: u32 = theme::WARNING;
const FG_CURSOR: u32 = theme::ACCENT;

const HISTORY_MAX: usize = 32;
const SCROLLBACK_MAX_LINES: usize = 2000;
const CURSOR_BLINK_MS: u64 = 500;

static DEBUG_MIRROR: AtomicBool = AtomicBool::new(false);

pub fn set_debug_mirror(enabled: bool) {
    DEBUG_MIRROR.store(enabled, Ordering::Release);
}

struct ForegroundJob {
    group: usize,
    pid: usize,
    job_id: Option<u64>,
    title: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnsiState {
    Ground,
    Escape,
    Csi,
}

#[derive(Clone, Copy)]
struct TerminalCell {
    ch: char,
    fg: u32,
}

pub struct TerminalApp {
    pub window: Window,
    tty_id: u64,
    cmd_buf: String,
    pending_key_sink_fd: Option<usize>,
    pending_browser_url: Option<String>,
    foreground_job: Option<ForegroundJob>,
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    screen: Vec<Vec<TerminalCell>>,
    scrollback: Vec<Vec<TerminalCell>>,
    scroll_top: usize,
    fg: u32,
    cwd: String,
    cmd_history: Vec<String>,
    history_pos: usize,
    saved_input: String,
    input_start_col: usize,
    last_width: i32,
    last_height: i32,
    ansi_state: AnsiState,
    ansi_params: [u16; 4],
    ansi_param_count: usize,
    ansi_param_value: u16,
    ansi_param_active: bool,
    ansi_private: bool,
    saved_col: usize,
    saved_row: usize,
    cursor_blink_on: bool,
    cursor_last_blink_tick: u64,
    cursor_painted_cell: Option<(usize, usize)>,
}

// Section files are included into this module so the split stays behavior-neutral.

include!("state.rs");
include!("commands.rs");
include!("render.rs");
include!("paths.rs");
