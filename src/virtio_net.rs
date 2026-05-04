//! Legacy virtio-net PCI driver used by Phase 15.
//!
//! The Makefile's network targets force QEMU into the legacy I/O-port virtio
//! transport (`disable-modern=on`) so coolOS can bring up Ethernet without a
//! full PCI capability parser. The driver is polling-only: RX buffers are kept
//! posted on queue 0, TX uses queue 1 synchronously, and the network stack calls
//! `poll()` from the main loop and from blocking socket syscalls.

extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::pci::{self, Header, Location};

const VIRTIO_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_NET_DEVICE_LEGACY: u16 = 0x1000;

const REG_DEVICE_FEATURES: u16 = 0x00;
const REG_GUEST_FEATURES: u16 = 0x04;
const REG_QUEUE_PFN: u16 = 0x08;
const REG_QUEUE_SIZE: u16 = 0x0c;
const REG_QUEUE_SELECT: u16 = 0x0e;
const REG_QUEUE_NOTIFY: u16 = 0x10;
const REG_DEVICE_STATUS: u16 = 0x12;
const REG_ISR_STATUS: u16 = 0x13;
const REG_CONFIG: u16 = 0x14;

const STATUS_ACKNOWLEDGE: u8 = 1;
const STATUS_DRIVER: u8 = 2;
const STATUS_DRIVER_OK: u8 = 4;

const VIRTIO_NET_F_MAC: u32 = 1 << 5;

const RX_QUEUE: u16 = 0;
const TX_QUEUE: u16 = 1;
const RX_BUFFERS: usize = 32;
const RX_BUFFER_BYTES: usize = 2048;
const TX_BUFFER_BYTES: usize = 2048;
const VIRTIO_NET_HDR_LEN: usize = 10;
const TX_TIMEOUT_SPINS: usize = 2_000_000;

const VIRTQ_DESC_F_WRITE: u16 = 2;

#[derive(Clone)]
pub struct DriverInfo {
    pub location: String,
    pub mac: [u8; 6],
    pub queue_size: u16,
    pub io_base: u16,
}

#[derive(Clone, Copy)]
pub struct DriverStats {
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

struct DmaBuffer {
    phys: u64,
    virt: u64,
    len: usize,
}

impl DmaBuffer {
    fn new(len: usize) -> Option<Self> {
        let pages = (len + 4095) / 4096;
        let frame = crate::vmm::alloc_contiguous_zeroed_frames(pages)?;
        let phys = frame.start_address().as_u64();
        let virt = crate::vmm::phys_to_virt(frame.start_address()).as_u64();
        Some(Self {
            phys,
            virt,
            len: pages * 4096,
        })
    }

    fn as_slice(&self, len: usize) -> &[u8] {
        let len = len.min(self.len);
        unsafe { core::slice::from_raw_parts(self.virt as *const u8, len) }
    }

    fn as_mut_slice(&mut self, len: usize) -> &mut [u8] {
        let len = len.min(self.len);
        unsafe { core::slice::from_raw_parts_mut(self.virt as *mut u8, len) }
    }
}

struct VirtQueue {
    size: u16,
    virt: u64,
    avail_idx: u16,
    last_used_idx: u16,
}

impl VirtQueue {
    fn new(io_base: u16, index: u16) -> Result<Self, &'static str> {
        out_u16(io_base, REG_QUEUE_SELECT, index);
        let size = in_u16(io_base, REG_QUEUE_SIZE);
        if size == 0 {
            return Err("virtqueue missing");
        }
        let bytes = virtq_total_bytes(size as usize);
        let pages = (bytes + 4095) / 4096;
        let frame = crate::vmm::alloc_contiguous_zeroed_frames(pages)
            .ok_or("virtqueue allocation failed")?;
        let phys = frame.start_address().as_u64();
        let virt = crate::vmm::phys_to_virt(frame.start_address()).as_u64();

        out_u32(io_base, REG_QUEUE_PFN, (phys >> 12) as u32);

        let queue = Self {
            size,
            virt,
            avail_idx: 0,
            last_used_idx: 0,
        };
        queue.write_avail_flags(0);
        queue.write_avail_idx(0);
        queue.write_used_flags(0);
        Ok(queue)
    }

    fn desc_addr(&self, id: u16) -> *mut VirtqDesc {
        (self.virt as *mut VirtqDesc).wrapping_add(id as usize)
    }

    fn avail_base(&self) -> u64 {
        self.virt + self.size as u64 * core::mem::size_of::<VirtqDesc>() as u64
    }

    fn used_base(&self) -> u64 {
        self.virt + virtq_used_offset(self.size as usize) as u64
    }

    fn write_avail_flags(&self, flags: u16) {
        write_u16(self.avail_base(), flags);
    }

    fn write_avail_idx(&self, idx: u16) {
        write_u16(self.avail_base() + 2, idx);
    }

    fn write_used_flags(&self, flags: u16) {
        write_u16(self.used_base(), flags);
    }

