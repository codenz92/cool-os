extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::apps::FileManagerOpenRequest;
use crate::framebuffer::{LIGHT_GRAY, WHITE, YELLOW};
use crate::wm::window::{Window, TITLE_H};

const UTILITY_W: i32 = 620;
const UTILITY_H: i32 = 420;
const CHAR_W: usize = 8;
const LINE_H: usize = 12;
const PAD_X: usize = 18;
const HEADER_H: usize = 88;
const FOOTER_H: usize = 20;
const TEXT_Y: usize = HEADER_H + 10;
const BUTTON_Y: i32 = 52;
const BUTTON_H: i32 = 22;
const TRASH_PATH: &str = "/Trash";
const DOCUMENTS_PATH: &str = "/Documents";
const PICTURES_PATH: &str = "/Pictures";
const NOTES_PATH: &str = "/Documents/NOTES.TXT";
const EDITOR_PATH: &str = "/Documents/UNTITLED.TXT";
const MAX_TEXT_BYTES: usize = 16 * 1024;

const BG_A: u32 = 0x00_03_07_14;
const BG_B: u32 = 0x00_01_03_0A;
const PANEL: u32 = 0x00_00_0A_1C;
const PANEL_ALT: u32 = 0x00_00_0E_24;
const PANEL_BORDER: u32 = 0x00_18_3C_62;
const ACCENT: u32 = 0x00_44_DD_FF;
const ACCENT_ALT: u32 = 0x00_88_FF_CC;
const MUTED: u32 = 0x00_6F_91_AE;
const DANGER: u32 = 0x00_FF_88_66;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UtilityMode {
    Trash,
    Screenshot,
    Notes,
    TextEditor,
}

struct TrashEntry {
    name: String,
    is_dir: bool,
    size: u32,
}

pub struct UtilityApp {
    pub window: Window,
    mode: UtilityMode,
    text: String,
    cursor: usize,
    scroll_line: usize,
    rows: usize,
    cols: usize,
    status: String,
    trash_entries: Vec<TrashEntry>,
    pending_open: Option<FileManagerOpenRequest>,
    last_width: i32,
    last_height: i32,
}

impl UtilityApp {
    pub fn trash_bin(x: i32, y: i32) -> Self {
        let mut app = Self::new(x, y, "Trash Bin", UtilityMode::Trash);
        app.ensure_trash_dir();
        app.refresh_trash_entries();
        app.status = String::from("Trash is ready");
        app.render();
        app
    }

    pub fn screenshot(x: i32, y: i32) -> Self {
        let mut app = Self::new(x, y, "Screenshot", UtilityMode::Screenshot);
        app.ensure_dir(PICTURES_PATH);
        app.status = String::from("Ready to capture the focused window");
        app.render();
        app
    }

    pub fn notes(x: i32, y: i32) -> Self {
        let mut app = Self::new(x, y, "Notes", UtilityMode::Notes);
        app.load_editor_file(NOTES_PATH);
        app.render();
        app
    }

    pub fn text_editor(x: i32, y: i32) -> Self {
        let mut app = Self::new(x, y, "Text Editor", UtilityMode::TextEditor);
        app.load_editor_file(EDITOR_PATH);
        app.render();
        app
    }

    fn new(x: i32, y: i32, title: &'static str, mode: UtilityMode) -> Self {
        UtilityApp {
            window: Window::new(x, y, UTILITY_W, UTILITY_H, title),
            mode,
            text: String::new(),
            cursor: 0,
            scroll_line: 0,
            rows: 0,
            cols: 0,
            status: String::new(),
            trash_entries: Vec::new(),
            pending_open: None,
            last_width: UTILITY_W,
            last_height: UTILITY_H,
        }
    }

    pub fn take_open_request(&mut self) -> Option<FileManagerOpenRequest> {
        self.pending_open.take()
    }

