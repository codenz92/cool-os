/// CoolFS: the native coolOS root filesystem.
///
/// CoolFS owns the resolved root region of the attached OS disk. Legacy live
/// images keep that region at LBA 0; installed disks can expose it through an
/// MBR partition. The legacy FAT32 area, when present, is an
/// import/compatibility mount at `/FAT`; it is no longer required to mount `/`.
extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

use crate::fat32::{DirEntryInfo, FsError};

pub const MOUNT_PATH: &str = "/";

const MAGIC: [u8; 8] = *b"COOLFS1\0";
const VERSION: u32 = 1;
const SECTOR_SIZE: usize = 512;
const SECTORS_PER_BLOCK: u32 = 8;
const BLOCK_SIZE: usize = 4096;
const TOTAL_BLOCKS: u32 = 1536;
const INODE_COUNT: u32 = 512;
const INODE_SIZE: usize = 256;
const DIRECT_BLOCKS: usize = 48;
const INDIRECT_ENTRIES: usize = BLOCK_SIZE / 4;
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
const CACHE_SLOTS: usize = 64;
const INODE_UID_OFFSET: usize = 12 + DIRECT_BLOCKS * 4;
const INODE_GID_OFFSET: usize = INODE_UID_OFFSET + 4;
const INODE_MODE_OFFSET: usize = INODE_GID_OFFSET + 4;
const INODE_USED_BYTES: usize = INODE_MODE_OFFSET + 4;

static COOLFS_IMAGE: Mutex<Option<Image>> = Mutex::new(None);
static DIRTY: AtomicBool = AtomicBool::new(false);

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
    indirect: u32,
    uid: u32,
    gid: u32,
    mode: u16,
}

impl Inode {
    const fn free() -> Self {
        Self {
            kind: KIND_FREE,
            size: 0,
            direct: [0; DIRECT_BLOCKS],
            indirect: 0,
            uid: 0,
            gid: 0,
            mode: 0,
        }
    }

    fn new(kind: u8) -> Self {
        let mode = match kind {
            KIND_DIR => crate::security::DEFAULT_DIR_MODE,
            KIND_FILE => crate::security::DEFAULT_FILE_MODE,
            _ => 0,
        };
        Self {
            kind,
            size: 0,
            direct: [0; DIRECT_BLOCKS],
            indirect: 0,
            uid: crate::security::ROOT_UID,
            gid: crate::security::ROOT_GID,
            mode,
        }
    }

    fn new_with_metadata(kind: u8, uid: u32, gid: u32, mode: u16) -> Self {
        Self {
            kind,
            size: 0,
            direct: [0; DIRECT_BLOCKS],
            indirect: 0,
            uid,
            gid,
            mode: mode & 0o777,
        }
    }

    fn is_dir(&self) -> bool {
        self.kind == KIND_DIR
    }

    fn is_file(&self) -> bool {
        self.kind == KIND_FILE
    }
}

#[derive(Clone, Copy)]
pub struct Metadata {
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    pub size: u32,
    pub is_dir: bool,
    pub is_file: bool,
}

struct DirectoryEntry {
    inode: u32,
    name: String,
}

struct CacheEntry {
    block: u32,
    data: [u8; BLOCK_SIZE],
    dirty: bool,
    age: u64,
}

struct BlockCache {
    entries: Vec<CacheEntry>,
    clock: u64,
}

struct Image {
    sb: Superblock,
    cache: BlockCache,
}

#[derive(Clone, Copy)]
pub struct CoolFsStats {
    pub total_blocks: u32,
    pub used_blocks: u32,
    pub free_blocks: u32,
    pub block_size: u32,
    pub files: u32,
    pub dirs: u32,
    pub cached_blocks: u32,
    pub dirty_blocks: u32,
}

#[derive(Clone, Copy)]
pub struct CoolFsCheckReport {
    pub ok: bool,
    pub root_entries: usize,
    pub stats: CoolFsStats,
}

fn with_image_result<R>(f: impl FnOnce(&mut Image) -> Result<R, FsError>) -> Result<R, FsError> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut slot = COOLFS_IMAGE.lock();
        let image = ensure_image(&mut slot)?;
        f(image)
    })
}

