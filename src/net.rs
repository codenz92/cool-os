extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use spin::Mutex;

const ETHERTYPE_IPV4: u16 = 0x0800;
const ETHERTYPE_ARP: u16 = 0x0806;
const IP_PROTO_ICMP: u8 = 1;
const IP_PROTO_TCP: u8 = 6;
const IP_PROTO_UDP: u8 = 17;

const LOCAL_ADDR: u32 = 0x0a00_020f; // QEMU user-net default guest address: 10.0.2.15
const NETMASK: u32 = 0xffff_ff00;
const GATEWAY_ADDR: u32 = 0x0a00_0202;
const DNS_ADDR: u32 = 0x0a00_0203;
const EXAMPLE_ADDR: u32 = 0x5db8_d822; // 93.184.216.34
const BROADCAST_MAC: [u8; 6] = [0xff; 6];
const TCP_MSS: usize = 1200;
const SOCKET_BASE: u64 = 1024;
pub(crate) const KERNEL_SOCKET_OWNER: usize = usize::MAX;
const POLL_SPINS: usize = 3_000_000;
const TCP_RETRY_SPINS: usize = POLL_SPINS / 3;
const TCP_MAX_RETRIES: usize = 3;
const HTTP_MAX_BYTES: usize = 128 * 1024;
const HTTP_MAX_REDIRECTS: usize = 5;
const DNS_CACHE_MAX: usize = 16;
const DNS_CACHE_TTL_TICKS: u64 = crate::interrupts::ticks_for_millis(300_000);

#[derive(Clone)]
pub struct NetAdapter {
    pub location: String,
    pub name: String,
    pub driver: &'static str,
    pub mac: [u8; 6],
    pub link_up: bool,
}

#[derive(Clone)]
pub struct HttpResponse {
    pub host: String,
    pub path: String,
    pub final_url: String,
    pub redirect_count: usize,
    pub resolved_addr: u32,
    pub tls_trust_root: Option<&'static str>,
    pub request: String,
    pub status_line: String,
    pub content_type: Option<String>,
    pub body: String,
    pub body_bytes: Vec<u8>,
}

struct NormalizedHttpResponse {
    text: String,
    body_bytes: Vec<u8>,
}

#[derive(Clone)]
struct ArpEntry {
    ip: u32,
    mac: [u8; 6],
    tick: u64,
}

struct DnsWait {
    txid: u16,
    port: u16,
    result: Option<Vec<u32>>,
}

