use core::fmt;

use super::{sys, Error, Result};

pub const STDIN: u64 = 0;
pub const STDOUT: u64 = 1;
pub const STDERR: u64 = 2;
pub const O_RDONLY: u64 = 0;
pub const O_WRONLY: u64 = 1;
pub const O_RDWR: u64 = 2;
pub const O_CREAT: u64 = 0x40;
pub const O_TRUNC: u64 = 0x200;

pub fn write(fd: u64, bytes: &[u8]) -> Result<usize> {
    if bytes.is_empty() {
        return Ok(0);
    }
    let ret =
        unsafe { sys::syscall3(sys::WRITE, fd, bytes.as_ptr() as u64, bytes.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn write_all(fd: u64, mut bytes: &[u8]) -> Result<()> {
    while !bytes.is_empty() {
        let n = write(fd, bytes)?;
        if n == 0 {
            return Err(Error::Failed);
        }
        bytes = &bytes[n.min(bytes.len())..];
    }
    Ok(())
}

#[inline]
pub fn write_stdout(bytes: &[u8]) {
    let _ = write_all(STDOUT, bytes);
}

#[inline]
pub fn write_stderr(bytes: &[u8]) {
    let _ = write_all(STDERR, bytes);
}

#[inline]
pub fn write_byte(fd: u64, byte: u8) -> Result<()> {
    write_all(fd, &[byte])
}

pub fn read(fd: u64, buf: &mut [u8]) -> Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let ret =
        unsafe { sys::syscall3(sys::READ, fd, buf.as_mut_ptr() as u64, buf.len() as u64) };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn open(path: &[u8]) -> Result<u64> {
    let ret = unsafe { sys::syscall2(sys::OPEN, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret)
}

pub fn create(path: &[u8]) -> Result<u64> {
    let ret =
        unsafe { sys::syscall2(sys::OPEN_WRITE, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret)
}

pub fn open_flags(path: &[u8], flags: u64) -> Result<u64> {
    let access = flags & 0x3;
    let allowed = O_CREAT | O_TRUNC | 0x3;
    if flags & !allowed != 0 || access == O_RDWR {
        return Err(Error::Invalid);
    }
    if access == O_WRONLY || flags & (O_CREAT | O_TRUNC) != 0 {
        if access != O_WRONLY || flags & O_CREAT == 0 {
            return Err(Error::Invalid);
        }
        return create(path);
    }
    open(path)
}

pub fn close(fd: u64) {
    unsafe {
        sys::syscall1(sys::CLOSE, fd);
    }
}

pub fn dup(fd: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall1(sys::DUP, fd) };
    Error::from_ret(ret)
}

pub fn pipe() -> Result<(u64, u64)> {
    let mut fds = [0u64; 2];
    let ret = unsafe { sys::syscall1(sys::PIPE, fds.as_mut_ptr() as u64) };
    Error::from_ret(ret).map(|_| (fds[0], fds[1]))
}

pub struct File {
    fd: u64,
}

impl File {
    pub fn open(path: &[u8]) -> Result<Self> {
        open(path).map(|fd| File { fd })
    }

    pub fn create(path: &[u8]) -> Result<Self> {
        create(path).map(|fd| File { fd })
    }

    pub fn open_flags(path: &[u8], flags: u64) -> Result<Self> {
        open_flags(path, flags).map(|fd| File { fd })
    }

    #[inline]
    pub const fn fd(&self) -> u64 {
        self.fd
    }

    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        read(self.fd, buf)
    }

    #[inline]
    pub fn write(&self, bytes: &[u8]) -> Result<usize> {
        write(self.fd, bytes)
    }

    #[inline]
    pub fn close(self) {
        let fd = self.fd;
        core::mem::forget(self);
        close(fd);
    }
}

impl Drop for File {
    fn drop(&mut self) {
        close(self.fd);
    }
}

pub struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_all(STDOUT, s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    let _ = fmt::write(&mut Stdout, args);
}

pub fn write_u64(mut n: u64) {
    if n == 0 {
        write_stdout(b"0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut len = 0usize;
    while n > 0 {
        buf[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        let _ = write_byte(STDOUT, buf[len]);
    }
}

pub fn write_ipv4(addr: u32) {
    write_u64(((addr >> 24) & 0xff) as u64);
    write_stdout(b".");
    write_u64(((addr >> 16) & 0xff) as u64);
    write_stdout(b".");
    write_u64(((addr >> 8) & 0xff) as u64);
    write_stdout(b".");
    write_u64((addr & 0xff) as u64);
}
