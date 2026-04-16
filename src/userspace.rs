/// Userspace demo task (Phase 9).
///
/// `userspace_demo_task` runs as a normal scheduler task (ring 0) that
/// immediately transfers to ring 3 via iretq.  The ring-3 stub makes two
/// syscalls: write (prints a hello message to the terminal) and exit (marks
/// the task Blocked so the scheduler never picks it up again).

// 64 KiB stack for the ring-3 stub.
const USER_STACK_SIZE: usize = 64 * 1024;
static mut USER_STACK: [u8; USER_STACK_SIZE] = [0; USER_STACK_SIZE];

/// Ring-3 stub: write hello then exit.
///
/// This function is in the kernel binary's .text section but executes at
/// privilege level 3.  All pages are marked user-accessible in Phase 9
/// (single address space), so both the code and the USER_STACK are reachable
/// from ring 3.  Phase 10 will introduce per-process page tables.
fn user_stub() -> ! {
    const MSG: &[u8] = b"[ring 3] Hello from userspace!\n";
    unsafe {
        // sys_write(1, buf, len)
        core::arch::asm!(
            "syscall",
            inout("rax") 1u64 => _,
            in("rdi") 1u64,
            in("rsi") MSG.as_ptr() as u64,
            in("rdx") MSG.len() as u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        // sys_exit(0)
        core::arch::asm!(
            "syscall",
            inout("rax") 0u64 => _,
            in("rdi") 0u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
    // Spin until the next timer tick preempts this now-Blocked task.
    loop {
        core::hint::spin_loop();
    }
}

/// Scheduler task entry point: switches to ring 3 and runs `user_stub`.
pub fn userspace_demo_task() -> ! {
    let stack_top = core::ptr::addr_of!(USER_STACK) as u64 + USER_STACK_SIZE as u64;
    unsafe { crate::syscall::jump_to_userspace(user_stub as *const () as u64, stack_top) }
}
