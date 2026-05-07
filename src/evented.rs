extern crate alloc;

use alloc::vec::Vec;

pub const SOURCE_FD: u64 = 1;
pub const SOURCE_SOCKET: u64 = 2;
pub const SOURCE_GUI: u64 = 3;
pub const SOURCE_CHILD: u64 = 4;
pub const SOURCE_TTY: u64 = 5;

pub const EVENT_READ: u64 = 1 << 0;
pub const EVENT_WRITE: u64 = 1 << 1;
pub const EVENT_HANGUP: u64 = 1 << 2;
pub const EVENT_ERROR: u64 = 1 << 3;
pub const EVENT_CHILD: u64 = 1 << 4;

pub const TIMEOUT_FOREVER: u64 = u64::MAX;

pub fn add_waiter(waiters: &mut Vec<usize>, task_id: usize) {
    if !waiters.iter().any(|&waiter| waiter == task_id) {
        waiters.push(task_id);
    }
}

pub fn remove_waiter(waiters: &mut Vec<usize>, task_id: usize) {
    waiters.retain(|&waiter| waiter != task_id);
}

pub fn wake_task(queue: &str, task_id: usize) {
    crate::wait_queue::wake(queue, task_id);
    crate::scheduler::unblock(task_id);
}

pub fn wake_tasks(queue: &str, tasks: Vec<usize>) {
    for task_id in tasks {
        wake_task(queue, task_id);
    }
}
