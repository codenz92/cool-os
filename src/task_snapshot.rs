extern crate alloc;

use alloc::{format, string::String, vec::Vec};

const LOG_DIR: &str = "/LOGS";
const SNAPSHOT_PATH: &str = "/LOGS/TASKS.TXT";

pub fn lines() -> Vec<String> {
    let sched = crate::scheduler::SCHEDULER.lock();
    sched
        .tasks
        .iter()
        .enumerate()
        .map(|(pid, task)| {
            format!(
                "pid={} parent={:?} group={} tty={:?} state={:?} exit={:?} name={}",
                pid,
                task.parent,
                task.process_group,
                task.controlling_tty,
                task.status,
                task.exit_code,
                task.name
            )
        })
        .collect()
}

pub fn persist() -> Result<(), crate::fat32::FsError> {
    let snapshot = lines();
    let _ = crate::vfs::vfs_kernel_create_dir(LOG_DIR);
    let mut out = String::new();
    for line in snapshot {
        out.push_str(&line);
        out.push('\n');
    }
    crate::vfs::vfs_kernel_safe_write_file(SNAPSHOT_PATH, out.as_bytes())
}