    fn set_desc(&self, id: u16, addr: u64, len: usize, flags: u16, next: u16) {
        let desc = VirtqDesc {
            addr,
            len: len as u32,
            flags,
            next,
        };
        unsafe { core::ptr::write_volatile(self.desc_addr(id), desc) };
    }

    fn push_avail(&mut self, id: u16) {
        let ring_off =
            self.avail_base() + 4 + (self.avail_idx as usize % self.size as usize) as u64 * 2;
        write_u16(ring_off, id);
        fence(Ordering::SeqCst);
        self.avail_idx = self.avail_idx.wrapping_add(1);
        self.write_avail_idx(self.avail_idx);
        fence(Ordering::SeqCst);
    }

    fn pop_used(&mut self) -> Option<(u16, u32)> {
        let used_idx = read_u16(self.used_base() + 2);
        if used_idx == self.last_used_idx {
            return None;
        }
        let ring_off =
            self.used_base() + 4 + (self.last_used_idx as usize % self.size as usize) as u64 * 8;
        let id = read_u32(ring_off) as u16;
        let len = read_u32(ring_off + 4);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);
        Some((id, len))
    }
}

struct VirtioNet {
    info: DriverInfo,
    rx_queue: VirtQueue,
    tx_queue: VirtQueue,
    rx_buffers: Vec<DmaBuffer>,
    tx_buffer: DmaBuffer,
    stats: DriverStats,
}

impl VirtioNet {
    fn notify(&self, queue: u16) {
        out_u16(self.info.io_base, REG_QUEUE_NOTIFY, queue);
    }

    fn post_rx(&mut self, id: u16) {
        if let Some(buf) = self.rx_buffers.get(id as usize) {
            self.rx_queue
                .set_desc(id, buf.phys, RX_BUFFER_BYTES, VIRTQ_DESC_F_WRITE, 0);
            self.rx_queue.push_avail(id);
        }
    }

    fn transmit(&mut self, frame: &[u8]) -> Result<(), &'static str> {
        if frame.is_empty() || frame.len() + VIRTIO_NET_HDR_LEN > TX_BUFFER_BYTES {
            self.stats.tx_errors = self.stats.tx_errors.saturating_add(1);
            return Err("frame too large");
        }
        {
            let buf = self
                .tx_buffer
                .as_mut_slice(frame.len() + VIRTIO_NET_HDR_LEN);
            for byte in &mut buf[..VIRTIO_NET_HDR_LEN] {
                *byte = 0;
            }
            buf[VIRTIO_NET_HDR_LEN..VIRTIO_NET_HDR_LEN + frame.len()].copy_from_slice(frame);
        }
        self.tx_queue.set_desc(
            0,
            self.tx_buffer.phys,
            frame.len() + VIRTIO_NET_HDR_LEN,
            0,
            0,
        );
        self.tx_queue.push_avail(0);
        self.notify(TX_QUEUE);

        for _ in 0..TX_TIMEOUT_SPINS {
            if self.tx_queue.pop_used().is_some() {
                self.stats.tx_packets = self.stats.tx_packets.saturating_add(1);
                return Ok(());
            }
            core::hint::spin_loop();
        }

        self.stats.tx_errors = self.stats.tx_errors.saturating_add(1);
        crate::println!("[net] virtio tx timeout len={}", frame.len());
        Err("tx timeout")
    }

    fn poll(&mut self) -> Vec<Vec<u8>> {
        let _ = in_u8(self.info.io_base, REG_ISR_STATUS);
        let mut frames = Vec::new();
        let mut requeued = false;
        while let Some((id, len)) = self.rx_queue.pop_used() {
            let Some(buf) = self.rx_buffers.get(id as usize) else {
                self.stats.rx_dropped = self.stats.rx_dropped.saturating_add(1);
                continue;
            };
            let len = len as usize;
            if len > VIRTIO_NET_HDR_LEN && len <= RX_BUFFER_BYTES {
                let frame_len = len - VIRTIO_NET_HDR_LEN;
                let mut frame = Vec::with_capacity(frame_len);
                frame.extend_from_slice(
                    &buf.as_slice(len)[VIRTIO_NET_HDR_LEN..VIRTIO_NET_HDR_LEN + frame_len],
                );
                frames.push(frame);
                self.stats.rx_packets = self.stats.rx_packets.saturating_add(1);
            } else {
                self.stats.rx_dropped = self.stats.rx_dropped.saturating_add(1);
            }
            self.post_rx(id);
            requeued = true;
        }
        if requeued {
            self.notify(RX_QUEUE);
        }
        frames
    }
}

static DEVICE: Mutex<Option<VirtioNet>> = Mutex::new(None);