#[derive(Clone)]
struct DnsCacheEntry {
    host: String,
    addrs: Vec<u32>,
    tick: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TcpState {
    Closed,
    SynSent,
    Established,
    CloseWait,
}

struct TcpSocket {
    owner: usize,
    local_port: u16,
    remote_ip: u32,
    remote_port: u16,
    seq: u32,
    tx_acked: u32,
    ack: u32,
    state: TcpState,
    rx: Vec<u8>,
    peer_closed: bool,
}

struct TcpReply {
    src_port: u16,
    dst_ip: u32,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
}

static ADAPTERS: Mutex<Vec<NetAdapter>> = Mutex::new(Vec::new());
static NET_STATE: Mutex<NetState> = Mutex::new(NetState {
    tx_tail: 0,
    rx_tail: 0,
    tx_packets: 0,
    rx_packets: 0,
    dropped: 0,
    ip_ident: 1,
    next_port: 49152,
    next_dns_txid: 0x4300,
    arp_entries: Vec::new(),
    dns_cache: Vec::new(),
    dns_waits: Vec::new(),
    sockets: Vec::new(),
    icmp_wait: None,
    last_ping_ok: false,
});

struct NetState {
    tx_tail: usize,
    rx_tail: usize,
    tx_packets: u64,
    rx_packets: u64,
    dropped: u64,
    ip_ident: u16,
    next_port: u16,
    next_dns_txid: u16,
    arp_entries: Vec<ArpEntry>,
    dns_cache: Vec<DnsCacheEntry>,
    dns_waits: Vec<DnsWait>,
    sockets: Vec<Option<TcpSocket>>,
    icmp_wait: Option<u16>,
    last_ping_ok: bool,
}

pub fn init() {
    let mut adapters = Vec::new();
    match crate::virtio_net::init() {
        Ok(info) => {
            adapters.push(NetAdapter {
                location: info.location,
                name: format!(
                    "virtio network adapter q={} io={:#x}",
                    info.queue_size, info.io_base
                ),
                driver: "virtio-net",
                mac: info.mac,
                link_up: true,
            });
            crate::device_registry::register_virtual(
                "network stack",
                "network",
                "virtio-net link up",
            );
            crate::println!(
                "[net] virtio-net ready driver=virtio-net mac={} local={} gateway={}",
                mac_string(info.mac),
                ipv4_string(LOCAL_ADDR),
                ipv4_string(GATEWAY_ADDR)
            );
            crate::klog::log("network: virtio-net initialized; Ethernet stack online");
        }
        Err(err) => {
            crate::pci::scan(|loc, hdr| {
                if hdr.class == 0x02 {
                    adapters.push(NetAdapter {
                        location: format!("{:02x}:{:02x}.{}", loc.bus, loc.device, loc.function),
                        name: format!("vendor {:04x} device {:04x}", hdr.vendor_id, hdr.device_id),
                        driver: if hdr.vendor_id == 0x1af4 {
                            "virtio-net-modern-unsupported"
                        } else if hdr.vendor_id == 0x8086 {
                            "e1000-unbound"
                        } else {
                            "unbound"
                        },
                        mac: synthetic_mac(loc.bus, loc.device, loc.function),
                        link_up: false,
                    });
                }
            });
            if adapters.is_empty() {
                crate::device_registry::register_virtual(
                    "network stack",
                    "network",
                    "no adapter found",
                );
                crate::println!("[net] offline: no usable adapter ({})", err);
                crate::klog::log_owned(format!("network: no usable adapter ({})", err));
            } else {
                crate::device_registry::register_virtual(
                    "network stack",
                    "network",
                    "adapter detected but no bound driver",
                );
                crate::println!(
                    "[net] offline: adapter detected but driver offline ({})",
                    err
                );
                crate::klog::log_owned(format!(
                    "network: adapter detected, driver offline ({})",
                    err
                ));
            }
        }
    }
    *ADAPTERS.lock() = adapters;
}

pub fn status_lines() -> Vec<String> {
    let adapters = ADAPTERS.lock();
    if adapters.is_empty() {
        let settings = crate::settings_state::snapshot();
        return vec![
            String::from("network: no PCI adapter detected"),
            format!(
                "stack: offline_api={} dns={} http={}",
                if settings.network_offline_api {
                    "on"
                } else {
                    "off"
                },
                if settings.network_dns_enabled {
                    "on"
                } else {
                    "off"
                },
                if settings.network_http_enabled {
                    "on"
                } else {
                    "off"
                }
            ),
        ];
    }

    let mut lines = Vec::new();
    for adapter in adapters.iter() {
        lines.push(format!(
            "{} {} driver={} mac={} link={}",
            adapter.location,
            adapter.name,
            adapter.driver,
            mac_string(adapter.mac),
            if adapter.link_up { "up" } else { "down" }
        ));
    }
    let state = NET_STATE.lock();
    lines.push(format!(
        "rings: tx_tail={} rx_tail={} tx={} rx={} dropped={}",
        state.tx_tail, state.rx_tail, state.tx_packets, state.rx_packets, state.dropped
    ));
    if let Some(stats) = crate::virtio_net::stats() {
        lines.push(format!(
            "virtio: tx={} rx={} tx_errors={} rx_dropped={}",
            stats.tx_packets, stats.rx_packets, stats.tx_errors, stats.rx_dropped
        ));
    }
    lines.push(format!(
        "ipv4: local={} gateway={} dns={}",
        ipv4_string(LOCAL_ADDR),
        ipv4_string(GATEWAY_ADDR),
        ipv4_string(DNS_ADDR)
    ));
    lines.push(format!("dns cache: {} entrie(s)", state.dns_cache.len()));
    for entry in state.arp_entries.iter().take(4) {
        lines.push(format!(
            "arp {} -> {} tick={}",
            ipv4_string(entry.ip),
            mac_string(entry.mac),
            entry.tick
        ));
    }
    lines.push(String::from(
        "stack: Ethernet, ARP, IPv4, ICMP, UDP, DNS, TCP sockets, HTTP, TLS, wget",
    ));
    lines.extend(crate::tls::status_lines());
    lines
}

pub fn protocol_lines() -> Vec<String> {
    let state = NET_STATE.lock();
    let open_sockets = state.sockets.iter().filter(|slot| slot.is_some()).count();
    vec![
        format!("ARP: {} cached entrie(s)", state.arp_entries.len()),
        format!(
            "IPv4: local={} default={} mtu=1500",
            ipv4_string(LOCAL_ADDR),
            ipv4_string(GATEWAY_ADDR)
        ),
        format!(
            "ICMP: last_ping={}",
            if state.last_ping_ok { "ok" } else { "none" }
        ),
        format!(
            "UDP: tx_packets={} rx_packets={} dns_waits={} dns_cache={}",
            state.tx_packets,
            state.rx_packets,
            state.dns_waits.len(),
            state.dns_cache.len()
        ),
        format!("TCP: open_socket(s)={} mss={}", open_sockets, TCP_MSS),
        String::from("TLS: TLS 1.3 over kernel TCP with verified certificate chains"),
        String::from("Socket syscalls: socket/connect/send/recv exposed as 19-22"),
    ]
}

pub fn poll() {
    let frames = crate::virtio_net::poll();
    for frame in frames {
        handle_ethernet(&frame);
    }
}

pub fn queue_tx_packet(kind: &str, bytes: usize) {
    let mut state = NET_STATE.lock();
    if !has_link() {
        state.dropped = state.dropped.saturating_add(1);
        crate::profiler::record("net-drop", kind, &format!("{} bytes", bytes));
        return;
    }
    state.tx_tail = (state.tx_tail + 1) % 64;
    state.tx_packets = state.tx_packets.saturating_add(1);
    crate::profiler::record("net-tx", kind, &format!("{} bytes", bytes));
}

#[allow(dead_code)]
pub fn udp_send(dst: u32, port: u16, payload: &[u8]) -> Result<usize, &'static str> {
    if !has_link() {
        return Err("no network adapter");
    }
    let src_port = alloc_ephemeral_port();
    send_udp(src_port, dst, port, payload)?;
    Ok(payload.len())
}

