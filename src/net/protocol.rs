pub fn ipv4_string(addr: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (addr >> 24) & 0xff,
        (addr >> 16) & 0xff,
        (addr >> 8) & 0xff,
        addr & 0xff
    )
}

fn handle_ethernet(frame: &[u8]) {
    if frame.len() < 14 {
        return;
    }
    let Some(local_mac) = local_mac() else {
        return;
    };
    let dst = read_mac(&frame[0..6]);
    if dst != local_mac && dst != BROADCAST_MAC {
        return;
    }
    let ethertype = read_be16(frame, 12);
    {
        let mut state = NET_STATE.lock();
        state.rx_tail = (state.rx_tail + 1) % 64;
        state.rx_packets = state.rx_packets.saturating_add(1);
    }
    match ethertype {
        ETHERTYPE_ARP => handle_arp(&frame[14..]),
        ETHERTYPE_IPV4 => handle_ipv4(&frame[14..]),
        _ => {}
    }
}

fn handle_arp(packet: &[u8]) {
    if packet.len() < 28 {
        return;
    }
    let op = read_be16(packet, 6);
    let sender_mac = read_mac(&packet[8..14]);
    let sender_ip = read_be32(packet, 14);
    let target_ip = read_be32(packet, 24);
    remember_arp(sender_ip, sender_mac);

    if op == 1 && target_ip == LOCAL_ADDR {
        let Some(local_mac) = local_mac() else {
            return;
        };
        let mut reply = Vec::with_capacity(28);
        push_be16(&mut reply, 1);
        push_be16(&mut reply, ETHERTYPE_IPV4);
        reply.push(6);
        reply.push(4);
        push_be16(&mut reply, 2);
        reply.extend_from_slice(&local_mac);
        push_be32(&mut reply, LOCAL_ADDR);
        reply.extend_from_slice(&sender_mac);
        push_be32(&mut reply, sender_ip);
        let _ = emit_ethernet(sender_mac, ETHERTYPE_ARP, &reply);
    }
}

fn handle_ipv4(packet: &[u8]) {
    if packet.len() < 20 || packet[0] >> 4 != 4 {
        return;
    }
    let ihl = ((packet[0] & 0x0f) as usize) * 4;
    if ihl < 20 || packet.len() < ihl {
        return;
    }
    let total_len = read_be16(packet, 2) as usize;
    if total_len < ihl || total_len > packet.len() {
        return;
    }
    let protocol = packet[9];
    let src = read_be32(packet, 12);
    let dst = read_be32(packet, 16);
    if dst != LOCAL_ADDR {
        return;
    }
    let payload = &packet[ihl..total_len];
    match protocol {
        IP_PROTO_ICMP => handle_icmp(src, payload),
        IP_PROTO_UDP => handle_udp(src, payload),
        IP_PROTO_TCP => handle_tcp(src, payload),
        _ => {}
    }
}

fn handle_icmp(src: u32, packet: &[u8]) {
    if packet.len() < 8 {
        return;
    }
    match packet[0] {
        0 => {
            let ident = read_be16(packet, 4);
            let mut state = NET_STATE.lock();
            if state.icmp_wait == Some(ident) {
                state.icmp_wait = None;
                state.last_ping_ok = true;
            }
        }
        8 => {
            let mut reply = packet.to_vec();
            reply[0] = 0;
            reply[2] = 0;
            reply[3] = 0;
            let csum = checksum(&reply);
            reply[2] = (csum >> 8) as u8;
            reply[3] = csum as u8;
            let _ = send_ipv4(src, IP_PROTO_ICMP, &reply);
        }
        _ => {}
    }
}

fn handle_udp(_src: u32, packet: &[u8]) {
    if packet.len() < 8 {
        return;
    }
    let dst_port = read_be16(packet, 2);
    let udp_len = read_be16(packet, 4) as usize;
    if udp_len < 8 || udp_len > packet.len() {
        return;
    }
    let payload = &packet[8..udp_len];
    handle_dns_response(dst_port, payload);
}

fn handle_dns_response(dst_port: u16, packet: &[u8]) {
    if packet.len() < 12 {
        return;
    }
    let txid = read_be16(packet, 0);
    let flags = read_be16(packet, 2);
    if flags & 0x8000 == 0 {
        return;
    }
    let addrs = parse_dns_a_records(packet);
    if addrs.is_empty() {
        return;
    };
    let mut state = NET_STATE.lock();
    for wait in state.dns_waits.iter_mut() {
        if wait.txid == txid && wait.port == dst_port {
            wait.result = Some(addrs);
            break;
        }
    }
}

