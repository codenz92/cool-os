extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    mem,
    sync::atomic::{AtomicU64, Ordering},
};
use spin::Mutex;

const TTY_BUFFER_SIZE: usize = 8192;
const TTY_INPUT_SIZE: usize = 4096;
const TTY_LINE_LIMIT: usize = 1024;
const TTY_READ_QUEUE: &str = "tty-read";

pub const TTY_MODE_CANONICAL: u64 = 1 << 0;
pub const TTY_MODE_ECHO: u64 = 1 << 1;
pub const TTY_MODE_SIGNALS: u64 = 1 << 2;
pub const TTY_MODE_DEFAULT: u64 = TTY_MODE_CANONICAL | TTY_MODE_ECHO | TTY_MODE_SIGNALS;
pub const TTY_MODE_MASK: u64 = TTY_MODE_DEFAULT;

pub const TTY_CTL_GET_MODE: u64 = 0;
pub const TTY_CTL_SET_MODE: u64 = 1;
pub const TTY_CTL_GET_SIZE: u64 = 2;
pub const TTY_CTL_SET_SIZE: u64 = 3;

struct Tty {
    id: u64,
    foreground_group: Option<usize>,
    buffer: Vec<u8>,
    head: usize,
    tail: usize,
    dropped: u64,
    input: Vec<u8>,
    input_head: usize,
    input_tail: usize,
    input_dropped: u64,
    line: Vec<u8>,
    waiting_readers: Vec<usize>,
    eof_pending: bool,
    mode: u64,
    cols: u16,
    rows: u16,
}

impl Tty {
    fn new(id: u64) -> Self {
        let mut buffer = Vec::new();
        buffer.resize(TTY_BUFFER_SIZE, 0);
        let mut input = Vec::new();
        input.resize(TTY_INPUT_SIZE, 0);
        Tty {
            id,
            foreground_group: None,
            buffer,
            head: 0,
            tail: 0,
            dropped: 0,
            input,
            input_head: 0,
            input_tail: 0,
            input_dropped: 0,
            line: Vec::new(),
            waiting_readers: Vec::new(),
            eof_pending: false,
            mode: TTY_MODE_DEFAULT,
            cols: 80,
            rows: 25,
        }
    }