pub fn dns_resolve(host: &str) -> Result<u32, &'static str> {
    let settings = crate::settings_state::snapshot();
    if !settings.network_dns_enabled {
        return Err("DNS API disabled in Settings");
    }
    let host = host.trim();
    let addrs = resolve_host_addrs(host)?;
    addrs.first().copied().ok_or("DNS returned no address")
}

pub(crate) fn resolve_host_addrs(host: &str) -> Result<Vec<u32>, &'static str> {
    if let Some(addr) = parse_ipv4_literal(host) {
        return Ok(vec![addr]);
    }
    let settings = crate::settings_state::snapshot();
    if !settings.network_dns_enabled {
        return Err("DNS API disabled in Settings");
    }
    if host.is_empty() || host.len() > 253 || host.contains('/') || host.contains(' ') {
        return Err("invalid host");
    }

    if let Some(addrs) = dns_cache_lookup(host) {
        return Ok(addrs);
    }

    if !has_link() {
        if settings.network_offline_api && host == "example.com" {
            queue_tx_packet("dns-offline", host.len());
            let addrs = vec![EXAMPLE_ADDR];
            dns_cache_remember(host, &addrs);
            return Ok(addrs);
        }
        return Err("no network adapter");
    }

    let txid = next_dns_txid();
    let port = alloc_ephemeral_port();
    let query = build_dns_query(txid, host)?;
    {
        let mut state = NET_STATE.lock();
        state.dns_waits.push(DnsWait {
            txid,
            port,
            result: None,
        });
    }
    send_udp(port, DNS_ADDR, 53, &query)?;
    for _ in 0..POLL_SPINS {
        poll();
        if let Some(addr) = take_dns_result(txid, port) {
            dns_cache_remember(host, &addr);
            return Ok(addr);
        }
        core::hint::spin_loop();
    }
    remove_dns_wait(txid, port);
    Err("DNS timeout")
}

