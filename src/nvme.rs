extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{fence, Ordering};
use spin::Mutex;
use x86_64::PhysAddr;

use crate::pci::{self, Header, Location};
use crate::storage::{BlockDevice, BlockDeviceInfo};

const PCI_CLASS_STORAGE: u8 = 0x01;
const PCI_SUBCLASS_NVME: u8 = 0x08;
const PCI_PROGIF_NVME: u8 = 0x02;

const REG_CAP: u64 = 0x00;
const REG_VS: u64 = 0x08;
const REG_CC: u64 = 0x14;
const REG_CSTS: u64 = 0x1c;
const REG_AQA: u64 = 0x24;
const REG_ASQ: u64 = 0x28;
const REG_ACQ: u64 = 0x30;
const REG_DOORBELL_BASE: u64 = 0x1000;

const CC_EN: u32 = 1 << 0;
const CC_IOSQES_64: u32 = 6 << 16;
const CC_IOCQES_16: u32 = 4 << 20;
const CSTS_RDY: u32 = 1 << 0;
const CSTS_CFS: u32 = 1 << 1;

const ADMIN_IDENTIFY: u8 = 0x06;
const ADMIN_CREATE_IO_CQ: u8 = 0x05;
const ADMIN_CREATE_IO_SQ: u8 = 0x01;

const NVM_FLUSH: u8 = 0x00;
const NVM_WRITE: u8 = 0x01;
const NVM_READ: u8 = 0x02;

const ADMIN_QUEUE_ENTRIES: u16 = 16;
const IO_QUEUE_ENTRIES: u16 = 16;
const ADMIN_QUEUE_ID: u16 = 0;
const IO_QUEUE_ID: u16 = 1;
const NVME_TIMEOUT: u64 = 10_000_000;

struct NvmeState {
    disks: Vec<NvmeDisk>,
}

struct NvmeDisk {
    device: BlockDevice,
    namespace_id: u32,
    sectors: u32,
    sectors_per_block: u32,
    mmio: u64,
    dstrd: u8,
    io_queue: NvmeQueue,
    buffer_phys: u64,
    buffer_virt: u64,
}

struct NvmeQueue {
    id: u16,
    entries: u16,
    sq_phys: u64,
    sq_virt: u64,
    cq_phys: u64,
    cq_virt: u64,
    sq_tail: u16,
    cq_head: u16,
    cq_phase: bool,
    next_cid: u16,
}

struct NvmeCommand {
    dwords: [u32; 16],
}

static STATE: Mutex<Option<NvmeState>> = Mutex::new(None);

pub fn init() {
    let controllers = find_controllers();
    if controllers.is_empty() {
        *STATE.lock() = None;
        return;
    }

    let mut disks = Vec::new();
    for (controller_idx, (loc, hdr, bar0_phys)) in controllers.into_iter().enumerate() {
        if controller_idx > 3 {
            crate::println!(
                "[nvme] controller {} ignored; max nvme3n1 exposed",
                controller_idx
            );
            continue;
        }
        pci::enable_bus_master(loc);
        let mmio = crate::vmm::phys_to_virt(PhysAddr::new(bar0_phys)).as_u64();
        match unsafe { init_controller(mmio, controller_idx as u8) } {
            Ok(disk) => {
                crate::println!(
                    "[nvme] controller vendor={:#06x} device={:#06x} nvme{}n1 present sectors={}",
                    hdr.vendor_id,
                    hdr.device_id,
                    controller_idx,
                    disk.sectors
                );
                disks.push(disk);
            }
            Err(err) => {
                crate::println!(
                    "[nvme] controller vendor={:#06x} device={:#06x} init failed: {}",
                    hdr.vendor_id,
                    hdr.device_id,
                    err
                );
            }
        }
    }

    *STATE.lock() = Some(NvmeState { disks });
}

pub fn devices() -> Vec<BlockDevice> {
    let state = STATE.lock();
    let Some(state) = state.as_ref() else {
        return Vec::new();
    };
    state.disks.iter().map(|disk| disk.device).collect()
}

pub fn device_info(device: BlockDevice) -> BlockDeviceInfo {
    let state = STATE.lock();
    let sectors = state
        .as_ref()
        .and_then(|state| state.disks.iter().find(|disk| disk.device == device))
        .map(|disk| disk.sectors)
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
    let mut state = STATE.lock();
    let Some(state) = state.as_mut() else {
        return false;
    };
    let Some(disk) = state.disks.iter_mut().find(|disk| disk.device == device) else {
        return false;
    };
    unsafe {
        let mut command = NvmeCommand::new(NVM_FLUSH, disk.namespace_id);
        issue_io_command(disk, &mut command)
    }
}

