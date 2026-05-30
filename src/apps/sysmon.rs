extern crate alloc;

use alloc::string::String;

use crate::apps::theme;
use crate::wm::window::Window;

pub const SYSMON_W: i32 = 520;
pub const SYSMON_H: i32 = 340;

const CHAR_W_SMALL: usize = 8;
const CHAR_H_SMALL: usize = 8;

const BG: u32 = theme::BG_TOP;
const BG_ALT: u32 = theme::BG_BOTTOM;
const PANEL_BG: u32 = theme::CARD_SURFACE;
const PANEL_ALT: u32 = theme::CONTROL_FILL;
const PANEL_BORDER: u32 = theme::BORDER;
const PANEL_INNER: u32 = theme::DIVIDER;
const LABEL: u32 = theme::TEXT;
const MUTED: u32 = theme::TEXT_MUTED;
const TEXT: u32 = theme::TEXT;
const BAR_TRACK: u32 = theme::FIELD;
const USB_GOOD: u32 = theme::STATUS_SUCCESS;
const USB_WARN: u32 = theme::STATUS_WARNING;
const METRIC_INFO: u32 = theme::STATUS_INFO;
const METRIC_GOOD: u32 = theme::STATUS_SUCCESS;
const METRIC_WARN: u32 = theme::STATUS_WARNING;
const METRIC_DANGER: u32 = theme::STATUS_DANGER;
const DISABLED_DOT: u32 = theme::CONTROL_DISABLED;

#[derive(Clone)]
pub enum SysMonRequest {
    ClosePid(usize),
    KillPid(usize),
    OpenPath(String),
}

pub struct SysMonApp {
    pub window: Window,
    last_redraw_tick: u64,
    last_width: i32,
    last_height: i32,
    selected_app: usize,
    status_note: &'static str,
    pending_request: Option<SysMonRequest>,
}

impl SysMonApp {
    pub fn new(x: i32, y: i32) -> Self {
        let mut app = SysMonApp {
            window: Window::new(x, y, SYSMON_W, SYSMON_H, "System Monitor"),
            last_redraw_tick: 0,
            last_width: SYSMON_W,
            last_height: SYSMON_H,
            selected_app: 0,
            status_note: "ready",
            pending_request: None,
        };
        app.update();
        app
    }

    pub fn take_request(&mut self) -> Option<SysMonRequest> {
        self.pending_request.take()
    }

    pub fn handle_key(&mut self, c: char) {
        match c {
            'j' | 'J' => self.select_next_app(),
            'k' | 'K' => self.select_prev_app(),
            'c' | 'C' => self.request_close_selected(),
            'x' | 'X' => self.request_kill_selected(),
            'p' | 'P' => self.request_path_selected(),
            _ => return,
        }
        self.force_redraw();
    }

    pub fn handle_click(&mut self, lx: i32, ly: i32) {
        if lx >= 280 && lx < 492 && ly >= 220 && ly < 246 {
            let row = ((ly - 220) / 12).max(0) as usize;
            let running = crate::app_lifecycle::running_apps();
            if row < running.len() {
                self.selected_app = row;
                self.status_note = "selected app";
                self.force_redraw();
                return;
            }
        }
        if hit(lx, ly, 280, 258, 52, 15) {
            self.request_close_selected();
        } else if hit(lx, ly, 340, 258, 44, 15) {
            self.request_kill_selected();
        } else if hit(lx, ly, 392, 258, 48, 15) {
            self.request_path_selected();
        } else {
            return;
        }
        self.force_redraw();
    }

