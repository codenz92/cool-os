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