fn dns_cache_lookup(host: &str) -> Option<Vec<u32>> {
    let now = crate::interrupts::ticks();
    let mut state = NET_STATE.lock();
    state
        .dns_cache
        .retain(|entry| now.wrapping_sub(entry.tick) <= DNS_CACHE_TTL_TICKS);
    state
        .dns_cache
        .iter()
        .find(|entry| entry.host.eq_ignore_ascii_case(host))
        .map(|entry| entry.addrs.clone())
}

fn dns_cache_remember(host: &str, addrs: &[u32]) {
    if addrs.is_empty() {
        return;
    }
    let now = crate::interrupts::ticks();
    let mut state = NET_STATE.lock();
    if let Some(entry) = state
        .dns_cache
        .iter_mut()
        .find(|entry| entry.host.eq_ignore_ascii_case(host))
    {
        entry.addrs = addrs.to_vec();
        entry.tick = now;
        return;
    }
    state.dns_cache.push(DnsCacheEntry {
        host: String::from(host),
        addrs: addrs.to_vec(),
        tick: now,
    });
    if state.dns_cache.len() > DNS_CACHE_MAX {
        state.dns_cache.remove(0);
    }
}

pub fn http_get(host: &str, path: &str) -> Result<String, &'static str> {
    http_get_response(host, path).map(|response| response.request)
}

pub fn web_get_response(url: &str) -> Result<HttpResponse, &'static str> {
    let (scheme, host, path) = parse_web_url(url)?;
    http_get_response_follow(&scheme, &host, &path, 0)
}

pub fn http_get_response(host: &str, path: &str) -> Result<HttpResponse, &'static str> {
    http_get_response_follow("http", host, path, 0)
}

