/// ATA PIO driver.
///
/// The normal coolOS QEMU layout is:
/// - ide0-master: BIOS boot image, or the self-booting installed disk
/// - ide0-slave: legacy live CoolFS root image
/// - ide1-master: optional installer target image
///
/// Only LBA28 PIO transfers are implemented, which is enough for the current
/// QEMU disk images and the installer path.
use spin::Mutex;
use x86_64::instructions::port::Port;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IdeDevice {
    Ide0Master,
    Ide0Slave,
    Ide1Master,
    Ide1Slave,
}

#[derive(Clone, Copy)]
pub struct AtaDeviceInfo {
    pub device: IdeDevice,
    pub present: bool,
    pub sectors: u32,
}

#[derive(Clone, Copy)]
pub struct RootDisk {
    pub device: IdeDevice,
    pub base_lba: u32,
    pub sectors: u32,
    pub layout: RootDiskLayout,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RootDiskLayout {
    RawCoolFs,
    MbrCoolFs,
    GptCoolFs,
}

impl RootDiskLayout {
    pub const fn suffix(self) -> &'static str {
        match self {
            RootDiskLayout::RawCoolFs => "",
            RootDiskLayout::MbrCoolFs => ":mbr-coolfs",
            RootDiskLayout::GptCoolFs => ":gpt-coolfs",
        }
    }
}

#[derive(Clone, Copy)]
struct AtaBus {
    data: u16,
    features: u16,
    seccount: u16,
    lba_lo: u16,
    lba_mid: u16,
    lba_hi: u16,
    drive_hdr: u16,
    status_cmd: u16,
    dev_ctrl: u16,
}

const PRIMARY_BUS: AtaBus = AtaBus {
    data: 0x1F0,
    features: 0x1F1,
    seccount: 0x1F2,
    lba_lo: 0x1F3,
    lba_mid: 0x1F4,
    lba_hi: 0x1F5,
    drive_hdr: 0x1F6,
    status_cmd: 0x1F7,
    dev_ctrl: 0x3F6,
};

const SECONDARY_BUS: AtaBus = AtaBus {
    data: 0x170,
    features: 0x171,
    seccount: 0x172,
    lba_lo: 0x173,
    lba_mid: 0x174,
    lba_hi: 0x175,
    drive_hdr: 0x176,
    status_cmd: 0x177,
    dev_ctrl: 0x376,
};

const LEGACY_ROOT_DEVICE: IdeDevice = IdeDevice::Ide0Slave;
const CMD_IDENTIFY: u8 = 0xEC;
const CMD_READ: u8 = 0x20;
const CMD_WRITE: u8 = 0x30;
const CMD_CACHE_FLUSH: u8 = 0xE7;
const STATUS_BSY: u8 = 0x80;
const STATUS_DF: u8 = 0x20;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;
const ATA_TIMEOUT_ITERS: u32 = 250_000;
const ATA_RETRIES: usize = 3;

static ROOT_DISK: Mutex<Option<RootDisk>> = Mutex::new(None);

impl IdeDevice {
    pub const fn name(self) -> &'static str {
        match self {
            IdeDevice::Ide0Master => "ide0-master",
            IdeDevice::Ide0Slave => "ide0-slave",
            IdeDevice::Ide1Master => "ide1-master",
            IdeDevice::Ide1Slave => "ide1-slave",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ide0-master" => Some(IdeDevice::Ide0Master),
            "ide0-slave" => Some(IdeDevice::Ide0Slave),
            "ide1-master" => Some(IdeDevice::Ide1Master),
            "ide1-slave" => Some(IdeDevice::Ide1Slave),
            _ => None,
        }
    }

    const fn bus(self) -> AtaBus {
        match self {
            IdeDevice::Ide0Master | IdeDevice::Ide0Slave => PRIMARY_BUS,
            IdeDevice::Ide1Master | IdeDevice::Ide1Slave => SECONDARY_BUS,
        }
    }

    const fn drive_select(self, lba: u32) -> u8 {
        let base = match self {
            IdeDevice::Ide0Master | IdeDevice::Ide1Master => 0xE0,
            IdeDevice::Ide0Slave | IdeDevice::Ide1Slave => 0xF0,
        };
        base | ((lba >> 24) as u8 & 0x0F)
    }
}

