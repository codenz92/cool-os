#![no_std]

use core::arch::asm;

pub const SDK_VERSION: u64 = 1;
pub const ABI_VERSION: u64 = 13;
pub const U64_MAX: u64 = u64::MAX;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Failed,
    Invalid,
}

impl Error {
    #[inline]
    pub const fn from_ret(ret: u64) -> Result<u64> {
        if ret == U64_MAX {
            Err(Error::Failed)
        } else {
            Ok(ret)
        }
    }
}

pub mod sys {
    use super::*;

    pub const EXIT: u64 = 0;
    pub const WRITE: u64 = 1;
    pub const YIELD: u64 = 2;
    pub const GETPID: u64 = 3;
    pub const MMAP: u64 = 4;
    pub const OPEN: u64 = 5;
    pub const READ: u64 = 6;
    pub const CLOSE: u64 = 7;
    pub const EXEC: u64 = 8;
    pub const PIPE: u64 = 9;
    pub const DUP: u64 = 10;
    pub const SHMEM_CREATE: u64 = 11;
    pub const SHMEM_MAP: u64 = 12;
    pub const WAITPID: u64 = 13;
    pub const SPAWN: u64 = 14;
    pub const SLEEP_MS: u64 = 15;
    pub const ABI_VERSION: u64 = 16;
    pub const DNS_RESOLVE: u64 = 17;
    pub const HTTP_GET: u64 = 18;
    pub const SOCKET: u64 = 19;
    pub const CONNECT: u64 = 20;
    pub const SEND: u64 = 21;
    pub const RECV: u64 = 22;
    pub const GUI_OPEN: u64 = 23;
    pub const GUI_PRESENT: u64 = 24;
    pub const GUI_POLL_EVENT: u64 = 25;
    pub const GUI_CLOSE: u64 = 26;
    pub const FS_WRITE_FILE: u64 = 27;
    pub const FS_CREATE_DIR: u64 = 28;
    pub const FS_DELETE_TREE: u64 = 29;
    pub const FS_LIST_DIR: u64 = 30;
    pub const SCREENSHOT: u64 = 31;
    pub const SIGNAL: u64 = 32;
    pub const SETPGID: u64 = 33;
    pub const GETPGID: u64 = 34;
    pub const SIGNAL_GROUP: u64 = 35;
    pub const SPAWN_ARGS: u64 = 36;
    pub const CHDIR: u64 = 37;
    pub const GETCWD: u64 = 38;
    pub const STAT: u64 = 39;
    pub const RENAME: u64 = 40;
    pub const OPEN_WRITE: u64 = 41;
    pub const SPAWN_FDS_ARGS: u64 = 42;
    pub const SYNC: u64 = 43;
    pub const TIME: u64 = 44;
    pub const POLL: u64 = 45;
    pub const TTY_CONTROL: u64 = 46;
    pub const THREAD_SPAWN: u64 = 47;
    pub const FUTEX_WAIT: u64 = 48;
    pub const FUTEX_WAKE: u64 = 49;
    pub const THREAD_TLS_SET: u64 = 50;
    pub const THREAD_TLS_GET: u64 = 51;
    pub const THREAD_SPAWN_TLS: u64 = 52;
    pub const MPROTECT: u64 = 53;

    #[inline]
    pub unsafe fn syscall0(nr: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            lateout("rcx") _,
            lateout("rdi") _,
            lateout("rsi") _,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
        );
        ret
    }

    #[inline]
    pub unsafe fn syscall1(nr: u64, a1: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            lateout("rcx") _,
            lateout("rsi") _,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
        );
        ret
    }

    #[inline]
    pub unsafe fn syscall2(nr: u64, a1: u64, a2: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            inlateout("rsi") a2 => _,
            lateout("rcx") _,
            lateout("rdx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
        );
        ret
    }

    #[inline]
    pub unsafe fn syscall3(nr: u64, a1: u64, a2: u64, a3: u64) -> u64 {
        let ret: u64;
        asm!(
            "syscall",
            inlateout("rax") nr => ret,
            inlateout("rdi") a1 => _,
            inlateout("rsi") a2 => _,
            inlateout("rdx") a3 => _,
            lateout("rcx") _,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
            lateout("r11") _,
        );
        ret
    }
}

pub mod args {
    #[derive(Clone, Copy)]
    pub struct Args {
        rsp: u64,
        argc: usize,
    }

    impl Args {
        /// Build argv access from the initial userspace stack.
        ///
        /// The kernel lays the stack out as `argc, argv..., null, envp_null`.
        #[inline]
        pub unsafe fn from_stack(rsp: u64) -> Self {
            let argc = *(rsp as *const u64) as usize;
            Args { rsp, argc }
        }

        #[inline]
        pub const fn len(self) -> usize {
            self.argc
        }

        #[inline]
        pub const fn is_empty(self) -> bool {
            self.argc == 0
        }

        pub fn get(self, index: usize) -> Option<&'static [u8]> {
            if index >= self.argc {
                return None;
            }
            let ptr_slot = (self.rsp + 8 + index as u64 * 8) as *const u64;
            let ptr = unsafe { *ptr_slot } as *const u8;
            if ptr.is_null() {
                return None;
            }
            let len = unsafe { c_strlen(ptr) };
            Some(unsafe { core::slice::from_raw_parts(ptr, len) })
        }

        #[inline]
        pub fn program(self) -> Option<&'static [u8]> {
            self.get(0)
        }
    }

    unsafe fn c_strlen(mut s: *const u8) -> usize {
        let mut n = 0usize;
        while *s != 0 {
            n += 1;
            s = s.add(1);
        }
        n
    }
}

pub use args::Args;

pub mod process {
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
}

pub mod thread {
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
}

#[allow(non_camel_case_types)]
pub mod posix {
    use core::mem;
    use core::ptr;
    use core::sync::atomic::{AtomicU64, Ordering};

    use super::{process, thread};

    pub type c_int = i32;
    pub type c_void = u8;
    pub type pthread_t = u64;
    pub type pthread_key_t = usize;
    pub type pthread_start_t = extern "C" fn(*mut c_void) -> *mut c_void;
    pub type pthread_destructor_t = extern "C" fn(*mut c_void);

    pub const EAGAIN: c_int = 11;
    pub const EDEADLK: c_int = 35;
    pub const EINVAL: c_int = 22;
    pub const ESRCH: c_int = 3;
    pub const ETIMEDOUT: c_int = 110;
    pub const PTHREAD_THREADS_MAX: usize = 16;
    pub const PTHREAD_KEYS_MAX: usize = thread::TLS_SLOT_COUNT - thread::TLS_APP_KEY_START;

    const PTHREAD_MAIN_LOGICAL_ID: u64 = 1;
    const PTHREAD_SLOT_LOGICAL_BASE: u64 = 1000;
    const SLOT_FREE: u64 = 0;
    const SLOT_RESERVED: u64 = 1;
    const SLOT_RUNNING: u64 = 2;
    const SLOT_EXITED: u64 = 3;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct timespec {
        pub tv_sec: i64,
        pub tv_nsec: i64,
    }

    #[repr(C)]
    pub struct pthread_attr_t {
        flags: u64,
    }

    impl pthread_attr_t {
        pub const fn new() -> Self {
            Self { flags: 0 }
        }
    }

    #[repr(C)]
    pub struct pthread_mutexattr_t {
        flags: u64,
    }

    impl pthread_mutexattr_t {
        pub const fn new() -> Self {
            Self { flags: 0 }
        }
    }

    #[repr(C)]
    pub struct pthread_condattr_t {
        flags: u64,
    }

    impl pthread_condattr_t {
        pub const fn new() -> Self {
            Self { flags: 0 }
        }
    }

    #[repr(C)]
    pub struct pthread_mutex_t {
        inner: thread::PThreadMutex,
    }

    impl pthread_mutex_t {
        pub const fn new() -> Self {
            Self {
                inner: thread::PThreadMutex::new(),
            }
        }
    }

    #[repr(C)]
    pub struct pthread_cond_t {
        inner: thread::PThreadCondvar,
    }

    impl pthread_cond_t {
        pub const fn new() -> Self {
            Self {
                inner: thread::PThreadCondvar::new(),
            }
        }
    }

    #[repr(C)]
    pub struct pthread_once_t {
        inner: thread::PThreadOnce,
    }

    impl pthread_once_t {
        pub const fn new() -> Self {
            Self {
                inner: thread::PThreadOnce::new(),
            }
        }
    }

    struct PthreadSlot {
        state: AtomicU64,
        tid: AtomicU64,
        start: AtomicU64,
        arg: AtomicU64,
        result: AtomicU64,
        tls: thread::TlsBlock,
    }

    unsafe impl Sync for PthreadSlot {}

    impl PthreadSlot {
        const fn new() -> Self {
            Self {
                state: AtomicU64::new(SLOT_FREE),
                tid: AtomicU64::new(0),
                start: AtomicU64::new(0),
                arg: AtomicU64::new(0),
                result: AtomicU64::new(0),
                tls: thread::TlsBlock::new(0),
            }
        }
    }

    static mut PTHREAD_SLOTS: [PthreadSlot; PTHREAD_THREADS_MAX] =
        [const { PthreadSlot::new() }; PTHREAD_THREADS_MAX];

    pub unsafe fn init_main_thread(block: *mut thread::TlsBlock) -> c_int {
        match thread::install_tls_block(block, PTHREAD_MAIN_LOGICAL_ID) {
            Ok(()) => {
                set_errno(0);
                0
            }
            Err(_) => EINVAL,
        }
    }

    pub fn gettid() -> pthread_t {
        thread::id()
    }

    pub fn sched_yield() -> c_int {
        process::yield_now();
        0
    }

    pub fn nanosleep(req: *const timespec, rem: *mut timespec) -> c_int {
        let Some(req) = (unsafe { req.as_ref() }) else {
            return return_errno(EINVAL);
        };
        if req.tv_sec < 0 || req.tv_nsec < 0 || req.tv_nsec >= 1_000_000_000 {
            return return_errno(EINVAL);
        }
        let sec_ms = (req.tv_sec as u64).saturating_mul(1000);
        let nsec_ms = ((req.tv_nsec as u64).saturating_add(999_999)) / 1_000_000;
        process::sleep_ms(sec_ms.saturating_add(nsec_ms));
        if !rem.is_null() {
            unsafe {
                ptr::write(
                    rem,
                    timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                );
            }
        }
        0
    }

    pub fn usleep(usec: u64) -> c_int {
        process::sleep_ms(usec.saturating_add(999) / 1000);
        0
    }