    pub fn handle_key(&mut self, c: char) {
        match self.mode {
            UtilityMode::Trash => match c {
                'e' | 'E' => self.empty_trash(),
                'r' | 'R' => self.refresh_trash(),
                'o' | 'O' | '\n' => {
                    self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(TRASH_PATH)));
                }
                _ => {}
            },
            UtilityMode::Screenshot => match c {
                's' | 'S' | '\n' => self.capture_screenshot(),
                'o' | 'O' => {
                    self.pending_open =
                        Some(FileManagerOpenRequest::Dir(String::from(PICTURES_PATH)));
                }
                _ => {}
            },
            UtilityMode::Notes | UtilityMode::TextEditor => self.handle_editor_key(c),
        }
        self.render();
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        match self.mode {
            UtilityMode::Trash => {
                if button_hit(lx, ly, 18, BUTTON_Y, 110, BUTTON_H) {
                    self.pending_open = Some(FileManagerOpenRequest::Dir(String::from(TRASH_PATH)));
                } else if button_hit(lx, ly, 138, BUTTON_Y, 112, BUTTON_H) {
                    self.empty_trash();
                } else if button_hit(lx, ly, 260, BUTTON_Y, 92, BUTTON_H) {
                    self.refresh_trash();
                }
            }
            UtilityMode::Screenshot => {
                if button_hit(lx, ly, 18, BUTTON_Y, 112, BUTTON_H) {
                    self.capture_screenshot();
                } else if button_hit(lx, ly, 140, BUTTON_Y, 128, BUTTON_H) {
                    self.pending_open =
                        Some(FileManagerOpenRequest::Dir(String::from(PICTURES_PATH)));
                }
            }
            UtilityMode::Notes | UtilityMode::TextEditor => {
                if button_hit(lx, ly, 18, BUTTON_Y, 80, BUTTON_H) {
                    self.save_editor();
                } else if button_hit(lx, ly, 108, BUTTON_Y, 118, BUTTON_H) {
                    self.pending_open =
                        Some(FileManagerOpenRequest::Dir(String::from(DOCUMENTS_PATH)));
                } else {
                    self.place_cursor_from_click(lx, ly);
                }
            }
        }
        self.render();
    }

    pub fn handle_scroll(&mut self, delta: i32) {
        let total = self.total_scroll_lines();
        let max = total.saturating_sub(self.rows.max(1));
        let new = (self.scroll_line as i32 + delta.signum() * 3).clamp(0, max as i32) as usize;
        if new != self.scroll_line {
            self.scroll_line = new;
            self.render();
        }
    }

    pub fn update(&mut self) {
        if self.window.width != self.last_width || self.window.height != self.last_height {
            self.last_width = self.window.width;
            self.last_height = self.window.height;
            self.render();
            return;
        }

        let expected = self.scroll_line as i32 * LINE_H as i32;
        if self.window.scroll.offset != expected {
            let total = self.total_scroll_lines();
            let max = total.saturating_sub(self.rows.max(1));
            self.scroll_line = ((self.window.scroll.offset / LINE_H as i32) as usize).min(max);
            self.render();
        }
    }

    fn render(&mut self) {
        match self.mode {
            UtilityMode::Trash => self.render_trash(),
            UtilityMode::Screenshot => self.render_screenshot(),
            UtilityMode::Notes | UtilityMode::TextEditor => self.render_editor(),
        }
        self.window.mark_dirty_all();
    }

    fn render_trash(&mut self) {
        let (width, content_h, stride) = self.begin_render();
        self.draw_header("Trash Bin", "deleted items staged for permanent removal");
        self.draw_button(18, BUTTON_Y, 110, BUTTON_H, "Open Trash", ACCENT);
        self.draw_button(138, BUTTON_Y, 112, BUTTON_H, "Empty Trash", DANGER);
        self.draw_button(260, BUTTON_Y, 92, BUTTON_H, "Refresh", ACCENT);

        let list_h = content_h.saturating_sub(TEXT_Y + FOOTER_H + 8);
        self.rows = list_h / LINE_H;
        self.cols = width.saturating_sub(PAD_X * 2 + 16) / CHAR_W;
        self.window.scroll.content_h = (self.trash_entries.len().max(1) * LINE_H) as i32;
        self.window.scroll.offset = self.scroll_line as i32 * LINE_H as i32;
        self.window.scroll.clamp((self.rows * LINE_H) as i32);

        let mut total_size = 0u64;
        for entry in self.trash_entries.iter() {
            total_size += entry.size as u64;
        }
        let mut summary = String::new();
        push_number(&mut summary, self.trash_entries.len());
        summary.push_str(" items, ");
        push_number(&mut summary, total_size as usize);
        summary.push_str(" bytes");
        self.put_str(stride, PAD_X, 28, &summary, LIGHT_GRAY);

        if self.trash_entries.is_empty() {
            self.put_str(stride, PAD_X, TEXT_Y, "Trash is empty", MUTED);
        } else {
            for row in 0..self.rows {
                let idx = self.scroll_line + row;
                if idx >= self.trash_entries.len() {
                    break;
                }
                let entry = &self.trash_entries[idx];
                let mut line = if entry.is_dir {
                    String::from("[DIR] ")
                } else {
                    String::from("      ")
                };
                line.push_str(&entry.name);
                if !entry.is_dir {
                    line.push_str("  ");
                    push_number(&mut line, entry.size as usize);
                    line.push_str(" B");
                }
                let y = TEXT_Y + row * LINE_H;
                if row % 2 == 0 {
                    self.fill_rect(stride, PAD_X - 6, y - 1, width - PAD_X * 2, LINE_H, PANEL);
                }
                self.put_truncated(stride, PAD_X, y, &line, WHITE);
            }
        }

        self.draw_footer("Enter/o open   e empty   r refresh");
    }

    fn render_screenshot(&mut self) {
        let (width, content_h, stride) = self.begin_render();
        self.draw_header("Screenshot", "capture the focused window as a PPM image");
        self.draw_button(18, BUTTON_Y, 112, BUTTON_H, "Capture", ACCENT_ALT);
        self.draw_button(140, BUTTON_Y, 128, BUTTON_H, "Open Pictures", ACCENT);
        self.rows = 1;
        self.cols = width.saturating_sub(PAD_X * 2 + 16) / CHAR_W;
        self.window.scroll.content_h = 0;
        self.window.scroll.offset = 0;

        let panel_y = TEXT_Y;
        let panel_h = content_h.saturating_sub(TEXT_Y + FOOTER_H + 18).min(160);
        self.fill_rect(
            stride,
            PAD_X - 6,
            panel_y - 6,
            width - PAD_X * 2,
            panel_h,
            PANEL,
        );
        self.draw_rect(
            stride,
            PAD_X - 6,
            panel_y - 6,
            width - PAD_X * 2,
            panel_h,
            PANEL_BORDER,
        );
        self.put_str(stride, PAD_X, panel_y + 8, "Target folder", MUTED);
        self.put_str(stride, PAD_X, panel_y + 22, PICTURES_PATH, WHITE);
        self.put_str(stride, PAD_X, panel_y + 48, "Next capture name", MUTED);
        let next = next_screenshot_path();
        self.put_str(stride, PAD_X, panel_y + 62, &next, LIGHT_GRAY);
        self.put_str(stride, PAD_X, panel_y + 94, &self.status.clone(), YELLOW);
        self.draw_footer("Enter/s capture   o open Pictures");
    }

    fn render_editor(&mut self) {
        let (width, content_h, stride) = self.begin_render();
        let title = if self.mode == UtilityMode::Notes {
            "Notes"
        } else {
            "Text Editor"
        };
        let path = self.editor_path();
        self.draw_header(title, path);
        self.draw_button(18, BUTTON_Y, 80, BUTTON_H, "Save", ACCENT_ALT);
        self.draw_button(108, BUTTON_Y, 118, BUTTON_H, "Open Folder", ACCENT);

        let text_h = content_h.saturating_sub(TEXT_Y + FOOTER_H + 8);
        self.rows = text_h / LINE_H;
        self.cols = width.saturating_sub(PAD_X * 2 + 16) / CHAR_W;
        let lines = self.visual_lines();
        self.ensure_cursor_visible(&lines);
        self.window.scroll.content_h = (lines.len().max(1) * LINE_H) as i32;
        self.window.scroll.offset = self.scroll_line as i32 * LINE_H as i32;
        self.window.scroll.clamp((self.rows * LINE_H) as i32);

        self.fill_rect(
            stride,
            PAD_X - 8,
            TEXT_Y - 8,
            width.saturating_sub(PAD_X * 2),
            text_h + 12,
            PANEL,
        );
        self.draw_rect(
            stride,
            PAD_X - 8,
            TEXT_Y - 8,
            width.saturating_sub(PAD_X * 2),
            text_h + 12,
            PANEL_BORDER,
        );

        if self.text.is_empty() {
            let placeholder = if self.mode == UtilityMode::Notes {
                "Type a note..."
            } else {
                "Start typing..."
            };
            self.put_str(stride, PAD_X, TEXT_Y, placeholder, MUTED);
        }

        for row in 0..self.rows {
            let line_idx = self.scroll_line + row;
            if line_idx >= lines.len() {
                break;
            }
            let (start, end) = lines[line_idx];
            if start <= end && end <= self.text.len() {
                let line = String::from(&self.text[start..end]);
                self.put_truncated(stride, PAD_X, TEXT_Y + row * LINE_H, &line, WHITE);
            }
        }

        if let Some((cursor_row, cursor_col)) = cursor_visual_position(&lines, self.cursor) {
            if cursor_row >= self.scroll_line && cursor_row < self.scroll_line + self.rows {
                let local_row = cursor_row - self.scroll_line;
                let x = PAD_X + cursor_col.min(self.cols) * CHAR_W;
                let y = TEXT_Y + local_row * LINE_H;
                self.fill_rect(stride, x, y, 2, LINE_H.saturating_sub(2), ACCENT_ALT);
            }
        }

        let mut footer = self.status.clone();
        footer.push_str("   ");
        push_number(&mut footer, self.text.len());
        footer.push_str(" bytes");
        self.draw_footer(&footer);
    }

    fn begin_render(&mut self) -> (usize, usize, usize) {
        let width = self.window.width.max(1) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        let stride = width;
        for (idx, pixel) in self.window.buf.iter_mut().enumerate() {
            let y = idx / stride;
            *pixel = if y % 10 < 5 { BG_A } else { BG_B };
        }
        self.fill_rect(stride, 0, 0, width, HEADER_H, PANEL_ALT);
        self.fill_rect(stride, 0, HEADER_H - 1, width, 1, PANEL_BORDER);
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(FOOTER_H),
            width,
            FOOTER_H,
            PANEL,
        );
        self.fill_rect(
            stride,
            0,
            content_h.saturating_sub(FOOTER_H),
            width,
            1,
            PANEL_BORDER,
        );
        (width, content_h, stride)
    }

    fn draw_header(&mut self, title: &str, subtitle: &str) {
        let stride = self.window.width.max(1) as usize;
        self.fill_rect(stride, PAD_X - 10, 12, 3, 22, ACCENT);
        self.put_str(stride, PAD_X, 12, title, WHITE);
        self.put_str(stride, PAD_X, 28, subtitle, MUTED);
    }

    fn draw_footer(&mut self, text: &str) {
        let stride = self.window.width.max(1) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        self.put_truncated(
            stride,
            PAD_X,
            content_h.saturating_sub(FOOTER_H).saturating_add(5),
            text,
            MUTED,
        );
    }

    fn draw_button(&mut self, x: i32, y: i32, w: i32, h: i32, label: &str, accent: u32) {
        let stride = self.window.width.max(1) as usize;
        self.fill_rect(
            stride,
            x as usize,
            y as usize,
            w as usize,
            h as usize,
            0x00_05_12_24,
        );
        self.draw_rect(
            stride, x as usize, y as usize, w as usize, h as usize, accent,
        );
        self.put_str(stride, x as usize + 9, y as usize + 7, label, WHITE);
    }

    fn fill_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let width = self.window.width.max(0) as usize;
        let content_h = (self.window.height - TITLE_H).max(0) as usize;
        for row in y..(y + h).min(content_h) {
            let base = row * stride;
            for col in x..(x + w).min(width) {
                let idx = base + col;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = color;
                }
            }
        }
    }

    fn draw_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w < 2 || h < 2 {
            return;
        }
        self.fill_rect(stride, x, y, w, 1, color);
        self.fill_rect(stride, x, y + h - 1, w, 1, color);
        self.fill_rect(stride, x, y, 1, h, color);
        self.fill_rect(stride, x + w - 1, y, 1, h, color);
    }

    fn put_str(&mut self, stride: usize, px: usize, py: usize, s: &str, color: u32) {
        crate::font::draw_str(
            &mut self.window.buf,
            stride,
            px,
            py,
            s,
            color,
            None,
            crate::font::UI_FONT,
        );
    }

    fn put_truncated(&mut self, stride: usize, px: usize, py: usize, s: &str, color: u32) {
        let max_cols = self
            .window
            .width
            .max(0)
            .saturating_sub(px as i32 + PAD_X as i32) as usize
            / CHAR_W;
        let mut out = String::new();
        for (idx, ch) in s.chars().enumerate() {
            if idx >= max_cols {
                break;
            }
            out.push(ch);
        }
        self.put_str(stride, px, py, &out, color);
    }

    fn ensure_dir(&mut self, path: &str) {
        match crate::vfs::vfs_create_dir(path) {
            Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
            Err(err) => {
                self.status = String::from(err.as_str());
            }
        }
    }

    fn ensure_trash_dir(&mut self) {
        self.ensure_dir(TRASH_PATH);
    }

    fn refresh_trash(&mut self) {
        self.ensure_trash_dir();
        self.refresh_trash_entries();
        self.status = String::from("Trash refreshed");
    }

    fn refresh_trash_entries(&mut self) {
        self.trash_entries.clear();
        if let Some(entries) = crate::vfs::vfs_list_dir(TRASH_PATH) {
            for entry in entries {
                self.trash_entries.push(TrashEntry {
                    name: entry.name,
                    is_dir: entry.is_dir,
                    size: entry.size,
                });
            }
        }
        let max = self.trash_entries.len().saturating_sub(self.rows.max(1));
        self.scroll_line = self.scroll_line.min(max);
    }

    fn empty_trash(&mut self) {
        self.ensure_trash_dir();
        let entries = crate::vfs::vfs_list_dir(TRASH_PATH).unwrap_or_default();
        let mut removed = 0usize;
        let mut last_err = None;
        for entry in entries {
            let path = join_path(TRASH_PATH, &entry.name);
            match delete_path_recursive(&path, entry.is_dir) {
                Ok(()) => removed += 1,
                Err(err) => last_err = Some(String::from(err.as_str())),
            }
        }
        self.refresh_trash_entries();
        if let Some(err) = last_err {
            self.status = err;
        } else {
            self.status = format!("removed {} items", removed);
        }
    }

    fn capture_screenshot(&mut self) {
        self.ensure_dir(PICTURES_PATH);
        let path = next_screenshot_path();
        crate::wm::request_focused_screenshot(&path);
        self.status = format!("queued {}", path);
        self.window.minimized = true;
    }

    fn load_editor_file(&mut self, path: &str) {
        self.ensure_dir(DOCUMENTS_PATH);
        match crate::vfs::vfs_read_file(path) {
            Some(bytes) => match core::str::from_utf8(&bytes) {
                Ok(text) => {
                    self.text = String::from(text);
                    self.cursor = self.text.len();
                    self.status = String::from("loaded");
                }
                Err(_) => {
                    self.text.clear();
                    self.cursor = 0;
                    self.status = String::from("file is not UTF-8 text");
                }
            },
            None => {
                self.text.clear();
                self.cursor = 0;
                self.save_editor();
            }
        }
    }

    fn save_editor(&mut self) {
        self.ensure_dir(DOCUMENTS_PATH);
        match crate::vfs::vfs_safe_write_file(self.editor_path(), self.text.as_bytes()) {
            Ok(()) => self.status = String::from("saved"),
            Err(err) => self.status = String::from(err.as_str()),
        }
    }

    fn editor_path(&self) -> &'static str {
        if self.mode == UtilityMode::Notes {
            NOTES_PATH
        } else {
            EDITOR_PATH
        }
    }

    fn handle_editor_key(&mut self, c: char) {
        let mut changed = false;
        match c {
            '\u{0008}' => {
                if let Some(prev) = prev_char_boundary(&self.text, self.cursor) {
                    self.text.drain(prev..self.cursor);
                    self.cursor = prev;
                    changed = true;
                }
            }
            '\u{007F}' => {
                if self.cursor < self.text.len() {
                    let next = next_char_boundary(&self.text, self.cursor);
                    self.text.drain(self.cursor..next);
                    changed = true;
                }
            }
            '\n' => {
                changed = self.insert_text("\n");
            }
            '\t' => {
                changed = self.insert_text("    ");
            }
            '\u{F702}' => {
                if let Some(prev) = prev_char_boundary(&self.text, self.cursor) {
                    self.cursor = prev;
                }
            }
            '\u{F703}' => {
                self.cursor = next_char_boundary(&self.text, self.cursor);
            }
            '\u{F700}' => self.move_cursor_vertical(-1),
            '\u{F701}' => self.move_cursor_vertical(1),
            '\u{F704}' => self.move_cursor_to_line_edge(false),
            '\u{F705}' => self.move_cursor_to_line_edge(true),
            _ if c >= ' ' && (c as u32) < 0xF700 => {
                let mut buf = [0u8; 4];
                changed = self.insert_text(c.encode_utf8(&mut buf));
            }
            _ => {}
        }
        if changed {
            self.save_editor();
        }
        let lines = self.visual_lines();
        self.ensure_cursor_visible(&lines);
    }

    fn insert_text(&mut self, input: &str) -> bool {
        if self.text.len().saturating_add(input.len()) > MAX_TEXT_BYTES {
            self.status = String::from("document limit reached");
            return false;
        }
        self.cursor = clamp_to_char_boundary(&self.text, self.cursor);
        self.text.insert_str(self.cursor, input);
        self.cursor += input.len();
        true
    }

    fn visual_lines(&self) -> Vec<(usize, usize)> {
        visual_lines_for(&self.text, self.cols.max(1))
    }

    fn ensure_cursor_visible(&mut self, lines: &[(usize, usize)]) {
        let Some((row, _)) = cursor_visual_position(lines, self.cursor) else {
            return;
        };
        if row < self.scroll_line {
            self.scroll_line = row;
        } else if row >= self.scroll_line + self.rows.max(1) {
            self.scroll_line = row.saturating_sub(self.rows.max(1).saturating_sub(1));
        }
    }

    fn move_cursor_vertical(&mut self, delta: i32) {
        let lines = self.visual_lines();
        let Some((row, col)) = cursor_visual_position(&lines, self.cursor) else {
            return;
        };
        let next_row = (row as i32 + delta).clamp(0, lines.len().saturating_sub(1) as i32) as usize;
        let Some(&(start, end)) = lines.get(next_row) else {
            return;
        };
        self.cursor = byte_at_col(&self.text, start, end, col);
    }

    fn move_cursor_to_line_edge(&mut self, end_edge: bool) {
        let lines = self.visual_lines();
        let Some((row, _)) = cursor_visual_position(&lines, self.cursor) else {
            return;
        };
        if let Some(&(start, end)) = lines.get(row) {
            self.cursor = if end_edge { end } else { start };
        }
    }

    fn place_cursor_from_click(&mut self, lx: i32, ly: i32) {
        if ly < TEXT_Y as i32 || lx < PAD_X as i32 {
            return;
        }
        let row = self.scroll_line + ((ly as usize).saturating_sub(TEXT_Y) / LINE_H);
        let col = ((lx as usize).saturating_sub(PAD_X) / CHAR_W).min(self.cols);
        let lines = self.visual_lines();
        if let Some(&(start, end)) = lines.get(row) {
            self.cursor = byte_at_col(&self.text, start, end, col);
        }
    }

    fn total_scroll_lines(&self) -> usize {
        match self.mode {
            UtilityMode::Trash => self.trash_entries.len().max(1),
            UtilityMode::Screenshot => 1,
            UtilityMode::Notes | UtilityMode::TextEditor => self.visual_lines().len().max(1),
        }
    }
}