    fn push_byte(&mut self, byte: u8) {
        let next = (self.head + 1) % self.buffer.len();
        if next == self.tail {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        self.buffer[self.head] = byte;
        self.head = next;
    }

    fn pop_byte(&mut self) -> Option<u8> {
        if self.tail == self.head {
            return None;
        }
        let byte = self.buffer[self.tail];
        self.tail = (self.tail + 1) % self.buffer.len();
        Some(byte)
    }

    fn pending_bytes(&self) -> usize {
        if self.head >= self.tail {
            self.head - self.tail
        } else {
            self.buffer.len() - self.tail + self.head
        }
    }

    fn push_input_byte(&mut self, byte: u8) {
        let next = (self.input_head + 1) % self.input.len();
        if next == self.input_tail {
            self.input_dropped = self.input_dropped.saturating_add(1);
            return;
        }
        self.input[self.input_head] = byte;
        self.input_head = next;
    }

    fn push_raw_input(&mut self, bytes: &[u8]) -> Vec<usize> {
        for &byte in bytes {
            self.push_input_byte(byte);
            if self.mode & TTY_MODE_ECHO != 0 {
                self.push_byte(byte);
            }
        }
        self.take_readers()
    }

    fn pop_input_byte(&mut self) -> Option<u8> {
        if self.input_tail == self.input_head {
            return None;
        }
        let byte = self.input[self.input_tail];
        self.input_tail = (self.input_tail + 1) % self.input.len();
        Some(byte)
    }

    fn pending_input_bytes(&self) -> usize {
        if self.input_head >= self.input_tail {
            self.input_head - self.input_tail
        } else {
            self.input.len() - self.input_tail + self.input_head
        }
    }

    fn add_reader(&mut self, task_id: usize) {
        crate::evented::add_waiter(&mut self.waiting_readers, task_id);
    }

    fn remove_reader(&mut self, task_id: usize) {
        crate::evented::remove_waiter(&mut self.waiting_readers, task_id);
    }

    fn take_readers(&mut self) -> Vec<usize> {
        mem::take(&mut self.waiting_readers)
    }

    fn flush_line(&mut self, newline: bool) -> Vec<usize> {
        for idx in 0..self.line.len() {
            self.push_input_byte(self.line[idx]);
        }
        self.line.clear();
        if newline {
            self.push_input_byte(b'\n');
        }
        self.take_readers()
    }

    fn remove_last_line_char(&mut self) -> bool {
        while let Some(byte) = self.line.pop() {
            if byte & 0b1100_0000 != 0b1000_0000 {
                return true;
            }
        }
        false
    }
}

static NEXT_TTY_ID: AtomicU64 = AtomicU64::new(1);
static TTYS: Mutex<Vec<Tty>> = Mutex::new(Vec::new());

pub fn create() -> u64 {
    let id = NEXT_TTY_ID.fetch_add(1, Ordering::Relaxed);
    TTYS.lock().push(Tty::new(id));
    crate::event_bus::emit("tty", "create", "terminal");
    id
}

pub fn destroy(id: u64) -> bool {
    let mut ttys = TTYS.lock();
    let Some(pos) = ttys.iter().position(|tty| tty.id == id) else {
        return false;
    };
    ttys.remove(pos);
    true
}

pub fn write(id: u64, bytes: &[u8]) -> usize {
    let mut ttys = TTYS.lock();
    let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
        return 0;
    };
    for &byte in bytes {
        tty.push_byte(byte);
    }
    bytes.len()
}

pub fn submit_char(id: u64, c: char) -> bool {
    let mut encoded = [0u8; 4];
    let bytes = c.encode_utf8(&mut encoded).as_bytes();
    let wake_tasks = {
        let mut ttys = TTYS.lock();
        let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
            return false;
        };
        if tty.mode & TTY_MODE_CANONICAL == 0 {
            tty.push_raw_input(bytes)
        } else if c == '\n' || c == '\r' {
            tty.push_byte(b'\n');
            tty.flush_line(true)
        } else if c == '\u{0008}' || c == '\u{007F}' {
            if tty.remove_last_line_char() {
                tty.push_byte(8);
                tty.push_byte(b' ');
                tty.push_byte(8);
            }
            Vec::new()
        } else if c == '\t' || !c.is_control() {
            if tty.line.len().saturating_add(bytes.len()) <= TTY_LINE_LIMIT {
                for &byte in bytes {
                    tty.line.push(byte);
                    tty.push_byte(byte);
                }
            }
            Vec::new()
        } else {
            Vec::new()
        }
    };
    wake_readers(wake_tasks);
    crate::wm::request_repaint();
    true
}

pub fn submit_bytes(id: u64, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    let wake_tasks = {
        let mut ttys = TTYS.lock();
        let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
            return false;
        };
        if tty.mode & TTY_MODE_CANONICAL == 0 {
            tty.push_raw_input(bytes)
        } else {
            let mut wake_tasks = Vec::new();
            for &byte in bytes {
                let c = byte as char;
                if c == '\n' || c == '\r' {
                    tty.push_byte(b'\n');
                    wake_tasks = tty.flush_line(true);
                } else if byte == b'\x08' || byte == b'\x7f' {
                    if tty.remove_last_line_char() {
                        tty.push_byte(8);
                        tty.push_byte(b' ');
                        tty.push_byte(8);
                    }
                } else if c == '\t' || !c.is_control() {
                    if tty.line.len().saturating_add(1) <= TTY_LINE_LIMIT {
                        tty.line.push(byte);
                        tty.push_byte(byte);
                    }
                }
            }
            wake_tasks
        }
    };
    wake_readers(wake_tasks);
    crate::wm::request_repaint();
    true
}

