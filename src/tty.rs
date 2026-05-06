extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

const TTY_BUFFER_SIZE: usize = 8192;

struct Tty {
    id: u64,
    foreground_group: Option<usize>,
    buffer: Vec<u8>,
    head: usize,
    tail: usize,
    dropped: u64,
}

impl Tty {
    fn new(id: u64) -> Self {
        let mut buffer = Vec::new();
        buffer.resize(TTY_BUFFER_SIZE, 0);
        Tty {
            id,
            foreground_group: None,
            buffer,
            head: 0,
            tail: 0,
            dropped: 0,
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
                "tty #{} foreground={} pending={} dropped={}",
                tty.id,
                tty.foreground_group
                    .map(|group| group.to_string())
                    .unwrap_or_else(|| String::from("-")),
                tty.pending_bytes(),
                tty.dropped
            )
        })
        .collect()
}

pub fn selftest_passes() -> bool {
    let id = create();
    let passed = set_foreground_group(id, Some(42))
        && foreground_group(id) == Some(42)
        && write(id, b"ok") == 2
        && pop_output_byte(id) == Some(b'o')
        && pop_output_byte(id) == Some(b'k')
        && pop_output_byte(id).is_none();
    destroy(id);
    passed
}
