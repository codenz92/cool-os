extern crate alloc;

use alloc::{vec, vec::Vec};
use spin::Mutex;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockDevice {
    Ide(crate::ata::IdeDevice),
    Sata0,
    Sata1,
    Sata2,
    Sata3,
    Sata4,
    Sata5,
    Sata6,
    Sata7,
}

#[derive(Clone, Copy)]
pub struct BlockDeviceInfo {
    pub device: BlockDevice,
    pub present: bool,
    pub sectors: u32,
}

#[derive(Clone, Copy)]
pub struct RootDisk {
    pub device: BlockDevice,
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

static ROOT_DISK: Mutex<Option<RootDisk>> = Mutex::new(None);

impl BlockDevice {
    pub const fn name(self) -> &'static str {
        match self {
            BlockDevice::Ide(device) => device.name(),
            BlockDevice::Sata0 => "sata0",
            BlockDevice::Sata1 => "sata1",
            BlockDevice::Sata2 => "sata2",
            BlockDevice::Sata3 => "sata3",
            BlockDevice::Sata4 => "sata4",
            BlockDevice::Sata5 => "sata5",
            BlockDevice::Sata6 => "sata6",
            BlockDevice::Sata7 => "sata7",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        if let Some(ide) = crate::ata::IdeDevice::parse(name) {
            return Some(BlockDevice::Ide(ide));
        }
        match name {
            "sata0" => Some(BlockDevice::Sata0),
            "sata1" => Some(BlockDevice::Sata1),
            "sata2" => Some(BlockDevice::Sata2),
            "sata3" => Some(BlockDevice::Sata3),
            "sata4" => Some(BlockDevice::Sata4),
            "sata5" => Some(BlockDevice::Sata5),
            "sata6" => Some(BlockDevice::Sata6),
            "sata7" => Some(BlockDevice::Sata7),
            _ => None,
        }
    }

    pub const fn sata_port(self) -> Option<u8> {
        match self {
            BlockDevice::Sata0 => Some(0),
            BlockDevice::Sata1 => Some(1),
            BlockDevice::Sata2 => Some(2),
            BlockDevice::Sata3 => Some(3),
            BlockDevice::Sata4 => Some(4),
            BlockDevice::Sata5 => Some(5),
            BlockDevice::Sata6 => Some(6),
            BlockDevice::Sata7 => Some(7),
            BlockDevice::Ide(_) => None,
        }
    }
}

pub fn init() {
    crate::ahci::init();
}

pub fn all_devices() -> Vec<BlockDevice> {
    let mut devices = vec![
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Slave),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Slave),
    ];
    devices.extend(crate::ahci::devices());
    devices
}

pub fn device_info(device: BlockDevice) -> BlockDeviceInfo {
    match device {
        BlockDevice::Ide(ide) => {
            let info = crate::ata::device_info(ide);
            BlockDeviceInfo {
                device: BlockDevice::Ide(info.device),
                present: info.present,
                sectors: info.sectors,
            }
        }
        _ => crate::ahci::device_info(device),
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
    let Some(root) = root_disk() else {
        return false;
    };
    if lba >= root.sectors {
        return false;
    }
    let Some(abs_lba) = root.base_lba.checked_add(lba) else {
        return false;
    };
    read_sector_from(root.device, abs_lba, buf)
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

pub fn read_sector_from(device: BlockDevice, lba: u32, buf: &mut [u8; 512]) -> bool {
    match device {
        BlockDevice::Ide(ide) => crate::ata::read_sector_from(ide, lba, buf),
        _ => crate::ahci::read_sector_from(device, lba, buf),
    }
}

pub fn write_sector_to(device: BlockDevice, lba: u32, buf: &[u8; 512]) -> bool {
    match device {
        BlockDevice::Ide(ide) => crate::ata::write_sector_to(ide, lba, buf),
        _ => crate::ahci::write_sector_to(device, lba, buf),
    }
}

pub fn flush_device(device: BlockDevice) -> bool {
    match device {
        BlockDevice::Ide(ide) => crate::ata::flush_device(ide),
        _ => crate::ahci::flush_device(device),
    }
}

pub fn find_gpt_partition(
    device: BlockDevice,
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

fn detect_root_disk() -> Option<RootDisk> {
    for device in root_priority_devices() {
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

    for device in all_devices() {
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

    for device in all_devices() {
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

fn root_priority_devices() -> Vec<BlockDevice> {
    let mut devices = vec![
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Slave),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Slave),
    ];
    devices.extend(crate::ahci::devices());
    devices
}
