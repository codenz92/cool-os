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
