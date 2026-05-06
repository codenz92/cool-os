#![no_std]
#![no_main]

use libcool::{prelude::*, sys};

libcool::entry!(main);

fn main(_args: Args) -> ! {
    let ret = unsafe { sys::syscall2(sys::EXEC, 0x100000, 16) };
    if ret == u64::MAX {
        println!("badexec: denied");
        park();
    }

    println!("badexec: allowed");
    park();
}

fn park() -> ! {
    loop {
        sleep_ms(1000);
    }
}