    pub fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int {
        write_or_errno(attr, pthread_attr_t::new())
    }

    pub fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int {
        if attr.is_null() {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_mutexattr_init(attr: *mut pthread_mutexattr_t) -> c_int {
        write_or_errno(attr, pthread_mutexattr_t::new())
    }

    pub fn pthread_mutexattr_destroy(attr: *mut pthread_mutexattr_t) -> c_int {
        if attr.is_null() {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_condattr_init(attr: *mut pthread_condattr_t) -> c_int {
        write_or_errno(attr, pthread_condattr_t::new())
    }

    pub fn pthread_condattr_destroy(attr: *mut pthread_condattr_t) -> c_int {
        if attr.is_null() {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_create(
        thread_out: *mut pthread_t,
        _attr: *const pthread_attr_t,
        start: pthread_start_t,
        arg: *mut c_void,
    ) -> c_int {
        if thread_out.is_null() {
            return return_errno(EINVAL);
        }
        let Some(index) = reserve_slot() else {
            return return_errno(EAGAIN);
        };
        let slot = unsafe { &*slot_ptr(index) };
        slot.tid.store(0, Ordering::Release);
        slot.result.store(0, Ordering::Release);
        slot.start.store(start as usize as u64, Ordering::Release);
        slot.arg.store(arg as u64, Ordering::Release);
        slot.state.store(SLOT_RUNNING, Ordering::Release);

        let tls = unsafe { ptr::addr_of_mut!((*slot_ptr(index)).tls) };
        if unsafe { thread::prepare_tls_block(tls, PTHREAD_SLOT_LOGICAL_BASE + index as u64) }
            .is_err()
        {
            slot.state.store(SLOT_FREE, Ordering::Release);
            return return_errno(EINVAL);
        }

        match unsafe { thread::spawn_tls(pthread_trampoline, index as u64, tls) } {
            Ok(tid) => {
                slot.tid.store(tid, Ordering::Release);
                unsafe {
                    *thread_out = tid;
                }
                0
            }
            Err(_) => {
                slot.state.store(SLOT_FREE, Ordering::Release);
                return_errno(EAGAIN)
            }
        }
    }

    pub fn pthread_join(thread: pthread_t, retval: *mut *mut c_void) -> c_int {
        if thread == pthread_self() {
            return return_errno(EDEADLK);
        }
        let Some(index) = find_slot_by_tid(thread) else {
            return return_errno(ESRCH);
        };
        let slot = unsafe { &*slot_ptr(index) };
        match thread::join(thread) {
            Ok(code) => {
                if !retval.is_null() {
                    unsafe {
                        *retval = code as usize as *mut c_void;
                    }
                }
                slot.start.store(0, Ordering::Release);
                slot.arg.store(0, Ordering::Release);
                slot.result.store(code, Ordering::Release);
                slot.tid.store(0, Ordering::Release);
                slot.state.store(SLOT_FREE, Ordering::Release);
                0
            }
            Err(_) => return_errno(ESRCH),
        }
    }

    pub fn pthread_exit(retval: *mut c_void) -> ! {
        if let Some(index) = find_slot_by_tid(thread::id()) {
            finish_slot(index, retval);
        }
        thread::exit(retval as u64)
    }

    pub fn pthread_self() -> pthread_t {
        thread::id()
    }

    pub fn pthread_equal(left: pthread_t, right: pthread_t) -> c_int {
        if left == right {
            1
        } else {
            0
        }
    }

    pub fn pthread_mutex_init(
        mutex: *mut pthread_mutex_t,
        _attr: *const pthread_mutexattr_t,
    ) -> c_int {
        write_or_errno(mutex, pthread_mutex_t::new())
    }

    pub fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int {
        if mutex.is_null() {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int {
        let Some(mutex) = (unsafe { mutex.as_ref() }) else {
            return return_errno(EINVAL);
        };
        mutex.inner.lock();
        0
    }

    pub fn pthread_mutex_unlock(mutex: *mut pthread_mutex_t) -> c_int {
        let Some(mutex) = (unsafe { mutex.as_ref() }) else {
            return return_errno(EINVAL);
        };
        mutex.inner.unlock();
        0
    }

    pub fn pthread_cond_init(cond: *mut pthread_cond_t, _attr: *const pthread_condattr_t) -> c_int {
        write_or_errno(cond, pthread_cond_t::new())
    }

    pub fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> c_int {
        if cond.is_null() {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_cond_wait(cond: *mut pthread_cond_t, mutex: *mut pthread_mutex_t) -> c_int {
        let Some(cond) = (unsafe { cond.as_ref() }) else {
            return return_errno(EINVAL);
        };
        let Some(mutex) = (unsafe { mutex.as_ref() }) else {
            return return_errno(EINVAL);
        };
        match cond.inner.wait(&mutex.inner) {
            Ok(()) => 0,
            Err(_) => return_errno(EINVAL),
        }
    }

    pub fn pthread_cond_signal(cond: *mut pthread_cond_t) -> c_int {
        let Some(cond) = (unsafe { cond.as_ref() }) else {
            return return_errno(EINVAL);
        };
        cond.inner.notify_one();
        0
    }

    pub fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int {
        let Some(cond) = (unsafe { cond.as_ref() }) else {
            return return_errno(EINVAL);
        };
        cond.inner.notify_all();
        0
    }

    pub fn pthread_once(once: *mut pthread_once_t, init_routine: extern "C" fn()) -> c_int {
        let Some(once) = (unsafe { once.as_ref() }) else {
            return return_errno(EINVAL);
        };
        match once.inner.call_once_c(init_routine) {
            Ok(()) => 0,
            Err(_) => return_errno(EINVAL),
        }
    }

    pub fn pthread_key_create(
        key_out: *mut pthread_key_t,
        _destructor: Option<pthread_destructor_t>,
    ) -> c_int {
        if key_out.is_null() {
            return return_errno(EINVAL);
        }
        match thread::tls_key_create() {
            Ok(key) => {
                unsafe {
                    *key_out = key.index();
                }
                0
            }
            Err(_) => return_errno(EAGAIN),
        }
    }

    pub fn pthread_key_delete(key: pthread_key_t) -> c_int {
        if key < thread::TLS_APP_KEY_START || key >= thread::TLS_SLOT_COUNT {
            return return_errno(EINVAL);
        }
        0
    }

    pub fn pthread_setspecific(key: pthread_key_t, value: *mut c_void) -> c_int {
        let Ok(key) = thread::TlsKey::from_index(key) else {
            return return_errno(EINVAL);
        };
        match thread::tls_set(key, value as u64) {
            Ok(()) => 0,
            Err(_) => return_errno(EINVAL),
        }
    }

    pub fn pthread_getspecific(key: pthread_key_t) -> *mut c_void {
        let Ok(key) = thread::TlsKey::from_index(key) else {
            set_errno(EINVAL);
            return ptr::null_mut();
        };
        match thread::tls_get(key) {
            Ok(value) => value as usize as *mut c_void,
            Err(_) => {
                set_errno(EINVAL);
                ptr::null_mut()
            }
        }
    }

    pub fn errno() -> c_int {
        match thread::tls_get(errno_key()) {
            Ok(value) => value as c_int,
            Err(_) => 0,
        }
    }

    pub fn set_errno(value: c_int) {
        let _ = thread::tls_set(errno_key(), value as u64);
    }

    pub fn errno_location() -> *mut c_int {
        match thread::tls_slot_addr(errno_key()) {
            Ok(addr) => addr as usize as *mut c_int,
            Err(_) => ptr::null_mut(),
        }
    }

    pub fn __errno_location() -> *mut c_int {
        errno_location()
    }

    fn write_or_errno<T>(dst: *mut T, value: T) -> c_int {
        if dst.is_null() {
            return return_errno(EINVAL);
        }
        unsafe {
            ptr::write(dst, value);
        }
        0
    }

    fn return_errno(value: c_int) -> c_int {
        set_errno(value);
        value
    }

    fn errno_key() -> thread::TlsKey {
        match thread::TlsKey::from_index(thread::TLS_ERRNO_SLOT) {
            Ok(key) => key,
            Err(_) => process::abort(),
        }
    }

    fn reserve_slot() -> Option<usize> {
        for index in 0..PTHREAD_THREADS_MAX {
            let slot = unsafe { &*slot_ptr(index) };
            if slot
                .state
                .compare_exchange(
                    SLOT_FREE,
                    SLOT_RESERVED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Some(index);
            }
        }
        None
    }

    fn find_slot_by_tid(tid: pthread_t) -> Option<usize> {
        if tid == 0 {
            return None;
        }
        for index in 0..PTHREAD_THREADS_MAX {
            let slot = unsafe { &*slot_ptr(index) };
            let state = slot.state.load(Ordering::Acquire);
            if state != SLOT_FREE && slot.tid.load(Ordering::Acquire) == tid {
                return Some(index);
            }
        }
        None
    }

    unsafe fn slot_ptr(index: usize) -> *mut PthreadSlot {
        ptr::addr_of_mut!(PTHREAD_SLOTS)
            .cast::<PthreadSlot>()
            .add(index)
    }

    extern "C" fn pthread_trampoline(index: u64) -> ! {
        let index = index as usize;
        if index >= PTHREAD_THREADS_MAX {
            thread::exit(EINVAL as u64);
        }
        let slot = unsafe { &*slot_ptr(index) };
        slot.tid.store(thread::id(), Ordering::Release);
        let _ = thread::bind_current_tls_os_tid();

        let start_bits = slot.start.load(Ordering::Acquire);
        if start_bits == 0 {
            finish_slot(index, EINVAL as usize as *mut c_void);
        }
        let start: pthread_start_t = unsafe { mem::transmute(start_bits as usize) };
        let arg = slot.arg.load(Ordering::Acquire) as usize as *mut c_void;
        let result = start(arg);
        finish_slot(index, result);
    }

    fn finish_slot(index: usize, result: *mut c_void) -> ! {
        let slot = unsafe { &*slot_ptr(index) };
        let code = result as u64;
        slot.result.store(code, Ordering::Release);
        slot.state.store(SLOT_EXITED, Ordering::Release);
        thread::exit(code)
    }
}

pub mod libc {
    pub use crate::posix::*;
}

pub mod memory {
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
}

pub mod io {
    use core::fmt;

    use super::{sys, Error, Result};

    pub const STDIN: u64 = 0;
    pub const STDOUT: u64 = 1;
    pub const STDERR: u64 = 2;

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
}

pub mod dynlink {
    use core::{mem, ptr};

    use super::{io, memory, thread, Error, Result};

    pub const DEFAULT_LOAD_BASE: u64 = 0x0000_7fff_2000_0000;
    pub const DEFAULT_OBJECT_STRIDE: u64 = 0x0000_0000_0020_0000;
    pub const MAX_IMAGE_BYTES: usize = 16 * 1024;
    pub const MAX_OBJECTS: usize = 4;
    pub const MAX_OBJECT_NAME: usize = 48;
    pub const MAX_PATH_BYTES: usize = 64;
    pub const MAX_TLS_BYTES: usize = 256;

    const PAGE_SIZE: u64 = 4096;
    const MAX_PHDRS: usize = 16;
    const MAX_LOAD_SEGMENTS: usize = 8;
    const MAX_NEEDED: usize = 4;
    const MAX_SYMBOLS: usize = 128;
    const MAX_RELOCATIONS: usize = 128;
    const MAX_INIT_ARRAY: usize = 16;

    const ET_DYN: u16 = 3;
    const EM_X86_64: u16 = 62;
    const PT_LOAD: u32 = 1;
    const PT_DYNAMIC: u32 = 2;
    const PT_TLS: u32 = 7;
    const PF_X: u32 = 1;
    const PF_W: u32 = 2;

    const DT_NULL: i64 = 0;
    const DT_NEEDED: i64 = 1;
    const DT_HASH: i64 = 4;
    const DT_STRTAB: i64 = 5;
    const DT_SYMTAB: i64 = 6;
    const DT_RELA: i64 = 7;
    const DT_RELASZ: i64 = 8;
    const DT_RELAENT: i64 = 9;
    const DT_STRSZ: i64 = 10;
    const DT_SYMENT: i64 = 11;
    const DT_SONAME: i64 = 14;
    const DT_INIT_ARRAY: i64 = 25;
    const DT_FINI_ARRAY: i64 = 26;
    const DT_INIT_ARRAYSZ: i64 = 27;
    const DT_FINI_ARRAYSZ: i64 = 28;

    const R_X86_64_NONE: u32 = 0;
    const R_X86_64_64: u32 = 1;
    const R_X86_64_GLOB_DAT: u32 = 6;
    const R_X86_64_JUMP_SLOT: u32 = 7;
    const R_X86_64_RELATIVE: u32 = 8;
    const R_X86_64_DTPMOD64: u32 = 16;
    const R_X86_64_DTPOFF64: u32 = 17;
    const R_X86_64_TPOFF64: u32 = 18;

    const STT_TLS: u8 = 6;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Header {
        e_ident: [u8; 16],
        e_type: u16,
        e_machine: u16,
        e_version: u32,
        e_entry: u64,
        e_phoff: u64,
        e_shoff: u64,
        e_flags: u32,
        e_ehsize: u16,
        e_phentsize: u16,
        e_phnum: u16,
        e_shentsize: u16,
        e_shnum: u16,
        e_shstrndx: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64ProgramHeader {
        p_type: u32,
        p_flags: u32,
        p_offset: u64,
        p_vaddr: u64,
        p_paddr: u64,
        p_filesz: u64,
        p_memsz: u64,
        p_align: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Dyn {
        d_tag: i64,
        d_val: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Rela {
        r_offset: u64,
        r_info: u64,
        r_addend: i64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Elf64Sym {
        st_name: u32,
        st_info: u8,
        st_other: u8,
        st_shndx: u16,
        st_value: u64,
        st_size: u64,
    }

    #[derive(Clone, Copy)]
    struct LoadSegment {
        vaddr_start: u64,
        vaddr_end: u64,
        map_start: u64,
        map_len: u64,
        flags: u32,
    }

    impl LoadSegment {
        const fn empty() -> Self {
            Self {
                vaddr_start: 0,
                vaddr_end: 0,
                map_start: 0,
                map_len: 0,
                flags: 0,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct DynamicInfo {
        hash: u64,
        strtab: u64,
        strsz: usize,
        symtab: u64,
        syment: usize,
        needed: [u64; MAX_NEEDED],
        needed_count: usize,
        soname: u64,
        rela: u64,
        relasz: usize,
        relaent: usize,
        init_array: u64,
        init_arraysz: usize,
        fini_array: u64,
        fini_arraysz: usize,
    }

    impl DynamicInfo {
        const fn empty() -> Self {
            Self {
                hash: 0,
                strtab: 0,
                strsz: 0,
                symtab: 0,
                syment: mem::size_of::<Elf64Sym>(),
                needed: [0; MAX_NEEDED],
                needed_count: 0,
                soname: 0,
                rela: 0,
                relasz: 0,
                relaent: mem::size_of::<Elf64Rela>(),
                init_array: 0,
                init_arraysz: 0,
                fini_array: 0,
                fini_arraysz: 0,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct NeededName {
        bytes: [u8; MAX_OBJECT_NAME],
        len: usize,
    }

    impl NeededName {
        const fn empty() -> Self {
            Self {
                bytes: [0; MAX_OBJECT_NAME],
                len: 0,
            }
        }

        fn as_slice(&self) -> &[u8] {
            &self.bytes[..self.len]
        }
    }

    #[derive(Clone, Copy)]
    struct TlsInfo {
        present: bool,
        offset: u64,
        vaddr: u64,
        filesz: u64,
        memsz: u64,
        align: u64,
    }

    impl TlsInfo {
        const fn empty() -> Self {
            Self {
                present: false,
                offset: 0,
                vaddr: 0,
                filesz: 0,
                memsz: 0,
                align: 1,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct ResolvedSymbol {
        value: u64,
        tls_module_id: u64,
        tls_offset: u64,
        is_tls: bool,
    }

    #[derive(Clone, Copy)]
    pub struct LoadedObject {
        base: u64,
        bias: u64,
        path: [u8; MAX_PATH_BYTES],
        path_len: usize,
        soname: [u8; MAX_OBJECT_NAME],
        soname_len: usize,
        strtab: u64,
        strsz: usize,
        symtab: u64,
        syment: usize,
        symbol_count: usize,
        load_count: usize,
        needed_count: usize,
        relocation_count: usize,
        init_array: u64,
        init_count: usize,
        fini_count: usize,
        tls_addr: u64,
        tls_vaddr: u64,
        tls_memsz: usize,
        tls_module_id: u64,
    }

    impl LoadedObject {
        const fn empty() -> Self {
            Self {
                base: 0,
                bias: 0,
                path: [0; MAX_PATH_BYTES],
                path_len: 0,
                soname: [0; MAX_OBJECT_NAME],
                soname_len: 0,
                strtab: 0,
                strsz: 0,
                symtab: 0,
                syment: mem::size_of::<Elf64Sym>(),
                symbol_count: 0,
                load_count: 0,
                needed_count: 0,
                relocation_count: 0,
                init_array: 0,
                init_count: 0,
                fini_count: 0,
                tls_addr: 0,
                tls_vaddr: 0,
                tls_memsz: 0,
                tls_module_id: 0,
            }
        }

        pub const fn base(self) -> u64 {
            self.base
        }

        pub fn path(&self) -> &[u8] {
            &self.path[..self.path_len]
        }

        pub fn soname(&self) -> &[u8] {
            &self.soname[..self.soname_len]
        }

        pub const fn load_count(self) -> usize {
            self.load_count
        }

        pub const fn needed_count(self) -> usize {
            self.needed_count
        }

        pub const fn relocation_count(self) -> usize {
            self.relocation_count
        }

        pub const fn init_count(self) -> usize {
            self.init_count
        }

        pub const fn fini_count(self) -> usize {
            self.fini_count
        }

        pub const fn tls_module_id(self) -> u64 {
            self.tls_module_id
        }

        pub const fn tls_bytes(self) -> usize {
            self.tls_memsz
        }

        pub fn symbol(self, name: &[u8]) -> Result<u64> {
            if name.is_empty() || self.symbol_count == 0 {
                return Err(Error::Invalid);
            }
            let mut idx = 0usize;
            while idx < self.symbol_count {
                let sym = unsafe {
                    read_runtime::<Elf64Sym>(self.symtab + idx as u64 * self.syment as u64)
                };
                if sym.st_name as usize >= self.strsz {
                    idx += 1;
                    continue;
                }
                let str_addr = self.strtab + sym.st_name as u64;
                if cstr_eq(str_addr, self.strsz - sym.st_name as usize, name) {
                    if sym.st_shndx == 0
                        || (sym.st_value == 0 && symbol_type(sym.st_info) != STT_TLS)
                    {
                        return Err(Error::Invalid);
                    }
                    return object_symbol_value(&self, sym).map(|resolved| resolved.value);
                }
                idx += 1;
            }
            Err(Error::Invalid)
        }

        pub unsafe fn call_init_array(self) -> Result<()> {
            let mut idx = 0usize;
            while idx < self.init_count {
                let slot = self.init_array + idx as u64 * 8;
                let func_addr = read_runtime::<u64>(slot);
                if func_addr != 0 {
                    let init: extern "C" fn() = mem::transmute(func_addr as usize);
                    init();
                }
                idx += 1;
            }
            Ok(())
        }
    }

    pub struct Workspace {
        images: [[u8; MAX_IMAGE_BYTES]; MAX_OBJECTS],
        tls: [u8; MAX_TLS_BYTES],
    }

    impl Workspace {
        pub const fn new() -> Self {
            Self {
                images: [[0; MAX_IMAGE_BYTES]; MAX_OBJECTS],
                tls: [0; MAX_TLS_BYTES],
            }
        }
    }

    pub struct LoadedSet {
        objects: [LoadedObject; MAX_OBJECTS],
        object_count: usize,
        image_count: usize,
        dependency_count: usize,
        relocation_count: usize,
        init_count: usize,
        tls_bytes: usize,
        next_tls_module: u64,
    }

    impl LoadedSet {
        const fn empty() -> Self {
            Self {
                objects: [LoadedObject::empty(); MAX_OBJECTS],
                object_count: 0,
                image_count: 0,
                dependency_count: 0,
                relocation_count: 0,
                init_count: 0,
                tls_bytes: 0,
                next_tls_module: 1,
            }
        }

        pub const fn object_count(&self) -> usize {
            self.object_count
        }

        pub const fn dependency_count(&self) -> usize {
            self.dependency_count
        }

        pub const fn relocation_count(&self) -> usize {
            self.relocation_count
        }

        pub const fn init_count(&self) -> usize {
            self.init_count
        }

        pub const fn tls_bytes(&self) -> usize {
            self.tls_bytes
        }

        pub fn object(&self, index: usize) -> Option<LoadedObject> {
            if index < self.object_count {
                Some(self.objects[index])
            } else {
                None
            }
        }

        pub fn symbol(&self, name: &[u8]) -> Result<u64> {
            let mut idx = self.object_count;
            while idx > 0 {
                idx -= 1;
                if let Ok(addr) = self.objects[idx].symbol(name) {
                    return Ok(addr);
                }
            }
            Err(Error::Invalid)
        }

        fn find_loaded(&self, path: &[u8]) -> Option<usize> {
            let name = basename(path);
            let mut idx = 0usize;
            while idx < self.object_count {
                let object = &self.objects[idx];
                if bytes_eq(object.path(), path)
                    || bytes_eq(basename(object.path()), name)
                    || (!object.soname().is_empty() && bytes_eq(object.soname(), name))
                {
                    return Some(idx);
                }
                idx += 1;
            }
            None
        }
    }

    pub fn load(path: &[u8], image: &mut [u8], load_base: u64) -> Result<LoadedObject> {
        if image.len() > MAX_IMAGE_BYTES || load_base & (PAGE_SIZE - 1) != 0 {
            return Err(Error::Invalid);
        }
        let len = read_whole_file(path, image)?;
        load_image(&image[..len], load_base)
    }

    pub fn load_with_deps(
        path: &[u8],
        workspace: &mut Workspace,
        load_base: u64,
    ) -> Result<LoadedSet> {
        if load_base & (PAGE_SIZE - 1) != 0 || DEFAULT_OBJECT_STRIDE & (PAGE_SIZE - 1) != 0 {
            return Err(Error::Invalid);
        }
        let mut set = LoadedSet::empty();
        load_recursive(path, workspace, &mut set, load_base)?;
        Ok(set)
    }

    pub fn load_image(image: &[u8], load_base: u64) -> Result<LoadedObject> {
        let header = parse_header(image)?;
        let mut loads = [LoadSegment::empty(); MAX_LOAD_SEGMENTS];
        let load_count = collect_load_segments(image, &header, load_base, &mut loads)?;
        let bias = load_bias(load_base, &loads, load_count)?;
        let mut adjust = 0usize;
        while adjust < load_count {
            loads[adjust].map_start = runtime_addr(bias, loads[adjust].vaddr_start)?;
            adjust += 1;
        }

        let mut idx = 0usize;
        while idx < load_count {
            map_load_segment(image, &loads[idx], bias, &header)?;
            idx += 1;
        }

        let dyninfo = parse_dynamic(image, &header)?;
        let symbol_count = symbol_count(&dyninfo, bias, &loads, load_count)?;
        let relocation_count = apply_relocations(&dyninfo, bias, symbol_count, &loads, load_count)?;
        if dyninfo.init_arraysz != 0
            && (dyninfo.init_array == 0
                || !vaddr_range_loaded(
                    &loads,
                    load_count,
                    dyninfo.init_array,
                    dyninfo.init_arraysz as u64,
                    false,
                ))
        {
            return Err(Error::Invalid);
        }
        protect_load_segments(&loads, load_count)?;

        let init_count = dyninfo.init_arraysz / 8;
        if init_count > MAX_INIT_ARRAY {
            return Err(Error::Invalid);
        }
        let object = LoadedObject {
            base: load_base,
            bias,
            path: [0; MAX_PATH_BYTES],
            path_len: 0,
            soname: [0; MAX_OBJECT_NAME],
            soname_len: 0,
            strtab: runtime_addr(bias, dyninfo.strtab)?,
            strsz: dyninfo.strsz,
            symtab: runtime_addr(bias, dyninfo.symtab)?,
            syment: dyninfo.syment,
            symbol_count,
            load_count,
            needed_count: dyninfo.needed_count,
            relocation_count,
            init_array: if dyninfo.init_array == 0 {
                0
            } else {
                runtime_addr(bias, dyninfo.init_array)?
            },
            init_count,
            fini_count: dyninfo.fini_arraysz / 8,
            tls_addr: 0,
            tls_vaddr: 0,
            tls_memsz: 0,
            tls_module_id: 0,
        };
        unsafe { object.call_init_array()? };
        Ok(object)
    }

    fn load_recursive(
        path: &[u8],
        workspace: &mut Workspace,
        set: &mut LoadedSet,
        load_base: u64,
    ) -> Result<usize> {
        if let Some(index) = set.find_loaded(path) {
            return Ok(index);
        }
        if set.image_count >= MAX_OBJECTS {
            return Err(Error::Invalid);
        }
        let image_index = set.image_count;
        set.image_count += 1;
        let len = read_whole_file(path, &mut workspace.images[image_index])?;

        let mut needed = [NeededName::empty(); MAX_NEEDED];
        let needed_count = {
            let image = &workspace.images[image_index][..len];
            let header = parse_header(image)?;
            let dyninfo = parse_dynamic(image, &header)?;
            collect_needed_names(image, &header, &dyninfo, &mut needed)?
        };

        let mut dep_idx = 0usize;
        while dep_idx < needed_count {
            let mut dep_path = [0u8; MAX_PATH_BYTES];
            let dep_len = build_lib_path(needed[dep_idx].as_slice(), &mut dep_path)?;
            let before = set.object_count;
            load_recursive(&dep_path[..dep_len], workspace, set, load_base)?;
            if set.object_count > before {
                set.dependency_count += 1;
            }
            dep_idx += 1;
        }

        if set.object_count >= MAX_OBJECTS {
            return Err(Error::Invalid);
        }
        let object_index = set.object_count;
        let object_base = load_base
            .checked_add(
                DEFAULT_OBJECT_STRIDE
                    .checked_mul(object_index as u64)
                    .ok_or(Error::Invalid)?,
            )
            .ok_or(Error::Invalid)?;
        let image = &workspace.images[image_index][..len];
        let object = load_image_with_set(image, object_base, path, &mut workspace.tls, set)?;
        set.objects[object_index] = object;
        set.object_count += 1;
        set.relocation_count = set
            .relocation_count
            .checked_add(object.relocation_count)
            .ok_or(Error::Invalid)?;
        set.init_count = set
            .init_count
            .checked_add(object.init_count)
            .ok_or(Error::Invalid)?;
        unsafe { set.objects[object_index].call_init_array()? };
        Ok(object_index)
    }

    fn load_image_with_set(
        image: &[u8],
        load_base: u64,
        path: &[u8],
        tls_workspace: &mut [u8; MAX_TLS_BYTES],
        set: &mut LoadedSet,
    ) -> Result<LoadedObject> {
        let header = parse_header(image)?;
        let mut loads = [LoadSegment::empty(); MAX_LOAD_SEGMENTS];
        let load_count = collect_load_segments(image, &header, load_base, &mut loads)?;
        let bias = load_bias(load_base, &loads, load_count)?;
        let mut adjust = 0usize;
        while adjust < load_count {
            loads[adjust].map_start = runtime_addr(bias, loads[adjust].vaddr_start)?;
            adjust += 1;
        }

        let mut idx = 0usize;
        while idx < load_count {
            map_load_segment(image, &loads[idx], bias, &header)?;
            idx += 1;
        }

        let dyninfo = parse_dynamic(image, &header)?;
        let symbol_count = symbol_count(&dyninfo, bias, &loads, load_count)?;
        let tlsinfo = parse_tls(image, &header)?;
        let (tls_addr, tls_module_id) = allocate_tls(image, &tlsinfo, tls_workspace, set)?;
        let mut path_buf = [0u8; MAX_PATH_BYTES];
        let path_len = copy_bytes(path, &mut path_buf)?;
        let mut soname = [0u8; MAX_OBJECT_NAME];
        let soname_len =
            copy_dynamic_string(image, &header, &dyninfo, dyninfo.soname, &mut soname)?;
        let init_count = dyninfo.init_arraysz / 8;
        let fini_count = dyninfo.fini_arraysz / 8;
        if init_count > MAX_INIT_ARRAY || fini_count > MAX_INIT_ARRAY {
            return Err(Error::Invalid);
        }
        if dyninfo.init_arraysz != 0
            && (dyninfo.init_array == 0
                || !vaddr_range_loaded(
                    &loads,
                    load_count,
                    dyninfo.init_array,
                    dyninfo.init_arraysz as u64,
                    false,
                ))
        {
            return Err(Error::Invalid);
        }
        if dyninfo.fini_arraysz != 0
            && (dyninfo.fini_array == 0
                || !vaddr_range_loaded(
                    &loads,
                    load_count,
                    dyninfo.fini_array,
                    dyninfo.fini_arraysz as u64,
                    false,
                ))
        {
            return Err(Error::Invalid);
        }

        let mut object = LoadedObject {
            base: load_base,
            bias,
            path: path_buf,
            path_len,
            soname,
            soname_len,
            strtab: runtime_addr(bias, dyninfo.strtab)?,
            strsz: dyninfo.strsz,
            symtab: runtime_addr(bias, dyninfo.symtab)?,
            syment: dyninfo.syment,
            symbol_count,
            load_count,
            needed_count: dyninfo.needed_count,
            relocation_count: 0,
            init_array: if dyninfo.init_array == 0 {
                0
            } else {
                runtime_addr(bias, dyninfo.init_array)?
            },
            init_count,
            fini_count,
            tls_addr,
            tls_vaddr: tlsinfo.vaddr,
            tls_memsz: tlsinfo.memsz as usize,
            tls_module_id,
        };
        let relocation_count =
            apply_relocations_with_set(&dyninfo, &object, &loads, load_count, set)?;
        object.relocation_count = relocation_count;
        protect_load_segments(&loads, load_count)?;
        Ok(object)
    }

    fn read_whole_file(path: &[u8], image: &mut [u8]) -> Result<usize> {
        let file = io::File::open(path)?;
        let mut total = 0usize;
        loop {
            if total == image.len() {
                file.close();
                return Err(Error::Invalid);
            }
            let n = file.read(&mut image[total..])?;
            if n == 0 {
                file.close();
                return Ok(total);
            }
            total = total.checked_add(n).ok_or(Error::Invalid)?;
        }
    }

    fn parse_header(image: &[u8]) -> Result<Elf64Header> {
        let header = read_struct::<Elf64Header>(image, 0).ok_or(Error::Invalid)?;
        if &header.e_ident[0..4] != b"\x7fELF"
            || header.e_ident[4] != 2
            || header.e_ident[5] != 1
            || header.e_ident[6] != 1
        {
            return Err(Error::Invalid);
        }
        if header.e_type != ET_DYN || header.e_machine != EM_X86_64 {
            return Err(Error::Invalid);
        }
        if header.e_phentsize as usize != mem::size_of::<Elf64ProgramHeader>()
            || header.e_phnum as usize > MAX_PHDRS
        {
            return Err(Error::Invalid);
        }
        let ph_bytes = (header.e_phnum as usize)
            .checked_mul(header.e_phentsize as usize)
            .ok_or(Error::Invalid)?;
        let ph_end = (header.e_phoff as usize)
            .checked_add(ph_bytes)
            .ok_or(Error::Invalid)?;
        if ph_end > image.len() {
            return Err(Error::Invalid);
        }
        Ok(header)
    }

    fn collect_load_segments(
        image: &[u8],
        header: &Elf64Header,
        _load_base: u64,
        loads: &mut [LoadSegment; MAX_LOAD_SEGMENTS],
    ) -> Result<usize> {
        let mut count = 0usize;
        let mut i = 0u16;
        while i < header.e_phnum {
            let ph = program_header(image, header, i)?;
            if ph.p_type == PT_LOAD {
                if count >= MAX_LOAD_SEGMENTS {
                    return Err(Error::Invalid);
                }
                validate_load(image, &ph)?;
                let start = align_down(ph.p_vaddr, PAGE_SIZE);
                let end = align_up(
                    ph.p_vaddr.checked_add(ph.p_memsz).ok_or(Error::Invalid)?,
                    PAGE_SIZE,
                )?;
                let mut existing = 0usize;
                while existing < count {
                    if ranges_overlap(
                        start,
                        end,
                        loads[existing].vaddr_start,
                        loads[existing].vaddr_end,
                    ) {
                        return Err(Error::Invalid);
                    }
                    existing += 1;
                }
                loads[count] = LoadSegment {
                    vaddr_start: start,
                    vaddr_end: end,
                    map_start: 0,
                    map_len: end - start,
                    flags: ph.p_flags,
                };
                count += 1;
            }
            i += 1;
        }
        if count == 0 {
            return Err(Error::Invalid);
        }
        Ok(count)
    }

    fn load_bias(
        load_base: u64,
        loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
        count: usize,
    ) -> Result<u64> {
        let mut min = u64::MAX;
        let mut idx = 0usize;
        while idx < count {
            if loads[idx].vaddr_start < min {
                min = loads[idx].vaddr_start;
            }
            idx += 1;
        }
        load_base.checked_sub(min).ok_or(Error::Invalid)
    }

    fn validate_load(image: &[u8], ph: &Elf64ProgramHeader) -> Result<()> {
        if ph.p_memsz == 0 || ph.p_filesz > ph.p_memsz {
            return Err(Error::Invalid);
        }
        if ph.p_align > 1 && !ph.p_align.is_power_of_two() {
            return Err(Error::Invalid);
        }
        if ph.p_align >= PAGE_SIZE
            && (ph.p_offset & (PAGE_SIZE - 1)) != (ph.p_vaddr & (PAGE_SIZE - 1))
        {
            return Err(Error::Invalid);
        }
        let file_end = ph.p_offset.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
        if file_end > image.len() as u64 {
            return Err(Error::Invalid);
        }
        ph.p_vaddr.checked_add(ph.p_memsz).ok_or(Error::Invalid)?;
        Ok(())
    }

    fn map_load_segment(
        image: &[u8],
        load: &LoadSegment,
        bias: u64,
        header: &Elf64Header,
    ) -> Result<()> {
        memory::mmap(load.map_start, load.map_len as usize, true)?;

        let mut i = 0u16;
        while i < header.e_phnum {
            let ph = program_header(image, header, i)?;
            let start = align_down(ph.p_vaddr, PAGE_SIZE);
            if ph.p_type == PT_LOAD && start == load.vaddr_start {
                if ph.p_filesz != 0 {
                    let dst = runtime_addr(bias, ph.p_vaddr)? as *mut u8;
                    let src = image.as_ptr().wrapping_add(ph.p_offset as usize);
                    unsafe {
                        ptr::copy_nonoverlapping(src, dst, ph.p_filesz as usize);
                    }
                }
                return Ok(());
            }
            i += 1;
        }
        Err(Error::Invalid)
    }

    fn parse_dynamic(image: &[u8], header: &Elf64Header) -> Result<DynamicInfo> {
        let mut info = DynamicInfo::empty();
        let mut found = false;
        let mut i = 0u16;
        while i < header.e_phnum {
            let ph = program_header(image, header, i)?;
            if ph.p_type == PT_DYNAMIC {
                found = true;
                let count = (ph.p_filesz as usize) / mem::size_of::<Elf64Dyn>();
                let mut idx = 0usize;
                while idx < count {
                    let dynent = read_struct::<Elf64Dyn>(
                        image,
                        ph.p_offset as usize + idx * mem::size_of::<Elf64Dyn>(),
                    )
                    .ok_or(Error::Invalid)?;
                    match dynent.d_tag {
                        DT_NULL => break,
                        DT_NEEDED => {
                            if info.needed_count >= MAX_NEEDED {
                                return Err(Error::Invalid);
                            }
                            info.needed[info.needed_count] = dynent.d_val;
                            info.needed_count += 1;
                        }
                        DT_HASH => info.hash = dynent.d_val,
                        DT_STRTAB => info.strtab = dynent.d_val,
                        DT_SYMTAB => info.symtab = dynent.d_val,
                        DT_STRSZ => info.strsz = dynent.d_val as usize,
                        DT_SYMENT => info.syment = dynent.d_val as usize,
                        DT_SONAME => info.soname = dynent.d_val,
                        DT_RELA => info.rela = dynent.d_val,
                        DT_RELASZ => info.relasz = dynent.d_val as usize,
                        DT_RELAENT => info.relaent = dynent.d_val as usize,
                        DT_INIT_ARRAY => info.init_array = dynent.d_val,
                        DT_INIT_ARRAYSZ => info.init_arraysz = dynent.d_val as usize,
                        DT_FINI_ARRAY => info.fini_array = dynent.d_val,
                        DT_FINI_ARRAYSZ => info.fini_arraysz = dynent.d_val as usize,
                        _ => {}
                    }
                    idx += 1;
                }
            }
            i += 1;
        }
        if !found
            || info.hash == 0
            || info.strtab == 0
            || info.symtab == 0
            || info.strsz == 0
            || info.syment != mem::size_of::<Elf64Sym>()
            || info.relaent != mem::size_of::<Elf64Rela>()
            || info.init_arraysz % 8 != 0
            || info.fini_arraysz % 8 != 0
        {
            return Err(Error::Invalid);
        }
        Ok(info)
    }

    fn collect_needed_names(
        image: &[u8],
        header: &Elf64Header,
        info: &DynamicInfo,
        out: &mut [NeededName; MAX_NEEDED],
    ) -> Result<usize> {
        let mut idx = 0usize;
        while idx < info.needed_count {
            let len =
                copy_dynamic_string(image, header, info, info.needed[idx], &mut out[idx].bytes)?;
            if len == 0 {
                return Err(Error::Invalid);
            }
            out[idx].len = len;
            idx += 1;
        }
        Ok(info.needed_count)
    }

    fn copy_dynamic_string(
        image: &[u8],
        header: &Elf64Header,
        info: &DynamicInfo,
        offset: u64,
        out: &mut [u8],
    ) -> Result<usize> {
        if offset == 0 {
            return Ok(0);
        }
        if offset >= info.strsz as u64 || out.is_empty() {
            return Err(Error::Invalid);
        }
        let start = vaddr_to_file_offset(
            image,
            header,
            info.strtab.checked_add(offset).ok_or(Error::Invalid)?,
            1,
        )?;
        let max = info.strsz - offset as usize;
        let mut idx = 0usize;
        while idx < max {
            let src = start.checked_add(idx).ok_or(Error::Invalid)?;
            if src >= image.len() {
                return Err(Error::Invalid);
            }
            let byte = image[src];
            if byte == 0 {
                if idx > out.len() {
                    return Err(Error::Invalid);
                }
                return Ok(idx);
            }
            if idx >= out.len() {
                return Err(Error::Invalid);
            }
            out[idx] = byte;
            idx += 1;
        }
        Err(Error::Invalid)
    }

    fn parse_tls(image: &[u8], header: &Elf64Header) -> Result<TlsInfo> {
        let mut info = TlsInfo::empty();
        let mut i = 0u16;
        while i < header.e_phnum {
            let ph = program_header(image, header, i)?;
            if ph.p_type == PT_TLS {
                if info.present || ph.p_filesz > ph.p_memsz || ph.p_memsz as usize > MAX_TLS_BYTES {
                    return Err(Error::Invalid);
                }
                if ph.p_align > 1 && !ph.p_align.is_power_of_two() {
                    return Err(Error::Invalid);
                }
                let file_end = ph.p_offset.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
                if file_end > image.len() as u64 {
                    return Err(Error::Invalid);
                }
                info = TlsInfo {
                    present: true,
                    offset: ph.p_offset,
                    vaddr: ph.p_vaddr,
                    filesz: ph.p_filesz,
                    memsz: ph.p_memsz,
                    align: if ph.p_align == 0 { 1 } else { ph.p_align },
                };
            }
            i += 1;
        }
        Ok(info)
    }

    fn allocate_tls(
        image: &[u8],
        info: &TlsInfo,
        tls_workspace: &mut [u8; MAX_TLS_BYTES],
        set: &mut LoadedSet,
    ) -> Result<(u64, u64)> {
        if !info.present || info.memsz == 0 {
            return Ok((0, 0));
        }
        let align = if info.align == 0 {
            1
        } else {
            info.align as usize
        };
        if align == 0 || align > 64 || !align.is_power_of_two() {
            return Err(Error::Invalid);
        }
        let start = align_up_usize(set.tls_bytes, align)?;
        let end = start
            .checked_add(info.memsz as usize)
            .ok_or(Error::Invalid)?;
        if end > MAX_TLS_BYTES {
            return Err(Error::Invalid);
        }
        let src_start = info.offset as usize;
        let src_end = src_start
            .checked_add(info.filesz as usize)
            .ok_or(Error::Invalid)?;
        if src_end > image.len() {
            return Err(Error::Invalid);
        }
        let mut idx = 0usize;
        while idx < info.filesz as usize {
            tls_workspace[start + idx] = image[src_start + idx];
            idx += 1;
        }
        while idx < info.memsz as usize {
            tls_workspace[start + idx] = 0;
            idx += 1;
        }
        let module_id = set.next_tls_module;
        set.next_tls_module = set.next_tls_module.checked_add(1).ok_or(Error::Invalid)?;
        set.tls_bytes = end;
        Ok((tls_workspace.as_ptr() as u64 + start as u64, module_id))
    }

    fn symbol_count(
        info: &DynamicInfo,
        bias: u64,
        loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
        load_count: usize,
    ) -> Result<usize> {
        if !vaddr_range_loaded(loads, load_count, info.hash, 8, false)
            || !vaddr_range_loaded(loads, load_count, info.strtab, info.strsz as u64, false)
            || !vaddr_range_loaded(loads, load_count, info.symtab, info.syment as u64, false)
        {
            return Err(Error::Invalid);
        }
        let hash_addr = runtime_addr(bias, info.hash)?;
        let nchain = unsafe { read_runtime::<u32>(hash_addr + 4) as usize };
        if nchain == 0 || nchain > MAX_SYMBOLS {
            return Err(Error::Invalid);
        }
        let sym_bytes = nchain.checked_mul(info.syment).ok_or(Error::Invalid)?;
        if !vaddr_range_loaded(loads, load_count, info.symtab, sym_bytes as u64, false) {
            return Err(Error::Invalid);
        }
        Ok(nchain)
    }

    fn apply_relocations(
        info: &DynamicInfo,
        bias: u64,
        symbol_count: usize,
        loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
        load_count: usize,
    ) -> Result<usize> {
        if info.relasz == 0 {
            return Ok(0);
        }
        if info.rela == 0 || info.relasz % info.relaent != 0 {
            return Err(Error::Invalid);
        }
        let count = info.relasz / info.relaent;
        if count > MAX_RELOCATIONS
            || !vaddr_range_loaded(loads, load_count, info.rela, info.relasz as u64, false)
        {
            return Err(Error::Invalid);
        }

        let mut idx = 0usize;
        while idx < count {
            let rela = unsafe {
                read_runtime::<Elf64Rela>(
                    runtime_addr(bias, info.rela)? + idx as u64 * info.relaent as u64,
                )
            };
            let r_type = (rela.r_info & 0xffff_ffff) as u32;
            let r_sym = (rela.r_info >> 32) as usize;
            if r_type == R_X86_64_NONE {
                idx += 1;
                continue;
            }
            if !vaddr_range_loaded(loads, load_count, rela.r_offset, 8, true) {
                return Err(Error::Invalid);
            }
            let value = match r_type {
                R_X86_64_RELATIVE => checked_add_i64(bias, rela.r_addend)?,
                R_X86_64_64 | R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                    let sym_value = symbol_value(info, bias, symbol_count, r_sym)?;
                    checked_add_i64(sym_value, rela.r_addend)?
                }
                _ => return Err(Error::Invalid),
            };
            unsafe {
                ptr::write_unaligned(runtime_addr(bias, rela.r_offset)? as *mut u64, value);
            }
            idx += 1;
        }
        Ok(count)
    }

    fn apply_relocations_with_set(
        info: &DynamicInfo,
        object: &LoadedObject,
        loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
        load_count: usize,
        set: &LoadedSet,
    ) -> Result<usize> {
        if info.relasz == 0 {
            return Ok(0);
        }
        if info.rela == 0 || info.relasz % info.relaent != 0 {
            return Err(Error::Invalid);
        }
        let count = info.relasz / info.relaent;
        if count > MAX_RELOCATIONS
            || !vaddr_range_loaded(loads, load_count, info.rela, info.relasz as u64, false)
        {
            return Err(Error::Invalid);
        }

        let mut idx = 0usize;
        while idx < count {
            let rela = unsafe {
                read_runtime::<Elf64Rela>(
                    runtime_addr(object.bias, info.rela)? + idx as u64 * info.relaent as u64,
                )
            };
            let r_type = (rela.r_info & 0xffff_ffff) as u32;
            let r_sym = (rela.r_info >> 32) as usize;
            if r_type == R_X86_64_NONE {
                idx += 1;
                continue;
            }
            if !vaddr_range_loaded(loads, load_count, rela.r_offset, 8, true) {
                return Err(Error::Invalid);
            }
            let value = match r_type {
                R_X86_64_RELATIVE => checked_add_i64(object.bias, rela.r_addend)?,
                R_X86_64_64 | R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                    let resolved = resolve_symbol_reference(object, set, r_sym)?;
                    checked_add_i64(resolved.value, rela.r_addend)?
                }
                R_X86_64_DTPMOD64 => {
                    let resolved = resolve_symbol_reference(object, set, r_sym)?;
                    if !resolved.is_tls || resolved.tls_module_id == 0 {
                        return Err(Error::Invalid);
                    }
                    checked_add_i64(resolved.tls_module_id, rela.r_addend)?
                }
                R_X86_64_DTPOFF64 => {
                    let resolved = resolve_symbol_reference(object, set, r_sym)?;
                    if !resolved.is_tls {
                        return Err(Error::Invalid);
                    }
                    checked_add_i64(resolved.tls_offset, rela.r_addend)?
                }
                R_X86_64_TPOFF64 => {
                    let resolved = resolve_symbol_reference(object, set, r_sym)?;
                    if !resolved.is_tls {
                        return Err(Error::Invalid);
                    }
                    let target = checked_add_i64(resolved.value, rela.r_addend)?;
                    let tls_base = thread::tls_base();
                    if tls_base == 0 || target < tls_base {
                        return Err(Error::Invalid);
                    }
                    target.checked_sub(tls_base).ok_or(Error::Invalid)?
                }
                _ => return Err(Error::Invalid),
            };
            unsafe {
                ptr::write_unaligned(runtime_addr(object.bias, rela.r_offset)? as *mut u64, value);
            }
            idx += 1;
        }
        Ok(count)
    }

    fn symbol_value(
        info: &DynamicInfo,
        bias: u64,
        symbol_count: usize,
        index: usize,
    ) -> Result<u64> {
        if index == 0 || index >= symbol_count {
            return Err(Error::Invalid);
        }
        let sym = unsafe {
            read_runtime::<Elf64Sym>(
                runtime_addr(bias, info.symtab)? + index as u64 * info.syment as u64,
            )
        };
        if sym.st_shndx == 0 || sym.st_value == 0 {
            return Err(Error::Invalid);
        }
        runtime_addr(bias, sym.st_value)
    }

    fn resolve_symbol_reference(
        object: &LoadedObject,
        set: &LoadedSet,
        index: usize,
    ) -> Result<ResolvedSymbol> {
        if index == 0 || index >= object.symbol_count {
            return Err(Error::Invalid);
        }
        let sym = unsafe { read_object_symbol(object, index) };
        if sym.st_name as usize >= object.strsz {
            return Err(Error::Invalid);
        }
        if sym.st_shndx != 0 {
            return object_symbol_value(object, sym);
        }
        let name_addr = object.strtab + sym.st_name as u64;
        let name_max = object.strsz - sym.st_name as usize;
        let mut idx = 0usize;
        while idx < set.object_count {
            if let Ok(resolved) =
                find_export_by_runtime_name(&set.objects[idx], name_addr, name_max)
            {
                return Ok(resolved);
            }
            idx += 1;
        }
        Err(Error::Invalid)
    }

    fn find_export_by_runtime_name(
        object: &LoadedObject,
        name_addr: u64,
        name_max: usize,
    ) -> Result<ResolvedSymbol> {
        let mut idx = 1usize;
        while idx < object.symbol_count {
            let sym = unsafe { read_object_symbol(object, idx) };
            if sym.st_name as usize >= object.strsz || sym.st_shndx == 0 {
                idx += 1;
                continue;
            }
            let str_addr = object.strtab + sym.st_name as u64;
            if cstr_runtime_eq(
                str_addr,
                object.strsz - sym.st_name as usize,
                name_addr,
                name_max,
            ) {
                return object_symbol_value(object, sym);
            }
            idx += 1;
        }
        Err(Error::Invalid)
    }

    fn object_symbol_value(object: &LoadedObject, sym: Elf64Sym) -> Result<ResolvedSymbol> {
        if sym.st_shndx == 0 {
            return Err(Error::Invalid);
        }
        if symbol_type(sym.st_info) == STT_TLS {
            let tls_offset = object_tls_offset(object, sym.st_value, sym.st_size)?;
            return Ok(ResolvedSymbol {
                value: object
                    .tls_addr
                    .checked_add(tls_offset)
                    .ok_or(Error::Invalid)?,
                tls_module_id: object.tls_module_id,
                tls_offset,
                is_tls: true,
            });
        }
        Ok(ResolvedSymbol {
            value: runtime_addr(object.bias, sym.st_value)?,
            tls_module_id: 0,
            tls_offset: 0,
            is_tls: false,
        })
    }

    fn object_tls_offset(object: &LoadedObject, value: u64, size: u64) -> Result<u64> {
        if object.tls_addr == 0 || object.tls_memsz == 0 {
            return Err(Error::Invalid);
        }
        let offset = if value >= object.tls_vaddr {
            let end = object
                .tls_vaddr
                .checked_add(object.tls_memsz as u64)
                .ok_or(Error::Invalid)?;
            if value < end {
                value - object.tls_vaddr
            } else {
                value
            }
        } else {
            value
        };
        let need = if size == 0 { 1 } else { size };
        let end = offset.checked_add(need).ok_or(Error::Invalid)?;
        if end > object.tls_memsz as u64 {
            return Err(Error::Invalid);
        }
        Ok(offset)
    }

    unsafe fn read_object_symbol(object: &LoadedObject, index: usize) -> Elf64Sym {
        read_runtime::<Elf64Sym>(object.symtab + index as u64 * object.syment as u64)
    }

    fn symbol_type(info: u8) -> u8 {
        info & 0x0f
    }

    fn protect_load_segments(loads: &[LoadSegment; MAX_LOAD_SEGMENTS], count: usize) -> Result<()> {
        let mut idx = 0usize;
        while idx < count {
            let flags = loads[idx].flags;
            if flags & PF_W != 0 && flags & PF_X != 0 {
                return Err(Error::Invalid);
            }
            let prot = if flags & PF_X != 0 {
                memory::PROT_EXEC
            } else if flags & PF_W != 0 {
                memory::PROT_WRITE
            } else {
                0
            };
            memory::mprotect(loads[idx].map_start, loads[idx].map_len as usize, prot)?;
            idx += 1;
        }
        Ok(())
    }

    fn program_header(
        image: &[u8],
        header: &Elf64Header,
        index: u16,
    ) -> Result<Elf64ProgramHeader> {
        let off = header.e_phoff as usize + index as usize * header.e_phentsize as usize;
        read_struct::<Elf64ProgramHeader>(image, off).ok_or(Error::Invalid)
    }

    fn vaddr_range_loaded(
        loads: &[LoadSegment; MAX_LOAD_SEGMENTS],
        count: usize,
        vaddr: u64,
        len: u64,
        writable: bool,
    ) -> bool {
        if len == 0 {
            return false;
        }
        let Some(end) = vaddr.checked_add(len) else {
            return false;
        };
        let mut idx = 0usize;
        while idx < count {
            let load = loads[idx];
            if vaddr >= load.vaddr_start
                && end <= load.vaddr_end
                && (!writable || load.flags & PF_W != 0)
            {
                return true;
            }
            idx += 1;
        }
        false
    }

    fn cstr_eq(addr: u64, max_len: usize, name: &[u8]) -> bool {
        let mut idx = 0usize;
        loop {
            if idx >= max_len {
                return false;
            }
            let byte = unsafe { ptr::read((addr + idx as u64) as *const u8) };
            if idx == name.len() {
                return byte == 0;
            }
            if byte == 0 || byte != name[idx] {
                return false;
            }
            idx += 1;
        }
    }

    fn cstr_runtime_eq(left: u64, left_max: usize, right: u64, right_max: usize) -> bool {
        let mut idx = 0usize;
        loop {
            if idx >= left_max || idx >= right_max {
                return false;
            }
            let left_byte = unsafe { ptr::read((left + idx as u64) as *const u8) };
            let right_byte = unsafe { ptr::read((right + idx as u64) as *const u8) };
            if left_byte != right_byte {
                return false;
            }
            if left_byte == 0 {
                return true;
            }
            idx += 1;
        }
    }

    fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
        if left.len() != right.len() {
            return false;
        }
        let mut idx = 0usize;
        while idx < left.len() {
            if left[idx] != right[idx] {
                return false;
            }
            idx += 1;
        }
        true
    }

    fn basename(path: &[u8]) -> &[u8] {
        let mut start = 0usize;
        let mut idx = 0usize;
        while idx < path.len() {
            if path[idx] == b'/' {
                start = idx + 1;
            }
            idx += 1;
        }
        &path[start..]
    }

    fn build_lib_path(name: &[u8], out: &mut [u8; MAX_PATH_BYTES]) -> Result<usize> {
        if name.is_empty() || name.len() + 5 > out.len() {
            return Err(Error::Invalid);
        }
        let mut idx = 0usize;
        while idx < name.len() {
            if name[idx] == b'/' || name[idx] == 0 {
                return Err(Error::Invalid);
            }
            idx += 1;
        }
        out[0] = b'/';
        out[1] = b'l';
        out[2] = b'i';
        out[3] = b'b';
        out[4] = b'/';
        idx = 0;
        while idx < name.len() {
            out[5 + idx] = name[idx];
            idx += 1;
        }
        Ok(5 + name.len())
    }

    fn copy_bytes(src: &[u8], dst: &mut [u8]) -> Result<usize> {
        if src.len() > dst.len() {
            return Err(Error::Invalid);
        }
        let mut idx = 0usize;
        while idx < src.len() {
            dst[idx] = src[idx];
            idx += 1;
        }
        Ok(src.len())
    }

    fn vaddr_to_file_offset(
        image: &[u8],
        header: &Elf64Header,
        vaddr: u64,
        len: u64,
    ) -> Result<usize> {
        let end = vaddr.checked_add(len).ok_or(Error::Invalid)?;
        let mut i = 0u16;
        while i < header.e_phnum {
            let ph = program_header(image, header, i)?;
            if ph.p_type == PT_LOAD {
                let file_end_vaddr = ph.p_vaddr.checked_add(ph.p_filesz).ok_or(Error::Invalid)?;
                if vaddr >= ph.p_vaddr && end <= file_end_vaddr {
                    let off = ph
                        .p_offset
                        .checked_add(vaddr - ph.p_vaddr)
                        .ok_or(Error::Invalid)?;
                    if off.checked_add(len).ok_or(Error::Invalid)? <= image.len() as u64 {
                        return Ok(off as usize);
                    }
                }
            }
            i += 1;
        }
        Err(Error::Invalid)
    }

    fn read_struct<T: Copy>(bytes: &[u8], offset: usize) -> Option<T> {
        let end = offset.checked_add(mem::size_of::<T>())?;
        if end > bytes.len() {
            return None;
        }
        Some(unsafe { ptr::read_unaligned(bytes.as_ptr().add(offset) as *const T) })
    }

    unsafe fn read_runtime<T: Copy>(addr: u64) -> T {
        ptr::read_unaligned(addr as *const T)
    }

    fn runtime_addr(bias: u64, vaddr: u64) -> Result<u64> {
        bias.checked_add(vaddr).ok_or(Error::Invalid)
    }

    fn checked_add_i64(base: u64, value: i64) -> Result<u64> {
        if value >= 0 {
            base.checked_add(value as u64).ok_or(Error::Invalid)
        } else {
            base.checked_sub(value.wrapping_neg() as u64)
                .ok_or(Error::Invalid)
        }
    }

    fn align_down(value: u64, align: u64) -> u64 {
        value & !(align - 1)
    }

    fn align_up(value: u64, align: u64) -> Result<u64> {
        value
            .checked_add(align - 1)
            .map(|v| v & !(align - 1))
            .ok_or(Error::Invalid)
    }

    fn align_up_usize(value: usize, align: usize) -> Result<usize> {
        value
            .checked_add(align - 1)
            .map(|v| v & !(align - 1))
            .ok_or(Error::Invalid)
    }

    fn ranges_overlap(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> bool {
        a_start < b_end && b_start < a_end
    }
}

pub mod ipc {
    use super::{sys, Error, Result};

    pub fn shmem_create(len: usize) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::SHMEM_CREATE, len as u64) };
        Error::from_ret(ret)
    }

    pub fn shmem_map(id: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall1(sys::SHMEM_MAP, id) };
        Error::from_ret(ret)
    }
}

pub mod fs {
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
}

pub mod time {
    use super::sys;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct DateTime {
        pub year: u16,
        pub month: u8,
        pub day: u8,
        pub hour: u8,
        pub minute: u8,
    }

    pub fn now() -> Option<DateTime> {
        let packed = unsafe { sys::syscall0(sys::TIME) };
        if packed == 0 {
            return None;
        }
        Some(DateTime {
            year: (packed >> 32) as u16,
            month: (packed >> 24) as u8,
            day: (packed >> 16) as u8,
            hour: (packed >> 8) as u8,
            minute: packed as u8,
        })
    }
}

pub mod net {
    use super::{sys, Error, Result};

    pub const AF_INET: u64 = 2;
    pub const SOCK_STREAM: u64 = 1;
    pub const IPPROTO_TCP: u64 = 6;

    pub fn dns_resolve(host: &[u8]) -> Result<u32> {
        let ret =
            unsafe { sys::syscall2(sys::DNS_RESOLVE, host.as_ptr() as u64, host.len() as u64) };
        Error::from_ret(ret).map(|addr| addr as u32)
    }

    pub fn http_get(host: &[u8]) -> Result<usize> {
        let ret = unsafe { sys::syscall2(sys::HTTP_GET, host.as_ptr() as u64, host.len() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn socket(domain: u64, socket_type: u64, protocol: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall3(sys::SOCKET, domain, socket_type, protocol) };
        Error::from_ret(ret)
    }

    pub fn tcp_socket() -> Result<u64> {
        socket(AF_INET, SOCK_STREAM, IPPROTO_TCP)
    }

    pub fn connect(socket: u64, ipv4_be: u32, port: u16) -> Result<()> {
        let ret = unsafe { sys::syscall3(sys::CONNECT, socket, ipv4_be as u64, port as u64) };
        Error::from_ret(ret).map(|_| ())
    }

    pub fn send(socket: u64, bytes: &[u8]) -> Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let ret =
            unsafe { sys::syscall3(sys::SEND, socket, bytes.as_ptr() as u64, bytes.len() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn send_all(socket: u64, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let n = send(socket, bytes)?;
            if n == 0 {
                return Err(Error::Failed);
            }
            bytes = &bytes[n.min(bytes.len())..];
        }
        Ok(())
    }

    pub fn recv(socket: u64, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let ret =
            unsafe { sys::syscall3(sys::RECV, socket, buf.as_mut_ptr() as u64, buf.len() as u64) };
        Error::from_ret(ret).map(|n| n as usize)
    }

    pub fn parse_ipv4(s: &[u8]) -> Option<u32> {
        let mut out = 0u32;
        let mut part = 0u32;
        let mut parts = 0usize;
        let mut saw_digit = false;
        for &b in s {
            if b == b'.' {
                if !saw_digit || part > 255 {
                    return None;
                }
                out = (out << 8) | part;
                part = 0;
                saw_digit = false;
                parts += 1;
            } else if b.is_ascii_digit() {
                part = part * 10 + (b - b'0') as u32;
                saw_digit = true;
            } else {
                return None;
            }
        }
        if !saw_digit || part > 255 || parts != 3 {
            return None;
        }
        Some((out << 8) | part)
    }
}

pub mod event {
    use super::{io, Error, Result};

    pub const INPUT_FD: u64 = 3;
    pub const EVENT_PACKET_SIZE: usize = 8;
    pub const EVENT_KIND_KEY_CHAR: u8 = 1;
    pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        KeyChar { bytes: [u8; 4], len: usize },
        MouseDown { x: u16, y: u16 },
    }

    impl Event {
        pub fn parse(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Self> {
            match packet[0] {
                EVENT_KIND_KEY_CHAR => {
                    let len = packet[1] as usize;
                    if len == 0 || len > 4 {
                        return Err(Error::Invalid);
                    }
                    let mut bytes = [0u8; 4];
                    bytes[..len].copy_from_slice(&packet[2..2 + len]);
                    Ok(Event::KeyChar { bytes, len })
                }
                EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
                    x: u16::from_le_bytes([packet[2], packet[3]]),
                    y: u16::from_le_bytes([packet[4], packet[5]]),
                }),
                _ => Err(Error::Invalid),
            }
        }
    }

    pub fn read_event(fd: u64) -> Result<Option<Event>> {
        let mut packet = [0u8; EVENT_PACKET_SIZE];
        let n = io::read(fd, &mut packet)?;
        if n == 0 {
            return Ok(None);
        }
        if n != EVENT_PACKET_SIZE {
            return Err(Error::Invalid);
        }
        Event::parse(&packet).map(Some)
    }
}

pub mod evented {
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
}

pub mod tty {
    use super::{sys, Error, Result};

    pub const MODE_CANONICAL: u64 = 1 << 0;
    pub const MODE_ECHO: u64 = 1 << 1;
    pub const MODE_SIGNALS: u64 = 1 << 2;
    pub const MODE_DEFAULT: u64 = MODE_CANONICAL | MODE_ECHO | MODE_SIGNALS;
    pub const MODE_RAW: u64 = 0;

    pub const CTL_GET_MODE: u64 = 0;
    pub const CTL_SET_MODE: u64 = 1;
    pub const CTL_GET_SIZE: u64 = 2;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Size {
        pub cols: u16,
        pub rows: u16,
    }

    pub fn control(op: u64, arg1: u64, arg2: u64) -> Result<u64> {
        let ret = unsafe { sys::syscall3(sys::TTY_CONTROL, op, arg1, arg2) };
        Error::from_ret(ret)
    }

    pub fn mode() -> Result<u64> {
        control(CTL_GET_MODE, 0, 0)
    }

    pub fn set_mode(mode: u64) -> Result<u64> {
        control(CTL_SET_MODE, mode, 0)
    }

    pub fn enter_raw_mode() -> Result<u64> {
        set_mode(MODE_RAW)
    }

    pub fn restore_mode(mode: u64) -> Result<()> {
        set_mode(mode).map(|_| ())
    }

    pub fn size() -> Result<Size> {
        let packed = control(CTL_GET_SIZE, 0, 0)?;
        Ok(Size {
            cols: (packed & 0xffff) as u16,
            rows: ((packed >> 16) & 0xffff) as u16,
        })
    }
}

pub mod gui {
    use font8x8::UnicodeFonts;

    use super::{evented, process, sys, Error, Result};

    pub const EVENT_PACKET_SIZE: usize = 16;
    pub const EVENT_KIND_KEY_CHAR: u8 = 1;
    pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;
    pub const EVENT_KIND_CLOSE: u8 = 3;
    pub const EVENT_KIND_RESIZE: u8 = 4;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        KeyChar { bytes: [u8; 4], len: usize },
        MouseDown { button: u8, x: u16, y: u16 },
        Close,
        Resize { width: u16, height: u16 },
    }

    pub struct Window {
        handle: u64,
        width: usize,
        height: usize,
    }

    impl Window {
        pub fn open(title: &[u8], width: u16, height: u16) -> Result<Self> {
            let dims = (width as u64) | ((height as u64) << 16);
            let mut ret = u64::MAX;
            for _ in 0..128 {
                ret = unsafe {
                    sys::syscall3(
                        sys::GUI_OPEN,
                        title.as_ptr() as u64,
                        title.len() as u64,
                        dims,
                    )
                };
                if ret != u64::MAX {
                    break;
                }
                process::sleep_ms(1);
            }
            Error::from_ret(ret).map(|handle| Window {
                handle,
                width: width as usize,
                height: height as usize,
            })
        }

        #[inline]
        pub const fn handle(&self) -> u64 {
            self.handle
        }

        #[inline]
        pub const fn width(&self) -> usize {
            self.width
        }

        #[inline]
        pub const fn height(&self) -> usize {
            self.height
        }

        pub fn present(&self, pixels: &[u32]) -> Result<()> {
            if pixels.len() != self.width.saturating_mul(self.height) {
                return Err(Error::Invalid);
            }
            let bytes = unsafe {
                core::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)
            };
            let mut ret = u64::MAX;
            for _ in 0..64 {
                ret = unsafe {
                    sys::syscall3(
                        sys::GUI_PRESENT,
                        self.handle,
                        bytes.as_ptr() as u64,
                        bytes.len() as u64,
                    )
                };
                if ret != u64::MAX {
                    break;
                }
                process::sleep_ms(1);
            }
            Error::from_ret(ret).map(|_| ())
        }

        pub fn poll_event(&mut self) -> Result<Option<Event>> {
            let mut packet = [0u8; EVENT_PACKET_SIZE];
            let ret = unsafe {
                sys::syscall3(
                    sys::GUI_POLL_EVENT,
                    self.handle,
                    packet.as_mut_ptr() as u64,
                    EVENT_PACKET_SIZE as u64,
                )
            };
            if ret == u64::MAX {
                return Err(Error::Failed);
            }
            if ret == 0 {
                return Ok(None);
            }
            let event = parse_event(&packet)?;
            if let Event::Resize { width, height } = event {
                self.width = width as usize;
                self.height = height as usize;
            }
            Ok(Some(event))
        }

        pub fn wait_event_ready(&self, timeout_ms: u64) -> Result<bool> {
            evented::wait_gui_event(self.handle, timeout_ms)
        }

        pub fn close(self) -> Result<()> {
            let handle = self.handle;
            core::mem::forget(self);
            close_handle(handle)
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            let _ = close_handle(self.handle);
        }
    }

    pub struct Canvas<'a> {
        pixels: &'a mut [u32],
        width: usize,
        height: usize,
    }

    impl<'a> Canvas<'a> {
        pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Self {
            Canvas {
                pixels,
                width,
                height,
            }
        }

        pub fn clear(&mut self, color: u32) {
            for pixel in self.pixels.iter_mut() {
                *pixel = color & 0x00ff_ffff;
            }
        }

        pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
            let x0 = x.max(0) as usize;
            let y0 = y.max(0) as usize;
            let x1 = (x + w).clamp(0, self.width as i32) as usize;
            let y1 = (y + h).clamp(0, self.height as i32) as usize;
            if x0 >= x1 || y0 >= y1 {
                return;
            }
            let color = color & 0x00ff_ffff;
            for row in y0..y1 {
                let start = row * self.width + x0;
                let end = row * self.width + x1;
                for pixel in &mut self.pixels[start..end] {
                    *pixel = color;
                }
            }
        }

        pub fn border(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
            if w <= 0 || h <= 0 {
                return;
            }
            self.rect(x, y, w, 1, color);
            self.rect(x, y + h - 1, w, 1, color);
            self.rect(x, y, 1, h, color);
            self.rect(x + w - 1, y, 1, h, color);
        }

        pub fn text(&mut self, x: i32, y: i32, text: &str, color: u32) {
            let mut cursor = x;
            for ch in text.chars() {
                self.char(cursor, y, ch, color);
                cursor += 8;
            }
        }

        pub fn char(&mut self, x: i32, y: i32, ch: char, color: u32) {
            let glyph = font8x8::BASIC_FONTS
                .get(ch)
                .or_else(|| font8x8::BASIC_FONTS.get(' '));
            let Some(glyph) = glyph else {
                return;
            };
            let color = color & 0x00ff_ffff;
            for (gy, &byte) in glyph.iter().enumerate() {
                for bit in 0..8usize {
                    if byte & (1 << bit) == 0 {
                        continue;
                    }
                    let px = x + bit as i32;
                    let py = y + gy as i32;
                    if px >= 0 && py >= 0 {
                        let px = px as usize;
                        let py = py as usize;
                        if px < self.width && py < self.height {
                            self.pixels[py * self.width + px] = color;
                        }
                    }
                }
            }
        }
    }

    fn close_handle(handle: u64) -> Result<()> {
        let mut ret = u64::MAX;
        for _ in 0..64 {
            ret = unsafe { sys::syscall1(sys::GUI_CLOSE, handle) };
            if ret != u64::MAX {
                break;
            }
            process::sleep_ms(1);
        }
        Error::from_ret(ret).map(|_| ())
    }

    fn parse_event(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Event> {
        match packet[0] {
            EVENT_KIND_KEY_CHAR => {
                let len = packet[1] as usize;
                if len == 0 || len > 4 {
                    return Err(Error::Invalid);
                }
                let mut bytes = [0u8; 4];
                bytes[..len].copy_from_slice(&packet[2..2 + len]);
                Ok(Event::KeyChar { bytes, len })
            }
            EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
                button: packet[1],
                x: u16::from_le_bytes([packet[2], packet[3]]),
                y: u16::from_le_bytes([packet[4], packet[5]]),
            }),
            EVENT_KIND_CLOSE => Ok(Event::Close),
            EVENT_KIND_RESIZE => Ok(Event::Resize {
                width: u16::from_le_bytes([packet[2], packet[3]]),
                height: u16::from_le_bytes([packet[4], packet[5]]),
            }),
            _ => Err(Error::Invalid),
        }
    }
}

pub mod prelude {
    pub use crate::args::Args;
    pub use crate::event::{read_event, Event, INPUT_FD};
    pub use crate::evented::{
        poll, wait_child, wait_fd_read, wait_gui_event, wait_socket_read, PollDesc,
    };
    pub use crate::io::{close, create, open, pipe, read, write, write_all, write_stdout, File};
    pub use crate::memory::{mmap, mmap_flags, mprotect, PROT_EXEC, PROT_WRITE};
    pub use crate::process::{
        abi_version, exit, get_process_group, getpid, set_process_group, signal, signal_group,
        sleep_ms, spawn, spawn_args, spawn_fds_args, waitpid, yield_now, Signal,
    };
    pub use crate::thread::{
        self, FutexWait, PThreadCondvar, PThreadMutex, PThreadOnce, TlsBlock, TlsKey,
    };
    pub use crate::tty;
    pub use crate::{entry, print, println, Error, Result, ABI_VERSION, SDK_VERSION};
    pub use crate::{libc, posix};
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::io::_print(core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::io::write_stdout(b"\n");
    }};
    ($fmt:expr) => {{
        $crate::print!(concat!($fmt, "\n"));
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!(concat!($fmt, "\n"), $($arg)*);
    }};
}

#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub extern "C" fn _start() -> ! {
            core::arch::naked_asm!(
                "mov rdi, rsp",
                // __libcool_entry is a normal SysV function. Enter it with the
                // same 16-byte stack alignment it would see after a call.
                "sub rsp, 8",
                "jmp {entry}",
                entry = sym __libcool_entry,
            );
        }

        extern "C" fn __libcool_entry(rsp: u64) -> ! {
            let args = unsafe { $crate::Args::from_stack(rsp) };
            $main(args)
        }

        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            $crate::process::abort()
        }
    };
}
