use std::{
    env,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const SECTOR_SIZE: usize = 512;
const DEFAULT_SIZE_MIB: u64 = 96;
const GPT_ENTRY_SIZE: usize = 128;
const GPT_ENTRY_COUNT: usize = 128;
const GPT_ENTRY_SECTORS: u64 = 32;
const GPT_PRIMARY_HEADER_LBA: u64 = 1;
const GPT_PRIMARY_ENTRIES_LBA: u64 = 2;
const GPT_TRAILING_SECTORS: u64 = 33;
const INSTALL_ALIGNMENT_SECTORS: u64 = 2048;
const GPT_HEADER_SIZE: usize = 92;

const EFI_SYSTEM_PARTITION_GUID: [u8; 16] = [
    0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
];
const COOLFS_GPT_PARTITION_GUID: [u8; 16] = [
    0xf0, 0xc6, 0xc7, 0xb9, 0x2e, 0x9d, 0x6a, 0x4f, 0x9a, 0x3d, 0x43, 0x4f, 0x4f, 0x4c, 0x46, 0x53,
];
const INSTALL_DISK_GUID: [u8; 16] = [
    0x86, 0x00, 0x00, 0xc0, 0x00, 0x86, 0x00, 0x45, 0x90, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
];
const INSTALL_ESP_UNIQUE_GUID: [u8; 16] = [
    0x86, 0x10, 0x00, 0xc0, 0x00, 0x86, 0x00, 0x45, 0x91, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
];
const INSTALL_ROOT_UNIQUE_GUID: [u8; 16] = [
    0x86, 0x20, 0x00, 0xc0, 0x00, 0x86, 0x00, 0x45, 0x92, 0x00, 0x43, 0x4f, 0x4f, 0x4c, 0x4f, 0x53,
];

#[derive(Clone, Copy)]
struct Partition {
    type_guid: [u8; 16],
    start: u64,
    end: u64,
}

fn main() {
    let mut args = env::args().skip(1);
    let uefi = PathBuf::from(args.next().expect("usage: usb-image <uefi.img> <fs.img> <out.img> [size_mib]"));
    let fs = PathBuf::from(args.next().expect("usage: usb-image <uefi.img> <fs.img> <out.img> [size_mib]"));
    let out = PathBuf::from(args.next().expect("usage: usb-image <uefi.img> <fs.img> <out.img> [size_mib]"));
    let size_mib = args
        .next()
        .map(|value| value.parse::<u64>().expect("size_mib must be an integer"))
        .unwrap_or(DEFAULT_SIZE_MIB);

    let esp = find_esp(&uefi).expect("source uefi.img has no readable ESP GPT partition");
    let fs_len = std::fs::metadata(&fs).expect("fs.img metadata failed").len();
    let root_sectors = div_ceil(fs_len, SECTOR_SIZE as u64);
    let target_sectors = size_mib * 1024 * 1024 / SECTOR_SIZE as u64;
    let esp_end = esp.end + 1;
    let root_start = align_up(esp_end, INSTALL_ALIGNMENT_SECTORS);
    let root_end = root_start + root_sectors - 1;
    let backup_entries_lba = target_sectors - GPT_TRAILING_SECTORS;
    let last_usable_lba = backup_entries_lba - 1;
    if root_end > last_usable_lba {
        panic!(
            "target too small: need root through LBA {}, last usable {}",
            root_end, last_usable_lba
        );
    }

    let mut out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&out)
        .expect("open output image failed");
    out_file
        .set_len(target_sectors * SECTOR_SIZE as u64)
        .expect("resize output image failed");

    copy_sectors(&uefi, &mut out_file, esp.start, esp.start, esp.end - esp.start + 1)
        .expect("copy ESP failed");
    copy_file_to_lba(&fs, &mut out_file, root_start).expect("copy CoolFS failed");
    write_gpt(
        &mut out_file,
        target_sectors,
        esp,
        Partition {
            type_guid: COOLFS_GPT_PARTITION_GUID,
            start: root_start,
            end: root_end,
        },
    )
    .expect("write GPT failed");

    println!(
        "{} size_mib={} esp_lba={} esp_sectors={} root_lba={} root_sectors={}",
        out.display(),
        size_mib,
        esp.start,
        esp.end - esp.start + 1,
        root_start,
        root_sectors
    );
}

