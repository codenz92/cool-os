extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use core::mem;
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
const TCP_RX_WINDOW: usize = 32 * 1024;
const TCP_OUT_OF_ORDER_MAX: usize = 8;
const SOCKET_BASE: u64 = 1024;
pub(crate) const KERNEL_SOCKET_OWNER: usize = usize::MAX;
const POLL_SPINS: usize = 3_000_000;
const TCP_RETRY_SPINS: usize = POLL_SPINS / 3;
const TCP_MAX_RETRIES: usize = 3;
const HTTP_MAX_BYTES: usize = 512 * 1024;
const HTTP_MAX_REQUEST_BODY: usize = 64 * 1024;
const HTTP_MAX_REDIRECTS: usize = 5;
const DNS_CACHE_MAX: usize = 16;
const DNS_CACHE_TTL_TICKS: u64 = crate::interrupts::ticks_for_millis(300_000);

#[derive(Clone, Copy)]
pub struct NetResourceStats {
    pub open_sockets: usize,
    pub socket_slots: usize,
    pub kernel_owned_sockets: usize,
    pub max_sockets_per_task: usize,
    pub max_sockets_total: usize,
}

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
    pub session_cookies_stored: usize,
    pub body: String,
    pub body_bytes: Vec<u8>,
}

struct NormalizedHttpResponse {
    text: String,
    body_bytes: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpRequestMethod {
    Get,
    Post,
}

impl HttpRequestMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

struct HttpRequestSession {
    enabled: bool,
    stored_cookies: usize,
}

impl HttpRequestSession {
    fn disabled() -> Self {
        Self {
            enabled: false,
            stored_cookies: 0,
        }
    }

    fn enabled() -> Self {
        Self {
            enabled: true,
            stored_cookies: 0,
        }
    }
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
    pending_rx: Vec<TcpPendingSegment>,
    peer_closed: bool,
    waiting_readers: Vec<usize>,
    waiting_writers: Vec<usize>,
}

impl TcpSocket {
    fn add_reader_waiter(&mut self, task_id: usize) {
        crate::evented::add_waiter(&mut self.waiting_readers, task_id);
    }

    fn add_writer_waiter(&mut self, task_id: usize) {
        crate::evented::add_waiter(&mut self.waiting_writers, task_id);
    }

    fn remove_waiter(&mut self, task_id: usize) {
        crate::evented::remove_waiter(&mut self.waiting_readers, task_id);
        crate::evented::remove_waiter(&mut self.waiting_writers, task_id);
    }

    fn take_readers(&mut self) -> Vec<usize> {
        mem::take(&mut self.waiting_readers)
    }

    fn take_writers(&mut self) -> Vec<usize> {
        mem::take(&mut self.waiting_writers)
    }

    fn take_waiters(&mut self) -> Vec<usize> {
        let mut waiters = self.take_readers();
        waiters.extend(self.take_writers());
        waiters
    }

    fn revents(&self, events: u64) -> u64 {
        let mut revents = 0u64;
        let disconnected =
            self.peer_closed || (self.state == TcpState::Closed && self.remote_ip != 0);
        let readable = !self.rx.is_empty() || disconnected;
        if events & crate::evented::EVENT_READ != 0 && readable {
            revents |= crate::evented::EVENT_READ;
        }
        if events & crate::evented::EVENT_WRITE != 0
            && matches!(self.state, TcpState::Established | TcpState::CloseWait)
            && !self.peer_closed
        {
            revents |= crate::evented::EVENT_WRITE;
        }
        if disconnected {
            revents |= crate::evented::EVENT_HANGUP;
        }
        revents
    }
}

struct TcpPendingSegment {
    seq: u32,
    data: Vec<u8>,
}

struct TcpReply {
    src_port: u16,
    dst_ip: u32,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
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
        format!(
            "TCP: open_socket(s)={} mss={} window={}",
            open_sockets, TCP_MSS, TCP_RX_WINDOW
        ),
        String::from("TLS: TLS 1.3 over kernel TCP with verified certificate chains"),
        String::from("Socket syscalls: socket/connect/send/recv exposed as 19-22"),
    ]
}

pub fn resource_stats() -> NetResourceStats {
    let state = NET_STATE.lock();
    let mut open_sockets = 0usize;
    let mut kernel_owned_sockets = 0usize;
    for socket in state.sockets.iter().flatten() {
        open_sockets = open_sockets.saturating_add(1);
        if socket.owner == KERNEL_SOCKET_OWNER {
            kernel_owned_sockets = kernel_owned_sockets.saturating_add(1);
        }
    }
    NetResourceStats {
        open_sockets,
        socket_slots: state.sockets.len(),
        kernel_owned_sockets,
        max_sockets_per_task: crate::resource_limits::MAX_SOCKETS_PER_TASK,
        max_sockets_total: crate::resource_limits::MAX_SOCKETS_TOTAL,
    }
}

pub fn owner_socket_count(owner: usize) -> usize {
    NET_STATE
        .lock()
        .sockets
        .iter()
        .flatten()
        .filter(|socket| socket.owner == owner)
        .count()
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

// Section files are included into this module so the split stays behavior-neutral.

include!("http.rs");
include!("sockets.rs");
include!("protocol.rs");
