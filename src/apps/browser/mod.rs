extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

use super::filemanager::FileManagerOpenRequest;
use crate::apps::theme;
use crate::wm::window::{Window, TITLE_H};

pub const BROWSER_W: i32 = 760;
pub const BROWSER_H: i32 = 520;

const CHAR_W: usize = 8;
const LINE_H: usize = 14;
const TOOLBAR_H: usize = 54;
const STATUS_H: usize = 18;
const PAD_X: usize = 16;
const REFRESH_BUTTON_X: i32 = 82;
const REFRESH_BUTTON_W: i32 = 72;
const ADDRESS_X: i32 = 162;
const SEARCH_BUTTON_W: i32 = 72;
const BOOKMARKS_PATH: &str = "/CONFIG/BROWSER.CFG";
const DOWNLOADS_DIR: &str = "/Downloads";
const SESSION_INTERNAL_URL: &str = "browser://session";
const CACHE_INTERNAL_URL: &str = "browser://cache";
const JS_INTERNAL_URL: &str = "browser://js";
const STORAGE_INTERNAL_URL: &str = "browser://storage";
const COMPAT_INTERNAL_URL: &str = "browser://compat";
const ENGINE_INTERNAL_URL: &str = "browser://engine";
const MAX_BOOKMARKS: usize = 32;
const MAX_INLINE_PNG_PIXELS: usize = 1_048_576;
const MAX_HTML_INLINE_IMAGES: usize = 4;
const MAX_STYLESHEET_SUBRESOURCES: usize = 8;
const MAX_SCRIPT_SUBRESOURCES: usize = 8;
const MAX_SCRIPT_BYTES: usize = 64 * 1024;
const MAX_SCRIPT_STATEMENTS: usize = 192;
const MAX_SCRIPT_EVENT_HANDLERS: usize = 64;
const MAX_SCRIPT_RECURSION: usize = 4;
const MAX_SCRIPT_VARS: usize = 8;
const MAX_SCRIPT_FETCH_BYTES: usize = 64 * 1024;
const MAX_SESSION_STORAGE_ENTRIES: usize = 32;
const MAX_COMPAT_NOTES: usize = 8;
const MAX_BROWSER_CACHE_ENTRIES: usize = 16;
const MAX_BROWSER_RESOURCE_BYTES: usize = 512 * 1024;
const INLINE_IMAGE_MAX_H: usize = 168;
const INLINE_IMAGE_RESERVED_ROWS: usize = 14;
const CONTROL_H: usize = 24;
const CONTROL_GAP: usize = 10;
const BLOCK_GAP: usize = 6;
const MAX_DOM_NODES: usize = 768;
const MAX_DOM_ATTRS: usize = 8;
const MAX_FORM_CONTROLS: usize = 96;
const MAX_FORM_OPTIONS: usize = 16;
const MAX_FORM_VALUE: usize = 192;

const PAGE: u32 = 0x00_F2_F6_F8;
const TEXT: u32 = 0x00_12_1A_22;
const LINK: u32 = 0x00_0A_66_BB;
const MUTED: u32 = 0x00_5D_6B_78;
const BAR: u32 = theme::PANEL_ALT;
const BORDER: u32 = theme::BORDER;
const CHROME_TEXT: u32 = theme::TEXT;
const CHROME_MUTED: u32 = theme::TEXT_MUTED;
const ADDRESS_BG: u32 = theme::FIELD;
const ADDRESS_SELECTED: u32 = theme::SELECTION_GLOW;
const BUTTON_DIM: u32 = theme::CONTROL_DISABLED;
const BUTTON_HOT: u32 = theme::INPUT_FOCUS;
const WHITE: u32 = 0x00_FF_FF_FF;

// Section files are included into this module so the split stays behavior-neutral.

include!("types.rs");
include!("state.rs");
include!("layout.rs");
include!("pages.rs");
include!("images.rs");
include!("url.rs");
include!("css.rs");
include!("render.rs");
include!("script.rs");
include!("html.rs");