fn find_esp(path: &PathBuf) -> std::io::Result<Partition> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    let disk_sectors = file.metadata()?.len() / SECTOR_SIZE as u64;
    let header = read_sector(&mut file, GPT_PRIMARY_HEADER_LBA)?;
    if header.get(0..8) != Some(b"EFI PART") {
        panic!("source uefi.img has no GPT header");
    }
    let entries_lba = read_u64(&header, 72);
    let entry_count = read_u32(&header, 80).min(GPT_ENTRY_COUNT as u32);
    let entry_size = read_u32(&header, 84) as usize;
    if entry_size != GPT_ENTRY_SIZE {
        panic!("unsupported GPT entry size {}", entry_size);
    }
    for idx in 0..entry_count {
        let entry_lba = entries_lba + (idx as usize * entry_size / SECTOR_SIZE) as u64;
        let entry_off = idx as usize * entry_size % SECTOR_SIZE;
        let sector = read_sector(&mut file, entry_lba)?;
        let entry = &sector[entry_off..entry_off + GPT_ENTRY_SIZE];
        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&entry[0..16]);
        if type_guid != EFI_SYSTEM_PARTITION_GUID {
            continue;
        }
        let start = read_u64(entry, 32);
        let end = read_u64(entry, 40);
        if start == 0 || end < start || end >= disk_sectors {
            panic!("invalid ESP bounds {}..{}", start, end);
        }
        return Ok(Partition {
            type_guid,
            start,
            end,
        });
    }
    panic!("ESP partition not found");
}

fn copy_sectors(
    src: &PathBuf,
    out: &mut std::fs::File,
    src_lba: u64,
    dst_lba: u64,
    sectors: u64,
) -> std::io::Result<()> {
    let mut src_file = OpenOptions::new().read(true).open(src)?;
    let mut buf = [0u8; SECTOR_SIZE];
    for offset in 0..sectors {
        src_file.seek(SeekFrom::Start((src_lba + offset) * SECTOR_SIZE as u64))?;
        src_file.read_exact(&mut buf)?;
        out.seek(SeekFrom::Start((dst_lba + offset) * SECTOR_SIZE as u64))?;
        out.write_all(&buf)?;
    }
    Ok(())
}

fn copy_file_to_lba(src: &PathBuf, out: &mut std::fs::File, dst_lba: u64) -> std::io::Result<()> {
    let mut src_file = OpenOptions::new().read(true).open(src)?;
    out.seek(SeekFrom::Start(dst_lba * SECTOR_SIZE as u64))?;
    std::io::copy(&mut src_file, out)?;
    Ok(())
}

fn write_gpt(
    out: &mut std::fs::File,
    target_sectors: u64,
    esp: Partition,
    root: Partition,
) -> std::io::Result<()> {
    let backup_entries_lba = target_sectors - GPT_TRAILING_SECTORS;
    let backup_header_lba = target_sectors - 1;
    let first_usable_lba = GPT_PRIMARY_ENTRIES_LBA + GPT_ENTRY_SECTORS;
    let last_usable_lba = backup_entries_lba - 1;
    let mut mbr = [0u8; SECTOR_SIZE];
    write_protective_mbr(&mut mbr, target_sectors);
    write_sector(out, 0, &mbr)?;

    let mut entries = vec![0u8; GPT_ENTRY_SIZE * GPT_ENTRY_COUNT];
    write_gpt_entry(
        &mut entries,
        0,
        esp.type_guid,
        INSTALL_ESP_UNIQUE_GUID,
        esp.start,
        esp.end,
        "EFI System",
    );
    write_gpt_entry(
        &mut entries,
        1,
        root.type_guid,
        INSTALL_ROOT_UNIQUE_GUID,
        root.start,
        root.end,
        "coolOS CoolFS",
    );
    let entries_crc = crc32(&entries);
    write_bytes_at_lba(out, GPT_PRIMARY_ENTRIES_LBA, &entries)?;
    write_bytes_at_lba(out, backup_entries_lba, &entries)?;

    let mut header = [0u8; SECTOR_SIZE];
    write_gpt_header(
        &mut header,
        GPT_PRIMARY_HEADER_LBA,
        backup_header_lba,
        first_usable_lba,
        last_usable_lba,
        GPT_PRIMARY_ENTRIES_LBA,
        entries_crc,
    );
    write_sector(out, GPT_PRIMARY_HEADER_LBA, &header)?;

    let mut backup_header = [0u8; SECTOR_SIZE];
    write_gpt_header(
        &mut backup_header,
        backup_header_lba,
        GPT_PRIMARY_HEADER_LBA,
        first_usable_lba,
        last_usable_lba,
        backup_entries_lba,
        entries_crc,
    );
    write_sector(out, backup_header_lba, &backup_header)?;
    out.flush()
}