fn transfer_sector(device: BlockDevice, lba: u32, buf: &mut [u8; 512], write: bool) -> bool {
    let mut state = STATE.lock();
    let Some(state) = state.as_mut() else {
        return false;
    };
    let Some(disk) = state.disks.iter_mut().find(|disk| disk.device == device) else {
        return false;
    };
    if lba >= disk.sectors {
        return false;
    }
    let nvme_lba = lba / disk.sectors_per_block;
    let sector_offset = ((lba % disk.sectors_per_block) * 512) as usize;

    unsafe {
        if write && disk.sectors_per_block > 1 {
            let mut read_command = NvmeCommand::new(NVM_READ, disk.namespace_id);
            read_command.set_prp1(disk.buffer_phys);
            read_command.dwords[10] = nvme_lba;
            read_command.dwords[11] = 0;
            read_command.dwords[12] = 0;
            if !issue_io_command(disk, &mut read_command) {
                return false;
            }
        }

        if write {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                (disk.buffer_virt as *mut u8).add(sector_offset),
                512,
            );
        }

        let opcode = if write { NVM_WRITE } else { NVM_READ };
        let mut command = NvmeCommand::new(opcode, disk.namespace_id);
        command.set_prp1(disk.buffer_phys);
        command.dwords[10] = nvme_lba;
        command.dwords[11] = 0;
        command.dwords[12] = 0;
        if !issue_io_command(disk, &mut command) {
            return false;
        }

        if !write {
            core::ptr::copy_nonoverlapping(
                (disk.buffer_virt as *const u8).add(sector_offset),
                buf.as_mut_ptr(),
                512,
            );
        }
        true
    }
}

fn find_controllers() -> Vec<(Location, Header, u64)> {
    let mut found = Vec::new();
    pci::scan(|loc, hdr| {
        if hdr.class == PCI_CLASS_STORAGE
            && hdr.subclass == PCI_SUBCLASS_NVME
            && hdr.prog_if == PCI_PROGIF_NVME
        {
            if let Some(base) = pci::bar(loc, 0) {
                found.push((loc, hdr, base));
            }
        }
    });
    found
}