pub fn all_devices() -> [IdeDevice; 4] {
    [
        IdeDevice::Ide0Master,
        IdeDevice::Ide0Slave,
        IdeDevice::Ide1Master,
        IdeDevice::Ide1Slave,
    ]
}

pub fn device_info(device: IdeDevice) -> AtaDeviceInfo {
    let sectors = identify_sectors(device).unwrap_or(0);
    AtaDeviceInfo {
        device,
        present: sectors > 0,
        sectors,
    }
}

pub fn root_disk() -> Option<RootDisk> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut slot = ROOT_DISK.lock();
        if slot.is_none() {
            *slot = detect_root_disk();
        }
        *slot
    })
}

pub fn read_sector(lba: u32, buf: &mut [u8; 512]) -> bool {
    if let Some(root) = root_disk() {
        if lba >= root.sectors {
            return false;
        }
        if let Some(abs_lba) = root.base_lba.checked_add(lba) {
            return read_sector_from(root.device, abs_lba, buf);
        }
        return false;
    }
    read_sector_from(LEGACY_ROOT_DEVICE, lba, buf)
}

pub fn write_sector(lba: u32, buf: &[u8; 512]) -> bool {
    let Some(root) = root_disk() else {
        return false;
    };
    if lba >= root.sectors {
        return false;
    }
    let Some(abs_lba) = root.base_lba.checked_add(lba) else {
        return false;
    };
    if !write_sector_to(root.device, abs_lba, buf) {
        return false;
    }
    flush_device(root.device)
}

pub fn read_sector_from(device: IdeDevice, lba: u32, buf: &mut [u8; 512]) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        for attempt in 0..ATA_RETRIES {
            if attempt > 0 {
                unsafe {
                    soft_reset(device, lba, "read retry");
                }
            }
            if read_sector_inner(device, lba, buf) {
                return true;
            }
        }
        false
    })
}

pub fn write_sector_to(device: IdeDevice, lba: u32, buf: &[u8; 512]) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        for attempt in 0..ATA_RETRIES {
            if attempt > 0 {
                unsafe {
                    soft_reset(device, lba, "write retry");
                }
            }
            if write_sector_inner(device, lba, buf) {
                return true;
            }
        }
        false
    })
}

pub fn flush_device(device: IdeDevice) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| unsafe { flush_device_inner(device) })
}

