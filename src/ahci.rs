extern crate alloc;

use alloc::{format, vec::Vec};
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;
use x86_64::PhysAddr;

use crate::pci::{self, Header, Location};
use crate::storage::{BlockDevice, BlockDeviceInfo};

const PCI_CLASS_STORAGE: u8 = 0x01;
const PCI_SUBCLASS_SATA: u8 = 0x06;
const PCI_PROGIF_AHCI: u8 = 0x01;

const HBA_CAP: u64 = 0x00;
const HBA_GHC: u64 = 0x04;
const HBA_PI: u64 = 0x0c;
const HBA_VS: u64 = 0x10;
const HBA_PORT_BASE: u64 = 0x100;
const HBA_PORT_STRIDE: u64 = 0x80;

const GHC_HR: u32 = 1 << 0;
const GHC_AE: u32 = 1 << 31;

const PX_CLB: u64 = 0x00;
const PX_CLBU: u64 = 0x04;
const PX_FB: u64 = 0x08;
const PX_FBU: u64 = 0x0c;
const PX_IS: u64 = 0x10;
const PX_CMD: u64 = 0x18;
const PX_TFD: u64 = 0x20;
const PX_SIG: u64 = 0x24;
const PX_SSTS: u64 = 0x28;
const PX_SERR: u64 = 0x30;
const PX_CI: u64 = 0x38;

const PX_CMD_ST: u32 = 1 << 0;
const PX_CMD_FRE: u32 = 1 << 4;
const PX_CMD_FR: u32 = 1 << 14;
const PX_CMD_CR: u32 = 1 << 15;

const PX_TFD_ERR: u32 = 1 << 0;
const PX_TFD_DRQ: u32 = 1 << 3;
const PX_TFD_BSY: u32 = 1 << 7;
const PX_IS_TFES: u32 = 1 << 30;

const SATA_SIG_ATA: u32 = 0x0000_0101;
const FIS_TYPE_REG_H2D: u8 = 0x27;
const ATA_CMD_IDENTIFY: u8 = 0xec;
const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
const ATA_CMD_FLUSH_CACHE_EXT: u8 = 0xea;

const COMMAND_SLOT: u32 = 0;
const SPIN_TIMEOUT: u64 = 5_000_000;

struct AhciState {
    mmio: u64,
    version: u32,
    ports: Vec<AhciPort>,
}

struct AhciPort {
    port: u8,
    clb_virt: u64,
    ctba_phys: u64,
    ctba_virt: u64,
    buffer_phys: u64,
    buffer_virt: u64,
    sectors: u32,
}

static STATE: Mutex<Option<AhciState>> = Mutex::new(None);

pub fn init() {
    let Some((loc, hdr, bar5_phys)) = find_controller() else {
        *STATE.lock() = None;
        return;
    };
    pci::enable_bus_master(loc);
    let bar5 = crate::vmm::phys_to_virt(PhysAddr::new(bar5_phys)).as_u64();
    match unsafe { init_controller(bar5) } {
        Ok(state) => {
            crate::println!(
                "[ahci] controller vendor={:#06x} device={:#06x} ports={} version={:#x}",
                hdr.vendor_id,
                hdr.device_id,
                state.ports.len(),
                state.version
            );
            *STATE.lock() = Some(state);
        }
        Err(err) => {
            crate::println!("[ahci] init failed: {}", err);
            *STATE.lock() = Some(AhciState {
                mmio: bar5,
                version: 0,
                ports: Vec::new(),
            });
        }
    }
}

pub fn devices() -> Vec<BlockDevice> {
    let state = STATE.lock();
    let Some(state) = state.as_ref() else {
        return Vec::new();
    };
    state
        .ports
        .iter()
        .filter_map(|port| block_device_for_port(port.port))
        .collect()
}

pub fn device_info(device: BlockDevice) -> BlockDeviceInfo {
    let Some(port_num) = device.sata_port() else {
        return BlockDeviceInfo {
            device,
            present: false,
            sectors: 0,
        };
    };
    let state = STATE.lock();
    let sectors = state
        .as_ref()
        .and_then(|state| state.ports.iter().find(|port| port.port == port_num))
        .map(|port| port.sectors)
        .unwrap_or(0);
    BlockDeviceInfo {
        device,
        present: sectors > 0,
        sectors,
    }
}

