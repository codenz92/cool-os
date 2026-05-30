use core::arch::asm;

pub const EXIT: u64 = 0;
pub const WRITE: u64 = 1;
pub const YIELD: u64 = 2;
pub const GETPID: u64 = 3;
pub const MMAP: u64 = 4;
pub const OPEN: u64 = 5;
pub const READ: u64 = 6;
pub const CLOSE: u64 = 7;
pub const EXEC: u64 = 8;
pub const PIPE: u64 = 9;
pub const DUP: u64 = 10;
pub const SHMEM_CREATE: u64 = 11;
pub const SHMEM_MAP: u64 = 12;
pub const WAITPID: u64 = 13;
pub const SPAWN: u64 = 14;
pub const SLEEP_MS: u64 = 15;
pub const ABI_VERSION: u64 = 16;
pub const DNS_RESOLVE: u64 = 17;
pub const HTTP_GET: u64 = 18;
pub const SOCKET: u64 = 19;
pub const CONNECT: u64 = 20;
pub const SEND: u64 = 21;
pub const RECV: u64 = 22;
pub const GUI_OPEN: u64 = 23;
pub const GUI_PRESENT: u64 = 24;
pub const GUI_POLL_EVENT: u64 = 25;
pub const GUI_CLOSE: u64 = 26;
pub const FS_WRITE_FILE: u64 = 27;
pub const FS_CREATE_DIR: u64 = 28;
pub const FS_DELETE_TREE: u64 = 29;
pub const FS_LIST_DIR: u64 = 30;
pub const SCREENSHOT: u64 = 31;
pub const SIGNAL: u64 = 32;
pub const SETPGID: u64 = 33;
pub const GETPGID: u64 = 34;
pub const SIGNAL_GROUP: u64 = 35;
pub const SPAWN_ARGS: u64 = 36;
pub const CHDIR: u64 = 37;
pub const GETCWD: u64 = 38;
pub const STAT: u64 = 39;
pub const RENAME: u64 = 40;
pub const OPEN_WRITE: u64 = 41;
pub const SPAWN_FDS_ARGS: u64 = 42;
pub const SYNC: u64 = 43;
pub const TIME: u64 = 44;
pub const POLL: u64 = 45;
pub const TTY_CONTROL: u64 = 46;
pub const THREAD_SPAWN: u64 = 47;
pub const FUTEX_WAIT: u64 = 48;
pub const FUTEX_WAKE: u64 = 49;
pub const THREAD_TLS_SET: u64 = 50;
pub const THREAD_TLS_GET: u64 = 51;
pub const THREAD_SPAWN_TLS: u64 = 52;
pub const MPROTECT: u64 = 53;
pub const MMAP_FILE: u64 = 54;

#[inline]
pub unsafe fn syscall0(nr: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        inlateout("rax") nr => ret,
        lateout("rcx") _,
        lateout("rdi") _,
        lateout("rsi") _,
        lateout("rdx") _,
        lateout("r8") _,
        lateout("r9") _,
        lateout("r10") _,
        lateout("r11") _,
    );
    ret
}

#[inline]
pub unsafe fn syscall1(nr: u64, a1: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        inlateout("rax") nr => ret,
        inlateout("rdi") a1 => _,
        lateout("rcx") _,
        lateout("rsi") _,
        lateout("rdx") _,
        lateout("r8") _,
        lateout("r9") _,
        lateout("r10") _,
        lateout("r11") _,
    );
    ret
}

#[inline]
pub unsafe fn syscall2(nr: u64, a1: u64, a2: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        inlateout("rax") nr => ret,
        inlateout("rdi") a1 => _,
        inlateout("rsi") a2 => _,
        lateout("rcx") _,
        lateout("rdx") _,
        lateout("r8") _,
        lateout("r9") _,
        lateout("r10") _,
        lateout("r11") _,
    );
    ret
}

#[inline]
pub unsafe fn syscall3(nr: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let ret: u64;
    asm!(
        "syscall",
        inlateout("rax") nr => ret,
        inlateout("rdi") a1 => _,
        inlateout("rsi") a2 => _,
        inlateout("rdx") a3 => _,
        lateout("rcx") _,
        lateout("r8") _,
        lateout("r9") _,
        lateout("r10") _,
        lateout("r11") _,
    );
    ret
}
