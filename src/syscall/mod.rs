/// SYSCALL/SYSRET interface (Phase 9).
///
/// Register convention on SYSCALL entry (Linux-compatible):
///   rax = syscall number
///   rdi = arg1, rsi = arg2, rdx = arg3
///   rcx = saved user RIP (by CPU), r11 = saved user RFLAGS (by CPU)
///   RSP = user stack (NOT switched by SYSCALL — we do it manually)
///
/// Syscall table:
///   0  exit(code)
///   1  write(fd, buf, len) → bytes written
///   2  yield()
///   3  getpid() → current task id
///   4  mmap(addr, len, flags) → addr on success, u64::MAX on failure
///   5  open(path_ptr, path_len) → fd on success, u64::MAX on failure
///   6  read(fd, buf_ptr, len) → bytes read, u64::MAX on error
///      fd 0 reads from the current task's controlling TTY when one is assigned
///   7  close(fd) → 0
///   8  exec(path_ptr, path_len) → 0 on success, u64::MAX on error
///   9  pipe(fds_ptr) → 0 on success, u64::MAX on failure
///   10 dup(fd) → new fd on success, u64::MAX on failure
///   11 shmem_create(len) → id on success, u64::MAX on failure
///   12 shmem_map(id) → virtual address on success, u64::MAX on failure
///   13 waitpid(pid, status_ptr) → pid on success, u64::MAX on failure
///   14 spawn(path_ptr, path_len) → pid on success, u64::MAX on failure
///   15 sleep_ms(ms) → 0
///   16 abi_version() → kernel/user ABI version
///   17 dns_resolve(host_ptr, host_len) → IPv4 u32 on success, u64::MAX on failure
///   18 http_get(host_ptr, host_len) → request bytes written to stdout
///   19 socket(domain, type, proto) → socket fd on success, u64::MAX on failure
///   20 connect(socket, ipv4_be, port) → 0 on success, u64::MAX on failure
///   21 send(socket, buf_ptr, len) → bytes sent, u64::MAX on failure
///   22 recv(socket, buf_ptr, len) → bytes read, 0 on EOF/timeout, u64::MAX on failure
///   23 gui_open(title_ptr, title_len, dims) → window handle
///   24 gui_present(handle, pixels_ptr, len) → 0 on success
///   25 gui_poll_event(handle, packet_ptr, len) → packet bytes, 0 if no event
///   26 gui_close(handle) → 0 on success
///   27 fs_write_file(desc_ptr) → 0 on success
///   28 fs_create_dir(path_ptr, path_len) → 0 on success
///   29 fs_delete_tree(path_ptr, path_len) → 0 on success
///   30 fs_list_dir(desc_ptr) → bytes written to output buffer
///   31 screenshot(path_ptr, path_len, flags) → 0 on queued
///   32 signal(pid, signal) → 0 on success
///   33 setpgid(pid, pgid) → 0 on success
///   34 getpgid(pid) → pgid on success
///   35 signal_group(pgid, signal) → delivered count on success
///   36 spawn_args(desc_ptr) → pid on success, u64::MAX on failure
///   37 chdir(path_ptr, path_len) → 0 on success
///   38 getcwd(buf_ptr, len) → bytes written
///   39 stat(desc_ptr) → metadata record written to output buffer
///   40 rename(desc_ptr) → 0 on success
///   41 open_write(path_ptr, path_len) → fd on success
///   42 spawn_fds_args(desc_ptr) → pid on success, u64::MAX on failure
///   43 sync() → 0 on success
///   44 time() → packed UTC-ish RTC timestamp
///   45 poll(desc_ptr, count, timeout_ms) → ready descriptor count
///   46 tty_control(op, arg1, arg2) → mode/size/control result
///   47 thread_spawn(entry, arg, flags) → tid sharing caller address space
///   48 futex_wait(addr, expected, timeout_ms) → 0 woken, 1 mismatch, 2 timeout
///   49 futex_wake(addr, count, flags) → waiter count woken
///   50 thread_tls_set(base, flags) → 0 on success
///   51 thread_tls_get() → current FS/TLS base
///   52 thread_spawn_tls(desc_ptr) → tid, desc=[entry,arg,tls_base,flags]
///   53 mprotect(addr, len, flags) → 0 on success
///   54 mmap_file(desc_ptr) → addr, desc=[fd,addr,len,file_offset,flags]
///
/// Output path: sys_write routes bytes to the current task's controlling TTY
/// when one is assigned unless fd 1/2 is explicitly mapped in the task fd
/// table for shell redirection or pipes.
extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};

// ── Syscall output ring buffer ────────────────────────────────────────────────