fn handle_tcp(src_ip: u32, packet: &[u8]) {
    if packet.len() < 20 {
        return;
    }
    let src_port = read_be16(packet, 0);
    let dst_port = read_be16(packet, 2);
    let seq = read_be32(packet, 4);
    let ack_no = read_be32(packet, 8);
    let data_offset = ((packet[12] >> 4) as usize) * 4;
    if data_offset < 20 || data_offset > packet.len() {
        return;
    }
    let flags = packet[13];
    let payload = &packet[data_offset..];
    let mut reply = None;
    let mut wake_tasks = Vec::new();

    {
        let mut state = NET_STATE.lock();
        for slot in state.sockets.iter_mut() {
            let Some(socket) = slot.as_mut() else {
                continue;
            };
            if socket.local_port != dst_port {
                continue;
            }
            if socket.remote_ip != 0
                && (socket.remote_ip != src_ip || socket.remote_port != src_port)
            {
                continue;
            }

            if socket.state == TcpState::SynSent && flags & 0x12 == 0x12 {
                socket.remote_ip = src_ip;
                socket.remote_port = src_port;
                socket.seq = ack_no;
                socket.tx_acked = ack_no;
                socket.ack = seq.wrapping_add(1);
                socket.state = TcpState::Established;
                wake_tasks.extend(socket.take_writers());
                reply = Some(TcpReply {
                    src_port: socket.local_port,
                    dst_ip: src_ip,
                    dst_port: src_port,
                    seq: socket.seq,
                    ack: socket.ack,
                    flags: 0x10,
                    window: tcp_receive_window(socket),
                });
                break;
            }

            if socket.state == TcpState::Established || socket.state == TcpState::CloseWait {
                if flags & 0x04 != 0 {
                    socket.peer_closed = true;
                    socket.state = TcpState::Closed;
                    socket.pending_rx.clear();
                    wake_tasks.extend(socket.take_waiters());
                    break;
                }
                if flags & 0x10 != 0 && seq_at_or_after(ack_no, socket.tx_acked) {
                    socket.tx_acked = ack_no;
                    wake_tasks.extend(socket.take_writers());
                }
                let mut should_ack = false;
                if !payload.is_empty() {
                    if receive_tcp_payload(socket, seq, payload) {
                        wake_tasks.extend(socket.take_readers());
                    }
                    should_ack = true;
                }
                if flags & 0x01 != 0 {
                    should_ack = true;
                    let fin_seq = seq.wrapping_add(payload.len() as u32);
                    if fin_seq == socket.ack {
                        socket.ack = socket.ack.wrapping_add(1);
                        socket.peer_closed = true;
                        socket.state = TcpState::CloseWait;
                        wake_tasks.extend(socket.take_readers());
                        wake_tasks.extend(socket.take_writers());
                    }
                }
                if should_ack {
                    reply = Some(TcpReply {
                        src_port: socket.local_port,
                        dst_ip: src_ip,
                        dst_port: src_port,
                        seq: socket.seq,
                        ack: socket.ack,
                        flags: 0x10,
                        window: tcp_receive_window(socket),
                    });
                }
                break;
            }
        }
    }

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
}

fn send_udp(src_port: u16, dst: u32, dst_port: u16, payload: &[u8]) -> Result<(), &'static str> {
    let mut packet = Vec::with_capacity(8 + payload.len());
    push_be16(&mut packet, src_port);
    push_be16(&mut packet, dst_port);
    push_be16(&mut packet, (8 + payload.len()) as u16);
    push_be16(&mut packet, 0); // IPv4 UDP checksum is optional.
    packet.extend_from_slice(payload);
    queue_tx_packet("udp", payload.len());
    send_ipv4(dst, IP_PROTO_UDP, &packet)
}

fn send_tcp_segment(
    src_port: u16,
    dst: u32,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
    payload: &[u8],
) -> Result<(), &'static str> {
    let mut packet = Vec::with_capacity(20 + payload.len());
    push_be16(&mut packet, src_port);
    push_be16(&mut packet, dst_port);
    push_be32(&mut packet, seq);
    push_be32(&mut packet, ack);
    packet.push(5 << 4);
    packet.push(flags);
    push_be16(&mut packet, window);
    push_be16(&mut packet, 0);
    push_be16(&mut packet, 0);
    packet.extend_from_slice(payload);
    let csum = tcp_checksum(LOCAL_ADDR, dst, &packet);
    packet[16] = (csum >> 8) as u8;
    packet[17] = csum as u8;
    queue_tx_packet("tcp", packet.len());
    send_ipv4(dst, IP_PROTO_TCP, &packet)
}

