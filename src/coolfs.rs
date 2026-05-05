/// CoolFS: a small native filesystem mounted at `/COOL`.
///
/// The current block device remains FAT32 so existing boot and userspace flows
/// keep working. CoolFS stores its own superblock, inode table, block bitmap,
/// and directory records inside a persistent `/COOLFS.IMG` file on that disk.
extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use spin::Mutex;

use crate::fat32::{DirEntryInfo, FsError};

pub const MOUNT_PATH: &str = "/COOL";
const BACKING_PATH: &str = "/COOLFS.IMG";

const MAGIC: [u8; 8] = *b"COOLFS1\0";
const VERSION: u32 = 1;
const BLOCK_SIZE: usize = 512;
const TOTAL_BLOCKS: u32 = 512;
const INODE_COUNT: u32 = 128;
const INODE_SIZE: usize = 256;
const DIRECT_BLOCKS: usize = 48;
const DIR_ENTRY_SIZE: usize = 32;
const MAX_NAME_LEN: usize = 27;

const INODE_TABLE_START: u32 = 1;
const INODE_TABLE_BLOCKS: u32 =
    ((INODE_COUNT as usize * INODE_SIZE + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;
const BITMAP_START: u32 = INODE_TABLE_START + INODE_TABLE_BLOCKS;
const BITMAP_BLOCKS: u32 = 1;
const DATA_START: u32 = BITMAP_START + BITMAP_BLOCKS;
const ROOT_INODE: u32 = 0;

const KIND_FREE: u8 = 0;
const KIND_FILE: u8 = 1;
const KIND_DIR: u8 = 2;

static COOLFS_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
struct Superblock {
    total_blocks: u32,
    inode_count: u32,
    inode_table_start: u32,
    inode_table_blocks: u32,
    bitmap_start: u32,
    bitmap_blocks: u32,
    data_start: u32,
    root_inode: u32,
}

impl Superblock {
    const fn new() -> Self {
        Self {
            total_blocks: TOTAL_BLOCKS,
            inode_count: INODE_COUNT,
            inode_table_start: INODE_TABLE_START,
            inode_table_blocks: INODE_TABLE_BLOCKS,
            bitmap_start: BITMAP_START,
            bitmap_blocks: BITMAP_BLOCKS,
            data_start: DATA_START,
            root_inode: ROOT_INODE,
        }
    }

    fn parse(image: &[u8]) -> Option<Self> {
        if image.len() < BLOCK_SIZE || image.get(0..8)? != MAGIC {
            return None;
        }
        if read_u32(image, 8)? != VERSION
            || read_u32(image, 12)? != BLOCK_SIZE as u32
            || read_u32(image, 24)? != INODE_SIZE as u32
        {
            return None;
        }
        let sb = Self {
            total_blocks: read_u32(image, 16)?,
            inode_count: read_u32(image, 20)?,
            inode_table_start: read_u32(image, 28)?,
            inode_table_blocks: read_u32(image, 32)?,
            bitmap_start: read_u32(image, 36)?,
            bitmap_blocks: read_u32(image, 40)?,
            data_start: read_u32(image, 44)?,
            root_inode: read_u32(image, 48)?,
        };
        if sb.total_blocks == 0
            || sb.inode_count == 0
            || sb.root_inode >= sb.inode_count
            || sb.data_start >= sb.total_blocks
            || sb.bitmap_start + sb.bitmap_blocks > sb.data_start
        {
            return None;
        }
        Some(sb)
    }

    fn write(self, image: &mut [u8]) {
        image[0..8].copy_from_slice(&MAGIC);
        write_u32(image, 8, VERSION);
        write_u32(image, 12, BLOCK_SIZE as u32);
        write_u32(image, 16, self.total_blocks);
        write_u32(image, 20, self.inode_count);
        write_u32(image, 24, INODE_SIZE as u32);
        write_u32(image, 28, self.inode_table_start);
        write_u32(image, 32, self.inode_table_blocks);
        write_u32(image, 36, self.bitmap_start);
        write_u32(image, 40, self.bitmap_blocks);
        write_u32(image, 44, self.data_start);
        write_u32(image, 48, self.root_inode);
    }
}

#[derive(Clone)]
struct Inode {
    kind: u8,
    size: u32,
    direct: [u32; DIRECT_BLOCKS],
}

impl Inode {
    const fn free() -> Self {
        Self {
            kind: KIND_FREE,
            size: 0,
            direct: [0; DIRECT_BLOCKS],
        }
    }

    fn new(kind: u8) -> Self {
        Self {
            kind,
            size: 0,
            direct: [0; DIRECT_BLOCKS],
        }
    }

    fn is_dir(&self) -> bool {
        self.kind == KIND_DIR
    }

    fn is_file(&self) -> bool {
        self.kind == KIND_FILE
    }
}

struct DirectoryEntry {
    inode: u32,
    name: String,
}

struct Image {
    data: Vec<u8>,
    sb: Superblock,
}

#[derive(Clone, Copy)]
pub struct CoolFsStats {
    pub total_blocks: u32,
    pub used_blocks: u32,
    pub free_blocks: u32,
    pub block_size: u32,
    pub files: u32,
    pub dirs: u32,
}

#[derive(Clone, Copy)]
pub struct CoolFsCheckReport {
    pub ok: bool,
    pub root_entries: usize,
    pub stats: CoolFsStats,
}

pub fn mount_or_format() -> Result<(), FsError> {
    let _guard = COOLFS_LOCK.lock();
    let _ = load_or_format()?;
    Ok(())
}

pub fn lines() -> Vec<String> {
    match stats() {
        Some(stats) => alloc::vec![
            format!(
                "mount {} type=coolfs flags=rw,native-image,bitmap-inodes",
                MOUNT_PATH
            ),
            format!(
                "image={} blocks used={} free={} bytes/block={}",
                BACKING_PATH, stats.used_blocks, stats.free_blocks, stats.block_size
            ),
            format!("inodes files={} dirs={}", stats.files, stats.dirs),
        ],
        None => alloc::vec![String::from("coolfs unavailable")],
    }
}

pub fn stats() -> Option<CoolFsStats> {
    let _guard = COOLFS_LOCK.lock();
    let image = load_or_format().ok()?;
    Some(image.stats())
}

pub fn check() -> Option<CoolFsCheckReport> {
    let _guard = COOLFS_LOCK.lock();
    let image = load_or_format().ok()?;
    let root_entries = read_dir_entries(&image, image.sb.root_inode).ok()?.len();
    let root = image.read_inode(image.sb.root_inode)?;
    let stats = image.stats();
    Some(CoolFsCheckReport {
        ok: root.is_dir() && stats.used_blocks <= stats.total_blocks,
        root_entries,
        stats,
    })
}

pub fn list_dir(path: &str) -> Option<Vec<DirEntryInfo>> {
    let _guard = COOLFS_LOCK.lock();
    let image = load_or_format().ok()?;
    let inode_idx = resolve_path(&image, path).ok()?;
    let inode = image.read_inode(inode_idx)?;
    if !inode.is_dir() {
        return None;
    }
    let entries = read_dir_entries(&image, inode_idx).ok()?;
    let mut out = Vec::new();
    for entry in entries {
        if let Some(child) = image.read_inode(entry.inode) {
            out.push(DirEntryInfo {
                name: entry.name,
                is_dir: child.is_dir(),
                size: child.size,
            });
        }
    }
    Some(out)
}

pub fn read_file(path: &str) -> Option<Vec<u8>> {
    let _guard = COOLFS_LOCK.lock();
    let image = load_or_format().ok()?;
    let inode_idx = resolve_path(&image, path).ok()?;
    let inode = image.read_inode(inode_idx)?;
    if !inode.is_file() {
        return None;
    }
    image.read_inode_bytes(&inode).ok()
}

pub fn create_file(path: &str) -> Result<(), FsError> {
    create_node(path, KIND_FILE)
}

pub fn create_dir(path: &str) -> Result<(), FsError> {
    create_node(path, KIND_DIR)
}

pub fn write_file(path: &str, data: &[u8]) -> Result<(), FsError> {
    let _guard = COOLFS_LOCK.lock();
    let mut image = load_or_format()?;
    let inode_idx = resolve_path(&image, path)?;
    let inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
    if !inode.is_file() {
        return Err(FsError::InvalidPath);
    }
    image.write_inode_bytes(inode_idx, data)?;
    persist(&image)?;
    record_mutation("coolfs-write", path);
    Ok(())
}

pub fn safe_write_file(path: &str, data: &[u8]) -> Result<(), FsError> {
    write_file(path, data)
}

pub fn delete_file(path: &str) -> Result<(), FsError> {
    let _guard = COOLFS_LOCK.lock();
    let mut image = load_or_format()?;
    let (parent_path, name) = split_parent_and_name(path)?;
    let parent_inode = resolve_path(&image, &parent_path)?;
    let mut entries = read_dir_entries(&image, parent_inode)?;
    let pos = entries
        .iter()
        .position(|entry| names_equal(&entry.name, &name))
        .ok_or(FsError::NotFound)?;
    let target_inode = entries[pos].inode;
    let target = image.read_inode(target_inode).ok_or(FsError::NotFound)?;
    if target.is_dir() && !read_dir_entries(&image, target_inode)?.is_empty() {
        return Err(FsError::NotEmpty);
    }

    entries.remove(pos);
    write_dir_entries(&mut image, parent_inode, &entries)?;
    image.free_inode(target_inode)?;
    persist(&image)?;
    record_mutation("coolfs-delete", path);
    Ok(())
}

pub fn rename(path: &str, new_name: &str) -> Result<(), FsError> {
    validate_name(new_name)?;
    let _guard = COOLFS_LOCK.lock();
    let mut image = load_or_format()?;
    let (parent_path, old_name) = split_parent_and_name(path)?;
    if names_equal(&old_name, new_name) {
        return Ok(());
    }
    let parent_inode = resolve_path(&image, &parent_path)?;
    let mut entries = read_dir_entries(&image, parent_inode)?;
    if entries
        .iter()
        .any(|entry| names_equal(&entry.name, new_name))
    {
        return Err(FsError::AlreadyExists);
    }
    let entry = entries
        .iter_mut()
        .find(|entry| names_equal(&entry.name, &old_name))
        .ok_or(FsError::NotFound)?;
    entry.name = String::from(new_name);
    write_dir_entries(&mut image, parent_inode, &entries)?;
    persist(&image)?;
    record_mutation("coolfs-rename", path);
    Ok(())
}

pub fn copy_file(src: &str, dst: &str) -> Result<(), FsError> {
    let data = read_file(src).ok_or(FsError::NotFound)?;
    create_file(dst)?;
    write_file(dst, &data)
}

fn create_node(path: &str, kind: u8) -> Result<(), FsError> {
    let _guard = COOLFS_LOCK.lock();
    let mut image = load_or_format()?;
    let (parent_path, name) = split_parent_and_name(path)?;
    validate_name(&name)?;
    let parent_inode = resolve_path(&image, &parent_path)?;
    let mut entries = read_dir_entries(&image, parent_inode)?;
    if entries.iter().any(|entry| names_equal(&entry.name, &name)) {
        return Err(FsError::AlreadyExists);
    }

    let inode_idx = image.alloc_inode(kind)?;
    entries.push(DirectoryEntry {
        inode: inode_idx,
        name,
    });
    if let Err(err) = write_dir_entries(&mut image, parent_inode, &entries) {
        let _ = image.free_inode(inode_idx);
        return Err(err);
    }

    persist(&image)?;
    record_mutation(
        if kind == KIND_DIR {
            "coolfs-create-dir"
        } else {
            "coolfs-create-file"
        },
        path,
    );
    Ok(())
}

impl Image {
    fn read_inode(&self, inode_idx: u32) -> Option<Inode> {
        if inode_idx >= self.sb.inode_count {
            return None;
        }
        let off = self.inode_offset(inode_idx)?;
        let kind = *self.data.get(off)?;
        let size = read_u32(&self.data, off + 4)?;
        let mut inode = Inode::new(kind);
        inode.size = size;
        for i in 0..DIRECT_BLOCKS {
            inode.direct[i] = read_u32(&self.data, off + 8 + i * 4)?;
        }
        Some(inode)
    }

    fn write_inode(&mut self, inode_idx: u32, inode: &Inode) -> Result<(), FsError> {
        let off = self.inode_offset(inode_idx).ok_or(FsError::Io)?;
        self.data[off] = inode.kind;
        self.data[off + 1..off + 4].fill(0);
        write_u32(&mut self.data, off + 4, inode.size);
        for i in 0..DIRECT_BLOCKS {
            write_u32(&mut self.data, off + 8 + i * 4, inode.direct[i]);
        }
        self.data[off + 8 + DIRECT_BLOCKS * 4..off + INODE_SIZE].fill(0);
        Ok(())
    }

    fn inode_offset(&self, inode_idx: u32) -> Option<usize> {
        let base = self.sb.inode_table_start as usize * BLOCK_SIZE;
        let off = base.checked_add(inode_idx as usize * INODE_SIZE)?;
        if off + INODE_SIZE <= self.data.len() {
            Some(off)
        } else {
            None
        }
    }

    fn block_range(&self, block: u32) -> Option<core::ops::Range<usize>> {
        if block >= self.sb.total_blocks {
            return None;
        }
        let start = block as usize * BLOCK_SIZE;
        let end = start + BLOCK_SIZE;
        if end <= self.data.len() {
            Some(start..end)
        } else {
            None
        }
    }

    fn bitmap_byte_bit(&self, block: u32) -> Option<(usize, u8)> {
        if block >= self.sb.total_blocks {
            return None;
        }
        let bitmap_start = self.sb.bitmap_start as usize * BLOCK_SIZE;
        let byte = bitmap_start + block as usize / 8;
        if byte >= self.data.len() {
            return None;
        }
        Some((byte, 1u8 << (block % 8)))
    }

    fn block_used(&self, block: u32) -> bool {
        self.bitmap_byte_bit(block)
            .map(|(byte, bit)| self.data[byte] & bit != 0)
            .unwrap_or(true)
    }

    fn set_block_used(&mut self, block: u32, used: bool) -> Result<(), FsError> {
        let (byte, bit) = self.bitmap_byte_bit(block).ok_or(FsError::Io)?;
        if used {
            self.data[byte] |= bit;
        } else {
            self.data[byte] &= !bit;
        }
        Ok(())
    }

    fn alloc_block(&mut self) -> Result<u32, FsError> {
        for block in self.sb.data_start..self.sb.total_blocks {
            if !self.block_used(block) {
                self.set_block_used(block, true)?;
                let range = self.block_range(block).ok_or(FsError::Io)?;
                self.data[range].fill(0);
                return Ok(block);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_block(&mut self, block: u32) -> Result<(), FsError> {
        if block < self.sb.data_start || block >= self.sb.total_blocks {
            return Ok(());
        }
        self.set_block_used(block, false)
    }

    fn alloc_inode(&mut self, kind: u8) -> Result<u32, FsError> {
        for inode_idx in 1..self.sb.inode_count {
            if self
                .read_inode(inode_idx)
                .map(|inode| inode.kind == KIND_FREE)
                .unwrap_or(false)
            {
                self.write_inode(inode_idx, &Inode::new(kind))?;
                return Ok(inode_idx);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_inode(&mut self, inode_idx: u32) -> Result<(), FsError> {
        let inode = self.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        for &block in inode.direct.iter().filter(|&&block| block != 0) {
            self.free_block(block)?;
        }
        self.write_inode(inode_idx, &Inode::free())
    }

    fn read_inode_bytes(&self, inode: &Inode) -> Result<Vec<u8>, FsError> {
        let mut out = Vec::with_capacity(inode.size as usize);
        let mut remaining = inode.size as usize;
        for &block in &inode.direct {
            if remaining == 0 {
                break;
            }
            if block == 0 {
                return Err(FsError::Io);
            }
            let range = self.block_range(block).ok_or(FsError::Io)?;
            let take = remaining.min(BLOCK_SIZE);
            out.extend_from_slice(&self.data[range.start..range.start + take]);
            remaining -= take;
        }
        if remaining == 0 {
            Ok(out)
        } else {
            Err(FsError::Io)
        }
    }

    fn write_inode_bytes(&mut self, inode_idx: u32, bytes: &[u8]) -> Result<(), FsError> {
        if bytes.len() > DIRECT_BLOCKS * BLOCK_SIZE {
            return Err(FsError::NoSpace);
        }
        let needed = (bytes.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut new_blocks = Vec::new();
        for _ in 0..needed {
            match self.alloc_block() {
                Ok(block) => new_blocks.push(block),
                Err(err) => {
                    for block in new_blocks {
                        let _ = self.free_block(block);
                    }
                    return Err(err);
                }
            }
        }

        for (idx, &block) in new_blocks.iter().enumerate() {
            let range = self.block_range(block).ok_or(FsError::Io)?;
            self.data[range.clone()].fill(0);
            let start = idx * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(bytes.len());
            self.data[range.start..range.start + (end - start)].copy_from_slice(&bytes[start..end]);
        }

        let mut inode = self.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        let old_blocks: Vec<u32> = inode
            .direct
            .iter()
            .copied()
            .filter(|&block| block != 0)
            .collect();
        inode.size = bytes.len() as u32;
        inode.direct = [0; DIRECT_BLOCKS];
        for (idx, block) in new_blocks.iter().copied().enumerate() {
            inode.direct[idx] = block;
        }
        self.write_inode(inode_idx, &inode)?;
        for block in old_blocks {
            self.free_block(block)?;
        }
        Ok(())
    }

    fn stats(&self) -> CoolFsStats {
        let mut used_blocks = 0u32;
        for block in 0..self.sb.total_blocks {
            if self.block_used(block) {
                used_blocks += 1;
            }
        }
        let mut files = 0u32;
        let mut dirs = 0u32;
        for inode_idx in 0..self.sb.inode_count {
            if let Some(inode) = self.read_inode(inode_idx) {
                match inode.kind {
                    KIND_FILE => files += 1,
                    KIND_DIR => dirs += 1,
                    _ => {}
                }
            }
        }
        CoolFsStats {
            total_blocks: self.sb.total_blocks,
            used_blocks,
            free_blocks: self.sb.total_blocks.saturating_sub(used_blocks),
            block_size: BLOCK_SIZE as u32,
            files,
            dirs,
        }
    }
}

fn load_or_format() -> Result<Image, FsError> {
    if let Some(bytes) = crate::fat32::read_file(BACKING_PATH) {
        if let Some(sb) = Superblock::parse(&bytes) {
            let expected = sb.total_blocks as usize * BLOCK_SIZE;
            if bytes.len() >= expected {
                return Ok(Image {
                    data: bytes[..expected].to_vec(),
                    sb,
                });
            }
        }
    }

    let image = format_image();
    persist(&image)?;
    Ok(image)
}

fn format_image() -> Image {
    let sb = Superblock::new();
    let mut data = alloc::vec![0u8; sb.total_blocks as usize * BLOCK_SIZE];
    sb.write(&mut data);
    let mut image = Image { data, sb };
    for block in 0..sb.data_start {
        let _ = image.set_block_used(block, true);
    }
    let _ = image.write_inode(sb.root_inode, &Inode::new(KIND_DIR));
    image
}

fn persist(image: &Image) -> Result<(), FsError> {
    match crate::fat32::create_file(BACKING_PATH) {
        Ok(()) | Err(FsError::AlreadyExists) => {}
        Err(err) => return Err(err),
    }
    crate::fat32::write_file(BACKING_PATH, &image.data)
}

fn resolve_path(image: &Image, path: &str) -> Result<u32, FsError> {
    let path = trim_abs_path(path)?;
    if path == "/" {
        return Ok(image.sb.root_inode);
    }
    let mut inode_idx = image.sb.root_inode;
    for component in path.split('/').filter(|component| !component.is_empty()) {
        let inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        if !inode.is_dir() {
            return Err(FsError::NotDirectory);
        }
        let entries = read_dir_entries(image, inode_idx)?;
        let entry = entries
            .iter()
            .find(|entry| names_equal(&entry.name, component))
            .ok_or(FsError::NotFound)?;
        inode_idx = entry.inode;
    }
    Ok(inode_idx)
}

fn read_dir_entries(image: &Image, inode_idx: u32) -> Result<Vec<DirectoryEntry>, FsError> {
    let inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
    if !inode.is_dir() {
        return Err(FsError::NotDirectory);
    }
    let bytes = image.read_inode_bytes(&inode)?;
    let mut entries = Vec::new();
    for chunk in bytes.chunks(DIR_ENTRY_SIZE) {
        if chunk.len() < DIR_ENTRY_SIZE {
            break;
        }
        let inode = read_u32(chunk, 0).ok_or(FsError::Io)?;
        let name_len = chunk[4] as usize;
        if inode == 0 || name_len == 0 || name_len > MAX_NAME_LEN {
            continue;
        }
        let name = core::str::from_utf8(&chunk[5..5 + name_len]).map_err(|_| FsError::Io)?;
        entries.push(DirectoryEntry {
            inode,
            name: String::from(name),
        });
    }
    Ok(entries)
}

fn write_dir_entries(
    image: &mut Image,
    inode_idx: u32,
    entries: &[DirectoryEntry],
) -> Result<(), FsError> {
    let mut bytes = Vec::with_capacity(entries.len() * DIR_ENTRY_SIZE);
    for entry in entries {
        validate_name(&entry.name)?;
        let mut raw = [0u8; DIR_ENTRY_SIZE];
        write_u32(&mut raw, 0, entry.inode);
        raw[4] = entry.name.len() as u8;
        raw[5..5 + entry.name.len()].copy_from_slice(entry.name.as_bytes());
        bytes.extend_from_slice(&raw);
    }
    image.write_inode_bytes(inode_idx, &bytes)
}

fn trim_abs_path(path: &str) -> Result<&str, FsError> {
    if !path.starts_with('/') {
        return Err(FsError::InvalidPath);
    }
    let mut end = path.len();
    while end > 1 && path.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }
    Ok(&path[..end])
}

fn split_parent_and_name(path: &str) -> Result<(String, String), FsError> {
    let path = trim_abs_path(path)?;
    if path == "/" {
        return Err(FsError::InvalidPath);
    }
    let slash = path.rfind('/').ok_or(FsError::InvalidPath)?;
    let parent = if slash == 0 {
        String::from("/")
    } else {
        String::from(&path[..slash])
    };
    let name = String::from(&path[slash + 1..]);
    validate_name(&name)?;
    Ok((parent, name))
}

fn validate_name(name: &str) -> Result<(), FsError> {
    if name.is_empty() || name == "." || name == ".." || name.len() > MAX_NAME_LEN {
        return Err(FsError::UnsupportedName);
    }
    if !name
        .bytes()
        .all(|b| (0x20..=0x7e).contains(&b) && b != b'/')
    {
        return Err(FsError::UnsupportedName);
    }
    Ok(())
}

fn names_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn record_mutation(kind: &str, path: &str) {
    let vfs_path = mount_path_for(path);
    crate::fs_hardening::journal_operation(kind, &vfs_path);
    crate::search_index::record_change(&vfs_path);
    crate::writeback::enqueue(kind, &vfs_path);
}

fn mount_path_for(path: &str) -> String {
    let mut out = String::from(MOUNT_PATH);
    let normalized = crate::vfs::normalize_path(path);
    if normalized != "/" {
        out.push_str(&normalized);
    }
    out
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

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
