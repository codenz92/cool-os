pub const SECTOR_SIZE: usize = 512;
pub const BOOTLOADER_STAGE2_PARTITION_TYPE: u8 = 0x20;
pub const BOOT_FAT32_PARTITION_TYPE: u8 = 0x0c;
pub const COOLFS_PARTITION_TYPE: u8 = 0xc0;
pub const INSTALL_ALIGNMENT_SECTORS: u32 = 2048;

const MBR_PARTITION_TABLE_OFFSET: usize = 446;
const MBR_PARTITION_ENTRY_SIZE: usize = 16;
const MBR_SIGNATURE_OFFSET: usize = 510;
const MBR_SIGNATURE: [u8; 2] = [0x55, 0xaa];
const COOLFS_MAGIC: [u8; 8] = *b"COOLFS1\0";

#[derive(Clone, Copy)]
pub struct MbrPartition {
    pub boot: u8,
    pub sys: u8,
    pub starting_lba: u32,
    pub sectors: u32,
}

impl MbrPartition {
    pub const fn empty() -> Self {
        Self {
            boot: 0,
            sys: 0,
            starting_lba: 0,
            sectors: 0,
        }
    }

    pub const fn is_unused(self) -> bool {
        self.sys == 0 || self.sectors == 0
    }

    pub fn end_lba(self) -> Option<u32> {
        if self.is_unused() {
            return Some(0);
        }
        self.starting_lba.checked_add(self.sectors)
    }
}

pub fn has_mbr_signature(sector: &[u8; SECTOR_SIZE]) -> bool {
    sector[MBR_SIGNATURE_OFFSET..MBR_SIGNATURE_OFFSET + 2] == MBR_SIGNATURE
}

pub fn has_coolfs_magic(sector: &[u8; SECTOR_SIZE]) -> bool {
    sector.get(0..8) == Some(&COOLFS_MAGIC)
}

pub fn parse_mbr(sector: &[u8; SECTOR_SIZE]) -> Option<[MbrPartition; 4]> {
    if !has_mbr_signature(sector) {
        return None;
    }
    let mut partitions = [MbrPartition::empty(); 4];
    for (idx, partition) in partitions.iter_mut().enumerate() {
        let off = MBR_PARTITION_TABLE_OFFSET + idx * MBR_PARTITION_ENTRY_SIZE;
        partition.boot = sector[off];
        partition.sys = sector[off + 4];
        partition.starting_lba = read_u32(sector, off + 8)?;
        partition.sectors = read_u32(sector, off + 12)?;
    }
    Some(partitions)
}

pub fn write_partition_entry(
    sector: &mut [u8; SECTOR_SIZE],
    index: usize,
    boot: u8,
    sys: u8,
    starting_lba: u32,
    sectors: u32,
) -> Option<()> {
    if index >= 4 {
        return None;
    }
    let off = MBR_PARTITION_TABLE_OFFSET + index * MBR_PARTITION_ENTRY_SIZE;
    sector[off] = boot;
    sector[off + 1..off + 4].fill(0);
    sector[off + 4] = sys;
    sector[off + 5..off + 8].fill(0);
    sector[off + 8..off + 12].copy_from_slice(&starting_lba.to_le_bytes());
    sector[off + 12..off + 16].copy_from_slice(&sectors.to_le_bytes());
    sector[MBR_SIGNATURE_OFFSET..MBR_SIGNATURE_OFFSET + 2].copy_from_slice(&MBR_SIGNATURE);
    Some(())
}

pub fn find_partition(partitions: &[MbrPartition; 4], sys: u8) -> Option<MbrPartition> {
    partitions
        .iter()
        .copied()
        .find(|partition| partition.sys == sys && !partition.is_unused())
}

pub fn boot_area_end_lba(partitions: &[MbrPartition; 4]) -> Option<u32> {
    let mut end = 1u32;
    for partition in partitions {
        if partition.sys == BOOTLOADER_STAGE2_PARTITION_TYPE
            || partition.sys == BOOT_FAT32_PARTITION_TYPE
        {
            end = end.max(partition.end_lba()?);
        }
    }
    Some(end)
}

pub fn align_up_lba(lba: u32, alignment: u32) -> Option<u32> {
    if alignment == 0 {
        return Some(lba);
    }
    let add = alignment.checked_sub(1)?;
    let rounded = lba.checked_add(add)?;
    Some(rounded / alignment * alignment)
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > bytes.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}
