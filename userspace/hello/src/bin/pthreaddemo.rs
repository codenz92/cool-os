#![no_std]
#![no_main]

use core::ptr::{addr_of_mut, null, null_mut};
use core::sync::atomic::{AtomicU64, Ordering};

use libcool::posix::*;
use libcool::prelude::*;

libcool::entry!(main);

static DONE: AtomicU64 = AtomicU64::new(0);
static SUM: AtomicU64 = AtomicU64::new(0);
static ERROR: AtomicU64 = AtomicU64::new(0);
static ONCE_RUNS: AtomicU64 = AtomicU64::new(0);

static mut MAIN_TLS: TlsBlock = TlsBlock::new(0);
static mut MUTEX: pthread_mutex_t = pthread_mutex_t::new();
static mut COND: pthread_cond_t = pthread_cond_t::new();
static mut ONCE: pthread_once_t = pthread_once_t::new();

fn main(_args: Args) -> ! {
    println!("pthreaddemo: abi={}", abi_version());
    if unsafe { init_main_thread(addr_of_mut!(MAIN_TLS)) } != 0 {
        println!("pthreaddemo: main pthread tls failed");
        exit(1);
    }

    set_errno(5);
    let errno_ptr = errno_location();
    if errno_ptr.is_null() || unsafe { *errno_ptr } != 5 {
        println!("pthreaddemo: errno location failed");
        exit(2);
    }
    println!("pthreaddemo: main tid={} errno={}", pthread_self(), errno());

    let mut key = 0usize;
    if pthread_key_create(&mut key as *mut pthread_key_t, None) != 0 {
        println!("pthreaddemo: key create failed errno={}", errno());
        exit(3);
    }

    let mut first = 0u64;
    let mut second = 0u64;
    if pthread_mutex_lock(addr_of_mut!(MUTEX)) != 0 {
        println!("pthreaddemo: mutex lock failed");
        exit(4);
    }
    if pthread_create(
        &mut first as *mut pthread_t,
        null(),
        worker,
        pack_arg(21, key),
    ) != 0
    {
        let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));
        println!("pthreaddemo: spawn first failed errno={}", errno());
        exit(5);
    }
    if pthread_create(
        &mut second as *mut pthread_t,
        null(),
        worker,
        pack_arg(51, key),
    ) != 0
    {
        let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));
        println!("pthreaddemo: spawn second failed errno={}", errno());
        exit(6);
    }
    println!("pthreaddemo: spawned {} {}", first, second);

    while DONE.load(Ordering::SeqCst) < 2 {
        if pthread_cond_wait(addr_of_mut!(COND), addr_of_mut!(MUTEX)) != 0 {
            let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));
            println!("pthreaddemo: cond wait failed errno={}", errno());
            exit(7);
        }
    }
    let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));

    let done = DONE.load(Ordering::SeqCst);
    let sum = SUM.load(Ordering::SeqCst);
    let once = ONCE_RUNS.load(Ordering::SeqCst);
    let error = ERROR.load(Ordering::SeqCst);
    println!(
        "pthreaddemo: done={} sum={} once={} errno={}",
        done,
        sum,
        once,
        errno()
    );
    if done != 2 || sum != 72 || once != 1 || error != 0 || errno() != 5 {
        println!("pthreaddemo: aggregate failed error={}", error);
        exit(8);
    }

    let mut first_ret: *mut c_void = null_mut();
    let mut second_ret: *mut c_void = null_mut();
    if pthread_join(first, &mut first_ret as *mut *mut c_void) != 0
        || pthread_join(second, &mut second_ret as *mut *mut c_void) != 0
    {
        println!("pthreaddemo: join failed errno={}", errno());
        exit(9);
    }
    let first_code = first_ret as usize as u64;
    let second_code = second_ret as usize as u64;
    println!("pthreaddemo: join {} {}", first_code, second_code);
    if first_code != 21 || second_code != 51 {
        exit(10);
    }

    let req = timespec {
        tv_sec: 0,
        tv_nsec: 1_000_000,
    };
    if sched_yield() != 0 || nanosleep(&req as *const timespec, null_mut()) != 0 {
        println!("pthreaddemo: yield/sleep failed errno={}", errno());
        exit(11);
    }
    println!("pthreaddemo: nanosleep ok");

    println!("pthreaddemo: phase74 ok");
    exit(0);
}

extern "C" fn worker(arg: *mut c_void) -> *mut c_void {
    let packed = arg as usize as u64;
    let value = packed >> 32;
    let key = (packed & 0xffff) as pthread_key_t;

    set_errno(value as c_int);
    if gettid() == 0 || errno() != value as c_int {
        return worker_fail(80);
    }
    if pthread_setspecific(key, value as usize as *mut c_void) != 0 {
        return worker_fail(81);
    }
    if pthread_getspecific(key) as usize as u64 != value {
        return worker_fail(82);
    }
    if pthread_once(addr_of_mut!(ONCE), init_once) != 0 {
        return worker_fail(83);
    }
    if pthread_mutex_lock(addr_of_mut!(MUTEX)) != 0 {
        return worker_fail(84);
    }
    SUM.fetch_add(value, Ordering::SeqCst);
    DONE.fetch_add(1, Ordering::SeqCst);
    let _ = pthread_cond_signal(addr_of_mut!(COND));
    let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));

    if errno() != value as c_int {
        return worker_fail(85);
    }
    value as usize as *mut c_void
}

extern "C" fn init_once() {
    ONCE_RUNS.fetch_add(1, Ordering::SeqCst);
}

fn worker_fail(code: u64) -> *mut c_void {
    ERROR
        .compare_exchange(0, code, Ordering::SeqCst, Ordering::SeqCst)
        .ok();
    if pthread_mutex_lock(addr_of_mut!(MUTEX)) == 0 {
        DONE.fetch_add(1, Ordering::SeqCst);
        let _ = pthread_cond_signal(addr_of_mut!(COND));
        let _ = pthread_mutex_unlock(addr_of_mut!(MUTEX));
    }
    code as usize as *mut c_void
}

fn pack_arg(value: u64, key: pthread_key_t) -> *mut c_void {
    (((value as usize) << 32) | (key & 0xffff)) as *mut c_void
}