pub fn read_sector_from(device: BlockDevice, lba: u32, buf: &mut [u8; 512]) -> bool {
    transfer_sector(device, lba, buf, false)
}

pub fn write_sector_to(device: BlockDevice, lba: u32, buf: &[u8; 512]) -> bool {
    let mut temp = [0u8; 512];
    temp.copy_from_slice(buf);
    transfer_sector(device, lba, &mut temp, true)
}

pub fn flush_device(device: BlockDevice) -> bool {
    let Some(port_num) = device.sata_port() else {
        return false;
    };
    let mut state = STATE.lock();
    let Some(state) = state.as_mut() else {
        return false;
    };
    let Some(port) = state.ports.iter_mut().find(|port| port.port == port_num) else {
        return false;
    };
    unsafe { issue_command(state.mmio, port, ATA_CMD_FLUSH_CACHE_EXT, 0, 0, false) }
}

fn transfer_sector(device: BlockDevice, lba: u32, buf: &mut [u8; 512], write: bool) -> bool {
    let Some(port_num) = device.sata_port() else {
        return false;
    };
    let mut state = STATE.lock();
    let Some(state) = state.as_mut() else {
        return false;
    };
    let Some(port) = state.ports.iter_mut().find(|port| port.port == port_num) else {
        return false;
    };
    if lba >= port.sectors {
        return false;
    }
    unsafe {
        if write {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), port.buffer_virt as *mut u8, 512);
            if !issue_command(state.mmio, port, ATA_CMD_WRITE_DMA_EXT, lba, 1, true) {
                return false;
            }
            true
        } else {
            if !issue_command(state.mmio, port, ATA_CMD_READ_DMA_EXT, lba, 1, false) {
                return false;
            }
            core::ptr::copy_nonoverlapping(port.buffer_virt as *const u8, buf.as_mut_ptr(), 512);
            true
        }
    }
}

fn find_controller() -> Option<(Location, Header, u64)> {
    let mut found = None;
    pci::scan(|loc, hdr| {
        if found.is_some() {
            return;
        }
        if hdr.class == PCI_CLASS_STORAGE
            && hdr.subclass == PCI_SUBCLASS_SATA
            && hdr.prog_if == PCI_PROGIF_AHCI
        {
            if let Some(base) = pci::bar(loc, 5) {
                found = Some((loc, hdr, base));
            }
        }
    });
    found
}

unsafe fn init_controller(mmio: u64) -> Result<AhciState, &'static str> {
    let cap = read_u32(mmio + HBA_CAP);
    let version = read_u32(mmio + HBA_VS);
    write_u32(mmio + HBA_GHC, read_u32(mmio + HBA_GHC) | GHC_AE);
    if read_u32(mmio + HBA_GHC) & GHC_HR != 0 {
        wait_until("controller reset", || {
            read_u32(mmio + HBA_GHC) & GHC_HR == 0
        })?;
    }
    let implemented = read_u32(mmio + HBA_PI);
    let mut ports = Vec::new();
    let mut status = alloc::vec![format!("AHCI: cap={:#x} pi={:#x}", cap, implemented)];
    for port_num in 0..32u8 {
        if implemented & (1 << port_num) == 0 {
            continue;
        }
        if port_num > 7 {
            status.push(format!(
                "AHCI: port {} ignored; max sata7 exposed",
                port_num
            ));
            continue;
        }
        let port_base = port_base(mmio, port_num);
        let ssts = read_u32(port_base + PX_SSTS);
        let det = ssts & 0x0f;
        let ipm = (ssts >> 8) & 0x0f;
        let sig = read_u32(port_base + PX_SIG);
        if det != 3 || ipm != 1 {
            status.push(format!(
                "AHCI: port {} empty det={} ipm={} sig={:#x}",
                port_num, det, ipm, sig
            ));
            continue;
        }
        if sig != SATA_SIG_ATA {
            status.push(format!(
                "AHCI: port {} unsupported signature {:#x}",
                port_num, sig
            ));
            continue;
        }
        match init_port(mmio, port_num) {
            Ok(port) => {
                status.push(format!(
                    "AHCI: sata{} present sectors={}",
                    port_num, port.sectors
                ));
                ports.push(port);
            }
            Err(err) => status.push(format!("AHCI: sata{} init failed: {}", port_num, err)),
        }
    }
    for line in status.iter() {
        crate::println!("[ahci] {}", line);
    }
    Ok(AhciState {
        mmio,
        version,
        ports,
    })
}

