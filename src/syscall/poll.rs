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
