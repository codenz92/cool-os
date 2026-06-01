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
    Nvme0n1,
    Nvme1n1,
    Nvme2n1,
    Nvme3n1,
    Usb0,
    Usb1,
    Usb2,
    Usb3,
    Usb4,
    Usb5,
    Usb6,
    Usb7,
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

    pub const fn name(self) -> &'static str {
        match self {
            RootDiskLayout::RawCoolFs => "raw-coolfs",
            RootDiskLayout::MbrCoolFs => "mbr-coolfs",
            RootDiskLayout::GptCoolFs => "gpt-coolfs",
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
            BlockDevice::Nvme0n1 => "nvme0n1",
            BlockDevice::Nvme1n1 => "nvme1n1",
            BlockDevice::Nvme2n1 => "nvme2n1",
            BlockDevice::Nvme3n1 => "nvme3n1",
            BlockDevice::Usb0 => "usb0",
            BlockDevice::Usb1 => "usb1",
            BlockDevice::Usb2 => "usb2",
            BlockDevice::Usb3 => "usb3",
            BlockDevice::Usb4 => "usb4",
            BlockDevice::Usb5 => "usb5",
            BlockDevice::Usb6 => "usb6",
            BlockDevice::Usb7 => "usb7",
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
            "nvme0n1" => Some(BlockDevice::Nvme0n1),
            "nvme1n1" => Some(BlockDevice::Nvme1n1),
            "nvme2n1" => Some(BlockDevice::Nvme2n1),
            "nvme3n1" => Some(BlockDevice::Nvme3n1),
            "usb0" => Some(BlockDevice::Usb0),
            "usb1" => Some(BlockDevice::Usb1),
            "usb2" => Some(BlockDevice::Usb2),
            "usb3" => Some(BlockDevice::Usb3),
            "usb4" => Some(BlockDevice::Usb4),
            "usb5" => Some(BlockDevice::Usb5),
            "usb6" => Some(BlockDevice::Usb6),
            "usb7" => Some(BlockDevice::Usb7),
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
            BlockDevice::Ide(_)
            | BlockDevice::Nvme0n1
            | BlockDevice::Nvme1n1
            | BlockDevice::Nvme2n1
            | BlockDevice::Nvme3n1
            | BlockDevice::Usb0
            | BlockDevice::Usb1
            | BlockDevice::Usb2
            | BlockDevice::Usb3
            | BlockDevice::Usb4
            | BlockDevice::Usb5
            | BlockDevice::Usb6
            | BlockDevice::Usb7 => None,
        }
    }

    pub const fn nvme_index(self) -> Option<u8> {
        match self {
            BlockDevice::Nvme0n1 => Some(0),
            BlockDevice::Nvme1n1 => Some(1),
            BlockDevice::Nvme2n1 => Some(2),
            BlockDevice::Nvme3n1 => Some(3),
            _ => None,
        }
    }

    pub const fn usb_index(self) -> Option<u8> {
        match self {
            BlockDevice::Usb0 => Some(0),
            BlockDevice::Usb1 => Some(1),
            BlockDevice::Usb2 => Some(2),
            BlockDevice::Usb3 => Some(3),
            BlockDevice::Usb4 => Some(4),
            BlockDevice::Usb5 => Some(5),
            BlockDevice::Usb6 => Some(6),
            BlockDevice::Usb7 => Some(7),
            _ => None,
        }
    }

    pub const fn bus_label(self) -> &'static str {
        match self {
            BlockDevice::Ide(_) => "ide",
            BlockDevice::Sata0
            | BlockDevice::Sata1
            | BlockDevice::Sata2
            | BlockDevice::Sata3
            | BlockDevice::Sata4
            | BlockDevice::Sata5
            | BlockDevice::Sata6
            | BlockDevice::Sata7 => "sata",
            BlockDevice::Nvme0n1
            | BlockDevice::Nvme1n1
            | BlockDevice::Nvme2n1
            | BlockDevice::Nvme3n1 => "nvme",
            BlockDevice::Usb0
            | BlockDevice::Usb1
            | BlockDevice::Usb2
            | BlockDevice::Usb3
            | BlockDevice::Usb4
            | BlockDevice::Usb5
            | BlockDevice::Usb6
            | BlockDevice::Usb7 => "usb",
        }
    }

    pub const fn is_physical_install_target(self) -> bool {
        self.sata_port().is_some() || self.nvme_index().is_some()
    }
}

pub fn init() {
    crate::ahci::init();
    crate::nvme::init();
    crate::usb::init_storage();
}