    pub fn update(&mut self) {
        let ticks = crate::interrupts::ticks();
        let resized =
            self.window.width != self.last_width || self.window.height != self.last_height;
        let redraw_interval = (crate::interrupts::TIMER_HZ / 12).max(1) as u64;
        if !resized && ticks.wrapping_sub(self.last_redraw_tick) < redraw_interval {
            return;
        }
        self.last_redraw_tick = ticks;
        self.last_width = self.window.width;
        self.last_height = self.window.height;

        let stride = self.window.width as usize;
        self.fill_background(stride);

        let cpuid = raw_cpuid::CpuId::new();
        let vendor_info = cpuid.get_vendor_info();
        let vendor = vendor_info
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        let heap = crate::allocator::heap_snapshot();
        let pressure = crate::memory_pressure::snapshot();
        let used = heap.used;
        let heap_total = heap.total;
        let heap_free = heap.free;
        let heap_ratio = if heap_total > 0 {
            (used.saturating_mul(100) / heap_total).min(100)
        } else {
            0
        };

        let secs = crate::interrupts::uptime_secs();
        let hrs = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let secs_only = secs % 60;

        let counter =
            crate::scheduler::BACKGROUND_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
        let (task_count, ready_count, blocked_count, exited_count, current_pid) = {
            let sched = crate::scheduler::SCHEDULER.lock();
            let mut ready = 0usize;
            let mut blocked = 0usize;
            let mut exited = 0usize;
            for task in sched.tasks.iter() {
                match task.status {
                    crate::scheduler::TaskStatus::Ready => ready += 1,
                    crate::scheduler::TaskStatus::Running => {}
                    crate::scheduler::TaskStatus::Blocked => blocked += 1,
                    crate::scheduler::TaskStatus::Stopped => blocked += 1,
                    crate::scheduler::TaskStatus::Exited => exited += 1,
                    crate::scheduler::TaskStatus::Reaped => {}
                }
            }
            (sched.tasks.len(), ready, blocked, exited, sched.current)
        };
        let resource_stats = crate::scheduler::resource_stats();
        let service_health = crate::services::health();
        let usb_lines = crate::usb::status_lines();
        let (usb_keyboard, usb_mouse) = crate::usb::input_presence();
        let usb_present = !usb_lines.is_empty();
        let usb_active = usb_lines
            .iter()
            .any(|line| line.contains("active init ready"));
        let compositor = crate::wm::compositor::compositor_stats();
        let running_apps = crate::app_lifecycle::running_apps();
        let finished_apps = crate::app_lifecycle::finished_apps();
        if self.selected_app >= running_apps.len() {
            self.selected_app = running_apps.len().saturating_sub(1);
        }

        self.put_str_centered_px(stride, 14, "SYSTEM DASHBOARD", LABEL);
        self.put_str_centered_px(
            stride,
            26,
            "runtime view for scheduler, memory, USB, and apps",
            MUTED,
        );

        let card_w = 236usize;
        let card_h = 58usize;
        self.draw_card_frame(stride, 16, 44, card_w, card_h, METRIC_INFO);
        self.draw_card_frame(stride, 268, 44, card_w, card_h, METRIC_WARN);
        self.draw_card_frame(stride, 16, 114, card_w, card_h, METRIC_GOOD);
        self.draw_card_frame(stride, 268, 114, card_w, card_h, METRIC_INFO);
        self.draw_card_frame(stride, 16, 184, card_w, 98, theme::ACCENT_ALT);
        self.draw_card_frame(stride, 268, 184, card_w, 98, METRIC_DANGER);

        self.put_str_px(stride, 28, 58, "CPU VENDOR", LABEL);
        self.put_str_px(stride, 28, 76, vendor, METRIC_GOOD);

        self.put_str_px(stride, 280, 58, "HEAP USED", LABEL);
        let mut heap_used_line = NumberLine::new();
        heap_used_line.push_size(used);
        heap_used_line.push_str(" of ");
        heap_used_line.push_size(heap_total);
        self.put_str_px(stride, 280, 72, heap_used_line.as_str(), METRIC_WARN);
        let mut heap_free_line = NumberLine::new();
        heap_free_line.push_size(heap_free);
        heap_free_line.push_str(" free   ");
        heap_free_line.push_usize(heap_ratio);
        heap_free_line.push_str("%");
        self.put_str_px(stride, 280, 82, heap_free_line.as_str(), MUTED);
        let mut pressure_line = NumberLine::new();
        pressure_line.push_str("pressure ");
        pressure_line.push_str(pressure.level.as_str());
        pressure_line.push_str(" oom ");
        pressure_line.push_usize(pressure.oom_kills);
        self.put_str_px(stride, 280, 92, pressure_line.as_str(), MUTED);
        self.draw_bar(stride, 280, 98, 200, 6, heap_ratio, BAR_TRACK, METRIC_WARN);

        self.put_str_px(stride, 28, 128, "UPTIME", LABEL);
        let time = [
            b'0' + (hrs / 10) as u8,
            b'0' + (hrs % 10) as u8,
            b':',
            b'0' + (mins / 10) as u8,
            b'0' + (mins % 10) as u8,
            b':',
            b'0' + (secs_only / 10) as u8,
            b'0' + (secs_only % 10) as u8,
        ];
        if let Ok(time_str) = core::str::from_utf8(&time) {
            self.put_str_px(stride, 28, 146, time_str, METRIC_INFO);
        }
        let mut tick_line = NumberLine::new();
        tick_line.push_u64(ticks);
        tick_line.push_str(" ticks");
        self.put_str_px(stride, 116, 146, tick_line.as_str(), MUTED);
        let mut service_line = NumberLine::new();
        service_line.push_str("services ");
        service_line.push_usize(service_health.running);
        service_line.push_str("/");
        service_line.push_usize(service_health.total);
        service_line.push_str(" degraded ");
        service_line.push_usize(service_health.degraded);
        self.put_str_px(
            stride,
            28,
            158,
            service_line.as_str(),
            if service_health.degraded == 0 {
                METRIC_GOOD
            } else {
                METRIC_DANGER
            },
        );

        self.put_str_px(stride, 280, 128, "SCHEDULER", LABEL);
        let mut task_line = NumberLine::new();
        task_line.push_str("pid ");
        task_line.push_usize(current_pid);
        task_line.push_str("  tasks ");
        task_line.push_usize(resource_stats.active_tasks);
        task_line.push_str("/");
        task_line.push_usize(resource_stats.max_active_tasks);
        self.put_str_px(stride, 280, 144, task_line.as_str(), METRIC_INFO);
        let mut state_line = NumberLine::new();
        state_line.push_str("slots ");
        state_line.push_usize(task_count);
        state_line.push_str(" ");
        state_line.push_str("r ");
        state_line.push_usize(ready_count);
        state_line.push_str(" b ");
        state_line.push_usize(blocked_count);
        state_line.push_str(" e ");
        state_line.push_usize(exited_count);
        self.put_str_px(stride, 280, 154, state_line.as_str(), MUTED);
        let pulse = ((counter as usize / 64) % 100).max(8);
        self.draw_bar(stride, 280, 166, 200, 6, pulse, BAR_TRACK, METRIC_INFO);

        self.put_str_px(stride, 28, 198, "USB + COMPOSITOR", LABEL);
        self.draw_status_pill(
            stride,
            28,
            214,
            "CTRL",
            usb_present,
            if usb_present { METRIC_INFO } else { MUTED },
        );
        self.draw_status_pill(
            stride,
            90,
            214,
            "ACTIVE",
            usb_active,
            if usb_active { USB_GOOD } else { USB_WARN },
        );
        self.draw_status_pill(
            stride,
            164,
            214,
            "KBD",
            usb_keyboard,
            if usb_keyboard { USB_GOOD } else { MUTED },
        );
        self.draw_status_pill(
            stride,
            28,
            232,
            "MOUSE",
            usb_mouse,
            if usb_mouse { USB_GOOD } else { MUTED },
        );

        let mut comp_line = NumberLine::new();
        comp_line.push_str("fps ");
        comp_line.push_u64(compositor.fps);
        comp_line.push_str("  frame ");
        comp_line.push_u64(compositor.frame_ticks_last);
        comp_line.push_str("t");
        self.put_str_px(stride, 90, 235, comp_line.as_str(), METRIC_INFO);

        let mut row = 250usize;
        if usb_lines.is_empty() {
            self.put_str_px(stride, 28, row, "USB: no probe data", MUTED);
        } else {
            for line in usb_lines {
                if row + CHAR_H_SMALL > 278 {
                    break;
                }
                self.put_str_px_max(stride, 28, row, &line, METRIC_INFO, 252);
                row += 10;
            }
        }

        self.put_str_px(stride, 280, 198, "APP LIFECYCLE", LABEL);
        let mut app_summary = NumberLine::new();
        app_summary.push_str("running ");
        app_summary.push_usize(running_apps.len());
        app_summary.push_str("  finished ");
        app_summary.push_usize(finished_apps.len());
        self.put_str_px(stride, 280, 208, app_summary.as_str(), MUTED);

        if running_apps.is_empty() {
            self.put_str_px(stride, 280, 224, "no userspace apps", MUTED);
        } else {
            for (idx, app) in running_apps.iter().take(2).enumerate() {
                let y = 222 + idx * 12;
                let selected = idx == self.selected_app;
                if selected {
                    self.fill_rect(stride, 278, y.saturating_sub(1), 214, 11, PANEL_INNER);
                }
                let mut app_line = NumberLine::new();
                if selected {
                    app_line.push_str("> ");
                } else {
                    app_line.push_str("  ");
                }
                app_line.push_str("pid ");
                app_line.push_usize(app.pid);
                app_line.push_str(" ");
                app_line.push_str(&app.app);
                self.put_str_px(
                    stride,
                    280,
                    y,
                    app_line.as_str(),
                    if selected { TEXT } else { METRIC_INFO },
                );
            }
        }

        if let Some(app) = running_apps.get(self.selected_app) {
            let mut path_line = NumberLine::new();
            path_line.push_str("path ");
            path_line.push_str(&app.path);
            self.put_str_px_max(stride, 280, 250, path_line.as_str(), MUTED, 492);
        } else if let Some(done) = finished_apps.first() {
            let mut done_line = NumberLine::new();
            done_line.push_str("last ");
            done_line.push_usize(done.pid);
            done_line.push_str(" ");
            done_line.push_str(&done.status);
            self.put_str_px_max(stride, 280, 250, done_line.as_str(), MUTED, 492);
        }
        self.draw_action_button(stride, 280, 258, 52, 15, "Close", METRIC_INFO);
        self.draw_action_button(stride, 340, 258, 44, 15, "Kill", METRIC_DANGER);
        self.draw_action_button(stride, 392, 258, 48, 15, "Path", METRIC_WARN);
        self.put_str_px(stride, 280, 274, self.status_note, MUTED);
        self.window.mark_dirty_all();
    }

