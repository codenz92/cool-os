pub fn icmp_ping(dst: u32) -> Result<(), &'static str> {
    if !has_link() {
        return Err("no network adapter");
    }
    let ident = crate::interrupts::ticks() as u16 ^ 0x434f;
    {
        let mut state = NET_STATE.lock();
        state.icmp_wait = Some(ident);
        state.last_ping_ok = false;
    }
    let mut packet = Vec::new();
    packet.push(8); // echo request
    packet.push(0);
    packet.extend_from_slice(&[0, 0]);
    push_be16(&mut packet, ident);
    push_be16(&mut packet, 1);
    packet.extend_from_slice(b"coolOS");
    let csum = checksum(&packet);
    packet[2] = (csum >> 8) as u8;
    packet[3] = csum as u8;
    send_ipv4(dst, IP_PROTO_ICMP, &packet)?;

    for _ in 0..POLL_SPINS {
        poll();
        if NET_STATE.lock().last_ping_ok {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("ICMP timeout")
}

pub fn socket_open(
    owner: usize,
    _domain: u64,
    socket_type: u64,
    protocol: u64,
) -> Result<u64, &'static str> {
    if socket_type != 1 || !(protocol == 0 || protocol == 6) {
        return Err("only TCP stream sockets are supported");
    }
    let mut state = NET_STATE.lock();
    let total_open = state.sockets.iter().filter(|slot| slot.is_some()).count();
    if total_open >= crate::resource_limits::MAX_SOCKETS_TOTAL {
        return Err("socket table full");
    }
    if owner != KERNEL_SOCKET_OWNER {
        let owner_open = state
            .sockets
            .iter()
            .flatten()
            .filter(|socket| socket.owner == owner)
            .count();
        if owner_open >= crate::resource_limits::MAX_SOCKETS_PER_TASK {
            return Err("task socket limit reached");
        }
    }
    let local_port = state.alloc_port_locked();
    let socket = TcpSocket {
        owner,
        local_port,
        remote_ip: 0,
        remote_port: 0,
        seq: 0x1000_0000u32.wrapping_add(local_port as u32),
        tx_acked: 0x1000_0000u32.wrapping_add(local_port as u32),
        ack: 0,
        state: TcpState::Closed,
        rx: Vec::new(),
        pending_rx: Vec::new(),
        peer_closed: false,
        waiting_readers: Vec::new(),
        waiting_writers: Vec::new(),
    };
    for (idx, slot) in state.sockets.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(socket);
            return Ok(SOCKET_BASE + idx as u64);
        }
    }
    state.sockets.push(Some(socket));
    Ok(SOCKET_BASE + (state.sockets.len() - 1) as u64)
}

pub fn socket_connect(
    owner: usize,
    socket_fd: u64,
    remote_ip: u32,
    remote_port: u16,
) -> Result<(), &'static str> {
    if !has_link() {
        return Err("no network adapter");
    }
    let (local_port, seq) = {
        let mut state = NET_STATE.lock();
        let socket = socket_mut_locked(&mut state, owner, socket_fd)?;
        socket.remote_ip = remote_ip;
        socket.remote_port = remote_port;
        socket.state = TcpState::SynSent;
        (socket.local_port, socket.seq)
    };
    for _attempt in 0..TCP_MAX_RETRIES {
        send_tcp_segment(
            local_port,
            remote_ip,
            remote_port,
            seq,
            0,
            0x02,
            tcp_full_window(),
            &[],
        )?;
        for _ in 0..TCP_RETRY_SPINS {
            poll();
            if socket_state(owner, socket_fd) == Some(TcpState::Established) {
                return Ok(());
            }
            core::hint::spin_loop();
        }
    }
    Err("TCP connect timeout")
}

pub fn socket_send(owner: usize, socket_fd: u64, bytes: &[u8]) -> Result<usize, &'static str> {
    if bytes.is_empty() {
        return Ok(0);
    }
    let mut sent = 0usize;
    while sent < bytes.len() {
        let take = (bytes.len() - sent).min(TCP_MSS);
        let (local_port, remote_ip, remote_port, seq, ack, window) = {
            let mut state = NET_STATE.lock();
            let socket = socket_mut_locked(&mut state, owner, socket_fd)?;
            if socket.state != TcpState::Established {
                return Err("socket not established");
            }
            let seq = socket.seq;
            socket.seq = socket.seq.wrapping_add(take as u32);
            (
                socket.local_port,
                socket.remote_ip,
                socket.remote_port,
                seq,
                socket.ack,
                tcp_receive_window(socket),
            )
        };
        let target_ack = seq.wrapping_add(take as u32);
        let mut delivered = false;
        for _attempt in 0..TCP_MAX_RETRIES {
            send_tcp_segment(
                local_port,
                remote_ip,
                remote_port,
                seq,
                ack,
                0x18,
                window,
                &bytes[sent..sent + take],
            )?;
            for _ in 0..TCP_RETRY_SPINS {
                poll();
                if socket_tx_acked(owner, socket_fd, target_ack)? {
                    delivered = true;
                    break;
                }
                core::hint::spin_loop();
            }
            if delivered {
                break;
            }
        }
        if !delivered {
            return Err("TCP send timeout");
        }
        sent += take;
    }
    Ok(sent)
}

