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
        sched.tasks[cur].tls_base = 0;
        (cur, old)
    };
    crate::app_lifecycle::record_process_start(cur, &path, &path);

    unsafe { crate::vmm::switch_to(image.pml4) };
    let _ = crate::scheduler::set_current_tls_base(0);
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
