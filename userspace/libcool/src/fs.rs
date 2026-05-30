use super::{io, sys, Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Missing,
    File,
    Directory,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub kind: FileKind,
    pub size: u64,
    pub uid: u64,
    pub gid: u64,
    pub mode: u64,
}

pub fn read_file(path: &[u8], out: &mut [u8]) -> Result<usize> {
    let file = io::File::open(path)?;
    let n = file.read(out)?;
    file.close();
    Ok(n)
}

pub fn write_file(path: &[u8], data: &[u8]) -> Result<()> {
    let desc = [
        path.as_ptr() as u64,
        path.len() as u64,
        data.as_ptr() as u64,
        data.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::FS_WRITE_FILE, desc.as_ptr() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn create_dir(path: &[u8]) -> Result<()> {
    let ret =
        unsafe { sys::syscall2(sys::FS_CREATE_DIR, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn delete_tree(path: &[u8]) -> Result<()> {
    let ret =
        unsafe { sys::syscall2(sys::FS_DELETE_TREE, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn list_dir(path: &[u8], out: &mut [u8]) -> Result<usize> {
    if out.is_empty() {
        return Err(Error::Invalid);
    }
    let desc = [
        path.as_ptr() as u64,
        path.len() as u64,
        out.as_mut_ptr() as u64,
        out.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::FS_LIST_DIR, desc.as_ptr() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn stat(path: &[u8]) -> Result<Metadata> {
    let mut out = [0u8; 40];
    let desc = [
        path.as_ptr() as u64,
        path.len() as u64,
        out.as_mut_ptr() as u64,
        out.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::STAT, desc.as_ptr() as u64) };
    Error::from_ret(ret)?;
    let kind = match read_u64(&out, 0) {
        1 => FileKind::File,
        2 => FileKind::Directory,
        0 => FileKind::Missing,
        _ => FileKind::Other,
    };
    Ok(Metadata {
        kind,
        size: read_u64(&out, 8),
        uid: read_u64(&out, 16),
        gid: read_u64(&out, 24),
        mode: read_u64(&out, 32),
    })
}

pub fn rename(src: &[u8], dst: &[u8]) -> Result<()> {
    let desc = [
        src.as_ptr() as u64,
        src.len() as u64,
        dst.as_ptr() as u64,
        dst.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::RENAME, desc.as_ptr() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn chdir(path: &[u8]) -> Result<()> {
    let ret = unsafe { sys::syscall2(sys::CHDIR, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn getcwd(out: &mut [u8]) -> Result<usize> {
    if out.is_empty() {
        return Err(Error::Invalid);
    }
    let ret = unsafe { sys::syscall2(sys::GETCWD, out.as_mut_ptr() as u64, out.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn sync() -> Result<()> {
    let ret = unsafe { sys::syscall0(sys::SYNC) };
    Error::from_ret(ret).map(|_| ())
}

pub fn screenshot(path: &[u8]) -> Result<()> {
    let ret =
        unsafe { sys::syscall3(sys::SCREENSHOT, path.as_ptr() as u64, path.len() as u64, 0) };
    Error::from_ret(ret).map(|_| ())
}

fn read_u64(bytes: &[u8; 40], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}
