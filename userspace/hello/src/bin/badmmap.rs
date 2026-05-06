#![no_std]
#![no_main]

use libcool::prelude::*;

libcool::entry!(main);

fn main(_args: Args) -> ! {
    match mmap(0x100000, 4096, true) {
        Err(_) => {
            println!("badmmap: denied");
            park();
        }
        Ok(_) => {
            println!("badmmap: allowed");
            park();
        }
    }
}

fn park() -> ! {
    loop {
        sleep_ms(1000);
    }
}
