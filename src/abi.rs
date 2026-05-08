extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const KERNEL_ABI_VERSION: u64 = 13;
pub const KERNEL_ABI_NAME: &str = "coolOS-userspace-abi";

pub fn version() -> u64 {
    KERNEL_ABI_VERSION
}

pub fn lines() -> Vec<String> {
    alloc::vec![
        format!("{} version {}", KERNEL_ABI_NAME, KERNEL_ABI_VERSION),
        String::from(
            "sdk: libcool v1 wraps startup, argv, process, files, pipes, threads, futexes, TLS, POSIX pthread/libc shims, dynlink deps/TLS, mmap/mprotect, shmem, net",
        ),
        String::from("syscalls: exit/write/yield/getpid/mmap/open/read/close/exec"),
        String::from("syscalls: pipe/dup/shmem/waitpid/spawn/sleep_ms/abi/dns/http"),
        String::from("syscalls: socket/connect/send/recv"),
        String::from("syscalls: gui_open/gui_present/gui_poll_event/gui_close"),
        String::from("syscalls: fs_write_file/fs_create_dir/fs_delete_tree/fs_list_dir/screenshot"),
        String::from("syscalls: signal/setpgid/getpgid/signal_group/spawn_args"),
        String::from(
            "syscalls: chdir/getcwd/stat/rename/open_write/spawn_fds_args/sync/time/poll/tty_control",
        ),
        String::from("syscalls: thread_spawn/futex_wait/futex_wake/thread_tls_set/thread_tls_get/thread_spawn_tls"),
        String::from("syscalls: mprotect for W^X dynamic-loader mappings"),
        format!(
            "browser-engine-port abi={} target={} fallback={}",
            crate::browser_engine::PORT_ABI_VERSION,
            crate::browser_engine::TARGET_ENGINE,
            crate::browser_engine::FALLBACK_ENGINE
        ),
        String::from("stdio: mapped fd 0/1/2 override TTY for pipes and redirection"),
    ]
}
