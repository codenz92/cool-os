/// Terminal app — renders a shell into a window's pixel back-buffer.
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use font8x8::UnicodeFonts;

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

const TERM_BG_A: u32 = 0x00_03_09_06;
const TERM_BG_B: u32 = 0x00_01_04_02;
const TERM_BG_C: u32 = 0x00_06_0F_09;
const FG_OUTPUT: u32 = 0x00_B8_F3_CE;
const FG_PROMPT: u32 = 0x00_00_FF_88;
const FG_INPUT: u32 = 0x00_E4_FF_F1;
const FG_ACCENT: u32 = 0x00_55_FF_FF;
const FG_DIM: u32 = 0x00_58_8A_70;
const FG_ERROR: u32 = 0x00_FF_72_72;
const FG_DIR: u32 = 0x00_55_DD_FF;
const FG_WARN: u32 = 0x00_FF_CC_44;

const HISTORY_MAX: usize = 32;

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

pub struct TerminalApp {
    pub window: Window,
    tty_id: u64,
    cmd_buf: String,
    pending_key_sink_fd: Option<usize>,
    foreground_job: Option<ForegroundJob>,
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    fg: u32,
    cwd: String,
    cmd_history: Vec<String>,
    history_pos: usize,
    saved_input: String,
    input_start_col: usize,
    last_width: i32,
    last_height: i32,
}

impl TerminalApp {
    pub fn new(x: i32, y: i32) -> Self {
        let window = Window::new(x, y, TERM_W, TERM_H, "Terminal");
        let cols = text_cols(TERM_W as usize);
        let rows = text_rows((TERM_H - TITLE_H) as usize);
        let tty_id = crate::tty::create();

        let mut t = TerminalApp {
            window,
            tty_id,
            cmd_buf: String::new(),
            pending_key_sink_fd: None,
            foreground_job: None,
            col: 0,
            row: 0,
            cols,
            rows,
            fg: FG_OUTPUT,
            cwd: String::from("/"),
            cmd_history: Vec::new(),
            history_pos: 0,
            saved_input: String::new(),
            input_start_col: 0,
            last_width: TERM_W,
            last_height: TERM_H,
        };
        t.fill_background();
        t.set_fg(FG_ACCENT);
        t.print_str("coolOS phosphor shell\n");
        t.set_fg(FG_DIM);
        t.print_str("type help for commands\n\n");
        t.print_prompt();
        t
    }

    pub fn update(&mut self) {
        self.drain_tty_output();
        self.poll_foreground_job();
        if self.window.width == self.last_width && self.window.height == self.last_height {
            return;
        }

        let old_width = self.last_width.max(0) as usize;
        let old_content_h = (self.last_height - TITLE_H).max(0) as usize;
        self.last_width = self.window.width;
        self.last_height = self.window.height;
        self.refresh_layout();
        self.paint_exposed_background(old_width, old_content_h);
    }

    pub fn is_busy(&self) -> bool {
        self.foreground_job.is_some()
    }

    pub fn handle_key(&mut self, c: char) {
        if self.foreground_job.is_some() {
            return;
        }
        self.refresh_layout();
        match c {
            // Arrow keys (private-use Unicode set by keyboard drivers)
            '\u{F700}' => self.history_up(),
            '\u{F701}' => self.history_down(),
            '\u{F702}' | '\u{F703}' => {} // left/right: ignore (no cursor movement)

            '\n' => {
                self.print_char('\n');
                let cmd = core::mem::take(&mut self.cmd_buf);
                self.push_history(&cmd);
                crate::app_lifecycle::record_command(&cmd);
                self.run_command(&cmd);
            }
            '\u{0008}' => {
                if self.cmd_buf.pop().is_some() && self.col > self.input_start_col {
                    self.col -= 1;
                    self.draw_char_at(self.col, self.row, ' ');
                }
            }
            c if !c.is_control() => {
                let max = self.cols.saturating_sub(self.input_start_col + 1);
                if self.cmd_buf.len() < max {
                    self.cmd_buf.push(c);
                    self.print_char(c);
                }
            }
            _ => {}
        }
    }

    pub fn handle_key_input(&mut self, input: KeyInput) {
        if input.has_ctrl() && !input.has_alt() {
            match input.key {
                Key::Character('c') | Key::Character('C') => {
                    if self.signal_foreground(crate::process_model::Signal::Int, "^C\n") {
                        return;
                    }
                    return;
                }
                Key::Character('z') | Key::Character('Z') => {
                    if self.signal_foreground(crate::process_model::Signal::Stop, "^Z\n") {
                        return;
                    }
                    return;
                }
                _ => {}
            }
        }
        if self.foreground_job.is_some() {
            self.forward_foreground_input(input);
            return;
        }
        if let Some(c) = input.legacy_char() {
            self.handle_key(c);
        }
    }

    fn forward_foreground_input(&mut self, input: KeyInput) {
        if input.has_ctrl() && !input.has_alt() {
            match input.key {
                Key::Character('d') | Key::Character('D') => {
                    let _ = crate::tty::submit_eof(self.tty_id);
                }
                _ => {}
            }
            return;
        }
        if input.has_alt() {
            return;
        }
        match input.key {
            Key::Character(c) => {
                let _ = crate::tty::submit_char(self.tty_id, c);
            }
            Key::Space => {
                let _ = crate::tty::submit_char(self.tty_id, ' ');
            }
            Key::Tab => {
                let _ = crate::tty::submit_char(self.tty_id, '\t');
            }
            Key::Enter => {
                let _ = crate::tty::submit_enter(self.tty_id);
            }
            Key::Backspace | Key::Delete => {
                let _ = crate::tty::submit_backspace(self.tty_id);
            }
            Key::Escape
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::F2
            | Key::F4
            | Key::F5 => {}
        }
    }

    pub fn execute_command(&mut self, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        if self.foreground_job.is_some() {
            crate::wm::queue_startup_command(command);
            return;
        }
        self.refresh_layout();
        for c in command.chars() {
            if !c.is_control() {
                self.print_char(c);
            }
        }
        self.print_char('\n');
        self.push_history(command);
        crate::app_lifecycle::record_command(command);
        self.run_command(command);
    }

    pub fn take_pending_key_sink(&mut self) -> Option<usize> {
        self.pending_key_sink_fd.take()
    }

    pub fn drain_tty_output(&mut self) {
        while let Some(byte) = crate::tty::pop_output_byte(self.tty_id) {
            self.print_char(byte as char);
        }
    }

    pub fn poll_foreground_job(&mut self) {
        let Some(job) = self.foreground_job.as_ref() else {
            return;
        };
        match crate::scheduler::process_group_status(job.group) {
            crate::scheduler::ProcessGroupStatus::Running => {}
            crate::scheduler::ProcessGroupStatus::Stopped => {
                let job = self.foreground_job.take().unwrap();
                crate::tty::set_foreground_group(self.tty_id, None);
                self.set_fg(FG_WARN);
                self.print_str("\n[fg stopped] ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&job.title);
                self.print_str(" pgid=");
                self.print_u64(job.group as u64);
                if let Some(job_id) = job.job_id {
                    self.print_str(" job #");
                    self.print_u64(job_id);
                }
                self.print_char('\n');
                self.print_prompt();
            }
            crate::scheduler::ProcessGroupStatus::Exited
            | crate::scheduler::ProcessGroupStatus::Empty => {
                let job = self.foreground_job.take().unwrap();
                crate::tty::set_foreground_group(self.tty_id, None);
                self.set_fg(FG_ACCENT);
                self.print_str("\n[fg done] ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&job.title);
                self.print_str(" pid=");
                self.print_u64(job.pid as u64);
                if let Some(code) = crate::scheduler::process_group_exit_code(job.group) {
                    self.print_str(" exit=");
                    self.print_u64(code);
                }
                if let Some(job_id) = job.job_id {
                    self.print_str(" job #");
                    self.print_u64(job_id);
                }
                self.print_char('\n');
                self.print_prompt();
            }
        }
    }

    fn signal_foreground(&mut self, signal: crate::process_model::Signal, marker: &str) -> bool {
        let Some(group) = crate::tty::foreground_group(self.tty_id) else {
            return false;
        };
        self.set_fg(FG_DIM);
        self.print_str(marker);
        match crate::scheduler::send_signal_to_group(group, signal) {
            Ok(_) => true,
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("signal: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                true
            }
        }
    }

    // ── History ───────────────────────────────────────────────────────────────

    fn push_history(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return;
        }
        if self.cmd_history.last().map(|s| s.as_str()) == Some(cmd) {
            return;
        }
        if self.cmd_history.len() >= HISTORY_MAX {
            self.cmd_history.remove(0);
        }
        self.cmd_history.push(String::from(cmd));
        self.history_pos = 0;
        self.saved_input.clear();
    }

    fn history_up(&mut self) {
        if self.cmd_history.is_empty() {
            return;
        }
        let new_pos = (self.history_pos + 1).min(self.cmd_history.len());
        if new_pos == self.history_pos {
            return;
        }
        if self.history_pos == 0 {
            self.saved_input = self.cmd_buf.clone();
        }
        self.history_pos = new_pos;
        let entry = self.cmd_history[self.cmd_history.len() - self.history_pos].clone();
        self.erase_input();
        self.cmd_buf = entry.clone();
        self.set_fg(FG_INPUT);
        self.print_str(&entry);
    }

    fn history_down(&mut self) {
        if self.history_pos == 0 {
            return;
        }
        self.history_pos -= 1;
        self.erase_input();
        if self.history_pos == 0 {
            let saved = self.saved_input.clone();
            self.cmd_buf = saved.clone();
            self.set_fg(FG_INPUT);
            self.print_str(&saved);
        } else {
            let entry = self.cmd_history[self.cmd_history.len() - self.history_pos].clone();
            self.cmd_buf = entry.clone();
            self.set_fg(FG_INPUT);
            self.print_str(&entry);
        }
    }

    fn erase_input(&mut self) {
        while self.col > self.input_start_col {
            self.col -= 1;
            self.draw_char_at(self.col, self.row, ' ');
        }
        self.cmd_buf.clear();
    }

    // ── Command dispatch ──────────────────────────────────────────────────────

    fn run_command(&mut self, input: &str) {
        let mut words = input.split_whitespace();
        self.set_fg(FG_OUTPUT);
        match words.next() {
            Some("help") => self.cmd_help(),

            Some("clear") => {
                self.fill_background();
                self.col = 0;
                self.row = 0;
            }

            Some("reboot") => crate::interrupts::reboot(),

            Some("echo") => {
                for word in words {
                    self.print_str(word);
                    self.print_char(' ');
                }
                self.print_char('\n');
            }

            Some("pwd") => {
                self.set_fg(FG_DIR);
                let cwd = self.cwd.clone();
                self.print_str(&cwd);
                self.print_char('\n');
            }

            Some("cd") => {
                let target = match words.next() {
                    Some(p) => resolve_path(&self.cwd, p),
                    None => String::from("/"),
                };
                if crate::vfs::vfs_list_dir(&target).is_some() {
                    self.cwd = target;
                } else {
                    self.set_fg(FG_ERROR);
                    self.print_str("cd: no such directory\n");
                }
            }

            Some("ls") => {
                let path_arg = words.next();
                let path = match path_arg {
                    Some(p) => resolve_path(&self.cwd, p),
                    None => self.cwd.clone(),
                };
                self.cmd_ls(&path);
            }

            Some("touch") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_touch(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: touch <path>\n");
                }
            },

            Some("mkdir") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_mkdir(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: mkdir <path>\n");
                }
            },

            Some("cat") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_cat(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: cat <path>\n");
                }
            },