pub fn all_devices() -> Vec<BlockDevice> {
    let mut devices = vec![
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Slave),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Master),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide1Slave),
    ];
    devices.extend(crate::ahci::devices());
    devices.extend(crate::nvme::devices());
    devices.extend(crate::usb::storage_devices());
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
        BlockDevice::Sata0
        | BlockDevice::Sata1
        | BlockDevice::Sata2
        | BlockDevice::Sata3
        | BlockDevice::Sata4
        | BlockDevice::Sata5
        | BlockDevice::Sata6
        | BlockDevice::Sata7 => crate::ahci::device_info(device),
        BlockDevice::Nvme0n1
        | BlockDevice::Nvme1n1
        | BlockDevice::Nvme2n1
        | BlockDevice::Nvme3n1 => crate::nvme::device_info(device),
        _ => crate::usb::storage_device_info(device),
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
        BlockDevice::Sata0
        | BlockDevice::Sata1
        | BlockDevice::Sata2
        | BlockDevice::Sata3
        | BlockDevice::Sata4
        | BlockDevice::Sata5
        | BlockDevice::Sata6
        | BlockDevice::Sata7 => crate::ahci::read_sector_from(device, lba, buf),
        BlockDevice::Nvme0n1
        | BlockDevice::Nvme1n1
        | BlockDevice::Nvme2n1
        | BlockDevice::Nvme3n1 => crate::nvme::read_sector_from(device, lba, buf),
        _ => crate::usb::storage_read_sector(device, lba, buf),
    }
}

pub fn write_sector_to(device: BlockDevice, lba: u32, buf: &[u8; 512]) -> bool {
    match device {
        BlockDevice::Ide(ide) => crate::ata::write_sector_to(ide, lba, buf),
        BlockDevice::Sata0
        | BlockDevice::Sata1
        | BlockDevice::Sata2
        | BlockDevice::Sata3
        | BlockDevice::Sata4
        | BlockDevice::Sata5
        | BlockDevice::Sata6
        | BlockDevice::Sata7 => crate::ahci::write_sector_to(device, lba, buf),
        BlockDevice::Nvme0n1
        | BlockDevice::Nvme1n1
        | BlockDevice::Nvme2n1
        | BlockDevice::Nvme3n1 => crate::nvme::write_sector_to(device, lba, buf),
        _ => crate::usb::storage_write_sector(device, lba, buf),
    }
}

pub fn flush_device(device: BlockDevice) -> bool {
    match device {
        BlockDevice::Ide(ide) => crate::ata::flush_device(ide),
        BlockDevice::Sata0
        | BlockDevice::Sata1
        | BlockDevice::Sata2
        | BlockDevice::Sata3
        | BlockDevice::Sata4
        | BlockDevice::Sata5
        | BlockDevice::Sata6
        | BlockDevice::Sata7 => crate::ahci::flush_device(device),
        BlockDevice::Nvme0n1
        | BlockDevice::Nvme1n1
        | BlockDevice::Nvme2n1
        | BlockDevice::Nvme3n1 => crate::nvme::flush_device(device),
        _ => crate::usb::storage_flush(device),
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
            let root = RootDisk {
                device,
                base_lba: 0,
                sectors: info.sectors,
                layout: RootDiskLayout::RawCoolFs,
            };
            log_root_disk(root);
            return Some(root);
        }
    }

    for device in root_priority_devices() {
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
            let root = RootDisk {
                device,
                base_lba: partition.starting_lba,
                sectors: partition.sectors,
                layout: RootDiskLayout::MbrCoolFs,
            };
            log_root_disk(root);
            return Some(root);
        }
    }

    for device in root_priority_devices() {
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
            let root = RootDisk {
                device,
                base_lba: partition.starting_lba,
                sectors: partition_sectors,
                layout: RootDiskLayout::GptCoolFs,
            };
            log_root_disk(root);
            return Some(root);
        }
    }

    crate::println!("[storage] root scan failed {}", root_scan_summary());
    None
}

fn log_root_disk(root: RootDisk) {
    crate::println!(
        "[storage] root device={} layout={} base_lba={} sectors={}",
        root.device.name(),
        root.layout.name(),
        root.base_lba,
        root.sectors,
    );
}

fn root_scan_summary() -> alloc::string::String {
    let mut present = Vec::new();
    for device in root_priority_devices() {
        let info = device_info(device);
        if info.present {
            present.push(device.name());
        }
    }
    if present.is_empty() {
        alloc::string::String::from("present=none")
    } else {
        let mut out = alloc::string::String::from("present=");
        for (idx, name) in present.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push_str(name);
        }
        out
    }
}

fn root_priority_devices() -> Vec<BlockDevice> {
    let mut devices = vec![
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Slave),
        BlockDevice::Ide(crate::ata::IdeDevice::Ide0Master),
    ];
    devices.extend(crate::usb::storage_devices());
    devices.extend(crate::nvme::devices());
    devices.extend(crate::ahci::devices());
    devices.push(BlockDevice::Ide(crate::ata::IdeDevice::Ide1Master));
    devices.push(BlockDevice::Ide(crate::ata::IdeDevice::Ide1Slave));
    devices
}