fn http_get_response_follow(
    scheme: &str,
    host: &str,
    path: &str,
    redirect_count: usize,
) -> Result<HttpResponse, &'static str> {
    let settings = crate::settings_state::snapshot();
    if !settings.network_http_enabled {
        return Err("HTTP API disabled in Settings");
    }
    let host = host.trim();
    if host.is_empty() || host.len() > 253 || host.contains('/') || host.contains(' ') {
        return Err("invalid host");
    }
    let path = if path.is_empty() { "/" } else { path };
    // QEMU user networking currently sees bare google.com close after TLS tickets;
    // use the canonical host that Google's public redirect targets.
    if scheme == "https" && host.eq_ignore_ascii_case("google.com") {
        if redirect_count >= HTTP_MAX_REDIRECTS {
            return Err("HTTP redirect limit reached");
        }
        return http_get_response_follow("https", "www.google.com", path, redirect_count + 1);
    }

    let mut request = String::from("GET ");
    request.push_str(path);
    request.push_str(" HTTP/1.1\r\nHost: ");
    request.push_str(host);
    request.push_str("\r\nUser-Agent: coolOS/19\r\nAccept: text/html,text/plain,image/*,*/*\r\nAccept-Encoding: gzip, identity\r\nConnection: close\r\n\r\n");

    if scheme == "https" {
        if !has_link() {
            return Err("HTTPS requires a network adapter");
        }
        let exchange = crate::tls::https_exchange(host, path, &request, HTTP_MAX_BYTES)?;
        crate::println!(
            "[tls] https {}{} via {} root={}",
            host,
            path,
            ipv4_string(exchange.resolved_addr),
            exchange.trust_root
        );
        let body = normalize_http_response_bytes(&exchange.raw_response)?;
        return finish_web_response(
            scheme,
            host,
            path,
            redirect_count,
            exchange.resolved_addr,
            Some(exchange.trust_root),
            request,
            body,
        );
    }

    let addrs = resolve_host_addrs(host)?;
    let resolved_addr = *addrs.first().ok_or("DNS returned no address")?;

    if !has_link() {
        if settings.network_offline_api {
            let body = format!(
                "HTTP/1.1 200 OK (offline)\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\ncoolOS offline HTTP response from {} at {}",
                host,
                ipv4_string(resolved_addr)
            );
            let body_bytes = response_body_bytes(body.as_bytes())
                .unwrap_or_else(|| body.as_bytes())
                .to_vec();
            return Ok(HttpResponse {
                host: String::from(host),
                path: String::from(path),
                final_url: format!("{}://{}{}", scheme, host, path),
                redirect_count,
                resolved_addr,
                tls_trust_root: None,
                request,
                status_line: String::from("HTTP/1.1 200 OK (offline)"),
                content_type: Some(String::from("text/plain")),
                body,
                body_bytes,
            });
        }
        return Err("no network adapter");
    }

    let mut last_err = "HTTP connect failed";
    let mut body_bytes = Vec::new();
    let mut connected_addr = resolved_addr;
    for addr in addrs {
        let socket = socket_open(KERNEL_SOCKET_OWNER, 2, 1, 6)?;
        match socket_connect(KERNEL_SOCKET_OWNER, socket, addr, 80)
            .and_then(|()| socket_send(KERNEL_SOCKET_OWNER, socket, request.as_bytes()).map(|_| ()))
        {
            Ok(()) => {
                connected_addr = addr;
                let mut buf = [0u8; 512];
                loop {
                    let n = socket_recv(KERNEL_SOCKET_OWNER, socket, &mut buf)?;
                    if n == 0 {
                        if socket_peer_closed(KERNEL_SOCKET_OWNER, socket) {
                            break;
                        }
                        let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
                        return Err("HTTP response timeout");
                    }
                    body_bytes.extend_from_slice(&buf[..n]);
                    if body_bytes.len() > HTTP_MAX_BYTES {
                        break;
                    }
                }
                let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
                break;
            }
            Err(err) => {
                last_err = err;
                let _ = socket_close(KERNEL_SOCKET_OWNER, socket);
            }
        }
    }
    if body_bytes.is_empty() {
        return Err(last_err);
    }

    let body = normalize_http_response_bytes(&body_bytes)?;
    finish_web_response(
        scheme,
        host,
        path,
        redirect_count,
        connected_addr,
        None,
        request,
        body,
    )
}

fn finish_web_response(
    scheme: &str,
    host: &str,
    path: &str,
    redirect_count: usize,
    connected_addr: u32,
    tls_trust_root: Option<&'static str>,
    request: String,
    body: NormalizedHttpResponse,
) -> Result<HttpResponse, &'static str> {
    let status_line = String::from(
        body.text
            .split('\n')
            .next()
            .map(|line| line.trim_end_matches('\r'))
            .unwrap_or("HTTP response"),
    );
    let content_type = http_header_value(&body.text, "content-type");
    let status = http_status_code(&status_line).unwrap_or(0);
    if is_redirect_status(status) {
        if redirect_count >= HTTP_MAX_REDIRECTS {
            return Err("HTTP redirect limit reached");
        }
        let Some(location) = http_header_value(&body.text, "location") else {
            return Err("HTTP redirect missing Location");
        };
        let (next_scheme, next_host, next_path) =
            resolve_web_location(scheme, host, path, &location)?;
        return http_get_response_follow(&next_scheme, &next_host, &next_path, redirect_count + 1);
    }

    Ok(HttpResponse {
        host: String::from(host),
        path: String::from(path),
        final_url: format!("{}://{}{}", scheme, host, path),
        redirect_count,
        resolved_addr: connected_addr,
        tls_trust_root,
        request,
        status_line,
        content_type,
        body: body.text,
        body_bytes: body.body_bytes,
    })
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn http_status_code(status_line: &str) -> Option<u16> {
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
}

