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
        _ => u64::MAX,
    }
}

fn sys_write(fd: u64, buf: *const u8, len: u64) -> u64 {
    if len == 0 {
        return 0;
    }
    let Some(bytes) = user_slice(buf, len, MAX_USER_BUFFER) else {
        return u64::MAX;
    };

    if (fd == 1 || fd == 2) && !crate::vfs::vfs_has_fd(fd as usize) {
        write_output_bytes(bytes);
        return len;
    }

    let n = crate::vfs::vfs_write(fd as usize, bytes);
    if n == usize::MAX {
        u64::MAX
    } else {
        n as u64
    }
}

fn sys_pipe(fds_ptr: *mut u64) -> u64 {
    if !validate_user_range(fds_ptr as u64, 16, 16, true) {
        return u64::MAX;
    }
    match crate::vfs::vfs_pipe() {
        Some((read_fd, write_fd)) => unsafe {
            *fds_ptr.add(0) = read_fd as u64;
            *fds_ptr.add(1) = write_fd as u64;
            0
        },
        None => u64::MAX,
    }
}

fn sys_getpid() -> u64 {
    let sched = crate::scheduler::SCHEDULER.lock();
    sched.current as u64
}

/// Map `len` bytes at virtual address `addr` in the calling process's address
/// space with the given protection flags (bit 0 = writable).  Allocates
/// physical frames and inserts PTEs.  Returns `addr` on success, `u64::MAX`
/// on failure.
fn sys_mmap(addr: u64, len: u64, flags: u64) -> u64 {
    use x86_64::{structures::paging::PageTableFlags, VirtAddr};

    if addr == 0 || len == 0 {
        return u64::MAX;
    }

    // Round length up to page boundary.
    let Some(len_aligned) = len.checked_add(4095).map(|value| value & !4095) else {
        return u64::MAX;
    };
    if !valid_user_address_range(addr, len_aligned, MAX_USER_BUFFER * 64)
        || !crate::vmm::valid_user_mmap_range(addr, len_aligned)
    {
        return u64::MAX;
    }
    if len_aligned > crate::resource_limits::MAX_USER_MMAP_BYTES_PER_CALL {
        return u64::MAX;
    }

    let mut pte_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & 1 != 0 {
        pte_flags |= PageTableFlags::WRITABLE;
    }
    pte_flags |= PageTableFlags::NO_EXECUTE;

    // Determine the current process's PML4.
    let pml4 = crate::vmm::current_pml4();
    let pages = len_aligned.saturating_div(4096) as usize;
    if !crate::vmm::can_add_owned_pages(pml4, pages) {
        return u64::MAX;
    }

    match crate::vmm::map_region(pml4, VirtAddr::new(addr), len_aligned, pte_flags) {
        Ok(()) => addr,
        Err(_) => u64::MAX,
    }
}

fn sys_exit(_code: u64) {
    crate::scheduler::exit_current(_code);
    // Interrupts are still disabled here (SFMASK cleared IF on SYSCALL entry).
    // The naked handler will sysretq back to ring 3; the task spins with
    // core::hint::spin_loop() until the timer fires and switches it out
    // permanently (Exited tasks are never picked by the round-robin scheduler).
}