fn detect_root_disk() -> Option<RootDisk> {
    for device in [
        IdeDevice::Ide0Slave,
        IdeDevice::Ide0Master,
        IdeDevice::Ide1Master,
        IdeDevice::Ide1Slave,
    ] {
        let info = device_info(device);
        if !info.present {
            continue;
        }
        let mut first = [0u8; crate::disk_layout::SECTOR_SIZE];
        if read_sector_from(device, 0, &mut first) && crate::disk_layout::has_coolfs_magic(&first) {
            return Some(RootDisk {
                device,
                base_lba: 0,
                sectors: info.sectors,
                layout: RootDiskLayout::RawCoolFs,
            });
        }
    }

    for device in [
        IdeDevice::Ide0Master,
        IdeDevice::Ide0Slave,
        IdeDevice::Ide1Master,
        IdeDevice::Ide1Slave,
    ] {
        let info = device_info(device);
        if !info.present {
            continue;
        }
        let mut mbr = [0u8; crate::disk_layout::SECTOR_SIZE];
        if !read_sector_from(device, 0, &mut mbr) {
            continue;
        }
        let Some(partitions) = crate::disk_layout::parse_mbr(&mbr) else {
            continue;
        };
        let Some(partition) = crate::disk_layout::find_partition(
            &partitions,
            crate::disk_layout::COOLFS_PARTITION_TYPE,
        ) else {
            continue;
        };
        let Some(end_lba) = partition.end_lba() else {
            continue;
        };
        if partition.starting_lba >= info.sectors || end_lba > info.sectors {
            continue;
        }
        let mut first = [0u8; crate::disk_layout::SECTOR_SIZE];
        if read_sector_from(device, partition.starting_lba, &mut first)
            && crate::disk_layout::has_coolfs_magic(&first)
        {
            return Some(RootDisk {
                device,
                base_lba: partition.starting_lba,
                sectors: partition.sectors,
                layout: RootDiskLayout::MbrCoolFs,
            });
        }
    }

    for device in [
        IdeDevice::Ide0Master,
        IdeDevice::Ide0Slave,
        IdeDevice::Ide1Master,
        IdeDevice::Ide1Slave,
    ] {
        let info = device_info(device);
        if !info.present {
            continue;
        }
        let Some(partition) = find_gpt_partition(
            device,
            info.sectors,
            crate::disk_layout::COOLFS_GPT_PARTITION_GUID,
        ) else {
            continue;
        };
        let Some(partition_sectors) = partition.sectors() else {
            continue;
        };
        let Some(end_lba) = partition.end_lba() else {
            continue;
        };
        if partition.starting_lba >= info.sectors || end_lba > info.sectors {
            continue;
        }
        let mut first = [0u8; crate::disk_layout::SECTOR_SIZE];
        if read_sector_from(device, partition.starting_lba, &mut first)
            && crate::disk_layout::has_coolfs_magic(&first)
        {
            return Some(RootDisk {
                device,
                base_lba: partition.starting_lba,
                sectors: partition_sectors,
                layout: RootDiskLayout::GptCoolFs,
            });
        }
    }

    None
}

pub fn find_gpt_partition(
    device: IdeDevice,
    disk_sectors: u32,
    type_guid: crate::disk_layout::GptGuid,
) -> Option<crate::disk_layout::GptPartition> {
    let mut mbr = [0u8; crate::disk_layout::SECTOR_SIZE];
    if !read_sector_from(device, 0, &mut mbr) || !crate::disk_layout::has_protective_mbr(&mbr) {
        return None;
    }
    let mut header_sector = [0u8; crate::disk_layout::SECTOR_SIZE];
    if !read_sector_from(
        device,
        crate::disk_layout::GPT_PRIMARY_HEADER_LBA,
        &mut header_sector,
    ) {
        return None;
    }
    let header = crate::disk_layout::parse_gpt_header(&header_sector, disk_sectors)?;
    let max_entries = header
        .entry_count
        .min(crate::disk_layout::GPT_ENTRY_COUNT as u32);
    let entries_per_sector = (crate::disk_layout::SECTOR_SIZE as u32 / header.entry_size).max(1);
    let mut sector = [0u8; crate::disk_layout::SECTOR_SIZE];
    for idx in 0..max_entries {
        let sector_lba = header.entries_lba.checked_add(idx / entries_per_sector)?;
        let entry_in_sector = idx % entries_per_sector;
        let offset = (entry_in_sector * header.entry_size) as usize;
        if !read_sector_from(device, sector_lba, &mut sector) {
            return None;
        }
        let end = offset.checked_add(header.entry_size as usize)?;
        let Some(partition) = crate::disk_layout::parse_gpt_partition_entry(&sector[offset..end])
        else {
            continue;
        };
        if partition.type_guid == type_guid {
            return Some(partition);
        }
    }
    None
}