fn with_image_option<R>(f: impl FnOnce(&mut Image) -> Option<R>) -> Option<R> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut slot = COOLFS_IMAGE.lock();
        let image = ensure_image(&mut slot).ok()?;
        f(image)
    })
}

pub fn mount_or_format() -> Result<(), FsError> {
    with_image_result(|_| Ok(()))
}

pub fn flush() -> Result<(), FsError> {
    if !DIRTY.load(Ordering::Acquire) {
        return Ok(());
    }
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut slot = COOLFS_IMAGE.lock();
        let Some(image) = slot.as_mut() else {
            DIRTY.store(false, Ordering::Release);
            return Ok(());
        };
        image.flush()?;
        DIRTY.store(false, Ordering::Release);
        Ok(())
    })
}

pub fn trim_clean_cache(keep_blocks: usize) -> Result<usize, FsError> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut slot = COOLFS_IMAGE.lock();
        let Some(image) = slot.as_mut() else {
            return Ok(0);
        };
        image.cache.trim_clean(keep_blocks)
    })
}

pub fn lines() -> Vec<String> {
    match stats() {
        Some(stats) => alloc::vec![
            format!(
                "mount {} type=coolfs flags=rw,native-root,bitmap-inodes,indirect-blocks,uid-gid-mode",
                MOUNT_PATH
            ),
            format!(
                "device={} blocks used={} free={} bytes/block={}",
                device_label(),
                stats.used_blocks,
                stats.free_blocks,
                stats.block_size
            ),
            format!(
                "cache slots={} cached={} dirty={}",
                CACHE_SLOTS, stats.cached_blocks, stats.dirty_blocks
            ),
            format!("inodes files={} dirs={}", stats.files, stats.dirs),
        ],
        None => alloc::vec![String::from("coolfs unavailable")],
    }
}

pub fn stats() -> Option<CoolFsStats> {
    with_image_option(|image| Some(image.stats()))
}

pub fn check() -> Option<CoolFsCheckReport> {
    with_image_option(|image| {
        let root_entries = read_dir_entries(image, image.sb.root_inode).ok()?.len();
        let root = image.read_inode(image.sb.root_inode)?;
        let stats = image.stats();
        Some(CoolFsCheckReport {
            ok: root.is_dir() && stats.used_blocks <= stats.total_blocks,
            root_entries,
            stats,
        })
    })
}

pub fn list_dir(path: &str) -> Option<Vec<DirEntryInfo>> {
    with_image_option(|image| {
        let inode_idx = resolve_path(image, path).ok()?;
        let inode = image.read_inode(inode_idx)?;
        if !inode.is_dir() {
            return None;
        }
        let entries = read_dir_entries(image, inode_idx).ok()?;
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
    })
}

pub fn read_file(path: &str) -> Option<Vec<u8>> {
    with_image_option(|image| {
        let inode_idx = resolve_path(image, path).ok()?;
        let inode = image.read_inode(inode_idx)?;
        if !inode.is_file() {
            return None;
        }
        image.read_inode_bytes(&inode).ok()
    })
}

pub fn metadata(path: &str) -> Option<Metadata> {
    with_image_option(|image| {
        let inode_idx = resolve_path(image, path).ok()?;
        let inode = image.read_inode(inode_idx)?;
        Some(Metadata {
            uid: inode.uid,
            gid: inode.gid,
            mode: inode.mode,
            size: inode.size,
            is_dir: inode.is_dir(),
            is_file: inode.is_file(),
        })
    })
}

pub fn chmod(path: &str, mode: u16) -> Result<(), FsError> {
    with_image_result(|image| {
        let inode_idx = resolve_path(image, path)?;
        let mut inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        if inode.kind == KIND_FREE {
            return Err(FsError::NotFound);
        }
        inode.mode = mode & 0o777;
        image.write_inode(inode_idx, &inode)
    })?;
    record_mutation("coolfs-chmod", path);
    Ok(())
}

pub fn chown(path: &str, uid: u32, gid: u32) -> Result<(), FsError> {
    with_image_result(|image| {
        let inode_idx = resolve_path(image, path)?;
        let mut inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        if inode.kind == KIND_FREE {
            return Err(FsError::NotFound);
        }
        inode.uid = uid;
        inode.gid = gid;
        image.write_inode(inode_idx, &inode)
    })?;
    record_mutation("coolfs-chown", path);
    Ok(())
}