fn send_ipv4(dst: u32, protocol: u8, payload: &[u8]) -> Result<(), &'static str> {
    let ident = {
        let mut state = NET_STATE.lock();
        let ident = state.ip_ident;
        state.ip_ident = state.ip_ident.wrapping_add(1);
        ident
    };
    let total_len = 20 + payload.len();
    if total_len > 1500 {
        return Err("IPv4 packet too large");
    }

    let mut packet = Vec::with_capacity(total_len);
    packet.push(0x45);
    packet.push(0);
    push_be16(&mut packet, total_len as u16);
    push_be16(&mut packet, ident);
    push_be16(&mut packet, 0x4000); // don't fragment
    packet.push(64);
    packet.push(protocol);
    push_be16(&mut packet, 0);
    push_be32(&mut packet, LOCAL_ADDR);
    push_be32(&mut packet, dst);
    let csum = checksum(&packet);
    packet[10] = (csum >> 8) as u8;
    packet[11] = csum as u8;
    packet.extend_from_slice(payload);

    let next_hop = route_next_hop(dst);
    let mac = resolve_mac(next_hop)?;
    emit_ethernet(mac, ETHERTYPE_IPV4, &packet)
}

fn emit_ethernet(dst_mac: [u8; 6], ethertype: u16, payload: &[u8]) -> Result<(), &'static str> {
    let Some(src_mac) = local_mac() else {
        return Err("no local MAC");
    };
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    push_be16(&mut frame, ethertype);
    frame.extend_from_slice(payload);
    crate::virtio_net::transmit(&frame)
}

fn resolve_mac(ip: u32) -> Result<[u8; 6], &'static str> {
    if let Some(mac) = lookup_arp(ip) {
        return Ok(mac);
    }
    send_arp_request(ip)?;
    for _ in 0..POLL_SPINS {
        poll();
        if let Some(mac) = lookup_arp(ip) {
            return Ok(mac);
        }
        core::hint::spin_loop();
    }
    Err("ARP timeout")
}

fn send_arp_request(target_ip: u32) -> Result<(), &'static str> {
    let Some(local_mac) = local_mac() else {
        return Err("no local MAC");
    };
    let mut packet = Vec::with_capacity(28);
    push_be16(&mut packet, 1);
    push_be16(&mut packet, ETHERTYPE_IPV4);
    packet.push(6);
    packet.push(4);
    push_be16(&mut packet, 1);
    packet.extend_from_slice(&local_mac);
    push_be32(&mut packet, LOCAL_ADDR);
    packet.extend_from_slice(&[0; 6]);
    push_be32(&mut packet, target_ip);
    emit_ethernet(BROADCAST_MAC, ETHERTYPE_ARP, &packet)
}

fn route_next_hop(dst: u32) -> u32 {
    if dst & NETMASK == LOCAL_ADDR & NETMASK {
        dst
    } else {
        GATEWAY_ADDR
    }
}

fn remember_arp(ip: u32, mac: [u8; 6]) {
    let mut state = NET_STATE.lock();
    for entry in state.arp_entries.iter_mut() {
        if entry.ip == ip {
            entry.mac = mac;
            entry.tick = crate::interrupts::ticks();
            return;
        }
    }
    state.arp_entries.push(ArpEntry {
        ip,
        mac,
        tick: crate::interrupts::ticks(),
    });
}

fn lookup_arp(ip: u32) -> Option<[u8; 6]> {
    NET_STATE
        .lock()
        .arp_entries
        .iter()
        .find(|entry| entry.ip == ip)
        .map(|entry| entry.mac)
}

fn build_dns_query(txid: u16, host: &str) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    push_be16(&mut out, txid);
    push_be16(&mut out, 0x0100); // recursion desired
    push_be16(&mut out, 1);
    push_be16(&mut out, 0);
    push_be16(&mut out, 0);
    push_be16(&mut out, 0);
    for label in host.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err("invalid DNS label");
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    push_be16(&mut out, 1);
    push_be16(&mut out, 1);
    Ok(out)
}