fn http_header_value(response: &str, name: &str) -> Option<String> {
    let header_block = response
        .split_once("\r\n\r\n")
        .map(|(headers, _)| headers)
        .or_else(|| response.split_once("\n\n").map(|(headers, _)| headers))
        .unwrap_or(response);
    for line in header_block.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(String::from(value.trim()));
        }
    }
    None
}

fn normalize_http_response_bytes(response: &[u8]) -> Result<NormalizedHttpResponse, &'static str> {
    let Some((headers, body, sep)) = split_http_response_bytes(response) else {
        return Ok(NormalizedHttpResponse {
            text: String::from_utf8_lossy(response).into_owned(),
            body_bytes: response.to_vec(),
        });
    };
    let headers_text = String::from_utf8_lossy(headers);
    let mut body_bytes = body.to_vec();
    if http_header_value(&headers_text, "transfer-encoding")
        .map(|value| header_contains_token(&value, "chunked"))
        .unwrap_or(false)
    {
        body_bytes = decode_chunked_body_bytes(&body_bytes)?;
    }
    if let Some(length) = http_header_value(&headers_text, "content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
    {
        if body_bytes.len() >= length {
            body_bytes.truncate(length);
        }
    }
    let decoded_gzip = http_header_value(&headers_text, "content-encoding")
        .map(|value| header_contains_token(&value, "gzip"))
        .unwrap_or(false);
    if decoded_gzip {
        body_bytes = decode_gzip_body(&body_bytes)?;
    }

    let mut out = headers_without_body_framing(&headers_text);
    if decoded_gzip {
        out.push_str("\r\nContent-Encoding: identity");
        out.push_str("\r\nX-coolOS-Decoded-Encoding: gzip");
    }
    out.push_str("\r\nContent-Length: ");
    push_decimal(&mut out, body_bytes.len() as u64);
    out.push_str(sep);
    out.push_str(&String::from_utf8_lossy(&body_bytes));
    Ok(NormalizedHttpResponse {
        text: out,
        body_bytes,
    })
}

fn headers_without_body_framing(headers: &str) -> String {
    let mut out = String::new();
    for line in headers.lines() {
        if let Some((name, _)) = line.split_once(':') {
            let name = name.trim();
            if name.eq_ignore_ascii_case("transfer-encoding")
                || name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("content-encoding")
            {
                continue;
            }
        }
        if !out.is_empty() {
            out.push_str("\r\n");
        }
        out.push_str(line.trim_end_matches('\r'));
    }
    out
}

fn response_body_bytes(response: &[u8]) -> Option<&[u8]> {
    split_http_response_bytes(response).map(|(_, body, _)| body)
}

fn split_http_response_bytes(response: &[u8]) -> Option<(&[u8], &[u8], &'static str)> {
    if let Some(pos) = find_bytes(response, b"\r\n\r\n") {
        return Some((&response[..pos], &response[pos + 4..], "\r\n\r\n"));
    }
    find_bytes(response, b"\n\n").map(|pos| (&response[..pos], &response[pos + 2..], "\n\n"))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn header_contains_token(value: &str, token: &str) -> bool {
    value
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case(token))
}

fn decode_chunked_body_bytes(body: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        if pos >= body.len() {
            return if out.is_empty() {
                Err("bad chunk header")
            } else {
                Ok(out)
            };
        }
        let line_end = find_crlf(body, pos).ok_or("bad chunk header")?;
        let header = core::str::from_utf8(&body[pos..line_end]).map_err(|_| "bad chunk header")?;
        let size_text = header.split(';').next().unwrap_or("").trim();
        let size = parse_hex_usize(size_text).ok_or("bad chunk size")?;
        pos = line_end + crlf_len(body, line_end);
        if size == 0 {
            break;
        }
        if pos + size > body.len() {
            return Err("truncated chunk body");
        }
        out.extend_from_slice(&body[pos..pos + size]);
        pos += size;
        if pos < body.len() {
            if body.get(pos) == Some(&b'\r') && body.get(pos + 1) == Some(&b'\n') {
                pos += 2;
            } else if body.get(pos) == Some(&b'\n') {
                pos += 1;
            } else {
                return Err("bad chunk terminator");
            }
        }
        if out.len() > HTTP_MAX_BYTES {
            return Err("decoded response too large");
        }
    }
    Ok(out)
}

