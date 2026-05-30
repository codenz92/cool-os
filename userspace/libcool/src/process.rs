use super::{sys, Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    Int,
    User1,
    Term,
    Continue,
    Stop,
}

impl Signal {
    pub const fn code(self) -> u64 {
        match self {
            Signal::Int => 2,
            Signal::User1 => 10,
            Signal::Term => 15,
            Signal::Continue => 18,
            Signal::Stop => 19,
        }
    }
}

#[inline]
pub fn exit(code: u64) -> ! {
    unsafe {
        sys::syscall1(sys::EXIT, code);
    }
    abort()
}

#[inline]
pub fn abort() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[inline]
pub fn yield_now() {
    unsafe {
        sys::syscall0(sys::YIELD);
    }
}

#[inline]
pub fn getpid() -> u64 {
    unsafe { sys::syscall0(sys::GETPID) }
}

#[inline]
pub fn abi_version() -> u64 {
    unsafe { sys::syscall0(sys::ABI_VERSION) }
}

#[inline]
pub fn sleep_ms(ms: u64) {
    unsafe {
        sys::syscall1(sys::SLEEP_MS, ms);
    }
}

pub fn exec(path: &[u8]) -> Result<()> {
    let ret = unsafe { sys::syscall2(sys::EXEC, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret).map(|_| ())
}

pub fn spawn(path: &[u8]) -> Result<u64> {
    let ret = unsafe { sys::syscall2(sys::SPAWN, path.as_ptr() as u64, path.len() as u64) };
    Error::from_ret(ret)
}

pub fn spawn_args(path: &[u8], args: &[&[u8]]) -> Result<u64> {
    const MAX_ARGS: usize = 7;
    if args.len() > MAX_ARGS {
        return Err(Error::Invalid);
    }
    let mut pairs = [0u64; MAX_ARGS * 2];
    for (idx, arg) in args.iter().enumerate() {
        pairs[idx * 2] = arg.as_ptr() as u64;
        pairs[idx * 2 + 1] = arg.len() as u64;
    }
    let desc = [
        path.as_ptr() as u64,
        path.len() as u64,
        pairs.as_ptr() as u64,
        args.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::SPAWN_ARGS, desc.as_ptr() as u64) };
    Error::from_ret(ret)
}

pub fn spawn_fds_args(path: &[u8], args: &[&[u8]], fds: &[(u64, u64)]) -> Result<u64> {
    const MAX_ARGS: usize = 7;
    const MAX_FDS: usize = 4;
    if args.len() > MAX_ARGS || fds.len() > MAX_FDS {
        return Err(Error::Invalid);
    }
    let mut arg_pairs = [0u64; MAX_ARGS * 2];
    for (idx, arg) in args.iter().enumerate() {
        arg_pairs[idx * 2] = arg.as_ptr() as u64;
        arg_pairs[idx * 2 + 1] = arg.len() as u64;
    }
    let mut fd_pairs = [0u64; MAX_FDS * 2];
    for (idx, &(parent_fd, child_fd)) in fds.iter().enumerate() {
        fd_pairs[idx * 2] = parent_fd;
        fd_pairs[idx * 2 + 1] = child_fd;
    }
    let desc = [
        path.as_ptr() as u64,
        path.len() as u64,
        arg_pairs.as_ptr() as u64,
        args.len() as u64,
        fd_pairs.as_ptr() as u64,
        fds.len() as u64,
    ];
    let ret = unsafe { sys::syscall1(sys::SPAWN_FDS_ARGS, desc.as_ptr() as u64) };
    Error::from_ret(ret)
}

pub fn waitpid(pid: u64) -> Result<u64> {
    let mut status = 0u64;
    let ret = unsafe { sys::syscall2(sys::WAITPID, pid, &mut status as *mut u64 as u64) };
    Error::from_ret(ret).map(|_| status)
}

pub fn signal(pid: u64, signal: Signal) -> Result<()> {
    let ret = unsafe { sys::syscall2(sys::SIGNAL, pid, signal.code()) };
    Error::from_ret(ret).map(|_| ())
}

pub fn set_process_group(pid: u64, group: u64) -> Result<()> {
    let ret = unsafe { sys::syscall2(sys::SETPGID, pid, group) };
    Error::from_ret(ret).map(|_| ())
}

pub fn get_process_group(pid: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall1(sys::GETPGID, pid) };
    Error::from_ret(ret)
}

pub fn signal_group(group: u64, signal: Signal) -> Result<u64> {
    let ret = unsafe { sys::syscall2(sys::SIGNAL_GROUP, group, signal.code()) };
    Error::from_ret(ret)
}
