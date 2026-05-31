pub const SECTOR_SIZE: usize = 512;
pub const BOOTLOADER_STAGE2_PARTITION_TYPE: u8 = 0x20;
pub const BOOT_FAT32_PARTITION_TYPE: u8 = 0x0c;
pub const COOLFS_PARTITION_TYPE: u8 = 0xc0;
pub const INSTALL_ALIGNMENT_SECTORS: u32 = 2048;
pub const GPT_ENTRY_SIZE: usize = 128;
pub const GPT_ENTRY_COUNT: usize = 128;
pub const GPT_ENTRY_SECTORS: u32 = 32;
pub const GPT_PRIMARY_HEADER_LBA: u32 = 1;
pub const GPT_PRIMARY_ENTRIES_LBA: u32 = 2;
pub const GPT_TRAILING_SECTORS: u32 = 33;

const MBR_PARTITION_TABLE_OFFSET: usize = 446;
const MBR_PARTITION_ENTRY_SIZE: usize = 16;
const MBR_SIGNATURE_OFFSET: usize = 510;
const MBR_SIGNATURE: [u8; 2] = [0x55, 0xaa];
const COOLFS_MAGIC: [u8; 8] = *b"COOLFS1\0";
const GPT_SIGNATURE: [u8; 8] = *b"EFI PART";
const GPT_HEADER_SIZE: u32 = 92;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GptGuid(pub [u8; 16]);

// GPT stores the first three UUID fields little-endian.
pub const EFI_SYSTEM_PARTITION_GUID: GptGuid = GptGuid([
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
]);
pub const COOLFS_GPT_PARTITION_GUID: GptGuid = GptGuid([
    0xf0, 0xc6, 0xc7, 0xb9, 0x2e, 0x9d, 0x6a, 0x4f, 0x9a, 0x3d, 0x43, 0x4f, 0x4f, 0x4c, 0x46, 0x53,
]);
pub const COOLFS_GPT_PARTITION_GUID_TEXT: &str = "B9C7C6F0-9D2E-4F6A-9A3D-434F4F4C4653";
pub const INSTALL_DISK_GUID: GptGuid = GptGuid([
    0x85, 0x00, 0x00, 0xc0, 0x00, 0x85, 0x00, 0x45, 0x90, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
]);
pub const INSTALL_ESP_UNIQUE_GUID: GptGuid = GptGuid([
    0x85, 0x10, 0x00, 0xc0, 0x00, 0x85, 0x00, 0x45, 0x91, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
]);
pub const INSTALL_ROOT_UNIQUE_GUID: GptGuid = GptGuid([
    0x85, 0x20, 0x00, 0xc0, 0x00, 0x85, 0x00, 0x45, 0x92, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
]);

#[derive(Clone, Copy)]
pub struct MbrPartition {
    pub boot: u8,
    pub sys: u8,
    pub starting_lba: u32,
    pub sectors: u32,
}

#[derive(Clone, Copy)]
pub struct GptHeader {
    pub entries_lba: u32,
    pub entry_count: u32,
    pub entry_size: u32,
}