fn decode_gzip_body(body: &[u8]) -> Result<Vec<u8>, &'static str> {
    if body.len() < 18 || body[0] != 0x1f || body[1] != 0x8b || body[2] != 8 {
        return Err("bad gzip response");
    }
    let flags = body[3];
    let mut pos = 10usize;
    if flags & 0x04 != 0 {
        if pos + 2 > body.len() {
            return Err("bad gzip extra");
        }
        let extra_len = u16::from_le_bytes([body[pos], body[pos + 1]]) as usize;
        pos += 2 + extra_len;
    }
    if flags & 0x08 != 0 {
        pos = skip_gzip_cstring(body, pos)?;
    }
    if flags & 0x10 != 0 {
        pos = skip_gzip_cstring(body, pos)?;
    }
    if flags & 0x02 != 0 {
        pos = pos.checked_add(2).ok_or("bad gzip header")?;
    }
    if pos >= body.len().saturating_sub(8) {
        return Err("bad gzip payload");
    }
    let compressed = &body[pos..body.len() - 8];
    miniz_oxide::inflate::decompress_to_vec_with_limit(compressed, HTTP_MAX_BYTES)
        .map_err(|_| "gzip decode failed")
}

fn skip_gzip_cstring(body: &[u8], mut pos: usize) -> Result<usize, &'static str> {
    while pos < body.len() {
        if body[pos] == 0 {
            return Ok(pos + 1);
        }
        pos += 1;
    }
    Err("bad gzip header")
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            return Some(if i > start && bytes[i - 1] == b'\r' {
                i - 1
            } else {
                i
            });
        }
        i += 1;
    }
    None
}

fn crlf_len(bytes: &[u8], line_end: usize) -> usize {
    if bytes.get(line_end) == Some(&b'\r') && bytes.get(line_end + 1) == Some(&b'\n') {
        2
    } else {
        1
    }
}

fn parse_hex_usize(input: &str) -> Option<usize> {
    let mut value = 0usize;
    let mut saw_digit = false;
    for b in input.bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as usize,
            b'a'..=b'f' => (b - b'a' + 10) as usize,
            b'A'..=b'F' => (b - b'A' + 10) as usize,
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(digit)?;
        saw_digit = true;
    }
    saw_digit.then_some(value)
}

