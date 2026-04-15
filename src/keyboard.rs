//! Lock-free keyboard ring buffer.
//!
//! The PS/2 keyboard IRQ handler pushes decoded chars here without touching
//! the WM lock.  The compositor drains the buffer at the start of each frame
//! while it already holds the WM lock.  This prevents the classic
//! interrupt-context deadlock: IRQ fires while compose() holds WM.lock(),
//! IRQ tries to acquire WM.lock(), single-core deadlock.

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const QUEUE_SIZE: usize = 64;

const ZERO: AtomicU32 = AtomicU32::new(0);
static QUEUE: [AtomicU32; QUEUE_SIZE] = [ZERO; QUEUE_SIZE];
static HEAD: AtomicUsize = AtomicUsize::new(0); // written by IRQ handler
static TAIL: AtomicUsize = AtomicUsize::new(0); // read  by main loop

/// Push a character from interrupt context. Silently drops if the buffer is full.
pub fn push(c: char) {
    let head = HEAD.load(Ordering::Relaxed);
    let next = (head + 1) % QUEUE_SIZE;
    if next == TAIL.load(Ordering::Acquire) {
        return; // full — drop
    }
    QUEUE[head].store(c as u32, Ordering::Relaxed);
    HEAD.store(next, Ordering::Release);
}

/// Pop a character from the main loop. Returns `None` when empty.
pub fn pop() -> Option<char> {
    let tail = TAIL.load(Ordering::Relaxed);
    if tail == HEAD.load(Ordering::Acquire) {
        return None;
    }
    let v = QUEUE[tail].load(Ordering::Relaxed);
    TAIL.store((tail + 1) % QUEUE_SIZE, Ordering::Release);
    char::from_u32(v)
}