unsafe fn init_controller(mmio: u64, controller: u8) -> Result<NvmeDisk, &'static str> {
    let cap = read_u64(mmio + REG_CAP);
    let version = read_u32(mmio + REG_VS);
    let dstrd = ((cap >> 32) & 0x0f) as u8;
    let mps_min = ((cap >> 48) & 0x0f) as u8;
    if mps_min > 0 {
        return Err("minimum page size above 4K unsupported");
    }

    let mut admin = NvmeQueue::new(ADMIN_QUEUE_ID, ADMIN_QUEUE_ENTRIES)?;
    disable_controller(mmio)?;
    write_u32(
        mmio + REG_AQA,
        ((ADMIN_QUEUE_ENTRIES as u32 - 1) << 16) | (ADMIN_QUEUE_ENTRIES as u32 - 1),
    );
    write_u64(mmio + REG_ASQ, admin.sq_phys);
    write_u64(mmio + REG_ACQ, admin.cq_phys);
    write_u32(mmio + REG_CC, CC_EN | CC_IOSQES_64 | CC_IOCQES_16);
    wait_controller_ready(mmio, true)?;

    let (identify_phys, identify_virt) =
        alloc_zeroed_phys().ok_or("identify buffer alloc failed")?;
    let mut identify_controller = NvmeCommand::new(ADMIN_IDENTIFY, 0);
    identify_controller.set_prp1(identify_phys);
    identify_controller.dwords[10] = 1;
    if !issue_admin_command(mmio, dstrd, &mut admin, &mut identify_controller) {
        return Err("identify controller failed");
    }
    let namespaces = read_le_u32(identify_virt + 516);
    if namespaces == 0 {
        return Err("no namespaces");
    }

    zero_memory(identify_virt, 4096);
    let mut identify_namespace = NvmeCommand::new(ADMIN_IDENTIFY, 1);
    identify_namespace.set_prp1(identify_phys);
    identify_namespace.dwords[10] = 0;
    if !issue_admin_command(mmio, dstrd, &mut admin, &mut identify_namespace) {
        return Err("identify namespace failed");
    }
    let nsze = read_le_u64(identify_virt);
    if nsze == 0 || nsze > u32::MAX as u64 {
        return Err("namespace size unsupported");
    }
    let flbas = read_u8(identify_virt + 26);
    let lba_format = ((flbas & 0x0f) | ((flbas >> 1) & 0x10)) as u64;
    let lbaf_offset = 128 + lba_format * 4;
    if lbaf_offset + 3 >= 4096 {
        return Err("namespace LBA format invalid");
    }
    let lbads = read_u8(identify_virt + lbaf_offset + 2);
    if !(9..=12).contains(&lbads) {
        return Err("NVMe LBA size unsupported");
    }
    let block_size = 1u32 << lbads;
    let sectors_per_block = block_size / 512;
    let Some(exposed_sectors) = nsze.checked_mul(sectors_per_block as u64) else {
        return Err("namespace size unsupported");
    };
    if exposed_sectors > u32::MAX as u64 {
        return Err("namespace size unsupported");
    }

    let io_queue = NvmeQueue::new(IO_QUEUE_ID, IO_QUEUE_ENTRIES)?;
    let qsize = IO_QUEUE_ENTRIES as u32 - 1;
    let mut create_cq = NvmeCommand::new(ADMIN_CREATE_IO_CQ, 0);
    create_cq.set_prp1(io_queue.cq_phys);
    create_cq.dwords[10] = IO_QUEUE_ID as u32 | (qsize << 16);
    create_cq.dwords[11] = 1;
    if !issue_admin_command(mmio, dstrd, &mut admin, &mut create_cq) {
        return Err("create io completion queue failed");
    }

    let mut create_sq = NvmeCommand::new(ADMIN_CREATE_IO_SQ, 0);
    create_sq.set_prp1(io_queue.sq_phys);
    create_sq.dwords[10] = IO_QUEUE_ID as u32 | (qsize << 16);
    create_sq.dwords[11] = 1 | ((IO_QUEUE_ID as u32) << 16);
    if !issue_admin_command(mmio, dstrd, &mut admin, &mut create_sq) {
        return Err("create io submission queue failed");
    }

    let (buffer_phys, buffer_virt) = alloc_zeroed_phys().ok_or("data buffer alloc failed")?;
    crate::println!(
        "[nvme] controller{} version={:#x} namespaces={} lba_size={}",
        controller,
        version,
        namespaces,
        block_size
    );
    Ok(NvmeDisk {
        device: block_device_for_controller(controller).ok_or("controller index unsupported")?,
        namespace_id: 1,
        sectors: exposed_sectors as u32,
        sectors_per_block,
        mmio,
        dstrd,
        io_queue,
        buffer_phys,
        buffer_virt,
    })
}

unsafe fn issue_admin_command(
    mmio: u64,
    dstrd: u8,
    queue: &mut NvmeQueue,
    command: &mut NvmeCommand,
) -> bool {
    issue_command(mmio, dstrd, queue, command)
}

unsafe fn issue_io_command(disk: &mut NvmeDisk, command: &mut NvmeCommand) -> bool {
    issue_command(disk.mmio, disk.dstrd, &mut disk.io_queue, command)
}

unsafe fn issue_command(
    mmio: u64,
    dstrd: u8,
    queue: &mut NvmeQueue,
    command: &mut NvmeCommand,
) -> bool {
    let cid = queue.next_cid;
    queue.next_cid = queue.next_cid.wrapping_add(1);
    command.dwords[0] = (command.dwords[0] & 0x0000_ffff) | ((cid as u32) << 16);

    let sq_entry = queue.sq_virt + queue.sq_tail as u64 * 64;
    for idx in 0..16 {
        write_u32(sq_entry + idx as u64 * 4, command.dwords[idx]);
    }
    fence(Ordering::SeqCst);

    queue.sq_tail = (queue.sq_tail + 1) % queue.entries;
    write_u32(sq_doorbell(mmio, dstrd, queue.id), queue.sq_tail as u32);

    for _ in 0..NVME_TIMEOUT {
        let cqe = queue.cq_virt + queue.cq_head as u64 * 16;
        let status = read_u32(cqe + 12);
        let phase = ((status >> 16) & 1) != 0;
        if phase == queue.cq_phase {
            let completion_cid = (status & 0xffff) as u16;
            let code = (status >> 17) & 0x7fff;
            queue.cq_head += 1;
            if queue.cq_head == queue.entries {
                queue.cq_head = 0;
                queue.cq_phase = !queue.cq_phase;
            }
            write_u32(cq_doorbell(mmio, dstrd, queue.id), queue.cq_head as u32);
            return completion_cid == cid && code == 0;
        }
        if read_u32(mmio + REG_CSTS) & CSTS_CFS != 0 {
            return false;
        }
        core::hint::spin_loop();
    }
    false
}