fn push_decimal(out: &mut String, mut value: u64) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    loop {
        digits[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for idx in (0..len).rev() {
        out.push(digits[idx] as char);
    }
}

fn parse_web_url(url: &str) -> Result<(String, String, String), &'static str> {
    if let Some(rest) = url.trim().strip_prefix("http://") {
        let (host, path) = split_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = url.trim().strip_prefix("https://") {
        let (host, path) = split_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    Err("URL must start with http:// or https://")
}

fn resolve_web_location(
    base_scheme: &str,
    base_host: &str,
    base_path: &str,
    location: &str,
) -> Result<(String, String, String), &'static str> {
    let location = location.trim();
    if let Some(rest) = location.strip_prefix("https://") {
        let (host, path) = split_web_host_path("https", rest)?;
        return Ok((String::from("https"), host, path));
    }
    if let Some(rest) = location.strip_prefix("http://") {
        let (host, path) = split_web_host_path("http", rest)?;
        return Ok((String::from("http"), host, path));
    }
    if let Some(rest) = location.strip_prefix("//") {
        let (host, path) = split_web_host_path(base_scheme, rest)?;
        return Ok((String::from(base_scheme), host, path));
    }
    if location.starts_with('/') {
        return Ok((
            String::from(base_scheme),
            String::from(base_host),
            String::from(location),
        ));
    }
    let mut dir = String::from(base_path);
    if let Some(pos) = dir.rfind('/') {
        dir.truncate(pos + 1);
    } else {
        dir = String::from("/");
    }
    dir.push_str(location);
    Ok((String::from(base_scheme), String::from(base_host), dir))
}

fn split_web_host_path(scheme: &str, rest: &str) -> Result<(String, String), &'static str> {
    let slash = rest.find('/').unwrap_or(rest.len());
    let host = rest[..slash].trim();
    if host.is_empty() || host.len() > 253 || host.contains(' ') {
        return Err("invalid redirect host");
    }
    if let Some((name, port)) = host.rsplit_once(':') {
        let expected_port = if scheme == "https" { "443" } else { "80" };
        if port != expected_port {
            return Err("web redirect port unsupported");
        }
        if name.is_empty() {
            return Err("invalid redirect host");
        }
        let path = if slash < rest.len() {
            &rest[slash..]
        } else {
            "/"
        };
        return Ok((String::from(name), String::from(path)));
    }
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        "/"
    };
    Ok((String::from(host), String::from(path)))
}

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
        peer_closed: false,
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
        send_tcp_segment(local_port, remote_ip, remote_port, seq, 0, 0x02, &[])?;
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
        let (local_port, remote_ip, remote_port, seq, ack) = {
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
        {
            let mut state = NET_STATE.lock();
            let socket = socket_mut_locked(&mut state, owner, socket_fd)?;
            if !socket.rx.is_empty() {
                let n = out.len().min(socket.rx.len());
                out[..n].copy_from_slice(&socket.rx[..n]);
                socket.rx.drain(0..n);
                return Ok(n);
            }
            if socket.peer_closed {
                return Ok(0);
            }
        }
        poll();
        core::hint::spin_loop();
    }
    Ok(0)
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
    let mut state = NET_STATE.lock();
    let Some(slot) = state.sockets.get_mut(idx) else {
        return false;
    };
    if slot.as_ref().map(|s| s.owner == owner).unwrap_or(false) {
        *slot = None;
        true
    } else {
        false
    }
}

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
                reply = Some(TcpReply {
                    src_port: socket.local_port,
                    dst_ip: src_ip,
                    dst_port: src_port,
                    seq: socket.seq,
                    ack: socket.ack,
                    flags: 0x10,
                });
                break;
            }

            if socket.state == TcpState::Established || socket.state == TcpState::CloseWait {
                if flags & 0x10 != 0 && seq_at_or_after(ack_no, socket.tx_acked) {
                    socket.tx_acked = ack_no;
                }
                let mut ack = socket.ack;
                if !payload.is_empty() && seq == socket.ack {
                    socket.rx.extend_from_slice(payload);
                    ack = seq.wrapping_add(payload.len() as u32);
                }
                if flags & 0x01 != 0 {
                    ack = ack.wrapping_add(1);
                    socket.peer_closed = true;
                    socket.state = TcpState::CloseWait;
                }
                if ack != socket.ack || flags & 0x01 != 0 {
                    socket.ack = ack;
                    reply = Some(TcpReply {
                        src_port: socket.local_port,
                        dst_ip: src_ip,
                        dst_port: src_port,
                        seq: socket.seq,
                        ack: socket.ack,
                        flags: 0x10,
                    });
                }
                break;
            }
        }
    }

    if let Some(reply) = reply {
        let _ = send_tcp_segment(
            reply.src_port,
            reply.dst_ip,
            reply.dst_port,
            reply.seq,
            reply.ack,
            reply.flags,
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
    payload: &[u8],
) -> Result<(), &'static str> {
    let mut packet = Vec::with_capacity(20 + payload.len());
    push_be16(&mut packet, src_port);
    push_be16(&mut packet, dst_port);
    push_be32(&mut packet, seq);
    push_be32(&mut packet, ack);
    packet.push(5 << 4);
    packet.push(flags);
    push_be16(&mut packet, 4096);
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
