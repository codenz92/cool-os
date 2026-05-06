#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    println!("baduserread: touching kernel page");
    sleep_ms(100);
    let value = unsafe { core::ptr::read_volatile(0x100000 as *const u64) };
    println!("baduserread: survived {:#x}", value);
    park();
}

fn park() -> ! {
    loop {
        sleep_ms(1000);
    }
}