unsafe fn init_port(mmio: u64, port_num: u8) -> Result<AhciPort, &'static str> {
    let port_base = port_base(mmio, port_num);
    stop_port(port_base)?;
    let (clb_phys, clb_virt) = alloc_zeroed_phys().ok_or("command list alloc failed")?;
    let (fb_phys, _fb_virt) = alloc_zeroed_phys().ok_or("fis alloc failed")?;
    let (ctba_phys, ctba_virt) = alloc_zeroed_phys().ok_or("command table alloc failed")?;
    let (buffer_phys, buffer_virt) = alloc_zeroed_phys().ok_or("data buffer alloc failed")?;

    write_u32(port_base + PX_CLB, clb_phys as u32);
    write_u32(port_base + PX_CLBU, (clb_phys >> 32) as u32);
    write_u32(port_base + PX_FB, fb_phys as u32);
    write_u32(port_base + PX_FBU, (fb_phys >> 32) as u32);
    write_u32(port_base + PX_SERR, 0xffff_ffff);
    write_u32(port_base + PX_IS, 0xffff_ffff);
    start_port(port_base)?;

    let mut port = AhciPort {
        port: port_num,
        clb_virt,
        ctba_phys,
        ctba_virt,
        buffer_phys,
        buffer_virt,
        sectors: 0,
    };
    if !issue_command(mmio, &mut port, ATA_CMD_IDENTIFY, 0, 1, false) {
        return Err("identify failed");
    }
    port.sectors = identify_sector_count(buffer_virt).ok_or("identify sectors unavailable")?;
    Ok(port)
}

unsafe fn issue_command(
    mmio: u64,
    port: &mut AhciPort,
    command: u8,
    lba: u32,
    count: u16,
    write: bool,
) -> bool {
    let port_base = port_base(mmio, port.port);
    let has_data = count > 0;
    if !wait_port_ready(port_base) {
        reset_port(port_base);
        if !wait_port_ready(port_base) {
            return false;
        }
    }
    write_u32(port_base + PX_IS, 0xffff_ffff);
    write_u32(port_base + PX_SERR, 0xffff_ffff);

    zero_memory(port.clb_virt, 1024);
    zero_memory(port.ctba_virt, 4096);

    let prdtl = if has_data { 1u32 } else { 0u32 };
    let header = 5u32 | if write { 1 << 6 } else { 0 } | (prdtl << 16);
    write_u32(port.clb_virt, header);
    write_u32(port.clb_virt + 4, 0);
    write_u32(port.clb_virt + 8, port.ctba_phys as u32);
    write_u32(port.clb_virt + 12, (port.ctba_phys >> 32) as u32);

    if has_data {
        let dbc = (count as u32 * 512).saturating_sub(1);
        let prdt = port.ctba_virt + 0x80;
        write_u32(prdt, port.buffer_phys as u32);
        write_u32(prdt + 4, (port.buffer_phys >> 32) as u32);
        write_u32(prdt + 8, 0);
        write_u32(prdt + 12, dbc);
    }

    write_reg_h2d_fis(port.ctba_virt, command, lba, count);
    fence(Ordering::SeqCst);
    write_u32(port_base + PX_CI, 1 << COMMAND_SLOT);

    for _ in 0..SPIN_TIMEOUT {
        let ci = read_u32(port_base + PX_CI);
        let is = read_u32(port_base + PX_IS);
        if is & PX_IS_TFES != 0 {
            reset_port(port_base);
            return false;
        }
        if ci & (1 << COMMAND_SLOT) == 0 {
            let tfd = read_u32(port_base + PX_TFD);
            return tfd & (PX_TFD_ERR | PX_TFD_BSY | PX_TFD_DRQ) == 0;
        }
        core::hint::spin_loop();
    }
    reset_port(port_base);
    false
}

unsafe fn write_reg_h2d_fis(addr: u64, command: u8, lba: u32, count: u16) {
    write_u8(addr, FIS_TYPE_REG_H2D);
    write_u8(addr + 1, 1 << 7);
    write_u8(addr + 2, command);
    write_u8(addr + 3, 0);
    write_u8(addr + 4, lba as u8);
    write_u8(addr + 5, (lba >> 8) as u8);
    write_u8(addr + 6, (lba >> 16) as u8);
    write_u8(addr + 7, 1 << 6);
    write_u8(addr + 8, (lba >> 24) as u8);
    write_u8(addr + 9, 0);
    write_u8(addr + 10, 0);
    write_u8(addr + 11, 0);
    write_u8(addr + 12, count as u8);
    write_u8(addr + 13, (count >> 8) as u8);
    write_u8(addr + 14, 0);
    write_u8(addr + 15, 0);
}

