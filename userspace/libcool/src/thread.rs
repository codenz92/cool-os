use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{process, sys, Error, Result};

pub type Entry = extern "C" fn(u64) -> !;

pub const FUTEX_WAIT_MISMATCH: u64 = 1;
pub const FUTEX_WAIT_TIMEOUT: u64 = 2;
pub const TIMEOUT_FOREVER: u64 = u64::MAX;
pub const TLS_MAGIC: u64 = 0x434f_4f4c_544c_5331;
pub const TLS_SLOT_COUNT: usize = 16;
pub const TLS_ERRNO_SLOT: usize = 0;
pub const TLS_APP_KEY_START: usize = 1;
const TLS_MAGIC_OFFSET: usize = 0;
const TLS_LOGICAL_ID_OFFSET: usize = 8;
const TLS_OS_TID_OFFSET: usize = 16;
const TLS_SLOT_OFFSET: usize = 24;
static NEXT_TLS_KEY: AtomicU64 = AtomicU64::new(TLS_APP_KEY_START as u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FutexWait {
    Woken,
    Mismatch,
    Timeout,
}

#[repr(C)]
pub struct TlsBlock {
    magic: u64,
    logical_id: u64,
    os_tid: u64,
    slots: [u64; TLS_SLOT_COUNT],
}

impl TlsBlock {
    pub const fn new(logical_id: u64) -> Self {
        Self {
            magic: TLS_MAGIC,
            logical_id,
            os_tid: 0,
            slots: [0; TLS_SLOT_COUNT],
        }
    }

    pub fn prepare(&mut self, logical_id: u64) {
        self.magic = TLS_MAGIC;
        self.logical_id = logical_id;
        self.os_tid = 0;
        self.slots = [0; TLS_SLOT_COUNT];
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TlsKey {
    index: usize,
}

impl TlsKey {
    pub const fn index(self) -> usize {
        self.index
    }

    pub fn from_index(index: usize) -> Result<Self> {
        if index < TLS_SLOT_COUNT {
            Ok(Self { index })
        } else {
            Err(Error::Invalid)
        }
    }
}

pub fn spawn(entry: Entry, arg: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall3(sys::THREAD_SPAWN, entry as usize as u64, arg, 0) };
    Error::from_ret(ret)
}

pub unsafe fn spawn_tls(entry: Entry, arg: u64, block: *mut TlsBlock) -> Result<u64> {
    if block.is_null() {
        return Err(Error::Invalid);
    }
    let desc = [entry as usize as u64, arg, block as u64, 0];
    let ret = sys::syscall1(sys::THREAD_SPAWN_TLS, desc.as_ptr() as u64);
    Error::from_ret(ret)
}

pub fn join(tid: u64) -> Result<u64> {
    process::waitpid(tid)
}

pub fn id() -> u64 {
    process::getpid()
}

pub fn exit(code: u64) -> ! {
    process::exit(code)
}

pub fn set_tls_base(base: u64) -> Result<()> {
    let ret = unsafe { sys::syscall2(sys::THREAD_TLS_SET, base, 0) };
    Error::from_ret(ret).map(|_| ())
}

pub fn tls_base() -> u64 {
    unsafe { sys::syscall0(sys::THREAD_TLS_GET) }
}

pub unsafe fn install_tls_block(block: *mut TlsBlock, logical_id: u64) -> Result<()> {
    if block.is_null() {
        return Err(Error::Invalid);
    }
    prepare_tls_block(block, logical_id)?;
    set_tls_base(block as u64)?;
    bind_current_tls_os_tid()
}

pub unsafe fn prepare_tls_block(block: *mut TlsBlock, logical_id: u64) -> Result<()> {
    if block.is_null() {
        return Err(Error::Invalid);
    }
    (*block).prepare(logical_id);
    Ok(())
}

pub fn bind_current_tls_os_tid() -> Result<()> {
    ensure_tls()?;
    unsafe {
        fs_write_u64(TLS_OS_TID_OFFSET, id());
    }
    Ok(())
}

pub fn tls_logical_id() -> Result<u64> {
    ensure_tls()?;
    Ok(unsafe { fs_read_u64(TLS_LOGICAL_ID_OFFSET) })
}

pub fn tls_os_tid() -> Result<u64> {
    ensure_tls()?;
    Ok(unsafe { fs_read_u64(TLS_OS_TID_OFFSET) })
}

pub fn tls_key_create() -> Result<TlsKey> {
    let index = NEXT_TLS_KEY.fetch_add(1, Ordering::SeqCst) as usize;
    if index >= TLS_SLOT_COUNT {
        Err(Error::Failed)
    } else {
        Ok(TlsKey { index })
    }
}

pub fn tls_set(key: TlsKey, value: u64) -> Result<()> {
    if key.index >= TLS_SLOT_COUNT {
        return Err(Error::Invalid);
    }
    ensure_tls()?;
    unsafe {
        fs_write_u64(TLS_SLOT_OFFSET + key.index * 8, value);
    }
    Ok(())
}

pub fn tls_get(key: TlsKey) -> Result<u64> {
    if key.index >= TLS_SLOT_COUNT {
        return Err(Error::Invalid);
    }
    ensure_tls()?;
    Ok(unsafe { fs_read_u64(TLS_SLOT_OFFSET + key.index * 8) })
}

pub fn tls_slot_addr(key: TlsKey) -> Result<u64> {
    if key.index >= TLS_SLOT_COUNT {
        return Err(Error::Invalid);
    }
    ensure_tls()?;
    Ok(tls_base().saturating_add((TLS_SLOT_OFFSET + key.index * 8) as u64))
}

fn ensure_tls() -> Result<()> {
    if tls_base() == 0 {
        return Err(Error::Failed);
    }
    let magic = unsafe { fs_read_u64(TLS_MAGIC_OFFSET) };
    if magic == TLS_MAGIC {
        Ok(())
    } else {
        Err(Error::Failed)
    }
}

pub unsafe fn fs_read_u64(offset: usize) -> u64 {
    let value: u64;
    asm!(
        "mov {value}, qword ptr fs:[{offset}]",
        value = out(reg) value,
        offset = in(reg) offset,
        options(nostack, preserves_flags, readonly),
    );
    value
}

pub unsafe fn fs_write_u64(offset: usize, value: u64) {
    asm!(
        "mov qword ptr fs:[{offset}], {value}",
        offset = in(reg) offset,
        value = in(reg) value,
        options(nostack, preserves_flags),
    );
}

pub fn futex_wait(addr: *const u64, expected: u64, timeout_ms: u64) -> Result<FutexWait> {
    let ret = unsafe { sys::syscall3(sys::FUTEX_WAIT, addr as u64, expected, timeout_ms) };
    match Error::from_ret(ret)? {
        0 => Ok(FutexWait::Woken),
        FUTEX_WAIT_MISMATCH => Ok(FutexWait::Mismatch),
        FUTEX_WAIT_TIMEOUT => Ok(FutexWait::Timeout),
        _ => Err(Error::Failed),
    }
}

pub fn futex_wake(addr: *const u64, count: u64) -> Result<u64> {
    let ret = unsafe { sys::syscall3(sys::FUTEX_WAKE, addr as u64, count, 0) };
    Error::from_ret(ret)
}

pub struct PThreadMutex {
    state: AtomicU64,
}

impl PThreadMutex {
    pub const fn new() -> Self {
        Self {
            state: AtomicU64::new(0),
        }
    }

    pub fn lock(&self) {
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }
        loop {
            if self.state.swap(2, Ordering::Acquire) == 0 {
                return;
            }
            let _ = futex_wait(self.addr(), 2, TIMEOUT_FOREVER);
        }
    }

    pub fn unlock(&self) {
        if self.state.swap(0, Ordering::Release) == 2 {
            let _ = futex_wake(self.addr(), 1);
        }
    }

    fn addr(&self) -> *const u64 {
        &self.state as *const AtomicU64 as *const u64
    }
}

pub struct PThreadCondvar {
    seq: AtomicU64,
}

impl PThreadCondvar {
    pub const fn new() -> Self {
        Self {
            seq: AtomicU64::new(0),
        }
    }

    pub fn wait(&self, mutex: &PThreadMutex) -> Result<()> {
        let observed = self.seq.load(Ordering::SeqCst);
        mutex.unlock();
        let waited = futex_wait(self.addr(), observed, TIMEOUT_FOREVER);
        mutex.lock();
        match waited {
            Ok(FutexWait::Woken) | Ok(FutexWait::Mismatch) => Ok(()),
            Ok(FutexWait::Timeout) => Err(Error::Failed),
            Err(err) => Err(err),
        }
    }

    pub fn notify_one(&self) {
        self.seq.fetch_add(1, Ordering::SeqCst);
        let _ = futex_wake(self.addr(), 1);
    }

    pub fn notify_all(&self) {
        self.seq.fetch_add(1, Ordering::SeqCst);
        let _ = futex_wake(self.addr(), 64);
    }

    fn addr(&self) -> *const u64 {
        &self.seq as *const AtomicU64 as *const u64
    }
}

pub struct PThreadOnce {
    state: AtomicU64,
}

impl PThreadOnce {
    pub const fn new() -> Self {
        Self {
            state: AtomicU64::new(0),
        }
    }

    pub fn call_once(&self, init: fn()) -> Result<()> {
        self.call_once_with(|| init())
    }

    pub fn call_once_c(&self, init: extern "C" fn()) -> Result<()> {
        self.call_once_with(|| init())
    }

    fn call_once_with<F>(&self, init: F) -> Result<()>
    where
        F: FnOnce(),
    {
        loop {
            match self.state.load(Ordering::Acquire) {
                2 => return Ok(()),
                0 => {
                    if self
                        .state
                        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok()
                    {
                        init();
                        self.state.store(2, Ordering::Release);
                        let _ = futex_wake(self.addr(), 64);
                        return Ok(());
                    }
                }
                1 => match futex_wait(self.addr(), 1, TIMEOUT_FOREVER)? {
                    FutexWait::Woken | FutexWait::Mismatch => {}
                    FutexWait::Timeout => return Err(Error::Failed),
                },
                _ => return Err(Error::Failed),
            }
        }
    }

    fn addr(&self) -> *const u64 {
        &self.state as *const AtomicU64 as *const u64
    }
}