            Some("hash") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_hash(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: hash <path>\n");
                }
            },

            Some("write") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    let text = collect_words(words);
                    self.cmd_write_file(&path, &text);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: write <path> <text>\n");
                }
            },

            Some("rm") => match words.next() {
                Some(p) => {
                    let path = resolve_path(&self.cwd, p);
                    self.cmd_rm(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: rm <path>\n");
                }
            },

            Some("ps") => self.cmd_ps(),

            Some("kill") => match words.next().and_then(parse_usize) {
                Some(pid) => self.cmd_kill(pid),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: kill <pid>\n");
                }
            },

            Some("wait") => match words.next().and_then(parse_usize) {
                Some(pid) => self.cmd_wait(pid),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: wait <pid>\n");
                }
            },

            Some("reap") => self.cmd_reap(),

            Some("info") => self.cmd_info(),

            Some("uptime") => self.cmd_uptime(),

            Some("devices") => self.cmd_devices(),

            Some("net") => self.cmd_lines("NETWORK", crate::net::status_lines()),

            Some("netproto") => self.cmd_lines("NETWORK PROTOCOLS", crate::net::protocol_lines()),

            Some("tlscheck") => {
                let lines = crate::tls::selftest_lines();
                for line in &lines {
                    crate::println!("{}", line);
                }
                self.cmd_lines("TLS HOSTNAME CHECK", lines);
            }

            Some("netapi") => {
                self.cmd_lines("NETWORK API SETTINGS", crate::settings_state::lines())
            }

            Some(cmd @ ("http" | "https")) => {
                let host = words.next();
                let path = words.next().unwrap_or("/");
                match host {
                    Some(host) => self.cmd_http(cmd, host, path),
                    None => {
                        self.set_fg(FG_ERROR);
                        self.print_str("usage: http|https <host-or-url> [path]\n");
                    }
                }
            }

            Some("dns") => match words.next() {
                Some(host) => match crate::net::dns_resolve(host) {
                    Ok(addr) => self.cmd_lines(
                        "DNS",
                        alloc::vec![alloc::format!(
                            "{} -> {}",
                            host,
                            crate::net::ipv4_string(addr)
                        )],
                    ),
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("dns: ");
                        self.print_str(err);
                        self.print_char('\n');
                    }
                },
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: dns <host>\n");
                }
            },

            Some("ping") => match words.next() {
                Some(host) => match crate::net::dns_resolve(host).and_then(crate::net::icmp_ping) {
                    Ok(()) => self.cmd_lines("PING", alloc::vec![alloc::format!("{} ok", host)]),
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("ping: ");
                        self.print_str(err);
                        self.print_char('\n');
                    }
                },
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: ping <host-or-ip>\n");
                }
            },

            Some("power") => self.cmd_power(words.next()),

            Some("log") => self.cmd_log(),

            Some("logs") => self.cmd_lines("LOG VIEW", crate::klog::lines()),

            Some("diagnostics") | Some("diag") => {
                self.cmd_lines("DIAGNOSTICS", diagnostics_lines())
            }

            Some("sysreport") => self.cmd_sysreport(words.next()),

            Some("devkit") => self.cmd_devkit(),

            Some("profiler") => {
                let mut lines = crate::profiler::lines();
                lines.extend(crate::boot_watchdog::lines());
                lines.extend(crate::deferred::lines());
                self.cmd_lines("BOOT/SESSION PROFILER", lines);
            }

            Some("compositor") | Some("smoothness") => {
                self.cmd_lines("COMPOSITOR", crate::wm::compositor::compositor_lines())
            }

            Some("heap") => self.cmd_lines("HEAP DIAGNOSTICS", crate::allocator::heap_lines()),

            Some("slab") => self.cmd_lines("SLAB DIAGNOSTICS", crate::slab::lines()),

            Some("waitq") => self.cmd_lines("WAIT QUEUES", crate::wait_queue::lines()),

            Some("writeback") => self.cmd_lines("WRITEBACK", crate::writeback::lines()),

            Some("selftest") => self.cmd_lines("SELFTEST", crate::selftest::lines()),

            Some("font") => self.cmd_lines("FONT RENDERER", crate::font::lines()),

            Some("deferred") => self.cmd_lines("DEFERRED WORK", crate::deferred::lines()),

            Some("tasksnap") => self.cmd_lines("TASK SNAPSHOT", crate::task_snapshot::lines()),

            Some("fsck") => self.cmd_fsck(),

            Some("coolfs") => self.cmd_lines("COOLFS", crate::coolfs::lines()),

            Some("fsrepair") => self.cmd_lines("FS REPAIR", crate::fs_hardening::repair()),

            Some("recovery") => self.cmd_recovery(words.collect()),

            Some("mounts") => self.cmd_lines("MOUNTS", crate::fs_hardening::status_lines()),

            Some("vfs") => self.cmd_lines("VFS", crate::vfs::mount_lines()),

            Some("path") => match words.next() {
                Some(path) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_lines("PATH", crate::vfs::path_lines(&[&path]));
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: path <file-or-dir>\n");
                }
            },

            Some("perm") => match words.next() {
                Some(path) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_perm(&path);
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: perm <path>\n");
                }
            },

            Some("chmod") => match (words.next(), words.next()) {
                (Some(mode), Some(path)) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_chmod(mode, &path);
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: chmod <mode> <path>\n");
                }
            },

            Some("chown") => match (words.next(), words.next()) {
                (Some(owner), Some(path)) => {
                    let path = resolve_path(&self.cwd, path);
                    self.cmd_chown(owner, &path);
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: chown <uid>[:gid] <path>\n");
                }
            },

            Some("journal") => self.cmd_lines("FS JOURNAL", crate::fs_hardening::journal_lines()),

            Some("flush") => self.print_result(
                "flush",
                crate::writeback::barrier().map_err(|_| "flush failed"),
            ),

            Some("df") => self.cmd_df(),

            Some("shortcuts") => self.cmd_lines("SHORTCUTS", crate::shortcuts::summary_lines()),

            Some("icons") => self.cmd_lines("DESKTOP ICONS", crate::desktop_settings::icon_lines()),

            Some("access") => self.cmd_access(words.next(), words.next()),

            Some("apps") => self.cmd_lines("APP LIFECYCLE", crate::app_lifecycle::lines()),

            Some("appcats") => {
                self.cmd_lines("APP CATEGORIES", crate::app_metadata::category_lines())
            }

            Some("pinned") => self.cmd_pinned(words.collect()),

            Some("unpin") => self.cmd_unpin(collect_words(words)),

            Some("startmenu") => self.cmd_startmenu(words.next()),

            Some("recent") => self.cmd_recent(),

            Some("startup") => self.cmd_startup(words.collect()),

            Some("search") => {
                let query = collect_words(words);
                if query.is_empty() {
                    self.cmd_lines("SEARCH INDEX", crate::search_index::lines(None));
                } else {
                    self.cmd_lines("SEARCH", crate::search_index::lines(Some(&query)));
                }
            }

            Some("index") => {
                crate::search_index::refresh();
                self.cmd_lines("SEARCH INDEX", crate::search_index::lines(None));
            }

            Some("drivers") => {
                crate::drivers::refresh();
                self.cmd_lines("DRIVERS", crate::drivers::lines());
            }

            Some("whoami") => self.cmd_whoami(),

            Some("id") => self.cmd_id(words.next()),

            Some("groups") => self.cmd_groups(words.next()),

            Some("login") | Some("su") => self.cmd_login(words.next(), words.next()),

            Some("lock") => self.cmd_lock(),

            Some("logout") => self.cmd_logout(),

            Some("passwd") => self.cmd_passwd(words.next(), words.next()),

            Some("setup") => self.cmd_setup(words.next(), words.next()),

            Some("account") => self.cmd_account(words.collect()),

            Some("umask") => self.cmd_umask(words.next()),

            Some("users") => self.cmd_lines("USERS", crate::security::lines()),

            Some("security") => self.cmd_lines("SECURITY", crate::security::lines()),

            Some("pkg") => {
                let op = words.next();
                let arg = words.next();
                let rest: Vec<&str> = words.collect();
                self.cmd_pkg(op, arg, rest);
            }

            Some("proc") => self.cmd_lines("PROCESS MODEL", crate::process_model::status_lines()),

            Some("zombies") => {
                self.cmd_lines("ZOMBIE POLICY", crate::process_model::zombie_policy_lines())
            }

            Some("signal") => self.cmd_signal(words.next(), words.next()),

            Some("pgroup") => self.cmd_pgroup(words.next(), words.next()),

            Some("tty") => self.cmd_tty(),

            Some("events") => self.cmd_lines("EVENTS", crate::event_bus::lines(12)),

            Some("jobs") => self.cmd_lines("JOBS", crate::jobs::lines()),

            Some("job") => self.cmd_job(words.collect()),

            Some("fg") => self.cmd_fg(words.next()),

            Some("bg") => self.cmd_bg(words.next()),

            Some("services") => self.cmd_services(words.next(), words.next()),

            Some("crash") => self.cmd_lines("CRASH DUMP", crate::crashdump::detailed_lines()),

            Some("abi") => self.cmd_lines("ABI", crate::abi::lines()),

            Some("notify") => self.cmd_notify(words.next(), words.next()),

            Some("screenshot") => {
                let path = words.next().unwrap_or("/LOGS/WINDOW.PPM");
                crate::wm::request_focused_screenshot(path);
                self.set_fg(FG_ACCENT);
                self.print_str("queued screenshot ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }

            Some("clip") => {
                let mut text = String::new();
                for word in words {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(word);
                }
                if text.is_empty() {
                    self.set_fg(FG_ACCENT);
                    self.print_str("clipboard: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(&crate::clipboard::summary());
                    self.print_str(" [");
                    self.print_str(crate::clipboard::mime_type());
                    self.print_str("]");
                    self.print_char('\n');
                } else {
                    crate::clipboard::set_text(&text);
                    self.set_fg(FG_ACCENT);
                    self.print_str("copied text\n");
                }
            }

            Some("clipmimes") => {
                self.cmd_lines("CLIPBOARD MIME TYPES", crate::clipboard::mime_lines())
            }

            Some("clipimg") => match (words.next(), words.next()) {
                (Some(w), Some(h)) => {
                    let width = parse_usize(w).unwrap_or(16).min(64);
                    let height = parse_usize(h).unwrap_or(16).min(64);
                    let mut pixels = Vec::new();
                    pixels.resize(width.saturating_mul(height).saturating_mul(4), 0u8);
                    for y in 0..height {
                        for x in 0..width {
                            let idx = (y * width + x) * 4;
                            let hot = ((x / 4) + (y / 4)) % 2 == 0;
                            pixels[idx] = if hot { 0x00 } else { 0x22 };
                            pixels[idx + 1] = if hot { 0xbb } else { 0x44 };
                            pixels[idx + 2] = if hot { 0xff } else { 0x88 };
                            pixels[idx + 3] = 0xff;
                        }
                    }
                    crate::clipboard::set_image(width as u32, height as u32, pixels, "image/rgba");
                    self.set_fg(FG_ACCENT);
                    self.print_str("copied image payload\n");
                }
                _ => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: clipimg <w> <h>\n");
                }
            },

            Some("paste") => match crate::clipboard::get_text() {
                Some(text) => {
                    self.set_fg(FG_OUTPUT);
                    self.print_str(&text);
                    self.print_char('\n');
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("paste: clipboard has no text\n");
                }
            },

            Some("exec") => match words.next() {
                Some(path) => {
                    let args: Vec<&str> = words.collect();
                    let abs = resolve_path(&self.cwd, path);
                    match crate::elf::spawn_elf_process_suspended_with_args(&abs, &args) {
                        Ok(pid) => {
                            if self.configure_process_tty(pid, pid) {
                                self.begin_foreground(pid, pid, None, &abs);
                                crate::scheduler::unblock(pid);
                            } else {
                                let _ = crate::scheduler::kill_task(pid, 143);
                            }
                        }
                        Err(err) => {
                            self.set_fg(FG_ERROR);
                            self.print_str("exec: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("usage: exec <path> [args...]\n");
                }
            },

            Some("sh") | Some("shell") => {
                let abs = "/bin/sh";
                match crate::elf::spawn_elf_process_suspended_with_args(abs, &[]) {
                    Ok(pid) => {
                        if self.configure_process_tty(pid, pid) {
                            self.begin_foreground(pid, pid, None, abs);
                            crate::scheduler::unblock(pid);
                        } else {
                            let _ = crate::scheduler::kill_task(pid, 143);
                        }
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("sh: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err.as_str());
                        self.print_char('\n');
                    }
                }
            }

            Some("ipc") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    let r =
                        crate::elf::spawn_elf_process_with_fds("/bin/piperd", &[], &[(read_fd, 3)]);
                    let w = crate::elf::spawn_elf_process_with_fds(
                        "/bin/pipewr",
                        &[],
                        &[(write_fd, 3)],
                    );
                    crate::vfs::vfs_close(read_fd);
                    crate::vfs::vfs_close(write_fd);
                    match (r, w) {
                        (Ok(_), Ok(_)) => {
                            self.set_fg(FG_ACCENT);
                            self.print_str("pipe demo spawned\n");
                        }
                        _ => {
                            self.set_fg(FG_ERROR);
                            self.print_str("ipc: spawn failed\n");
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("ipc: no pipe slots\n");
                }
            },

            Some("keydemo") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    match crate::elf::spawn_elf_process_with_fds(
                        "/bin/keyecho",
                        &[],
                        &[(read_fd, 3)],
                    ) {
                        Ok(_) => {
                            crate::vfs::vfs_close(read_fd);
                            self.pending_key_sink_fd = Some(write_fd);
                            self.set_fg(FG_ACCENT);
                            self.print_str("keydemo active — ~ ends\n");
                        }
                        Err(err) => {
                            crate::vfs::vfs_close(read_fd);
                            crate::vfs::vfs_close(write_fd);
                            self.set_fg(FG_ERROR);
                            self.print_str("keydemo: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("keydemo: no pipe slots\n");
                }
            },

            Some("term") => match crate::vfs::vfs_pipe() {
                Some((read_fd, write_fd)) => {
                    match crate::elf::spawn_elf_process_with_stdin("/bin/terminal", &[], read_fd) {
                        Ok(()) => {
                            self.pending_key_sink_fd = Some(write_fd);
                            self.set_fg(FG_ACCENT);
                            self.print_str("userspace terminal — Ctrl+D ends\n");
                        }
                        Err(err) => {
                            crate::vfs::vfs_close(read_fd);
                            crate::vfs::vfs_close(write_fd);
                            self.set_fg(FG_ERROR);
                            self.print_str("term: ");
                            self.set_fg(FG_OUTPUT);
                            self.print_str(err.as_str());
                            self.print_char('\n');
                        }
                    }
                }
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("term: no pipe slots\n");
                }
            },

            Some("usb") => {
                let lines = crate::usb::status_lines();
                if lines.is_empty() {
                    self.set_fg(FG_WARN);
                    self.print_str("USB: no probe data\n");
                } else {
                    self.set_fg(FG_ACCENT);
                    self.print_str("USB STATUS\n");
                    for line in lines {
                        self.set_fg(FG_OUTPUT);
                        self.print_str(&line);
                        self.print_char('\n');
                    }
                }
            }

            Some(unknown) => {
                self.set_fg(FG_ERROR);
                self.print_str(unknown);
                self.set_fg(FG_DIM);
                self.print_str(": not found. ");
                self.set_fg(FG_OUTPUT);
                self.print_str("type help\n");
            }

            None => {}
        }
        if self.foreground_job.is_none() {
            self.print_prompt();
        }
    }

    fn cmd_help(&mut self) {
        let cmds: &[(&str, &str)] = &[
            ("help", "list commands"),
            ("clear", "clear terminal"),
            ("reboot", "restart OS"),
            ("pwd", "print working directory"),
            ("cd <dir>", "change directory"),
            ("ls [path]", "list directory contents"),
            ("touch <path>", "create empty file"),
            ("mkdir <path>", "create folder"),
            ("cat <path>", "print file to terminal"),
            ("hash <path>", "print file length and byte sum"),
            ("write <path> <text>", "write text file"),
            ("rm <path>", "remove file or empty folder"),
            ("ps", "list running processes"),
            ("kill <pid>", "terminate a task"),
            ("wait <pid>", "reap an exited child"),
            ("reap", "reap all exited tasks"),
            ("exec <path>", "run ELF binary"),
            ("sh", "start userspace shell"),
            ("info", "CPU, memory, and system info"),
            ("uptime", "time since boot"),
            ("usb", "USB controller status"),
            ("devices", "PCI/USB/device registry"),
            ("drivers", "driver binding + /DEV nodes"),
            ("net", "network stack status"),
            ("netproto", "ARP/IP/UDP/DNS/HTTP status"),
            ("tlscheck", "TLS hostname negative checks"),
            ("netapi", "network/settings API toggles"),
            ("dns <host>", "resolve host with staged DNS"),
            ("ping <host>", "send ICMP echo request"),
            ("http|https <host-or-url> [path]", "run kernel web client"),
            ("power <op>", "ACPI power status"),
            ("log", "kernel log tail"),
            ("logs", "open combined log summary"),
            ("diagnostics", "combined logs/profiler/fs/memory status"),
            ("sysreport [write]", "combined diagnostics report"),
            ("devkit", "SDK docs and app templates"),
            ("profiler", "boot/service/task timing"),
            ("compositor", "FPS, frame, damage, and cursor telemetry"),
            ("smoothness", "compositor latency telemetry"),
            ("heap", "heap diagnostics"),
            ("slab", "slab allocator diagnostics"),
            ("waitq", "kernel wait queue diagnostics"),
            ("writeback", "async disk writeback state"),
            ("selftest", "boot kernel unit-style checks"),
            ("font", "font renderer diagnostics"),
            ("deferred", "deferred work queue"),
            ("tasksnap", "persistent task snapshot"),
            ("fsck", "filesystem check summary"),
            ("coolfs", "CoolFS mount status"),
            ("fsrepair", "repair standard FS dirs"),
            ("recovery [op]", "boot recovery status/repair"),
            ("mounts", "mount/cache/journal status"),
            ("vfs", "mount table and fd tables"),
            ("path <path>", "inspect normalized VFS path"),
            ("perm <path>", "show owner and mode"),
            ("chmod <mode> <path>", "change CoolFS mode"),
            ("chown <uid>[:gid] <path>", "change CoolFS owner"),
            ("journal", "filesystem journal tail"),
            ("flush", "flush filesystem journal"),
            ("df", "filesystem free space"),
            ("shortcuts", "configured shortcut keys"),
            ("icons", "desktop icon positions"),
            ("access [key on/off]", "accessibility settings"),
            ("apps", "app lifecycle metadata"),
            ("appcats", "app categories"),
            ("pinned [apps...]", "view/set pinned apps"),
            ("unpin <item>", "remove pinned Start item"),
            ("startmenu [compact|full]", "view/set Start menu layout"),
            ("recent", "recent apps, files, commands, searches"),
            ("startup [apps...]", "view/set startup apps"),
            ("search <query>", "search indexed files"),
            ("index", "rebuild desktop search index"),
            ("whoami", "current user and task grants"),
            ("id [user]", "user identity and home"),
            ("groups [user]", "group membership"),
            ("login <user> <pass>", "switch active session"),
            ("lock", "lock the desktop session"),
            ("logout", "return to guest session"),
            ("passwd <old> <new>", "change current password"),
            ("setup <user> <pass>", "complete first-run admin setup"),
            ("account <op>", "admin user management"),
            ("umask [mode]", "view/set file creation mask"),
            ("users", "user/security status"),
            ("pkg <op>", "package list/install/remove/run"),
            ("proc", "process groups and signals"),
            ("zombies", "zombie cleanup policy"),
            ("signal <pid|-pgid> <sig>", "deliver signal to task/group"),
            ("pgroup <pid> [grp]", "view/set process group"),
            ("tty", "current terminal session state"),
            ("events", "event bus tail"),
            ("jobs", "background job history"),
            ("job run|cancel|pause|resume", "manage background jobs"),
            ("fg [id|last]", "resume job in foreground"),
            ("bg [id|last]", "resume job in background"),
            ("services <op>", "service supervisor"),
            ("crash", "crash dump summary"),
            ("abi", "kernel/user ABI version"),
            ("notify <op>", "notification history/actions"),
            ("screenshot [path]", "save focused window PPM"),
            ("clip [text]", "shared clipboard"),
            ("clipmimes", "clipboard MIME negotiation"),
            ("clipimg <w> <h>", "copy RGBA image payload"),
            ("paste", "paste shared clipboard text"),
            ("echo <text>", "print text"),
            ("ipc", "pipe demo"),
            ("keydemo", "keyboard event stream"),
            ("term", "userspace terminal"),
        ];
        self.set_fg(FG_ACCENT);
        self.print_str("Commands:\n");
        for &(name, desc) in cmds {
            self.set_fg(FG_PROMPT);
            self.print_str("  ");
            self.print_str(name);
            // pad to column 18
            let name_len = name.len() + 2;
            for _ in name_len..20 {
                self.print_char(' ');
            }
            self.set_fg(FG_DIM);
            self.print_str(desc);
            self.print_char('\n');
        }
    }

    fn cmd_ls(&mut self, path: &str) {
        match crate::vfs::vfs_list_dir(path) {
            Some(mut entries) => {
                entries.sort_by(|a, b| {
                    if a.is_dir == b.is_dir {
                        a.name.cmp(&b.name)
                    } else if a.is_dir {
                        core::cmp::Ordering::Less
                    } else {
                        core::cmp::Ordering::Greater
                    }
                });
                if entries.is_empty() {
                    self.set_fg(FG_DIM);
                    self.print_str("(empty)\n");
                } else {
                    for e in &entries {
                        if e.is_dir {
                            self.set_fg(FG_DIR);
                            self.print_str(&e.name);
                            self.print_char('/');
                        } else {
                            self.set_fg(FG_OUTPUT);
                            self.print_str(&e.name);
                        }
                        self.print_char('\n');
                    }
                }
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("ls: no such directory\n");
            }
        }
    }

    fn cmd_cat(&mut self, path: &str) {
        match crate::vfs::vfs_read_file(path) {
            Some(bytes) => match core::str::from_utf8(&bytes) {
                Ok(text) => {
                    self.set_fg(FG_OUTPUT);
                    self.print_str(text);
                    if !text.ends_with('\n') {
                        self.print_char('\n');
                    }
                }
                Err(_) => {
                    self.set_fg(FG_WARN);
                    self.print_str("cat: binary file (");
                    self.print_u64(bytes.len() as u64);
                    self.print_str(" bytes)\n");
                }
            },
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("cat: file not found\n");
            }
        }
    }

    fn cmd_hash(&mut self, path: &str) {
        match crate::vfs::vfs_read_file(path) {
            Some(bytes) => {
                let sum = bytes
                    .iter()
                    .fold(0u64, |acc, byte| acc.wrapping_add(*byte as u64));
                self.set_fg(FG_OUTPUT);
                self.print_str("hash ");
                self.print_str(path);
                self.print_str(" len=");
                self.print_u64(bytes.len() as u64);
                self.print_str(" sum=");
                self.print_u64(sum);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("hash: file not found\n");
            }
        }
    }

    fn cmd_perm(&mut self, path: &str) {
        match crate::vfs::vfs_metadata(path) {
            Some(meta) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_str(" ");
                self.print_str(if meta.is_dir { "dir" } else { "file" });
                self.print_str(" uid=");
                self.print_u64(meta.uid as u64);
                self.print_str(" gid=");
                self.print_u64(meta.gid as u64);
                self.print_str(" mode=");
                self.print_str(&crate::security::format_mode(meta.mode));
                self.print_str(" size=");
                self.print_u64(meta.size);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("perm: not found or denied\n");
            }
        }
    }

    fn cmd_chmod(&mut self, mode: &str, path: &str) {
        let Some(mode) = crate::security::parse_mode(mode) else {
            self.set_fg(FG_ERROR);
            self.print_str("chmod: invalid mode\n");
            return;
        };
        match crate::vfs::vfs_chmod(path, mode) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("chmod ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("chmod: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_chown(&mut self, owner: &str, path: &str) {
        let Some((uid, gid)) = parse_owner(owner) else {
            self.set_fg(FG_ERROR);
            self.print_str("chown: invalid owner\n");
            return;
        };
        match crate::vfs::vfs_chown(path, uid, gid) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("chown ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("chown: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_write_file(&mut self, path: &str, text: &str) {
        match crate::vfs::vfs_create_file(path) {
            Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("write: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                return;
            }
        }
        match crate::vfs::vfs_write_file(path, text.as_bytes()) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("wrote ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("write: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_rm(&mut self, path: &str) {
        match crate::vfs::vfs_delete(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("removed ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("rm: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_touch(&mut self, path: &str) {
        match crate::vfs::vfs_create_file(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("created ");
                self.set_fg(FG_OUTPUT);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("touch: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_mkdir(&mut self, path: &str) {
        match crate::vfs::vfs_create_dir(path) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("created ");
                self.set_fg(FG_DIR);
                self.print_str(path);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("mkdir: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_ps(&mut self) {
        // Copy task info while holding the lock, then drop it before printing.
        let tasks: Vec<(
            usize,
            &'static str,
            crate::scheduler::TaskStatus,
            bool,
            bool,
            Option<u64>,
            Option<usize>,
            u32,
        )> = {
            let sched = crate::scheduler::SCHEDULER.lock();
            let cur = sched.current;
            sched
                .tasks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    (
                        i,
                        t.name,
                        t.status,
                        i == cur,
                        t.pml4.is_some(),
                        t.exit_code,
                        t.parent,
                        t.credentials.uid,
                    )
                })
                .collect()
        };

        self.set_fg(FG_ACCENT);
        self.print_str("PID  PPID  UID   RING  STATUS   EXIT  NAME\n");
        self.set_fg(FG_DIM);
        self.print_str("---  ----  ----  ----  -------  ----  ----\n");
        for (id, name, status, is_cur, is_user, exit_code, parent, uid) in tasks {
            self.set_fg(if is_cur { FG_PROMPT } else { FG_OUTPUT });
            self.print_u64(id as u64);
            self.print_str("    ");
            self.set_fg(FG_DIM);
            if let Some(parent) = parent {
                self.print_u64(parent as u64);
            } else {
                self.print_char('-');
            }
            self.print_str("     ");
            self.set_fg(FG_DIM);
            self.print_u64(uid as u64);
            self.print_str("  ");
            self.set_fg(FG_DIM);
            let ring = if is_user { "u" } else { "k" };
            self.print_str(ring);
            self.print_str("     ");
            self.set_fg(FG_OUTPUT);
            let status_str = match status {
                crate::scheduler::TaskStatus::Running => "running",
                crate::scheduler::TaskStatus::Ready => "ready  ",
                crate::scheduler::TaskStatus::Blocked => "blocked",
                crate::scheduler::TaskStatus::Stopped => "stopped",
                crate::scheduler::TaskStatus::Exited => "exited ",
                crate::scheduler::TaskStatus::Reaped => "reaped ",
            };
            self.print_str(status_str);
            self.print_str("  ");
            self.set_fg(FG_DIM);
            if let Some(code) = exit_code {
                self.print_u64(code);
            } else {
                self.print_char('-');
            }
            self.print_str("     ");
            if is_cur {
                self.set_fg(FG_PROMPT);
            } else if status == crate::scheduler::TaskStatus::Exited {
                self.set_fg(FG_DIM);
            } else {
                self.set_fg(FG_OUTPUT);
            }
            self.print_str(name);
            self.print_char('\n');
        }
    }

    fn cmd_kill(&mut self, pid: usize) {
        match crate::scheduler::kill_task(pid, 130) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("killed task ");
                self.set_fg(FG_OUTPUT);
                self.print_u64(pid as u64);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("kill: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_wait(&mut self, pid: usize) {
        match crate::scheduler::waitpid(0, pid) {
            Ok(code) => {
                self.set_fg(FG_ACCENT);
                self.print_str("reaped ");
                self.set_fg(FG_OUTPUT);
                self.print_u64(pid as u64);
                self.print_str(" exit ");
                self.print_u64(code);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("wait: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_reap(&mut self) {
        let count = crate::scheduler::reap_all_exited(0);
        self.set_fg(FG_ACCENT);
        self.print_str("reaped ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(count as u64);
        self.print_str(" task(s)\n");
    }

    fn cmd_devices(&mut self) {
        crate::device_registry::refresh_pci();
        self.cmd_lines("DEVICES", crate::device_registry::lines());
    }

    fn cmd_sysreport(&mut self, op: Option<&str>) {
        match op {
            Some("write") => match crate::sysreport::write_report() {
                Ok(()) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("wrote ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(crate::sysreport::report_path());
                    self.print_char('\n');
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("sysreport: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err);
                    self.print_char('\n');
                }
            },
            _ => self.cmd_lines("SYSREPORT", crate::sysreport::lines()),
        }
    }

    fn cmd_devkit(&mut self) {
        self.cmd_lines(
            "DEVKIT",
            alloc::vec![
                String::from("coolOS devkit ABI=8"),
                String::from("docs=/SDK/README.TXT"),
                String::from("app_template=/SDK/APP_TEMPLATE.RS"),
                String::from("package_template=/SDK/PACKAGE_TEMPLATE.PKG"),
                String::from("example: exec /bin/devkit"),
            ],
        );
    }

    fn cmd_lines(&mut self, title: &str, lines: Vec<String>) {
        self.set_fg(FG_ACCENT);
        self.print_str(title);
        self.print_char('\n');
        if lines.is_empty() {
            self.set_fg(FG_DIM);
            self.print_str("(none)\n");
            return;
        }
        for line in lines {
            self.set_fg(FG_OUTPUT);
            self.print_str(&line);
            self.print_char('\n');
        }
    }

    fn cmd_recovery(&mut self, args: Vec<&str>) {
        match args.as_slice() {
            ["repair"] => self.cmd_lines("RECOVERY REPAIR", crate::recovery::repair_lines()),
            ["fsck-on-boot", "on"] | ["on"] => {
                self.cmd_lines("RECOVERY", crate::recovery::set_fsck_on_boot(true))
            }
            ["fsck-on-boot", "off"] | ["off"] => {
                self.cmd_lines("RECOVERY", crate::recovery::set_fsck_on_boot(false))
            }
            ["fsck-on-boot"] => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: recovery fsck-on-boot <on|off>\n");
            }
            [other, ..] => {
                self.set_fg(FG_ERROR);
                self.print_str("recovery: unknown op ");
                self.set_fg(FG_OUTPUT);
                self.print_str(other);
                self.print_char('\n');
            }
            [] => self.cmd_lines("RECOVERY", crate::recovery::status_lines()),
        }
    }

    fn cmd_whoami(&mut self) {
        let user = crate::security::current_user();
        let creds = crate::security::current_credentials();
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(creds.uid as u64);
        self.print_str(" gid=");
        self.print_u64(creds.gid as u64);
        self.print_str(" caps=");
        self.print_str(&crate::security::capability_label(creds.caps));
        self.print_str(" home=");
        self.print_str(&user.home);
        self.print_char('\n');
    }

    fn cmd_id(&mut self, user: Option<&str>) {
        let user = match user {
            Some(name) => crate::security::user_by_name(name),
            None => Some(crate::security::current_user()),
        };
        match user {
            Some(user) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_str(" gid=");
                self.print_u64(user.gid as u64);
                self.print_str(" role=");
                self.print_str(&user.role);
                self.print_str(" home=");
                self.print_str(&user.home);
                self.print_str(" login=");
                self.print_str(if user.login_enabled {
                    "enabled"
                } else {
                    "disabled"
                });
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("id: no such user\n");
            }
        }
    }

    fn cmd_groups(&mut self, user: Option<&str>) {
        let name = user
            .map(String::from)
            .unwrap_or_else(|| crate::security::current_user().name);
        match crate::security::groups_for(&name) {
            Some(groups) => {
                self.set_fg(FG_OUTPUT);
                self.print_str(&name);
                self.print_str(":");
                for group in groups {
                    self.print_char(' ');
                    self.print_str(&group.name);
                    self.print_char('(');
                    self.print_u64(group.gid as u64);
                    self.print_char(')');
                }
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("groups: no such user\n");
            }
        }
    }

    fn cmd_login(&mut self, user: Option<&str>, password: Option<&str>) {
        let (Some(user), Some(password)) = (user, password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: login <user> <password>\n");
            return;
        };
        match crate::security::login(user, password) {
            Ok(user) => {
                self.set_fg(FG_ACCENT);
                self.print_str("session user ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("login: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_logout(&mut self) {
        let user = crate::security::logout();
        crate::wm::request_session_lock();
        self.set_fg(FG_ACCENT);
        self.print_str("session user ");
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(user.uid as u64);
        self.print_char('\n');
    }

    fn cmd_lock(&mut self) {
        crate::wm::request_session_lock();
        self.set_fg(FG_ACCENT);
        self.print_str("session locked\n");
    }

    fn cmd_passwd(&mut self, old_password: Option<&str>, new_password: Option<&str>) {
        let (Some(old_password), Some(new_password)) = (old_password, new_password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: passwd <old-password> <new-password>\n");
            return;
        };
        match crate::security::change_password(old_password, new_password) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("password updated\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("passwd: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_setup(&mut self, user: Option<&str>, password: Option<&str>) {
        let (Some(user), Some(password)) = (user, password) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: setup <admin-user> <password>\n");
            return;
        };
        match crate::security::complete_first_run_admin(user, password) {
            Ok(user) => {
                self.set_fg(FG_ACCENT);
                self.print_str("first-run admin ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&user.name);
                self.print_str(" uid=");
                self.print_u64(user.uid as u64);
                self.print_char('\n');
            }
            Err(err) => self.print_account_error("setup", err),
        }
    }

    fn cmd_account(&mut self, args: Vec<&str>) {
        let Some(op) = args.first().copied() else {
            self.print_account_usage();
            return;
        };
        match op {
            "list" | "ls" => self.cmd_lines("ACCOUNTS", crate::security::lines()),
            "add" => {
                let (Some(name), Some(password)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                let role = args.get(3).copied().unwrap_or("user");
                match crate::security::create_user(name, password, role) {
                    Ok(user) => self.print_account_user("added", &user),
                    Err(err) => self.print_account_error("account add", err),
                }
            }
            "enable" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_enabled(name, true) {
                    Ok(user) => self.print_account_user("enabled", &user),
                    Err(err) => self.print_account_error("account enable", err),
                }
            }
            "disable" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_enabled(name, false) {
                    Ok(user) => self.print_account_user("disabled", &user),
                    Err(err) => self.print_account_error("account disable", err),
                }
            }
            "role" => {
                let (Some(name), Some(role)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::set_user_role(name, role) {
                    Ok(user) => self.print_account_user("role", &user),
                    Err(err) => self.print_account_error("account role", err),
                }
            }
            "pass" | "password" => {
                let (Some(name), Some(password)) = (args.get(1), args.get(2)) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::reset_user_password(name, password) {
                    Ok(user) => self.print_account_user("password", &user),
                    Err(err) => self.print_account_error("account pass", err),
                }
            }
            "delete" | "del" | "remove" | "rm" => {
                let Some(name) = args.get(1) else {
                    self.print_account_usage();
                    return;
                };
                match crate::security::delete_user(name) {
                    Ok(user) => self.print_account_user("deleted", &user),
                    Err(err) => self.print_account_error("account delete", err),
                }
            }
            _ => self.print_account_usage(),
        }
    }

    fn print_account_usage(&mut self) {
        self.set_fg(FG_ERROR);
        self.print_str("usage: account list|add <user> <pass> [admin|user]|enable <user>|disable <user>|role <user> <admin|user>|pass <user> <pass>|delete <user>\n");
    }

    fn print_account_user(&mut self, action: &str, user: &crate::security::User) {
        self.set_fg(FG_ACCENT);
        self.print_str("account ");
        self.print_str(action);
        self.print_char(' ');
        self.set_fg(FG_OUTPUT);
        self.print_str(&user.name);
        self.print_str(" uid=");
        self.print_u64(user.uid as u64);
        self.print_str(" role=");
        self.print_str(&user.role);
        self.print_str(" login=");
        self.print_str(if user.login_enabled {
            "enabled"
        } else {
            "disabled"
        });
        self.print_char('\n');
    }

    fn print_account_error(&mut self, label: &str, err: crate::security::AccountError) {
        self.set_fg(FG_ERROR);
        self.print_str(label);
        self.print_str(": ");
        self.set_fg(FG_OUTPUT);
        self.print_str(err.as_str());
        self.print_char('\n');
    }

    fn cmd_umask(&mut self, mode: Option<&str>) {
        match mode {
            Some(mode) => {
                let Some(mode) = crate::security::parse_mode(mode) else {
                    self.set_fg(FG_ERROR);
                    self.print_str("umask: invalid mode\n");
                    return;
                };
                let old = crate::security::set_umask(mode);
                self.set_fg(FG_ACCENT);
                self.print_str("umask ");
                self.set_fg(FG_OUTPUT);
                self.print_str(&crate::security::format_mode(old));
                self.print_str(" -> ");
                self.print_str(&crate::security::format_mode(mode));
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_OUTPUT);
                self.print_str("umask ");
                self.print_str(&crate::security::format_mode(crate::security::umask()));
                self.print_char('\n');
            }
        }
    }

    fn cmd_http(&mut self, scheme: &str, host: &str, path: &str) {
        let result = if host.starts_with("http://") || host.starts_with("https://") {
            crate::net::web_get_response(host)
        } else if scheme == "https" {
            let mut url = String::from("https://");
            url.push_str(host);
            if path.starts_with('/') {
                url.push_str(path);
            } else {
                url.push('/');
                url.push_str(path);
            }
            crate::net::web_get_response(&url)
        } else {
            crate::net::http_get_response(host, path)
        };
        match result {
            Ok(response) => {
                self.set_fg(FG_ACCENT);
                self.print_str("HTTP CLIENT\n");
                self.set_fg(FG_OUTPUT);
                self.print_str(&response.status_line);
                self.print_char('\n');
                self.print_str("resolved ");
                self.print_str(&response.host);
                self.print_str(&response.path);
                self.print_str(" -> ");
                self.print_str(&crate::net::ipv4_string(response.resolved_addr));
                self.print_char('\n');
                if let Some(root) = response.tls_trust_root {
                    self.print_str("tls root ");
                    self.print_str(root);
                    self.print_char('\n');
                }
                if response.redirect_count > 0 {
                    self.print_str("final ");
                    self.print_str(&response.final_url);
                    self.print_str(" redirects=");
                    self.print_u64(response.redirect_count as u64);
                    self.print_char('\n');
                }
                self.print_str(&response.request);
                self.print_str(&response.body);
                self.print_char('\n');
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("http: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn cmd_access(&mut self, key: Option<&str>, value: Option<&str>) {
        match (key, value.and_then(parse_bool_word)) {
            (Some(key), Some(value)) => {
                if crate::accessibility::set(key, value) {
                    self.set_fg(FG_ACCENT);
                    self.print_str("updated accessibility setting\n");
                } else {
                    self.set_fg(FG_ERROR);
                    self.print_str("access: unknown key\n");
                }
            }
            (None, _) => self.cmd_lines("ACCESSIBILITY", crate::accessibility::lines()),
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: access <keyboard_nav|focus_rings|large_text|reduced_motion> <on|off>\n",
                );
            }
        }
    }

    fn cmd_recent(&mut self) {
        let mut lines = Vec::new();
        lines.push(String::from("apps:"));
        lines.extend(crate::app_lifecycle::recent_apps());
        lines.push(String::from("files:"));
        lines.extend(crate::app_lifecycle::recent_files());
        lines.push(String::from("commands:"));
        lines.extend(crate::app_lifecycle::recent_commands());
        lines.push(String::from("searches:"));
        lines.extend(crate::app_lifecycle::recent_searches());
        self.cmd_lines("RECENT", lines);
    }

    fn cmd_startmenu(&mut self, mode: Option<&str>) {
        match mode {
            Some("compact") => {
                crate::app_lifecycle::set_start_menu_compact(true);
                self.print_str("Start menu compact layout enabled\n");
            }
            Some("full") => {
                crate::app_lifecycle::set_start_menu_compact(false);
                self.print_str("Start menu full layout enabled\n");
            }
            Some(_) => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: startmenu [compact|full]\n");
            }
            None => self.cmd_lines("START MENU", crate::app_lifecycle::lines()),
        }
    }

    fn cmd_pinned(&mut self, apps: Vec<&str>) {
        if apps.is_empty() {
            self.cmd_lines("PINNED APPS", crate::app_lifecycle::pinned_apps());
            return;
        }
        crate::app_lifecycle::set_pinned(apps.iter().map(|app| String::from(*app)).collect());
        self.set_fg(FG_ACCENT);
        self.print_str("pinned apps updated\n");
    }

    fn cmd_unpin(&mut self, item: String) {
        if item.is_empty() {
            self.set_fg(FG_ERROR);
            self.print_str("usage: unpin <item>\n");
            return;
        }
        if crate::app_lifecycle::unpin_item(&item) {
            self.print_str("pinned item removed\n");
        } else {
            self.set_fg(FG_WARN);
            self.print_str("pinned item not found\n");
        }
    }

    fn cmd_startup(&mut self, apps: Vec<&str>) {
        if apps.is_empty() {
            self.cmd_lines("STARTUP APPS", crate::app_lifecycle::startup_apps());
            return;
        }
        crate::app_lifecycle::set_startup(apps.iter().map(|app| String::from(*app)).collect());
        self.set_fg(FG_ACCENT);
        self.print_str("startup apps updated\n");
    }

    fn cmd_pkg(&mut self, op: Option<&str>, arg: Option<&str>, args: Vec<&str>) {
        match (op, arg) {
            (None, _) | (Some("list"), _) => self.cmd_lines("PACKAGES", crate::packages::lines()),
            (Some("install"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::install(id));
            }
            (Some("remove"), Some(id)) | (Some("uninstall"), Some(id)) => {
                if !self.require_admin("pkg") {
                    return;
                }
                self.print_result("pkg", crate::packages::uninstall(id));
            }
            (Some("run"), Some(id)) | (Some("launch"), Some(id)) => {
                match crate::packages::launch(id, &args) {
                    Ok(launch) => {
                        self.set_fg(FG_ACCENT);
                        self.print_str("pkg: spawned ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(&launch.exec_path);
                        self.print_str(" pid=");
                        self.print_u64(launch.pid as u64);
                        self.print_char('\n');
                    }
                    Err(err) => {
                        self.set_fg(FG_ERROR);
                        self.print_str("pkg: ");
                        self.set_fg(FG_OUTPUT);
                        self.print_str(err);
                        self.print_char('\n');
                    }
                }
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: pkg [list|install <id>|remove <id>|run <id> [args...]]\n");
            }
        }
    }

    fn cmd_signal(&mut self, pid: Option<&str>, signal: Option<&str>) {
        let Some(target) = pid else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        let Some(signal) = signal.and_then(crate::process_model::Signal::parse) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        if let Some(group_text) = target.strip_prefix('-') {
            let Some(group) = parse_usize(group_text) else {
                self.set_fg(FG_ERROR);
                self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
                return;
            };
            match crate::scheduler::send_signal_to_group(group, signal) {
                Ok(count) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("signal delivered to ");
                    self.set_fg(FG_OUTPUT);
                    self.print_u64(count as u64);
                    self.print_str(" task(s)\n");
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("signal: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }
        let Some(pid) = parse_usize(target) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: signal <pid|-pgid> <term|int|usr1|stop|cont>\n");
            return;
        };
        match crate::scheduler::send_signal(pid, signal) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("signal delivered\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("signal: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_pgroup(&mut self, pid: Option<&str>, group: Option<&str>) {
        let Some(pid) = pid.and_then(parse_usize) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: pgroup <pid> [group]\n");
            return;
        };
        if group.is_none() {
            match crate::scheduler::get_process_group(pid) {
                Ok(group) => {
                    self.set_fg(FG_ACCENT);
                    self.print_str("process group ");
                    self.set_fg(FG_OUTPUT);
                    self.print_u64(group as u64);
                    self.print_char('\n');
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("pgroup: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }
        let Some(group) = group.and_then(parse_usize) else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: pgroup <pid> [group]\n");
            return;
        };
        match crate::scheduler::set_process_group(pid, group) {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("process group updated\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("pgroup: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
            }
        }
    }

    fn cmd_services(&mut self, op: Option<&str>, name: Option<&str>) {
        match (op, name) {
            (None, _) | (Some("list"), _) => self.cmd_lines("SERVICES", crate::services::lines()),
            (Some("run"), _) => {
                if !self.require_admin("services") {
                    return;
                }
                crate::services::supervise_once();
                self.set_fg(FG_ACCENT);
                self.print_str("service supervisor tick\n");
            }
            (Some("start"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::start(name));
            }
            (Some("stop"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::stop(name));
            }
            (Some("fail"), Some(name)) => {
                if !self.require_admin("services") {
                    return;
                }
                self.print_bool("service", crate::services::fail(name));
            }
            (Some(name), None) => match crate::services::status_lines(name) {
                Some(lines) => self.cmd_lines("SERVICE", lines),
                None => {
                    self.set_fg(FG_ERROR);
                    self.print_str("services: no such service\n");
                }
            },
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str(
                    "usage: services [list|run|<name>|start <name>|stop <name>|fail <name>]\n",
                );
            }
        }
    }

    fn cmd_job(&mut self, args: Vec<&str>) {
        let Some(op) = args.first().copied() else {
            self.set_fg(FG_ERROR);
            self.print_str(
                "usage: job run <path> [args...] | job <cancel|pause|resume> <id|last>\n",
            );
            return;
        };
        if op == "run" {
            let Some(path) = args.get(1).copied() else {
                self.set_fg(FG_ERROR);
                self.print_str("usage: job run <path> [args...]\n");
                return;
            };
            let exec_args: Vec<&str> = args.iter().skip(2).copied().collect();
            let abs = resolve_path(&self.cwd, path);
            match crate::elf::spawn_elf_process_suspended_with_args(&abs, &exec_args) {
                Ok(pid) => {
                    if self.configure_process_tty(pid, pid) {
                        let job = crate::jobs::start_process("Process", &abs, pid);
                        self.set_fg(FG_ACCENT);
                        self.print_str("job #");
                        self.set_fg(FG_OUTPUT);
                        self.print_u64(job);
                        self.print_str(" pid=");
                        self.print_u64(pid as u64);
                        self.print_str(" tty=");
                        self.print_u64(self.tty_id);
                        self.print_char('\n');
                        crate::scheduler::unblock(pid);
                    } else {
                        let _ = crate::scheduler::kill_task(pid, 143);
                    }
                }
                Err(err) => {
                    self.set_fg(FG_ERROR);
                    self.print_str("job: ");
                    self.set_fg(FG_OUTPUT);
                    self.print_str(err.as_str());
                    self.print_char('\n');
                }
            }
            return;
        }

        let Some(id_text) = args.get(1).copied() else {
            self.set_fg(FG_ERROR);
            self.print_str("usage: job <cancel|pause|resume> <id|last>\n");
            return;
        };
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("job: no such job\n");
            return;
        };
        let ok = match op {
            "cancel" => crate::jobs::cancel(id),
            "pause" => crate::jobs::pause(id),
            "resume" => crate::jobs::resume(id),
            _ => false,
        };
        self.print_bool("job", ok);
    }

    fn configure_process_tty(&mut self, pid: usize, group: usize) -> bool {
        if let Err(err) = crate::scheduler::set_process_group(pid, group) {
            self.set_fg(FG_ERROR);
            self.print_str("tty: setpgid failed: ");
            self.set_fg(FG_OUTPUT);
            self.print_str(err.as_str());
            self.print_char('\n');
            return false;
        }
        if let Err(err) = crate::scheduler::set_task_tty(pid, Some(self.tty_id)) {
            self.set_fg(FG_ERROR);
            self.print_str("tty: attach failed: ");
            self.set_fg(FG_OUTPUT);
            self.print_str(err.as_str());
            self.print_char('\n');
            return false;
        }
        true
    }

    fn begin_foreground(&mut self, pid: usize, group: usize, job_id: Option<u64>, title: &str) {
        crate::tty::set_foreground_group(self.tty_id, Some(group));
        self.foreground_job = Some(ForegroundJob {
            group,
            pid,
            job_id,
            title: String::from(title),
        });
        self.set_fg(FG_ACCENT);
        self.print_str("foreground ");
        self.set_fg(FG_OUTPUT);
        self.print_str(title);
        self.print_str(" pid=");
        self.print_u64(pid as u64);
        self.print_str(" pgid=");
        self.print_u64(group as u64);
        self.print_str(" tty=");
        self.print_u64(self.tty_id);
        self.print_char('\n');
    }

    fn cmd_tty(&mut self) {
        self.set_fg(FG_ACCENT);
        self.print_str("tty #");
        self.set_fg(FG_OUTPUT);
        self.print_u64(self.tty_id);
        self.print_str(" foreground pgid=");
        match crate::tty::foreground_group(self.tty_id) {
            Some(group) => self.print_u64(group as u64),
            None => self.print_char('-'),
        }
        let active = self
            .foreground_job
            .as_ref()
            .map(|job| (job.pid, job.job_id));
        if let Some((pid, job_id)) = active {
            self.print_str(" pid=");
            self.print_u64(pid as u64);
            if let Some(job_id) = job_id {
                self.print_str(" job #");
                self.print_u64(job_id);
            }
        }
        self.print_char('\n');
        for line in crate::tty::lines() {
            self.set_fg(FG_DIM);
            self.print_str(&line);
            self.print_char('\n');
        }
    }

    fn cmd_fg(&mut self, id_text: Option<&str>) {
        if self.foreground_job.is_some() {
            self.set_fg(FG_ERROR);
            self.print_str("fg: terminal already has a foreground job\n");
            return;
        }
        let id_text = id_text.unwrap_or("last");
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("fg: no such job\n");
            return;
        };
        let Some(pid) = crate::jobs::process_id(id) else {
            self.set_fg(FG_ERROR);
            self.print_str("fg: job has no process\n");
            return;
        };
        let group = crate::scheduler::get_process_group(pid).unwrap_or(pid);
        if crate::scheduler::set_task_tty(pid, Some(self.tty_id)).is_err() {
            self.set_fg(FG_ERROR);
            self.print_str("fg: could not attach tty\n");
            return;
        }
        let _ = crate::jobs::resume(id);
        self.begin_foreground(pid, group, Some(id), "job");
    }

    fn cmd_bg(&mut self, id_text: Option<&str>) {
        let id_text = id_text.unwrap_or("last");
        let Some(id) = parse_job_id(id_text) else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: no such job\n");
            return;
        };
        let Some(pid) = crate::jobs::process_id(id) else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: job has no process\n");
            return;
        };
        let _ = crate::scheduler::set_task_tty(pid, Some(self.tty_id));
        if crate::jobs::resume(id) {
            self.set_fg(FG_ACCENT);
            self.print_str("background job #");
            self.set_fg(FG_OUTPUT);
            self.print_u64(id);
            self.print_str(" pid=");
            self.print_u64(pid as u64);
            self.print_char('\n');
        } else {
            self.set_fg(FG_ERROR);
            self.print_str("bg: resume failed\n");
        }
    }

    fn cmd_notify(&mut self, op: Option<&str>, arg: Option<&str>) {
        match (op, arg) {
            (None, _) | (Some("history"), _) => self.cmd_lines(
                "NOTIFICATION HISTORY",
                crate::notifications::history_lines(),
            ),
            (Some("dismiss"), Some(id)) => {
                let ok = parse_u64(id)
                    .map(crate::notifications::dismiss)
                    .unwrap_or(false);
                self.print_bool("notify", ok);
            }
            (Some("group"), Some(title)) => {
                let count = crate::notifications::dismiss_group(title);
                self.set_fg(FG_ACCENT);
                self.print_str("dismissed ");
                self.print_u64(count as u64);
                self.print_str(" notification(s)\n");
            }
            (Some("clear"), _) => {
                crate::notifications::clear();
                self.set_fg(FG_ACCENT);
                self.print_str("notifications cleared\n");
            }
            _ => {
                self.set_fg(FG_ERROR);
                self.print_str("usage: notify [history|dismiss <id>|group <title>|clear]\n");
            }
        }
    }

    fn print_result(&mut self, prefix: &str, result: Result<(), &'static str>) {
        match result {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str(prefix);
                self.print_str(": ok\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str(prefix);
                self.print_str(": ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn print_bool(&mut self, prefix: &str, ok: bool) {
        if ok {
            self.set_fg(FG_ACCENT);
            self.print_str(prefix);
            self.print_str(": ok\n");
        } else {
            self.set_fg(FG_ERROR);
            self.print_str(prefix);
            self.print_str(": not found\n");
        }
    }

    fn require_admin(&mut self, prefix: &str) -> bool {
        match crate::security::require_admin() {
            Ok(()) => true,
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str(prefix);
                self.print_str(": ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err.as_str());
                self.print_char('\n');
                false
            }
        }
    }

    fn cmd_power(&mut self, op: Option<&str>) {
        match op {
            Some("reboot") => crate::acpi::reboot(),
            Some("shutdown") => self.print_power_result(crate::acpi::shutdown()),
            Some("sleep") => self.print_power_result(crate::acpi::sleep()),
            _ => self.cmd_lines("POWER", crate::acpi::status_lines()),
        }
    }

    fn print_power_result(&mut self, result: Result<(), &'static str>) {
        match result {
            Ok(()) => {
                self.set_fg(FG_ACCENT);
                self.print_str("power operation requested\n");
            }
            Err(err) => {
                self.set_fg(FG_ERROR);
                self.print_str("power: ");
                self.set_fg(FG_OUTPUT);
                self.print_str(err);
                self.print_char('\n');
            }
        }
    }

    fn cmd_log(&mut self) {
        let _ = crate::klog::flush_to_disk();
        self.cmd_lines("KERNEL LOG", crate::klog::lines());
    }

    fn cmd_fsck(&mut self) {
        match crate::coolfs::check() {
            Some(report) => {
                self.set_fg(if report.ok { FG_ACCENT } else { FG_WARN });
                self.print_str(if report.ok {
                    "coolfs root ok\n"
                } else {
                    "coolfs root warning\n"
                });
                self.set_fg(FG_OUTPUT);
                self.print_str("root entries ");
                self.print_u64(report.root_entries as u64);
                self.print_str("  blocks ");
                self.print_u64(report.stats.used_blocks as u64);
                self.print_char('/');
                self.print_u64(report.stats.total_blocks as u64);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_ERROR);
                self.print_str("coolfs: unable to read root filesystem\n");
            }
        }
        match crate::fat32::check() {
            Some(report) => {
                self.set_fg(if report.ok { FG_ACCENT } else { FG_WARN });
                self.print_str(if report.ok {
                    "legacy fat32 ok\n"
                } else {
                    "legacy fat32 warning\n"
                });
                self.set_fg(FG_OUTPUT);
                self.print_str("root entries ");
                self.print_u64(report.root_entries as u64);
                self.print_str("  clusters ");
                self.print_u64(report.stats.used_clusters as u64);
                self.print_char('/');
                self.print_u64(report.stats.total_clusters as u64);
                self.print_char('\n');
            }
            None => {
                self.set_fg(FG_WARN);
                self.print_str("legacy fat32: unavailable\n");
            }
        }
    }

    fn cmd_df(&mut self) {
        self.set_fg(FG_ACCENT);
        self.print_str("Filesystem  Used  Free  Total\n");
        self.set_fg(FG_OUTPUT);
        if let Some(cool) = crate::coolfs::stats() {
            let cool_used = cool.used_blocks as usize * cool.block_size as usize;
            let cool_free = cool.free_blocks as usize * cool.block_size as usize;
            let cool_total = cool.total_blocks as usize * cool.block_size as usize;
            self.print_str("coolfs:/    ");
            self.print_size(cool_used);
            self.print_str("  ");
            self.print_size(cool_free);
            self.print_str("  ");
            self.print_size(cool_total);
            self.print_char('\n');
        }
        if let Some(stats) = crate::fat32::stats() {
            let free = stats.free_clusters as usize * stats.bytes_per_cluster as usize;
            let used = stats.used_clusters as usize * stats.bytes_per_cluster as usize;
            let total = stats.total_clusters as usize * stats.bytes_per_cluster as usize;
            self.print_str("fat32:/FAT  ");
            self.print_size(used);
            self.print_str("  ");
            self.print_size(free);
            self.print_str("  ");
            self.print_size(total);
            self.print_char('\n');
        }
    }

    fn cmd_info(&mut self) {
        let heap_used = crate::allocator::heap_used();
        let heap_total = crate::allocator::HEAP_SIZE;
        let task_count = crate::scheduler::SCHEDULER.lock().tasks.len();

        self.set_fg(FG_ACCENT);
        self.print_str("Heap  : ");
        self.set_fg(FG_OUTPUT);
        self.print_size(heap_used);
        self.set_fg(FG_DIM);
        self.print_str(" / ");
        self.set_fg(FG_OUTPUT);
        self.print_size(heap_total);
        self.print_char('\n');

        self.set_fg(FG_ACCENT);
        self.print_str("Tasks : ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(task_count as u64);
        self.print_char('\n');

        let cpuid = raw_cpuid::CpuId::new();
        if let Some(v) = cpuid.get_vendor_info() {
            self.set_fg(FG_ACCENT);
            self.print_str("CPU   : ");
            self.set_fg(FG_OUTPUT);
            self.print_str(v.as_str());
            self.print_char('\n');
        }
        if let Some(b) = cpuid.get_processor_brand_string() {
            self.set_fg(FG_ACCENT);
            self.print_str("Brand : ");
            self.set_fg(FG_OUTPUT);
            self.print_str(b.as_str().trim());
            self.print_char('\n');
        }

        self.set_fg(FG_ACCENT);
        self.print_str("CWD   : ");
        self.set_fg(FG_DIR);
        let cwd = self.cwd.clone();
        self.print_str(&cwd);
        self.print_char('\n');
    }

    fn cmd_uptime(&mut self) {
        let ticks = crate::interrupts::ticks();
        let secs = crate::interrupts::uptime_secs();
        let mins = secs / 60;
        let hours = mins / 60;
        let s = secs % 60;
        let m = mins % 60;

        self.set_fg(FG_ACCENT);
        self.print_str("Up: ");
        self.set_fg(FG_OUTPUT);
        self.print_u64(hours);
        self.print_char(':');
        if m < 10 {
            self.print_char('0');
        }
        self.print_u64(m);
        self.print_char(':');
        if s < 10 {
            self.print_char('0');
        }
        self.print_u64(s);
        self.set_fg(FG_DIM);
        self.print_str("  (");
        self.print_u64(ticks);
        self.print_str(" ticks)\n");
    }

    // ── Rendering helpers ─────────────────────────────────────────────────────

    pub fn print_char(&mut self, c: char) {
        mirror_debug_char(c);
        self.refresh_layout();
        if c == '\r' {
            self.col = 0;
            return;
        }
        if c == '\n' {
            self.col = 0;
            self.advance_row();
            return;
        }
        if c == '\u{0008}' {
            if self.col > 0 {
                self.col -= 1;
                self.draw_char_at(self.col, self.row, ' ');
            }
            return;
        }
        if c == '\t' {
            let spaces = 4 - (self.col % 4);
            for _ in 0..spaces {
                self.print_char(' ');
            }
            return;
        }
        if c.is_control() {
            return;
        }
        if self.col >= self.cols {
            self.col = 0;
            self.advance_row();
        }
        self.draw_char_at(self.col, self.row, c);
        self.col += 1;
    }

    pub fn print_str(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }

    fn print_u64(&mut self, mut n: u64) {
        if n == 0 {
            self.print_char('0');
            return;
        }
        let mut buf = [0u8; 20];
        let mut i = 20usize;
        while n > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        for &b in &buf[i..] {
            self.print_char(b as char);
        }
    }

    fn print_size(&mut self, bytes: usize) {
        if bytes >= 1024 * 1024 {
            self.print_u64((bytes / (1024 * 1024)) as u64);
            self.print_str(" MB");
        } else if bytes >= 1024 {
            self.print_u64((bytes / 1024) as u64);
            self.print_str(" KB");
        } else {
            self.print_u64(bytes as u64);
            self.print_str(" B");
        }
    }

    fn advance_row(&mut self) {
        self.row += 1;
        if self.row >= self.rows {
            self.scroll_up();
            self.row = self.rows - 1;
        }
    }

    fn scroll_up(&mut self) {
        let stride = self.window.width as usize;
        let text_x = TERM_PAD_X;
        let text_y = TERM_PAD_Y;
        let text_w = self.cols * CHAR_W;
        let text_h = self.rows * LINE_H;

        if text_w == 0 || text_h <= LINE_H {
            return;
        }

        for y in 0..(text_h - LINE_H) {
            let dst_row = text_y + y;
            let src_row = dst_row + LINE_H;
            let dst = dst_row * stride + text_x;
            let src = src_row * stride + text_x;
            self.window.buf.copy_within(src..src + text_w, dst);
        }

        for y in (text_h - LINE_H)..text_h {
            let py = text_y + y;
            let row_start = py * stride + text_x;
            for x in 0..text_w {
                self.window.buf[row_start + x] = Self::bg_at(py);
            }
        }
        self.window
            .mark_dirty(text_x as i32, text_y as i32, text_w as i32, text_h as i32);
    }

    fn draw_char_at(&mut self, col: usize, row: usize, c: char) {
        let glyph = font8x8::BASIC_FONTS
            .get(c)
            .unwrap_or_else(|| font8x8::BASIC_FONTS.get(' ').unwrap());
        let px0 = TERM_PAD_X + col * CHAR_W;
        let py0 = TERM_PAD_Y + row * LINE_H + GLYPH_Y_INSET;
        let stride = self.window.width as usize;
        let large_text = crate::accessibility::snapshot().large_text;
        for (gy, &byte) in glyph.iter().take(CHAR_H).enumerate() {
            for bit in 0..8usize {
                let px = px0 + bit;
                let py = py0 + gy;
                let idx = py * stride + px;
                if idx < self.window.buf.len() {
                    let ink = byte & (1 << bit) != 0;
                    self.window.buf[idx] = if ink { self.fg } else { Self::bg_at(py) };
                    if large_text && ink && idx + 1 < self.window.buf.len() {
                        self.window.buf[idx + 1] = self.fg;
                    }
                }
            }
        }
        self.window
            .mark_dirty(px0 as i32, py0 as i32, CHAR_W as i32 + 1, CHAR_H as i32);
    }

    fn set_fg(&mut self, color: u32) {
        self.fg = color;
    }

    fn print_prompt(&mut self) {
        self.set_fg(FG_PROMPT);
        self.print_str("cool");
        self.set_fg(FG_ACCENT);
        self.print_str("> ");
        self.set_fg(FG_INPUT);
        self.input_start_col = self.col;
    }

    fn fill_background(&mut self) {
        let stride = self.window.width as usize;
        for (idx, pixel) in self.window.buf.iter_mut().enumerate() {
            let py = idx / stride;
            *pixel = Self::bg_at(py);
        }
        self.window.mark_dirty_all();
    }

    fn paint_exposed_background(&mut self, old_width: usize, old_content_h: usize) {
        let new_width = self.window.width.max(0) as usize;
        let new_content_h = (self.window.height - TITLE_H).max(0) as usize;
        let shared_h = old_content_h.min(new_content_h);

        if new_width > old_width {
            let fill_w = new_width - old_width;
            for py in 0..shared_h {
                let row_start = py * new_width + old_width;
                let row_end = row_start + fill_w;
                for idx in row_start..row_end {
                    self.window.buf[idx] = Self::bg_at(py);
                }
            }
        }

        if new_content_h > old_content_h {
            for py in old_content_h..new_content_h {
                let row_start = py * new_width;
                for idx in row_start..row_start + new_width {
                    self.window.buf[idx] = Self::bg_at(py);
                }
            }
        }
        self.window.mark_dirty_all();
    }

    fn refresh_layout(&mut self) {
        self.cols = text_cols(self.window.width as usize);
        self.rows = text_rows((self.window.height - TITLE_H).max(0) as usize);
        self.col = self.col.min(self.cols.saturating_sub(1));
        self.row = self.row.min(self.rows.saturating_sub(1));
        self.input_start_col = self.input_start_col.min(self.cols.saturating_sub(1));
    }

    fn bg_at(py: usize) -> u32 {
        match py % 6 {
            0 => TERM_BG_C,
            1 | 2 => TERM_BG_A,
            _ => TERM_BG_B,
        }
    }
}

fn mirror_debug_char(c: char) {
    if !DEBUG_MIRROR.load(Ordering::Acquire) {
        return;
    }
    if c == '\n' {
        debug_byte(b'\n');
    } else if c.is_ascii() && !c.is_control() {
        debug_byte(c as u8);
    } else if !c.is_control() {
        debug_byte(b'?');
    }
}

fn debug_byte(byte: u8) {
    unsafe {
        x86_64::instructions::port::Port::<u8>::new(0xE9).write(byte);
    }
}

// ── Path utilities ────────────────────────────────────────────────────────────

fn text_cols(width: usize) -> usize {
    (width.saturating_sub(TERM_PAD_X * 2) / CHAR_W).max(1)
}

fn text_rows(content_h: usize) -> usize {
    (content_h.saturating_sub(TERM_PAD_Y * 2) / LINE_H).max(1)
}

fn resolve_path(cwd: &str, input: &str) -> String {
    if input.starts_with('/') {
        normalize_path(input)
    } else {
        let mut base = String::from(cwd);
        if !base.ends_with('/') {
            base.push('/');
        }
        base.push_str(input);
        normalize_path(&base)
    }
}

fn parse_usize(input: &str) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0usize;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(out)
}

fn parse_u32(input: &str) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0u32;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(out)
}

fn parse_u64(input: &str) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0u64;
    for b in input.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(out)
}

fn parse_job_id(input: &str) -> Option<u64> {
    match input {
        "last" | "latest" => crate::jobs::latest_id(),
        _ => parse_u64(input),
    }
}

fn parse_owner(input: &str) -> Option<(u32, u32)> {
    if let Some((uid, gid)) = input.split_once(':') {
        return Some((parse_u32(uid)?, parse_u32(gid)?));
    }
    let uid = parse_u32(input)?;
    Some((uid, uid))
}

fn parse_bool_word(input: &str) -> Option<bool> {
    match input {
        "on" | "1" | "true" | "yes" => Some(true),
        "off" | "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn collect_words<'a, I>(words: I) -> String
where
    I: Iterator<Item = &'a str>,
{
    let mut out = String::new();
    for word in words {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn diagnostics_lines() -> Vec<String> {
    let mut lines = Vec::new();
    push_terminal_section(&mut lines, "kernel", crate::klog::lines());
    push_terminal_section(&mut lines, "profiler", crate::profiler::lines());
    push_terminal_section(&mut lines, "services", crate::services::lines());
    push_terminal_section(
        &mut lines,
        "compositor",
        crate::wm::compositor::compositor_lines(),
    );
    push_terminal_section(&mut lines, "heap", crate::allocator::heap_lines());
    push_terminal_section(&mut lines, "slab", crate::slab::lines());
    push_terminal_section(
        &mut lines,
        "filesystem",
        crate::fs_hardening::status_lines(),
    );
    push_terminal_section(&mut lines, "vfs", crate::vfs::mount_lines());
    push_terminal_section(&mut lines, "config", crate::config_store::lines());
    push_terminal_section(&mut lines, "settings", crate::settings_state::lines());
    push_terminal_section(&mut lines, "crash", crate::crashdump::detailed_lines());
    lines
}

fn push_terminal_section(out: &mut Vec<String>, name: &str, lines: Vec<String>) {
    let mut header = String::from("== ");
    header.push_str(name);
    header.push_str(" ==");
    out.push(header);
    if lines.is_empty() {
        out.push(String::from("(none)"));
    } else {
        out.extend(lines);
    }
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/').filter(|s| !s.is_empty()) {
        match component {
            ".." => {
                parts.pop();
            }
            "." => {}
            seg => parts.push(seg),
        }
    }
    if parts.is_empty() {
        return String::from("/");
    }
    let mut result = String::from("/");
    for (i, &part) in parts.iter().enumerate() {
        if i > 0 {
            result.push('/');
        }
        result.push_str(part);
    }
    result
}