fn write_protective_mbr(sector: &mut [u8; SECTOR_SIZE], total_sectors: u64) {
    sector.fill(0);
    sector[446 + 4] = 0xee;
    sector[446 + 8..446 + 12].copy_from_slice(&1u32.to_le_bytes());
    sector[446 + 12..446 + 16]
        .copy_from_slice(&(total_sectors.saturating_sub(1).min(u32::MAX as u64) as u32).to_le_bytes());
    sector[510..512].copy_from_slice(&[0x55, 0xaa]);
}

fn write_gpt_entry(
    entries: &mut [u8],
    index: usize,
    type_guid: [u8; 16],
    unique_guid: [u8; 16],
    start: u64,
    end: u64,
    name: &str,
) {
    let off = index * GPT_ENTRY_SIZE;
    let entry = &mut entries[off..off + GPT_ENTRY_SIZE];
    entry.fill(0);
    entry[0..16].copy_from_slice(&type_guid);
    entry[16..32].copy_from_slice(&unique_guid);
    entry[32..40].copy_from_slice(&start.to_le_bytes());
    entry[40..48].copy_from_slice(&end.to_le_bytes());
    for (idx, byte) in name.bytes().take(36).enumerate() {
        let name_off = 56 + idx * 2;
        entry[name_off] = byte;
    }
}

fn write_gpt_header(
    sector: &mut [u8; SECTOR_SIZE],
    current_lba: u64,
    backup_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    entries_lba: u64,
    entries_crc: u32,
) {
    sector.fill(0);
    sector[0..8].copy_from_slice(b"EFI PART");
    sector[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
    sector[12..16].copy_from_slice(&(GPT_HEADER_SIZE as u32).to_le_bytes());
    sector[24..32].copy_from_slice(&current_lba.to_le_bytes());
    sector[32..40].copy_from_slice(&backup_lba.to_le_bytes());
    sector[40..48].copy_from_slice(&first_usable_lba.to_le_bytes());
    sector[48..56].copy_from_slice(&last_usable_lba.to_le_bytes());
    sector[56..72].copy_from_slice(&INSTALL_DISK_GUID);
    sector[72..80].copy_from_slice(&entries_lba.to_le_bytes());
    sector[80..84].copy_from_slice(&(GPT_ENTRY_COUNT as u32).to_le_bytes());
    sector[84..88].copy_from_slice(&(GPT_ENTRY_SIZE as u32).to_le_bytes());
    sector[88..92].copy_from_slice(&entries_crc.to_le_bytes());
    let header_crc = crc32(&sector[..GPT_HEADER_SIZE]);
    sector[16..20].copy_from_slice(&header_crc.to_le_bytes());
}

fn read_sector(file: &mut std::fs::File, lba: u64) -> std::io::Result<[u8; SECTOR_SIZE]> {
    let mut sector = [0u8; SECTOR_SIZE];
    file.seek(SeekFrom::Start(lba * SECTOR_SIZE as u64))?;
    file.read_exact(&mut sector)?;
    Ok(sector)
}

fn write_sector(file: &mut std::fs::File, lba: u64, sector: &[u8; SECTOR_SIZE]) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(lba * SECTOR_SIZE as u64))?;
    file.write_all(sector)
}

fn write_bytes_at_lba(file: &mut std::fs::File, lba: u64, bytes: &[u8]) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(lba * SECTOR_SIZE as u64))?;
    file.write_all(bytes)
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn div_ceil(value: u64, divisor: u64) -> u64 {
    value.div_ceil(divisor)
}

fn align_up(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

fn crc32(bytes: &[u8]) -> u32 {
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