/// Open a file by path.  `path_ptr` is a user-space pointer to a UTF-8 string
/// of length `path_len` (no nul terminator required).
fn sys_open(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(bytes) = user_slice(path_ptr, path_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    match core::str::from_utf8(bytes) {
        Ok(path) => {
            let path = resolve_task_path(path);
            let fd = crate::vfs::vfs_open(&path);
            if fd == usize::MAX {
                u64::MAX
            } else {
                fd as u64
            }
        }
        Err(_) => u64::MAX,
    }
}

/// Read up to `len` bytes from `fd` into the user buffer at `buf_ptr`.
fn sys_read(fd: u64, buf_ptr: *mut u8, len: u64) -> u64 {
    if len == 0 {
        return 0;
    }
    if !validate_user_range(buf_ptr as u64, len, MAX_USER_BUFFER, true) {
        return u64::MAX;
    }

    let mut kernel_buf = Vec::new();
    if kernel_buf.try_reserve_exact(len as usize).is_err() {
        return u64::MAX;
    }
    kernel_buf.resize(len as usize, 0);

    let n = if fd == 0 {
        if crate::vfs::vfs_has_fd(0) {
            crate::vfs::vfs_read_blocking(fd as usize, &mut kernel_buf, len as usize)
        } else if let Some(tty) = crate::scheduler::current_tty() {
            crate::tty::read_input_blocking(tty, &mut kernel_buf, len as usize)
        } else {
            crate::vfs::vfs_read_blocking(fd as usize, &mut kernel_buf, len as usize)
        }
    } else {
        crate::vfs::vfs_read_blocking(fd as usize, &mut kernel_buf, len as usize)
    };
    if n == usize::MAX {
        u64::MAX
    } else {
        unsafe {
            core::ptr::copy_nonoverlapping(kernel_buf.as_ptr(), buf_ptr, n);
        }
        n as u64
    }
}

fn sys_close(fd: u64) {
    let owner = crate::scheduler::current_task_id();
    if !crate::net::socket_close(owner, fd) {
        crate::vfs::vfs_close(fd as usize);
    }
}

fn sys_tty_control(op: u64, arg1: u64, arg2: u64) -> u64 {
    let Some(tty) = crate::scheduler::current_tty() else {
        return u64::MAX;
    };
    crate::tty::control(tty, op, arg1, arg2).unwrap_or(u64::MAX)
}

fn sys_socket(domain: u64, socket_type: u64, protocol: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let owner = crate::scheduler::current_task_id();
    crate::net::socket_open(owner, domain, socket_type, protocol).unwrap_or(u64::MAX)
}

fn sys_connect(socket: u64, ipv4_be: u64, port: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if ipv4_be > u32::MAX as u64 || port > u16::MAX as u64 {
        return u64::MAX;
    }
    let owner = crate::scheduler::current_task_id();
    match crate::net::socket_connect(owner, socket, ipv4_be as u32, port as u16) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_send(socket: u64, buf: *const u8, len: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if len == 0 {
        return 0;
    }
    let Some(bytes) = user_slice(buf, len, MAX_USER_BUFFER) else {
        return u64::MAX;
    };
    let owner = crate::scheduler::current_task_id();
    match crate::net::socket_send(owner, socket, bytes) {
        Ok(n) => n as u64,
        Err(err) => {
            crate::println!("[net] sys_send failed: {}", err);
            u64::MAX
        }
    }
}

fn sys_recv(socket: u64, buf: *mut u8, len: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if len == 0 {
        return 0;
    }
    let Some(out) = user_slice_mut(buf, len, MAX_USER_BUFFER) else {
        return u64::MAX;
    };
    let owner = crate::scheduler::current_task_id();
    match crate::net::socket_recv(owner, socket, out) {
        Ok(n) => n as u64,
        Err(err) => {
            crate::println!("[net] sys_recv failed: {}", err);
            u64::MAX
        }
    }
}

fn sys_gui_open(title_ptr: *const u8, title_len: u64, dims: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let Some(title_bytes) = user_slice(title_ptr, title_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let title = match core::str::from_utf8(title_bytes) {
        Ok(title) => title,
        Err(_) => return u64::MAX,
    };
    let width = (dims & 0xffff) as u16;
    let height = ((dims >> 16) & 0xffff) as u16;
    if width == 0 || height == 0 {
        return u64::MAX;
    }
    let owner = crate::scheduler::current_task_id();
    crate::wm::user_gui_open(owner, title, width, height)
}

fn sys_gui_present(handle: u64, pixels_ptr: *const u8, len: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if handle == 0 || len == 0 || len % 4 != 0 {
        return u64::MAX;
    }
    let Some(pixels) = user_slice(pixels_ptr, len, MAX_GUI_SURFACE_BYTES) else {
        return u64::MAX;
    };
    let owner = crate::scheduler::current_task_id();
    if crate::wm::user_gui_present(owner, handle, pixels) {
        0
    } else {
        u64::MAX
    }
}

fn sys_gui_poll_event(handle: u64, packet_ptr: *mut u8, len: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if handle == 0 || len == 0 {
        return u64::MAX;
    }
    let Some(out) = user_slice_mut(packet_ptr, len, 64) else {
        return u64::MAX;
    };
    let owner = crate::scheduler::current_task_id();
    crate::wm::user_gui_poll_event(owner, handle, out)
        .map(|n| n as u64)
        .unwrap_or(0)
}

fn sys_gui_close(handle: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    if handle == 0 {
        return u64::MAX;
    }
    let owner = crate::scheduler::current_task_id();
    if crate::wm::user_gui_close(owner, handle) {
        0
    } else {
        u64::MAX
    }
}

fn sys_fs_write_file(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let data = if desc[3] == 0 {
        &[]
    } else {
        let Some(data) = user_slice(desc[2] as *const u8, desc[3], MAX_USER_BUFFER) else {
            return u64::MAX;
        };
        data
    };

    let path = resolve_task_path(path);
    match crate::vfs::vfs_create_file(&path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return u64::MAX,
    }

    match crate::vfs::vfs_write_file(&path, data) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_create_dir(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    match crate::vfs::vfs_create_dir(&path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_delete_tree(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    match crate::vfs::vfs_delete_recursive(&path) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_list_dir(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let Some(out) = user_slice_mut(desc[2] as *mut u8, desc[3], MAX_USER_DIR_LISTING) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let Some(entries) = crate::vfs::vfs_list_dir(&path) else {
        return u64::MAX;
    };

    let mut written = 0usize;
    for entry in entries {
        if !append_dir_listing_byte(out, &mut written, if entry.is_dir { b'D' } else { b'F' }) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\t') {
            break;
        }
        if !append_dir_listing_bytes(out, &mut written, entry.name.as_bytes()) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\t') {
            break;
        }
        if !append_dir_listing_u64(out, &mut written, entry.size as u64) {
            break;
        }
        if !append_dir_listing_byte(out, &mut written, b'\n') {
            break;
        }
    }
    written as u64
}

fn sys_screenshot(path_ptr: *const u8, path_len: u64, _flags: u64) -> u64 {
    if !crate::security::can_desktop(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    crate::wm::request_focused_screenshot(&path);
    0
}

fn sys_waitpid(pid: u64, status_ptr: *mut u64) -> u64 {
    if !status_ptr.is_null() && !validate_user_range(status_ptr as u64, 8, 8, true) {
        return u64::MAX;
    }
    let parent = crate::scheduler::current_task_id();
    loop {
        match crate::scheduler::waitpid(parent, pid as usize) {
            Ok(code) => unsafe {
                if !status_ptr.is_null() {
                    *status_ptr = code;
                }
                return pid;
            },
            Err(crate::scheduler::WaitError::NotExited) => {
                crate::wait_queue::wait("waitpid", parent);
                crate::scheduler::block_current();
                while crate::scheduler::current_task_blocked() {
                    if crate::scheduler::current_has_pending_signal() {
                        break;
                    }
                    unsafe {
                        core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
                    }
                }
                crate::wait_queue::wake("waitpid", parent);
                x86_64::instructions::interrupts::disable();
                if crate::scheduler::current_has_pending_signal() {
                    return u64::MAX;
                }
            }
            Err(_) => return u64::MAX,
        }
    }
}

fn sys_spawn(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(bytes) = user_slice(path_ptr, path_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let path = match core::str::from_utf8(bytes) {
        Ok(path) => path,
        Err(_) => return u64::MAX,
    };
    let path = resolve_task_path(path);
    match crate::elf::spawn_elf_process_with_args(&path, &[]) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_spawn_args(desc_ptr: *const u8) -> u64 {
    const MAX_ARGC: u64 = 7;
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let argc = desc[3];
    if argc > MAX_ARGC {
        return u64::MAX;
    }

    let arg_pairs = if argc == 0 {
        &[][..]
    } else {
        let pair_bytes = argc.saturating_mul(16);
        let Some(bytes) = user_slice(desc[2] as *const u8, pair_bytes, MAX_ARGC * 16) else {
            return u64::MAX;
        };
        bytes
    };

    let mut arg_strings = Vec::new();
    if arg_strings.try_reserve_exact(argc as usize).is_err() {
        return u64::MAX;
    }
    for idx in 0..argc as usize {
        let base = idx * 16;
        let ptr = u64::from_le_bytes(match arg_pairs[base..base + 8].try_into() {
            Ok(bytes) => bytes,
            Err(_) => return u64::MAX,
        });
        let len = u64::from_le_bytes(match arg_pairs[base + 8..base + 16].try_into() {
            Ok(bytes) => bytes,
            Err(_) => return u64::MAX,
        });
        let Some(bytes) = user_slice(ptr as *const u8, len, MAX_USER_STRING) else {
            return u64::MAX;
        };
        let Ok(arg) = core::str::from_utf8(bytes) else {
            return u64::MAX;
        };
        arg_strings.push(String::from(arg));
    }

    let path = resolve_task_path(path);
    let mut arg_refs = Vec::new();
    if arg_refs.try_reserve_exact(arg_strings.len()).is_err() {
        return u64::MAX;
    }
    for arg in &arg_strings {
        arg_refs.push(arg.as_str());
    }

    match crate::elf::spawn_elf_process_with_args(&path, &arg_refs) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_chdir(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    if crate::vfs::vfs_list_dir(&path).is_none() {
        return u64::MAX;
    }
    crate::scheduler::set_current_cwd(path);
    0
}

fn sys_getcwd(buf_ptr: *mut u8, len: u64) -> u64 {
    if len == 0 || !validate_user_range(buf_ptr as u64, len, MAX_USER_STRING, true) {
        return u64::MAX;
    }
    let cwd = crate::scheduler::current_cwd();
    let bytes = cwd.as_bytes();
    if bytes.len() > len as usize {
        return u64::MAX;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr, bytes.len());
    }
    bytes.len() as u64
}

fn sys_stat(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    if desc[3] < STAT_RECORD_BYTES {
        return u64::MAX;
    }
    let Some(out) = user_slice_mut(desc[2] as *mut u8, STAT_RECORD_BYTES, STAT_RECORD_BYTES) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let Some(meta) = crate::vfs::vfs_metadata(&path) else {
        return u64::MAX;
    };
    let kind = if meta.is_dir {
        2u64
    } else if meta.is_file {
        1u64
    } else {
        0u64
    };
    write_record_u64(out, 0, kind);
    write_record_u64(out, 8, meta.size);
    write_record_u64(out, 16, meta.uid as u64);
    write_record_u64(out, 24, meta.gid as u64);
    write_record_u64(out, 32, meta.mode as u64);
    STAT_RECORD_BYTES
}

fn sys_rename(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let Some(src) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let Some(dst) = user_path(desc[2] as *const u8, desc[3]) else {
        return u64::MAX;
    };
    let src = resolve_task_path(src);
    let dst = resolve_task_path(dst);
    match crate::vfs::vfs_rename_path(&src, &dst) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_open_write(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    let path = resolve_task_path(path);
    let fd = crate::vfs::vfs_open_write(&path);
    if fd == usize::MAX {
        u64::MAX
    } else {
        fd as u64
    }
}

fn sys_spawn_fds_args(desc_ptr: *const u8) -> u64 {
    const MAX_ARGC: u64 = 7;
    const MAX_FD_MAPS: u64 = 4;
    let Some(desc) = user_descriptor6(desc_ptr) else {
        return u64::MAX;
    };
    let Some(path) = user_path(desc[0] as *const u8, desc[1]) else {
        return u64::MAX;
    };
    let argc = desc[3];
    let fd_count = desc[5];
    if argc > MAX_ARGC || fd_count > MAX_FD_MAPS {
        return u64::MAX;
    }

    let Some(arg_strings) = parse_user_arg_strings(desc[2], argc, MAX_ARGC) else {
        return u64::MAX;
    };
    let Some(fd_mappings) = parse_user_fd_mappings(desc[4], fd_count, MAX_FD_MAPS) else {
        return u64::MAX;
    };

    let mut arg_refs = Vec::new();
    if arg_refs.try_reserve_exact(arg_strings.len()).is_err() {
        return u64::MAX;
    }
    for arg in &arg_strings {
        arg_refs.push(arg.as_str());
    }

    let path = resolve_task_path(path);
    match crate::elf::spawn_elf_process_with_fds(&path, &arg_refs, &fd_mappings) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
}

fn sys_sync() -> u64 {
    match crate::writeback::barrier() {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_time() -> u64 {
    let Some(dt) = crate::rtc::read_datetime() else {
        return 0;
    };
    ((dt.year as u64) << 32)
        | ((dt.month as u64) << 24)
        | ((dt.day as u64) << 16)
        | ((dt.hour as u64) << 8)
        | (dt.minute as u64)
}

#[derive(Clone, Copy)]
struct PollDesc {
    source: u64,
    handle: u64,
    events: u64,
    revents: u64,
}

fn sys_poll(desc_ptr: *mut u8, count: u64, timeout_ms: u64) -> u64 {
    if count > MAX_POLL_DESCS {
        return u64::MAX;
    }
    if count == 0 {
        block_poll_timeout(timeout_ms);
        return 0;
    }
    let Some(byte_len) = count.checked_mul(POLL_DESC_BYTES) else {
        return u64::MAX;
    };
    let Some(bytes) = user_slice_mut(
        desc_ptr,
        byte_len,
        MAX_POLL_DESCS.saturating_mul(POLL_DESC_BYTES),
    ) else {
        return u64::MAX;
    };
    let Some(mut descs) = read_poll_descs(bytes, count as usize) else {
        return u64::MAX;
    };

    let task_id = crate::scheduler::current_task_id();
    let deadline = poll_deadline(timeout_ms);
    loop {
        let ready = scan_poll_descriptors(&mut descs).0;
        if ready > 0 || timeout_ms == 0 || poll_deadline_expired(deadline) {
            write_poll_descs(bytes, &descs);
            return ready;
        }

        let registered = register_poll_waiters(&descs, task_id);
        let ready = scan_poll_descriptors(&mut descs).0;
        if ready > 0 || poll_deadline_expired(deadline) {
            unregister_poll_waiters(&descs, task_id);
            write_poll_descs(bytes, &descs);
            return ready;
        }

        crate::wait_queue::wait("poll", task_id);
        let wake_tick = if registered {
            deadline
        } else {
            Some(crate::interrupts::ticks().wrapping_add(1))
        };
        if let Some(wake_tick) = wake_tick {
            crate::scheduler::block_current_until(wake_tick);
        } else {
            crate::scheduler::block_current();
        }
        while crate::scheduler::current_task_blocked() {
            if crate::scheduler::current_has_pending_signal() {
                break;
            }
            unsafe {
                core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
            }
        }
        crate::wait_queue::wake("poll", task_id);
        x86_64::instructions::interrupts::disable();
        unregister_poll_waiters(&descs, task_id);

        if crate::scheduler::current_has_pending_signal() {
            write_poll_descs(bytes, &descs);
            return u64::MAX;
        }
    }
}

fn block_poll_timeout(timeout_ms: u64) {
    if timeout_ms == 0 {
        return;
    }
    let task_id = crate::scheduler::current_task_id();
    let deadline = poll_deadline(timeout_ms);
    crate::wait_queue::wait("poll", task_id);
    if let Some(wake_tick) = deadline {
        crate::scheduler::block_current_until(wake_tick);
    } else {
        crate::scheduler::block_current();
    }
    while crate::scheduler::current_task_blocked() {
        if crate::scheduler::current_has_pending_signal() {
            break;
        }
        unsafe {
            core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
        }
    }
    crate::wait_queue::wake("poll", task_id);
    x86_64::instructions::interrupts::disable();
}

fn poll_deadline(timeout_ms: u64) -> Option<u64> {
    if timeout_ms == crate::evented::TIMEOUT_FOREVER {
        None
    } else {
        Some(
            crate::interrupts::ticks()
                .wrapping_add(crate::interrupts::ticks_for_millis(timeout_ms.max(1))),
        )
    }
}

fn poll_deadline_expired(deadline: Option<u64>) -> bool {
    deadline
        .map(|deadline| crate::interrupts::ticks() >= deadline)
        .unwrap_or(false)
}

fn read_poll_descs(bytes: &[u8], count: usize) -> Option<Vec<PollDesc>> {
    let mut descs = Vec::new();
    if descs.try_reserve_exact(count).is_err() {
        return None;
    }
    for idx in 0..count {
        let base = idx * POLL_DESC_BYTES as usize;
        descs.push(PollDesc {
            source: u64::from_le_bytes(bytes[base..base + 8].try_into().ok()?),
            handle: u64::from_le_bytes(bytes[base + 8..base + 16].try_into().ok()?),
            events: u64::from_le_bytes(bytes[base + 16..base + 24].try_into().ok()?),
            revents: 0,
        });
    }
    Some(descs)
}

fn write_poll_descs(bytes: &mut [u8], descs: &[PollDesc]) {
    for (idx, desc) in descs.iter().enumerate() {
        let base = idx * POLL_DESC_BYTES as usize;
        bytes[base..base + 8].copy_from_slice(&desc.source.to_le_bytes());
        bytes[base + 8..base + 16].copy_from_slice(&desc.handle.to_le_bytes());
        bytes[base + 16..base + 24].copy_from_slice(&desc.events.to_le_bytes());
        bytes[base + 24..base + 32].copy_from_slice(&desc.revents.to_le_bytes());
    }
}

fn scan_poll_descriptors(descs: &mut [PollDesc]) -> (u64, bool) {
    crate::net::poll();
    let owner = crate::scheduler::current_task_id();
    let mut ready = 0u64;
    let mut busy = false;
    for desc in descs {
        let raw = poll_desc_revents(owner, desc, &mut busy);
        desc.revents = normalize_poll_revents(desc.events, raw);
        if desc.revents != 0 {
            ready += 1;
        }
    }
    (ready, busy)
}

fn normalize_poll_revents(events: u64, raw: u64) -> u64 {
    (raw & events)
        | (raw
            & (crate::evented::EVENT_HANGUP
                | crate::evented::EVENT_ERROR
                | crate::evented::EVENT_CHILD))
}

fn poll_desc_revents(owner: usize, desc: &PollDesc, busy: &mut bool) -> u64 {
    match desc.source {
        crate::evented::SOURCE_FD => poll_fd_revents(desc.handle, desc.events),
        crate::evented::SOURCE_SOCKET => {
            crate::net::socket_poll_revents(owner, desc.handle, desc.events)
        }
        crate::evented::SOURCE_GUI => match crate::wm::user_gui_event_readiness(owner, desc.handle)
        {
            Some(revents) => revents,
            None => {
                *busy = true;
                0
            }
        },
        crate::evented::SOURCE_CHILD => {
            crate::scheduler::child_exit_revents(owner, desc.handle as usize)
        }
        crate::evented::SOURCE_TTY => poll_tty_revents(desc.handle, desc.events),
        _ => crate::evented::EVENT_ERROR,
    }
}

fn poll_fd_revents(fd: u64, events: u64) -> u64 {
    if fd > usize::MAX as u64 {
        return crate::evented::EVENT_ERROR;
    }
    let fd_usize = fd as usize;
    if fd == 0 && !crate::vfs::vfs_has_fd(0) {
        if let Some(tty) = crate::scheduler::current_tty() {
            return poll_tty_revents(tty, events);
        }
        return crate::evented::EVENT_ERROR;
    }
    if (fd == 1 || fd == 2) && !crate::vfs::vfs_has_fd(fd_usize) {
        if events & crate::evented::EVENT_WRITE != 0 {
            return crate::evented::EVENT_WRITE;
        }
        return 0;
    }
    crate::vfs::vfs_poll_fd(fd_usize, events)
}

fn poll_tty_revents(handle: u64, events: u64) -> u64 {
    if events & crate::evented::EVENT_READ == 0 {
        return 0;
    }
    let tty = if handle == 0 {
        match crate::scheduler::current_tty() {
            Some(tty) => tty,
            None => return crate::evented::EVENT_ERROR,
        }
    } else {
        handle
    };
    crate::tty::input_readiness(tty)
}

fn register_poll_waiters(descs: &[PollDesc], task_id: usize) -> bool {
    let owner = crate::scheduler::current_task_id();
    let mut ok = true;
    for desc in descs {
        let registered = match desc.source {
            crate::evented::SOURCE_FD => register_fd_waiter(desc.handle, desc.events, task_id),
            crate::evented::SOURCE_SOCKET => {
                crate::net::socket_register_waiter(owner, desc.handle, task_id, desc.events)
            }
            crate::evented::SOURCE_GUI => {
                crate::wm::register_user_gui_event_waiter(owner, desc.handle, task_id)
            }
            crate::evented::SOURCE_CHILD => {
                crate::scheduler::register_child_waiter(owner, desc.handle as usize, task_id)
            }
            crate::evented::SOURCE_TTY => register_tty_waiter(desc.handle, task_id),
            _ => false,
        };
        ok &= registered;
    }
    ok
}

fn unregister_poll_waiters(descs: &[PollDesc], task_id: usize) {
    let owner = crate::scheduler::current_task_id();
    for desc in descs {
        match desc.source {
            crate::evented::SOURCE_FD => unregister_fd_waiter(desc.handle, task_id),
            crate::evented::SOURCE_SOCKET => {
                crate::net::socket_unregister_waiter(owner, desc.handle, task_id);
            }
            crate::evented::SOURCE_GUI => {
                crate::wm::unregister_user_gui_event_waiter(owner, desc.handle, task_id);
            }
            crate::evented::SOURCE_CHILD => {
                crate::scheduler::unregister_child_waiter(owner, desc.handle as usize, task_id);
            }
            crate::evented::SOURCE_TTY => unregister_tty_waiter(desc.handle, task_id),
            _ => {}
        }
    }
}

fn register_fd_waiter(fd: u64, events: u64, task_id: usize) -> bool {
    if fd > usize::MAX as u64 {
        return false;
    }
    let fd_usize = fd as usize;
    if fd == 0 && !crate::vfs::vfs_has_fd(0) {
        let Some(tty) = crate::scheduler::current_tty() else {
            return false;
        };
        return register_tty_waiter(tty, task_id);
    }
    if (fd == 1 || fd == 2) && !crate::vfs::vfs_has_fd(fd_usize) {
        return true;
    }
    crate::vfs::vfs_register_poll_waiter(fd_usize, task_id, events)
}

fn unregister_fd_waiter(fd: u64, task_id: usize) {
    if fd > usize::MAX as u64 {
        return;
    }
    let fd_usize = fd as usize;
    if fd == 0 && !crate::vfs::vfs_has_fd(0) {
        if let Some(tty) = crate::scheduler::current_tty() {
            unregister_tty_waiter(tty, task_id);
        }
        return;
    }
    if (fd == 1 || fd == 2) && !crate::vfs::vfs_has_fd(fd_usize) {
        return;
    }
    crate::vfs::vfs_unregister_poll_waiter(fd_usize, task_id);
}

fn register_tty_waiter(handle: u64, task_id: usize) -> bool {
    let tty = if handle == 0 {
        match crate::scheduler::current_tty() {
            Some(tty) => tty,
            None => return false,
        }
    } else {
        handle
    };
    crate::tty::register_input_waiter(tty, task_id)
}

fn unregister_tty_waiter(handle: u64, task_id: usize) {
    let tty = if handle == 0 {
        match crate::scheduler::current_tty() {
            Some(tty) => tty,
            None => return,
        }
    } else {
        handle
    };
    crate::tty::unregister_input_waiter(tty, task_id);
}

fn sys_signal(pid: u64, signal_code: u64) -> u64 {
    let Some(signal) = crate::process_model::Signal::from_code(signal_code) else {
        return u64::MAX;
    };
    let target = if pid == 0 {
        crate::scheduler::current_task_id()
    } else if pid > usize::MAX as u64 {
        return u64::MAX;
    } else {
        pid as usize
    };
    match crate::scheduler::send_signal(target, signal) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_setpgid(pid: u64, group: u64) -> u64 {
    if pid > usize::MAX as u64 || group > usize::MAX as u64 {
        return u64::MAX;
    }
    let target = if pid == 0 {
        crate::scheduler::current_task_id()
    } else {
        pid as usize
    };
    let group = if group == 0 { target } else { group as usize };
    let actor = crate::scheduler::current_task_id();
    match crate::scheduler::set_process_group_as(actor, target, group) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_getpgid(pid: u64) -> u64 {
    if pid > usize::MAX as u64 {
        return u64::MAX;
    }
    let target = if pid == 0 {
        crate::scheduler::current_task_id()
    } else {
        pid as usize
    };
    crate::scheduler::get_process_group(target)
        .map(|group| group as u64)
        .unwrap_or(u64::MAX)
}

fn sys_signal_group(group: u64, signal_code: u64) -> u64 {
    if group > usize::MAX as u64 {
        return u64::MAX;
    }
    let Some(signal) = crate::process_model::Signal::from_code(signal_code) else {
        return u64::MAX;
    };
    let group = if group == 0 {
        crate::scheduler::current_process_group()
    } else {
        group as usize
    };
    crate::scheduler::send_signal_to_group(group, signal)
        .map(|count| count as u64)
        .unwrap_or(u64::MAX)
}

fn sys_sleep_ms(ms: u64) {
    let ticks = crate::interrupts::ticks_for_millis(ms.max(1));
    let wake_tick = crate::interrupts::ticks().wrapping_add(ticks);
    crate::wait_queue::wait("timer-sleep", crate::scheduler::current_task_id());
    crate::scheduler::block_current_until(wake_tick);
    while crate::scheduler::current_task_blocked() {
        if crate::scheduler::current_has_pending_signal() {
            break;
        }
        unsafe {
            core::arch::asm!("sti; hlt; cli", options(nomem, nostack));
        }
    }
    crate::wait_queue::wake("timer-sleep", crate::scheduler::current_task_id());
    x86_64::instructions::interrupts::disable();
}

fn sys_dns_resolve(host_ptr: *const u8, host_len: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let Some(bytes) = user_slice(host_ptr, host_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let host = match core::str::from_utf8(bytes) {
        Ok(host) => host,
        Err(_) => return u64::MAX,
    };
    crate::net::dns_resolve(host)
        .map(|addr| addr as u64)
        .unwrap_or(u64::MAX)
}

fn sys_http_get(host_ptr: *const u8, host_len: u64) -> u64 {
    if !crate::security::can_network(crate::security::current_credentials()) {
        return u64::MAX;
    }
    let Some(bytes) = user_slice(host_ptr, host_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let host = match core::str::from_utf8(bytes) {
        Ok(host) => host,
        Err(_) => return u64::MAX,
    };
    let request = match crate::net::http_get(host, "/") {
        Ok(request) => request,
        Err(_) => return u64::MAX,
    };
    write_output_bytes(request.as_bytes());
    request.len() as u64
}

fn write_output_bytes(bytes: &[u8]) {
    let routed = crate::scheduler::current_tty()
        .map(|tty| crate::tty::write(tty, bytes) == bytes.len())
        .unwrap_or(false);
    if !routed {
        for &byte in bytes {
            push_output_byte(byte);
        }
    }
    for &byte in bytes {
        // Mirror to QEMU debugcon (port 0xE9) for headless verification.
        unsafe { x86_64::instructions::port::Port::<u8>::new(0xE9).write(byte) };
    }
    crate::wm::request_repaint();
}

fn sys_exec(frame: &mut SyscallFrame, path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(bytes) = user_slice(path_ptr, path_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let path = match core::str::from_utf8(bytes) {
        Ok(path) => path,
        Err(_) => return u64::MAX,
    };
    let path = resolve_task_path(path);

    let image = match crate::elf::load_elf_image(&path) {
        Ok(image) => image,
        Err(_) => return u64::MAX,
    };

    let (cur, old_pml4) = {
        let mut sched = crate::scheduler::SCHEDULER.lock();
        let cur = sched.current;
        let old = sched.tasks[cur].pml4.replace(image.pml4);
        (cur, old)
    };
    crate::app_lifecycle::record_process_start(cur, &path, &path);

    unsafe { crate::vmm::switch_to(image.pml4) };
    crate::vfs::drop_task_shmem_refs(cur);
    if let Some(old_pml4) = old_pml4 {
        crate::vmm::free_address_space(old_pml4);
    }

    // Replace the return frame so sysretq enters the new program instead of
    // resuming the old one.
    frame.r15 = 0;
    frame.r14 = 0;
    frame.r13 = 0;
    frame.r12 = 0;
    frame.rbx = 0;
    frame.rbp = 0;
    frame.user_rflags = 0x202;
    frame.user_rip = image.entry;
    frame.user_rsp = image.user_rsp;

    0
}

fn sys_dup(fd: u64) -> u64 {
    let new_fd = crate::vfs::vfs_dup(fd as usize);
    if new_fd == usize::MAX {
        u64::MAX
    } else {
        new_fd as u64
    }
}

fn sys_shmem_create(len: u64) -> u64 {
    if len == 0 {
        return u64::MAX;
    }
    if len > crate::resource_limits::MAX_SHMEM_REGION_BYTES as u64 {
        return u64::MAX;
    }
    let id = crate::vfs::vfs_shmem_create(len as usize);
    if id == usize::MAX {
        u64::MAX
    } else {
        id as u64
    }
}

fn sys_shmem_map(id: u64) -> u64 {
    let pml4 = crate::vmm::current_pml4();
    crate::vfs::vfs_shmem_map(id as usize, pml4)
}

fn user_path(path_ptr: *const u8, path_len: u64) -> Option<&'static str> {
    let bytes = user_slice(path_ptr, path_len, MAX_USER_STRING)?;
    core::str::from_utf8(bytes).ok()
}

fn user_descriptor4(desc_ptr: *const u8) -> Option<[u64; 4]> {
    let bytes = user_slice(desc_ptr, 32, 32)?;
    Some([
        u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        u64::from_le_bytes(bytes[24..32].try_into().ok()?),
    ])
}

fn user_descriptor6(desc_ptr: *const u8) -> Option<[u64; 6]> {
    let bytes = user_slice(desc_ptr, 48, 48)?;
    Some([
        u64::from_le_bytes(bytes[0..8].try_into().ok()?),
        u64::from_le_bytes(bytes[8..16].try_into().ok()?),
        u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        u64::from_le_bytes(bytes[24..32].try_into().ok()?),
        u64::from_le_bytes(bytes[32..40].try_into().ok()?),
        u64::from_le_bytes(bytes[40..48].try_into().ok()?),
    ])
}

fn resolve_task_path(path: &str) -> String {
    if path.starts_with('/') {
        return crate::vfs::normalize_path(path);
    }
    let cwd = crate::scheduler::current_cwd();
    if cwd == "/" {
        crate::vfs::normalize_path(&alloc::format!("/{}", path))
    } else {
        crate::vfs::normalize_path(&alloc::format!("{}/{}", cwd, path))
    }
}

fn parse_user_arg_strings(arg_pairs_ptr: u64, argc: u64, max_argc: u64) -> Option<Vec<String>> {
    if argc > max_argc {
        return None;
    }
    let arg_pairs = if argc == 0 {
        &[][..]
    } else {
        let pair_bytes = argc.saturating_mul(16);
        user_slice(arg_pairs_ptr as *const u8, pair_bytes, max_argc * 16)?
    };

    let mut arg_strings = Vec::new();
    if arg_strings.try_reserve_exact(argc as usize).is_err() {
        return None;
    }
    for idx in 0..argc as usize {
        let base = idx * 16;
        let ptr = u64::from_le_bytes(arg_pairs[base..base + 8].try_into().ok()?);
        let len = u64::from_le_bytes(arg_pairs[base + 8..base + 16].try_into().ok()?);
        let bytes = user_slice(ptr as *const u8, len, MAX_USER_STRING)?;
        let arg = core::str::from_utf8(bytes).ok()?;
        arg_strings.push(String::from(arg));
    }
    Some(arg_strings)
}

fn parse_user_fd_mappings(
    fd_pairs_ptr: u64,
    fd_count: u64,
    max_fd_count: u64,
) -> Option<Vec<(usize, usize)>> {
    if fd_count > max_fd_count {
        return None;
    }
    let pairs = if fd_count == 0 {
        &[][..]
    } else {
        let pair_bytes = fd_count.saturating_mul(16);
        user_slice(fd_pairs_ptr as *const u8, pair_bytes, max_fd_count * 16)?
    };
    let mut out = Vec::new();
    if out.try_reserve_exact(fd_count as usize).is_err() {
        return None;
    }
    for idx in 0..fd_count as usize {
        let base = idx * 16;
        let parent_fd = u64::from_le_bytes(pairs[base..base + 8].try_into().ok()?);
        let child_fd = u64::from_le_bytes(pairs[base + 8..base + 16].try_into().ok()?);
        if parent_fd > usize::MAX as u64 || child_fd > usize::MAX as u64 {
            return None;
        }
        out.push((parent_fd as usize, child_fd as usize));
    }
    Some(out)
}

fn write_record_u64(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn append_dir_listing_byte(out: &mut [u8], written: &mut usize, byte: u8) -> bool {
    if *written >= out.len() {
        return false;
    }
    out[*written] = byte;
    *written += 1;
    true
}

fn append_dir_listing_bytes(out: &mut [u8], written: &mut usize, bytes: &[u8]) -> bool {
    for &byte in bytes {
        if !append_dir_listing_byte(out, written, byte) {
            return false;
        }
    }
    true
}

fn append_dir_listing_u64(out: &mut [u8], written: &mut usize, mut value: u64) -> bool {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    if value == 0 {
        return append_dir_listing_byte(out, written, b'0');
    }
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        if !append_dir_listing_byte(out, written, digits[len]) {
            return false;
        }
    }
    true
}

fn sys_yield() {
    // No-op: the preemptive timer will preempt voluntarily yielding tasks.
}

fn valid_user_address_range(ptr: u64, len: u64, max_len: u64) -> bool {
    if ptr == 0 || len == 0 || len > max_len {
        return false;
    }
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };
    end <= crate::vmm::USER_TOP && ptr < crate::vmm::USER_TOP
}

fn validate_user_range(ptr: u64, len: u64, max_len: u64, writable: bool) -> bool {
    valid_user_address_range(ptr, len, max_len)
        && crate::vmm::user_range_accessible(ptr, len, writable)
}

pub fn validate_user_range_for_test(ptr: u64, len: u64, max_len: u64, _writable: bool) -> bool {
    valid_user_address_range(ptr, len, max_len)
}

fn user_slice(ptr: *const u8, len: u64, max_len: u64) -> Option<&'static [u8]> {
    if !validate_user_range(ptr as u64, len, max_len, false) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(ptr, len as usize) })
}

fn user_slice_mut(ptr: *mut u8, len: u64, max_len: u64) -> Option<&'static mut [u8]> {
    if !validate_user_range(ptr as u64, len, max_len, true) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts_mut(ptr, len as usize) })
}

// ── jump_to_userspace ─────────────────────────────────────────────────────────

/// Switch the current ring-0 context to ring-3 by pushing a synthetic iretq
/// frame and executing iretq.  Does not return.
///
/// `entry`    — virtual address of the first ring-3 instruction.
/// `user_rsp` — initial ring-3 stack pointer (must be 16-byte aligned).
#[allow(dead_code)]
pub unsafe fn jump_to_userspace(entry: u64, user_rsp: u64) -> ! {
    let user_cs = crate::gdt::user_code_selector().0 as u64;
    let user_ss = crate::gdt::user_data_selector().0 as u64;
    core::arch::asm!(
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss     = in(reg) user_ss,
        rsp    = in(reg) user_rsp,
        rflags = in(reg) 0x202u64,   // IF=1 (interrupts enabled in ring 3), reserved bit 1
        cs     = in(reg) user_cs,
        rip    = in(reg) entry,
        options(noreturn),
    );
}
