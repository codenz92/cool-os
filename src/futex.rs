extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use spin::Mutex;

const MAX_FUTEX_WAITERS: usize = 128;

#[derive(Clone, Copy)]
struct FutexWaiter {
    addr: u64,
    task: usize,
}

#[derive(Default)]
struct FutexStats {
    waits: u64,
    wakes: u64,
    mismatches: u64,
    timeouts: u64,
    interrupted: u64,
    dropped: u64,
    last_addr: u64,
    last_task: usize,
    max_waiters: usize,
}

#[derive(Default)]
struct FutexState {
    waiters: Vec<FutexWaiter>,
    stats: FutexStats,
}

static FUTEXES: Mutex<FutexState> = Mutex::new(FutexState {
    waiters: Vec::new(),
    stats: FutexStats {
        waits: 0,
        wakes: 0,
        mismatches: 0,
        timeouts: 0,
        interrupted: 0,
        dropped: 0,
        last_addr: 0,
        last_task: 0,
        max_waiters: 0,
    },
});

pub fn register_waiter(addr: u64, task: usize) -> bool {
    let mut state = FUTEXES.lock();
    if state
        .waiters
        .iter()
        .any(|waiter| waiter.addr == addr && waiter.task == task)
    {
        return true;
    }
    if state.waiters.len() >= MAX_FUTEX_WAITERS {
        return false;
    }
    state.waiters.push(FutexWaiter { addr, task });
    state.stats.waits = state.stats.waits.saturating_add(1);
    state.stats.last_addr = addr;
    state.stats.last_task = task;
    state.stats.max_waiters = state.stats.max_waiters.max(state.waiters.len());
    true
}

pub fn unregister_waiter(addr: u64, task: usize) -> bool {
    let mut state = FUTEXES.lock();
    let before = state.waiters.len();
    state
        .waiters
        .retain(|waiter| !(waiter.addr == addr && waiter.task == task));
    before != state.waiters.len()
}

pub fn wake(addr: u64, count: usize) -> Vec<usize> {
    if count == 0 {
        return Vec::new();
    }
    let mut state = FUTEXES.lock();
    let mut woken = Vec::new();
    state.waiters.retain(|waiter| {
        if waiter.addr == addr && woken.len() < count {
            woken.push(waiter.task);
            false
        } else {
            true
        }
    });
    state.stats.wakes = state.stats.wakes.saturating_add(woken.len() as u64);
    state.stats.last_addr = addr;
    if let Some(&task) = woken.last() {
        state.stats.last_task = task;
    }
    woken
}

pub fn drop_task_waiters(task: usize) {
    let mut state = FUTEXES.lock();
    let before = state.waiters.len();
    state.waiters.retain(|waiter| waiter.task != task);
    let dropped = before.saturating_sub(state.waiters.len());
    if dropped > 0 {
        state.stats.dropped = state.stats.dropped.saturating_add(dropped as u64);
        state.stats.last_task = task;
    }
}

pub fn record_mismatch(addr: u64, task: usize) {
    let mut state = FUTEXES.lock();
    state.stats.mismatches = state.stats.mismatches.saturating_add(1);
    state.stats.last_addr = addr;
    state.stats.last_task = task;
}

pub fn record_timeout(addr: u64, task: usize) {
    let mut state = FUTEXES.lock();
    state.stats.timeouts = state.stats.timeouts.saturating_add(1);
    state.stats.last_addr = addr;
    state.stats.last_task = task;
}

pub fn record_interrupted(addr: u64, task: usize) {
    let mut state = FUTEXES.lock();
    state.stats.interrupted = state.stats.interrupted.saturating_add(1);
    state.stats.last_addr = addr;
    state.stats.last_task = task;
}

pub fn lines() -> Vec<String> {
    let state = FUTEXES.lock();
    vec![format!(
        "futex waiters={} max_waiters={} waits={} wakes={} mismatches={} timeouts={} interrupted={} dropped={} last_addr={:#x} last_task={}",
        state.waiters.len(),
        state.stats.max_waiters,
        state.stats.waits,
        state.stats.wakes,
        state.stats.mismatches,
        state.stats.timeouts,
        state.stats.interrupted,
        state.stats.dropped,
        state.stats.last_addr,
        state.stats.last_task
    )]
}
