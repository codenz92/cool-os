use super::{sys, Error, Result};

pub fn shmem_create(len: usize) -> Result<u64> {
    let ret = unsafe { sys::syscall1(sys::SHMEM_CREATE, len as u64) };
    Error::from_ret(ret)
}

pub fn shmem_map(id: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall1(sys::SHMEM_MAP, id) };
    Error::from_ret(ret)
}