fn identify_sectors(device: IdeDevice) -> Option<u32> {
    x86_64::instructions::interrupts::without_interrupts(|| unsafe {
        let bus = device.bus();
        let mut status = Port::<u8>::new(bus.status_cmd);
        Port::<u8>::new(bus.dev_ctrl).write(0x02);
        if !select_device(device, 0, &mut status, "identify select") {
            return None;
        }

        Port::<u8>::new(bus.seccount).write(0);
        Port::<u8>::new(bus.lba_lo).write(0);
        Port::<u8>::new(bus.lba_mid).write(0);
        Port::<u8>::new(bus.lba_hi).write(0);
        Port::<u8>::new(bus.status_cmd).write(CMD_IDENTIFY);
        ata_delay(&mut status);

        let initial = status.read();
        if initial == 0 || initial == 0xFF {
            return None;
        }
        if !wait_not_busy(&mut status, device, 0, "identify") {
            return None;
        }
        let mid = Port::<u8>::new(bus.lba_mid).read();
        let hi = Port::<u8>::new(bus.lba_hi).read();
        if mid != 0 || hi != 0 {
            return None;
        }
        if !wait_drq(&mut status, device, 0, "identify") {
            return None;
        }

        let mut words = [0u16; 256];
        let mut data = Port::<u16>::new(bus.data);
        for word in words.iter_mut() {
            *word = data.read();
        }
        let sectors = words[60] as u32 | ((words[61] as u32) << 16);
        if sectors == 0 {
            None
        } else {
            Some(sectors)
        }
    })
}

fn read_sector_inner(device: IdeDevice, lba: u32, buf: &mut [u8; 512]) -> bool {
    unsafe {
        let bus = device.bus();
        let mut status = Port::<u8>::new(bus.status_cmd);
        Port::<u8>::new(bus.dev_ctrl).write(0x02);

        if !select_device(device, lba, &mut status, "before read") {
            return false;
        }

        Port::<u8>::new(bus.features).write(0);
        Port::<u8>::new(bus.seccount).write(1);
        Port::<u8>::new(bus.lba_lo).write(lba as u8);
        Port::<u8>::new(bus.lba_mid).write((lba >> 8) as u8);
        Port::<u8>::new(bus.lba_hi).write((lba >> 16) as u8);
        Port::<u8>::new(bus.status_cmd).write(CMD_READ);

        if !wait_drq(&mut status, device, lba, "read") {
            return false;
        }

        let mut data = Port::<u16>::new(bus.data);
        for chunk in buf.chunks_exact_mut(2) {
            let word = data.read();
            chunk[0] = word as u8;
            chunk[1] = (word >> 8) as u8;
        }

        wait_settle(&mut status, device, lba, "read settle")
    }
}

fn write_sector_inner(device: IdeDevice, lba: u32, buf: &[u8; 512]) -> bool {
    unsafe {
        let bus = device.bus();
        let mut status = Port::<u8>::new(bus.status_cmd);
        Port::<u8>::new(bus.dev_ctrl).write(0x02);

        if !select_device(device, lba, &mut status, "before write") {
            return false;
        }

        Port::<u8>::new(bus.features).write(0);
        Port::<u8>::new(bus.seccount).write(1);
        Port::<u8>::new(bus.lba_lo).write(lba as u8);
        Port::<u8>::new(bus.lba_mid).write((lba >> 8) as u8);
        Port::<u8>::new(bus.lba_hi).write((lba >> 16) as u8);
        Port::<u8>::new(bus.status_cmd).write(CMD_WRITE);

        if !wait_drq(&mut status, device, lba, "write") {
            return false;
        }

        let mut data = Port::<u16>::new(bus.data);
        for chunk in buf.chunks_exact(2) {
            let word = chunk[0] as u16 | ((chunk[1] as u16) << 8);
            data.write(word);
        }

        wait_settle(&mut status, device, lba, "write settle")
    }
}

unsafe fn flush_device_inner(device: IdeDevice) -> bool {
    let bus = device.bus();
    let mut status = Port::<u8>::new(bus.status_cmd);
    Port::<u8>::new(bus.dev_ctrl).write(0x02);
    if !select_device(device, 0, &mut status, "before flush") {
        return false;
    }
    Port::<u8>::new(bus.status_cmd).write(CMD_CACHE_FLUSH);
    ata_delay(&mut status);
    wait_settle(&mut status, device, 0, "flush")
}

unsafe fn select_device(device: IdeDevice, lba: u32, status: &mut Port<u8>, phase: &str) -> bool {
    if !wait_idle(status, device, lba, phase) {
        return false;
    }
    let bus = device.bus();
    Port::<u8>::new(bus.drive_hdr).write(device.drive_select(lba));
    ata_delay(status);
    wait_idle(status, device, lba, "after drive select")
}