pub fn submit_enter(id: u64) -> bool {
    submit_char(id, '\n')
}

pub fn submit_backspace(id: u64) -> bool {
    submit_char(id, '\u{0008}')
}

pub fn submit_eof(id: u64) -> bool {
    let wake_tasks = {
        let mut ttys = TTYS.lock();
        let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
            return false;
        };
        if tty.line.is_empty() {
            tty.eof_pending = true;
            tty.take_readers()
        } else {
            tty.flush_line(false)
        }
    };
    wake_readers(wake_tasks);
    crate::wm::request_repaint();
    true
}

pub fn read_input_blocking(id: u64, buf: &mut [u8], len: usize) -> usize {
    let max = len.min(buf.len());
    if max == 0 {
        return 0;
    }

    loop {
        let task_id = crate::scheduler::current_task_id();
        let read_result = {
            let mut ttys = TTYS.lock();
            let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
                return usize::MAX;
            };
            let mut n = 0usize;
            let canonical = tty.mode & TTY_MODE_CANONICAL != 0;
            while n < max {
                let Some(byte) = tty.pop_input_byte() else {
                    break;
                };
                buf[n] = byte;
                n += 1;
                if canonical && byte == b'\n' {
                    break;
                }
            }
            if n > 0 {
                Some(n)
            } else if tty.eof_pending {
                tty.eof_pending = false;
                Some(0)
            } else {
                tty.add_reader(task_id);
                None
            }
        };

        if let Some(n) = read_result {
            return n;
        }

        crate::wait_queue::wait(TTY_READ_QUEUE, crate::scheduler::current_task_id());
        crate::scheduler::block_current();
        while crate::scheduler::current_task_blocked() {
            unsafe {
                core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
            }
        }
        x86_64::instructions::interrupts::disable();
    }
}

pub fn input_readiness(id: u64) -> u64 {
    let ttys = TTYS.lock();
    let Some(tty) = ttys.iter().find(|tty| tty.id == id) else {
        return crate::evented::EVENT_ERROR;
    };
    if tty.pending_input_bytes() > 0 || tty.eof_pending {
        crate::evented::EVENT_READ
    } else {
        0
    }
}

pub fn register_input_waiter(id: u64, task_id: usize) -> bool {
    let mut ttys = TTYS.lock();
    let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
        return false;
    };
    tty.add_reader(task_id);
    true
}

pub fn unregister_input_waiter(id: u64, task_id: usize) {
    let mut ttys = TTYS.lock();
    if let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) {
        tty.remove_reader(task_id);
    }
}

pub fn input_mode(id: u64) -> Option<u64> {
    TTYS.lock()
        .iter()
        .find(|tty| tty.id == id)
        .map(|tty| tty.mode)
}

pub fn signals_enabled(id: u64) -> bool {
    input_mode(id)
        .map(|mode| mode & TTY_MODE_SIGNALS != 0)
        .unwrap_or(true)
}

pub fn set_input_mode(id: u64, mode: u64) -> Option<u64> {
    let mut ttys = TTYS.lock();
    let tty = ttys.iter_mut().find(|tty| tty.id == id)?;
    let previous = tty.mode;
    tty.mode = mode & TTY_MODE_MASK;
    Some(previous)
}

pub fn reset_input_mode(id: u64) -> bool {
    set_input_mode(id, TTY_MODE_DEFAULT).is_some()
}

pub fn size(id: u64) -> Option<(u16, u16)> {
    TTYS.lock()
        .iter()
        .find(|tty| tty.id == id)
        .map(|tty| (tty.cols, tty.rows))
}