pub fn socket_recv(owner: usize, socket_fd: u64, out: &mut [u8]) -> Result<usize, &'static str> {
    if out.is_empty() {
        return Ok(0);
    }
    for _ in 0..POLL_SPINS {
        let mut window_update = None;
        let mut received = None;
        {
            let mut state = NET_STATE.lock();
            let socket = socket_mut_locked(&mut state, owner, socket_fd)?;
            if !socket.rx.is_empty() {
                let old_window = tcp_receive_window(socket);
                let n = out.len().min(socket.rx.len());
                out[..n].copy_from_slice(&socket.rx[..n]);
                socket.rx.drain(0..n);
                let new_window = tcp_receive_window(socket);
                if old_window == 0
                    && new_window > 0
                    && socket.remote_ip != 0
                    && (socket.state == TcpState::Established
                        || socket.state == TcpState::CloseWait)
                {
                    window_update = Some(TcpReply {
                        src_port: socket.local_port,
                        dst_ip: socket.remote_ip,
                        dst_port: socket.remote_port,
                        seq: socket.seq,
                        ack: socket.ack,
                        flags: 0x10,
                        window: new_window,
                    });
                }
                received = Some(n);
            }
            if received.is_none() && socket.peer_closed {
                return Ok(0);
            }
        }
        if let Some(reply) = window_update {
            let _ = send_tcp_segment(
                reply.src_port,
                reply.dst_ip,
                reply.dst_port,
                reply.seq,
                reply.ack,
                reply.flags,
                reply.window,
                &[],
            );
        }
        if let Some(n) = received {
            return Ok(n);
        }
        poll();
        core::hint::spin_loop();
    }
    Ok(0)
}

pub fn socket_poll_revents(owner: usize, socket_fd: u64, events: u64) -> u64 {
    let state = NET_STATE.lock();
    let Some(idx) = socket_index(socket_fd) else {
        return crate::evented::EVENT_ERROR;
    };
    let Some(socket) = state.sockets.get(idx).and_then(Option::as_ref) else {
        return crate::evented::EVENT_ERROR;
    };
    if socket.owner != owner {
        return crate::evented::EVENT_ERROR;
    }
    socket.revents(events)
}

pub fn socket_register_waiter(owner: usize, socket_fd: u64, task_id: usize, events: u64) -> bool {
    let mut state = NET_STATE.lock();
    let Ok(socket) = socket_mut_locked(&mut state, owner, socket_fd) else {
        return false;
    };
    if events & crate::evented::EVENT_READ != 0 {
        socket.add_reader_waiter(task_id);
    }
    if events & crate::evented::EVENT_WRITE != 0 {
        socket.add_writer_waiter(task_id);
    }
    true
}

pub fn socket_unregister_waiter(owner: usize, socket_fd: u64, task_id: usize) {
    let mut state = NET_STATE.lock();
    if let Ok(socket) = socket_mut_locked(&mut state, owner, socket_fd) {
        socket.remove_waiter(task_id);
    }
}

pub fn socket_peer_closed(owner: usize, socket_fd: u64) -> bool {
    let state = NET_STATE.lock();
    let Some(idx) = socket_index(socket_fd) else {
        return true;
    };
    state
        .sockets
        .get(idx)
        .and_then(Option::as_ref)
        .map(|socket| socket.owner == owner && socket.peer_closed)
        .unwrap_or(true)
}

pub fn socket_close(owner: usize, socket_fd: u64) -> bool {
    let Some(idx) = socket_index(socket_fd) else {
        return false;
    };
    let (reply, wake_tasks) = {
        let mut state = NET_STATE.lock();
        let Some(slot) = state.sockets.get_mut(idx) else {
            return false;
        };
        let Some(socket) = slot.as_mut() else {
            return false;
        };
        if socket.owner != owner {
            return false;
        }
        let reply = if socket.remote_ip != 0
            && (socket.state == TcpState::Established || socket.state == TcpState::CloseWait)
        {
            Some(TcpReply {
                src_port: socket.local_port,
                dst_ip: socket.remote_ip,
                dst_port: socket.remote_port,
                seq: socket.seq,
                ack: socket.ack,
                flags: 0x11,
                window: tcp_receive_window(socket),
            })
        } else {
            None
        };
        let wake_tasks = socket.take_waiters();
        *slot = None;
        (reply, wake_tasks)
    };
    crate::evented::wake_tasks("socket", wake_tasks);
    if let Some(reply) = reply {
        let _ = send_tcp_segment(
            reply.src_port,
            reply.dst_ip,
            reply.dst_port,
            reply.seq,
            reply.ack,
            reply.flags,
            reply.window,
            &[],
        );
    }
    true
}

pub fn close_owner_sockets(owner: usize) -> usize {
    let (replies, wake_tasks, closed) = {
        let mut state = NET_STATE.lock();
        let mut replies = Vec::new();
        let mut wake_tasks = Vec::new();
        let mut closed = 0usize;
        for slot in state.sockets.iter_mut() {
            let Some(socket) = slot.as_mut() else {
                continue;
            };
            if socket.owner != owner {
                continue;
            }
            if socket.remote_ip != 0
                && (socket.state == TcpState::Established || socket.state == TcpState::CloseWait)
            {
                replies.push(TcpReply {
                    src_port: socket.local_port,
                    dst_ip: socket.remote_ip,
                    dst_port: socket.remote_port,
                    seq: socket.seq,
                    ack: socket.ack,
                    flags: 0x11,
                    window: tcp_receive_window(socket),
                });
            }
            wake_tasks.extend(socket.take_waiters());
            *slot = None;
            closed = closed.saturating_add(1);
        }
        (replies, wake_tasks, closed)
    };
    crate::evented::wake_tasks("socket-owner-close", wake_tasks);
    for reply in replies {
        let _ = send_tcp_segment(
            reply.src_port,
            reply.dst_ip,
            reply.dst_port,
            reply.seq,
            reply.ack,
            reply.flags,
            reply.window,
            &[],
        );
    }
    closed
}