fn button_hit(lx: i32, ly: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    lx >= x && lx < x + w && ly >= y && ly < y + h
}

fn join_path(parent: &str, name: &str) -> String {
    let mut path = String::from(parent);
    if !path.ends_with('/') {
        path.push('/');
    }
    path.push_str(name);
    path
}

fn delete_path_recursive(path: &str, is_dir: bool) -> Result<(), crate::fat32::FsError> {
    if path == "/" || path.eq_ignore_ascii_case(TRASH_PATH) {
        return Err(crate::fat32::FsError::InvalidPath);
    }
    if crate::security::is_protected_path(path) {
        return Err(crate::fat32::FsError::PermissionDenied);
    }
    if is_dir {
        let children = crate::vfs::vfs_list_dir(path).ok_or(crate::fat32::FsError::NotFound)?;
        for child in children {
            let child_path = join_path(path, &child.name);
            delete_path_recursive(&child_path, child.is_dir)?;
        }
    }
    crate::vfs::vfs_delete(path)
}

fn next_screenshot_path() -> String {
    let base = (crate::interrupts::ticks() as usize) % 10_000;
    for offset in 0..10_000usize {
        let n = (base + offset) % 10_000;
        let mut path = String::from(PICTURES_PATH);
        path.push_str("/SHOT");
        push_fixed4(&mut path, n);
        path.push_str(".PPM");
        if crate::vfs::vfs_read_file(&path).is_none() {
            return path;
        }
    }
    String::from("/Pictures/SHOT0000.PPM")
}