pub fn set_size(id: u64, cols: u16, rows: u16) -> bool {
    if cols == 0 || rows == 0 {
        return false;
    }
    let mut ttys = TTYS.lock();
    let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
        return false;
    };
    tty.cols = cols;
    tty.rows = rows;
    true
}

pub fn control(id: u64, op: u64, arg1: u64, arg2: u64) -> Option<u64> {
    match op {
        TTY_CTL_GET_MODE => input_mode(id),
        TTY_CTL_SET_MODE => set_input_mode(id, arg1),
        TTY_CTL_GET_SIZE => size(id).map(|(cols, rows)| cols as u64 | ((rows as u64) << 16)),
        TTY_CTL_SET_SIZE => {
            let cols = u16::try_from(arg1).ok()?;
            let rows = u16::try_from(arg2).ok()?;
            if set_size(id, cols, rows) {
                Some(cols as u64 | ((rows as u64) << 16))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn pop_output_byte(id: u64) -> Option<u8> {
    TTYS.lock()
        .iter_mut()
        .find(|tty| tty.id == id)
        .and_then(Tty::pop_byte)
}

pub fn set_foreground_group(id: u64, group: Option<usize>) -> bool {
    let mut ttys = TTYS.lock();
    let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
        return false;
    };
    tty.foreground_group = group;
    crate::wm::request_repaint();
    true
}

pub fn foreground_group(id: u64) -> Option<usize> {
    TTYS.lock()
        .iter()
        .find(|tty| tty.id == id)
        .and_then(|tty| tty.foreground_group)
}

pub fn lines() -> Vec<String> {
    let ttys = TTYS.lock();
    if ttys.is_empty() {
        return alloc::vec![String::from("no terminals")];
    }
    ttys.iter()
        .map(|tty| {
            format!(
                "tty #{} foreground={} mode={} size={}x{} output={} input={} dropped={}/{}",
                tty.id,
                tty.foreground_group
                    .map(|group| group.to_string())
                    .unwrap_or_else(|| String::from("-")),
                mode_name(tty.mode),
                tty.cols,
                tty.rows,
                tty.pending_bytes(),
                tty.pending_input_bytes(),
                tty.dropped,
                tty.input_dropped
            )
        })
        .collect()
}

pub fn selftest_passes() -> bool {
    let id = create();
    let mut input = [0u8; 8];
    let passed = set_foreground_group(id, Some(42))
        && foreground_group(id) == Some(42)
        && write(id, b"ok") == 2
        && pop_output_byte(id) == Some(b'o')
        && pop_output_byte(id) == Some(b'k')
        && pop_output_byte(id).is_none()
        && submit_char(id, 'o')
        && submit_char(id, 'x')
        && submit_backspace(id)
        && submit_char(id, 'k')
        && submit_enter(id)
        && read_input_blocking(id, &mut input, 8) == 3
        && &input[..3] == b"ok\n"
        && set_input_mode(id, 0) == Some(TTY_MODE_DEFAULT)
        && submit_char(id, 'q')
        && read_input_blocking(id, &mut input, 8) == 1
        && input[0] == b'q'
        && reset_input_mode(id);
    destroy(id);
    passed
}

fn wake_readers(task_ids: Vec<usize>) {
    crate::evented::wake_tasks(TTY_READ_QUEUE, task_ids);
}

fn mode_name(mode: u64) -> &'static str {
    match mode & TTY_MODE_MASK {
        TTY_MODE_DEFAULT => "canon+echo+sig",
        0 => "raw",
        TTY_MODE_CANONICAL => "canon",
        TTY_MODE_ECHO => "echo",
        TTY_MODE_SIGNALS => "sig",
        value if value == (TTY_MODE_CANONICAL | TTY_MODE_ECHO) => "canon+echo",
        value if value == (TTY_MODE_CANONICAL | TTY_MODE_SIGNALS) => "canon+sig",
        value if value == (TTY_MODE_ECHO | TTY_MODE_SIGNALS) => "echo+sig",
        _ => "custom",
    }
}
