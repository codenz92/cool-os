use super::{sys, Error, Result};

pub const SOURCE_FD: u64 = 1;
pub const SOURCE_SOCKET: u64 = 2;
pub const SOURCE_GUI: u64 = 3;
pub const SOURCE_CHILD: u64 = 4;
pub const SOURCE_TTY: u64 = 5;

pub const READ: u64 = 1 << 0;
pub const WRITE: u64 = 1 << 1;
pub const HANGUP: u64 = 1 << 2;
pub const ERROR: u64 = 1 << 3;
pub const CHILD: u64 = 1 << 4;

pub const TIMEOUT_FOREVER: u64 = u64::MAX;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollDesc {
    pub source: u64,
    pub handle: u64,
    pub events: u64,
    pub revents: u64,
}

impl PollDesc {
    pub const fn new(source: u64, handle: u64, events: u64) -> Self {
        Self {
            source,
            handle,
            events,
            revents: 0,
        }
    }

    pub const fn fd(fd: u64, events: u64) -> Self {
        Self::new(SOURCE_FD, fd, events)
    }

    pub const fn fd_read(fd: u64) -> Self {
        Self::fd(fd, READ)
    }

    pub const fn fd_write(fd: u64) -> Self {
        Self::fd(fd, WRITE)
    }

    pub const fn socket(socket: u64, events: u64) -> Self {
        Self::new(SOURCE_SOCKET, socket, events)
    }

    pub const fn socket_read(socket: u64) -> Self {
        Self::socket(socket, READ)
    }

    pub const fn gui(handle: u64) -> Self {
        Self::new(SOURCE_GUI, handle, READ)
    }

    pub const fn child(pid: u64) -> Self {
        Self::new(SOURCE_CHILD, pid, CHILD)
    }

    pub const fn tty(handle: u64) -> Self {
        Self::new(SOURCE_TTY, handle, READ)
    }

    pub const fn is_ready(self, mask: u64) -> bool {
        self.revents & mask != 0
    }
}

pub fn poll(descs: &mut [PollDesc], timeout_ms: u64) -> Result<usize> {
    if descs.is_empty() {
        let ret = unsafe { sys::syscall3(sys::POLL, 0, 0, timeout_ms) };
        return Error::from_ret(ret).map(|n| n as usize);
    }
    let ret = unsafe {
        sys::syscall3(
            sys::POLL,
            descs.as_mut_ptr() as u64,
            descs.len() as u64,
            timeout_ms,
        )
    };
    Error::from_ret(ret).map(|n| n as usize)
}

pub fn wait_fd(fd: u64, events: u64, timeout_ms: u64) -> Result<bool> {
    let mut desc = PollDesc::fd(fd, events);
    poll(core::slice::from_mut(&mut desc), timeout_ms).map(|n| n > 0 && desc.revents != 0)
}

pub fn wait_fd_read(fd: u64, timeout_ms: u64) -> Result<bool> {
    wait_fd(fd, READ, timeout_ms)
}

pub fn wait_fd_write(fd: u64, timeout_ms: u64) -> Result<bool> {
    wait_fd(fd, WRITE, timeout_ms)
}

pub fn wait_socket_read(socket: u64, timeout_ms: u64) -> Result<bool> {
    let mut desc = PollDesc::socket_read(socket);
    poll(core::slice::from_mut(&mut desc), timeout_ms).map(|n| n > 0 && desc.revents != 0)
}

pub fn wait_gui_event(handle: u64, timeout_ms: u64) -> Result<bool> {
    let mut desc = PollDesc::gui(handle);
    poll(core::slice::from_mut(&mut desc), timeout_ms).map(|n| n > 0 && desc.revents != 0)
}

pub fn wait_child(pid: u64, timeout_ms: u64) -> Result<bool> {
    let mut desc = PollDesc::child(pid);
    poll(core::slice::from_mut(&mut desc), timeout_ms).map(|n| n > 0 && desc.revents != 0)
}