unsafe fn disable_controller(mmio: u64) -> Result<(), &'static str> {
    let cc = read_u32(mmio + REG_CC);
    if cc & CC_EN != 0 {
        write_u32(mmio + REG_CC, cc & !CC_EN);
    }
    wait_controller_ready(mmio, false)
}

unsafe fn wait_controller_ready(mmio: u64, ready: bool) -> Result<(), &'static str> {
    for _ in 0..NVME_TIMEOUT {
        let csts = read_u32(mmio + REG_CSTS);
        if csts & CSTS_CFS != 0 {
            return Err("controller fatal status");
        }
        if (csts & CSTS_RDY != 0) == ready {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("controller ready timeout")
}

impl NvmeQueue {
    fn new(id: u16, entries: u16) -> Result<Self, &'static str> {
        let (sq_phys, sq_virt) = alloc_zeroed_phys().ok_or("submission queue alloc failed")?;
        let (cq_phys, cq_virt) = alloc_zeroed_phys().ok_or("completion queue alloc failed")?;
        Ok(Self {
            id,
            entries,
            sq_phys,
            sq_virt,
            cq_phys,
            cq_virt,
            sq_tail: 0,
            cq_head: 0,
            cq_phase: true,
            next_cid: 1,
        })
    }
}

impl NvmeCommand {
    const fn new(opcode: u8, namespace_id: u32) -> Self {
        let mut dwords = [0u32; 16];
        dwords[0] = opcode as u32;
        dwords[1] = namespace_id;
        Self { dwords }
    }

    fn set_prp1(&mut self, phys: u64) {
        self.dwords[6] = phys as u32;
        self.dwords[7] = (phys >> 32) as u32;
    }
}

fn block_device_for_controller(controller: u8) -> Option<BlockDevice> {
    match controller {
        0 => Some(BlockDevice::Nvme0n1),
        1 => Some(BlockDevice::Nvme1n1),
        2 => Some(BlockDevice::Nvme2n1),
        3 => Some(BlockDevice::Nvme3n1),
        _ => None,
    }
}

fn alloc_zeroed_phys() -> Option<(u64, u64)> {
    let frame = crate::vmm::alloc_zeroed_frame()?;
    let phys = frame.start_address().as_u64();
    let virt = crate::vmm::phys_to_virt(PhysAddr::new(phys)).as_u64();
    Some((phys, virt))
}

fn sq_doorbell(mmio: u64, dstrd: u8, qid: u16) -> u64 {
    let stride = 4u64 << dstrd;
    mmio + REG_DOORBELL_BASE + (qid as u64 * 2) * stride
}

fn cq_doorbell(mmio: u64, dstrd: u8, qid: u16) -> u64 {
    let stride = 4u64 << dstrd;
    mmio + REG_DOORBELL_BASE + (qid as u64 * 2 + 1) * stride
}

unsafe fn zero_memory(addr: u64, len: usize) {
    core::ptr::write_bytes(addr as *mut u8, 0, len);
}

unsafe fn read_u8(addr: u64) -> u8 {
    core::ptr::read_volatile(addr as *const u8)
}

unsafe fn read_u32(addr: u64) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

unsafe fn read_u64(addr: u64) -> u64 {
    core::ptr::read_volatile(addr as *const u64)
}

unsafe fn write_u32(addr: u64, val: u32) {
    core::ptr::write_volatile(addr as *mut u32, val)
}

unsafe fn write_u64(addr: u64, val: u64) {
    core::ptr::write_volatile(addr as *mut u64, val)
}

unsafe fn read_le_u32(addr: u64) -> u32 {
    core::ptr::read_unaligned(addr as *const u32)
}

unsafe fn read_le_u64(addr: u64) -> u64 {
    core::ptr::read_unaligned(addr as *const u64)
}