#[derive(Clone, Copy)]
pub struct GptPartition {
    pub type_guid: GptGuid,
    pub starting_lba: u32,
    pub ending_lba: u32,
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

pub fn has_protective_mbr(sector: &[u8; SECTOR_SIZE]) -> bool {
    let Some(partitions) = parse_mbr(sector) else {
        return false;
    };
    partitions
        .iter()
        .any(|partition| partition.sys == 0xee && partition.starting_lba == 1)
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

pub fn parse_gpt_header(sector: &[u8; SECTOR_SIZE], disk_sectors: u32) -> Option<GptHeader> {
    if sector.get(0..8) != Some(&GPT_SIGNATURE) {
        return None;
    }
    let header_size = read_u32(sector, 12)?;
    if !(GPT_HEADER_SIZE..=SECTOR_SIZE as u32).contains(&header_size) {
        return None;
    }
    let current_lba = u32::try_from(read_u64(sector, 24)?).ok()?;
    let backup_lba = u32::try_from(read_u64(sector, 32)?).ok()?;
    let first_usable_lba = u32::try_from(read_u64(sector, 40)?).ok()?;
    let last_usable_lba = u32::try_from(read_u64(sector, 48)?).ok()?;
    let entries_lba = u32::try_from(read_u64(sector, 72)?).ok()?;
    let entry_count = read_u32(sector, 80)?;
    let entry_size = read_u32(sector, 84)?;
    if entry_size != GPT_ENTRY_SIZE as u32 || entry_count == 0 {
        return None;
    }
    if current_lba >= disk_sectors || backup_lba >= disk_sectors {
        return None;
    }
    if first_usable_lba > last_usable_lba || last_usable_lba >= disk_sectors {
        return None;
    }
    Some(GptHeader {
        entries_lba,
        entry_count,
        entry_size,
    })
}

pub fn parse_gpt_partition_entry(entry: &[u8]) -> Option<GptPartition> {
    if entry.len() < GPT_ENTRY_SIZE {
        return None;
    }
    let mut type_guid = [0u8; 16];
    type_guid.copy_from_slice(&entry[0..16]);
    if type_guid.iter().all(|&byte| byte == 0) {
        return None;
    }
    let starting_lba = u32::try_from(read_u64(entry, 32)?).ok()?;
    let ending_lba = u32::try_from(read_u64(entry, 40)?).ok()?;
    if starting_lba == 0 || ending_lba < starting_lba {
        return None;
    }
    Some(GptPartition {
        type_guid: GptGuid(type_guid),
        starting_lba,
        ending_lba,
    })
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

pub fn write_protective_mbr(sector: &mut [u8; SECTOR_SIZE], total_sectors: u32) {
    sector.fill(0);
    let protective_sectors = total_sectors.saturating_sub(1);
    let off = MBR_PARTITION_TABLE_OFFSET;
    sector[off + 4] = 0xee;
    sector[off + 8..off + 12].copy_from_slice(&1u32.to_le_bytes());
    sector[off + 12..off + 16].copy_from_slice(&protective_sectors.to_le_bytes());
    sector[MBR_SIGNATURE_OFFSET..MBR_SIGNATURE_OFFSET + 2].copy_from_slice(&MBR_SIGNATURE);
}

pub fn write_gpt_partition_entry(
    entries: &mut [u8],
    index: usize,
    type_guid: GptGuid,
    unique_guid: GptGuid,
    starting_lba: u32,
    sectors: u32,
    name: &str,
) -> Option<()> {
    if sectors == 0 {
        return None;
    }
    let off = index.checked_mul(GPT_ENTRY_SIZE)?;
    if off + GPT_ENTRY_SIZE > entries.len() {
        return None;
    }
    let ending_lba = starting_lba.checked_add(sectors.checked_sub(1)?)?;
    let entry = &mut entries[off..off + GPT_ENTRY_SIZE];
    entry.fill(0);
    entry[0..16].copy_from_slice(&type_guid.0);
    entry[16..32].copy_from_slice(&unique_guid.0);
    write_u64(entry, 32, starting_lba as u64)?;
    write_u64(entry, 40, ending_lba as u64)?;
    for (idx, byte) in name.bytes().take(36).enumerate() {
        let name_off = 56 + idx * 2;
        entry[name_off] = byte;
        entry[name_off + 1] = 0;
    }
    Some(())
}

pub fn write_gpt_header(
    sector: &mut [u8; SECTOR_SIZE],
    current_lba: u32,
    backup_lba: u32,
    first_usable_lba: u32,
    last_usable_lba: u32,
    entries_lba: u32,
    entry_count: u32,
    entries_crc: u32,
    disk_guid: GptGuid,
) -> Option<()> {
    sector.fill(0);
    sector[0..8].copy_from_slice(&GPT_SIGNATURE);
    sector[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
    sector[12..16].copy_from_slice(&GPT_HEADER_SIZE.to_le_bytes());
    write_u64(sector, 24, current_lba as u64)?;
    write_u64(sector, 32, backup_lba as u64)?;
    write_u64(sector, 40, first_usable_lba as u64)?;
    write_u64(sector, 48, last_usable_lba as u64)?;
    sector[56..72].copy_from_slice(&disk_guid.0);
    write_u64(sector, 72, entries_lba as u64)?;
    sector[80..84].copy_from_slice(&entry_count.to_le_bytes());
    sector[84..88].copy_from_slice(&(GPT_ENTRY_SIZE as u32).to_le_bytes());
    sector[88..92].copy_from_slice(&entries_crc.to_le_bytes());
    let header_crc = crc32(&sector[..GPT_HEADER_SIZE as usize]);
    sector[16..20].copy_from_slice(&header_crc.to_le_bytes());
    Some(())
}

pub fn find_partition(partitions: &[MbrPartition; 4], sys: u8) -> Option<MbrPartition> {
    partitions
        .iter()
        .copied()
        .find(|partition| partition.sys == sys && !partition.is_unused())
}

impl GptPartition {
    pub fn sectors(self) -> Option<u32> {
        self.ending_lba
            .checked_sub(self.starting_lba)?
            .checked_add(1)
    }

    pub fn end_lba(self) -> Option<u32> {
        self.ending_lba.checked_add(1)
    }
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

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 > bytes.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ]))
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) -> Option<()> {
    if offset + 8 > bytes.len() {
        return None;
    }
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    Some(())
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
