#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU64, Ordering};

use libcool::prelude::*;

libcool::entry!(main);

static DONE: AtomicU64 = AtomicU64::new(0);
static SUM: AtomicU64 = AtomicU64::new(0);

fn main(_args: Args) -> ! {
    println!("threaddemo: abi={}", abi_version());

    let first = match thread::spawn(worker, 21) {
        Ok(tid) => tid,
        Err(_) => {
            println!("threaddemo: spawn first failed");
            exit(1);
        }
    };
    let second = match thread::spawn(worker, 51) {
        Ok(tid) => tid,
        Err(_) => {
            println!("threaddemo: spawn second failed");
            exit(2);
        }
    };
    println!("threaddemo: spawned {} {}", first, second);

    while DONE.load(Ordering::SeqCst) < 2 {
        let observed = DONE.load(Ordering::SeqCst);
        match thread::futex_wait(done_addr(), observed, thread::TIMEOUT_FOREVER) {
            Ok(FutexWait::Woken) | Ok(FutexWait::Mismatch) => {}
            Ok(FutexWait::Timeout) => {
                println!("threaddemo: futex timeout");
                exit(3);
            }
            Err(_) => {
                println!("threaddemo: futex wait failed");
                exit(4);
            }
        }
    }

    let done = DONE.load(Ordering::SeqCst);
    let sum = SUM.load(Ordering::SeqCst);
    println!("threaddemo: futex woke done={} sum={}", done, sum);
    if done != 2 || sum != 72 {
        exit(5);
    }

    let first_code = thread::join(first).unwrap_or(u64::MAX);
    let second_code = thread::join(second).unwrap_or(u64::MAX);
    println!("threaddemo: join {} {}", first_code, second_code);
    if first_code != 21 || second_code != 51 {
        exit(6);
    }

    println!("threaddemo: phase72 ok");
    exit(0);
}

extern "C" fn worker(value: u64) -> ! {
    SUM.fetch_add(value, Ordering::SeqCst);
    DONE.fetch_add(1, Ordering::SeqCst);
    let _ = thread::futex_wake(done_addr(), 1);
    exit(value);
}

fn done_addr() -> *const u64 {
    &DONE as *const AtomicU64 as *const u64
}
