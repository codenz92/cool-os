extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

pub const MAX_ACTIVE_TASKS: usize = 64;
pub const MAX_USER_ADDRESS_SPACE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_USER_ADDRESS_SPACE_PAGES: usize = (MAX_USER_ADDRESS_SPACE_BYTES as usize) / 4096;
pub const MAX_USER_MMAP_BYTES_PER_CALL: u64 = 4 * 1024 * 1024;
pub const MAX_SHMEM_REGION_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_SHMEM_BYTES_PER_TASK: usize = 8 * 1024 * 1024;
pub const MAX_SOCKETS_PER_TASK: usize = 8;
pub const MAX_SOCKETS_TOTAL: usize = 64;

pub fn lines() -> Vec<String> {
    let sched = crate::scheduler::resource_stats();
    let vmm = crate::vmm::resource_stats();
    let vfs = crate::vfs::resource_stats();
    let net = crate::net::resource_stats();
    vec![
        format!(
            "tasks active={}/{} user={} user_threads={} tls_threads={} slots={} reaped={}",
            sched.active_tasks,
            sched.max_active_tasks,
            sched.user_tasks,
            sched.user_threads,
            sched.tls_threads,
            sched.task_slots,
            sched.reaped_tasks
        ),
        format!(
            "address-space owned_pages={} table_pages={} spaces={} max_per_task={} bytes mmap_call_max={} bytes",
            vmm.owned_leaf_pages,
            vmm.page_table_pages,
            vmm.address_spaces,
            MAX_USER_ADDRESS_SPACE_BYTES,
            MAX_USER_MMAP_BYTES_PER_CALL
        ),
        format!(
            "vfs fd_tables={} open_fds={}/{} shared_objects={} files={} pipes={}",
            vfs.task_tables,
            vfs.open_fds,
            vfs.task_tables.saturating_mul(vfs.max_fds_per_task),
            vfs.shared_objects,
            vfs.file_objects,
            vfs.pipe_objects
        ),
        format!(
            "shmem regions={} frames={} per_task_max={} bytes region_max={} bytes",
            vfs.shmem_regions,
            vfs.shmem_frames,
            MAX_SHMEM_BYTES_PER_TASK,
            MAX_SHMEM_REGION_BYTES
        ),
        format!(
            "thread stacks slots={} stack_size={} bytes slot_size={} bytes",
            crate::vmm::USER_THREAD_STACK_SLOTS,
            crate::vmm::USER_STACK_SIZE,
            crate::vmm::USER_THREAD_STACK_SLOT_SIZE
        ),
        format!(
            "net sockets={}/{} slots={} per_task_max={} kernel_owned={}",
            net.open_sockets,
            net.max_sockets_total,
            net.socket_slots,
            net.max_sockets_per_task,
            net.kernel_owned_sockets
        ),
    ]
}

pub fn selftest_passes() -> bool {
    MAX_ACTIVE_TASKS >= 16
        && MAX_USER_ADDRESS_SPACE_PAGES == (MAX_USER_ADDRESS_SPACE_BYTES as usize) / 4096
        && MAX_USER_MMAP_BYTES_PER_CALL <= MAX_USER_ADDRESS_SPACE_BYTES
        && MAX_SHMEM_REGION_BYTES <= MAX_SHMEM_BYTES_PER_TASK
        && MAX_SOCKETS_PER_TASK <= MAX_SOCKETS_TOTAL
        && crate::vfs::max_fds_per_task() >= 8
}