pub fn create_file(path: &str) -> Result<(), FsError> {
    create_file_with_metadata(
        path,
        crate::security::current_credentials().uid,
        crate::security::current_credentials().gid,
        crate::security::DEFAULT_FILE_MODE,
    )
}

pub fn create_file_with_metadata(path: &str, uid: u32, gid: u32, mode: u16) -> Result<(), FsError> {
    create_node(path, KIND_FILE, uid, gid, mode)
}

#[allow(dead_code)]
pub fn create_dir(path: &str) -> Result<(), FsError> {
    create_dir_with_metadata(
        path,
        crate::security::current_credentials().uid,
        crate::security::current_credentials().gid,
        crate::security::DEFAULT_DIR_MODE,
    )
}

pub fn create_dir_with_metadata(path: &str, uid: u32, gid: u32, mode: u16) -> Result<(), FsError> {
    create_node(path, KIND_DIR, uid, gid, mode)
}

pub fn write_file(path: &str, data: &[u8]) -> Result<(), FsError> {
    with_image_result(|image| {
        let inode_idx = resolve_path(image, path)?;
        let inode = image.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        if !inode.is_file() {
            return Err(FsError::InvalidPath);
        }
        image.write_inode_bytes(inode_idx, data)
    })?;
    record_mutation("coolfs-write", path);
    Ok(())
}

#[allow(dead_code)]
pub fn safe_write_file(path: &str, data: &[u8]) -> Result<(), FsError> {
    let creds = crate::security::current_credentials();
    safe_write_file_with_metadata(
        path,
        data,
        creds.uid,
        creds.gid,
        crate::security::DEFAULT_FILE_MODE,
    )
}

pub fn safe_write_file_with_metadata(
    path: &str,
    data: &[u8],
    uid: u32,
    gid: u32,
    mode: u16,
) -> Result<(), FsError> {
    let (parent, name) = split_parent_and_name(path)?;
    let mut tmp = parent.clone();
    if !tmp.ends_with('/') {
        tmp.push('/');
    }
    tmp.push_str("CWTMP.TMP");

    match delete_file(&tmp) {
        Ok(()) | Err(FsError::NotFound) => {}
        Err(err) => return Err(err),
    }
    create_file_with_metadata(&tmp, uid, gid, mode)?;
    write_file(&tmp, data)?;
    match delete_file(path) {
        Ok(()) | Err(FsError::NotFound) => {}
        Err(err) => {
            let _ = delete_file(&tmp);
            return Err(err);
        }
    }
    rename(&tmp, &name)
}

pub fn delete_file(path: &str) -> Result<(), FsError> {
    with_image_result(|image| {
        let (parent_path, name) = split_parent_and_name(path)?;
        let parent_inode = resolve_path(image, &parent_path)?;
        let mut entries = read_dir_entries(image, parent_inode)?;
        let pos = entries
            .iter()
            .position(|entry| names_equal(&entry.name, &name))
            .ok_or(FsError::NotFound)?;
        let target_inode = entries[pos].inode;
        let target = image.read_inode(target_inode).ok_or(FsError::NotFound)?;
        if target.is_dir() && !read_dir_entries(image, target_inode)?.is_empty() {
            return Err(FsError::NotEmpty);
        }

        entries.remove(pos);
        write_dir_entries(image, parent_inode, &entries)?;
        image.free_inode(target_inode)
    })?;
    record_mutation("coolfs-delete", path);
    Ok(())
}

pub fn rename(path: &str, new_name: &str) -> Result<(), FsError> {
    validate_name(new_name)?;
    let mutated = with_image_result(|image| {
        let (parent_path, old_name) = split_parent_and_name(path)?;
        if names_equal(&old_name, new_name) {
            return Ok(false);
        }
        let parent_inode = resolve_path(image, &parent_path)?;
        let mut entries = read_dir_entries(image, parent_inode)?;
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
        write_dir_entries(image, parent_inode, &entries)?;
        Ok(true)
    })?;
    if mutated {
        record_mutation("coolfs-rename", path);
    }
    Ok(())
}

