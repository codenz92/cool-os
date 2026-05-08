#![no_std]
#![no_main]

use core::ptr::addr_of_mut;
use core::sync::atomic::{AtomicU64, Ordering};

use libcool::prelude::*;

libcool::entry!(main);

static MUTEX: PThreadMutex = PThreadMutex::new();
static COND: PThreadCondvar = PThreadCondvar::new();
static ONCE: PThreadOnce = PThreadOnce::new();
static DONE: AtomicU64 = AtomicU64::new(0);
static SUM: AtomicU64 = AtomicU64::new(0);
static ONCE_RUNS: AtomicU64 = AtomicU64::new(0);

static mut MAIN_TLS: TlsBlock = TlsBlock::new(0);
static mut WORKER_ONE_TLS: TlsBlock = TlsBlock::new(0);
static mut WORKER_TWO_TLS: TlsBlock = TlsBlock::new(0);

fn main(_args: Args) -> ! {
    println!("tlsdemo: abi={}", abi_version());

    if unsafe { thread::install_tls_block(addr_of_mut!(MAIN_TLS), 100) }.is_err() {
        println!("tlsdemo: main tls install failed");
        exit(1);
    }
    let main_id = thread::tls_logical_id().unwrap_or(0);
    println!(
        "tlsdemo: main tls base={:#x} logical={}",
        thread::tls_base(),
        main_id
    );
    if main_id != 100 {
        exit(2);
    }

    let key = match thread::tls_key_create() {
        Ok(key) => key,
        Err(_) => {
            println!("tlsdemo: key create failed");
            exit(3);
        }
    };
    if thread::tls_set(key, 900).is_err() || thread::tls_get(key).unwrap_or(0) != 900 {
        println!("tlsdemo: main tls slot failed");
        exit(4);
    }

    unsafe {
        let _ = thread::prepare_tls_block(addr_of_mut!(WORKER_ONE_TLS), 1);
        let _ = thread::prepare_tls_block(addr_of_mut!(WORKER_TWO_TLS), 2);
    }

    MUTEX.lock();
    let first =
        match unsafe { thread::spawn_tls(worker, pack_arg(21, key), addr_of_mut!(WORKER_ONE_TLS)) }
        {
            Ok(tid) => tid,
            Err(_) => {
                MUTEX.unlock();
                println!("tlsdemo: spawn first failed");
                exit(5);
            }
        };
    let second =
        match unsafe { thread::spawn_tls(worker, pack_arg(51, key), addr_of_mut!(WORKER_TWO_TLS)) }
        {
            Ok(tid) => tid,
            Err(_) => {
                MUTEX.unlock();
                println!("tlsdemo: spawn second failed");
                exit(6);
            }
        };
    println!("tlsdemo: spawned {} {}", first, second);

    while DONE.load(Ordering::SeqCst) < 2 {
        if COND.wait(&MUTEX).is_err() {
            MUTEX.unlock();
            println!("tlsdemo: cond wait failed");
            exit(7);
        }
    }
    MUTEX.unlock();

    let done = DONE.load(Ordering::SeqCst);
    let sum = SUM.load(Ordering::SeqCst);
    let once = ONCE_RUNS.load(Ordering::SeqCst);
    println!("tlsdemo: done={} sum={} once={}", done, sum, once);
    if done != 2 || sum != 102 || once != 1 {
        exit(8);
    }

    let first_code = thread::join(first).unwrap_or(u64::MAX);
    let second_code = thread::join(second).unwrap_or(u64::MAX);
    println!("tlsdemo: join {} {}", first_code, second_code);
    if first_code != 1 || second_code != 2 {
        exit(9);
    }

    println!("tlsdemo: phase73 ok");
    exit(0);
}

extern "C" fn worker(arg: u64) -> ! {
    if thread::bind_current_tls_os_tid().is_err() {
        thread::exit(80);
    }
    let logical = thread::tls_logical_id().unwrap_or(0);
    let os_tid = thread::tls_os_tid().unwrap_or(0);
    let value = arg >> 32;
    let key = match TlsKey::from_index((arg & 0xffff) as usize) {
        Ok(key) => key,
        Err(_) => thread::exit(81),
    };
    let slot_value = logical.saturating_mul(10).saturating_add(value);
    if os_tid == 0
        || thread::tls_set(key, slot_value).is_err()
        || thread::tls_get(key).unwrap_or(0) != slot_value
    {
        thread::exit(82);
    }

    MUTEX.lock();
    if ONCE.call_once(init_once).is_err() {
        MUTEX.unlock();
        thread::exit(83);
    }
    SUM.fetch_add(slot_value, Ordering::SeqCst);
    DONE.fetch_add(1, Ordering::SeqCst);
    COND.notify_one();
    MUTEX.unlock();

    thread::exit(logical);
}

fn init_once() {
    ONCE_RUNS.fetch_add(1, Ordering::SeqCst);
}

fn pack_arg(value: u64, key: TlsKey) -> u64 {
    (value << 32) | key.index() as u64
}
