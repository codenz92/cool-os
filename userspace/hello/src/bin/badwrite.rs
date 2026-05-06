#![no_std]
#![no_main]

use libcool::{prelude::*, sys};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let ret = unsafe { sys::syscall3(sys::READ, 0, 0x100000, 16) };
    if ret == u64::MAX {
        println!("badwrite: denied");
        park();
    }

    println!("badwrite: allowed");
    park();
}

fn park() -> ! {
    loop {
        sleep_ms(1000);
    }
}
