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
/// space with the given protection flags (bit 0 = writable, bit 1 =
/// executable). Writable+executable is rejected. Allocates physical frames and
/// inserts PTEs. Returns `addr` on success, `u64::MAX` on failure.
fn sys_mmap(addr: u64, len: u64, flags: u64) -> u64 {
    use x86_64::{structures::paging::PageTableFlags, VirtAddr};

    if addr == 0 || len == 0 {
        return u64::MAX;
    }
    if flags & !(USER_PROT_WRITE | USER_PROT_EXEC) != 0 {
        return u64::MAX;
    }
    if flags & USER_PROT_WRITE != 0 && flags & USER_PROT_EXEC != 0 {
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
    if flags & USER_PROT_WRITE != 0 {
        pte_flags |= PageTableFlags::WRITABLE;
    }
    if flags & USER_PROT_EXEC == 0 {
        pte_flags |= PageTableFlags::NO_EXECUTE;
    }

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

/// Change protections for existing mmap-arena pages in the calling process.
/// This keeps executable shared-object mappings W^X by rejecting write+exec and
/// by requiring the range to live inside the explicit userspace mmap arena.
fn sys_mprotect(addr: u64, len: u64, flags: u64) -> u64 {
    use x86_64::{structures::paging::PageTableFlags, VirtAddr};

    if addr == 0 || len == 0 || addr & 0xfff != 0 {
        return u64::MAX;
    }
    if flags & !(USER_PROT_WRITE | USER_PROT_EXEC) != 0 {
        return u64::MAX;
    }
    if flags & USER_PROT_WRITE != 0 && flags & USER_PROT_EXEC != 0 {
        return u64::MAX;
    }

    let Some(len_aligned) = len.checked_add(4095).map(|value| value & !4095) else {
        return u64::MAX;
    };
    if !valid_user_address_range(addr, len_aligned, MAX_USER_BUFFER * 64)
        || !crate::vmm::valid_user_mmap_range(addr, len_aligned)
    {
        return u64::MAX;
    }

    let mut pte_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & USER_PROT_WRITE != 0 {
        pte_flags |= PageTableFlags::WRITABLE;
    }
    if flags & USER_PROT_EXEC == 0 {
        pte_flags |= PageTableFlags::NO_EXECUTE;
    }

    match crate::vmm::protect_region(
        crate::vmm::current_pml4(),
        VirtAddr::new(addr),
        len_aligned,
        pte_flags,
    ) {
        Ok(()) => 0,
        Err(_) => u64::MAX,
    }
}

/// Map file bytes into the calling process as private read-only pages. Phase 77
/// intentionally exposes read-only and executable mappings only; writable file
/// mappings need dirty-page tracking and fsync semantics before they can be
/// made correct.
fn sys_mmap_file(desc_ptr: *const u8) -> u64 {
    use x86_64::{structures::paging::PageTableFlags, VirtAddr};

    let Some(desc) = user_descriptor5(desc_ptr) else {
        return u64::MAX;
    };
    let fd = desc[0];
    let addr = desc[1];
    let len = desc[2];
    let file_offset = desc[3];
    let flags = desc[4];

    if fd > usize::MAX as u64
        || addr == 0
        || len == 0
        || addr & 0xfff != 0
        || file_offset & 0xfff != 0
    {
        return u64::MAX;
    }
    if flags & !USER_PROT_EXEC != 0 {
        return u64::MAX;
    }

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

    let pml4 = crate::vmm::current_pml4();
    let pages = len_aligned.saturating_div(4096) as usize;
    if !crate::vmm::can_add_owned_pages(pml4, pages) {
        return u64::MAX;
    }

    let mut pte_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & USER_PROT_EXEC == 0 {
        pte_flags |= PageTableFlags::NO_EXECUTE;
    }

    let mut offset = 0u64;
    while offset < len_aligned {
        let Some(page_offset) = file_offset.checked_add(offset) else {
            return u64::MAX;
        };
        let Some(frame) = crate::vmm::alloc_zeroed_frame() else {
            return u64::MAX;
        };
        let ptr = crate::vmm::phys_to_virt(frame.start_address()).as_mut_ptr::<u8>();
        let dst = unsafe { core::slice::from_raw_parts_mut(ptr, 4096) };
        let n = crate::vfs::vfs_read_fd_at(fd as usize, page_offset, dst);
        if n == usize::MAX {
            crate::vmm::free_unmapped_frame(frame);
            return u64::MAX;
        }
        if crate::vmm::map_file_frame_in(pml4, VirtAddr::new(addr + offset), frame, pte_flags)
            .is_err()
        {
            crate::vmm::free_unmapped_frame(frame);
            return u64::MAX;
        }
        offset += 4096;
    }

    addr
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

fn sys_thread_spawn(entry: u64, arg: u64, flags: u64) -> u64 {
    sys_thread_spawn_common(entry, arg, 0, flags)
}

fn sys_thread_spawn_tls(desc_ptr: *const u8) -> u64 {
    let Some(desc) = user_descriptor4(desc_ptr) else {
        return u64::MAX;
    };
    let entry = desc[0];
    let arg = desc[1];
    let tls_base = desc[2];
    let flags = desc[3];
    sys_thread_spawn_common(entry, arg, tls_base, flags)
}

fn sys_thread_spawn_common(entry: u64, arg: u64, tls_base: u64, flags: u64) -> u64 {
    use x86_64::{structures::paging::PageTableFlags, VirtAddr};

    if flags != 0
        || entry == 0
        || !valid_user_address_range(entry, 1, 1)
        || !crate::vmm::user_range_accessible(entry, 1, false)
        || !valid_user_tls_base(tls_base)
    {
        return u64::MAX;
    }
    let Some(pml4) = crate::scheduler::current_user_pml4() else {
        return u64::MAX;
    };
    if !crate::scheduler::can_spawn_user_task() {
        return u64::MAX;
    }
    let Some(slot) = crate::scheduler::next_user_thread_stack_slot() else {
        return u64::MAX;
    };
    let Some((stack_bottom, stack_top)) = crate::vmm::user_thread_stack_range(slot) else {
        return u64::MAX;
    };

    let flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;
    if !crate::vmm::can_add_owned_pages(pml4, (crate::vmm::USER_STACK_SIZE / 4096) as usize) {
        return u64::MAX;
    }
    if crate::vmm::map_region(
        pml4,
        VirtAddr::new(stack_bottom),
        crate::vmm::USER_STACK_SIZE,
        flags,
    )
    .is_err()
    {
        return u64::MAX;
    }

    let user_rsp = stack_top - 8;
    if !validate_user_range(user_rsp, 8, 8, true) {
        return u64::MAX;
    }
    unsafe {
        core::ptr::write_volatile(user_rsp as *mut u64, 0);
    }

    crate::scheduler::spawn_user_thread(entry, user_rsp, arg, stack_bottom, tls_base)
        .map(|tid| tid as u64)
        .unwrap_or(u64::MAX)
}

fn sys_thread_tls_set(base: u64, flags: u64) -> u64 {
    if flags != 0 || !valid_user_tls_base(base) {
        return u64::MAX;
    }
    if crate::scheduler::set_current_tls_base(base) {
        0
    } else {
        u64::MAX
    }
}

fn sys_thread_tls_get() -> u64 {
    crate::scheduler::current_tls_base()
}

fn valid_user_tls_base(base: u64) -> bool {
    if base == 0 {
        return true;
    }
    base & 7 == 0
        && valid_user_address_range(base, 8, 8)
        && crate::vmm::user_range_accessible(base, 8, true)
}

fn sys_futex_wait(addr: u64, expected: u64, timeout_ms: u64) -> u64 {
    let task_id = crate::scheduler::current_task_id();
    let Some(value) = read_user_futex(addr) else {
        return u64::MAX;
    };
    if value != expected {
        crate::futex::record_mismatch(addr, task_id);
        return FUTEX_WAIT_MISMATCH;
    }
    if timeout_ms == 0 {
        crate::futex::record_timeout(addr, task_id);
        return FUTEX_WAIT_TIMEOUT;
    }
    if !crate::futex::register_waiter(addr, task_id) {
        return u64::MAX;
    }
    if read_user_futex(addr).unwrap_or(u64::MAX) != expected {
        crate::futex::unregister_waiter(addr, task_id);
        crate::futex::record_mismatch(addr, task_id);
        return FUTEX_WAIT_MISMATCH;
    }

    let deadline = futex_deadline(timeout_ms);
    crate::wait_queue::wait("futex", task_id);
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
    x86_64::instructions::interrupts::disable();

    let still_waiting = crate::futex::unregister_waiter(addr, task_id);
    if still_waiting {
        crate::wait_queue::wake("futex", task_id);
    }
    if crate::scheduler::current_has_pending_signal() {
        if still_waiting {
            crate::futex::record_interrupted(addr, task_id);
        }
        return u64::MAX;
    }
    if still_waiting && futex_deadline_expired(deadline) {
        crate::futex::record_timeout(addr, task_id);
        return FUTEX_WAIT_TIMEOUT;
    }
    0
}

fn sys_futex_wake(addr: u64, count: u64, flags: u64) -> u64 {
    if flags != 0
        || count > MAX_FUTEX_WAKE_COUNT
        || !validate_user_range(addr, 8, 8, false)
        || addr & 7 != 0
    {
        return u64::MAX;
    }
    let tasks = crate::futex::wake(addr, count as usize);
    let woken = tasks.len() as u64;
    crate::evented::wake_tasks("futex", tasks);
    woken
}

fn read_user_futex(addr: u64) -> Option<u64> {
    if addr & 7 != 0 || !validate_user_range(addr, 8, 8, false) {
        return None;
    }
    Some(unsafe { core::ptr::read_volatile(addr as *const u64) })
}

fn futex_deadline(timeout_ms: u64) -> Option<u64> {
    if timeout_ms == crate::evented::TIMEOUT_FOREVER {
        None
    } else {
        Some(
            crate::interrupts::ticks()
                .wrapping_add(crate::interrupts::ticks_for_millis(timeout_ms.max(1))),
        )
    }
}

fn futex_deadline_expired(deadline: Option<u64>) -> bool {
    deadline
        .map(|deadline| crate::interrupts::ticks() >= deadline)
        .unwrap_or(false)
}
