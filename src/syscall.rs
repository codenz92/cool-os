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
///
/// Output path: sys_write pushes bytes into SYSCALL_OUTPUT (a lock-free ring
/// buffer modelled on keyboard.rs). compositor::compose() drains it into the
/// terminal window each frame, avoiding any lock contention with the WM.
use core::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};

// ── Syscall output ring buffer ────────────────────────────────────────────────

const OUTPUT_SIZE: usize = 1024;
const USER_TOP: u64 = 0x0000_8000_0000_0000;
const MAX_USER_STRING: u64 = 4096;
const MAX_USER_BUFFER: u64 = 1024 * 1024;
const MAX_GUI_SURFACE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_USER_DIR_LISTING: u64 = 16 * 1024;
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
        efer.write(efer.read() | 1); // SCE = bit 0

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

    if fd == 1 || fd == 2 {
        for &b in bytes {
            push_output_byte(b);
            // Mirror to QEMU debugcon (port 0xE9) for headless verification.
            unsafe { x86_64::instructions::port::Port::<u8>::new(0xE9).write(b) };
        }
        crate::wm::request_repaint();
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

    if addr == 0 || len == 0 || !valid_user_address_range(addr, len, MAX_USER_BUFFER * 64) {
        return u64::MAX;
    }

    // Round length up to page boundary.
    let len_aligned = (len + 4095) & !4095;

    let mut pte_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & 1 != 0 {
        pte_flags |= PageTableFlags::WRITABLE;
    }

    // Determine the current process's PML4.
    let pml4 = crate::vmm::current_pml4();

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
            let fd = crate::vfs::vfs_open(path);
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
    let Some(buf) = user_slice_mut(buf_ptr, len, MAX_USER_BUFFER) else {
        return u64::MAX;
    };
    let n = crate::vfs::vfs_read_blocking(fd as usize, buf, len as usize);
    if n == usize::MAX {
        u64::MAX
    } else {
        n as u64
    }
}

fn sys_close(fd: u64) {
    let owner = crate::scheduler::current_task_id();
    if !crate::net::socket_close(owner, fd) {
        crate::vfs::vfs_close(fd as usize);
    }
}

fn sys_socket(domain: u64, socket_type: u64, protocol: u64) -> u64 {
    let owner = crate::scheduler::current_task_id();
    crate::net::socket_open(owner, domain, socket_type, protocol).unwrap_or(u64::MAX)
}

fn sys_connect(socket: u64, ipv4_be: u64, port: u64) -> u64 {
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

    match crate::vfs::vfs_create_file(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => {}
        Err(_) => return u64::MAX,
    }

    match crate::vfs::vfs_write_file(path, data) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_create_dir(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    match crate::vfs::vfs_create_dir(path) {
        Ok(()) | Err(crate::fat32::FsError::AlreadyExists) => 0,
        Err(_) => u64::MAX,
    }
}

fn sys_fs_delete_tree(path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    match crate::vfs::vfs_delete_recursive(path) {
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
    let Some(entries) = crate::vfs::vfs_list_dir(path) else {
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
    let Some(path) = user_path(path_ptr, path_len) else {
        return u64::MAX;
    };
    crate::wm::request_focused_screenshot(path);
    0
}

fn sys_waitpid(pid: u64, status_ptr: *mut u64) -> u64 {
    if !status_ptr.is_null() && !validate_user_range(status_ptr as u64, 8, 8, true) {
        return u64::MAX;
    }
    let parent = crate::scheduler::current_task_id();
    match crate::scheduler::waitpid(parent, pid as usize) {
        Ok(code) => unsafe {
            if !status_ptr.is_null() {
                *status_ptr = code;
            }
            pid
        },
        Err(_) => u64::MAX,
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
    match crate::elf::spawn_elf_process_with_args(path, &[]) {
        Ok(pid) => pid as u64,
        Err(_) => u64::MAX,
    }
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
            core::arch::asm!("sti; hlt", options(nomem, nostack));
        }
    }
    crate::wait_queue::wake("timer-sleep", crate::scheduler::current_task_id());
    x86_64::instructions::interrupts::disable();
}

fn sys_dns_resolve(host_ptr: *const u8, host_len: u64) -> u64 {
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
    for byte in request.bytes() {
        push_output_byte(byte);
        unsafe { x86_64::instructions::port::Port::<u8>::new(0xE9).write(byte) };
    }
    request.len() as u64
}

fn sys_exec(frame: &mut SyscallFrame, path_ptr: *const u8, path_len: u64) -> u64 {
    let Some(bytes) = user_slice(path_ptr, path_len, MAX_USER_STRING) else {
        return u64::MAX;
    };
    let path = match core::str::from_utf8(bytes) {
        Ok(path) => path,
        Err(_) => return u64::MAX,
    };

    let image = match crate::elf::load_elf_image(path) {
        Ok(image) => image,
        Err(_) => return u64::MAX,
    };

    let cur = {
        let mut sched = crate::scheduler::SCHEDULER.lock();
        let cur = sched.current;
        sched.tasks[cur].pml4 = Some(image.pml4);
        cur
    };
    crate::app_lifecycle::record_process_start(cur, path, path);

    unsafe { crate::vmm::switch_to(image.pml4) };

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
    end <= USER_TOP && ptr < USER_TOP
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