    fn select_next_app(&mut self) {
        let running = crate::app_lifecycle::running_apps();
        if running.is_empty() {
            self.selected_app = 0;
            self.status_note = "no app";
            return;
        }
        self.selected_app = (self.selected_app + 1).min(running.len() - 1);
        self.status_note = "selected app";
    }

    fn select_prev_app(&mut self) {
        if crate::app_lifecycle::running_apps().is_empty() {
            self.selected_app = 0;
            self.status_note = "no app";
            return;
        }
        self.selected_app = self.selected_app.saturating_sub(1);
        self.status_note = "selected app";
    }

    fn selected_running_app(&self) -> Option<crate::app_lifecycle::RunningApp> {
        crate::app_lifecycle::running_apps()
            .get(self.selected_app)
            .cloned()
    }

    fn request_close_selected(&mut self) {
        if let Some(app) = self.selected_running_app() {
            self.pending_request = Some(SysMonRequest::ClosePid(app.pid));
            self.status_note = "close queued";
        } else {
            self.status_note = "no app";
        }
    }

    fn request_kill_selected(&mut self) {
        if let Some(app) = self.selected_running_app() {
            self.pending_request = Some(SysMonRequest::KillPid(app.pid));
            self.status_note = "kill queued";
        } else {
            self.status_note = "no app";
        }
    }

