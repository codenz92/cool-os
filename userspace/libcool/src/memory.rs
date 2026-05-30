use super::{sys, Error, Result};

pub const PROT_WRITE: u64 = 1;
pub const PROT_EXEC: u64 = 2;

pub fn mmap(addr: u64, len: usize, writable: bool) -> Result<u64> {
    let flags = if writable { PROT_WRITE } else { 0 };
    mmap_flags(addr, len, flags)
}

pub fn mmap_flags(addr: u64, len: usize, flags: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall3(sys::MMAP, addr, len as u64, flags) };
    Error::from_ret(ret)
}

pub fn mprotect(addr: u64, len: usize, flags: u64) -> Result<()> {
    let ret = unsafe { sys::syscall3(sys::MPROTECT, addr, len as u64, flags) };
    Error::from_ret(ret).map(|_| ())
}

pub fn mmap_file(fd: u64, addr: u64, len: usize, file_offset: u64, flags: u64) -> Result<u64> {
    let desc = [fd, addr, len as u64, file_offset, flags];
    let ret = unsafe { sys::syscall1(sys::MMAP_FILE, desc.as_ptr() as u64) };
    Error::from_ret(ret)
}