pub fn copy_file(src: &str, dst: &str) -> Result<(), FsError> {
    let data = read_file(src).ok_or(FsError::NotFound)?;
    create_file(dst)?;
    write_file(dst, &data)
}

fn create_node(path: &str, kind: u8, uid: u32, gid: u32, mode: u16) -> Result<(), FsError> {
    with_image_result(|image| {
        let (parent_path, name) = split_parent_and_name(path)?;
        validate_name(&name)?;
        let parent_inode = resolve_path(image, &parent_path)?;
        let mut entries = read_dir_entries(image, parent_inode)?;
        if entries.iter().any(|entry| names_equal(&entry.name, &name)) {
            return Err(FsError::AlreadyExists);
        }

        let inode_idx = image.alloc_inode_with_metadata(kind, uid, gid, mode)?;
        entries.push(DirectoryEntry {
            inode: inode_idx,
            name,
        });
        if let Err(err) = write_dir_entries(image, parent_inode, &entries) {
            let _ = image.free_inode(inode_idx);
            return Err(err);
        }

        Ok(())
    })?;
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

impl BlockCache {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            clock: 0,
        }
    }

    fn with_clean_block(block: u32, data: [u8; BLOCK_SIZE]) -> Self {
        let mut cache = Self::new();
        cache.entries.push(CacheEntry {
            block,
            data,
            dirty: false,
            age: 1,
        });
        cache.clock = 1;
        cache
    }

    fn cached_blocks(&self) -> u32 {
        self.entries.len() as u32
    }

    fn dirty_blocks(&self) -> u32 {
        self.entries.iter().filter(|entry| entry.dirty).count() as u32
    }

    fn get(&mut self, block: u32) -> Result<&[u8; BLOCK_SIZE], FsError> {
        let idx = self.cache_index(block)?;
        Ok(&self.entries[idx].data)
    }

    fn get_mut(&mut self, block: u32) -> Result<&mut [u8; BLOCK_SIZE], FsError> {
        let idx = self.cache_index(block)?;
        self.entries[idx].dirty = true;
        Ok(&mut self.entries[idx].data)
    }

    fn cache_index(&mut self, block: u32) -> Result<usize, FsError> {
        if block >= TOTAL_BLOCKS {
            return Err(FsError::Io);
        }
        self.clock = self.clock.wrapping_add(1).max(1);
        if let Some(idx) = self.entries.iter().position(|entry| entry.block == block) {
            self.entries[idx].age = self.clock;
            return Ok(idx);
        }

        let data = read_disk_block(block)?;
        if self.entries.len() < CACHE_SLOTS {
            self.entries.push(CacheEntry {
                block,
                data,
                dirty: false,
                age: self.clock,
            });
            return Ok(self.entries.len() - 1);
        }

        let evict_idx = self
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(_, entry)| entry.age)
            .map(|(idx, _)| idx)
            .ok_or(FsError::Io)?;
        if self.entries[evict_idx].dirty {
            write_disk_block(self.entries[evict_idx].block, &self.entries[evict_idx].data)?;
        }
        self.entries[evict_idx] = CacheEntry {
            block,
            data,
            dirty: false,
            age: self.clock,
        };
        Ok(evict_idx)
    }

    fn flush(&mut self) -> Result<(), FsError> {
        for entry in self.entries.iter_mut() {
            if entry.dirty {
                write_disk_block(entry.block, &entry.data)?;
                entry.dirty = false;
            }
        }
        Ok(())
    }

    fn trim_clean(&mut self, keep_blocks: usize) -> Result<usize, FsError> {
        let mut removed = 0usize;
        while self.entries.len() > keep_blocks {
            let Some(evict_idx) = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| !entry.dirty && entry.block != 0)
                .min_by_key(|(_, entry)| entry.age)
                .map(|(idx, _)| idx)
            else {
                break;
            };
            self.entries.remove(evict_idx);
            removed = removed.saturating_add(1);
        }
        if removed > 0 {
            self.entries.shrink_to_fit();
        }
        Ok(removed.saturating_mul(BLOCK_SIZE))
    }
}