const OUTPUT_SIZE: usize = 1024;
const MAX_USER_STRING: u64 = 4096;
const MAX_USER_BUFFER: u64 = 1024 * 1024;
const MAX_GUI_SURFACE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_USER_DIR_LISTING: u64 = 16 * 1024;
const STAT_RECORD_BYTES: u64 = 40;
const MAX_POLL_DESCS: u64 = 16;
const POLL_DESC_BYTES: u64 = 32;
const MAX_FUTEX_WAKE_COUNT: u64 = 64;
const USER_PROT_WRITE: u64 = 1;
const USER_PROT_EXEC: u64 = 2;
const FUTEX_WAIT_MISMATCH: u64 = 1;
const FUTEX_WAIT_TIMEOUT: u64 = 2;
const ZERO8: AtomicU8 = AtomicU8::new(0);
static OUTPUT_BUF: [AtomicU8; OUTPUT_SIZE] = [ZERO8; OUTPUT_SIZE];
static OUTPUT_HEAD: AtomicUsize = AtomicUsize::new(0);
static OUTPUT_TAIL: AtomicUsize = AtomicUsize::new(0);

pub fn push_output_byte(b: u8) {
    let head = OUTPUT_HEAD.load(Ordering::Relaxed);
    let next = (head + 1) % OUTPUT_SIZE;
    if next == OUTPUT_TAIL.load(Ordering::Acquire) {
        return; // drop if full
    }
    OUTPUT_BUF[head].store(b, Ordering::Relaxed);
    OUTPUT_HEAD.store(next, Ordering::Release);
}

pub fn pop_output_byte() -> Option<u8> {
    let tail = OUTPUT_TAIL.load(Ordering::Relaxed);
    if tail == OUTPUT_HEAD.load(Ordering::Acquire) {
        return None;
    }
    let b = OUTPUT_BUF[tail].load(Ordering::Relaxed);
    OUTPUT_TAIL.store((tail + 1) % OUTPUT_SIZE, Ordering::Release);
    Some(b)
}

// ── Bootstrap syscall stack ───────────────────────────────────────────────────
//
// Normal syscall entry now switches to the currently running task's private
// kernel stack top (tracked by the scheduler). This fallback exists only for
// early/bootstrap edge cases where no per-task stack top is available yet.

const BOOTSTRAP_SYSCALL_STACK_SIZE: usize = 64 * 1024;
static mut BOOTSTRAP_SYSCALL_STACK: [u8; BOOTSTRAP_SYSCALL_STACK_SIZE] =
    [0; BOOTSTRAP_SYSCALL_STACK_SIZE];
static BOOTSTRAP_SYSCALL_STACK_TOP: AtomicU64 = AtomicU64::new(0);

// ── MSR init ─────────────────────────────────────────────────────────────────

pub fn init() {
    unsafe {
        BOOTSTRAP_SYSCALL_STACK_TOP.store(
            core::ptr::addr_of!(BOOTSTRAP_SYSCALL_STACK) as u64
                + BOOTSTRAP_SYSCALL_STACK_SIZE as u64,
            Ordering::Relaxed,
        );

        let mut efer = x86_64::registers::model_specific::Msr::new(0xC000_0080);
        efer.write(efer.read() | 1 | (1 << 11)); // SCE = bit 0, NXE = bit 11

        // STAR bits[47:32] = kernel CS (0x08), bits[63:48] = SYSRET base (0x10).
        let mut star = x86_64::registers::model_specific::Msr::new(0xC000_0081);
        star.write((0x0010u64 << 48) | (0x0008u64 << 32));

        let mut lstar = x86_64::registers::model_specific::Msr::new(0xC000_0082);
        lstar.write(syscall_entry as *const () as u64);

        // SFMASK: clear IF (bit 9) on SYSCALL entry so IRQs can't fire mid-handler.
        let mut sfmask = x86_64::registers::model_specific::Msr::new(0xC000_0084);
        sfmask.write(0x200);
    }
}

// ── Naked syscall entry ───────────────────────────────────────────────────────
//
// On entry from SYSCALL: rax=nr, rdi=a1, rsi=a2, rdx=a3,
//                        rcx=user RIP, r11=user RFLAGS, rsp=user RSP.
// We temporarily borrow r10 (arg4, unused in our ABI) to hold the user RSP
// while we switch onto the dedicated syscall kernel stack.
//
// Stack frame built on the kernel stack (each slot = 8 bytes):
//   [rsp+64]  user RSP   (bottom — pushed first after stack switch)
//   [rsp+56]  user RIP   (rcx — needed for sysretq)
//   [rsp+48]  user RFLAGS(r11 — needed for sysretq)
//   [rsp+40]  rbp
//   [rsp+32]  rbx
//   [rsp+24]  r12
//   [rsp+16]  r13
//   [rsp+ 8]  r14
//   [rsp+ 0]  r15        (top of frame — pushed last)