    fn request_path_selected(&mut self) {
        if let Some(app) = self.selected_running_app() {
            if app.path.is_empty() {
                self.status_note = "no path";
            } else {
                self.pending_request = Some(SysMonRequest::OpenPath(app.path));
                self.status_note = "path queued";
            }
        } else {
            self.status_note = "no app";
        }
    }

    fn force_redraw(&mut self) {
        self.last_redraw_tick = 0;
        self.update();
    }

    fn fill_background(&mut self, stride: usize) {
        for (idx, pixel) in self.window.buf.iter_mut().enumerate() {
            let py = idx / stride;
            *pixel = if py % 12 < 6 { BG } else { BG_ALT };
        }
    }

    fn draw_card_frame(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        accent: u32,
    ) {
        self.fill_rect(stride, x, y, w, h, PANEL_BG);
        self.fill_rect(stride, x, y, w, 3, accent);
        self.fill_rect(stride, x + 1, y + 1, w - 2, h - 2, PANEL_ALT);
        self.draw_rect_border(stride, x, y, w, h, PANEL_BORDER);
        self.draw_rect_border(stride, x + 1, y + 1, w - 2, h - 2, PANEL_INNER);
    }

    fn draw_bar(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        percent: usize,
        track: u32,
        fill: u32,
    ) {
        self.fill_rect(stride, x, y, w, h, track);
        self.draw_rect_border(stride, x, y, w, h, PANEL_INNER);
        let fill_w = (w.saturating_sub(2) * percent.min(100)) / 100;
        if fill_w > 0 {
            self.fill_rect(stride, x + 1, y + 1, fill_w, h.saturating_sub(2), fill);
        }
    }

    fn draw_status_pill(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        label: &str,
        active: bool,
        accent: u32,
    ) {
        let w = label.len() * CHAR_W_SMALL + 18;
        let bg = if active { PANEL_ALT } else { PANEL_BG };
        self.fill_rect(stride, x, y, w, 14, bg);
        self.draw_rect_border(
            stride,
            x,
            y,
            w,
            14,
            if active { accent } else { PANEL_INNER },
        );
        self.fill_rect(
            stride,
            x + 4,
            y + 4,
            4,
            4,
            if active { accent } else { DISABLED_DOT },
        );
        self.put_str_px(
            stride,
            x + 12,
            y + 3,
            label,
            if active { TEXT } else { MUTED },
        );
    }