fn visual_lines_for(text: &str, cols: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut col = 0usize;
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            lines.push((start, idx));
            start = idx + ch.len_utf8();
            col = 0;
            continue;
        }
        if col >= cols {
            lines.push((start, idx));
            start = idx;
            col = 0;
        }
        col += 1;
    }
    lines.push((start, text.len()));
    lines
}

fn cursor_visual_position(lines: &[(usize, usize)], cursor: usize) -> Option<(usize, usize)> {
    for (row, &(start, end)) in lines.iter().enumerate() {
        if cursor >= start && cursor <= end {
            return Some((row, text_col_between(lines, row, start, cursor)));
        }
    }
    lines.len().checked_sub(1).map(|row| {
        (
            row,
            text_col_between(lines, row, lines[row].0, lines[row].1),
        )
    })
}

fn text_col_between(lines: &[(usize, usize)], row: usize, start: usize, cursor: usize) -> usize {
    let Some(&(line_start, line_end)) = lines.get(row) else {
        return 0;
    };
    let cursor = cursor.clamp(line_start, line_end);
    if cursor < start {
        return 0;
    }
    cursor
        .saturating_sub(start)
        .min(line_end.saturating_sub(line_start))
}

fn byte_at_col(text: &str, start: usize, end: usize, target_col: usize) -> usize {
    let mut col = 0usize;
    for (idx, _) in text[start..end].char_indices() {
        if col >= target_col {
            return start + idx;
        }
        col += 1;
    }
    end
}

fn prev_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    if cursor == 0 {
        return None;
    }
    let cursor = clamp_to_char_boundary(text, cursor.min(text.len()));
    text[..cursor].char_indices().last().map(|(idx, _)| idx)
}

fn next_char_boundary(text: &str, cursor: usize) -> usize {
    let cursor = clamp_to_char_boundary(text, cursor.min(text.len()));
    if cursor >= text.len() {
        return text.len();
    }
    text[cursor..]
        .chars()
        .next()
        .map(|ch| cursor + ch.len_utf8())
        .unwrap_or(text.len())
}

fn clamp_to_char_boundary(text: &str, mut idx: usize) -> usize {
    idx = idx.min(text.len());
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn push_number(out: &mut String, mut n: usize) {
    if n == 0 {
        out.push('0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        out.push(digits[i] as char);
    }
}

fn push_fixed4(out: &mut String, n: usize) {
    out.push((b'0' + ((n / 1000) % 10) as u8) as char);
    out.push((b'0' + ((n / 100) % 10) as u8) as char);
    out.push((b'0' + ((n / 10) % 10) as u8) as char);
    out.push((b'0' + (n % 10) as u8) as char);
}