fn parse_dns_a_records(packet: &[u8]) -> Vec<u32> {
    let qd = read_be16(packet, 4) as usize;
    let an = read_be16(packet, 6) as usize;
    let mut off = 12usize;
    let mut addrs = Vec::new();
    for _ in 0..qd {
        let Some(next) = skip_dns_name(packet, off).and_then(|next| next.checked_add(4)) else {
            return addrs;
        };
        off = next;
        if off > packet.len() {
            return addrs;
        }
    }
    for _ in 0..an {
        let Some(next) = skip_dns_name(packet, off) else {
            return addrs;
        };
        off = next;
        if off + 10 > packet.len() {
            return addrs;
        }
        let typ = read_be16(packet, off);
        let class = read_be16(packet, off + 2);
        let rdlen = read_be16(packet, off + 8) as usize;
        off += 10;
        if off + rdlen > packet.len() {
            return addrs;
        }
        if typ == 1 && class == 1 && rdlen == 4 {
            addrs.push(read_be32(packet, off));
        }
        off += rdlen;
    }
    addrs
}

fn skip_dns_name(packet: &[u8], mut off: usize) -> Option<usize> {
    let mut guard = 0usize;
    loop {
        if off >= packet.len() || guard > 64 {
            return None;
        }
        let len = packet[off];
        if len & 0xc0 == 0xc0 {
            return off.checked_add(2);
        }
        off += 1;
        if len == 0 {
            return Some(off);
        }
        off = off.checked_add(len as usize)?;
        guard += 1;
    }
}

fn take_dns_result(txid: u16, port: u16) -> Option<Vec<u32>> {
    let mut state = NET_STATE.lock();
    let pos = state
        .dns_waits
        .iter()
        .position(|wait| wait.txid == txid && wait.port == port && wait.result.is_some())?;
    state.dns_waits.remove(pos).result
}

fn remove_dns_wait(txid: u16, port: u16) {
    NET_STATE
        .lock()
        .dns_waits
        .retain(|wait| wait.txid != txid || wait.port != port);
}

fn next_dns_txid() -> u16 {
    let mut state = NET_STATE.lock();
    let id = state.next_dns_txid;
    state.next_dns_txid = state.next_dns_txid.wrapping_add(1);
    id
}

fn alloc_ephemeral_port() -> u16 {
    let mut state = NET_STATE.lock();
    state.alloc_port_locked()
}

impl NetState {
    fn alloc_port_locked(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port = if self.next_port >= 60999 {
            49152
        } else {
            self.next_port + 1
        };
        port
    }
}

fn receive_tcp_payload(socket: &mut TcpSocket, seq: u32, payload: &[u8]) -> bool {
    if payload.is_empty() {
        return false;
    }
    if seq == socket.ack {
        if !append_tcp_payload(socket, payload) {
            return false;
        }
        let _ = drain_pending_tcp_payloads(socket);
        return true;
    }
    if seq_before(seq, socket.ack) {
        let offset = socket.ack.wrapping_sub(seq) as usize;
        if offset < payload.len() && append_tcp_payload(socket, &payload[offset..]) {
            let _ = drain_pending_tcp_payloads(socket);
            return true;
        }
        return false;
    }
    store_pending_tcp_payload(socket, seq, payload)
}

fn append_tcp_payload(socket: &mut TcpSocket, payload: &[u8]) -> bool {
    if tcp_buffered_bytes(socket).saturating_add(payload.len()) > TCP_RX_WINDOW {
        return false;
    }
    socket.rx.extend_from_slice(payload);
    socket.ack = socket.ack.wrapping_add(payload.len() as u32);
    true
}

fn drain_pending_tcp_payloads(socket: &mut TcpSocket) -> bool {
    let mut advanced = false;
    loop {
        let Some(pos) = socket
            .pending_rx
            .iter()
            .position(|segment| segment.seq == socket.ack)
        else {
            break;
        };
        let segment = socket.pending_rx.remove(pos);
        if !append_tcp_payload(socket, &segment.data) {
            socket.pending_rx.insert(pos, segment);
            break;
        }
        advanced = true;
    }
    advanced
}

fn store_pending_tcp_payload(socket: &mut TcpSocket, seq: u32, payload: &[u8]) -> bool {
    if socket.pending_rx.len() >= TCP_OUT_OF_ORDER_MAX
        || socket.pending_rx.iter().any(|segment| segment.seq == seq)
        || seq.wrapping_sub(socket.ack) as usize > TCP_RX_WINDOW
        || tcp_buffered_bytes(socket).saturating_add(payload.len()) > TCP_RX_WINDOW
    {
        return false;
    }
    socket.pending_rx.push(TcpPendingSegment {
        seq,
        data: payload.to_vec(),
    });
    true
}