pub fn init() -> Result<DriverInfo, &'static str> {
    let (loc, _hdr) = find_legacy_virtio_net().ok_or("virtio-net PCI device not found")?;
    let io_base = pci::io_bar(loc, 0).ok_or("virtio-net I/O BAR missing")?;
    pci::enable_bus_master(loc);

    out_u8(io_base, REG_DEVICE_STATUS, 0);
    out_u8(io_base, REG_DEVICE_STATUS, STATUS_ACKNOWLEDGE);
    out_u8(
        io_base,
        REG_DEVICE_STATUS,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER,
    );

    let features = in_u32(io_base, REG_DEVICE_FEATURES);
    let negotiated = features & VIRTIO_NET_F_MAC;
    out_u32(io_base, REG_GUEST_FEATURES, negotiated);

    let rx_queue = VirtQueue::new(io_base, RX_QUEUE)?;
    let tx_queue = VirtQueue::new(io_base, TX_QUEUE)?;

    let mac = if negotiated & VIRTIO_NET_F_MAC != 0 {
        [
            in_u8(io_base, REG_CONFIG),
            in_u8(io_base, REG_CONFIG + 1),
            in_u8(io_base, REG_CONFIG + 2),
            in_u8(io_base, REG_CONFIG + 3),
            in_u8(io_base, REG_CONFIG + 4),
            in_u8(io_base, REG_CONFIG + 5),
        ]
    } else {
        [0x02, 0x43, 0x4f, loc.bus, loc.device, loc.function]
    };

    let mut rx_buffers = Vec::new();
    let rx_count = RX_BUFFERS.min(rx_queue.size as usize);
    for _ in 0..rx_count {
        rx_buffers.push(DmaBuffer::new(RX_BUFFER_BYTES).ok_or("rx buffer allocation failed")?);
    }
    let tx_buffer = DmaBuffer::new(TX_BUFFER_BYTES).ok_or("tx buffer allocation failed")?;

    let info = DriverInfo {
        location: format!("{:02x}:{:02x}.{}", loc.bus, loc.device, loc.function),
        mac,
        queue_size: rx_queue.size,
        io_base,
    };

    let mut device = VirtioNet {
        info: info.clone(),
        rx_queue,
        tx_queue,
        rx_buffers,
        tx_buffer,
        stats: DriverStats {
            tx_packets: 0,
            rx_packets: 0,
            tx_errors: 0,
            rx_dropped: 0,
        },
    };
    for id in 0..rx_count {
        device.post_rx(id as u16);
    }
    device.notify(RX_QUEUE);

    out_u8(
        io_base,
        REG_DEVICE_STATUS,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK,
    );

    *DEVICE.lock() = Some(device);
    Ok(info)
}

pub fn transmit(frame: &[u8]) -> Result<(), &'static str> {
    let mut guard = DEVICE.lock();
    guard
        .as_mut()
        .ok_or("virtio-net not initialized")?
        .transmit(frame)
}

pub fn poll() -> Vec<Vec<u8>> {
    DEVICE
        .lock()
        .as_mut()
        .map(VirtioNet::poll)
        .unwrap_or_else(Vec::new)
}

pub fn stats() -> Option<DriverStats> {
    DEVICE.lock().as_ref().map(|device| device.stats)
}

fn find_legacy_virtio_net() -> Option<(Location, Header)> {
    let mut found = None;
    pci::scan(|loc, hdr| {
        if found.is_none()
            && hdr.vendor_id == VIRTIO_VENDOR_ID
            && hdr.device_id == VIRTIO_NET_DEVICE_LEGACY
        {
            found = Some((loc, hdr));
        }
    });
    found
}

fn virtq_used_offset(size: usize) -> usize {
    align_up(
        size * core::mem::size_of::<VirtqDesc>() + 6 + size * 2 + 2,
        4096,
    )
}

fn virtq_total_bytes(size: usize) -> usize {
    virtq_used_offset(size) + 6 + size * 8 + 2
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn in_u8(base: u16, off: u16) -> u8 {
    unsafe { Port::<u8>::new(base + off).read() }
}

fn in_u16(base: u16, off: u16) -> u16 {
    unsafe { Port::<u16>::new(base + off).read() }
}

fn in_u32(base: u16, off: u16) -> u32 {
    unsafe { Port::<u32>::new(base + off).read() }
}

fn out_u8(base: u16, off: u16, value: u8) {
    unsafe { Port::<u8>::new(base + off).write(value) }
}

fn out_u16(base: u16, off: u16, value: u16) {
    unsafe { Port::<u16>::new(base + off).write(value) }
}

fn out_u32(base: u16, off: u16, value: u32) {
    unsafe { Port::<u32>::new(base + off).write(value) }
}

fn read_u16(virt: u64) -> u16 {
    unsafe { core::ptr::read_volatile(virt as *const u16) }
}

fn read_u32(virt: u64) -> u32 {
    unsafe { core::ptr::read_volatile(virt as *const u32) }
}

fn write_u16(virt: u64, value: u16) {
    unsafe { core::ptr::write_volatile(virt as *mut u16, value) }
}