unsafe fn ata_delay(status: &mut Port<u8>) {
    for _ in 0..4 {
        let _ = status.read();
    }
}

unsafe fn wait_idle(status: &mut Port<u8>, device: IdeDevice, lba: u32, phase: &str) -> bool {
    let mut iters: u32 = 0;
    loop {
        let s = status.read();
        if s == 0xFF {
            return false;
        }
        if s & STATUS_BSY == 0 && s & STATUS_DRQ == 0 {
            return true;
        }
        iters += 1;
        if iters > ATA_TIMEOUT_ITERS {
            crate::println!(
                "[ata] idle timeout device={} phase={} lba={} status={:#x}",
                device.name(),
                phase,
                lba,
                s
            );
            return false;
        }
    }
}

unsafe fn wait_not_busy(status: &mut Port<u8>, device: IdeDevice, lba: u32, phase: &str) -> bool {
    let mut iters: u32 = 0;
    loop {
        let s = status.read();
        if s == 0xFF {
            return false;
        }
        if s & STATUS_ERR != 0 || s & STATUS_DF != 0 {
            return false;
        }
        if s & STATUS_BSY == 0 {
            return true;
        }
        iters += 1;
        if iters > ATA_TIMEOUT_ITERS {
            crate::println!(
                "[ata] busy timeout device={} phase={} lba={} status={:#x}",
                device.name(),
                phase,
                lba,
                s
            );
            return false;
        }
    }
}

unsafe fn wait_drq(status: &mut Port<u8>, device: IdeDevice, lba: u32, phase: &str) -> bool {
    let bus = device.bus();
    let mut iters: u32 = 0;
    loop {
        let s = status.read();
        if s == 0xFF {
            return false;
        }
        if s & STATUS_ERR != 0 || s & STATUS_DF != 0 {
            let err = Port::<u8>::new(bus.features).read();
            crate::println!(
                "[ata] error device={} phase={} lba={} status={:#x} err={:#x}",
                device.name(),
                phase,
                lba,
                s,
                err
            );
            return false;
        }
        if s & STATUS_BSY == 0 && s & STATUS_DRQ != 0 {
            return true;
        }
        iters += 1;
        if iters > ATA_TIMEOUT_ITERS {
            crate::println!(
                "[ata] DRQ timeout device={} phase={} lba={} status={:#x}",
                device.name(),
                phase,
                lba,
                s
            );
            return false;
        }
    }
}

unsafe fn wait_settle(status: &mut Port<u8>, device: IdeDevice, lba: u32, phase: &str) -> bool {
    let bus = device.bus();
    let mut iters: u32 = 0;
    loop {
        let s = status.read();
        if s == 0xFF {
            return false;
        }
        if s & STATUS_ERR != 0 || s & STATUS_DF != 0 {
            let err = Port::<u8>::new(bus.features).read();
            crate::println!(
                "[ata] settle error device={} phase={} lba={} status={:#x} err={:#x}",
                device.name(),
                phase,
                lba,
                s,
                err
            );
            return false;
        }
        if s & STATUS_BSY == 0 && s & STATUS_DRQ == 0 {
            return true;
        }
        iters += 1;
        if iters > ATA_TIMEOUT_ITERS {
            crate::println!(
                "[ata] settle timeout device={} phase={} lba={} status={:#x}",
                device.name(),
                phase,
                lba,
                s
            );
            return false;
        }
    }
}

unsafe fn soft_reset(device: IdeDevice, lba: u32, phase: &str) {
    crate::println!(
        "[ata] software reset device={} phase={} lba={}",
        device.name(),
        phase,
        lba
    );
    let bus = device.bus();
    let mut status = Port::<u8>::new(bus.status_cmd);
    Port::<u8>::new(bus.dev_ctrl).write(0x06);
    ata_delay(&mut status);
    Port::<u8>::new(bus.dev_ctrl).write(0x02);
    ata_delay(&mut status);
    let _ = wait_idle(&mut status, device, lba, "after software reset");
}
