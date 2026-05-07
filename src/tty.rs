extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

const TTY_BUFFER_SIZE: usize = 8192;
const TTY_INPUT_SIZE: usize = 4096;
const TTY_LINE_LIMIT: usize = 1024;
const TTY_READ_QUEUE: &str = "tty-read";

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
    waiting_reader: Option<usize>,
    eof_pending: bool,
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
            waiting_reader: None,
            eof_pending: false,
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

    fn take_reader(&mut self) -> Option<usize> {
        self.waiting_reader.take()
    }

    fn flush_line(&mut self, newline: bool) -> Option<usize> {
        for idx in 0..self.line.len() {
            self.push_input_byte(self.line[idx]);
        }
        self.line.clear();
        if newline {
            self.push_input_byte(b'\n');
        }
        self.take_reader()
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
    let wake_task = {
        let mut ttys = TTYS.lock();
        let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
            return false;
        };
        if c == '\n' || c == '\r' {
            tty.push_byte(b'\n');
            tty.flush_line(true)
        } else if c == '\u{0008}' || c == '\u{007F}' {
            if tty.remove_last_line_char() {
                tty.push_byte(8);
                tty.push_byte(b' ');
                tty.push_byte(8);
            }
            None
        } else if c == '\t' || !c.is_control() {
            if tty.line.len().saturating_add(bytes.len()) <= TTY_LINE_LIMIT {
                for &byte in bytes {
                    tty.line.push(byte);
                    tty.push_byte(byte);
                }
            }
            None
        } else {
            None
        }
    };
    wake_reader(wake_task);
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
    let wake_task = {
        let mut ttys = TTYS.lock();
        let Some(tty) = ttys.iter_mut().find(|tty| tty.id == id) else {
            return false;
        };
        if tty.line.is_empty() {
            tty.eof_pending = true;
            tty.take_reader()
        } else {
            tty.flush_line(false)
        }
    };
    wake_reader(wake_task);
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
            while n < max {
                let Some(byte) = tty.pop_input_byte() else {
                    break;
                };
                buf[n] = byte;
                n += 1;
                if byte == b'\n' {
                    break;
                }
            }
            if n > 0 {
                Some(n)
            } else if tty.eof_pending {
                tty.eof_pending = false;
                Some(0)
            } else {
                tty.waiting_reader = Some(task_id);
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
                "tty #{} foreground={} output={} input={} dropped={}/{}",
                tty.id,
                tty.foreground_group
                    .map(|group| group.to_string())
                    .unwrap_or_else(|| String::from("-")),
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
        && &input[..3] == b"ok\n";
    destroy(id);
    passed
}

fn wake_reader(task_id: Option<usize>) {
    if let Some(task_id) = task_id {
        crate::wait_queue::wake(TTY_READ_QUEUE, task_id);
        crate::scheduler::unblock(task_id);
    }
}