fn tcp_buffered_bytes(socket: &TcpSocket) -> usize {
    socket
        .pending_rx
        .iter()
        .fold(socket.rx.len(), |total, segment| {
            total.saturating_add(segment.data.len())
        })
}

fn tcp_receive_window(socket: &TcpSocket) -> u16 {
    TCP_RX_WINDOW
        .saturating_sub(tcp_buffered_bytes(socket))
        .min(u16::MAX as usize) as u16
}

fn tcp_full_window() -> u16 {
    TCP_RX_WINDOW.min(u16::MAX as usize) as u16
}

fn socket_state(owner: usize, socket_fd: u64) -> Option<TcpState> {
    let state = NET_STATE.lock();
    let idx = socket_index(socket_fd)?;
    let socket = state.sockets.get(idx)?.as_ref()?;
    if socket.owner == owner {
        Some(socket.state)
    } else {
        None
    }
}

fn socket_tx_acked(owner: usize, socket_fd: u64, target_ack: u32) -> Result<bool, &'static str> {
    let state = NET_STATE.lock();
    let idx = socket_index(socket_fd).ok_or("bad socket")?;
    let socket = state
        .sockets
        .get(idx)
        .and_then(Option::as_ref)
        .ok_or("bad socket")?;
    if socket.owner != owner {
        return Err("socket owner mismatch");
    }
    Ok(seq_at_or_after(socket.tx_acked, target_ack))
}

fn seq_at_or_after(current: u32, target: u32) -> bool {
    current == target || current.wrapping_sub(target) < 0x8000_0000
}

fn seq_before(current: u32, target: u32) -> bool {
    current != target && target.wrapping_sub(current) < 0x8000_0000
}

fn socket_mut_locked<'a>(
    state: &'a mut NetState,
    owner: usize,
    socket_fd: u64,
) -> Result<&'a mut TcpSocket, &'static str> {
    let idx = socket_index(socket_fd).ok_or("bad socket")?;
    let socket = state
        .sockets
        .get_mut(idx)
        .and_then(Option::as_mut)
        .ok_or("bad socket")?;
    if socket.owner != owner {
        return Err("socket owner mismatch");
    }
    Ok(socket)
}

fn socket_index(socket_fd: u64) -> Option<usize> {
    if socket_fd < SOCKET_BASE {
        None
    } else {
        Some((socket_fd - SOCKET_BASE) as usize)
    }
}

fn has_link() -> bool {
    ADAPTERS.lock().iter().any(|adapter| adapter.link_up)
}

fn local_mac() -> Option<[u8; 6]> {
    ADAPTERS
        .lock()
        .iter()
        .find(|adapter| adapter.link_up)
        .map(|adapter| adapter.mac)
}

fn parse_ipv4_literal(s: &str) -> Option<u32> {
    let mut value = 0u32;
    let mut parts = 0usize;
    for part in s.split('.') {
        if part.is_empty() || part.len() > 3 {
            return None;
        }
        let mut octet = 0u32;
        for byte in part.bytes() {
            if !byte.is_ascii_digit() {
                return None;
            }
            octet = octet * 10 + (byte - b'0') as u32;
        }
        if octet > 255 {
            return None;
        }
        value = (value << 8) | octet;
        parts += 1;
    }
    if parts == 4 {
        Some(value)
    } else {
        None
    }
}

fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&last) = chunks.remainder().first() {
        sum += (last as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn tcp_checksum(src: u32, dst: u32, tcp: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(12 + tcp.len());
    push_be32(&mut pseudo, src);
    push_be32(&mut pseudo, dst);
    pseudo.push(0);
    pseudo.push(IP_PROTO_TCP);
    push_be16(&mut pseudo, tcp.len() as u16);
    pseudo.extend_from_slice(tcp);
    checksum(&pseudo)
}

fn synthetic_mac(bus: u8, device: u8, function: u8) -> [u8; 6] {
    [0x02, 0x43, 0x4f, bus, device, function]
}

fn mac_string(mac: [u8; 6]) -> String {
    let mut out = String::new();
    for (idx, byte) in mac.iter().enumerate() {
        if idx > 0 {
            out.push(':');
        }
        push_hex_byte(&mut out, *byte);
    }
    out
}

fn read_mac(bytes: &[u8]) -> [u8; 6] {
    [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]]
}

fn read_be16(bytes: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([bytes[off], bytes[off + 1]])
}

fn read_be32(bytes: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

fn push_be16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_be32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_hex_byte(out: &mut String, value: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(HEX[(value >> 4) as usize] as char);
    out.push(HEX[(value & 0x0f) as usize] as char);
}