#[repr(C)]
struct SyscallFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
    user_rflags: u64,
    user_rip: u64,
    user_rsp: u64,
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // Save user RSP in r10 (clobbers arg4 which our table doesn't use).
        "mov r10, rsp",
        // Switch to the current task's private kernel stack.
        "mov r9, qword ptr [rip + {stack_top}]",
        "test r9, r9",
        "jnz 2f",
        "mov r9, qword ptr [rip + {bootstrap}]",
        "2:",
        "mov rsp, r9",
        // Build stack frame.
        "push r10",      // user RSP  — restored by `pop rsp` before sysretq
        "push rcx",      // user RIP  — must be in rcx for sysretq
        "push r11",      // user RFLAGS — must be in r11 for sysretq
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // Shuffle registers for dispatch(frame, nr, a1, a2, a3) using SysV:
        //   rdi=frame  rsi=nr  rdx=a1  rcx=a2  r8=a3
        // Input: rax=nr  rdi=a1  rsi=a2  rdx=a3
        "mov r8, rdx",
        "mov rcx, rsi",
        "mov rdx, rdi",
        "mov rsi, rax",
        "mov rdi, rsp",
        "call {dispatch}",
        // Return value in rax.  Restore saved registers.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop r11",       // user RFLAGS → r11
        "pop rcx",       // user RIP   → rcx
        "pop rsp",       // restore user RSP
        "sysretq",
        stack_top = sym crate::scheduler::CURRENT_SYSCALL_STACK_TOP,
        bootstrap = sym BOOTSTRAP_SYSCALL_STACK_TOP,
        dispatch = sym syscall_dispatch,
    );
}

// ── Dispatcher and handlers ───────────────────────────────────────────────────

extern "C" fn syscall_dispatch(
    frame: &mut SyscallFrame,
    nr: u64,
    a1: u64,
    a2: u64,
    a3: u64,
) -> u64 {
    match nr {
        0 => {
            sys_exit(a1);
            0
        }
        1 => sys_write(a1, a2 as *const u8, a3),
        2 => {
            sys_yield();
            0
        }
        3 => sys_getpid(),
        4 => sys_mmap(a1, a2, a3),
        5 => sys_open(a1 as *const u8, a2),
        6 => sys_read(a1, a2 as *mut u8, a3),
        7 => {
            sys_close(a1);
            0
        }
        8 => sys_exec(frame, a1 as *const u8, a2),
        9 => sys_pipe(a1 as *mut u64),
        10 => sys_dup(a1),
        11 => sys_shmem_create(a1),
        12 => sys_shmem_map(a1),
        13 => sys_waitpid(a1, a2 as *mut u64),
        14 => sys_spawn(a1 as *const u8, a2),
        15 => {
            sys_sleep_ms(a1);
            0
        }
        16 => crate::abi::version(),
        17 => sys_dns_resolve(a1 as *const u8, a2),
        18 => sys_http_get(a1 as *const u8, a2),
        19 => sys_socket(a1, a2, a3),
        20 => sys_connect(a1, a2, a3),
        21 => sys_send(a1, a2 as *const u8, a3),
        22 => sys_recv(a1, a2 as *mut u8, a3),
        23 => sys_gui_open(a1 as *const u8, a2, a3),
        24 => sys_gui_present(a1, a2 as *const u8, a3),
        25 => sys_gui_poll_event(a1, a2 as *mut u8, a3),
        26 => sys_gui_close(a1),
        27 => sys_fs_write_file(a1 as *const u8),
        28 => sys_fs_create_dir(a1 as *const u8, a2),
        29 => sys_fs_delete_tree(a1 as *const u8, a2),
        30 => sys_fs_list_dir(a1 as *const u8),
        31 => sys_screenshot(a1 as *const u8, a2, a3),
        32 => sys_signal(a1, a2),
        33 => sys_setpgid(a1, a2),
        34 => sys_getpgid(a1),
        35 => sys_signal_group(a1, a2),
        36 => sys_spawn_args(a1 as *const u8),
        37 => sys_chdir(a1 as *const u8, a2),
        38 => sys_getcwd(a1 as *mut u8, a2),
        39 => sys_stat(a1 as *const u8),
        40 => sys_rename(a1 as *const u8),
        41 => sys_open_write(a1 as *const u8, a2),
        42 => sys_spawn_fds_args(a1 as *const u8),
        43 => sys_sync(),
        44 => sys_time(),
        45 => sys_poll(a1 as *mut u8, a2, a3),
        46 => sys_tty_control(a1, a2, a3),
        47 => sys_thread_spawn(a1, a2, a3),
        48 => sys_futex_wait(a1, a2, a3),
        49 => sys_futex_wake(a1, a2, a3),
        50 => sys_thread_tls_set(a1, a2),
        51 => sys_thread_tls_get(),
        52 => sys_thread_spawn_tls(a1 as *const u8),
        53 => sys_mprotect(a1, a2, a3),
        54 => sys_mmap_file(a1 as *const u8),
        _ => u64::MAX,
    }
}

// Section files are included into this module so the split stays behavior-neutral.

include!("core.rs");
include!("net_gui.rs");
include!("fs.rs");
include!("poll.rs");
include!("process.rs");
include!("user_memory.rs");