impl Image {
    fn read_inode(&mut self, inode_idx: u32) -> Option<Inode> {
        if inode_idx >= self.sb.inode_count {
            return None;
        }
        let off = self.inode_offset(inode_idx)?;
        let kind = self.read_u8(off)?;
        let size = self.read_u32_at(off + 4)?;
        let mut inode = Inode::new(kind);
        inode.size = size;
        for i in 0..DIRECT_BLOCKS {
            inode.direct[i] = self.read_u32_at(off + 8 + i * 4)?;
        }
        inode.indirect = self.read_u32_at(off + 8 + DIRECT_BLOCKS * 4)?;
        inode.uid = self.read_u32_at(off + INODE_UID_OFFSET).unwrap_or(0);
        inode.gid = self.read_u32_at(off + INODE_GID_OFFSET).unwrap_or(0);
        inode.mode = self
            .read_u32_at(off + INODE_MODE_OFFSET)
            .map(|mode| (mode as u16) & 0o777)
            .unwrap_or(0);
        if inode.kind != KIND_FREE && inode.mode == 0 {
            inode.mode = if inode.kind == KIND_DIR {
                crate::security::DEFAULT_DIR_MODE
            } else {
                crate::security::DEFAULT_FILE_MODE
            };
        }
        Some(inode)
    }

    fn write_inode(&mut self, inode_idx: u32, inode: &Inode) -> Result<(), FsError> {
        let off = self.inode_offset(inode_idx).ok_or(FsError::Io)?;
        self.write_u8(off, inode.kind)?;
        self.fill_at(off + 1, 3, 0)?;
        self.write_u32_at(off + 4, inode.size)?;
        for i in 0..DIRECT_BLOCKS {
            self.write_u32_at(off + 8 + i * 4, inode.direct[i])?;
        }
        self.write_u32_at(off + 8 + DIRECT_BLOCKS * 4, inode.indirect)?;
        self.write_u32_at(off + INODE_UID_OFFSET, inode.uid)?;
        self.write_u32_at(off + INODE_GID_OFFSET, inode.gid)?;
        self.write_u32_at(off + INODE_MODE_OFFSET, (inode.mode & 0o777) as u32)?;
        self.fill_at(off + INODE_USED_BYTES, INODE_SIZE - INODE_USED_BYTES, 0)?;
        Ok(())
    }

    fn inode_offset(&self, inode_idx: u32) -> Option<usize> {
        let base = self.sb.inode_table_start as usize * BLOCK_SIZE;
        let off = base.checked_add(inode_idx as usize * INODE_SIZE)?;
        if off + INODE_SIZE <= self.fs_len() {
            Some(off)
        } else {
            None
        }
    }

    fn fs_len(&self) -> usize {
        self.sb.total_blocks as usize * BLOCK_SIZE
    }

    fn block_offset(&self, block: u32) -> Result<usize, FsError> {
        if block >= self.sb.total_blocks {
            return Err(FsError::Io);
        }
        Ok(block as usize * BLOCK_SIZE)
    }

    fn abs_parts(&self, offset: usize) -> Option<(u32, usize)> {
        if offset >= self.fs_len() {
            return None;
        }
        Some(((offset / BLOCK_SIZE) as u32, offset % BLOCK_SIZE))
    }

    fn read_u8(&mut self, offset: usize) -> Option<u8> {
        let (block, in_block) = self.abs_parts(offset)?;
        Some(self.cache.get(block).ok()?[in_block])
    }

    fn write_u8(&mut self, offset: usize, value: u8) -> Result<(), FsError> {
        let (block, in_block) = self.abs_parts(offset).ok_or(FsError::Io)?;
        self.cache.get_mut(block)?[in_block] = value;
        Ok(())
    }