unsafe fn identify_sector_count(buffer_virt: u64) -> Option<u32> {
    let words = buffer_virt as *const u16;
    let lba48_words = core::ptr::read_volatile(words.add(83));
    if lba48_words & (1 << 10) != 0 {
        let lo = core::ptr::read_volatile(words.add(100)) as u32;
        let hi = core::ptr::read_volatile(words.add(101)) as u32;
        let sectors = lo | (hi << 16);
        if sectors > 0 {
            return Some(sectors);
        }
    }
    let lo = core::ptr::read_volatile(words.add(60)) as u32;
    let hi = core::ptr::read_volatile(words.add(61)) as u32;
    let sectors = lo | (hi << 16);
    if sectors > 0 {
        Some(sectors)
    } else {
        None
    }
}

unsafe fn stop_port(port_base: u64) -> Result<(), &'static str> {
    let cmd = read_u32(port_base + PX_CMD);
    write_u32(port_base + PX_CMD, cmd & !(PX_CMD_ST | PX_CMD_FRE));
    wait_until("port stop", || {
        read_u32(port_base + PX_CMD) & (PX_CMD_FR | PX_CMD_CR) == 0
    })
}

unsafe fn start_port(port_base: u64) -> Result<(), &'static str> {
    let cmd = read_u32(port_base + PX_CMD);
    write_u32(port_base + PX_CMD, cmd | PX_CMD_FRE);
    wait_until("fis receive", || {
        read_u32(port_base + PX_CMD) & PX_CMD_FR != 0
    })?;
    write_u32(port_base + PX_CMD, read_u32(port_base + PX_CMD) | PX_CMD_ST);
    Ok(())
}

unsafe fn reset_port(port_base: u64) {
    let _ = stop_port(port_base);
    write_u32(port_base + PX_IS, 0xffff_ffff);
    write_u32(port_base + PX_SERR, 0xffff_ffff);
    let _ = start_port(port_base);
}

unsafe fn wait_port_ready(port_base: u64) -> bool {
    for _ in 0..SPIN_TIMEOUT {
        let tfd = read_u32(port_base + PX_TFD);
        if tfd & (PX_TFD_BSY | PX_TFD_DRQ) == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

unsafe fn wait_until<F: Fn() -> bool>(label: &'static str, ready: F) -> Result<(), &'static str> {
    for _ in 0..SPIN_TIMEOUT {
        if ready() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    crate::println!("[ahci] timeout while waiting for {}", label);
    Err(label)
}

fn block_device_for_port(port: u8) -> Option<BlockDevice> {
    match port {
        0 => Some(BlockDevice::Sata0),
        1 => Some(BlockDevice::Sata1),
        2 => Some(BlockDevice::Sata2),
        3 => Some(BlockDevice::Sata3),
        4 => Some(BlockDevice::Sata4),
        5 => Some(BlockDevice::Sata5),
        6 => Some(BlockDevice::Sata6),
        7 => Some(BlockDevice::Sata7),
        _ => None,
    }
}

fn port_base(mmio: u64, port: u8) -> u64 {
    mmio + HBA_PORT_BASE + port as u64 * HBA_PORT_STRIDE
}

fn alloc_zeroed_phys() -> Option<(u64, u64)> {
    let frame = crate::vmm::alloc_zeroed_frame()?;
    let phys = frame.start_address().as_u64();
    let virt = crate::vmm::phys_to_virt(PhysAddr::new(phys)).as_u64();
    Some((phys, virt))
}

unsafe fn zero_memory(addr: u64, len: usize) {
    core::ptr::write_bytes(addr as *mut u8, 0, len);
}

unsafe fn read_u32(addr: u64) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

unsafe fn write_u32(addr: u64, val: u32) {
    core::ptr::write_volatile(addr as *mut u32, val)
}

unsafe fn write_u8(addr: u64, val: u8) {
    core::ptr::write_volatile(addr as *mut u8, val)
}