    fn draw_action_button(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        label: &str,
        accent: u32,
    ) {
        self.fill_rect(stride, x, y, w, h, PANEL_ALT);
        self.draw_rect_border(stride, x, y, w, h, PANEL_BORDER);
        self.fill_rect(stride, x, y, w, 1, accent);
        self.put_str_px(stride, x + 6, y + 4, label, TEXT);
    }

    fn fill_rect(&mut self, stride: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let max_h = if stride > 0 {
            self.window.buf.len() / stride
        } else {
            0
        };
        for row in y..(y + h).min(max_h) {
            let base = row * stride;
            for col in x..(x + w).min(stride) {
                let idx = base + col;
                if idx < self.window.buf.len() {
                    self.window.buf[idx] = color;
                }
            }
        }
    }

    fn draw_rect_border(
        &mut self,
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: u32,
    ) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(stride, x, y, w, 1, color);
        self.fill_rect(stride, x, y + h - 1, w, 1, color);
        self.fill_rect(stride, x, y, 1, h, color);
        self.fill_rect(stride, x + w - 1, y, 1, h, color);
    }

    fn put_str_px(&mut self, stride: usize, px: usize, py: usize, s: &str, color: u32) {
        for (ci, c) in s.chars().enumerate() {
            let gx = px + ci * CHAR_W_SMALL;
            if gx + CHAR_W_SMALL > stride {
                break;
            }
            put_char_buf_transparent(&mut self.window.buf, stride, gx, py, c, color);
        }
    }

    fn put_str_px_max(
        &mut self,
        stride: usize,
        px: usize,
        py: usize,
        s: &str,
        color: u32,
        max_x: usize,
    ) {
        for (ci, c) in s.chars().enumerate() {
            let gx = px + ci * CHAR_W_SMALL;
            if gx + CHAR_W_SMALL > stride || gx + CHAR_W_SMALL > max_x {
                break;
            }
            put_char_buf_transparent(&mut self.window.buf, stride, gx, py, c, color);
        }
    }

    fn put_str_centered_px(&mut self, stride: usize, py: usize, s: &str, color: u32) {
        let text_w = s.chars().count() * CHAR_W_SMALL;
        let px = stride.saturating_sub(text_w) / 2;
        self.put_str_px(stride, px, py, s, color);
    }
}

struct NumberLine {
    buf: [u8; 64],
    len: usize,
}

impl NumberLine {
    fn new() -> Self {
        NumberLine {
            buf: [b' '; 64],
            len: 0,
        }
    }

    fn push_str(&mut self, s: &str) {
        for b in s.bytes() {
            if self.len < 64 {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
    }

    fn push_u64(&mut self, mut n: u64) {
        if n == 0 {
            self.push_str("0");
            return;
        }
        let mut tmp = [0u8; 20];
        let mut i = 20usize;
        while n > 0 {
            i -= 1;
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        for &b in &tmp[i..] {
            if self.len < 64 {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
    }

    fn push_usize(&mut self, n: usize) {
        self.push_u64(n as u64);
    }

    fn push_size(&mut self, bytes: usize) {
        const KIB: usize = 1024;
        const MIB: usize = 1024 * 1024;
        const GIB: usize = 1024 * 1024 * 1024;

        if bytes >= GIB {
            self.push_fixed_1(bytes, GIB);
            self.push_str(" GiB");
        } else if bytes >= MIB {
            self.push_fixed_1(bytes, MIB);
            self.push_str(" MiB");
        } else if bytes >= KIB {
            self.push_fixed_1(bytes, KIB);
            self.push_str(" KiB");
        } else {
            self.push_usize(bytes);
            self.push_str(" B");
        }
    }

    fn push_fixed_1(&mut self, value: usize, unit: usize) {
        let mut whole = value / unit;
        let mut frac = ((value % unit) * 10 + unit / 2) / unit;
        if frac >= 10 {
            whole += 1;
            frac = 0;
        }
        self.push_usize(whole);
        self.push_str(".");
        self.push_usize(frac);
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

fn put_char_buf_transparent(
    buf: &mut [u32],
    stride: usize,
    px0: usize,
    py0: usize,
    c: char,
    fg: u32,
) {
    let glyph = crate::font::glyph_rows(c, crate::font::UI_FONT);
    for (gy, &byte) in glyph.iter().enumerate() {
        for bit in 0..8usize {
            if byte & (1 << bit) == 0 {
                continue;
            }
            let px = px0 + bit;
            let py = py0 + gy;
            let idx = py * stride + px;
            if idx < buf.len() {
                buf[idx] = fg;
            }
        }
    }
}

fn hit(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}