    fn read_u32_at(&mut self, offset: usize) -> Option<u32> {
        let mut bytes = [0u8; 4];
        self.read_into(offset, &mut bytes).ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    fn write_u32_at(&mut self, offset: usize, value: u32) -> Result<(), FsError> {
        self.write_at(offset, &value.to_le_bytes())
    }

    fn read_into(&mut self, mut offset: usize, out: &mut [u8]) -> Result<(), FsError> {
        if offset.checked_add(out.len()).ok_or(FsError::Io)? > self.fs_len() {
            return Err(FsError::Io);
        }
        let mut done = 0usize;
        while done < out.len() {
            let (block, in_block) = self.abs_parts(offset).ok_or(FsError::Io)?;
            let take = (out.len() - done).min(BLOCK_SIZE - in_block);
            let data = self.cache.get(block)?;
            out[done..done + take].copy_from_slice(&data[in_block..in_block + take]);
            done += take;
            offset += take;
        }
        Ok(())
    }

    fn read_vec(&mut self, mut offset: usize, len: usize) -> Result<Vec<u8>, FsError> {
        if offset.checked_add(len).ok_or(FsError::Io)? > self.fs_len() {
            return Err(FsError::Io);
        }
        let mut out = Vec::with_capacity(len);
        let mut remaining = len;
        while remaining > 0 {
            let (block, in_block) = self.abs_parts(offset).ok_or(FsError::Io)?;
            let take = remaining.min(BLOCK_SIZE - in_block);
            let data = self.cache.get(block)?;
            out.extend_from_slice(&data[in_block..in_block + take]);
            offset += take;
            remaining -= take;
        }
        Ok(out)
    }

    fn write_at(&mut self, mut offset: usize, input: &[u8]) -> Result<(), FsError> {
        if offset.checked_add(input.len()).ok_or(FsError::Io)? > self.fs_len() {
            return Err(FsError::Io);
        }
        let mut done = 0usize;
        while done < input.len() {
            let (block, in_block) = self.abs_parts(offset).ok_or(FsError::Io)?;
            let take = (input.len() - done).min(BLOCK_SIZE - in_block);
            let data = self.cache.get_mut(block)?;
            data[in_block..in_block + take].copy_from_slice(&input[done..done + take]);
            done += take;
            offset += take;
        }
        Ok(())
    }

    fn fill_at(&mut self, mut offset: usize, mut len: usize, value: u8) -> Result<(), FsError> {
        if offset.checked_add(len).ok_or(FsError::Io)? > self.fs_len() {
            return Err(FsError::Io);
        }
        while len > 0 {
            let (block, in_block) = self.abs_parts(offset).ok_or(FsError::Io)?;
            let take = len.min(BLOCK_SIZE - in_block);
            self.cache.get_mut(block)?[in_block..in_block + take].fill(value);
            offset += take;
            len -= take;
        }
        Ok(())
    }

    fn zero_block(&mut self, block: u32) -> Result<(), FsError> {
        self.cache.get_mut(block)?.fill(0);
        Ok(())
    }

    fn block_used(&mut self, block: u32) -> bool {
        self.bitmap_byte_bit(block)
            .and_then(|(byte, bit)| self.read_u8(byte).map(|value| value & bit != 0))
            .unwrap_or(true)
    }

    fn bitmap_byte_bit(&self, block: u32) -> Option<(usize, u8)> {
        if block >= self.sb.total_blocks {
            return None;
        }
        let bitmap_start = self.sb.bitmap_start as usize * BLOCK_SIZE;
        let byte = bitmap_start + block as usize / 8;
        if byte >= self.fs_len() {
            return None;
        }
        Some((byte, 1u8 << (block % 8)))
    }

    fn set_block_used(&mut self, block: u32, used: bool) -> Result<(), FsError> {
        let (byte, bit) = self.bitmap_byte_bit(block).ok_or(FsError::Io)?;
        let mut value = self.read_u8(byte).ok_or(FsError::Io)?;
        if used {
            value |= bit;
        } else {
            value &= !bit;
        }
        self.write_u8(byte, value)
    }

    fn alloc_block(&mut self) -> Result<u32, FsError> {
        for block in self.sb.data_start..self.sb.total_blocks {
            if !self.block_used(block) {
                self.set_block_used(block, true)?;
                self.zero_block(block)?;
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

    #[allow(dead_code)]
    fn alloc_inode(&mut self, kind: u8) -> Result<u32, FsError> {
        self.alloc_inode_with_metadata(
            kind,
            crate::security::ROOT_UID,
            crate::security::ROOT_GID,
            if kind == KIND_DIR {
                crate::security::DEFAULT_DIR_MODE
            } else {
                crate::security::DEFAULT_FILE_MODE
            },
        )
    }

    fn alloc_inode_with_metadata(
        &mut self,
        kind: u8,
        uid: u32,
        gid: u32,
        mode: u16,
    ) -> Result<u32, FsError> {
        for inode_idx in 1..self.sb.inode_count {
            if self
                .read_inode(inode_idx)
                .map(|inode| inode.kind == KIND_FREE)
                .unwrap_or(false)
            {
                self.write_inode(inode_idx, &Inode::new_with_metadata(kind, uid, gid, mode))?;
                return Ok(inode_idx);
            }
        }
        Err(FsError::NoSpace)
    }

    fn free_inode(&mut self, inode_idx: u32) -> Result<(), FsError> {
        let inode = self.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        for block in self.inode_data_blocks(&inode)? {
            self.free_block(block)?;
        }
        if inode.indirect != 0 {
            self.free_block(inode.indirect)?;
        }
        self.write_inode(inode_idx, &Inode::free())
    }

    fn read_inode_bytes(&mut self, inode: &Inode) -> Result<Vec<u8>, FsError> {
        let mut out = Vec::with_capacity(inode.size as usize);
        let mut remaining = inode.size as usize;
        for block in self.inode_data_blocks(inode)? {
            if remaining == 0 {
                break;
            }
            let start = self.block_offset(block)?;
            let take = remaining.min(BLOCK_SIZE);
            out.extend_from_slice(&self.read_vec(start, take)?);
            remaining -= take;
        }
        if remaining == 0 {
            Ok(out)
        } else {
            Err(FsError::Io)
        }
    }

    fn write_inode_bytes(&mut self, inode_idx: u32, bytes: &[u8]) -> Result<(), FsError> {
        let max_blocks = DIRECT_BLOCKS + INDIRECT_ENTRIES;
        if bytes.len() > max_blocks * BLOCK_SIZE || bytes.len() > u32::MAX as usize {
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
        let mut indirect_block = 0u32;
        if needed > DIRECT_BLOCKS {
            match self.alloc_block() {
                Ok(block) => indirect_block = block,
                Err(err) => {
                    for block in new_blocks {
                        let _ = self.free_block(block);
                    }
                    return Err(err);
                }
            }
        }

        for (idx, &block) in new_blocks.iter().enumerate() {
            self.zero_block(block)?;
            let start = idx * BLOCK_SIZE;
            let end = (start + BLOCK_SIZE).min(bytes.len());
            self.write_at(self.block_offset(block)?, &bytes[start..end])?;
        }
        if indirect_block != 0 {
            self.zero_block(indirect_block)?;
            let indirect_start = self.block_offset(indirect_block)?;
            for (idx, &block) in new_blocks[DIRECT_BLOCKS..].iter().enumerate() {
                self.write_u32_at(indirect_start + idx * 4, block)?;
            }
        }

        let mut inode = self.read_inode(inode_idx).ok_or(FsError::NotFound)?;
        let old_blocks = self.inode_data_blocks(&inode)?;
        let old_indirect = inode.indirect;
        inode.size = bytes.len() as u32;
        inode.direct = [0; DIRECT_BLOCKS];
        inode.indirect = indirect_block;
        for (idx, block) in new_blocks.iter().copied().enumerate() {
            if idx < DIRECT_BLOCKS {
                inode.direct[idx] = block;
            }
        }
        self.write_inode(inode_idx, &inode)?;
        for block in old_blocks {
            self.free_block(block)?;
        }
        if old_indirect != 0 {
            self.free_block(old_indirect)?;
        }
        Ok(())
    }

    fn inode_data_blocks(&mut self, inode: &Inode) -> Result<Vec<u32>, FsError> {
        let needed = (inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut blocks = Vec::with_capacity(needed);
        for &block in inode.direct.iter().take(needed.min(DIRECT_BLOCKS)) {
            if block == 0 {
                return Err(FsError::Io);
            }
            blocks.push(block);
        }
        if needed > DIRECT_BLOCKS {
            if inode.indirect == 0 {
                return Err(FsError::Io);
            }
            let start = self.block_offset(inode.indirect)?;
            for idx in 0..needed - DIRECT_BLOCKS {
                let block = self.read_u32_at(start + idx * 4).ok_or(FsError::Io)?;
                if block == 0 {
                    return Err(FsError::Io);
                }
                blocks.push(block);
            }
        }
        Ok(blocks)
    }

    fn flush(&mut self) -> Result<(), FsError> {
        self.cache.flush()
    }

    fn stats(&mut self) -> CoolFsStats {
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
            cached_blocks: self.cache.cached_blocks(),
            dirty_blocks: self.cache.dirty_blocks(),
        }
    }
}

fn ensure_image(slot: &mut Option<Image>) -> Result<&mut Image, FsError> {
    if slot.is_none() {
        *slot = Some(load_or_format()?);
    }
    slot.as_mut().ok_or(FsError::Io)
}

fn load_or_format() -> Result<Image, FsError> {
    let first = read_disk_block(0)?;
    if let Some(sb) = Superblock::parse(&first) {
        if (1024..=TOTAL_BLOCKS).contains(&sb.total_blocks) {
            return Ok(Image {
                sb,
                cache: BlockCache::with_clean_block(0, first),
            });
        }
    }

    let mut image = format_image()?;
    image.flush()?;
    Ok(image)
}

fn format_image() -> Result<Image, FsError> {
    let sb = Superblock::new();
    let mut image = Image {
        sb,
        cache: BlockCache::new(),
    };
    for block in 0..sb.data_start {
        image.zero_block(block)?;
    }
    {
        let block0 = image.cache.get_mut(0)?;
        block0.fill(0);
        sb.write(block0);
    }
    for block in 0..sb.data_start {
        image.set_block_used(block, true)?;
    }
    image.write_inode(sb.root_inode, &Inode::new(KIND_DIR))?;
    Ok(image)
}

fn resolve_path(image: &mut Image, path: &str) -> Result<u32, FsError> {
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

fn read_dir_entries(image: &mut Image, inode_idx: u32) -> Result<Vec<DirectoryEntry>, FsError> {
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
    DIRTY.store(true, Ordering::Release);
    let vfs_path = mount_path_for(path);
    crate::fs_hardening::journal_operation(kind, &vfs_path);
    crate::search_index::record_change(&vfs_path);
    crate::writeback::enqueue(kind, &vfs_path);
}

fn mount_path_for(path: &str) -> String {
    let normalized = crate::vfs::normalize_path(path);
    if MOUNT_PATH == "/" {
        normalized
    } else {
        let mut out = String::from(MOUNT_PATH);
        if normalized != "/" {
            out.push_str(&normalized);
        }
        out
    }
}

fn read_disk_block(block: u32) -> Result<[u8; BLOCK_SIZE], FsError> {
    if block >= TOTAL_BLOCKS {
        return Err(FsError::Io);
    }
    let mut out = [0u8; BLOCK_SIZE];
    let base_lba = block.checked_mul(SECTORS_PER_BLOCK).ok_or(FsError::Io)?;
    for sector in 0..SECTORS_PER_BLOCK {
        let mut sec = [0u8; SECTOR_SIZE];
        if !crate::storage::read_sector(base_lba + sector, &mut sec) {
            return Err(FsError::Io);
        }
        let start = sector as usize * SECTOR_SIZE;
        out[start..start + SECTOR_SIZE].copy_from_slice(&sec);
    }
    Ok(out)
}

fn write_disk_block(block: u32, data: &[u8; BLOCK_SIZE]) -> Result<(), FsError> {
    if block >= TOTAL_BLOCKS {
        return Err(FsError::Io);
    }
    let base_lba = block.checked_mul(SECTORS_PER_BLOCK).ok_or(FsError::Io)?;
    for sector in 0..SECTORS_PER_BLOCK {
        let mut sec = [0u8; SECTOR_SIZE];
        let start = sector as usize * SECTOR_SIZE;
        sec.copy_from_slice(&data[start..start + SECTOR_SIZE]);
        if !crate::storage::write_sector(base_lba + sector, &sec) {
            return Err(FsError::Io);
        }
    }
    Ok(())
}

fn device_label() -> String {
    if let Some(root) = crate::storage::root_disk() {
        return format!(
            "block:{}:lba{}{}",
            root.device.name(),
            root.base_lba,
            root.layout.suffix()
        );
    }
    String::from("ata:unresolved")
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
